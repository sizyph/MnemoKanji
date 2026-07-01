//! Writable per-user state store: FSRS tracks + level progress in a separate SQLite DB.

use std::collections::HashMap;
use std::error::Error;

use chrono::{DateTime, NaiveDate, Utc};
use mnemokanji_core::{Card, StudyState, Track, TrackKind};
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

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
    ])
}

pub struct StateStore {
    conn: Connection,
}

impl StateStore {
    /// Open (creating + migrating to latest) the user-state DB at `path`.
    pub fn open(path: &str) -> Result<Self, Box<dyn Error>> {
        let mut conn = Connection::open(path)?;
        migrations().to_latest(&mut conn)?;
        Ok(Self { conn })
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

            state.tracks.insert(
                (1, TrackKind::Comprehension),
                Track {
                    kanji_id: 1,
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
            .get(&(1, TrackKind::Comprehension))
            .expect("track persisted");
        assert_eq!(t.introduced_at, now);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn migrations_are_valid() {
        assert!(migrations().validate().is_ok());
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
                (7, TrackKind::Comprehension),
                Track {
                    kanji_id: 7,
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
        assert!(st.tracks.contains_key(&(7, TrackKind::Comprehension)));
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
