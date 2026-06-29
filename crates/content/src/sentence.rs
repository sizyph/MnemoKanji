//! Slice 4: example sentences (JP + EN) for in-context reading and cloze.
//!
//! Source: jmdict-examples (JMdict-simplified with Tanaka/Tatoeba examples attached per word,
//! each carrying a paired Japanese sentence and English translation). We keep the shortest few
//! sentences that contain a wanted vocab surface, are translated, and fall in a length band.
//! Optional: if `data/sources/jmdict-examples.json` is absent, `build` returns `None`.
//!
//! We deserialize into slim structs (not a generic `Value`) so the ~120 MB source doesn't blow
//! up memory — serde skips every field we don't name.

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::Path;

use serde::Deserialize;

const EXAMPLES: &str = "data/sources/jmdict-examples.json";
const MAX_SENTENCES_PER_WORD: usize = 2;
const MIN_LEN: usize = 6;
const MAX_LEN: usize = 40;

pub struct Sentence {
    pub jp: String,
    pub en: String,
    pub source: String,
}

/// Wanted vocab surface -> its chosen example sentences.
pub type SentenceMap = HashMap<String, Vec<Sentence>>;

#[derive(Deserialize)]
struct ExFile {
    words: Vec<Word>,
}
#[derive(Deserialize)]
struct Word {
    #[serde(default)]
    kanji: Vec<Kanji>,
    #[serde(default)]
    sense: Vec<Sense>,
}
#[derive(Deserialize)]
struct Kanji {
    text: String,
}
#[derive(Deserialize)]
struct Sense {
    #[serde(default)]
    examples: Vec<Example>,
}
#[derive(Deserialize)]
struct Example {
    #[serde(default)]
    source: Source,
    #[serde(default)]
    sentences: Vec<Sent>,
}
#[derive(Deserialize, Default)]
struct Source {
    #[serde(default)]
    value: String,
}
#[derive(Deserialize)]
struct Sent {
    lang: String,
    text: String,
}

/// For each wanted surface, up to a couple of short translated example sentences containing it.
pub fn build(wanted: &HashSet<String>) -> Result<Option<SentenceMap>, Box<dyn Error>> {
    if !Path::new(EXAMPLES).exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(EXAMPLES)?;
    let file: ExFile = serde_json::from_str(text.trim_start_matches('\u{feff}'))?;

    let mut cands: HashMap<String, Vec<Sentence>> = HashMap::new();
    let mut seen: HashSet<String> = HashSet::new();
    for w in &file.words {
        let surfaces: Vec<&str> = w
            .kanji
            .iter()
            .map(|k| k.text.as_str())
            .filter(|t| wanted.contains(*t))
            .collect();
        if surfaces.is_empty() {
            continue;
        }
        for sense in &w.sense {
            for ex in &sense.examples {
                let jp = ex
                    .sentences
                    .iter()
                    .find(|s| s.lang == "jpn")
                    .map(|s| &s.text);
                let en = ex
                    .sentences
                    .iter()
                    .find(|s| s.lang == "eng")
                    .map(|s| &s.text);
                let (Some(jp), Some(en)) = (jp, en) else {
                    continue;
                };
                let len = jp.chars().count();
                if !(MIN_LEN..=MAX_LEN).contains(&len) {
                    continue;
                }
                for &surface in &surfaces {
                    if !jp.contains(surface) || !seen.insert(format!("{surface}\u{1}{jp}")) {
                        continue;
                    }
                    cands
                        .entry(surface.to_string())
                        .or_default()
                        .push(Sentence {
                            jp: jp.clone(),
                            en: en.clone(),
                            source: format!("tatoeba:{}", ex.source.value),
                        });
                }
            }
        }
    }

    for v in cands.values_mut() {
        v.sort_by_key(|s| s.jp.chars().count());
        v.truncate(MAX_SENTENCES_PER_WORD);
    }
    Ok(Some(cands))
}
