//! MnemoKanji content pipeline — offline dataset builder (not shipped in the app).
//!
//! Slice 1: build `assets/seed.sqlite` for JLPT **N5** from openly-licensed sources —
//! kanji core (KANJIDIC via davidluzgouveia/kanji-data, MIT), component decomposition
//! (kradfile-u, CC BY-SA), and a frequency-weighted topological learning order.
//!
//! Later slices add: phonetic table, dominant-reading derivation, vocabulary, sentences,
//! component/reading actors, and generated mnemonics. See `docs/08-DATASET.md`.
//!
//! Run from the repo root: `cargo run -p mnemokanji-content`
//! (expects `data/sources/kanji-data.json` and `data/sources/kradfile-u`; see scripts/fetch-sources.sh).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::Path;

use rusqlite::Connection;
use serde_json::Value;

mod sentence;
mod stroke;
mod vocab;

const SOURCE_JSON: &str = "data/sources/kanji-data.json";
const SOURCE_KRAD: &str = "data/sources/kradfile-u";
const OUT_DB: &str = "assets/seed.sqlite";
const SCHEMA: &str = include_str!("schema.sql");

// JLPT levels to build, in learning order: (jlpt_new value, label, ord/level_id). N5 is learned
// first (ord 1). Add (4, "N4", 2) once N4 authored content (keywords, actors, mnemonics) lands —
// the derived content (vocab, sentences, strokes, decomposition, learning order) needs no new code.
const LEVELS: &[(i64, &str, i64)] = &[(5, "N5", 1)];

// Reading actors are keyed by on'yomi sound and shared across every level (one persona per sound).
const AUTH_READ: &str = "data/authored/n5-reading-actors.json";

/// Per-level authored file paths (`data/authored/{n5|n4|…}-*.json`). Every file is optional:
/// absent => that facet is derived-only for the level. Keyed off the lowercased level label.
struct LevelAuthored {
    keywords: String,
    components: String,
    mnemonics: String,
    dominant: String,
    vglosses: String,
}

fn level_authored(label: &str) -> LevelAuthored {
    let p = label.to_lowercase();
    LevelAuthored {
        keywords: format!("data/authored/{p}-keywords.json"),
        components: format!("data/authored/{p}-component-actors.json"),
        mnemonics: format!("data/authored/{p}-mnemonics.json"),
        dominant: format!("data/authored/{p}-dominant-readings.json"),
        vglosses: format!("data/authored/{p}-vocab-glosses.json"),
    }
}

/// A kanji as assembled from the sources, before DB insertion.
struct KanjiRow {
    glyph: String,
    /// FK into the `level` table (== the level's `ord`).
    level_id: i64,
    strokes: Option<i64>,
    freq: Option<i64>,
    keyword: String,
    meanings: Vec<String>,
    on: Vec<String>,
    kun: Vec<String>,
    components: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let level_labels: Vec<&str> = LEVELS.iter().map(|(_, l, _)| *l).collect();
    println!(
        "mnemokanji-content: building seed for {}\n",
        level_labels.join(", ")
    );

    let kanji_data = load_json(SOURCE_JSON)?;
    let krad = load_kradfile(SOURCE_KRAD)?;

    // --- Assemble the kanji set for every configured level, grouped by level (glyph-sorted). ---
    let obj = kanji_data
        .as_object()
        .ok_or("kanji-data.json is not a JSON object")?;
    // (jlpt, label, ord, glyphs) per level, in learning order.
    let per_level: Vec<(i64, &str, i64, Vec<String>)> = LEVELS
        .iter()
        .map(|&(jlpt, label, ord)| {
            let mut glyphs: Vec<String> = obj
                .iter()
                .filter(|(_, v)| v.get("jlpt_new").and_then(Value::as_i64) == Some(jlpt))
                .map(|(k, _)| k.clone())
                .collect();
            glyphs.sort();
            (jlpt, label, ord, glyphs)
        })
        .collect();

    let selected: Vec<String> = per_level
        .iter()
        .flat_map(|(_, _, _, g)| g.iter().cloned())
        .collect();
    let selected_set: HashSet<&str> = selected.iter().map(String::as_str).collect();
    let selected_owned: HashSet<String> = selected.iter().cloned().collect();

    let krad_ref = &krad;
    let rows: Vec<KanjiRow> = per_level
        .iter()
        .flat_map(|&(_, _, ord, ref glyphs)| {
            glyphs.iter().map(move |g| {
                let info = &obj[g];
                let meanings = str_array(info, "meanings");
                KanjiRow {
                    glyph: g.clone(),
                    level_id: ord,
                    strokes: info.get("strokes").and_then(Value::as_i64),
                    freq: info.get("freq").and_then(Value::as_i64),
                    keyword: normalize_keyword(meanings.first().map(String::as_str).unwrap_or("")),
                    meanings,
                    on: str_array(info, "readings_on"),
                    kun: str_array(info, "readings_kun"),
                    components: krad_ref.get(g).cloned().unwrap_or_default(),
                }
            })
        })
        .collect();

    // --- Slice 3: dominant readings + in-context vocabulary (optional sources). ---
    let mut stored: vocab::StoredReadings = HashMap::new();
    for r in &rows {
        let mut rs = Vec::new();
        for x in &r.on {
            rs.push(("on".to_string(), x.clone()));
        }
        for x in &r.kun {
            rs.push(("kun".to_string(), x.clone()));
        }
        stored.insert(r.glyph.clone(), rs);
    }
    let kanji_levels: HashMap<String, i64> = obj
        .iter()
        .filter_map(|(g, v)| {
            v.get("jlpt_new")
                .and_then(Value::as_i64)
                .map(|l| (g.clone(), l))
        })
        .collect();
    let vocab_data = vocab::build(&selected_owned, &stored, &kanji_levels)?;

    // Example sentences for the selected vocab (optional source).
    let wanted_surfaces: HashSet<String> = vocab_data
        .as_ref()
        .map(|vd| vd.vocab.iter().map(|v| v.surface.clone()).collect())
        .unwrap_or_default();
    let sentence_map = sentence::build(&wanted_surfaces)?;

    // Stroke-order paths from KanjiVG (optional source).
    let stroke_map = stroke::build(&selected);

    // --- Frequency-weighted topological order, computed WITHIN each level (a component that is a
    // lower-level kanji is already learned, so it never gates the current level). ---
    let mut intro_rank: HashMap<String, i64> = HashMap::new();
    for (_, _, _, glyphs) in &per_level {
        let scope: HashSet<&str> = glyphs.iter().map(String::as_str).collect();
        for (i, g) in topological_order(&rows, &scope).into_iter().enumerate() {
            intro_rank.insert(g, i as i64);
        }
    }

    // --- Build the SQLite seed. ---
    if let Some(parent) = Path::new(OUT_DB).parent() {
        fs::create_dir_all(parent)?;
    }
    let _ = fs::remove_file(OUT_DB);
    let mut conn = Connection::open(OUT_DB)?;
    conn.execute_batch(SCHEMA)?;

    let tx = conn.transaction()?;
    for (_, label, ord, glyphs) in &per_level {
        tx.execute(
            "INSERT INTO level (id, jlpt, ord, kanji_count) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![ord, label, ord, glyphs.len() as i64],
        )?;
    }

    // Distinct components across all selected kanji.
    let mut comp_glyphs: BTreeSet<&str> = BTreeSet::new();
    for r in &rows {
        for c in &r.components {
            comp_glyphs.insert(c.as_str());
        }
    }
    let mut comp_id: HashMap<&str, i64> = HashMap::new();
    for c in &comp_glyphs {
        tx.execute(
            "INSERT INTO component (glyph, is_kanji) VALUES (?1, ?2)",
            rusqlite::params![c, selected_set.contains(c) as i64],
        )?;
        comp_id.insert(c, tx.last_insert_rowid());
    }

    let mut kanji_id: HashMap<&str, i64> = HashMap::new();
    for r in &rows {
        tx.execute(
            "INSERT INTO kanji (glyph, level_id, stroke_count, freq, primary_keyword, intro_rank)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                r.glyph,
                r.level_id,
                r.strokes,
                r.freq,
                r.keyword,
                intro_rank[r.glyph.as_str()]
            ],
        )?;
        let kid = tx.last_insert_rowid();
        kanji_id.insert(r.glyph.as_str(), kid);

        for reading in &r.on {
            tx.execute(
                "INSERT INTO reading (kanji_id, kind, reading) VALUES (?1, 'on', ?2)",
                rusqlite::params![kid, reading],
            )?;
        }
        for reading in &r.kun {
            tx.execute(
                "INSERT INTO reading (kanji_id, kind, reading) VALUES (?1, 'kun', ?2)",
                rusqlite::params![kid, reading],
            )?;
        }
        for (i, gloss) in r.meanings.iter().enumerate() {
            tx.execute(
                "INSERT INTO meaning (kanji_id, gloss, is_primary, sense_order) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![kid, gloss, (i == 0) as i64, i as i64],
            )?;
        }
        for c in &r.components {
            tx.execute(
                "INSERT OR IGNORE INTO kanji_component (kanji_id, component_id, role)
                 VALUES (?1, ?2, 'semantic')",
                rusqlite::params![kid, comp_id[c.as_str()]],
            )?;
        }
    }

    let authored = load_authored(&tx, &kanji_id, &comp_id)?;
    let vstats = insert_vocab(&tx, &kanji_id, &rows, vocab_data.as_ref())?;
    let sentence_count = insert_sentences(&tx, sentence_map.as_ref())?;
    let gloss_fixes = apply_gloss_overrides(&tx)?;

    // Slice 5: stroke-order paths.
    let mut stroke_count = 0;
    for (glyph, paths) in &stroke_map {
        if let Some(&kid) = kanji_id.get(glyph.as_str()) {
            for (i, d) in paths.iter().enumerate() {
                tx.execute(
                    "INSERT OR IGNORE INTO stroke (kanji_id, ord, path) VALUES (?1, ?2, ?3)",
                    rusqlite::params![kid, i as i64, d],
                )?;
                stroke_count += 1;
            }
        }
    }

    for (k, v) in build_meta(&rows) {
        tx.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)",
            rusqlite::params![k, v],
        )?;
    }
    for (k, v) in [
        ("dominant_derived", vstats.0.to_string()),
        ("dominant_fallback", vstats.1.to_string()),
        ("vocab_count", vstats.2.to_string()),
        ("sentence_count", sentence_count.to_string()),
        ("stroke_count", stroke_count.to_string()),
    ] {
        tx.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            rusqlite::params![k, v],
        )?;
    }
    tx.commit()?;

    println!(
        "Authored content loaded: {} keyword overrides, {} component actors, {} reading actors, {} mnemonics",
        authored.0, authored.1, authored.2, authored.3
    );
    println!(
        "Slice 3: {} dominant readings derived ({} fallback), {} vocab words",
        vstats.0, vstats.1, vstats.2
    );
    println!("Slice 4: {sentence_count} vocab-sentence links");
    println!("Slice 5: {stroke_count} stroke paths");
    println!("Reviewer gloss fixes applied: {gloss_fixes}");
    verify(&conn, &rows)?;
    println!("\nWrote {OUT_DB}");
    Ok(())
}

/// Normalize a KANJIDIC gloss into a concise primary keyword: drop parentheticals,
/// take the first comma-segment, trim, lowercase. (Never an invented keyword — docs/08 §2.1.)
fn normalize_keyword(raw: &str) -> String {
    let mut s = String::new();
    let mut depth = 0u32;
    for ch in raw.chars() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => break,
            _ if depth == 0 => s.push(ch),
            _ => {}
        }
    }
    s.trim().to_lowercase()
}

fn str_array(info: &Value, key: &str) -> Vec<String> {
    info.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn load_json(path: &str) -> Result<Value, Box<dyn Error>> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {path}: {e} (run scripts/fetch-sources.sh)"))?;
    Ok(serde_json::from_str(&text)?)
}

/// Parse kradfile-u: `kanji : comp1 comp2 ...` (UTF-8, '#' comments).
fn load_kradfile(path: &str) -> Result<HashMap<String, Vec<String>>, Box<dyn Error>> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {path}: {e} (run scripts/fetch-sources.sh)"))?;
    let mut map = HashMap::new();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        if let Some((k, rest)) = line.split_once(" : ") {
            let comps: Vec<String> = rest.split_whitespace().map(str::to_string).collect();
            map.insert(k.trim().to_string(), comps);
        }
    }
    Ok(map)
}

/// Frequency-weighted topological sort (Kahn) over the glyphs in `scope` (one JLPT level): among
/// kanji whose same-level-kanji components are all already placed, emit the most frequent next
/// (lower freq rank = more frequent; ties by fewer strokes, then codepoint). Components outside
/// `scope` (lower-level kanji, bare radicals) are assumed known just-in-time, so they don't gate.
/// See docs/02 §H. Returns the in-scope glyphs in learning order.
fn topological_order(rows: &[KanjiRow], scope: &HashSet<&str>) -> Vec<String> {
    let info: HashMap<&str, &KanjiRow> = rows.iter().map(|r| (r.glyph.as_str(), r)).collect();
    let glyphs: Vec<&str> = {
        let mut v: Vec<&str> = rows
            .iter()
            .map(|r| r.glyph.as_str())
            .filter(|g| scope.contains(g))
            .collect();
        v.sort_unstable();
        v
    };

    // Edge c -> k when component c is itself an in-scope kanji used in k (a prerequisite of k).
    let mut indeg: BTreeMap<&str, usize> = glyphs.iter().map(|&g| (g, 0)).collect();
    let mut succ: HashMap<&str, Vec<&str>> = HashMap::new();
    for &k in &glyphs {
        let mut prereqs: BTreeSet<&str> = BTreeSet::new();
        for c in &info[k].components {
            let cs = c.as_str();
            if cs != k && scope.contains(cs) {
                prereqs.insert(cs);
            }
        }
        for p in prereqs {
            succ.entry(p).or_default().push(k);
            *indeg.get_mut(k).unwrap() += 1;
        }
    }

    let rank = |g: &str| -> (i64, i64, u32) {
        let r = info[g];
        (
            r.freq.unwrap_or(i64::MAX),
            r.strokes.unwrap_or(i64::MAX),
            g.chars().next().map(|c| c as u32).unwrap_or(u32::MAX),
        )
    };

    let mut ready: Vec<&str> = glyphs.iter().copied().filter(|g| indeg[g] == 0).collect();
    let mut order: Vec<String> = Vec::with_capacity(glyphs.len());
    while !ready.is_empty() {
        // Pick the most frequent ready kanji.
        let (bi, _) = ready
            .iter()
            .enumerate()
            .min_by(|a, b| rank(a.1).cmp(&rank(b.1)))
            .unwrap();
        let chosen = ready.swap_remove(bi);
        order.push(chosen.to_string());
        if let Some(children) = succ.get(chosen) {
            for &child in children {
                let d = indeg.get_mut(child).unwrap();
                *d -= 1;
                if *d == 0 {
                    ready.push(child);
                }
            }
        }
    }
    // Cycle guard (kradfile decompositions shouldn't cycle, but never drop kanji).
    if order.len() < glyphs.len() {
        let placed: HashSet<&str> = order.iter().map(String::as_str).collect();
        let mut rest: Vec<&str> = glyphs
            .iter()
            .copied()
            .filter(|g| !placed.contains(g))
            .collect();
        rest.sort_by_key(|a| rank(a));
        order.extend(rest.into_iter().map(str::to_string));
    }
    order
}

/// Read an optional authored JSON file (returns None if it doesn't exist yet).
fn read_optional(path: &str) -> Result<Option<Value>, Box<dyn Error>> {
    if !Path::new(path).exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&fs::read_to_string(path)?)?))
}

/// Load authored content (keyword overrides, actors, mnemonics) from `data/authored/*.json`.
/// Returns counts (keywords, component_actors, reading_actors, mnemonics).
fn load_authored(
    tx: &rusqlite::Transaction,
    kanji_id: &HashMap<&str, i64>,
    comp_id: &HashMap<&str, i64>,
) -> Result<(usize, usize, usize, usize), Box<dyn Error>> {
    let (mut kw, mut ca, mut ra, mut mn) = (0, 0, 0, 0);

    // Reading actors are shared across all levels (keyed by on'yomi sound), loaded once.
    if let Some(v) = read_optional(AUTH_READ)? {
        for e in v.as_array().into_iter().flatten() {
            let r = e.get("reading").and_then(Value::as_str);
            let name = e.get("actor_name").and_then(Value::as_str);
            let vl = e
                .get("vowel_length")
                .and_then(Value::as_str)
                .unwrap_or("long");
            let note = e.get("note").and_then(Value::as_str);
            if let (Some(r), Some(name)) = (r, name) {
                tx.execute(
                    "INSERT OR REPLACE INTO reading_actor (reading, vowel_length, actor_name, note) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![r, vl, name, note],
                )?;
                ra += 1;
            }
        }
    }

    // Keyword overrides, component actors, and mnemonics are authored per level.
    for (_, label, _) in LEVELS {
        let ap = level_authored(label);

        if let Some(v) = read_optional(&ap.keywords)? {
            for e in v.as_array().into_iter().flatten() {
                let (g, k) = (
                    e.get("glyph").and_then(Value::as_str),
                    e.get("keyword").and_then(Value::as_str),
                );
                if let (Some(g), Some(k)) = (g, k) {
                    if let Some(&id) = kanji_id.get(g) {
                        tx.execute(
                            "UPDATE kanji SET primary_keyword = ?1 WHERE id = ?2",
                            rusqlite::params![k, id],
                        )?;
                        kw += 1;
                    }
                }
            }
        }

        if let Some(v) = read_optional(&ap.components)? {
            for e in v.as_array().into_iter().flatten() {
                let g = e.get("component").and_then(Value::as_str);
                let name = e.get("actor_name").and_then(Value::as_str);
                let img = e.get("image").and_then(Value::as_str).unwrap_or("");
                if let (Some(g), Some(name)) = (g, name) {
                    if let Some(&cid) = comp_id.get(g) {
                        tx.execute(
                            "INSERT OR REPLACE INTO component_actor (component_id, actor_name, image) VALUES (?1, ?2, ?3)",
                            rusqlite::params![cid, name, img],
                        )?;
                        ca += 1;
                    }
                }
            }
        }

        if let Some(v) = read_optional(&ap.mnemonics)? {
            for e in v.as_array().into_iter().flatten() {
                let g = e.get("glyph").and_then(Value::as_str);
                let story = e.get("story").and_then(Value::as_str);
                if let (Some(g), Some(story)) = (g, story) {
                    if let Some(&id) = kanji_id.get(g) {
                        let issues = e.get("issues").map(ToString::to_string);
                        tx.execute(
                            "INSERT OR REPLACE INTO mnemonic
                             (kanji_id, story, reading_story, reading_actor, meaning_placement, origin, verified, issues, imageability, distinctiveness)
                             VALUES (?1, ?2, ?3, ?4, ?5, 'generated', ?6, ?7, ?8, ?9)",
                            rusqlite::params![
                                id,
                                story,
                                e.get("reading_story").and_then(Value::as_str),
                                e.get("reading_actor_used").and_then(Value::as_str),
                                e.get("meaning_placement").and_then(Value::as_str),
                                e.get("verified").and_then(Value::as_bool).unwrap_or(false) as i64,
                                issues,
                                e.get("imageability").and_then(Value::as_i64),
                                e.get("distinctiveness").and_then(Value::as_i64),
                            ],
                        )?;
                        mn += 1;
                    }
                }
            }
        }
    }

    Ok((kw, ca, ra, mn))
}

/// Slice 3: set dominant readings (derived, with optional authored override + on'yomi-first
/// fallback) and insert in-context vocabulary. Returns (derived, fallback, vocab_count).
fn insert_vocab(
    tx: &rusqlite::Transaction,
    kanji_id: &HashMap<&str, i64>,
    rows: &[KanjiRow],
    vd: Option<&vocab::VocabData>,
) -> Result<(usize, usize, usize), Box<dyn Error>> {
    let Some(vd) = vd else {
        return Ok((0, 0, 0));
    };

    let mut dominant = vd.dominant.clone();
    for (_, label, _) in LEVELS {
        if let Some(v) = read_optional(&level_authored(label).dominant)? {
            for e in v.as_array().into_iter().flatten() {
                if let (Some(g), Some(r), Some(k)) = (
                    e.get("glyph").and_then(Value::as_str),
                    e.get("reading").and_then(Value::as_str),
                    e.get("kind").and_then(Value::as_str),
                ) {
                    dominant.insert(g.to_string(), (r.to_string(), k.to_string()));
                }
            }
        }
    }

    let (mut derived, mut fallback) = (0, 0);
    for r in rows {
        let kid = kanji_id[r.glyph.as_str()];
        let chosen = if let Some((reading, kind)) = dominant.get(&r.glyph) {
            derived += 1;
            Some((reading.clone(), kind.clone()))
        } else {
            let fb =
                r.on.first()
                    .map(|x| (x.clone(), "on".to_string()))
                    .or_else(|| r.kun.first().map(|x| (x.clone(), "kun".to_string())));
            if fb.is_some() {
                fallback += 1;
            }
            fb
        };
        if let Some((reading, kind)) = chosen {
            tx.execute(
                "UPDATE reading SET is_dominant = 1 WHERE kanji_id = ?1 AND reading = ?2 AND kind = ?3",
                rusqlite::params![kid, reading, kind],
            )?;
        }
    }

    let mut count = 0;
    for v in &vd.vocab {
        tx.execute(
            "INSERT OR IGNORE INTO vocab (surface, reading, gloss) VALUES (?1, ?2, ?3)",
            rusqlite::params![v.surface, v.reading, v.gloss],
        )?;
        let vid: i64 = tx.query_row(
            "SELECT id FROM vocab WHERE surface = ?1 AND reading = ?2",
            rusqlite::params![v.surface, v.reading],
            |row| row.get(0),
        )?;
        for (g, rt) in &v.kanji_parts {
            if let Some(&kid) = kanji_id.get(g.as_str()) {
                tx.execute(
                    "INSERT OR IGNORE INTO vocab_kanji (vocab_id, kanji_id, reading_in_word) VALUES (?1, ?2, ?3)",
                    rusqlite::params![vid, kid, rt],
                )?;
            }
        }
        count += 1;
    }
    Ok((derived, fallback, count))
}

/// Slice 4: insert example sentences, linking each to every vocab row with the same surface.
/// Returns the number of (sentence, vocab) links created.
fn insert_sentences(
    tx: &rusqlite::Transaction,
    map: Option<&sentence::SentenceMap>,
) -> Result<usize, Box<dyn Error>> {
    let Some(map) = map else {
        return Ok(0);
    };
    let mut links = 0;
    for (surface, sentences) in map {
        let vids: Vec<i64> = {
            let mut stmt = tx.prepare("SELECT id FROM vocab WHERE surface = ?1")?;
            let ids = stmt
                .query_map([surface], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            ids
        };
        if vids.is_empty() {
            continue;
        }
        for s in sentences {
            tx.execute(
                "INSERT OR IGNORE INTO sentence (jp, en, source) VALUES (?1, ?2, ?3)",
                rusqlite::params![s.jp, s.en, s.source],
            )?;
            let sid: i64 = tx.query_row("SELECT id FROM sentence WHERE jp = ?1", [&s.jp], |r| {
                r.get(0)
            })?;
            for vid in &vids {
                tx.execute(
                    "INSERT OR IGNORE INTO vocab_sentence (vocab_id, sentence_id) VALUES (?1, ?2)",
                    rusqlite::params![vid, sid],
                )?;
                links += 1;
            }
        }
    }
    Ok(links)
}

/// Apply reviewer-confirmed vocab gloss corrections from each level's `*-vocab-glosses.json`.
/// Returns the number of rows updated.
fn apply_gloss_overrides(tx: &rusqlite::Transaction) -> Result<usize, Box<dyn Error>> {
    let mut n = 0;
    for (_, label, _) in LEVELS {
        let Some(v) = read_optional(&level_authored(label).vglosses)? else {
            continue;
        };
        for e in v.as_array().into_iter().flatten() {
            if let (Some(surface), Some(reading), Some(gloss)) = (
                e.get("surface").and_then(Value::as_str),
                e.get("reading").and_then(Value::as_str),
                e.get("gloss").and_then(Value::as_str),
            ) {
                n += tx.execute(
                    "UPDATE vocab SET gloss = ?1 WHERE surface = ?2 AND reading = ?3",
                    rusqlite::params![gloss, surface, reading],
                )?;
            }
        }
    }
    Ok(n)
}

fn build_meta(rows: &[KanjiRow]) -> Vec<(&'static str, String)> {
    let levels = LEVELS
        .iter()
        .map(|(_, l, _)| *l)
        .collect::<Vec<_>>()
        .join(", ");
    vec![
        ("schema_version", "1".to_string()),
        (
            "slice",
            "5 (dominant readings, vocabulary, example sentences, stroke order)".to_string(),
        ),
        ("levels", levels),
        ("kanji_count", rows.len().to_string()),
        (
            "dominant_reading",
            "derived from JMdict-common x JmdictFurigana; overridable via data/authored/n5-dominant-readings.json"
                .to_string(),
        ),
        (
            "attribution",
            "Kanji data from KANJIDIC2 (c) EDRDG, CC BY-SA 4.0, via davidluzgouveia/kanji-data (MIT). \
             Component decomposition from kradfile-u, CC BY-SA. \
             WaniKani-derived fields are NOT used. See docs/04-DATA-SOURCES.md."
                .to_string(),
        ),
    ]
}

/// Print a verification summary so the build is self-checking.
fn verify(conn: &Connection, rows: &[KanjiRow]) -> Result<(), Box<dyn Error>> {
    let kanji: i64 = conn.query_row("SELECT COUNT(*) FROM kanji", [], |r| r.get(0))?;
    let comps: i64 = conn.query_row("SELECT COUNT(*) FROM component", [], |r| r.get(0))?;
    let readings: i64 = conn.query_row("SELECT COUNT(*) FROM reading", [], |r| r.get(0))?;
    let edges: i64 = conn.query_row("SELECT COUNT(*) FROM kanji_component", [], |r| r.get(0))?;
    let no_keyword: i64 = conn.query_row(
        "SELECT COUNT(*) FROM kanji WHERE primary_keyword = ''",
        [],
        |r| r.get(0),
    )?;
    let no_reading: i64 = conn.query_row(
        "SELECT COUNT(*) FROM kanji k WHERE NOT EXISTS (SELECT 1 FROM reading r WHERE r.kanji_id=k.id)",
        [],
        |r| r.get(0),
    )?;
    let dominant: i64 = conn.query_row(
        "SELECT COUNT(*) FROM reading WHERE is_dominant=1",
        [],
        |r| r.get(0),
    )?;
    let vocab_n: i64 = conn.query_row("SELECT COUNT(*) FROM vocab", [], |r| r.get(0))?;
    let vk_n: i64 = conn.query_row("SELECT COUNT(*) FROM vocab_kanji", [], |r| r.get(0))?;
    let no_dom: i64 = conn.query_row(
        "SELECT COUNT(*) FROM kanji k WHERE NOT EXISTS \
         (SELECT 1 FROM reading r WHERE r.kanji_id=k.id AND r.is_dominant=1)",
        [],
        |r| r.get(0),
    )?;

    println!("Verification:");
    println!("  kanji={kanji} (expected {})", rows.len());
    println!("  components={comps}  readings={readings}  component-edges={edges}");
    println!("  kanji with empty keyword: {no_keyword}");
    println!("  kanji with no reading:    {no_reading}");
    println!("  dominant readings set:    {dominant} ({no_dom} kanji without one)");
    println!("  vocab words={vocab_n}  vocab-kanji links={vk_n}");
    let sentences: i64 = conn.query_row("SELECT COUNT(*) FROM sentence", [], |r| r.get(0))?;
    let vs_n: i64 = conn.query_row("SELECT COUNT(*) FROM vocab_sentence", [], |r| r.get(0))?;
    let no_sentence: i64 = conn.query_row(
        "SELECT COUNT(*) FROM kanji k WHERE NOT EXISTS \
         (SELECT 1 FROM vocab_kanji vk JOIN vocab_sentence vs ON vs.vocab_id = vk.vocab_id \
          WHERE vk.kanji_id = k.id)",
        [],
        |r| r.get(0),
    )?;
    println!("  sentences={sentences}  vocab-sentence links={vs_n}  ({no_sentence} kanji with no sentence)");
    let strokes: i64 = conn.query_row("SELECT COUNT(*) FROM stroke", [], |r| r.get(0))?;
    let no_stroke: i64 = conn.query_row(
        "SELECT COUNT(*) FROM kanji k WHERE NOT EXISTS (SELECT 1 FROM stroke s WHERE s.kanji_id = k.id)",
        [],
        |r| r.get(0),
    )?;
    println!("  stroke paths={strokes}  ({no_stroke} kanji without strokes)");

    println!("\nFirst 12 by learning order (intro_rank · keyword · #components):");
    let mut stmt = conn.prepare(
        "SELECT k.glyph, k.intro_rank, k.primary_keyword,
                (SELECT COUNT(*) FROM kanji_component kc WHERE kc.kanji_id = k.id)
         FROM kanji k ORDER BY k.intro_rank LIMIT 12",
    )?;
    let mapped = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
        ))
    })?;
    for row in mapped {
        let (g, rank, kw, nc) = row?;
        println!("  {rank:>2}  {g}  {kw:<14} [{nc} comp]");
    }

    println!("\nSample dominant reading + vocab (first 10 by order):");
    let mut s2 = conn.prepare(
        "SELECT k.glyph, k.primary_keyword,
            COALESCE((SELECT r.kind || ':' || r.reading FROM reading r
                      WHERE r.kanji_id = k.id AND r.is_dominant = 1 LIMIT 1), '-'),
            COALESCE((SELECT group_concat(vv.surface, ' ') FROM
                      (SELECT v.surface FROM vocab_kanji vk JOIN vocab v ON v.id = vk.vocab_id
                       WHERE vk.kanji_id = k.id LIMIT 3) vv), '-')
         FROM kanji k ORDER BY k.intro_rank LIMIT 10",
    )?;
    let rows2 = s2.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    for row in rows2 {
        let (g, kw, dom, vocab) = row?;
        println!("  {g}  {kw:<12} dom={dom:<10} vocab: {vocab}");
    }
    Ok(())
}
