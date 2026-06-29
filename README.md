# MnemoKanji

A small, fast, cross-platform Rust app for memorizing **all JLPT kanji** (N5 → N1) the
*efficient* way — built on proven memory science and a Heisig-style structural/mnemonic
approach, not naive single-character flashcards.

> Status: **working desktop app (JLPT N5)**. A complete study loop runs on macOS/Windows/Linux:
> dashboard, all four review modes (recognition, reading-in-context, write, cloze) on a two-track
> FSRS schedule, stroke-order animation, a kanji-detail hub with editable mnemonics, browse,
> settings, and one-tap undo — over a fully-built N5 dataset (79 kanji, verified mnemonics,
> readings, vocab, sentences, stroke data). Run it: `cargo run -p mnemokanji-ui`.
> (Content for N4–N1, audio, iOS, and packaged releases are the next milestones — see
> [docs/05-PLAN.md](docs/05-PLAN.md).)

## What makes it different from generic Anki decks

The method is evidence-led — see [docs/06-RESEARCH.md](docs/06-RESEARCH.md) for the cited
research behind every choice below.

- **One kanji = one concept, two schedules.** A character is one hub (one page, one story, one
  set of links) — but recognition and production are scheduled separately, because the evidence
  shows they're different memories and a single timer lets the harder writing skill silently rot.
- **Readings via phonetic components — the part most apps skip.** ~61% of kanji carry a component
  that signals the on'yomi (青→清晴請精 = *sei*). MnemoKanji surfaces these phonetic series with
  reliability tiers so predictable readings are learned nearly for free.
- **Consistent "actors."** Each recurring component and each reading-sound gets a fixed,
  reusable persona, so mnemonics compound across the whole curriculum instead of being one-offs.
- **Learn in context, not in isolation.** Readings and meanings are practiced inside real words
  and sentences; each reading lives on the word where it's actually used.
- **Structure is a feature.** Components form a graph (semantic *and* phonetic edges) used both to
  *order* learning (frequency-weighted, parts-before-whole) and to *reinforce* recall.
- **Distinctive mnemonics + stroke order, Heisig-style.** Every kanji gets an editable story that
  names and places its components (distinctiveness over forced bizarreness), plus animated
  stroke-order shown where it helps and hidden where it hurts.
- **Audio everywhere** and a "say it aloud" step (dual coding + production effect).
- **Gated progression.** Clear a level's comprehension to unlock the next (N5 → N4 → N3 → N2 →
  N1), with a briefing screen (count, difficulty, time) and a congratulations screen.
- **Modern scheduling.** Reviews are driven by **FSRS** (the algorithm Anki now defaults to),
  which predicts recall far more accurately than SM-2 and so trims redundant reviews.
- **Forgiving by design.** The #1 reason people quit kanji apps is the "review-debt death spiral."
  MnemoKanji ships a daily review cap, backlog smoothing, vacation mode, leech rescue, typo-tolerant
  input + undo, and test-out — so a missed week doesn't end your progress.
- **Rigorous *and* rewarding.** A humane streak, mastery/progress visualization, and a flow
  "challenge dial" — gamifying the *return* and the *mastery*, never the learning itself.
- **Offline-first & a market gap.** No other tool combines reading + writing + mnemonics + a
  phonetic-component reading system, fully offline. See [docs/09](docs/09-COMPETITION-AND-ENGAGEMENT.md).

## Platforms

End goal: **iPhone, macOS, Windows, Ubuntu** from a single Rust codebase (UI in
[Dioxus](https://dioxuslabs.com/)). v1 ships desktop-first (macOS / Windows / Linux); iOS
follows once the core is proven.

## Documentation

| Doc | What it covers |
|-----|----------------|
| [docs/01-VISION.md](docs/01-VISION.md) | The goal in detail, scope, target user, success criteria, non-goals |
| [docs/02-LEARNING-METHOD.md](docs/02-LEARNING-METHOD.md) | The memory science + Heisig adaptation + linking model, and how each maps to a feature |
| [docs/03-DESIGN.md](docs/03-DESIGN.md) | Architecture, tech stack, data model, scheduling, screens, build/release |
| [docs/04-DATA-SOURCES.md](docs/04-DATA-SOURCES.md) | Every data source, its license, and our attribution obligations |
| [docs/05-PLAN.md](docs/05-PLAN.md) | Milestones, deliverables, acceptance criteria, risks |
| [docs/06-RESEARCH.md](docs/06-RESEARCH.md) | The cited memory-science research and the design rethink it drove |
| [docs/07-CONTENT-GENERATION.md](docs/07-CONTENT-GENERATION.md) | Actor registries, the AI mnemonic generation+verification pipeline, quality rubric, scaffold-fade (DCRP) |
| [docs/08-DATASET.md](docs/08-DATASET.md) | Dataset assembly: source-per-field, keyword/reading/vocab/sentence selection algorithms, merge & conflicts |
| [docs/09-COMPETITION-AND-ENGAGEMENT.md](docs/09-COMPETITION-AND-ENGAGEMENT.md) | Competitive landscape, positioning, anti-burnout retention UX, engagement/reward design, voice-of-user |

## License

Application code: TBD (intended permissive — MIT or Apache-2.0). Bundled language data keeps
its upstream licenses (mostly CC BY-SA); see [docs/04-DATA-SOURCES.md](docs/04-DATA-SOURCES.md).
