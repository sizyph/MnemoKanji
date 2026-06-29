//! Slice 5: stroke-order data from KanjiVG — the ordered SVG stroke paths per kanji.
//!
//! The UI renders these `<path d="…">` strings as an SVG and animates each in turn (CSS
//! `stroke-dashoffset`) for stroke-order display in the production/writing mode. Optional: if
//! `data/sources/kanjivg/` is absent, `build` returns an empty map.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

const KANJIVG_DIR: &str = "data/sources/kanjivg/kanji";

/// glyph -> ordered SVG path `d` strings (one per stroke).
pub fn build(glyphs: &[String]) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    if !Path::new(KANJIVG_DIR).exists() {
        return map;
    }
    for g in glyphs {
        let Some(cp) = g.chars().next().map(|c| c as u32) else {
            continue;
        };
        let file = format!("{KANJIVG_DIR}/{cp:05x}.svg");
        if let Ok(svg) = fs::read_to_string(&file) {
            let paths = extract_paths(&svg);
            if !paths.is_empty() {
                map.insert(g.clone(), paths);
            }
        }
    }
    map
}

/// Extract the ordered stroke `d` attributes. We match a leading-space ` d="` so we pick up
/// `<path … d="…">` and never the `id="…"` attribute (which also contains the substring `d="`).
fn extract_paths(svg: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = svg;
    while let Some(p) = rest.find(" d=\"") {
        let after = &rest[p + 4..];
        match after.find('"') {
            Some(end) => {
                out.push(after[..end].to_string());
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_paths_not_ids() {
        let svg = r#"<g id="kvg:StrokePaths_065e5"><path id="kvg:065e5-s1" d="M31.5,24.5z"/><path id="kvg:065e5-s2" d="M33.4,26z"/></g>"#;
        assert_eq!(extract_paths(svg), vec!["M31.5,24.5z", "M33.4,26z"]);
    }
}
