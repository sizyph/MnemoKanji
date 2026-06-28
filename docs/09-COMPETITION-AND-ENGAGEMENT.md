# 09 — Competition, Positioning & Engagement

Where MnemoKanji sits in the market, the white space it can own, and how it wins on the owner's
three axes — **learn better, in the shortest time, with more fun and more reward** — without the
dark patterns that make learning apps hollow. Grounded in the fourth research pass
([06-RESEARCH §8](06-RESEARCH.md)): a competitive landscape, the engagement/motivation science,
and the voice of real users.

## 1. Positioning (one line)

> **MnemoKanji = WaniKani's teaching & structure, minus the rigidity and review-debt punishment;
> Anki's flexibility, minus the setup pain — kanji learned in real context, with a phonetic
> reading system nobody else has, forgiving catch-up that survives a missed week, offline, full
> N5→N1, at a fair price.**

## 2. The competitive landscape (condensed)

| Tool | Strength | Fatal gap we exploit |
|------|----------|----------------------|
| **WaniKani** | The benchmark: radical→kanji→vocab + mnemonics + gating | Rigid fixed pace; no test-out; reading-only (no writing); online-only; paid; no phonetics |
| **Anki** | Best free engine (FSRS), unlimited | Tests but doesn't *teach*; config paralysis; ugly; card-making fatigue |
| **jpdb.io** | Frequency-ordered, auto i+1 sentences from a huge corpus | Brutal grind; no onboarding; no writing; online-only |
| **Renshuu** | Breadth + charming gamification + real free tier | UX overwhelm; no offline |
| **MaruMori** | All-in-one curriculum (closest strategic threat) | Web-only; pricey ($349 lifetime); weak writing; N2/N1 incomplete; no phonetics |
| **Skritter / Ringotan** | Best handwriting/production loops | Writing only — no reading/meaning system |
| **Kanji Koohii** | Community-voted mnemonic stories | No readings, no vocab; needs the RTK book |
| **Kanji Garden** | Best pedagogy (component order, groups confusables) | Weak habit loop, mobile, onboarding |
| **Duolingo** | The engagement gold standard | Shallow kanji; no production; the "kanji wall" |

**Hardest to beat, and on what axis:** WaniKani (mnemonic-SRS execution + brand) — so we don't
out-WaniKani it; we beat it on the axes it refuses (adaptivity, writing, offline, phonetics,
price). jpdb (corpus moat) — differentiate on structure/onboarding/production. MaruMori (same
integrated ambition) — stay ahead via offline + production + phonetic components + speed.

## 3. White space MnemoKanji can own

1. **A systematic phonetic-component reading system inside an SRS** — *essentially unclaimed.*
   Our [02 §E](02-LEARNING-METHOD.md) feature is the single biggest differentiator.
2. **The reading + writing + mnemonics trifecta in one tool** — no competitor does all three well.
3. **Offline-first** — a structural advantage a Rust/Dioxus app has and most rivals "literally
   cannot easily match" (Renshuu's dev: full offline is "thousands of hours").
4. **Rigorous *and* rewarding** — the market is bifurcated; nobody owns serious pedagogy + a
   humane, motivating habit loop.
5. **The "post-Duolingo / finished-RTK, now what?" bridge** — a large, stranded, motivated
   audience with no designed next step.
6. **Full N5→N1, offline, fair price, with production** — undercuts WaniKani (recognition-only)
   and MaruMori (pricey, incomplete at the top).

## 4. Anti-burnout & retention UX — the highest-leverage system

The #1 documented reason people quit kanji apps is the **review-debt death spiral** (miss a week
→ 1,000 due cards → accuracy craters → quit). It is *also* what every level-60 success story
hand-builds workarounds for. So MnemoKanji makes graceful survival a **default, not a workaround**:

- **Daily review cap (user-set target).** Never show more than, e.g., 60 due/day; overflow rolls
  forward smoothly instead of dumping all at once.
- **Backlog smoothing.** After a break, spread the accumulated debt over several days
  ("~40/morning") rather than 0-or-1000. FSRS over-due items are re-prioritized, not punished.
- **Decouple new lessons from due reviews.** Pausing *new* introductions never stops *reviews*,
  and a backlog never blocks the option to learn something new — the two queues are independent.
- **Vacation / illness mode.** Suspends streak penalties and freezes scheduling without corrupting
  memory state.
- **Active leech rescue.** Auto-detect chronic-fail items, **lift them out of the level-gating
  loop**, and **re-teach them differently** — regenerate the mnemonic with a different
  actor/angle, add an image, or group them with their confusables (§6) — rather than re-testing
  the same failing approach. (Directly leverages our editable-mnemonic + actor system.)
- **Forgiving input.** Typo tolerance, broad synonym acceptance, **retroactive credit** when a
  user adds a synonym, and a **one-tap undo**. Penalize *not knowing*, never *not typing it
  perfectly* (the most rage-inducing daily friction in WaniKani).
- **Placement / test-out.** A quick placement path + "I already know this" skip, so textbook
  graduates and returning learners don't grind known kanji (WaniKani's biggest early-bounce).

These map 1:1 onto the top user pain points in §7 and are, collectively, the most important
retention work in the project.

## 5. Engagement & reward design (fun + reward, safely)

**The guardrail (non-negotiable):** *gamify the return and the mastery — never the learning
itself.* Rewards must be **informational** (signaling real competence) not **controlling**
(bribing behavior), or they trigger the over-justification effect and erode the intrinsic interest
a self-motivated kanji learner already has. Keep a **learning-quality north star** (e.g.
mature-kanji retained / true recall accuracy) that **no engagement feature may degrade.**

**Tier 1 — high value, no backfire (build first):**
- **Mastery & progress visualization** — "learning / young / mature / mastered" counts, N5→N1
  progress bars, a **retention forecast** ("~94% of N4 next month"), review heatmap. This is the
  *informational* reward the science endorses, and competence is the need real learning satisfies.
- **Frictionless daily session + one gentle, dismissible prompt** — a "minimum viable session"
  (even 5 cards counts) keeps the habit alive on low-motivation days (Fogg: design for the tired
  Thursday).
- **Humane streak with built-in slack** — streak tied to *completing due reviews* (not a farmable
  XP target), with auto **streak-freeze**, a weekly **rest day**, and celebration of early
  milestones (3/7/14 days). Leniency *increases* persistence; never sell streak repair.
- **Milestone "kanji unlocked" celebrations** — small, unexpected, intangible rewards at real
  thresholds (finished a phonetic series, cleared N5). Unexpected + informational ⇒ no
  over-justification.

**Tier 2 — good value, mild guardrails:**
- **Flow "challenge dial"** — expose FSRS desired-retention as relaxed / balanced / intense, and
  surface when the learner is "in the zone." Turns the retention engine into *felt* flow — a
  differentiator no competitor offers.
- **Cooperative mnemonic sharing + voting** (later) — relatedness via contribution, and it
  *improves* the material (Koohii's only real moat). Cooperative, not competitive.
- **Personal records** — best week, longest streak, accuracy trend (compete with your past self,
  no social-comparison downside).

**Tier 3 — opt-in only, default OFF:**
- **Leagues/leaderboards** — small, peer-sized, reset weekly, improvement-focused. Risk:
  demotivates bottom-rankers and can make XP-the-goal displace learning.

**The "fun" core:** the intrinsic joy of kanji is the **"aha" of a character decomposing into
meaning + reading**. Make mnemonics vivid (even funny), the UI fast/beautiful, reviews tactile
(animation/sound/haptics), and end sessions while the learner still feels capable ("you're done
for today" is a feature). *Test:* would a session still be enjoyable with the streak counter
hidden? If yes, the fun is intrinsic and durable.

**Explicitly avoid:** farmable XP (performative learning), paid lives / paid streak repair,
guilt/shame notifications, pay-to-progress gates, demotivating public leaderboards, and any
over-gamified pile-on where the meta-game drowns the kanji.

## 6. Best ideas adopted from competitors

- WaniKani: dependency-gated radical→kanji→vocab + mnemonics (we already have; made *adaptive*).
- jpdb: auto **i+1 sentences** from a corpus (low-friction in-context reinforcement).
- KanjiDamage/WaniKani: **reading mnemonics with consistent on'yomi actors** ([07](07-CONTENT-GENERATION.md)).
- Kanji Garden: **deliberately group confusable look-alikes** (科/料, 寺/時) to teach
  discrimination — a cheap, high-payoff use of interleaving/distinctiveness ([02 §A](02-LEARNING-METHOD.md)).
- Renshuu: **level-scaled furigana that fades** as you advance — a natural form of our scaffold
  fade ([07 §5](07-CONTENT-GENERATION.md)).
- Ringotan: **trace → fade → blank-recall** writing scaffold.
- Skritter: **grading tiers** (snap-assist → raw) so production stays useful beginner→advanced.
- Koohii: community-editable, **upvoted** stories (compounding moat, later layer).

## 7. Voice-of-user: top pain points → our response

| # | Pain point (app) | MnemoKanji response |
|---|------------------|---------------------|
| 1 | Review pile-up / debt spiral (WK, Anki) | §4 daily cap + backlog smoothing + vacation mode + decoupled queues |
| 2 | Rigid fixed pace (WK) | User-set daily targets; self-paced; per-day batching |
| 3 | Can't skip known kanji (WK) | Placement test + "I already know this" test-out |
| 4 | Leeches block progress (WK, Anki) | Active leech rescue: un-gate + re-teach differently |
| 5 | Tests but doesn't teach (Anki) | Built-in mnemonics + component logic + the "why" |
| 6 | Config paralysis (Anki) | Zero-config guided path; power-user control optional underneath |
| 7 | No undo / typo & synonym punishment (WK) | Forgiving input + retroactive credit + one-tap undo |
| 8 | Card-creation fatigue (Anki) | Pre-built content; auto i+1 context; nothing to build |
| 9 | Shallow, no production (Duolingo) | Real production track (write + cloze) |
| 10 | Kanji in isolation (most) | In-context vocab/sentences from day one |
| 11 | Mid-level slump / useless vocab (WK) | Frequency-ordered useful vocab + "seen in the wild" + meaningful milestones |
| 12 | Ugly UI / cost (Anki) | Fast, beautiful, offline, fair price (MIT app) |

**AI note:** appetite for an "explain why" tutor is high but trust is the blocker (hallucinated
grammar burns users). Any AI explanation must be **vetted/grounded**, not raw LLM output — our
mnemonics already go through a verify+judge pipeline ([07 §2](07-CONTENT-GENERATION.md)); apply
the same rigor to any explanatory AI.

## 8. The learning-quality north star

One metric the whole product is optimized for and that **no engagement feature may degrade**:
**long-term retention of mature kanji per study-minute** (operationally: count of kanji at
`stability ≥ 21d` with true recall accuracy above target, per minute studied). Engagement features
are judged by whether they raise *this*, not DAU or streak length. This is the discipline that
keeps MnemoKanji on the "rigorous *and* rewarding" side of the market split.
