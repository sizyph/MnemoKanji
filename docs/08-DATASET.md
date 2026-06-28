# 08 — Dataset Assembly & Selection Algorithms

The concrete recipe for building `assets/seed.sqlite` from openly-licensed sources: which dataset
fills each field, how to *algorithmically* select keyword / reading / vocab / sentence per kanji,
how to merge, and where sources conflict. Licensing authority is [04-DATA-SOURCES](04-DATA-SOURCES.md);
this doc is the build spec. Grounded in the third research pass ([06-RESEARCH §7](06-RESEARCH.md)).

## 1. Source-of-truth per field

| Field | Source | License | Notes |
|-------|--------|---------|-------|
| **JLPT level (N5–N1)** | `davidluzgouveia/kanji-data` → `jlpt_new` | MIT | Based on Tanos.co.uk. Counts: N5 79 · N4 166 · N3 367 · N2 367 · N1 1232 (=2211). |
| Readings, meanings, strokes | KANJIDIC2 | CC BY-SA 4.0 | Meanings **unordered**; readings unranked. |
| Kanji frequency | KANJIDIC2 `freq` ⊕ `scriptin/kanji-frequency` (Wikipedia) | CC BY-SA 4.0 / CC BY 4.0 | Blend to correct newspaper bias. |
| Component structure + position | KanjiVG | CC BY-SA 3.0 | Has `kvg:position`, `kvg:radical`, `kvg:original`. Backbone. |
| Component naming granularity | KRADFILE/RADKFILE | CC BY-SA 4.0 | Decides which tree nodes become named actors. |
| Phonetic component + tier | Kanjium + KANJIDIC | CC BY-SA 4.0 | 天/上/中/下 computed (see [02 §E](02-LEARNING-METHOD.md)). |
| **Per-kanji kana alignment** | `JmdictFurigana` (Doublevil) | CC BY-SA / MIT* | **Linchpin** — maps a word's reading to which kana each kanji contributes. |
| Vocabulary + glosses | JMdict | CC BY-SA 4.0 | Glosses **are** ordered most-common-first (unlike KANJIDIC2). |
| Word frequency | `wordfreq` (rspeer) ⊕ `hermitdave/FrequencyWords` | Apache-2.0 / CC BY-SA | wordfreq frozen ~2021 (fine for core vocab); FrequencyWords = OpenSubtitles colloquial. |
| JLPT level of *words* | `stephenmk/yomitan-jlpt-vocab` | CC BY-SA | Keyed by JMdict ID (join by ID, not string). |
| Example sentences | Tatoeba exports + `jpn_indices.csv` | CC BY 2.0 FR (+ CC0 subset) | `jpn_indices` = Tanaka Corpus, pre-segmented by headword. |

\* JmdictFurigana README/LICENSE disagree — attribute under both to be safe.
**Build-time reference only (do NOT ship):** CHISE IDS (GPLv2) for decomposition cross-check;
WaniKani `wk_meanings` (proprietary editorial) as a keyword *quality signal* only.

## 2. Selection algorithms

### 2.1 Primary keyword (avoid the Heisig trap)
> Trap: KANJIDIC2 meanings are unordered **and ~1,900 were originally seeded from Heisig's
> keywords** — so a naive "first gloss" does *not* escape Heisig.

1. Baseline = KANJIDIC2 first `<meaning>`, normalized (strip parentheticals, first comma-segment,
   lowercase).
2. Cross-check against the first gloss of the **highest-frequency JMdict word** using the kanji
   (JMdict glosses are frequency-ordered); agreement ⇒ high confidence.
3. Cross-check against `wk_meanings` as a *quality flag only* (do not ship the WaniKani string).
4. Abstract/grammatical/compound-only kanji: fall back to KANJIDIC2 verbatim → compound-derived
   gloss (marked) → else `keyword=null, needs_manual=true`.
5. **Manual override table** (app-owned) for the null residue **and to break keyword collisions**
   (a learning app wants unique keywords; raw first-glosses collide on "cut", "bright", …).

### 2.2 Dominant reading (derive — no open dataset gives it)
For each kanji K: take its top-N frequent words; for each, use **JmdictFurigana** to align which
kana K contributes; normalize (strip okurigana, undo rendaku ばや→はや and gemination がっ→がく);
map to a canonical KANJIDIC2 on/kun entry; tally weighted by word frequency. Dominant =
argmax(tally). **Exclude** `<nanori>` and jukujikun/irregular whole-word readings. Teach
**on'yomi first** in general, but use the per-kanji on-ratio rather than a blanket rule.

### 2.3 Vocabulary per kanji/reading
For each JMdict entry whose kanji-form contains K, **honor `re_restr`/`re_nokanji`/`stagk`** (it's
not a blind kanji×reading cross-product); align with JmdictFurigana to learn *which* reading of K
the word shows; score by word frequency (wordfreq → FrequencyWords → JMdict `nfXX` band as coarse
tiebreak); join JLPT level by **JMdict ID**. Pick the highest-frequency at-or-below-level word
**per important reading** (学生→sei, 生きる→i, 生まれる→u).

### 2.4 Example sentences (Tatoeba)
Index `word → sentences` from `jpn_indices.csv` (headword + reading + sense, no morphology needed).
Hard filters: has English translation; target appears exactly once; length band (~8–30 chars, tune
per level); not tagged `@check`/`@needs-native-check`/rated −1; every other token ≤ learner level
(i+1). Score: **`~` tilde "checked good" marker** + native-speaker owner + `OK` tag + direct (not
chained) translation + audio-available + length fit. Take top 1–3. The combination **tilde +
native owner + OK** is the best trustworthy-sentence proxy (≈65% of JP sentences trace to the
Tanaka Corpus, much of it unnatural — native-owner filtering is essential). **Audio is per-row
licensed; empty license = unusable.** Do **not** bundle Anki Core 2k/6k sentences/audio
(commercial origin).

## 3. Merge strategy & conflicts

- One row per kanji character; **store provenance per field** (which source set the JLPT level /
  won the decomposition) — cheap now, invaluable for "this should be N2" reports.
- **JLPT level conflicts:** a kanji has one level; sources disagree on *which*. Priority:
  `davidluzgouveia jlpt_new` → forward-mapped KANJIDIC `jlpt_old` → unleveled. When two sources
  straddle the unreliable N2/N3 line, **assign the easier level (N3)** so the learner meets it
  sooner. Label levels in-app as "reconstructed (no official JLPT list exists)."
- **Decomposition conflicts:** KanjiVG wins for structure/position; KRADFILE sets naming
  granularity; CHISE IDS consulted at build time only; keep a small **hand override file** for the
  ~5% all sources get pedagogically wrong (reuse `scriptin/topokanji`'s prior-art overrides).

## 4. Critical pitfalls

1. **KANJIDIC2 `jlpt` field is the obsolete pre-2010 4-level scale (1–4, no N5).** NEVER use it
   for N5–N1. Use `jlpt_new`.
2. **No official JLPT kanji list exists** — reconstructed; N2/N3 split least reliable.
3. **Keyword:** first KANJIDIC2 gloss is unordered + partly Heisig-derived → use the cross-check
   chain + override table; expect collisions.
4. **Reading:** never "first KANJIDIC2 reading" (rare-reading trap); normalize rendaku/gemination
   before tallying or one reading fragments into several.
5. **Vocab:** reading↔kanji is constrained, not a cross-product — honor `re_restr` etc.
6. **Sentences:** Tanaka pollution; require translation + native owner; audio often unusably
   licensed.
7. **ShareAlike contagion:** derived data tables inherit **CC BY-SA + attribution**. The app
   *code* (MIT) is unaffected; the bundled *data* is not. (See [04](04-DATA-SOURCES.md).)
