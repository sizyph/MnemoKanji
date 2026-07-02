//! Read-only access to the bundled seed DB (`assets/seed.sqlite`) → core [`ContentView`].

use std::collections::HashMap;

use mnemokanji_core::{ContentView, KanjiMeta};
use rusqlite::{Connection, OpenFlags};

/// One reading of a kanji, for display.
#[derive(Clone, Debug)]
pub struct Reading {
    pub kind: String, // "on" | "kun"
    pub reading: String,
    pub is_dominant: bool,
}

#[derive(Clone, Debug)]
pub struct VocabItem {
    pub surface: String,
    pub reading: String,
    pub gloss: String,
}

#[derive(Clone, Debug)]
pub struct SentenceItem {
    pub jp: String,
    pub en: String,
}

#[derive(Clone, Debug)]
pub struct ComponentItem {
    pub glyph: String,
    pub actor: Option<String>,
    pub is_kanji: bool,
}

/// A compact row for the browse grid.
#[derive(Clone, Debug)]
pub struct BrowseItem {
    pub id: i64,
    pub glyph: String,
    pub keyword: String,
    /// JLPT label of the kanji's level (e.g. "N5"), for grouping the grid.
    pub level: String,
}

/// Everything needed to render a kanji in review or on its detail page.
#[derive(Clone, Debug)]
pub struct KanjiDetail {
    pub id: i64,
    pub glyph: String,
    pub keyword: String,
    pub stroke_count: Option<i64>,
    pub meanings: Vec<String>,
    pub readings: Vec<Reading>,
    pub vocab: Vec<VocabItem>,
    pub sentences: Vec<SentenceItem>,
    pub mnemonic: Option<String>,
    pub stroke_paths: Vec<String>,
    pub components: Vec<ComponentItem>,
}

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
        // Prerequisites: kanji_id -> [component-kanji ids]. A component that is itself a kanji is a
        // prerequisite ONLY if it belongs to the same or an earlier level (ord <= this kanji's ord).
        // A component that happens to be a LATER-level kanji (e.g. an N4 kanji appearing inside an
        // N5 glyph) must NOT gate — it is learned later, so it is treated as a just-in-time radical,
        // not a prerequisite (otherwise the earlier-level kanji could never be introduced).
        let mut prereqs: HashMap<i64, Vec<i64>> = HashMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT kc.kanji_id, k2.id
             FROM kanji_component kc
             JOIN component c ON c.id = kc.component_id AND c.is_kanji = 1
             JOIN kanji k2 ON k2.glyph = c.glyph
             JOIN kanji k1 ON k1.id = kc.kanji_id
             JOIN level l1 ON l1.id = k1.level_id
             JOIN level l2 ON l2.id = k2.level_id
             WHERE k2.id <> kc.kanji_id AND l2.ord <= l1.ord",
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

    /// Compact list of a level's kanji in learning order (for the browse grid).
    pub fn browse(&self, jlpt: &str) -> rusqlite::Result<Vec<BrowseItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT k.id, k.glyph, COALESCE(k.primary_keyword, ''), l.jlpt
             FROM kanji k JOIN level l ON l.id = k.level_id
             WHERE l.jlpt = ?1
             ORDER BY k.intro_rank",
        )?;
        let out = stmt
            .query_map([jlpt], |r| {
                Ok(BrowseItem {
                    id: r.get(0)?,
                    glyph: r.get(1)?,
                    keyword: r.get(2)?,
                    level: r.get(3)?,
                })
            })?
            .collect();
        out
    }

    /// Every built level's kanji, in learning order (level `ord`, then `intro_rank`) — the browse
    /// grid across all levels (N5, N4, …), grouped by `level`.
    pub fn browse_all(&self) -> rusqlite::Result<Vec<BrowseItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT k.id, k.glyph, COALESCE(k.primary_keyword, ''), l.jlpt
             FROM kanji k JOIN level l ON l.id = k.level_id
             ORDER BY l.ord, k.intro_rank",
        )?;
        let out = stmt
            .query_map([], |r| {
                Ok(BrowseItem {
                    id: r.get(0)?,
                    glyph: r.get(1)?,
                    keyword: r.get(2)?,
                    level: r.get(3)?,
                })
            })?
            .collect();
        out
    }

    /// Load the full presentation detail for one kanji.
    pub fn kanji_detail(&self, id: i64) -> rusqlite::Result<KanjiDetail> {
        let (glyph, keyword, stroke_count): (String, String, Option<i64>) = self.conn.query_row(
            "SELECT glyph, COALESCE(primary_keyword, ''), stroke_count FROM kanji WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;

        let meanings = self.collect(
            "SELECT gloss FROM meaning WHERE kanji_id = ?1 ORDER BY sense_order",
            id,
            |r| r.get::<_, String>(0),
        )?;

        let mut stmt = self.conn.prepare(
            "SELECT kind, reading, is_dominant FROM reading WHERE kanji_id = ?1
             ORDER BY is_dominant DESC, kind, id",
        )?;
        let readings = stmt
            .query_map([id], |r| {
                Ok(Reading {
                    kind: r.get(0)?,
                    reading: r.get(1)?,
                    is_dominant: r.get::<_, i64>(2)? != 0,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut stmt = self.conn.prepare(
            "SELECT v.surface, v.reading, v.gloss
             FROM vocab_kanji vk JOIN vocab v ON v.id = vk.vocab_id
             WHERE vk.kanji_id = ?1 LIMIT 6",
        )?;
        let vocab = stmt
            .query_map([id], |r| {
                Ok(VocabItem {
                    surface: r.get(0)?,
                    reading: r.get(1)?,
                    gloss: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT s.jp, s.en
             FROM vocab_kanji vk JOIN vocab_sentence vs ON vs.vocab_id = vk.vocab_id
             JOIN sentence s ON s.id = vs.sentence_id
             WHERE vk.kanji_id = ?1 LIMIT 4",
        )?;
        let sentences = stmt
            .query_map([id], |r| {
                Ok(SentenceItem {
                    jp: r.get(0)?,
                    en: r.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mnemonic = self
            .conn
            .query_row(
                "SELECT story FROM mnemonic WHERE kanji_id = ?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .ok();

        let stroke_paths = self.collect(
            "SELECT path FROM stroke WHERE kanji_id = ?1 ORDER BY ord",
            id,
            |r| r.get::<_, String>(0),
        )?;

        let mut stmt = self.conn.prepare(
            "SELECT c.glyph, ca.actor_name, c.is_kanji
             FROM kanji_component kc JOIN component c ON c.id = kc.component_id
             LEFT JOIN component_actor ca ON ca.component_id = c.id
             WHERE kc.kanji_id = ?1",
        )?;
        let components = stmt
            .query_map([id], |r| {
                Ok(ComponentItem {
                    glyph: r.get(0)?,
                    actor: r.get::<_, Option<String>>(1)?,
                    is_kanji: r.get::<_, i64>(2)? != 0,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(KanjiDetail {
            id,
            glyph,
            keyword,
            stroke_count,
            meanings,
            readings,
            vocab,
            sentences,
            mnemonic,
            stroke_paths,
            components,
        })
    }

    fn collect<T>(
        &self,
        sql: &str,
        id: i64,
        f: impl Fn(&rusqlite::Row) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<Vec<T>> {
        let mut stmt = self.conn.prepare(sql)?;
        let out = stmt.query_map([id], |r| f(r))?.collect();
        out
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
    fn kanji_detail_is_complete_for_all_n5() {
        let path = seed_path();
        if !std::path::Path::new(&path).exists() {
            eprintln!("skipping: {path} absent");
            return;
        }
        let repo = ContentRepo::open(&path).unwrap();
        let content = repo.content_view().unwrap();
        let n5: Vec<i64> = content
            .kanji
            .iter()
            .filter(|k| k.level == 5)
            .map(|k| k.id)
            .collect();
        assert_eq!(n5.len(), 79);

        for id in n5 {
            let d = repo.kanji_detail(id).unwrap();
            assert!(!d.glyph.is_empty());
            assert!(!d.keyword.is_empty(), "{} has no keyword", d.glyph);
            assert!(!d.readings.is_empty(), "{} has no readings", d.glyph);
            assert_eq!(
                d.readings.iter().filter(|r| r.is_dominant).count(),
                1,
                "{} should have exactly one dominant reading",
                d.glyph
            );
            assert!(
                !d.stroke_paths.is_empty(),
                "{} has no stroke paths",
                d.glyph
            );
            assert!(d.mnemonic.is_some(), "{} has no mnemonic", d.glyph);
            assert!(!d.vocab.is_empty(), "{} has no vocab", d.glyph);
            assert!(!d.sentences.is_empty(), "{} has no sentences", d.glyph);
            assert!(!d.components.is_empty(), "{} has no components", d.glyph);
        }
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
