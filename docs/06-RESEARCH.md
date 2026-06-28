# 06 — Research Findings & Design Rethink

A deliberate, evidence-led re-examination of MnemoKanji's method before any code. Five parallel
research passes covered: (A) competitive/expert memory techniques, (B) the cognitive science of
durable memory, (C) best-in-class kanji apps, (D) phonetic-component reading prediction, and
(E) learning order + multisensory/kinesthetic encoding. This document records what the evidence
says, what we keep, and what we change. Sources are listed at the end.

> Stance (per the owner's instruction): **trust the evidence over intuition.** Where a popular
> idea (including the owner's "sequential castle") is weakly supported, we say so and adapt.

## 0. What the evidence confirms we already had right

- **Spaced repetition + active retrieval** are the two highest-utility, best-evidenced learning
  techniques (Dunlosky 2013). FSRS is the correct modern operationalization. ✔ keep.
- **Component decomposition + a vivid story per kanji** (Heisig's core) is the strongest, most
  directly transferable mnemonic for shape↔meaning. ✔ keep.
- **In-context practice** (vocab + sentences) fixes Heisig's biggest flaw (isolation from real
  usage) and is supported by encoding-specificity / transfer-appropriate processing. ✔ keep.
- **Editable mnemonics** are doubly justified: editing is itself a generation + self-reference
  act (both memory-positive). ✔ keep.

## 1. The big changes (high-impact, evidence-driven)

### 1.1 Split scheduling into two tracks: Comprehension vs Production
**Evidence:** recognition and recall are dissociable memories; transfer is **asymmetric** —
recall practice improves later recognition, but recognition practice does *not* reliably produce
recall/writing (encoding specificity; transfer-appropriate processing; Rowland; Roediger lab).
A single schedule keyed to easy recognition lets the hard production skill decay silently
("I know it when I see it but can't write it").

**Change:** keep **one kanji = one concept/hub** (one detail page, one story, one set of links,
introduced once), but schedule it on **two FSRS states**:
- **Comprehension track** — kanji→meaning + kanji→reading-in-context (input).
- **Production track** — meaning→write-kanji + in-context cloze/produce-reading (output).

Gate the production track behind comprehension maturity (asymmetric transfer makes recognition
the natural prerequisite). This is the single most important revision. *(Touches the owner's
"one kanji = one task" requirement — see decision D1 in §3.)*

### 1.2 Demote "memory palace / sequential castle" from backbone to niche tool
**Evidence:** method of loci is superb for **fixed, ordered, arbitrary sequences recalled in
order** (digits, cards; Dresler 2017) and **does not generalize** beyond that access pattern.
Kanji learning is open-ended (2,000+), unordered, and **random-access** (you see a kanji and
need its meaning/reading, or vice-versa) — the access pattern loci is worst at. SRS already owns
scheduling/sequencing.

**Change:** do **not** organize the curriculum as a palace. Offer loci only as an **optional,
scoped tool** for *visually confusable look-alike kanji* (park a confusable set in one vivid
"room" to keep them distinct — a legitimate distinctiveness use).

### 1.3 Make READINGS first-class via phonetic components (the biggest new lever)
**Evidence:** ~61% of Jōyō kanji are phono-semantic ("keisei") — a semantic part + a **phonetic
part that signals the on'yomi** (青 SEI → 清 晴 請 精 静). Reliability is tiered (天/上/中/下).
The hardest part of kanji (readings) — the part Heisig skips entirely — has hidden structure we
can exploit.

**Change:** add a **phonetic edge type** to the component graph and a **phonetic-series** model.
On a kanji's page show "Phonetic: 青 → セイ" with a 天/上/中/下 confidence badge. **Front-load
high-yield "perfect series"** so a predictable on'yomi is taught nearly for free
("you know 青=セイ, so 清=セイ"). Fall back to mnemonics for kun'yomi and low-tier cases.
Data: build our own table from **CC-BY-SA Kanjium + KANJIDIC**, validated against the GPL-3.0
**Keisei** dataset (reference only — do not bundle GPL data into an MIT app).
Caveats: phonetics help **on'yomi only**, never kun'yomi; only the 天/上 tiers are "free."

### 1.4 Consistent "actors" for components AND readings
**Evidence:** the genuinely transferable kernel of PAO/Heisig-primitives is **a stable, vivid
persona per reusable symbol**. WaniKani's reading "actors" (a fixed character per reading sound)
turn N independent memorizations into reuse of a shared cast — its biggest advantage.

**Change:** two registries, both first-class data:
- **Component actors** — each recurring component gets one persistent persona (name + image);
  the AI story generator must compose each kanji's story from these fixed actors.
- **Reading actors** — each common reading (e.g. コウ, セイ) gets a fixed mnemonic
  character/place reused across all kanji with that reading. Encode **vowel length into the
  actor** (KanjiDamage: short vowel = clipped name, long vowel = stretched) to disambiguate
  しょ/しょう for free.

### 1.5 Distinctiveness, not bizarreness; with scaffold fade
**Evidence:** the "make it weird" folklore is **weak and fragile** — bizarreness helps only when
an item is *distinctive within its set* and can wash out or backfire at delay; **distinctiveness**
(von Restorff) and **concrete, interactive, personally-relevant imagery** are what work.
Keyword mnemonics reliably help **acquisition** but are **not** a durability solution alone —
durability is SRS's job — and they help most when they lift first-recall above ~50%.

**Change:** rewrite the AI mnemonic prompt around **concrete + interactive + multisensory +
personally relevant + spatially faithful to the component layout** (story must *name and place*
each component — avoids Heisig's "rambling story" failure). Reserve shock/absurd content for
**confusable or stubborn** cards only. Add a "make-it-stick" checklist to the story editor.
Implement **scaffold fade**: show the story on a card's first 1–2 reviews, then withhold it; add a
"recalled without the story" signal that retires the mnemonic (the mnemonic is a bridge you cross
and leave).

### 1.6 Segregate modalities by task (cognitive-load rule)
**Evidence (counterintuitive, well-supported):** stroke-order **animation and radical
highlighting can *reduce recognition accuracy*** by overloading working memory / splitting
attention (Zhang 2022). Multimodal input helps only when it doesn't exceed bandwidth.

**Change:** **recognition/reading prompts = clean glyph + audio only** (no animation, no
component highlighting on the prompt). Show animation, decomposition, and highlighting **only in
the writing/stroke-order mode and on the post-answer reveal.** Never stack all modalities on one
screen.

### 1.7 Audio everywhere + selective "say it aloud" (promote from deferred to core)
**Evidence:** dual coding / picture-superiority (verbal+visual+motor codes are additive); the
**production effect** (reading aloud >> silent) is robust and cheap; shadowing aids
pronunciation/prosody. Readings are half the task — audio isn't optional.

**Change:** native-quality **audio on every card** (TTS or recorded). Add a **selective**
"say the reading aloud" step (selective because the production effect is strongest in mixed
lists). Optional shadowing later. A **concrete image** per kanji/component (picture superiority)
is high-value — see decision D2 in §3 (scope/cost).

### 1.8 Writing: decouple from default review; recall-then-reveal first
**Evidence:** a preregistered RCT ("Save Your Strokes") found heavy hand-copying *worse* than
recognition practice **when the goal is recognition**, and the writing advantage faded by a week
— an **opportunity-cost** result, not "writing is useless." Handwriting genuinely strengthens
the orthography→meaning mapping and transfers to reading; typing strengthens phonology→meaning.

**Change:** don't tax every review with writing. Default reviews are recognition/reading; writing
is its **own mode** + at first exposure. Primary interaction: **self-graded recall-then-reveal**
(prompt → write from memory → reveal glyph + KanjiVG animation → self-grade), which captures the
production/retrieval benefit cheaply. Canvas with light stroke-count/order validation (not
pixel-matching) is the optional richer mode. Scaffold: **watch → trace → recall → check**, with
recall doing the memory work. *(Whether writing gates level progression — decision D1/§3.)*

### 1.9 Force generation in production mode
**Evidence:** the generation effect requires actually *producing* an answer (even a wrong one +
feedback). Flip-and-self-grade without producing forfeits the main benefit.

**Change:** production/cloze modes **require an attempt before reveal.**

### 1.10 Learning order: frequency-weighted topological sort within each level
**Evidence:** pure component order (RTK) maximizes mnemonic leverage but delays useful kanji and
hurts motivation; pure frequency shows wholes before their parts. The **topokanji / Yu 2016**
approach — a topological sort over the component DAG **weighted by frequency** — gives both
(parts-before-whole AND common-first) and beats both pure orders.

**Change:** top-level partition by **JLPT N5→N1** (matches the goal + milestone motivation);
**within each level**, order by frequency-weighted topological sort over the component graph.
Add **just-in-time "component cards"** for parts that aren't themselves in-level kanji.
**Pin early high-utility kanji** (numbers, days, 日月人大小…) to the very front of N5 for day-1
wins. **Pair each new kanji with one high-frequency vocab word** at introduction (immediate
context; leverages the kanji→vocab graph).

### 1.11 Readings & keywords data model
**Evidence:** cramming all readings onto one mega-card causes burnout; the winning pattern routes
each reading through a **real vocabulary word** where it's actually used. Heisig's invented
English keywords mislead.

**Change:** promote the **dominant on'yomi** to the kanji-level reading; route **other readings
through their own vocab cards** in reading/cloze modes. Anchor each kanji's keyword to a **real
dictionary meaning**, marking primary vs. extended meanings (no invented keywords).

## 2. Smaller refinements

- **Gating threshold:** unlock the next level at **~90% of the level's kanji at "mature"**, not
  100% (avoids one stubborn kanji blocking progress); **components must mature before their
  kanji unlock**; allow **test-out** for prior knowledge (WaniKani's main complaint).
- **FSRS desired retention:** default **0.90** (lean 0.88–0.90; forgetting a shared component
  cascades, so retention costs are high). Offer per-user optimization after ~1,000 reviews.
  Do **not** assume hand-tuned "expanding intervals" — FSRS targets recall probability adaptively.
- **Compositional jukugo display:** show *why* a compound means what it does from its component
  keywords, and surface XY-vs-YX order. Mark okurigana boundaries (e.g. 食＊べる).
- **Community voting over AI mnemonics** (later): AI solves cold-start; voting (Koohii-style
  top-3 + fresh) solves quality drift. Stories editable per-user from day one.

## 3. Decisions for the owner (these touch earlier choices)

- **D1 — Scheduling model & writing's role.** Adopt the **two-track (Comprehension /
  Production)** schedule (§1.1)? And should **level progression gate on Comprehension only**,
  with the Production/writing track running in parallel but **not blocking** unlock (so
  handwriting effort never blocks reading literacy)? *Recommended: yes to both.*
- **D2 — Mnemonic images.** Add a **concrete generated image** per kanji/component (strong
  picture-superiority evidence) in v1, or ship **text stories + audio** first and add images
  later (lower cost/scope)? *Recommended: text + audio in v1; images as a fast-follow.*

## 4. Honesty corrections

- The "FSRS needs 20–30% fewer reviews than SM-2" figure is a **simulation/derivation** from
  FSRS's superior recall-probability prediction (validated on 350M+ reviews), **not** a
  head-to-head RCT. FSRS is the right choice; we won't present the exact percentage as measured.
- "Expanding retrieval intervals are optimal" is **not** well-supported at long delays; placement
  of the *first* review matters more. FSRS already handles this.

## 5. Net effect on the product

The revised method keeps the proven core (SRS + retrieval + decomposition + in-context) and adds
three structural advantages most apps lack: **(1)** phonetic-component reading prediction for the
hardest part of kanji, **(2)** consistent component/reading actors that compound across the whole
curriculum, and **(3)** evidence-tuned encoding rules (two-track scheduling, modality
segregation, distinctiveness-over-bizarreness, scaffold fade, forced generation). Together these
target *long-term retention per minute*, the project's stated definition of "efficient."

## 6. Second research pass — content generation, actors, toolchain

A focused follow-up on the parts most likely to make or break the product (AI mnemonic quality,
the actor system, and the Rust toolchain). Full design in [07-CONTENT-GENERATION](07-CONTENT-GENERATION.md).

### 6.1 LLM mnemonic generation is feasible and has academic precedent
- **SMART** (EMNLP 2024), a **kanji-specific EM** paper (EMNLP 2025), and **PhoniTale** (2025)
  all generate vocabulary/kanji/reading mnemonics with LLMs. Key lessons: feed component keywords
  *in* (don't let the model invent them — guided LLMs hit only ~61% rule compliance); LLMs are
  systematically **weak at simplicity and imageability** (a pro human still beats them there);
  and — critically — **what learners *say* helps barely correlates with what *actually* helps**
  (r≈−0.06). ⇒ instrument **observed review performance**, not stars, as the quality signal.
- Practitioner precedent (WaniKani GPT-4 generator, ~$0.03/kanji) confirms the winning recipe:
  **constrain to supplied components, name them all, keep meaning at start/end, output JSON for
  validation**, and verify with a deterministic check + an LLM-judge before shipping.

### 6.2 Actor registries: size, vowel length, voicing, kun'yomi
- Budget **~300 component actors** (reuse learned kanji as actors) and **~400 reading actors**
  (top ~50 cover most kanji). **KanjiDamage** is the template: one keyword per on'yomi, with
  **vowel length encoded** (long = full word/default ~80%; short = clipped/acronym; "guess long
  except FU/KU/SO"). **Dakuten = "evil twin"** transform (halves the registry); rendaku/gemination
  handled at the vocab layer; **kun'yomi excluded** from the reading registry (anchored to
  meaning). Keep component vs reading actors in **disjoint namespaces** to avoid collisions.

### 6.3 AI-generate-then-edit is the right model; scaffold fade = DCRP
- For hard material, **provided ≈ self-made mnemonics once retrieval practice is added** (imposed
  vs induced was statistically null in an L2 keyword+testing study). ⇒ keep AI-generate-then-edit;
  make **editing first-class** and nudge **self-referential** personalization to harvest the
  generation effect cheaply.
- **Durability caveat:** mnemonics speed acquisition but **forget faster unless paired with
  retrieval practice** (Thomas & Wang; replicated for Chinese characters; Dunlosky rates the
  keyword method "low utility" for exactly this reason). FSRS is what rescues durability — and
  the scaffold must fade.
- **Scaffold fade has direct support as Diminishing-Cues Retrieval Practice (DCRP)** — but
  **graduated and reversible** (full → partial cue → prompt-only → faded; demote on lapse), gated
  on **latency + repeated cold success**, not a single recall. **Retrieval latency** is the key
  crutch-detection signal. *(Updates §1.5 from a binary hide to staged DCRP.)*
- Pictures were the one intervention that improved *character* durability — supports the image
  fast-follow, possibly sooner for abstract-meaning kanji.

### 6.4 Toolchain validated (build desktop-first; iOS is the risk)
- **Dioxus 0.7.9** renders via the **system WebView** (the native WGPU "Blitz" renderer is alpha,
  not ready) — which is a *plus*: HTML5 `<canvas>` (drawing), CSS SVG `stroke-dashoffset`
  animation (KanjiVG), and bundled webfonts all work. Design for the WebView.
- **FSRS:** use **`rs-fsrs`** (MIT, scheduler-only, no ML deps) in the app; add the **`fsrs`**
  crate (BSD-3, optimizer) later behind a flag. **Two independent `Card` states per kanji** are
  trivially supported — validates the two-track design.
- **SQLite:** `rusqlite` with `bundled` + `rusqlite_migration`; keep it the *sole* consumer of
  `libsqlite3-sys` (don't also add `sqlx`).
- **Audio:** `rodio` (or `kira`); on iOS link `-framework AudioToolbox`.
- **Drawing canvas** is an escape-hatch (`document::eval` + a self-contained JS module sending
  strokes back to Rust), not a first-class Dioxus primitive — budget integration time, test iOS
  touch events. **Bundle Noto Sans/Serif JP** (don't trust system CJK font resolution).
- **iOS is the highest-risk surface** (cpal linking, rusqlite cross-compile, canvas touch,
  signing). Do an **early iOS build spike** exercising Dioxus + rusqlite + cpal together before
  building iOS features. Confirms desktop-first sequencing.

## 7. Third research pass — dataset sourcing, selection algorithms, FSRS internals

Execution-correctness detail that de-risks M1/M2. Full build spec in [08-DATASET](08-DATASET.md).

- **JLPT membership:** use `davidluzgouveia/kanji-data` `jlpt_new` (MIT, Tanos-based; N5 79 · N4 166
  · N3 367 · N2 367 · N1 1232). **The KANJIDIC2 `jlpt` field is the obsolete pre-2010 4-level scale
  — never use it.** No official list exists; the N2/N3 split is least reliable.
- **Keyword trap:** KANJIDIC2 meanings are unordered **and ~1,900 were seeded from Heisig** — so a
  naive first-gloss doesn't escape Heisig. Use a cross-check chain (JMdict frequency-ordered
  glosses) + a manual override table for nulls and collisions.
- **Dominant reading must be derived** (no open dataset has it): word-frequency × **JmdictFurigana**
  alignment, normalizing rendaku/gemination, excluding nanori/jukujikun.
- **Vocab/sentences:** join word→kanji→reading via JmdictFurigana honoring `re_restr`; rank by
  `wordfreq`/`FrequencyWords`. Tatoeba via `jpn_indices.csv`, filter on the `~` checked-marker +
  native owner + translation. Avoid jpdb/BCCWJ/Anki-Core (licensing). Audio is per-row licensed.
- **FSRS correction:** `rs-fsrs` v1.2.1 ships **FSRS-4.5 (19 weights, fixed decay)**, not FSRS-5/6
  (that's the `fsrs` trainer crate). Ship FSRS-4.5 defaults; **turn fuzz ON** (crate default is
  off); personalize only after ~1,000 reviews.
- **Multi-mode → one rating:** test all due modes of a track in one sitting, submit **worst-of**
  as the single rating (one `repeat()` call). Maturity gate: **`stability ≥ 21d AND reps ≥ 2`**
  (a confident `Easy` first answer alone gives stability ≈ 15.5 — the `reps` guard prevents a
  too-easy unlock). **Defer a due production review ≥1 day** behind its comprehension review to
  avoid priming inflating the production rating (the main two-track correctness pitfall).

## 8. Fourth research pass — competition, engagement, voice-of-user

Full analysis in [09-COMPETITION-AND-ENGAGEMENT](09-COMPETITION-AND-ENGAGEMENT.md).

- **White space:** a **systematic phonetic-component reading system inside an SRS is essentially
  unclaimed** (exists only as a book + homemade Anki decks) — our biggest differentiator. No tool
  integrates **reading + writing + mnemonics**; **offline-first** is a structural edge rivals
  "can't easily match"; the market is split into engaging-but-shallow vs rigorous-but-joyless and
  **nobody owns "rigorous *and* rewarding."**
- **Hardest competitors:** WaniKani (mnemonic-SRS + brand — beat it on adaptivity/writing/offline/
  phonetics/price, not on its own axis), jpdb (corpus moat), MaruMori (same integrated ambition —
  stay ahead via offline + production + phonetics + speed).
- **Top quit causes (voice of user):** (1) **review-debt death spiral** — *the* dominant quit
  trigger; (2) leeches blocking progress; (3) can't skip known kanji; (4) no-undo / typo & synonym
  punishment; (5) "Anki tests but doesn't teach" / config paralysis; (6) kanji in isolation;
  (7) mid-level slump / useless vocab. ⇒ a whole **anti-burnout retention-UX system** (daily cap,
  backlog smoothing, vacation mode, decoupled queues, leech rescue, forgiving input + undo,
  placement/test-out) is now first-class — the highest-leverage retention work in the project.
- **Engagement science:** gamification helps learning only as a vehicle for feedback/goals
  (Sailer & Homner g≈.49); the over-justification effect means extrinsic rewards can *erode*
  intrinsic motivation. **Guardrail: gamify the return and the mastery, never the learning.**
  Rewards must be **informational** (competence signals) not **controlling**. Adopt: mastery/
  progress visualization, humane streak *with slack* (freeze + rest day; leniency *increases*
  persistence), milestone celebrations, FSRS-as-flow "challenge dial". Avoid: farmable XP, paid
  lives/streak-repair, guilt notifications, default-on leaderboards. Keep a **learning-quality
  north star** no engagement feature may degrade.

## Sources

Memory science & techniques:
- Dunlosky et al. 2013, *Improving Students' Learning…* — https://journals.sagepub.com/doi/abs/10.1177/1529100612453266
- Testing effect (Roediger & Karpicke 2006 overview) — https://en.wikipedia.org/wiki/Testing_effect
- Recall vs recognition transfer (Roediger lab) — https://www.sciencedirect.com/science/article/abs/pii/S0749596X19300026
- Encoding specificity / transfer-appropriate processing — https://en.wikipedia.org/wiki/Encoding_specificity_principle
- Bjork & Bjork, desirable difficulties — https://www.unh.edu/teaching-learning-resource-hub/sites/default/files/media/2023-06/itow-introducing-desirable-difficulties-into-practice-and-instruction-bjork-and-bjork.pdf
- Picture superiority effect — https://en.wikipedia.org/wiki/Picture_superiority_effect
- Distinctiveness vs dual coding (2025) — https://journals.sagepub.com/doi/10.1177/17470218241235520
- Self-reference effect meta-analysis — https://pubmed.ncbi.nlm.nih.gov/9136641/
- Karpicke & Roediger 2007 (expanding vs equal intervals) — https://learninglab.psych.purdue.edu/downloads/2007/2007_Karpicke_Roediger_JEPLMC.pdf
- Bizarreness effect (conditional/weak) — https://en.wikipedia.org/wiki/Bizarreness_effect
- Von Restorff / isolation — https://effectiviology.com/von-restorff-isolation-effect/
- Method of loci durability (Dresler 2017, via Stanford) — https://med.stanford.edu/news/all-news/2017/03/memorization-tool-bulks-up-brains-internal-connections.html
- Loci / transfer limits — https://pmc.ncbi.nlm.nih.gov/articles/PMC7862396/
- Keyword mnemonic + retrieval (~50% threshold) — https://pmc.ncbi.nlm.nih.gov/articles/PMC10839596/
- Keyword mnemonic for L2 vocab (mixed long-term) — https://link.springer.com/article/10.3758/s13421-019-00936-2
- Production effect (MacLeod & Bodner 2017) — https://journals.sagepub.com/doi/full/10.1177/0963721417691356
- FSRS optimal retention wiki — https://github.com/open-spaced-repetition/fsrs4anki/wiki/The-Optimal-Retention
- FSRS benchmark (prediction accuracy) — https://github.com/open-spaced-repetition/srs-benchmark
- Anki manual (desired retention guidance) — https://docs.ankiweb.net/deck-options.html

Kanji apps & method:
- Heisig RTK review/critique — https://migaku.com/blog/japanese/heisig-remembering-the-kanji-review
- WaniKani SRS / unlocking / level-up — https://knowledge.wanikani.com/wanikani/srs-stages/ , https://knowledge.wanikani.com/getting-started/unlocking-kanji/
- Kanji Koohii (voted community stories) — https://kanji.koohii.com/learnmore
- KanjiDamage (reading/vowel-length encoding, compositional jukugo) — https://www.kanjidamage.com/howto
- Tofugu radicals mnemonic method — https://www.tofugu.com/japanese/kanji-radicals-mnemonic-method/
- Ringotan (trace→fade handwriting) — https://www.ringotan.com/
- Skritter (handwriting SRS) — https://skritter.com/features

Phonetic components:
- Kanji Portraits — 61% of Jōyō are keisei moji — https://kanjiportraits.wordpress.com/2021/10/24/composite-formation-of-kanji-%E4%BC%9A%E6%84%8F%E6%96%87%E5%AD%97-and-%E5%BD%A2%E5%A3%B0%E6%96%87%E5%AD%97/
- The Kanji Code — keisei moji — https://thekanjicode.com/the-magic-of-the-keisei-moji-%E5%BD%A2%E5%A3%B0%E6%96%87%E5%AD%97/
- Keisei dataset (GPL-3.0; reference) — https://github.com/mwil/wanikani-userscripts
- Keisei 天上中下 indicator — https://github.com/mwil/wanikani-userscripts/blob/master/wanikani-phonetic-compounds/docs/indicator.md
- Morg — perfect series — https://morg.systems/Kanji-with-a-semantic-and-phonetic-component
- Kanjium (CC-BY-SA 4.0) — https://github.com/mifunetoshiro/kanjium

Learning order & multisensory:
- topokanji (frequency-weighted topo sort) — https://github.com/scriptin/topokanji/blob/master/README.md
- Yu et al. 2016 (topological sort for character order) — https://arxiv.org/abs/1602.08742
- Handwriting vs typing neuroscience — https://pmc.ncbi.nlm.nih.gov/articles/PMC11943480/
- Handwriting & visual word recognition (Chinese) — https://pmc.ncbi.nlm.nih.gov/articles/PMC8194694/
- "Save Your Strokes" RCT — https://journals.sagepub.com/doi/full/10.1177/2332858419890326
- Stroke-animation interference with recognition — https://pmc.ncbi.nlm.nih.gov/articles/PMC9403612/
- Shadowing systematic review (2025) — https://www.tandfonline.com/doi/full/10.1080/29984475.2025.2546827
- Dual coding vs cognitive load (L2 vocab) — https://www.frontiersin.org/journals/psychology/articles/10.3389/fpsyg.2022.834706/full

Second pass — content generation, actors, durability, toolchain:
- SMART mnemonic generation (EMNLP 2024) — https://arxiv.org/html/2406.15352v2 , https://github.com/nbalepur/Mnemonic
- Interpretable Mnemonic Generation for Kanji via EM (EMNLP 2025) — https://arxiv.org/html/2507.05137
- PhoniTale (phonological reading mnemonics, 2025) — https://arxiv.org/html/2507.05444
- WaniKani GPT-4 mnemonic generator (practitioner) — https://community.wanikani.com/t/generating-wanikani-mnemonics-with-gpt-4/60934
- Tofugu radicals + consistent-actor method — https://www.tofugu.com/japanese/kanji-radicals-mnemonic-method/
- KanjiDamage vowel-length / on'yomi keyword scheme — https://www.kanjidamage.com/howto , http://www.kanjidamage.com/appendix/longshortvowels
- On'yomi phonotactics — https://en.wikipedia.org/wiki/On%27yomi
- RTK primitive elements (count/naming) — http://rtkelements.blogspot.com/2014/11/index-of-all-rtk-primitive-elements.html
- Self-generated vs other-generated cues — https://link.springer.com/article/10.3758/s13421-021-01245-3
- Generation-constraint meta-analysis — https://link.springer.com/article/10.3758/s13423-020-01762-3
- Keyword mnemonic + retrieval practice (imposed vs induced null) — https://pmc.ncbi.nlm.nih.gov/articles/PMC10839596/
- Thomas & Wang (mnemonics forget faster) — https://memory-key.com/research/Wang95
- Wang (Chinese characters, imagery mnemonics at delay) — https://onlinelibrary.wiley.com/doi/abs/10.1111/j.1467-1770.1992.tb01340.x
- Diminishing-cues retrieval practice (Finley & Benjamin) — https://pmc.ncbi.nlm.nih.gov/articles/PMC3076684/
- DCRP works when regular testing doesn't — https://link.springer.com/article/10.3758/s13423-017-1366-9
- Kanji Alive open data (CC BY 4.0) — https://github.com/kanjialive/kanji-data-media
- KRADFILE-U (CC BY-SA) — https://github.com/jmettraux/kensaku
- kanjivg-radical (CC BY-SA) — https://github.com/yagays/kanjivg-radical
- Dioxus 0.7 release — https://dioxuslabs.com/blog/release-070/
- rs-fsrs (MIT scheduler) — https://github.com/open-spaced-repetition/rs-fsrs
- rusqlite issue on bundled/migrations — https://github.com/rusqlite/rusqlite/issues/1615
- rodio — https://github.com/RustAudio/rodio
- SVG stroke-dashoffset animation — https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-dashoffset

Third pass — datasets, selection, FSRS internals:
- davidluzgouveia/kanji-data (JLPT jlpt_new, MIT) — https://github.com/davidluzgouveia/kanji-data
- Tanos JLPT lists — http://www.tanos.co.uk/jlpt/
- KANJIDIC2 project / obsolete jlpt field — https://www.edrdg.org/wiki/index.php/KANJIDIC_Project
- scriptin/kanji-frequency (CC BY 4.0) — https://github.com/scriptin/kanji-frequency
- KanjiVG / CHISE IDS (GPLv2, build-time only) — https://github.com/KanjiVG/kanjivg , https://github.com/chise/ids
- JmdictFurigana (per-kanji alignment) — https://github.com/Doublevil/JmdictFurigana
- wordfreq (Apache/CC BY-SA) — https://github.com/rspeer/wordfreq
- hermitdave/FrequencyWords — https://github.com/hermitdave/FrequencyWords
- stephenmk/yomitan-jlpt-vocab — https://github.com/stephenmk/yomitan-jlpt-vocab
- Tatoeba exports / jpn_indices / tags — https://downloads.tatoeba.org/exports/ , https://en.wiki.tatoeba.org/articles/show/tags
- rs-fsrs (FSRS-4.5 source) — https://github.com/open-spaced-repetition/rs-fsrs
- FSRS algorithm explanation (Expertium) — https://expertium.github.io/Algorithm.html
- Anki stats (young/mature 21-day) — https://docs.ankiweb.net/stats.html
- FSRS optimization review-count guidance — https://forums.ankiweb.net/t/how-many-reviews-for-accurate-optimization/53320

Fourth pass — competition, engagement, voice-of-user:
- Phonetic-component reading (white space) — https://thekanjicode.com/ , https://www.edrdg.org/~jwb/kanjiphonetics/
- WaniKani knowledge / pros-cons — https://knowledge.wanikani.com/wanikani/srs-stages/ , https://cotoacademy.com/wanikani-review-learning-japanese-kanjithe-pros-and-cons/
- Anki vs WaniKani (teaches vs tests) — https://migaku.com/blog/japanese/anki-vs-wanikani
- jpdb / Renshuu / MaruMori / Kanji Garden — https://jpdb.io/faq , https://www.renshuu.org/ , https://marumori.io/ , https://www.tofugu.com/japanese-learning-resources-database/kanji-garden/
- Review-debt / pile-up (top quit cause) — https://community.wanikani.com/t/overwhelmed-i-stopped-for-a-week-or-two-and-dont-know-how-to-catch-up/54436
- Leeches — https://community.wanikani.com/t/leeches-and-my-brain-just-exploded-with-rage/56194
- No-undo / synonym punishment — https://community.wanikani.com/t/wanikani-is-literally-unusable-without-an-undo-button/61496
- Test-out request — https://community.wanikani.com/t/ignoring-simple-kanji-that-you-already-know-by-testing-out-in-the-beginning-for-the-first-levels/17218
- Moderate vacation mode — https://community.wanikani.com/t/moderate-vacation-mode/43417
- SDT × gamification meta-analysis — https://link.springer.com/article/10.1007/s11423-023-10337-7
- Over-justification effect — https://en.wikipedia.org/wiki/Overjustification_effect
- Gamification-of-learning meta-analysis (Sailer & Homner) — https://link.springer.com/article/10.1007/s10648-019-09498-w
- Duolingo streak/habit (with data) — https://blog.duolingo.com/how-duolingo-streak-builds-habit/
- Streak creep / dark patterns — https://thedecisionlab.com/insights/consumer-insights/streak-creep
- Flow & learning experience design — https://edtechbooks.org/ux/flow_theory_and_lxd
