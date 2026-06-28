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

echo "done. sources in data/sources/"
