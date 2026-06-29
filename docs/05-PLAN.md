# 05 — Execution Plan

Milestones are ordered and incremental. Each ends with a **demoable** result and explicit
acceptance criteria. v1 = desktop + full N5 content (see [01-VISION §Success](01-VISION.md)).

> **Authorship note:** commits and releases are authored as **Sizyph** only — no co-author
> attribution added.

---

## Progress (as of 2026-06-29)

- ✅ **M0** scaffold — Cargo workspace (core/data/content/ui), Dioxus app, CI, public repo.
- ✅ **M1** N5 dataset — kanji core + components + frequency-weighted topological order; authored
  keywords, component/reading actors, judge-verified mnemonics; dominant readings (3-judge panel
  verified); N5-appropriate vocab; example sentences; KanjiVG stroke data. (Phonetic table deferred
  — near-zero payoff at N5; lands with N4/N3.)
- ✅ **M2** core engine — two-track FSRS scheduler + session engine + multi-day simulation; data
  layer (read-only seed content repo + writable user state store, migrations, persistence).
- ✅ **M3** review UI (Dioxus) — dashboard; all four modes (recognition, reading-in-context, write
  w/ stroke animation, cloze); kanji-detail hub w/ editable mnemonics; browse grid; settings;
  one-tap undo; dev clock-skip. **Audio deferred** (needs a cross-platform decision).
- ✅ **M7** packaged releases — `bundle-seed` self-contained binaries (N5 dataset embedded);
  `release.yml` builds the seed once then a mac/win/linux matrix and uploads to the GitHub Release.
  **v0.1.0 published** with mac-arm64 / windows-x64 / linux-x64 downloads. (Unsigned; signing needs
  Apple/Microsoft developer accounts.)
- ⬜ **Next** — engagement layer (humane streak, progress/stats — M6); level briefing/congrats UI +
  N4–N1 content (M8/M4); export/import (M5); audio; iPhone (M9, needs Apple account/signing); cloud
  sync (M10); code-signed installers (.dmg/.msi/.AppImage).

---

## M0 — Project scaffold & decisions locked  *(foundations)*

- Initialize the Cargo workspace (`core`, `data`, `content`, `ui`) with Dioxus desktop hello-world.
- `git init`, add `.gitignore` (must exclude the Heisig PDF and local DBs), choose code license.
- CI: `cargo build`/`test`/`clippy`/`fmt` on the desktop matrix (macOS/Win/Ubuntu).
- **Acceptance:** empty Dioxus window builds & runs on all three desktop OSes via CI.

## M1 — Content pipeline + N5 dataset  *(the data)*

- Build `crates/content` per the assembly spec in [08-DATASET](08-DATASET.md): parse KANJIDIC2,
  `davidluzgouveia/kanji-data` (`jlpt_new`), KRADFILE, KanjiVG, JMdict, **JmdictFurigana**,
  word-frequency (`wordfreq`/`FrequencyWords`), Tatoeba (`jpn_indices`), Kanjium.
- Run the keyword / dominant-reading / vocab / sentence **selection algorithms** ([08 §2](08-DATASET.md));
  build the keyword manual-override table for the null/collision residue.
- Build the **phonetic table** (Kanjium + KANJIDIC; 天/上/中/下 tiers; validated vs GPL Keisei,
  not bundled). Tag component edges semantic vs phonetic.
- Seed the **component-actor** and **reading-actor** registries (reading actors keyed by reading
  + vowel length).
- Compute the **frequency-weighted topological** learning order (`intro_rank`); pin early-wins.
- Emit `assets/seed.sqlite` + attribution manifest.
- Populate **N5** fully: components, ≥1 reading/meaning each, dominant reading flagged, stroke
  data, phonetic info where applicable, ≥1 sentence each.
- **Acceptance:** querying the seed DB returns a complete, prerequisite-ordered N5 with all
  facets, phonetic tiers, and actor links present; attribution manifest generated.

## M2 — Domain core + FSRS + persistence  *(the engine, headless)*

- `crates/core`: domain model, session engine (mode selection by track maturity, interleaving,
  new-kanji budget, ordering, level gating), wired to **`rs-fsrs`** (MIT, scheduler-only).
- **Two-track scheduling** (FSRS-4.5 via `rs-fsrs`, `enable_fuzz=true`, retention 0.90): two
  `Track` rows per kanji; **worst-of** multi-mode rating (one `repeat()`/track); production gated
  on comprehension maturity and **deferred ≥1 day** vs a same-day comprehension review
  (anti-priming); level-clear gate `stability ≥ 21d AND reps ≥ 2`.
- `crates/data`: SQLite repositories + migrations; "two Tracks per kanji" invariant; `ReviewLog`.
- Thorough unit tests on two-track scheduling, production gating, comprehension-only level
  gating, and the ordering algorithm.
- **Acceptance:** a headless test drives a simulated multi-day study run; both tracks schedule
  correctly, production activates only after comprehension matures, and level-unlock fires on the
  comprehension threshold — no UI required.

## M3 — Review UI (the core loop)  *(first usable app)*

- Dashboard, Session/Review across all 4 modes, reveal + grade + linked-neighbor cues.
- **Encoding rules:** recognition/reading prompts = clean glyph + audio only; animation +
  decomposition only in production mode / on reveal (modality segregation). Production requires
  an attempt before reveal (self-grade + KanjiVG trace canvas via the JS escape-hatch).
  **Graduated, reversible scaffold fade (DCRP)** — full→partial→prompt-only→faded, gated on
  latency + consecutive cold success; log `elapsed_ms` per review ([07 §5](07-CONTENT-GENERATION.md)).
- **Audio** on every card (resolve TTS vs recordings — [04 open item](04-DATA-SOURCES.md)).
- Kanji detail (hub): stroke animation, semantic + phonetic components, phonetic series + tier
  badge, readings/meanings, sentences, **editable mnemonic** with actors + make-it-stick checklist.
- **Anti-burnout core (build into the loop, not bolted on):** user-set **daily review cap** +
  backlog smoothing; **decoupled** new-lesson vs due-review queues; **forgiving input** (typo
  tolerance, synonyms, retroactive credit) + **one-tap undo** ([09 §4](09-COMPETITION-AND-ENGAGEMENT.md)).
- **Acceptance:** a full N5 daily session is completable end-to-end on desktop; audio plays;
  modality segregation holds; daily cap + undo work; mnemonic edits persist.

## M4 — Levels, gating, briefing & congrats  *(progression)*

- Level briefing (count/difficulty/time/new components + phonetic series); **~90%
  comprehension** mastery gating; components-mature-before-kanji; **test-out** for known kanji;
  congratulations / level-up screen; lower levels stay reviewable.
- Add **N4** content via the M1 pipeline to prove multi-level flow.
- **Acceptance:** clearing ~90% of N5's comprehension tracks unlocks N4 with briefing + congrats;
  the production track does not block; new introductions respect the unlocked level.

## M5 — Storage: backup/export + import  *(data safety)*

- File export/import of user state (progress + edited mnemonics); settings screen.
- **Acceptance:** export on one machine, import on another, study continues from that state.

## M6 — Retention UX, engagement & polish  *(quality)*

- **Anti-burnout:** vacation/illness mode; **active leech rescue** (auto-detect → un-gate →
  re-teach with a different actor/angle or confusable grouping); onboarding **placement / test-out**.
- **Engagement (informational, guard-railed):** mastery/progress views + retention forecast +
  heatmap; **humane streak** (tied to completing due reviews; auto streak-freeze; weekly rest day;
  milestone celebrations); FSRS **challenge dial**; personal records. No farmable XP / paid lives /
  guilt notifications / default-on leaderboards ([09 §5](09-COMPETITION-AND-ENGAGEMENT.md)).
- Stats screen, settings, About/attribution screen, theming, performance pass.
- **Acceptance:** v1 success criteria in [01-VISION](01-VISION.md) all met on desktop; leech rescue
  and streak/progress work; the learning-quality north star ([09 §8](09-COMPETITION-AND-ENGAGEMENT.md))
  is instrumented and no engagement feature degrades it.

## M7 — Public desktop release  *(ship v1)*

- GitHub Actions release matrix → `.dmg` / `.msi` / `.AppImage`+`.deb`; tagged public release
  on `github.com/sizyph/MnemoKanji`; README install instructions.
- **Acceptance:** a fresh user can download and run MnemoKanji on each desktop OS.

## M8 — Remaining levels content  *(scale the dataset)*

- Generate + curate **N3, N2, N1** content through the pipeline; QA the generated mnemonics.
- **Acceptance:** all five levels fully populated and gated end-to-end.

## M9 — iPhone build & release  *(mobile)*

- **Prerequisite — iOS build spike (do early, any time after M2):** a throwaway build to
  `aarch64-apple-ios` exercising Dioxus + `rusqlite` (bundled) + `rodio`/`cpal` **together**
  (link `-framework AudioToolbox`; verify SDK sysroot for the `cc` build), plus canvas touch
  events in the iOS WebView. iOS is the highest-risk surface — validate the toolchain before
  building iOS features ([06 §6.4](06-RESEARCH.md)).
- Dioxus mobile target; iOS packaging, signing/provisioning; touch-tuned UI (esp. production
  canvas); TestFlight → release.
- **Acceptance:** MnemoKanji runs on iPhone with the full study loop; published.

## M10 — Cloud sync (optional)  *(multi-device auto-sync)*

- Its own design note first (account model, conflict resolution, infra/cost). Then implement
  optional account sync of user state.
- **Acceptance:** progress auto-syncs across two of the owner's devices.

---

## Cross-cutting / later

- **Concrete per-kanji/component images** (picture superiority) — **fast-follow after M3**; v1
  ships text + audio. (Owner decision.)
- **Community mnemonic voting** (Koohii-style top-3 + fresh) over the AI stories — later layer.
- **FSRS per-user optimization** — once `ReviewLog` has ~1,000 reviews of real use.
- **Shadowing / mic capture** for the production effect — beyond the v1 "say it aloud" prompt.
- **Handwriting recognition grading** — revisit beyond self-grade/stroke-validation if warranted.

## Immediate next step (pending owner go-ahead)

Start **M0**: scaffold the workspace, `git init`, `.gitignore` (excluding the Heisig PDF),
pick the code license, and stand up the Dioxus desktop hello-world with CI. No app logic yet.

## Decisions locked (no longer open)

1. Code license: **MIT**.
2. GitHub repo: created **after M0 builds locally** (then public on `github.com/sizyph`).
3. Scheduling: **two tracks** (Comprehension + Production); **comprehension gates** progression,
   production runs in parallel and never blocks.
4. v1 mnemonics: **text stories + audio** (concrete images are a fast-follow).
5. Starting values: **~10 new kanji/day**, desired retention **0.90**, level cleared at **~90%
   of kanji with comprehension stability ≥ 21 days** — all adjustable in Settings.

## Remaining open items (resolved during the relevant milestone)

- ~~JLPT mapping~~ → `davidluzgouveia/kanji-data` `jlpt_new`; ~~sentence filters~~ → [08-DATASET](08-DATASET.md). (resolved)
- Generation LLM + gold-example set, and per-registry persona style guide ([07 §6](07-CONTENT-GENERATION.md)) — M1.
- Audio approach: platform TTS vs bundled recordings/model — M3.
- Whether to move FSRS-4.5 (`rs-fsrs`) → FSRS-6 (`fsrs` trainer) for learnable decay + on-device
  optimization — revisit once a user has ~1,000 reviews (post-M3).
