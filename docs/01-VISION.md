# 01 — Vision & Goal

## The goal, in one sentence

Build **MnemoKanji**: a small, fast, offline-first Rust app that lets a learner memorize
**all JLPT kanji from N5 to N1**, durably and efficiently, by combining proven memory-science
techniques with a Heisig-style structural/mnemonic method and context-based practice — far
more effective than plain one-kanji-one-card flashcards.

## Why this exists (the problem)

Most kanji study tools fall into one of two traps:

1. **Naïve flashcards** — one card per kanji showing a single English keyword. They ignore
   that a kanji has *multiple* readings and *multiple* meanings, teach characters in
   isolation (so readings never stick in real words), and provide no help remembering the
   *shape* or *stroke order*.
2. **Pure mnemonic books** (e.g. Heisig's *Remembering the Kanji*) — excellent for memorizing
   shape ↔ keyword, but deliberately postpone readings and context, and are static paper with
   no spaced-repetition scheduling.

MnemoKanji deliberately fuses the strengths of both and adds a scheduler, so a single tool
covers **shape, meaning(s), reading(s), and in-context usage**, scheduled for long-term
retention.

## Target user

- **Primary:** Gildas — comfortable in English & French, self-studying for the JLPT, on
  macOS/iPhone day-to-day, with Windows/Ubuntu also in use.
- **Secondary (public release):** any self-learner who wants a free, open, efficient kanji
  trainer across desktop and mobile.

## Scope

### In scope (the product)

- **All five JLPT levels**, learned in **gated order: N5 → N4 → N3 → N2 → N1.**
  - Finishing a level **unlocks** the next one (a level is "finished" per a defined mastery
    threshold — see [03-DESIGN](03-DESIGN.md)).
  - Completing a level shows a **congratulations / level-up** screen.
  - Starting a level shows a **briefing screen**: how many kanji, relative difficulty,
    estimated time to complete, what's new vs. previous levels.
- **One kanji = one concept/hub** (one page, one story, one set of links), but **scheduled on
  two tracks** — Comprehension and Production — because recognition and production are different
  memories (see [02-LEARNING-METHOD §C](02-LEARNING-METHOD.md)).
- **Four review/test modes** mapped to those tracks (see [02-LEARNING-METHOD](02-LEARNING-METHOD.md)):
  recognition (kanji → meaning), reading (kanji/word → pronunciation, in context), production
  (meaning → write the kanji), and in-context sentence cloze.
- **Phonetic-component reading system** — exploit that ~61% of kanji carry a component signalling
  the on'yomi (青→清晴請精 = *sei*), with reliability tiers, to teach the hardest part (readings)
  efficiently. The structural lever most apps miss.
- **Consistent "actors"** for recurring components and readings, so mnemonics compound across the
  whole curriculum rather than being one-off.
- **Structural breakdown + distinctive mnemonics + animated stroke order** for every kanji, with
  stories **AI-generated as a starting point and fully editable**.
- **A link graph** (semantic *and* phonetic edges) between components, kanji, and vocabulary,
  used to order learning and reinforce recall.
- **Native-quality audio** of readings on every card.
- **FSRS-based spaced repetition** scheduling (desired retention ~0.90).
- **Anti-burnout retention UX** — the #1 reason people quit kanji apps is the "review-debt death
  spiral." MnemoKanji makes graceful survival a default: daily review cap, backlog smoothing,
  vacation mode, decoupled lesson/review queues, active leech rescue, forgiving input + undo, and
  placement/test-out. (See [09-COMPETITION-AND-ENGAGEMENT §4](09-COMPETITION-AND-ENGAGEMENT.md).)
- **Fun + reward, safely** — a humane streak (with slack), mastery/progress visualization,
  milestone celebrations, and a flow "challenge dial" — *gamifying the return and the mastery,
  never the learning itself* ([09 §5](09-COMPETITION-AND-ENGAGEMENT.md)).
- **Offline-first local storage**, plus **file export/import** for backup/manual sync, with
  **optional cloud account sync** as a later milestone.
- **Cross-platform**: macOS, Windows, Ubuntu (v1), then iPhone. Public releases on GitHub.

### Out of scope (at least for v1)

- Full Japanese course (grammar lessons, listening, full vocab SRS beyond what supports kanji
  context). MnemoKanji is a **kanji** trainer, not a complete JLPT prep suite.
- Automatic handwriting *recognition/grading* by ML. Production mode uses self-grading and an
  optional trace-against-reference canvas, not a handwriting recognizer (revisit later).
- **Concrete per-kanji illustrations** in v1. Strong evidence supports them (picture
  superiority), but they're a **fast-follow** after the engine works; v1 ships text stories +
  audio + stroke animation. (Owner decision.)
- Social features, leaderboards, marketplace of decks (community mnemonic voting is a later
  layer, not v1).
- Redistribution of any copyrighted third-party content (notably Heisig's actual stories —
  see [04-DATA-SOURCES](04-DATA-SOURCES.md)).

## Definition of "the most efficient way to remember"

We operationalize "efficient" as maximizing **long-term retention per minute studied**. The
design therefore commits to techniques with the strongest evidence base (active recall,
spaced repetition, elaboration, dual coding, interleaving, generation effect, desirable
difficulties) and to a scheduler (FSRS) that minimizes redundant reviews. The mapping from
each principle to a concrete feature is the subject of [02-LEARNING-METHOD](02-LEARNING-METHOD.md).

## Success criteria

**v1 (desktop, N5 content) is successful when:**

1. The app launches on macOS, Windows, and Ubuntu and runs fully offline.
2. The full **N5** kanji set is loaded with: components, ≥1 reading and meaning each, an
   editable mnemonic, animated stroke order, and ≥1 example sentence each.
3. A daily review session works end-to-end across all four test modes, scheduled by FSRS,
   treating each kanji as one linked item.
4. Level gating works: N5 must be cleared to unlock N4; briefing + congratulations screens
   appear.
5. Progress persists locally and can be exported to and re-imported from a file.

**Product (full vision) is successful when:** all five levels are populated, iPhone build is
released, and the user can study any unlocked level seamlessly across their devices.

## Guiding principles

- **Small and fast.** A focused app, not a framework. Native feel, instant startup, low
  memory.
- **Offline-first & private.** Your study data is yours and works on a plane.
- **Evidence over folklore.** Every learning feature traces back to a documented principle.
- **Editable, not prescriptive.** Generated mnemonics are a starting point; the best mnemonic
  is one you make your own — so everything is editable.
- **Legally clean data.** Only openly licensed sources, with attribution; no copyrighted text
  redistributed.
- **Rigorous *and* rewarding.** The market is split between engaging-but-shallow and
  rigorous-but-joyless apps; MnemoKanji refuses the trade-off. **Learning-quality north star:**
  *long-term retention of mature kanji per study-minute* — no engagement feature may degrade it
  ([09 §8](09-COMPETITION-AND-ENGAGEMENT.md)).

## Positioning

> MnemoKanji = WaniKani's teaching & structure, minus the rigidity and review-debt punishment;
> Anki's flexibility, minus the setup pain — kanji learned in real context, with a phonetic
> reading system nobody else has, forgiving catch-up that survives a missed week, offline, full
> N5→N1, at a fair price. Full landscape in [09-COMPETITION-AND-ENGAGEMENT](09-COMPETITION-AND-ENGAGEMENT.md).
