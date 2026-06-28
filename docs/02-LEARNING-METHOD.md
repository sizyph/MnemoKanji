# 02 — The Learning Method (v2, evidence-revised)

This is the heart of MnemoKanji. The app is a delivery vehicle for a specific, evidence-based
method. v2 reflects the research in [06-RESEARCH](06-RESEARCH.md) and the owner's decisions:
**two-track scheduling**, **comprehension-gated progression** (writing runs in parallel), and
**text stories + audio in v1** (images as a fast-follow).

## A. Foundations from memory science

The principles below are ranked roughly by strength of evidence (see [06-RESEARCH](06-RESEARCH.md)
for sources). We adopt the strong ones as pillars and treat the weak ones as minor tie-breakers.

| Principle | Strength | How MnemoKanji uses it |
|-----------|----------|------------------------|
| **Active recall (testing effect)** | Very strong | Every interaction is a retrieval *before* any reveal. No passive "study" cards count as reviews. |
| **Spaced repetition** | Very strong | **FSRS** schedules each track to its own forgetting curve. |
| **Transfer-appropriate processing / encoding specificity** | Strong | Practice in the form you'll need it → recognition and production are scheduled separately (§C). |
| **Generation effect** | Strong | Production & cloze modes require an actual attempt before reveal. |
| **Dual coding / picture superiority** | Strong | Verbal story + glyph + audio now; concrete image as fast-follow. |
| **Production effect (say it aloud)** | Strong, cheap | A selective "say the reading aloud" step. |
| **Interleaving** | Strong (for confusable items) | Sessions interleave kanji and rotate test modes. |
| **Desirable difficulties** | Strong | Spacing + forced production + scaffold fade keep retrieval effortful-but-possible. |
| **Self-reference / elaboration** | Moderate | Editable stories (editing is itself generative + self-referential). |
| **Distinctiveness (von Restorff)** | Moderate | Authoring rule: each story must stand out from its neighbors (§G). |
| **Bizarreness / emotional arousal** | Weak/fragile | Used *sparingly*, only for confusable/stubborn cards — never the house style. |

## B. The Heisig adaptation (shape + meaning via imaginative memory)

We adopt Heisig's *method* — decompose each kanji into recurring **components/primitives**, give
each a vivid concrete image, and bind them into a short story for the whole character — without
redistributing his copyrighted text (see [04-DATA-SOURCES](04-DATA-SOURCES.md)).

What we keep and sharpen:

1. **Components first.** A kanji is taught only after its parts are known (drives learning order,
   §H).
2. **One real keyword** per kanji, anchored to a genuine dictionary meaning (not an invented
   keyword); secondary meanings hang off it via vocab.
3. **A story that names and spatially places each component**, so recalling the story
   reconstructs the glyph (this is what makes the *write-the-kanji* mode work, and avoids
   Heisig's documented "rambling story" failure).
4. **Stroke order** learned in the production mode alongside the story.

Where we go beyond Heisig: we **do not postpone readings** (§E), we add **in-context practice**,
and we use **consistent actors** (§G) so stories compound across the curriculum.

## C. The unit of learning: one kanji hub, two schedules

> A kanji is **one concept** — one detail page, one story, one set of links, introduced once.
> But it is **scheduled on two independent FSRS tracks**, because recognition and production are
> different memories with *asymmetric transfer* (recall practice builds recognition, but not
> vice-versa). A single timer would let the harder writing skill silently decay.

- **Comprehension track** — input skills: *kanji → meaning* and *kanji → reading in context*.
- **Production track** — output skills: *meaning → write the kanji* and *in-context cloze /
  produce the reading*.

Rules:

- Each kanji has exactly these two tracks; each has its own FSRS state and due date.
- **Production is gated behind Comprehension maturity** — you don't try to *produce* a kanji
  until you reliably *recognize* it (asymmetric transfer makes recognition the prerequisite, and
  this avoids overwhelming early difficulty).
- **Level progression is gated on the Comprehension track only.** The Production track runs in
  parallel but **never blocks** advancing — reading literacy is never held hostage to
  handwriting. (Owner decision.)
- This honors "one kanji = one task" at the concept level (one hub the learner experiences)
  while being faithful to the evidence at the scheduling level.

## D. The four test modes → the two tracks

All four are *active recall*; together they exercise every direction a kanji must be known in.

| Mode | Track | Direction |
|------|-------|-----------|
| **Recognition** | Comprehension | kanji → meaning |
| **Reading (in context)** | Comprehension | kanji/word → pronunciation, always inside a word/sentence |
| **Production** | Production | meaning → *write* the kanji (must attempt before reveal) |
| **In-context cloze** | Production | produce the missing kanji/reading in a real sentence |

Which modes appear scales with each track's maturity; modes are **interleaved** within a session
even though they sit on two schedules. Within a track, the due modes are tested in one sitting and
the **worst** result drives that track's single FSRS rating (a kanji you can read but not interpret
isn't retained); the production track is also **deferred ≥1 day** when its comprehension track is
due the same day, to avoid priming. Mechanics in [03 §3](03-DESIGN.md).

## E. Readings: the phonetic-component system (the biggest lever)

Readings are the hardest part of kanji and the part Heisig ignores. ~61% of Jōyō kanji are
**phono-semantic**: a semantic component + a **phonetic component that signals the on'yomi**
(青 *sei* → 清 晴 請 精 静). MnemoKanji turns this hidden regularity into a teaching tool.

1. **Phonetic edges & series.** The component graph distinguishes **semantic** vs **phonetic**
   edges. Kanji sharing a phonetic component form a **phonetic series**.
2. **Reliability tiers (天/上/中/下).** Each kanji's phonetic prediction gets a confidence badge.
   We only *promise* a reading for the reliable 天/上 tiers; for 中/下 we explicitly warn the
   phonetic doesn't help.
3. **Front-load high-yield "perfect series"** in the learning order so a predictable on'yomi is
   taught nearly for free: "you know 青 = セイ, so 清 = セイ."
4. **Reading actors for the rest.** Phonetics cover only on'yomi, never **kun'yomi**, and not the
   low tiers — those fall back to mnemonics built on **reading actors** (§G).
5. **One dominant reading at the kanji level; the others through vocabulary.** We don't cram all
   readings onto one card; each additional reading lives on the **real word** where it's used
   (生活 *sei* / 生 *nama* / 生きる *i*), each its own context card.

Caveats we surface honestly: phonetics help on'yomi only; tiers vary; etymology is a *learning
heuristic*, not linguistic ground truth.

## F. The link graph (reinforcement through connections)

Three node types, two edge flavors:

- **Component** ──*semantic part of*──▶ **Kanji**
- **Component** ──*phonetic (→reading)*──▶ **Kanji**  *(new in v2)*
- **Kanji** ──*appears in*──▶ **Vocabulary** ──*occurs in*──▶ **Sentence**

Uses:

1. **Ordering** — teach parts before wholes; front-load high-yield phonetic series (§H).
2. **Elaborative reinforcement** — when studying a kanji, surface neighbors: "shares 青 (→セイ)
   with…", "appears in word…", "this component also means…". Each link is a free retrieval cue.
3. **Compositional jukugo** — show *why* a compound means what it does from its component
   keywords, and surface XY-vs-YX order; mark okurigana boundaries (食＊べる).

## G. Encoding rules (how stories, actors, and modalities are built)

These translate the evidence into concrete authoring/UX rules.

- **Component actors.** Each recurring component has **one persistent persona** (name + concrete
  image). The AI story generator must compose every kanji's story from these fixed actors — this
  makes stories consistent and compounding (the transferable kernel of Heisig primitives / PAO).
- **Reading actors.** Each common reading (e.g. コウ, セイ) has a **fixed mnemonic character/place**
  reused across all kanji with that reading, with **vowel length encoded into the actor** (short
  vowel = clipped name, long vowel = stretched) to disambiguate しょ/しょう for free.
- **Distinctiveness, not bizarreness.** Stories must be **concrete, interactive, multisensory,
  personally relevant, and spatially faithful** to the component layout. Reserve
  shock/absurd/surprising content for **confusable or stubborn** cards only. The editor shows a
  "make-it-stick" checklist; everything is editable (editing is itself a memory-positive act).
- **Modality segregation (cognitive-load rule).** Stroke animation and component highlighting can
  *hurt* recognition by overloading working memory, so: **recognition/reading prompts = clean
  glyph + audio only.** Animation, decomposition, and highlighting appear **only in the
  production/writing mode and on the post-answer reveal.** Never stack all modalities on one
  screen.
- **Confusable grouping.** Deliberately group visually similar look-alike kanji (科/料, 寺/時,
  未/末) and teach the *discrimination* between them — a high-payoff use of distinctiveness +
  interleaving that almost no app does (Kanji Garden being the exception). Also a leech-rescue
  tactic ([09 §4](09-COMPETITION-AND-ENGAGEMENT.md)).
- **Audio everywhere.** Native-quality audio of readings on every card (dual coding); a
  **selective** "say it aloud" step (production effect is strongest in mixed lists).
- **Scaffold fade = graduated, reversible Diminishing-Cues Retrieval Practice (DCRP).** Mnemonics
  speed acquisition but *forget faster unless paired with retrieval practice* — so the story must
  fade in stages, not vanish at once: **full story → partial cue (pivot word) → "recall the story
  yourself" → faded (direct recall, story available on tap as rescue).** Advance a stage after N
  consecutive **fast, correct** reviews; **demote on a lapse.** **Retrieval latency** is the
  crutch signal — fast+correct+stable ⇒ fade; slow-but-correct ⇒ hold. (Details + the generation
  pipeline and actor registries are in [07-CONTENT-GENERATION](07-CONTENT-GENERATION.md).)

## H. Learning order

- **Across levels:** JLPT **N5 → N4 → N3 → N2 → N1** (matches the goal; milestone motivation;
  bounded scope).
- **Within a level:** a **frequency-weighted topological sort** over the component graph
  (topokanji / Yu 2016): among all kanji whose components are already known, teach the most
  frequent next — giving *both* parts-before-whole *and* common-first.
- **Just-in-time component cards** for parts that aren't themselves in-level kanji (introduced,
  then reinforced through the kanji that use them).
- **Pinned early wins:** numbers, days, 日 月 人 大 小 … pinned to the very front of N5 for day-1
  success (the sort surfaces most of them anyway).
- **Vocabulary at introduction:** each new kanji is introduced with one high-frequency word that
  uses it, for immediate context.

## I. The production / writing approach

Writing belongs in the product but is scoped to avoid the "Save Your Strokes" opportunity-cost
trap (over-weighting copying hurts when the goal is recognition):

- Default reviews are recognition/reading; **writing is its own (Production) track**, plus at
  first exposure.
- Primary interaction: **self-graded recall-then-reveal** — prompt → write from memory (paper or
  screen) → reveal glyph + KanjiVG animation → self-grade. Captures the production/retrieval
  benefit cheaply.
- Optional richer mode: a **canvas** checking stroke count and rough order against KanjiVG (not
  pixel-perfect matching, which only frustrates).
- Scaffold progression: **watch → trace → recall → check**, with *recall* doing the memory work.

## J. Session shape

A daily session, interleaved:

1. **Due reviews first** across both tracks, ordered by FSRS urgency, each presented through
   maturity-appropriate modes; linked-neighbor cues shown after grading.
2. **New introductions** from the currently unlocked level only, gated by component
   prerequisites: breakdown → (editable) story with actors → audio → first recognition test.
   Production for that kanji unlocks later, once its Comprehension track matures.
3. **Mode rotation** so no single direction is over-practiced.
4. **Level guardrails:** clearing the Comprehension mastery threshold for ~90% of the level
   triggers congratulations + unlock of the next level.

## K. What we explicitly avoid

- Passive flip-through review (no retrieval = weak learning).
- A single schedule for recognition and production (lets the hard skill rot).
- Isolated reading drills (reading with no word/sentence) and reading mega-cards.
- Memory-palace machinery as the curriculum backbone (wrong tool for open-ended random access).
- "Make everything weird" mnemonics (distinctiveness, not blanket bizarreness).
- Stroke animation / highlighting on recognition prompts (cognitive overload).
- Letting production mode be flip-and-self-grade without an actual attempt.
- Invented English keywords detached from real meaning.
