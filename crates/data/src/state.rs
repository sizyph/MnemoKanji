//! Writable per-user state store: FSRS tracks + level progress in a separate SQLite DB.

use std::collections::HashMap;
use std::error::Error;

use chrono::{DateTime, NaiveDate, Utc};
use mnemokanji_core::{Card, StudyState, Track, TrackKind};
use rusqlite::Connection;
use rusqlite_migration::{MigrationDefinitionError, Migrations, M};

/// v5: seed kanji ids became Unicode codepoints; remap old auto-rowid references.
/// The frozen historical mapping inside is validated by tests below.
const V5_KANJI_CODEPOINT_IDS: &str = include_str!("migrations/v5_kanji_codepoint_ids.sql");

/// The largest kanji rowid any pre-v0.4 seed ever shipped (the frozen v5 map covers
/// 1..=245). After v5, no legitimate row can carry an id in this range — codepoints start
/// at U+3400 — so their presence means a pre-v0.4 binary wrote to the DB post-migration.
const LEGACY_ROWID_MAX: i64 = 245;

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(
            "CREATE TABLE track (
                kanji_id      INTEGER NOT NULL,
                kind          TEXT NOT NULL,              -- 'comprehension' | 'production'
                card_json     TEXT NOT NULL,              -- serialized rs-fsrs Card
                introduced_at TEXT NOT NULL,              -- RFC3339
                PRIMARY KEY (kanji_id, kind)
             );
             CREATE TABLE progress (
                id             INTEGER PRIMARY KEY CHECK (id = 1),
                unlocked_level INTEGER NOT NULL
             );
             INSERT INTO progress (id, unlocked_level) VALUES (1, 5);",
        ),
        M::up(
            "CREATE TABLE user_mnemonic (
                kanji_id  INTEGER PRIMARY KEY,
                story     TEXT NOT NULL,
                edited_at TEXT NOT NULL
             );
             CREATE TABLE app_settings (
                id               INTEGER PRIMARY KEY CHECK (id = 1),
                new_per_day      INTEGER NOT NULL,
                daily_review_cap INTEGER NOT NULL
             );
             INSERT INTO app_settings (id, new_per_day, daily_review_cap) VALUES (1, 10, 60);",
        ),
        M::up(
            "CREATE TABLE review_event (
                id       INTEGER PRIMARY KEY,
                ts       TEXT NOT NULL,             -- RFC3339
                kanji_id INTEGER NOT NULL,
                kind     TEXT NOT NULL,
                rating   INTEGER NOT NULL
             );
             CREATE INDEX idx_review_ts ON review_event(ts);",
        ),
        M::up("ALTER TABLE app_settings ADD COLUMN desired_retention REAL NOT NULL DEFAULT 0.9;"),
        M::up(V5_KANJI_CODEPOINT_IDS),
    ])
}

pub struct StateStore {
    conn: Connection,
}

impl StateStore {
    /// Open (creating + migrating to latest) the user-state DB at `path`.
    pub fn open(path: &str) -> Result<Self, Box<dyn Error>> {
        let mut conn = Connection::open(path)?;
        migrations().to_latest(&mut conn).map_err(|e| match e {
            rusqlite_migration::Error::MigrationDefinition(
                MigrationDefinitionError::DatabaseTooFarAhead,
            ) => format!(
                "user database {path} was created by a newer MnemoKanji — \
                 upgrade the app or restore an older backup"
            ),
            other => format!(
                "user database migration failed; no study progress was modified ({other}) — \
                 please report this bug"
            ),
        })?;

        // Guard against a pre-v0.4 instance having written rowid-keyed rows AFTER the v5
        // remap (e.g. an app left running across an upgrade saves on its next grade —
        // save_state full-replaces `track`, silently un-remapping everything). The old
        // binary can't be fixed, so fail loudly here instead of letting that progress
        // orphan: at v5+ the legacy rowid range is unreachable by any legitimate write.
        let stale: i64 = conn.query_row(
            "SELECT (SELECT COUNT(*) FROM track WHERE kanji_id BETWEEN 1 AND ?1)
                  + (SELECT COUNT(*) FROM review_event WHERE kanji_id BETWEEN 1 AND ?1)
                  + (SELECT COUNT(*) FROM user_mnemonic WHERE kanji_id BETWEEN 1 AND ?1)",
            [LEGACY_ROWID_MAX],
            |r| r.get(0),
        )?;
        if stale > 0 {
            return Err(format!(
                "user database {path} has {stale} rows written by an older MnemoKanji after \
                 the id migration — an old instance was probably still running during an \
                 upgrade. Restore a backup made with the current version, or delete the \
                 database to start fresh"
            )
            .into());
        }
        Ok(Self { conn })
    }

    /// A migration audit value (e.g. `v5_track_remapped`), if recorded.
    pub fn state_meta(&self, key: &str) -> Option<String> {
        self.conn
            .query_row("SELECT value FROM state_meta WHERE key = ?1", [key], |r| {
                r.get::<_, String>(0)
            })
            .ok()
    }

    pub fn load_state(&self) -> Result<StudyState, Box<dyn Error>> {
        let unlocked_level: u8 = self.conn.query_row(
            "SELECT unlocked_level FROM progress WHERE id = 1",
            [],
            |r| r.get::<_, i64>(0).map(|v| v as u8),
        )?;

        let mut tracks: HashMap<(i64, TrackKind), Track> = HashMap::new();
        let mut stmt = self
            .conn
            .prepare("SELECT kanji_id, kind, card_json, introduced_at FROM track")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        for row in rows {
            let (kanji_id, kind_s, card_json, introduced_s) = row?;
            let kind: TrackKind = kind_s.parse().map_err(|()| "unknown track kind")?;
            let card: Card = serde_json::from_str(&card_json)?;
            let introduced_at = DateTime::parse_from_rfc3339(&introduced_s)?.with_timezone(&Utc);
            tracks.insert(
                (kanji_id, kind),
                Track {
                    kanji_id,
                    kind,
                    card,
                    introduced_at,
                },
            );
        }
        Ok(StudyState {
            tracks,
            unlocked_level,
        })
    }

    pub fn save_state(&mut self, state: &StudyState) -> Result<(), Box<dyn Error>> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "UPDATE progress SET unlocked_level = ?1 WHERE id = 1",
            [state.unlocked_level as i64],
        )?;
        // Full replace so removed tracks (e.g. after an undo) are deleted, not left orphaned.
        tx.execute("DELETE FROM track", [])?;
        for ((kanji_id, kind), t) in &state.tracks {
            tx.execute(
                "INSERT OR REPLACE INTO track (kanji_id, kind, card_json, introduced_at)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    kanji_id,
                    kind.as_str(),
                    serde_json::to_string(&t.card)?,
                    t.introduced_at.to_rfc3339(),
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The user's edited mnemonic override for a kanji, if any.
    pub fn user_mnemonic(&self, kanji_id: i64) -> Option<String> {
        self.conn
            .query_row(
                "SELECT story FROM user_mnemonic WHERE kanji_id = ?1",
                [kanji_id],
                |r| r.get::<_, String>(0),
            )
            .ok()
    }

    /// Set (or clear, when blank) the user's mnemonic override for a kanji.
    pub fn set_user_mnemonic(&mut self, kanji_id: i64, story: &str) -> Result<(), Box<dyn Error>> {
        if story.trim().is_empty() {
            self.conn
                .execute("DELETE FROM user_mnemonic WHERE kanji_id = ?1", [kanji_id])?;
        } else {
            self.conn.execute(
                "INSERT OR REPLACE INTO user_mnemonic (kanji_id, story, edited_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![kanji_id, story, Utc::now().to_rfc3339()],
            )?;
        }
        Ok(())
    }

    /// Persisted (new_per_day, daily_review_cap, desired_retention).
    pub fn load_settings(&self) -> rusqlite::Result<(usize, usize, f64)> {
        self.conn.query_row(
            "SELECT new_per_day, daily_review_cap, desired_retention FROM app_settings WHERE id = 1",
            [],
            |r| {
                Ok((
                    r.get::<_, i64>(0)? as usize,
                    r.get::<_, i64>(1)? as usize,
                    r.get::<_, f64>(2)?,
                ))
            },
        )
    }

    pub fn save_settings(
        &mut self,
        new_per_day: usize,
        daily_review_cap: usize,
        desired_retention: f64,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE app_settings SET new_per_day = ?1, daily_review_cap = ?2, desired_retention = ?3 WHERE id = 1",
            rusqlite::params![new_per_day as i64, daily_review_cap as i64, desired_retention],
        )?;
        Ok(())
    }

    /// Record a graded review (for streak + stats).
    pub fn log_review(
        &mut self,
        kanji_id: i64,
        kind: &str,
        rating: u8,
        ts: DateTime<Utc>,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO review_event (ts, kanji_id, kind, rating) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![ts.to_rfc3339(), kanji_id, kind, rating as i64],
        )?;
        Ok(())
    }

    /// Distinct dates (UTC) on which the user reviewed, ascending.
    pub fn study_dates(&self) -> rusqlite::Result<Vec<NaiveDate>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT date(ts) FROM review_event ORDER BY date(ts)")?;
        let dates = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .filter_map(Result::ok)
            .filter_map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
            .collect();
        Ok(dates)
    }

    /// (#reviews on `today`, #reviews total).
    pub fn review_counts(&self, today: NaiveDate) -> rusqlite::Result<(usize, usize)> {
        let today_s = today.format("%Y-%m-%d").to_string();
        let today_n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM review_event WHERE date(ts) = ?1",
            [today_s],
            |r| r.get(0),
        )?;
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM review_event", [], |r| r.get(0))?;
        Ok((today_n as usize, total as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mnemokanji_core::Scheduler;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_db() -> String {
        static N: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir()
            .join(format!(
                "mnemokanji-test-{}-{}.sqlite",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn state_round_trips() {
        let path = temp_db();
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        {
            let mut store = StateStore::open(&path).unwrap();
            let mut state = store.load_state().unwrap();
            assert_eq!(state.unlocked_level, 5, "fresh DB starts unlocked at N5");

            // Kanji ids are codepoints (一 = U+4E00); the legacy rowid range 1..=245 is
            // refused by the stale-write guard in open().
            state.tracks.insert(
                (19968, TrackKind::Comprehension),
                Track {
                    kanji_id: 19968,
                    kind: TrackKind::Comprehension,
                    card: Scheduler::new_card(now),
                    introduced_at: now,
                },
            );
            state.unlocked_level = 4;
            store.save_state(&state).unwrap();
        }

        let store = StateStore::open(&path).unwrap();
        let reloaded = store.load_state().unwrap();
        assert_eq!(reloaded.unlocked_level, 4);
        let t = reloaded
            .tracks
            .get(&(19968, TrackKind::Comprehension))
            .expect("track persisted");
        assert_eq!(t.introduced_at, now);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn migrations_are_valid() {
        assert!(migrations().validate().is_ok());
    }

    /// The frozen (old_id, glyph, new_id) triples from the v5 migration SQL.
    fn v5_mapping() -> Vec<(i64, char, i64)> {
        V5_KANJI_CODEPOINT_IDS
            .lines()
            .filter(|l| l.starts_with('(') && (l.ends_with("),") || l.ends_with(");")))
            .map(|l| {
                let inner = &l[1..l.rfind(')').unwrap()];
                let mut parts = inner.split(',');
                let old: i64 = parts.next().unwrap().parse().unwrap();
                let glyph_q = parts.next().unwrap();
                let glyph: char = glyph_q.trim_matches('\'').chars().next().unwrap();
                let new: i64 = parts.next().unwrap().parse().unwrap();
                (old, glyph, new)
            })
            .collect()
    }

    /// A user DB at schema v4 (pre-remap), as a v0.1.3–v0.3 build would have left it.
    /// (v0.1.0–v0.1.2 shipped only two migrations — that start point is covered by
    /// `v5_remaps_from_a_v0_1_era_db`.)
    fn v4_db(path: &str) -> Connection {
        let mut conn = Connection::open(path).unwrap();
        migrations().to_version(&mut conn, 4).unwrap();
        conn
    }

    #[test]
    fn v5_mapping_is_internally_consistent() {
        let map = v5_mapping();
        assert_eq!(map.len(), 245, "one row per kanji ever shipped with rowids");
        // Old ids are exactly 1..=245 in order (the shipped insertion order).
        for (i, (old, _, _)) in map.iter().enumerate() {
            assert_eq!(*old, i as i64 + 1);
        }
        // new_id is the glyph's codepoint — a transposed row cannot pass this.
        for (old, glyph, new) in &map {
            assert_eq!(
                *new, *glyph as i64,
                "row {old}: new_id must be the codepoint of {glyph}"
            );
        }
        let mut new_ids: Vec<i64> = map.iter().map(|(_, _, n)| *n).collect();
        new_ids.sort_unstable();
        new_ids.dedup();
        assert_eq!(new_ids.len(), 245, "codepoints must be unique");
        assert!(
            new_ids.first().unwrap() >= &0x3400,
            "codepoints are all CJK — disjoint from old rowids 1..=245"
        );
    }

    /// Pre-N3 only: the frozen mapping must equal the old rowid order reconstructed from
    /// the current seed (level ord, then glyph). DELETE this test when N3 reshuffles
    /// membership — the frozen mapping itself stays correct (it is historical fact).
    #[test]
    fn v5_mapping_matches_seed_reconstruction() {
        let seed = format!("{}/../../assets/seed.sqlite", env!("CARGO_MANIFEST_DIR"));
        if !std::path::Path::new(&seed).exists() {
            eprintln!("skipping: {seed} absent");
            return;
        }
        let conn = Connection::open(&seed).unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT glyph, id FROM kanji
                 ORDER BY (SELECT ord FROM level WHERE level.id = kanji.level_id), glyph",
            )
            .unwrap();
        let seed_rows: Vec<(String, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let map = v5_mapping();
        assert_eq!(seed_rows.len(), map.len());
        for ((glyph, id), (old, mglyph, new)) in seed_rows.iter().zip(&map) {
            assert_eq!(
                glyph,
                &mglyph.to_string(),
                "glyph at shipped position {old}"
            );
            assert_eq!(id, new, "seed id for {glyph} must be its codepoint");
        }
    }

    #[test]
    fn v5_remaps_old_rowids_to_codepoints() {
        let path = temp_db();
        {
            let conn = v4_db(&path);
            for (old, kind) in [
                (1, "comprehension"),
                (79, "production"),
                (80, "comprehension"),
            ] {
                conn.execute(
                    "INSERT INTO track (kanji_id, kind, card_json, introduced_at)
                     VALUES (?1, ?2, '{}', '2026-01-01T00:00:00Z')",
                    rusqlite::params![old, kind],
                )
                .unwrap();
            }
            // An id that is neither a shipped rowid nor a codepoint (hand-edited/dev DB).
            conn.execute(
                "INSERT INTO track (kanji_id, kind, card_json, introduced_at)
                 VALUES (999, 'comprehension', '{}', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO review_event (ts, kanji_id, kind, rating)
                 VALUES ('2026-01-01T00:00:00Z', 245, 'comprehension', 3)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO user_mnemonic (kanji_id, story, edited_at)
                 VALUES (1, 'my story', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        }

        let store = StateStore::open(&path).unwrap(); // runs v5
        let by_old: HashMap<i64, i64> = v5_mapping().iter().map(|(o, _, n)| (*o, *n)).collect();
        let ids: Vec<(i64, String)> = {
            let mut stmt = store
                .conn
                .prepare("SELECT kanji_id, kind FROM track ORDER BY kanji_id")
                .unwrap();
            let v = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
            v
        };
        assert_eq!(ids.len(), 4, "row count preserved");
        assert!(
            ids.contains(&(999, "comprehension".into())),
            "invalid id untouched"
        );
        assert!(
            ids.contains(&(19968, "comprehension".into())),
            "1 → 一 U+4E00"
        );
        assert!(ids.contains(&(by_old[&79], "production".into())));
        assert!(ids.contains(&(by_old[&80], "comprehension".into())));
        let ev: i64 = store
            .conn
            .query_row("SELECT kanji_id FROM review_event", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ev, by_old[&245]);
        assert_eq!(store.user_mnemonic(19968).as_deref(), Some("my story"));

        // Audit trail.
        assert_eq!(store.state_meta("v5_track_remapped").as_deref(), Some("3"));
        assert_eq!(store.state_meta("v5_track_invalid").as_deref(), Some("1"));
        assert_eq!(
            store.state_meta("v5_review_event_remapped").as_deref(),
            Some("1")
        );
        assert_eq!(
            store.state_meta("v5_review_event_invalid").as_deref(),
            Some("0")
        );
        assert_eq!(
            store.state_meta("v5_user_mnemonic_remapped").as_deref(),
            Some("1")
        );
        assert_eq!(
            store.state_meta("v5_user_mnemonic_invalid").as_deref(),
            Some("0")
        );

        let _ = std::fs::remove_file(&path);
    }

    /// The earliest installed base: v0.1.0–v0.1.2 shipped only two migrations, so their
    /// DBs sit at user_version 2 (no review_event table, no desired_retention column)
    /// with real data — the 2→5 jump in a single open must remap losslessly.
    #[test]
    fn v5_remaps_from_a_v0_1_era_db() {
        let path = temp_db();
        {
            let mut conn = Connection::open(&path).unwrap();
            migrations().to_version(&mut conn, 2).unwrap();
            conn.execute(
                "INSERT INTO track (kanji_id, kind, card_json, introduced_at)
                 VALUES (1, 'comprehension', '{}', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO user_mnemonic (kanji_id, story, edited_at)
                 VALUES (79, 'v0.1 story', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        }

        let store = StateStore::open(&path).unwrap(); // v3 + v4 + v5 in one open
        let by_old: HashMap<i64, i64> = v5_mapping().iter().map(|(o, _, n)| (*o, *n)).collect();
        let tid: i64 = store
            .conn
            .query_row("SELECT kanji_id FROM track", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tid, 19968, "1 → 一 U+4E00");
        assert_eq!(
            store.user_mnemonic(by_old[&79]).as_deref(),
            Some("v0.1 story")
        );
        assert_eq!(store.state_meta("v5_track_remapped").as_deref(), Some("1"));
        assert_eq!(
            store.state_meta("v5_review_event_remapped").as_deref(),
            Some("0"),
            "review_event was created empty by v3 in the same open"
        );

        let _ = std::fs::remove_file(&path);
    }

    /// A DB from a future app version (restored backup / downgrade) must produce the
    /// actionable "newer MnemoKanji" message, not the generic migration-failure one.
    #[test]
    fn open_reports_db_from_newer_app() {
        let path = temp_db();
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch("PRAGMA user_version = 99;").unwrap();
        }
        let err = StateStore::open(&path).err().expect("must refuse");
        assert!(err.to_string().contains("newer MnemoKanji"), "{err}");

        let _ = std::fs::remove_file(&path);
    }

    /// If a pre-v0.4 instance writes rowid-keyed rows AFTER the v5 remap (old app left
    /// running across an upgrade; save_state full-replaces `track`), open must refuse
    /// loudly instead of silently treating the un-remapped progress as orphans.
    #[test]
    fn open_refuses_stale_rowid_writes_after_migration() {
        let path = temp_db();
        {
            // Migrated (v5) DB...
            let store = StateStore::open(&path).unwrap();
            drop(store);
            // ...then an old binary full-replaces track with rowid-keyed rows.
            let conn = Connection::open(&path).unwrap();
            conn.execute(
                "INSERT INTO track (kanji_id, kind, card_json, introduced_at)
                 VALUES (42, 'comprehension', '{}', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        }
        let err = StateStore::open(&path).err().expect("must refuse");
        assert!(err.to_string().contains("older MnemoKanji"), "{err}");

        let _ = std::fs::remove_file(&path);
    }

    /// Mixed old/new ids for the same kanji+kind (dev-only artifact): the migration must
    /// fail closed — roll back entirely, leaving the DB at v4 with every row intact.
    #[test]
    fn v5_fails_closed_on_mixed_id_collision() {
        let path = temp_db();
        {
            let conn = v4_db(&path);
            for id in [1, 19968] {
                conn.execute(
                    "INSERT INTO track (kanji_id, kind, card_json, introduced_at)
                     VALUES (?1, 'comprehension', '{}', '2026-01-01T00:00:00Z')",
                    [id],
                )
                .unwrap();
            }
        }
        assert!(
            StateStore::open(&path).is_err(),
            "collision must abort the migration"
        );

        let conn = Connection::open(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 4, "rolled back to v4");
        let ids: Vec<i64> = {
            let mut stmt = conn
                .prepare("SELECT kanji_id FROM track ORDER BY kanji_id")
                .unwrap();
            let v = stmt
                .query_map([], |r| r.get(0))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
            v
        };
        assert_eq!(ids, vec![1, 19968], "both rows intact, unmodified");

        let _ = std::fs::remove_file(&path);
    }

    /// Orphan tracks (id not in the current content) must survive save/load round-trips:
    /// they are quarantined from scheduling, never deleted. Guards the full-replace
    /// invariant in save_state against future refactors that filter tracks before saving.
    #[test]
    fn orphan_track_survives_saves() {
        let path = temp_db();
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let mut store = StateStore::open(&path).unwrap();
        let mut state = store.load_state().unwrap();
        for id in [999, 19968] {
            state.tracks.insert(
                (id, TrackKind::Comprehension),
                Track {
                    kanji_id: id,
                    kind: TrackKind::Comprehension,
                    card: Scheduler::new_card(now),
                    introduced_at: now,
                },
            );
        }
        store.save_state(&state).unwrap();

        // A later session mutates unrelated state and saves again.
        let mut store = StateStore::open(&path).unwrap();
        let mut state = store.load_state().unwrap();
        state.unlocked_level = 4;
        store.save_state(&state).unwrap();

        let reloaded = StateStore::open(&path).unwrap().load_state().unwrap();
        assert!(
            reloaded
                .tracks
                .contains_key(&(999, TrackKind::Comprehension)),
            "orphan track must persist across saves"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn review_logging_counts_and_dates() {
        let path = temp_db();
        let mut store = StateStore::open(&path).unwrap();
        let base = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        store.log_review(1, "comprehension", 3, base).unwrap();
        store.log_review(2, "comprehension", 4, base).unwrap();
        store
            .log_review(1, "comprehension", 3, base + chrono::Duration::days(1))
            .unwrap();

        assert_eq!(store.study_dates().unwrap().len(), 2);
        let (today_n, total_n) = store.review_counts(base.date_naive()).unwrap();
        assert_eq!(today_n, 2);
        assert_eq!(total_n, 3);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn db_file_backup_round_trips() {
        let path = temp_db();
        let backup = temp_db();
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        {
            let mut store = StateStore::open(&path).unwrap();
            let mut state = store.load_state().unwrap();
            state.unlocked_level = 3;
            state.tracks.insert(
                (19975, TrackKind::Comprehension),
                Track {
                    kanji_id: 19975, // 万 — ids are codepoints, never legacy rowids
                    kind: TrackKind::Comprehension,
                    card: Scheduler::new_card(now),
                    introduced_at: now,
                },
            );
            store.save_state(&state).unwrap();
            store.save_settings(15, 80, 0.85).unwrap();
        }
        // "Export" = copy the DB file; "import" = open the copy.
        std::fs::copy(&path, &backup).unwrap();
        let restored = StateStore::open(&backup).unwrap();
        let st = restored.load_state().unwrap();
        assert_eq!(st.unlocked_level, 3);
        assert!(st.tracks.contains_key(&(19975, TrackKind::Comprehension)));
        assert_eq!(restored.load_settings().unwrap(), (15, 80, 0.85));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&backup);
    }

    #[test]
    fn in_memory_store_opens() {
        // Import opens an in-memory store to release the lock on the destination file.
        assert!(StateStore::open(":memory:").is_ok());
    }
}
