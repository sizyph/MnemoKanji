# 03 — Design & Architecture (v2, evidence-revised)

How the method in [02-LEARNING-METHOD](02-LEARNING-METHOD.md) becomes software. v2 incorporates
the research in [06-RESEARCH](06-RESEARCH.md) and the owner's decisions (two-track scheduling,
comprehension-gated progression, text+audio first).

## 1. Tech stack

| Concern | Choice | Why |
|---------|--------|-----|
| Language | **Rust** | Owner requirement; one fast, safe codebase for all targets. |
| UI | **Dioxus 0.7** (RSX), **system WebView** renderer | One codebase desktop + mobile; WebView gives HTML5 canvas, CSS SVG animation, webfonts (Blitz/WGPU renderer is alpha — not used). |
| Scheduler | **`rs-fsrs`** v1.2.1 (MIT, **FSRS-4.5**, 19 weights, `serde` on); `fsrs` trainer (FSRS-6 + optimizer) later behind a flag | Pure Rust; **stateless ⇒ two `Card` states per kanji trivial**; whole `Card` struct is the persisted state. |
| Local storage | **`rusqlite`** (`bundled`) + `rusqlite_migration` | Embedded SQLite, offline, same on desktop + iOS; sole consumer of `libsqlite3-sys` (no `sqlx`). |
| Stroke rendering | **KanjiVG** SVG + CSS `stroke-dashoffset` animation | Animated stroke order + component highlighting (production/reveal only). |
| Drawing canvas | HTML5 `<canvas>` via `document::eval` escape-hatch | Stroke capture for writing mode; self-contained JS module → strokes back to Rust. |
| Audio | **`rodio`** (or `kira`); platform TTS / CC recordings for content | Readings audio on every card; on iOS link `-framework AudioToolbox`. |
| Fonts | Bundled **Noto Sans/Serif JP** | Correct CJK glyphs on all platforms (don't trust system font resolution). |
| Build/release | **GitHub Actions** matrix + `dx`/`cargo` bundlers | Per-OS artifacts from one workflow. |

### Workspace layout (Cargo workspace)

```
mnemokanji/
├─ Cargo.toml                # workspace
├─ crates/
│  ├─ core/                  # domain model, FSRS wiring, session engine, ordering — no UI
│  ├─ data/                  # storage layer (SQLite), repositories, migrations
│  ├─ content/               # offline dataset builder: upstream sources → seed DB
│  └─ ui/                    # Dioxus app (desktop + mobile)
├─ assets/                   # seed DB, KanjiVG SVGs, fonts, attribution manifest
├─ docs/                     # this documentation
└─ .github/workflows/        # CI + release matrix
```

Rationale: **core** is UI-agnostic and unit-testable; **content** is an offline pipeline so
regenerating data never touches app code.

## 2. Domain model

Entities (conceptual; SQLite schema in `crates/data/migrations`):

- **Level** — `{ id, jlpt (N5..N1), order, kanji_count, est_minutes, difficulty_label, blurb }`.
- **Component** (primitive/radical) — `{ id, glyph, keyword, kind (radical|primitive), actor_name, actor_desc, actor_image_ref? }`.  *(actor = persistent persona, §G of method)*
- **Kanji** — `{ id, glyph, level_id, stroke_count, primary_keyword, frequency, intro_rank }`.
- **KanjiComponent** (edge) — `{ kanji_id, component_id, role (semantic|phonetic), position }`.
- **PhoneticComponent** — `{ component_id, on_readings[] }`; per-kanji prediction stored on
  Kanji as `{ phonetic_component_id?, phonetic_tier (天|上|中|下|none) }`.  *(v2)*
- **ReadingActor** — `{ reading_kana, vowel_length, actor_name, actor_desc }`.  *(v2: consistent persona per reading sound)*
- **Reading** — `{ id, kanji_id, kind (on|kun), reading, is_dominant, is_common }`.
- **Meaning** — `{ id, kanji_id, gloss, lang, is_primary, sense_order }`.
- **Vocab** — `{ id, surface, reading, gloss, jlpt_hint }` + **VocabKanji** edge (which reading each kanji contributes).
- **Sentence** — `{ id, jp, translation, source, license }` + **SentenceVocab** edge.
- **Mnemonic** — `{ id, kanji_id, story, stroke_hint, origin (generated|user), edited(bool), actors_used[], judge_score, verified(bool), edited_at }`.  *(quality/provenance fields per [07](07-CONTENT-GENERATION.md))*
- **Track** (the scheduled unit) — `{ id, kanji_id, kind (comprehension|production), fsrs_state(blob), due, stability, difficulty, reps, lapses, introduced_at, scaffold_stage (0=full…3=faded) }`.  *(DCRP fade stage, [07 §5](07-CONTENT-GENERATION.md))*
- **ReviewLog** — `{ id, track_id, ts, mode, rating, elapsed_ms, recalled_without_story(bool) }`.
- **Track** also carries `is_leech(bool)` (derived: high lapses / low accuracy ⇒ rescue, §11).
- **Settings** — `{ daily_review_cap, new_kanji_per_day, desired_retention, challenge_level (relaxed|balanced|intense), vacation_mode(bool), reminder_time?, audio_on }`.
- **Streak** — `{ current, longest, last_active_date, freezes_available, rest_day_weekday }`.

### Invariants

- **Each Kanji has exactly two Tracks**: one `comprehension`, one `production`. The kanji is the
  single concept/hub; the two tracks are its independent schedules (method §C).
- The **production** track is *activated* only when the kanji's **comprehension** track reaches
  the maturity threshold (gated; asymmetric transfer).
- A Kanji's components must be known before the kanji is introduced (ordering, §6).

### Entity relationships (text ERD)

```
Level 1──* Kanji
Kanji 1──2 Track (comprehension, production) 1──* ReviewLog
Kanji *──* Component        (KanjiComponent, role = semantic | phonetic)
Component 0..1── PhoneticComponent ; Kanji →(phonetic_component_id, tier)
Reading *──1 ReadingActor   (by reading_kana + vowel_length)
Kanji 1──* Reading ; 1──* Meaning ; 1──1 Mnemonic
Kanji *──* Vocab (VocabKanji) ; Vocab *──* Sentence (SentenceVocab)
```

## 3. Scheduling: two FSRS tracks per kanji

- Each `Track` holds its own FSRS state and `due` date.
- A **review event** selects modes appropriate to that track's maturity, presents them, collects
  **one rating** for the track, and advances its FSRS state once. `ReviewLog` records per-mode
  detail (for analytics + future FSRS optimization).
- **Comprehension maturity → modes:** new → recognition; maturing → + reading-in-context.
- **Production activation:** when comprehension `stability ≥` threshold (default ~7–14 days),
  create/activate the production track → write-from-keyword; later → + cloze.
- **Modes are interleaved** within a session across both tracks.
- **One rating per track from multiple modes (worst-of):** a comprehension review tests
  kanji→meaning *and* kanji→reading in one sitting; the engine self-grades each and submits
  **`min(rating_meaning, rating_reading)`** as the single FSRS rating (one `repeat()` call). Same
  for production (write + cloze). A kanji you can read but not interpret isn't retained, so the
  weakest facet must drive the schedule. `ReviewLog` keeps the per-mode detail.
- **Production activation & anti-priming:** the production `Card` stays `New` (unscheduled) until
  the comprehension card is mature; on unlock, simply begin reviewing it. If both tracks of a
  kanji are due the same day, **do comprehension first and defer production ≥1 day** — seeing the
  kanji for comprehension would otherwise prime the write and inflate the production rating.
- **Config to ship:** `request_retention = 0.90`, `maximum_interval = 36500`,
  **`enable_fuzz = true`** (override the crate default `false`; deterministic per-card seed to
  de-cluster bulk-added kanji), `enable_short_term = true`, default 19 weights unmodified. Offer
  per-user optimization only after **~1,000 reviews** on a track (needs the `fsrs` trainer crate).
- **New-track budget** per session is configurable (default ~10 new kanji/day), drawn only from
  the unlocked level, respecting component prerequisites.

## 4. Level progression & gating

- Levels ordered **N5 → N4 → N3 → N2 → N1**; only the lowest unlocked, unfinished level supplies
  new introductions.
- **Mastery threshold (initial, tunable):** a level is cleared when **~90% of its kanji have a
  mature *comprehension* track** (`stability ≥ 21 days` **AND `reps ≥ 2`** — the `reps ≥ 2` guard
  stops a single confident `Easy` first answer, whose init stability ≈ 15.5, from tripping the
  gate). The production track does **not** gate progression (owner decision). 90% (not 100%)
  prevents one stubborn kanji from blocking everything.
- **Components must be mature before their kanji unlock.**
- **Test-out:** allow marking known kanji as already-learned to skip needless relearning
  (WaniKani's main complaint).
- **Briefing screen** on entering a level: kanji count, difficulty, estimated time, new
  components/phonetic series introduced.
- **Congratulations / level-up screen** on clearing: kanji mastered, days taken, retention %,
  explicit "Unlock N4".
- Cleared levels stay fully reviewable; only *new introductions* advance by level.

## 5. UI screens (Dioxus)

1. **Home / Dashboard** — due counts (per track), new-today budget, current level + progress,
   streak, "Start session".
2. **Level briefing** — §4.
3. **Session / Review** — prompt → (forced attempt for production) → reveal → grade
   (Again/Hard/Good/Easy) → linked-neighbor cues → next.
   - **Recognition/reading prompts: clean glyph + audio only** (no animation/highlighting).
   - **Production: draw canvas + KanjiVG reference on reveal**, animation + decomposition shown
     here (modality segregation, method §G).
4. **Kanji detail (hub)** — glyph, animated stroke order, components (semantic + phonetic,
   clickable), phonetic series + 天/上/中/下 badge, readings/meanings, vocab + sentences,
   **editable mnemonic** with actors and "make-it-stick" checklist, link panel.
5. **Browse / Map** — list/grid by level with per-track state; component & phonetic-series graph
   views.
6. **Stats** — retention, reviews over time, due-load forecast (per track).
7. **Settings** — new-kanji/day, mastery threshold, desired retention, mode policy, audio,
   export/import, (later) sync.
8. **Congratulations / level-up.**
9. **Onboarding / placement** — 60-second zero-config start; optional placement test + "I already
   know this" test-out so prior knowledge isn't re-ground ([09 §4](09-COMPETITION-AND-ENGAGEMENT.md)).
10. **Progress & mastery** — learning/young/mature/mastered counts, N5→N1 bars, retention
    forecast, review heatmap, personal records (the informational reward layer, [09 §5](09-COMPETITION-AND-ENGAGEMENT.md)).

## 6. Learning order (algorithm)

- Build the component DAG (edge: kanji *contains* component). Within each JLPT level run a
  **frequency-weighted topological sort** (Kahn's algorithm + frequency tie-break): among kanji
  whose components are all known, pick the highest-frequency next.
- Emit **just-in-time component cards** when a needed component isn't itself an in-level kanji.
- **Pin** a hand-curated early-wins list to the front of N5.
- Surface **high-yield phonetic series** early so predictable on'yomi come "for free".
- Store the resulting `intro_rank` per kanji in the seed DB.

## 7. Storage, backup & sync (phased)

- **Phase 1 — Local-first (v1).** Single SQLite DB in the OS app-data dir; fully offline.
- **Phase 2 — File export/import.** Backup/restore *user state* (tracks, logs, edited mnemonics,
  settings) to a portable file; drop it in iCloud/Dropbox for poor-man's sync. No server.
- **Phase 3 — Cloud account sync.** Optional account + sync backend; conflict resolution
  (last-write-wins per track, or CRDT-lite on review logs). Later milestone, own design note.

Separation: **bundled content** (kanji, readings, SVGs, phonetic table, actors, seed mnemonics)
is read-only and ships with the app; only **user state** is mutable and is what export/sync moves.

## 8. Content pipeline (`crates/content`)

Offline build tool producing the seed DB + assets. **Source-of-truth per field, the
keyword/reading/vocab/sentence selection algorithms, merge strategy, and conflict handling are
specified in [08-DATASET](08-DATASET.md).** Outline:

1. **KANJIDIC2** + `davidluzgouveia/kanji-data` (`jlpt_new`) → kanji, strokes, readings, meanings,
   JLPT level (never the obsolete KANJIDIC `jlpt` field).
2. **KRADFILE/RADKFILE** + **KanjiVG** → component decomposition + stroke data.
3. **Phonetic table** → built from **CC-BY-SA Kanjium + KANJIDIC**, with 天/上/中/下 tiers
   computed by reading-intersection; **validated against the GPL-3.0 Keisei dataset (reference
   only — not bundled)** to keep the shipped data MIT/CC-BY-SA clean.
4. **JMdict** + **JmdictFurigana** + word-frequency (`wordfreq`/`FrequencyWords`) → dominant
   reading (derived) and vocabulary per reading ([08 §2.2–2.3](08-DATASET.md)).
5. **Tatoeba** (`jpn_indices.csv`) → example sentences, filtered by tilde/native-owner/translation/
   length/i+1 ([08 §2.4](08-DATASET.md)).
6. **Actors** → seed the component-actor and reading-actor registries (reading actors keyed by
   reading + vowel length).
7. **Order** → frequency-weighted topological sort → `intro_rank`.
8. **Starter mnemonics** → generated via the constrained **generation → deterministic-verify →
   LLM-judge** pipeline in [07-CONTENT-GENERATION](07-CONTENT-GENERATION.md) (actors as fixed
   input, JSON output validated for component coverage / meaning placement); stored
   `origin = generated`, `verified`, editable.
9. Emit `assets/seed.sqlite` + attribution manifest.

## 9. Build & release

- **GitHub Actions** matrix: macOS (.app/.dmg), Windows (.exe/.msi), Ubuntu (.AppImage/.deb),
  later iOS (.ipa) via Dioxus mobile + Xcode signing.
- Tagged `vX.Y.Z` → public GitHub Release on `github.com/sizyph/MnemoKanji`.
- Code license **MIT**; bundled data keeps upstream licenses (see [04](04-DATA-SOURCES.md)).
- Commits/releases authored as **Sizyph** only (no co-author).

## 10. Key technical risks & mitigations

| Risk | Mitigation |
|------|-----------|
| Dioxus mobile / iOS packaging friction | Desktop-first; UI isolated from core so an alternate renderer is possible. |
| Generated mnemonic quality varies | Mark generated; editable; regenerate; curate N5 by hand; actors + distinctiveness rules in prompt. |
| Phonetic data licensing (GPL Keisei) | Build our own table on CC-BY-SA Kanjium/KANJIDIC; use Keisei as validation reference only. |
| Cross-platform audio quality (Linux TTS weak) | Platform TTS where good; bundle openly-licensed recordings where available; treat as open item. |
| Two-track scheduling bookkeeping | Clean `Track` table (2 rows/kanji); production gated on comprehension; well unit-tested. |
| Mastery threshold mis-tuned | Threshold is a setting; default conservative; revisit with usage. |
| Modern N5–N1 mapping is unofficial | Pick & record a well-maintained CC/MIT mapping (M1 open item). |

## 11. Retention UX & engagement (anti-burnout)

Design + rationale in [09-COMPETITION-AND-ENGAGEMENT](09-COMPETITION-AND-ENGAGEMENT.md); the
mechanics here. These directly target the top user-quit causes.

- **Daily review cap + backlog smoothing.** The session engine never surfaces more than
  `Settings.daily_review_cap` due items; overflow rolls forward and, after a gap, the accumulated
  debt is spread over several days (over-due items re-prioritized, not penalized).
- **Decoupled queues.** New introductions and due reviews are independent; pausing new lessons
  never stops reviews and a review backlog never blocks learning something new.
- **Vacation mode** (`Settings.vacation_mode`): freezes scheduling + streak penalties without
  mutating FSRS memory state.
- **Leech rescue.** A track is flagged `is_leech` from lapse/accuracy thresholds; the engine then
  (a) removes it from level-gating, and (b) triggers **re-teaching** — regenerate the mnemonic
  with a different actor/angle ([07](07-CONTENT-GENERATION.md)), optionally add an image, or group
  it with its confusables — rather than re-testing the same failing card.
- **Forgiving input.** Typo tolerance (edit-distance), synonym sets, **retroactive credit** on
  adding a synonym, and a **one-tap undo** that restores the prior FSRS state from `ReviewLog`.
- **Placement / test-out** at onboarding marks known kanji as already-mature so prior knowledge is
  skipped.
- **Engagement layer (informational, not controlling):** mastery/progress views, a humane streak
  (tied to *completing due reviews*, with auto streak-freeze + a weekly rest day), milestone
  celebrations, and the FSRS desired-retention exposed as a **challenge dial** (relaxed/balanced/
  intense). **Guardrail:** no farmable XP, paid lives, paid streak repair, guilt notifications, or
  default-on leaderboards; a learning-quality north star ([09 §8](09-COMPETITION-AND-ENGAGEMENT.md))
  governs every engagement feature.
