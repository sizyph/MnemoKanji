//! Slice 3: dominant-reading derivation + in-context vocabulary.
//!
//! From JMdict-common (vocab + glosses + commonness) and JmdictFurigana (per-kanji kana
//! alignment), we (a) derive each N5 kanji's *dominant* reading by tallying which reading it
//! contributes across common words, and (b) select a handful of common words per kanji for
//! in-context practice. See `docs/08-DATASET.md` §2.2–2.3. Both sources are optional: if absent
//! (e.g. `scripts/fetch-sources.sh` not run), `build` returns `None` and the slice is skipped.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::Path;

use serde_json::Value;

const JMDICT: &str = "data/sources/jmdict-eng-common.json";
const FURIGANA: &str = "data/sources/JmdictFurigana.json";
const FREQ: &str = "data/sources/ja-freq.txt";
const JLPT_DIR: &str = "data/sources/jlpt";
const MAX_VOCAB_PER_KANJI: usize = 6;

/// A selected vocabulary word and the N5 kanji parts it exemplifies.
pub struct VocabEntry {
    pub surface: String,
    pub reading: String,
    pub gloss: String,
    /// (kanji glyph, kana the kanji contributes in this word) per the furigana split.
    pub kanji_parts: Vec<(String, String)>,
}

pub struct VocabData {
    /// kanji glyph -> (stored reading text, kind) chosen as dominant.
    pub dominant: HashMap<String, (String, String)>,
    pub vocab: Vec<VocabEntry>,
}

/// glyph -> [(kind "on"|"kun", stored reading text)].
pub type StoredReadings = HashMap<String, Vec<(String, String)>>;

/// (surface, reading) -> per-segment [(ruby, Some(rt) | None-for-okurigana)] from JmdictFurigana.
type FuriganaMap = HashMap<(String, String), Vec<(String, Option<String>)>>;
/// kanji glyph -> [(context-form, (kind, stored reading))] for dominant-reading matching.
type ReadingForms<'a> = HashMap<&'a str, Vec<(String, (String, String))>>;

pub fn build(
    n5: &HashSet<String>,
    stored: &StoredReadings,
    kanji_levels: &HashMap<String, i64>,
) -> Result<Option<VocabData>, Box<dyn Error>> {
    if !Path::new(JMDICT).exists() || !Path::new(FURIGANA).exists() {
        return Ok(None);
    }

    // (surface, reading) -> per-segment [(ruby, Some(rt) | None-for-okurigana)].
    let furigana = read_json(FURIGANA)?;
    let mut fmap: FuriganaMap = HashMap::new();
    for e in furigana.as_array().into_iter().flatten() {
        let t = e.get("text").and_then(Value::as_str).unwrap_or_default();
        let r = e.get("reading").and_then(Value::as_str).unwrap_or_default();
        let segs = e
            .get("furigana")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .map(|s| {
                        (
                            s.get("ruby")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            s.get("rt").and_then(Value::as_str).map(str::to_string),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        fmap.insert((t.to_string(), r.to_string()), segs);
    }
    drop(furigana);

    // Per N5 kanji: every context-form a stored reading can take -> the stored (kind, reading).
    let mut forms: ReadingForms = HashMap::new();
    for (g, readings) in stored {
        if !n5.contains(g) {
            continue;
        }
        let mut v = Vec::new();
        for (kind, rd) in readings {
            let stem = reading_stem(rd);
            if stem.is_empty() {
                continue;
            }
            for f in context_forms(&stem) {
                v.push((hira(&f), (kind.clone(), rd.clone())));
            }
        }
        forms.insert(g.as_str(), v);
    }

    // Optional surface-frequency ranks (OpenSubtitles, CC BY-SA); lower rank = more frequent.
    let freq: HashMap<String, usize> = match fs::read_to_string(FREQ) {
        Ok(t) => t
            .lines()
            .enumerate()
            .filter_map(|(i, l)| l.split_whitespace().next().map(|w| (w.to_string(), i)))
            .collect(),
        Err(_) => HashMap::new(),
    };

    // Optional JLPT word levels (Yomitan dict); surface -> easiest level (5=N5 .. 1=N1).
    let jlpt = load_jlpt();

    // Tally dominant readings and collect vocab candidates in one pass over common words.
    let jm = read_json(JMDICT)?;
    let mut tally: HashMap<String, HashMap<(String, String), i64>> = HashMap::new();
    let mut candidates: Vec<(usize, VocabEntry)> = Vec::new(); // (selection cost, entry)

    for w in jm
        .get("words")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(surface) = w
            .get("kanji")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find(|k| k.get("common").and_then(Value::as_bool).unwrap_or(false))
            .and_then(|k| k.get("text").and_then(Value::as_str))
        else {
            continue;
        };
        if !surface.chars().any(|c| n5.contains(&c.to_string())) {
            continue;
        }
        let Some(reading) = w
            .get("kana")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|k| k.get("common").and_then(Value::as_bool).unwrap_or(false))
            .find_map(|k| {
                let applies = match k.get("appliesToKanji").and_then(Value::as_array) {
                    Some(arr) => arr
                        .iter()
                        .any(|x| x.as_str() == Some("*") || x.as_str() == Some(surface)),
                    None => true,
                };
                applies
                    .then(|| k.get("text").and_then(Value::as_str))
                    .flatten()
            })
        else {
            continue;
        };
        let gloss = w
            .get("sense")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .find_map(|s| {
                s.get("gloss")
                    .and_then(Value::as_array)
                    .and_then(|g| g.first())
                    .and_then(|g| g.get("text"))
                    .and_then(Value::as_str)
            })
            .unwrap_or_default();

        // Split into per-kanji parts via the furigana alignment (regular words only).
        let mut parts: Vec<(String, String)> = Vec::new();
        if let Some(segs) = fmap.get(&(surface.to_string(), reading.to_string())) {
            for (ruby, rt) in segs {
                let Some(rt) = rt else { continue };
                if ruby.chars().count() != 1 || !n5.contains(ruby) {
                    continue;
                }
                parts.push((ruby.clone(), rt.clone()));
                if let Some(cands) = forms.get(ruby.as_str()) {
                    let rtn = hira(rt);
                    if let Some((_, kr)) = cands.iter().find(|(f, _)| *f == rtn) {
                        *tally
                            .entry(ruby.clone())
                            .or_default()
                            .entry(kr.clone())
                            .or_default() += 1;
                    }
                }
            }
        }
        // Skip irregular/jukujikun (no aligned N5 segment) and overly long/obscure compounds.
        if parts.is_empty() || surface.chars().count() > 4 {
            continue;
        }
        // JLPT appropriateness: drop words listed N1/N2; for unlisted words require every kanji
        // to be N5–N3 (keeps essential listed-N5 words like 大丈夫 whose kanji are advanced).
        let wl = jlpt.get(surface).copied();
        let appropriate = match wl {
            Some(l) if l <= 2 => false,
            Some(_) => true,
            None => surface.chars().all(|c| {
                !is_kanji(c) || matches!(kanji_levels.get(&c.to_string()).copied(), Some(3..=5))
            }),
        };
        if !appropriate {
            continue;
        }
        // Selection cost: JLPT tier first (N5<N4<N3<unlisted), then compounds before single-kanji
        // standalones, then OpenSubtitles frequency (length as fallback).
        let tier = match wl {
            Some(5) => 0usize,
            Some(4) => 1,
            Some(3) => 2,
            _ => 3,
        };
        let clen = surface.chars().count();
        let len_cost = clen * 10 + reading.chars().count();
        let base = freq.get(surface).copied().unwrap_or(1_000_000 + len_cost);
        let cost = tier * 100_000_000 + if clen == 1 { 10_000_000 + base } else { base };
        candidates.push((
            cost,
            VocabEntry {
                surface: surface.to_string(),
                reading: reading.to_string(),
                gloss: gloss.to_string(),
                kanji_parts: parts,
            },
        ));
    }
    drop(jm);
    drop(fmap);

    let mut dominant = HashMap::new();
    for (g, counts) in &tally {
        if let Some(((kind, reading), _)) = counts.iter().max_by_key(|(_, c)| **c) {
            dominant.insert(g.clone(), (reading.clone(), kind.clone()));
        }
    }

    // Select up to MAX_VOCAB_PER_KANJI common words per kanji, cheapest (most basic) first.
    candidates.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.surface.cmp(&b.1.surface)));
    // Per kanji track (total, single-char standalones); cap standalones at 1 so a kanji's slots
    // go mostly to compounds.
    let mut per_kanji: HashMap<String, (usize, usize)> = HashMap::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut vocab: Vec<VocabEntry> = Vec::new();
    for (_, entry) in candidates {
        let single = entry.surface.chars().count() == 1;
        let wanted = entry.kanji_parts.iter().any(|(g, _)| {
            let (total, singles) = per_kanji.get(g).copied().unwrap_or((0, 0));
            total < MAX_VOCAB_PER_KANJI && (!single || singles < 1)
        });
        if !wanted || !seen.insert(format!("{}\u{1}{}", entry.surface, entry.reading)) {
            continue;
        }
        for (g, _) in &entry.kanji_parts {
            let e = per_kanji.entry(g.clone()).or_insert((0, 0));
            e.0 += 1;
            if single {
                e.1 += 1;
            }
        }
        vocab.push(entry);
    }

    Ok(Some(VocabData { dominant, vocab }))
}

/// Read a JSON file, tolerating a leading UTF-8 BOM (JmdictFurigana has one).
fn read_json(path: &str) -> Result<Value, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(text.trim_start_matches('\u{feff}'))?)
}

/// Load JLPT word levels from the Yomitan term_meta_bank files (surface -> easiest level, 5=N5).
fn load_jlpt() -> HashMap<String, u8> {
    let mut m: HashMap<String, u8> = HashMap::new();
    let Ok(dir) = fs::read_dir(JLPT_DIR) else {
        return m;
    };
    for entry in dir.flatten() {
        let p = entry.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default();
        if !name.starts_with("term_meta_bank_") || !name.ends_with(".json") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&p) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) else {
            continue;
        };
        for e in v.as_array().into_iter().flatten() {
            let surface = e.get(0).and_then(Value::as_str);
            let level = e
                .get(2)
                .and_then(|x| x.get("frequency"))
                .and_then(|f| f.get("displayValue"))
                .and_then(Value::as_str)
                .and_then(level_num);
            if let (Some(s), Some(n)) = (surface, level) {
                let cur = m.entry(s.to_string()).or_insert(0);
                if n > *cur {
                    *cur = n;
                }
            }
        }
    }
    m
}

fn level_num(s: &str) -> Option<u8> {
    match s {
        "N5" => Some(5),
        "N4" => Some(4),
        "N3" => Some(3),
        "N2" => Some(2),
        "N1" => Some(1),
        _ => None,
    }
}

fn is_kanji(c: char) -> bool {
    ('\u{3400}'..='\u{9FFF}').contains(&c)
}

/// Stem of a stored reading: drop okurigana after '.', strip '-' markers (e.g. `い.きる`->`い`,
/// `おお-`->`おお`, `-り`->`り`).
fn reading_stem(rd: &str) -> String {
    rd.split('.')
        .next()
        .unwrap_or(rd)
        .replace('-', "")
        .trim()
        .to_string()
}

/// The context forms a reading can take inside a compound: itself, rendaku (voiced first mora),
/// gemination (trailing っ), and the combination.
fn context_forms(base: &str) -> Vec<String> {
    let mut out = vec![base.to_string()];
    if let Some(g) = geminate(base) {
        out.push(g);
    }
    if let Some(r) = rendaku(base) {
        if let Some(g) = geminate(&r) {
            out.push(g);
        }
        out.push(r);
    }
    out
}

/// Gemination: an on'yomi ending in く/つ/ち/き becomes っ before certain consonants.
fn geminate(s: &str) -> Option<String> {
    let last = s.chars().last()?;
    if "くつちき".contains(last) && s.chars().count() > 1 {
        let mut t: String = s.chars().take(s.chars().count() - 1).collect();
        t.push('っ');
        Some(t)
    } else {
        None
    }
}

/// Rendaku: voice the first mora (か->が, さ->ざ, た->だ, は->ば, …).
fn rendaku(s: &str) -> Option<String> {
    let mut chars: Vec<char> = s.chars().collect();
    let v = voiced(*chars.first()?)?;
    chars[0] = v;
    Some(chars.into_iter().collect())
}

fn voiced(c: char) -> Option<char> {
    Some(match c {
        'か' => 'が',
        'き' => 'ぎ',
        'く' => 'ぐ',
        'け' => 'げ',
        'こ' => 'ご',
        'さ' => 'ざ',
        'し' => 'じ',
        'す' => 'ず',
        'せ' => 'ぜ',
        'そ' => 'ぞ',
        'た' => 'だ',
        'ち' => 'ぢ',
        'つ' => 'づ',
        'て' => 'で',
        'と' => 'ど',
        'は' => 'ば',
        'ひ' => 'び',
        'ふ' => 'ぶ',
        'へ' => 'べ',
        'ほ' => 'ぼ',
        _ => return None,
    })
}

/// Normalize katakana to hiragana for matching (readings are hiragana, but be defensive).
fn hira(s: &str) -> String {
    s.chars()
        .map(|c| {
            let u = c as u32;
            if (0x30A1..=0x30F6).contains(&u) {
                char::from_u32(u - 0x60).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}
