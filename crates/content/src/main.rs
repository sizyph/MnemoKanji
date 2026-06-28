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

const SOURCE_JSON: &str = "data/sources/kanji-data.json";
const SOURCE_KRAD: &str = "data/sources/kradfile-u";
const OUT_DB: &str = "assets/seed.sqlite";
const SCHEMA: &str = include_str!("schema.sql");

/// A kanji as assembled from the sources, before DB insertion.
struct KanjiRow {
    glyph: String,
    strokes: Option<i64>,
    freq: Option<i64>,
    keyword: String,
    meanings: Vec<String>,
    on: Vec<String>,
    kun: Vec<String>,
    components: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("mnemokanji-content: building N5 seed (slice 1)\n");

    let kanji_data = load_json(SOURCE_JSON)?;
    let krad = load_kradfile(SOURCE_KRAD)?;

    // --- Assemble the N5 kanji set (jlpt_new == 5), deterministically ordered by glyph. ---
    let obj = kanji_data
        .as_object()
        .ok_or("kanji-data.json is not a JSON object")?;
    let mut n5_glyphs: Vec<String> = obj
        .iter()
        .filter(|(_, v)| v.get("jlpt_new").and_then(Value::as_i64) == Some(5))
        .map(|(k, _)| k.clone())
        .collect();
    n5_glyphs.sort();
    let n5_set: HashSet<&str> = n5_glyphs.iter().map(String::as_str).collect();

    let rows: Vec<KanjiRow> = n5_glyphs
        .iter()
        .map(|g| {
            let info = &obj[g];
            let meanings = str_array(info, "meanings");
            KanjiRow {
                glyph: g.clone(),
                strokes: info.get("strokes").and_then(Value::as_i64),
                freq: info.get("freq").and_then(Value::as_i64),
                keyword: normalize_keyword(meanings.first().map(String::as_str).unwrap_or("")),
                meanings,
                on: str_array(info, "readings_on"),
                kun: str_array(info, "readings_kun"),
                components: krad.get(g).cloned().unwrap_or_default(),
            }
        })
        .collect();

    // --- Frequency-weighted topological order within N5. ---
    let intro_order = topological_order(&rows, &n5_set);
    let intro_rank: HashMap<&str, i64> = intro_order
        .iter()
        .enumerate()
        .map(|(i, g)| (g.as_str(), i as i64))
        .collect();

    // --- Build the SQLite seed. ---
    if let Some(parent) = Path::new(OUT_DB).parent() {
        fs::create_dir_all(parent)?;
    }
    let _ = fs::remove_file(OUT_DB);
    let mut conn = Connection::open(OUT_DB)?;
    conn.execute_batch(SCHEMA)?;

    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO level (id, jlpt, ord, kanji_count) VALUES (1, 'N5', 1, ?1)",
        [rows.len() as i64],
    )?;

    // Distinct components across all N5 kanji.
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
            rusqlite::params![c, n5_set.contains(c) as i64],
        )?;
        comp_id.insert(c, tx.last_insert_rowid());
    }

    for r in &rows {
        tx.execute(
            "INSERT INTO kanji (glyph, level_id, stroke_count, freq, primary_keyword, intro_rank)
             VALUES (?1, 1, ?2, ?3, ?4, ?5)",
            rusqlite::params![r.glyph, r.strokes, r.freq, r.keyword, intro_rank[r.glyph.as_str()]],
        )?;
        let kid = tx.last_insert_rowid();

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

    for (k, v) in build_meta(&rows) {
        tx.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)",
            rusqlite::params![k, v],
        )?;
    }
    tx.commit()?;

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

/// Frequency-weighted topological sort (Kahn): among kanji whose N5-kanji components are all
/// already placed, emit the most frequent next (lower freq rank = more frequent; ties by fewer
/// strokes, then codepoint). Non-N5 components are assumed known (just-in-time), so they don't
/// gate. See docs/02 §H. Returns the glyphs in learning order.
fn topological_order(rows: &[KanjiRow], n5_set: &HashSet<&str>) -> Vec<String> {
    let info: HashMap<&str, &KanjiRow> = rows.iter().map(|r| (r.glyph.as_str(), r)).collect();
    let glyphs: Vec<&str> = {
        let mut v: Vec<&str> = rows.iter().map(|r| r.glyph.as_str()).collect();
        v.sort_unstable();
        v
    };

    // Edge c -> k when component c is itself an N5 kanji used in k (c is a prerequisite of k).
    let mut indeg: BTreeMap<&str, usize> = glyphs.iter().map(|&g| (g, 0)).collect();
    let mut succ: HashMap<&str, Vec<&str>> = HashMap::new();
    for &k in &glyphs {
        let mut prereqs: BTreeSet<&str> = BTreeSet::new();
        for c in &info[k].components {
            let cs = c.as_str();
            if cs != k && n5_set.contains(cs) {
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
        let mut rest: Vec<&str> = glyphs.iter().copied().filter(|g| !placed.contains(g)).collect();
        rest.sort_by(|a, b| rank(a).cmp(&rank(b)));
        order.extend(rest.into_iter().map(str::to_string));
    }
    order
}

fn build_meta(rows: &[KanjiRow]) -> Vec<(&'static str, String)> {
    vec![
        ("schema_version", "1".to_string()),
        ("slice", "1 (kanji core + components + order)".to_string()),
        ("levels", "N5".to_string()),
        ("kanji_count", rows.len().to_string()),
        ("dominant_reading", "provisional: none (set in vocab slice)".to_string()),
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
    let no_keyword: i64 =
        conn.query_row("SELECT COUNT(*) FROM kanji WHERE primary_keyword = ''", [], |r| r.get(0))?;
    let no_reading: i64 = conn.query_row(
        "SELECT COUNT(*) FROM kanji k WHERE NOT EXISTS (SELECT 1 FROM reading r WHERE r.kanji_id=k.id)",
        [],
        |r| r.get(0),
    )?;

    println!("Verification:");
    println!("  kanji={kanji} (expected {})", rows.len());
    println!("  components={comps}  readings={readings}  component-edges={edges}");
    println!("  kanji with empty keyword: {no_keyword}");
    println!("  kanji with no reading:    {no_reading}");

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
    Ok(())
}
