# 07 — Content Generation: Actors, Mnemonics & Quality

The mnemonics are the part MnemoKanji *invents* (Heisig/WaniKani hand-author theirs; ours are
AI-generated then user-edited). This doc specifies how we make that content reliably good at
scale. It is grounded in the second research pass (see [06-RESEARCH §Second pass](06-RESEARCH.md))
— notably the SMART (EMNLP 2024), kanji-EM (EMNLP 2025), and PhoniTale (2025) papers, plus the
WaniKani/KanjiDamage/Heisig practitioner record.

## 1. Core principle: actors are pre-resolved input, not LLM invention

The single biggest reliability lever from the research: **do not let the LLM choose component or
reading keywords per kanji.** The kanji-EM paper found learners interpret the same component
keyword differently ("spoon" = utensil vs cuddling position), and guided LLMs only hit ~61% rule
compliance. So we maintain two curated **actor registries** and feed them to the generator as
fixed inputs.

### 1.1 Component-actor registry (~300 entries)
- One persistent, **concrete, imageable persona** per recurring component (Heisig uses ~229 named
  primitives; WaniKani ~480 radicals; we budget ~300, plus **reuse already-learned kanji as
  actors** to cap the count).
- Prefer **agents/characters** (can "act" in a story) over abstract nouns.
- Namespace: **things / places / creatures** (kept disjoint from reading actors — see 1.3).
- We **author our own names** — WaniKani's radical names, Heisig's keywords, and KanjiDamage's
  keywords are copyrighted. Seed allowed from CC sources (Kanji Alive radical names, CC BY 4.0).

### 1.2 Reading-actor registry (~400 entries; top ~50 cover most kanji)
Model: **KanjiDamage** — one fixed persona per on'yomi sound, reused across every kanji with that
reading. Upgraded to *characters* (WaniKani-style) so the persona can recur as a story agent.

- **Persona name evokes the sound** (こう → "Coe"; しょう → "Showman").
- **Vowel length encoded into the persona** (the #1 reading error, しょ vs しょう):
  - **Long vowel = full/stretched persona** (the default — long dominates ~80% of on'yomi).
  - **Short vowel = clipped/"shrunken twin"** of the long persona (KanjiDamage spells short ones
    as acronyms to signal brevity). Surface the heuristic "guess long except for **FU, KU, SO**."
- **Dakuten/voicing (k→g, s→z, t→d, h→b) = "evil/heavier twin"** of the base actor (か CAR → が
  growling car), not a separate registry entry — roughly halves the registry and teaches the
  systematic relationship. **Handakuten (h→p)** likewise.
- **Rendaku & gemination are contextual** (compound-position phenomena) → handled at the
  **vocabulary/sentence layer** as "costume modifiers," NOT baked into a kanji's reading actor.
- Namespace: **named people** (disjoint from component actors).

### 1.3 Kun'yomi are excluded from the reading registry
Phonetic actors pay off only where the same on'yomi recurs across many unrelated kanji. **Kun'yomi
(native readings) get no reading actor** — they're anchored to the kanji's **meaning/component
story** and consolidated through real vocabulary (川 かわ via the word, not a "kawa" actor). This
on'yomi-vs-kun'yomi split is a deliberate design decision.

## 2. The generation pipeline

Five stages; stages 0 and 5 are human-in-the-loop, 1–4 are automated.

- **Stage 0 — Curate the registries (one-time, human).** Build/curate the ~300 component and
  ~400 reading actors with collision checks (no persona shared across registries). The registries
  are the generator's controlled vocabulary; quality here propagates to every story.
- **Stage 1 — Deterministic decomposition (no LLM).** From the kanji DB, assemble structured
  input: real keyword/meaning, ordered component list (stroke-order reading direction), each
  component's actor, the spatial relation, the dominant reading + its reading actor.
- **Stage 2 — Constrained generation.** Prompt rules (from the WaniKani GPT-4 experiment + the
  papers):
  - Use **only** the supplied actors; **name every one**; introduce **no** component not supplied.
  - Weave in the reading actor (for reading stories).
  - Put the **meaning at the start or end, never buried**.
  - Stage the scene to reflect the **components' spatial layout**.
  - **Concrete, one vivid/striking image, ≤ ~40 words, no filler/abstraction/rambling.**
  - **No etymology claims** — "this is an imaginative aid, not history."
  - **Output JSON**: `{ story, actors_used[], reading_actor_used, meaning_placement }`, with each
    actor delimited in the story text for highlighting + verification.
  - Few-shot with 2–3 **curated human gold** examples (LLMs are weakest at simplicity/imageability
    — anchor on human style).
- **Stage 3 — Verify before ship (two gates).**
  1. **Deterministic verifier (code, free):** all supplied actors present; no phantom components;
     reading actor present; length OK; meaning at start/end. Fail → auto-regenerate (≤ N tries).
  2. **LLM-judge (separate call), rubric §3, evidence-citing:** must pass all *gating* dimensions;
     borderline → human queue; fail → regenerate with the critique appended.
- **Stage 4 — Batch & cost.** Generate per JLPT level (respects component-before-kanji order).
  Strong model for first draft + judge, cheap model for retries. Full ~2,200-kanji corpus is a
  one-time cost in the tens of dollars at 2024 GPT-4 prices; spend the savings on curation.
- **Stage 5 — Ship as editable draft + close the loop.** Every story ships **editable, never
  canonical**. The trustworthy quality signal is **observed review performance** (recall success,
  latency, lapses) — **NOT stars** (SMART/PhoniTale: expressed ≠ observed preference, r≈−0.06).
  Underperforming stories and **weak actors** (an actor that drags down every kanji using it) go
  back to the queue / registry for revision.

## 3. Quality rubric

The generator must satisfy this; the in-app **"make-it-stick" editor** surfaces the starred
items live as the user edits.

**Gating (binary — must pass to ship):**
1. ★ **Component coverage** — every supplied component actor appears. *(top failure mode)*
2. **No phantom components** — nothing named that isn't in the kanji.
3. ★ **Reading actor present** — for reading stories.
4. ★ **Meaning lands** — keyword is the payoff, at start or end, not buried.
5. **Actor consistency** — actors match the registry persona exactly.
6. **No false etymology** — framed as imaginative aid.
7. **Length & focus** — ≤ ~40 words, no extraneous detail.

**Graded (1–5 — drives ranking/curation):**
8. **Concreteness** · 9. ★ **Imageability** (weight heavily — LLM weak spot) ·
10. **Distinctiveness** (unlikely to interfere with neighbors) · 11. ★ **Spatial fidelity** ·
12. **Coherence** · 13. **Conciseness / low load** · 14. **Cultural & content safety**
(never trade safety for recall, even though offensive mnemonics can aid recall).

## 4. Editing & personalization (harvesting the generation effect)

Research: for hard material, provided ≈ self-made **once retrieval practice is added** — so
AI-generate-then-edit is the right call (don't force from-scratch authoring). But editing is
**constrained generation** and captures part of the generation effect for free, so:

- Make **editing a first-class, encouraged action**, not buried.
- Nudge **self-reference**: "make it *yours* — swap in a person/place you know" (the strong
  self-referential variant of the effect).
- Track **edited-vs-default** per card (a quality + engagement signal).

## 5. Scaffold fade = Diminishing-Cues Retrieval Practice (graduated & reversible)

Mnemonics speed acquisition but **forget faster unless paired with retrieval practice** — FSRS
supplies that, and the scaffold must **fade**. The evidence (DCRP: Finley & Benjamin) says fade
**graduated and reversible**, not a one-shot hide:

- **Stage 0 (learning):** full story shown.
- **Stage 1 (partial cue):** show only the pivot word / first line.
- **Stage 2 (prompt-only):** "recall the story yourself" before reveal (the story becomes a
  retrieval target).
- **Stage 3 (faded):** direct kanji→meaning; story available on tap as a "rescue" (= feedback).
- **Transitions gate on FSRS signals, not card count:** advance after **N consecutive fast,
  correct** reviews at the current stage; **demote on a lapse** (re-show more scaffold).
- **Crutch detection = retrieval latency.** A direct route is fast; a mnemonic-mediated route is
  slow. Fast + correct + stable ⇒ fade; slow-but-correct ⇒ hold (still leaning on it). Optionally
  a one-tap "I don't need the story anymore."

Implication for data model: each track stores a **`scaffold_stage`**, and `ReviewLog` records
**`elapsed_ms`** (latency) so fade decisions and later analysis are possible.

## 6. Open content questions (resolved during M1/M3)

- Which LLM to generate with (Claude is the in-house default) and the exact gold-example set.
- Whether to add a small **image/emoji** per kanji sooner than the general image fast-follow —
  pictures were the one intervention that improved *character* durability, especially for
  abstract-meaning kanji.
- Final per-registry persona style guide (tone, content-safety bounds).
