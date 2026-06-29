//! Read-only access to the bundled seed DB (`assets/seed.sqlite`) → core [`ContentView`].

use std::collections::HashMap;

use mnemokanji_core::{ContentView, KanjiMeta};
use rusqlite::{Connection, OpenFlags};

pub struct ContentRepo {
    conn: Connection,
}

impl ContentRepo {
    /// Open the seed DB read-only.
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Ok(Self { conn })
    }

    /// Build the scheduling view: every kanji with its level, intro order, and kanji-component
    /// prerequisites (a component that is itself a learned kanji must be introduced first).
    pub fn content_view(&self) -> rusqlite::Result<ContentView> {
        // Prerequisites: kanji_id -> [component-kanji ids].
        let mut prereqs: HashMap<i64, Vec<i64>> = HashMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT kc.kanji_id, k2.id
             FROM kanji_component kc
             JOIN component c ON c.id = kc.component_id AND c.is_kanji = 1
             JOIN kanji k2 ON k2.glyph = c.glyph
             WHERE k2.id <> kc.kanji_id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
        for row in rows {
            let (kid, prereq) = row?;
            prereqs.entry(kid).or_default().push(prereq);
        }

        let mut stmt = self.conn.prepare(
            "SELECT k.id, l.jlpt, k.intro_rank
             FROM kanji k JOIN level l ON l.id = k.level_id
             ORDER BY l.ord, k.intro_rank",
        )?;
        let kanji = stmt
            .query_map([], |r| {
                let id: i64 = r.get(0)?;
                let jlpt: String = r.get(1)?;
                let intro_rank: i64 = r.get::<_, Option<i64>>(2)?.unwrap_or(0);
                Ok(KanjiMeta {
                    id,
                    level: level_num(&jlpt),
                    intro_rank,
                    prereq_kanji: Vec::new(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .map(|mut k| {
                k.prereq_kanji = prereqs.remove(&k.id).unwrap_or_default();
                k
            })
            .collect();

        Ok(ContentView { kanji })
    }
}

/// Map a JLPT label to its level number (5 = N5, learned first).
fn level_num(jlpt: &str) -> u8 {
    match jlpt {
        "N5" => 5,
        "N4" => 4,
        "N3" => 3,
        "N2" => 2,
        "N1" => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Duration, Utc};
    use mnemokanji_core::{Engine, Rating, Settings, StudyState, TrackKind};

    fn seed_path() -> String {
        format!("{}/../../assets/seed.sqlite", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn content_view_from_real_seed_drives_engine() {
        let path = seed_path();
        if !std::path::Path::new(&path).exists() {
            eprintln!(
                "skipping: {path} absent (run scripts/fetch-sources.sh + cargo run -p mnemokanji-content)"
            );
            return;
        }
        let repo = ContentRepo::open(&path).unwrap();
        let content = repo.content_view().unwrap();

        let n5 = content.kanji.iter().filter(|k| k.level == 5).count();
        assert_eq!(n5, 79, "N5 should have 79 kanji");
        assert!(
            content.kanji.iter().any(|k| !k.prereq_kanji.is_empty()),
            "expected some kanji-component prerequisites in N5"
        );

        // Drive the engine on the real curriculum for a couple of weeks.
        let engine = Engine::new(
            &content,
            Settings {
                new_per_day: 10,
                ..Default::default()
            },
        );
        let mut state = StudyState {
            unlocked_level: 5,
            ..Default::default()
        };
        let mut now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        for _ in 0..15 {
            engine.introduce_new(&mut state, now);
            for (kid, kind) in engine.due_items(&state, now) {
                engine.grade(&mut state, kid, kind, &[Rating::Good], now);
            }
            now += Duration::days(1);
        }

        // All 79 N5 kanji introduced within the budget (topo order keeps prereqs early).
        let introduced = state
            .tracks
            .keys()
            .filter(|(_, k)| *k == TrackKind::Comprehension)
            .count();
        assert_eq!(
            introduced, 79,
            "all N5 kanji should be introduced in 15 days"
        );

        // Ordering invariant: every introduced kanji's prerequisites were also introduced.
        for (id, _) in state
            .tracks
            .keys()
            .filter(|(_, k)| *k == TrackKind::Comprehension)
        {
            let meta = content.kanji.iter().find(|k| k.id == *id).unwrap();
            for p in &meta.prereq_kanji {
                assert!(
                    state.tracks.contains_key(&(*p, TrackKind::Comprehension)),
                    "prerequisite {p} of kanji {id} must be introduced"
                );
            }
        }
    }
}
