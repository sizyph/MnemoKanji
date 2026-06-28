# 04 — Data Sources & Licensing

MnemoKanji ships only **openly licensed** language data, with required attribution, and
redistributes **no copyrighted third-party text**. This document is the authority on what we
use and the obligations attached.

## Sources we use

| Source | Provides | License | Obligation |
|--------|----------|---------|------------|
| **KANJIDIC2** (EDRDG) | Kanji, stroke counts, on/kun readings, English meanings, JLPT tags | CC BY-SA 4.0 | Attribute EDRDG; derived data stays share-alike. |
| **JMdict** (EDRDG) | Vocabulary, readings, glosses | CC BY-SA 4.0 | Attribute EDRDG; share-alike on derived data. |
| **KRADFILE / RADKFILE** (EDRDG) | Kanji → component/radical decomposition | EDRDG Licence (free use w/ attribution) | Attribute EDRDG. |
| **KanjiVG** (Ulrich Apel et al.) | Stroke-order vectors + component structure (SVG) | CC BY-SA 3.0 | Attribute KanjiVG; share-alike on derived SVG/data. |
| **Tatoeba** | Example sentences + translations (+ some sentence audio) | CC BY 2.0 FR | Attribute Tatoeba + sentence authors. |
| **Kanjium** | Phonetic-component data, frequency, extra readings | CC BY-SA 4.0 | Attribute; share-alike. **Base for our phonetic table.** |
| **Kanji Alive** | Radical names/meanings, positions (247 radicals) | CC BY 4.0 | Attribute. **Seed for component-actor naming.** |
| **KRADFILE-U** | Unicode-extended component decomposition | CC BY-SA 3.0 | Attribute; share-alike. |
| **kanjivg-radical** | KanjiVG ↔ radical mapping | CC BY-SA | Attribute; share-alike. |
| **davidluzgouveia/kanji-data** | **JLPT N5–N1 membership (`jlpt_new`)** | MIT | **Primary JLPT source** (based on Tanos). Credit Tanos/EDRDG too. |
| **JmdictFurigana** (Doublevil) | Per-kanji kana alignment | CC BY-SA / MIT | **Linchpin** for reading/vocab selection. Attribute under both. |
| **scriptin/kanji-frequency** | Kanji frequency (Wikipedia corpus) | CC BY 4.0 | Blend with KANJIDIC `freq` to fix newspaper bias. |
| **wordfreq** (rspeer) | Word frequency | Apache-2.0 / CC BY-SA | Primary word-freq ranker (frozen ~2021). |
| **hermitdave/FrequencyWords** | Colloquial word frequency | CC BY-SA | OpenSubtitles; supplements wordfreq. |
| **stephenmk/yomitan-jlpt-vocab** | JLPT level of *words* (by JMdict ID) | CC BY-SA | Join by ID, not string. |
| **(Optional) topokanji** | Reference topological kanji ordering | MIT | Attribute (permissive). |
| **CHISE IDS** | Decomposition cross-check | **GPLv2** | **Build-time reference ONLY — not bundled** (copyleft). |
| **Keisei** (mwil) | Phonetic-series + 天/上/中/下 reference | **GPL-3.0** | **Reference/validation only — NOT bundled** (see note). |

All EDRDG files are used under the EDRDG Licence: <https://www.edrdg.org/edrdg/licence.html>.

### Phonetic data — licensing strategy

The richest phonetic-component dataset (**Keisei**) is **GPL-3.0**, which is incompatible with
shipping inside an MIT app. So we **do not bundle Keisei**. Instead we **build our own phonetic
table from CC BY-SA Kanjium + KANJIDIC** (compatible with the rest of our data) and use Keisei
only at build time to *validate* our tiers. This keeps the shipped dataset clean.

### Audio

Readings audio is needed on every card. Options, in order of preference: (1) **platform TTS**
(macOS/iOS and Windows have good Japanese voices; offline), (2) **openly-licensed recordings**
where available (e.g. CC Tatoeba sentence audio), (3) a bundled offline TTS model if platform
voices prove insufficient (notably on Linux). Final approach is an **M3 open item**; no
proprietary/non-redistributable audio (e.g. Forvo) will be bundled.

## What WE create

- **Starter mnemonic stories & stroke hints** — generated originally from the open component
  decompositions (KRADFILE/KanjiVG), then editable by the user. These are **our original
  content**, not derived from Heisig. Marked `origin = generated` in the DB.
- **Topological learning order** — computed by us from the component graph (optionally
  cross-checked against topokanji).
- **Application code** — intended permissive (MIT or Apache-2.0), TBD.

## What we will NOT do

- **No redistribution of Heisig's *Remembering the Kanji* keywords or stories.** They are
  copyrighted. The PDF present in this repo (`James W. Heisig - Remembering Kanji ... Vol 1.pdf`)
  is the **owner's personal copy for private reference only**; it is **not** a data source, is
  **not** parsed into the app, and must **not** be committed to the public GitHub repo.
  → Action: add it to `.gitignore` before the repo goes public.
- No scraping of paywalled/closed dictionaries or proprietary deck content.
- **No copying of WaniKani radical names, Heisig keywords, or KanjiDamage keywords/mnemonics** —
  these are copyrighted. Our component-actor and reading-actor names are **authored originally**
  (optionally seeded from CC BY Kanji Alive radical data). We study those systems as *method*,
  not as a data source. See [07-CONTENT-GENERATION](07-CONTENT-GENERATION.md).

## License compatibility notes

- The bundled **content** mixes CC BY-SA (4.0 / 3.0) and CC BY sources. We keep content in a
  clearly delimited `assets/` area, preserve each source's notice, and treat **derived
  language data as CC BY-SA** (the most restrictive share-alike present). This is independent
  of the **code** license.
- Practically: the repo will carry a top-level `NOTICE`/`ATTRIBUTION.md` and the content
  pipeline emits a per-build attribution manifest into `assets/`.
- Because of share-alike, anyone redistributing our derived *data* must keep it open — which
  aligns with the project being free and public anyway.

## Attribution text (to ship in-app "About")

> Kanji and word data from KANJIDIC2 and JMdict © the Electronic Dictionary Research and
> Development Group, used under CC BY-SA 4.0. Component data from KRADFILE/RADKFILE © EDRDG.
> Stroke-order graphics from the KanjiVG project © Ulrich Apel, used under CC BY-SA 3.0.
> Phonetic-component data derived from Kanjium, used under CC BY-SA 4.0.
> Example sentences from Tatoeba, used under CC BY 2.0 FR. Mnemonic stories and reading/component
> "actors" are original to MnemoKanji and editable by the user.

## Open verification items (to confirm during M1)

- [x] **JLPT-level → kanji mapping: `davidluzgouveia/kanji-data` `jlpt_new`** (MIT, Tanos-based).
      ⚠️ The KANJIDIC2 `jlpt` field is the **obsolete pre-2010 4-level scale (1–4, no N5)** —
      never use it for N5–N1. Full assembly spec in [08-DATASET](08-DATASET.md).
- [x] **Tatoeba sentence selection:** via `jpn_indices.csv`; require translation + native owner +
      `~`/`OK`; length band; i+1. See [08-DATASET §2.4](08-DATASET.md).
- [x] Code license: **MIT** (decided).
- [ ] Audio approach (platform TTS vs bundled recordings/model) — confirm at M3.
