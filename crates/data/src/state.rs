//! Writable per-user state store: FSRS tracks + level progress in a separate SQLite DB.

use std::collections::HashMap;
use std::error::Error;

use chrono::{DateTime, Utc};
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

    /// Persisted (new_per_day, daily_review_cap).
    pub fn load_settings(&self) -> rusqlite::Result<(usize, usize)> {
        self.conn.query_row(
            "SELECT new_per_day, daily_review_cap FROM app_settings WHERE id = 1",
            [],
            |r| Ok((r.get::<_, i64>(0)? as usize, r.get::<_, i64>(1)? as usize)),
        )
    }

    pub fn save_settings(
        &mut self,
        new_per_day: usize,
        daily_review_cap: usize,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE app_settings SET new_per_day = ?1, daily_review_cap = ?2 WHERE id = 1",
            rusqlite::params![new_per_day as i64, daily_review_cap as i64],
        )?;
        Ok(())
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
}
