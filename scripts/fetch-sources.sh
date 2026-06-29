#!/usr/bin/env bash
# Fetch the openly-licensed source datasets for the content pipeline into data/sources/.
# These are regenerable and git-ignored. Licenses + provenance: docs/04-DATA-SOURCES.md.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p data/sources

fetch() { # url dest
  echo "fetching $2"
  curl -fsSL -o "data/sources/$2" "$1"
}

# JLPT levels + KANJIDIC-derived readings/meanings/strokes/freq (MIT; uses KANJIDIC2 CC BY-SA).
fetch "https://raw.githubusercontent.com/davidluzgouveia/kanji-data/master/kanji.json" "kanji-data.json"
# Component decomposition (CC BY-SA, UTF-8).
fetch "https://raw.githubusercontent.com/jmettraux/kensaku/master/data/kradfile-u" "kradfile-u"

# Slice 3 (vocab + dominant readings) — versioned GitHub release assets, fetched via `gh`.
# JMdict (common-only, English; CC BY-SA, EDRDG) + per-kanji furigana alignment (JmdictFurigana).
echo "fetching jmdict-eng-common + JmdictFurigana (requires gh CLI)"
gh release download --repo scriptin/jmdict-simplified --pattern 'jmdict-eng-common-*.json.tgz' --dir data/sources --clobber
tar -xzf data/sources/jmdict-eng-common-*.json.tgz -C data/sources
mv -f data/sources/jmdict-eng-common-*.json data/sources/jmdict-eng-common.json
rm -f data/sources/jmdict-eng-common-*.json.tgz
gh release download --repo Doublevil/JmdictFurigana --pattern 'JmdictFurigana.json' --dir data/sources --clobber
# Example sentences (JP+EN), JMdict-simplified with Tanaka/Tatoeba examples per word (slice 4).
gh release download --repo scriptin/jmdict-simplified --pattern 'jmdict-examples-eng-*.json.tgz' --dir data/sources --clobber
tar -xzf data/sources/jmdict-examples-eng-*.json.tgz -C data/sources
mv -f data/sources/jmdict-examples-eng-*.json data/sources/jmdict-examples.json
rm -f data/sources/jmdict-examples-eng-*.json.tgz
# Word frequency for vocab ranking (OpenSubtitles 2016 top-50k, CC BY-SA).
fetch "https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2016/ja/ja_50k.txt" "ja-freq.txt"
# JLPT word levels (word-level, Yomitan dict) for beginner-appropriate vocab selection.
gh release download --repo stephenmk/yomitan-jlpt-vocab --pattern 'jlpt.zip' --dir data/sources --clobber
mkdir -p data/sources/jlpt && unzip -oq data/sources/jlpt.zip -d data/sources/jlpt
# Stroke-order vectors (slice 5): KanjiVG main set (CC BY-SA 3.0).
gh release download --repo KanjiVG/kanjivg --pattern 'kanjivg-*-main.zip' --dir data/sources --clobber
mkdir -p data/sources/kanjivg && unzip -oq data/sources/kanjivg-*-main.zip -d data/sources/kanjivg

echo "done. sources in data/sources/"
