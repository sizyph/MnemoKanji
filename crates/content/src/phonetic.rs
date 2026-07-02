//! Slice 6: phonetic components (sound markers) from Kanjium's kanjidict (CC BY-SA 4.0).
//!
//! A phono-semantic kanji carries a component that marks its on'yomi (寺 marks ジ in 時/持;
//! 青 marks セイ in 晴/清). Kanjium's kanjidict.txt records that component per kanji (column 4).
//! We store it as a `kanji_component` edge with `role = 'phonetic'` — the systematic
//! phonetic-series reading system is a core method feature (docs/02 §reading, docs/09).
//! Optional: if `data/sources/kanjium-kanjidict.txt` is absent, `build` returns `None`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

const KANJIDICT: &str = "data/sources/kanjium-kanjidict.txt";

/// kanji glyph -> phonetic-component glyph, for every kanji that has one.
pub type PhoneticMap = HashMap<String, String>;

/// Parse the Kanjium kanjidict TSV: column 0 = kanji, column 3 = phonetic component (may be
/// empty). Restricted to `selected` so the map stays small.
pub fn build(selected: &[String]) -> Option<PhoneticMap> {
    if !Path::new(KANJIDICT).exists() {
        return None;
    }
    let text = fs::read_to_string(KANJIDICT).ok()?;
    let wanted: std::collections::HashSet<&str> = selected.iter().map(String::as_str).collect();
    let mut map = PhoneticMap::new();
    for line in text.lines() {
        let mut f = line.split('\t');
        let (Some(kanji), _, _, Some(phonetic)) = (f.next(), f.next(), f.next(), f.next()) else {
            continue;
        };
        if phonetic.is_empty() || !wanted.contains(kanji) {
            continue;
        }
        map.insert(kanji.to_string(), phonetic.to_string());
    }
    Some(map)
}
