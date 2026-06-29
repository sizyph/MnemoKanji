//! The session engine: introductions (ordered, prerequisite-gated, daily-budgeted), due-review
//! selection (comprehension-first, production deferred behind same-day comprehension), grading
//! (worst-of multi-mode), production activation, and comprehension-gated level unlocking.
//!
//! Storage-agnostic: it operates on an in-memory `StudyState` + a `ContentView`. The data crate
//! loads those from SQLite and persists changes back.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use rs_fsrs::Rating;

use crate::domain::{Track, TrackKind};
use crate::scheduler::{comprehension_mature, Scheduler};

/// Static content about one kanji needed for scheduling (from the seed DB).
#[derive(Clone, Debug)]
pub struct KanjiMeta {
    pub id: i64,
    /// JLPT level as a number: 5 = N5 (learned first) .. 1 = N1.
    pub level: u8,
    /// Within-level learning order (lower = earlier).
    pub intro_rank: i64,
    /// Other kanji that must be introduced before this one (component prerequisites).
    pub prereq_kanji: Vec<i64>,
}

pub struct ContentView {
    pub kanji: Vec<KanjiMeta>,
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub new_per_day: usize,
    pub daily_review_cap: usize,
    pub mature_stability_days: f64,
    /// Comprehension stability at which the production track activates.
    pub production_gate_days: f64,
    /// Fraction of a level's kanji that must be mature to unlock the next level.
    pub clear_fraction: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            new_per_day: 10,
            daily_review_cap: 60,
            mature_stability_days: 21.0,
            production_gate_days: 7.0,
            clear_fraction: 0.9,
        }
    }
}

/// All mutable per-user study state.
#[derive(Default)]
pub struct StudyState {
    pub tracks: HashMap<(i64, TrackKind), Track>,
    /// Highest unlocked level number (5 = N5 first). 0 = nothing unlocked yet.
    pub unlocked_level: u8,
}

pub struct Engine<'a> {
    pub content: &'a ContentView,
    pub settings: Settings,
    pub scheduler: Scheduler,
}

impl<'a> Engine<'a> {
    pub fn new(content: &'a ContentView, settings: Settings) -> Self {
        Self {
            content,
            settings,
            scheduler: Scheduler::new(),
        }
    }

    fn introduced(&self, state: &StudyState, id: i64) -> bool {
        state.tracks.contains_key(&(id, TrackKind::Comprehension))
    }

    /// Kanji eligible to introduce: in the unlocked level, not yet introduced, all prerequisites
    /// introduced; ordered by intro_rank.
    fn introducible(&self, state: &StudyState) -> Vec<i64> {
        let mut v: Vec<&KanjiMeta> = self
            .content
            .kanji
            .iter()
            .filter(|k| {
                k.level == state.unlocked_level
                    && !self.introduced(state, k.id)
                    && k.prereq_kanji.iter().all(|p| self.introduced(state, *p))
            })
            .collect();
        v.sort_by_key(|k| k.intro_rank);
        v.into_iter().map(|k| k.id).collect()
    }

    fn introduced_today(&self, state: &StudyState, now: DateTime<Utc>) -> usize {
        let today = now.date_naive();
        state
            .tracks
            .values()
            .filter(|t| t.kind == TrackKind::Comprehension && t.introduced_at.date_naive() == today)
            .count()
    }

    /// Introduce up to the remaining daily budget of new kanji; returns the introduced ids.
    pub fn introduce_new(&self, state: &mut StudyState, now: DateTime<Utc>) -> Vec<i64> {
        let budget = self
            .settings
            .new_per_day
            .saturating_sub(self.introduced_today(state, now));
        let ids: Vec<i64> = self.introducible(state).into_iter().take(budget).collect();
        for id in &ids {
            state.tracks.insert(
                (*id, TrackKind::Comprehension),
                Track {
                    kanji_id: *id,
                    kind: TrackKind::Comprehension,
                    card: Scheduler::new_card(now),
                    introduced_at: now,
                },
            );
        }
        ids
    }

    /// Due items for review at `now`: comprehension first (earliest due first), then production —
    /// excluding any production track whose comprehension is also due today (anti-priming). Capped.
    pub fn due_items(&self, state: &StudyState, now: DateTime<Utc>) -> Vec<(i64, TrackKind)> {
        let comp_due: HashSet<i64> = state
            .tracks
            .values()
            .filter(|t| t.kind == TrackKind::Comprehension && t.card.due <= now)
            .map(|t| t.kanji_id)
            .collect();

        let mut comp: Vec<&Track> = state
            .tracks
            .values()
            .filter(|t| t.kind == TrackKind::Comprehension && t.card.due <= now)
            .collect();
        let mut prod: Vec<&Track> = state
            .tracks
            .values()
            .filter(|t| {
                t.kind == TrackKind::Production
                    && t.card.due <= now
                    && !comp_due.contains(&t.kanji_id)
            })
            .collect();
        comp.sort_by_key(|t| t.card.due);
        prod.sort_by_key(|t| t.card.due);

        comp.into_iter()
            .chain(prod)
            .take(self.settings.daily_review_cap)
            .map(|t| (t.kanji_id, t.kind))
            .collect()
    }

    /// Grade a track with the worst of the per-mode ratings (one FSRS update). May activate the
    /// production track and may unlock the next level. No-op if the track doesn't exist.
    pub fn grade(
        &self,
        state: &mut StudyState,
        kanji_id: i64,
        kind: TrackKind,
        ratings: &[Rating],
        now: DateTime<Utc>,
    ) {
        let Some(worst) = ratings.iter().copied().min_by_key(|r| *r as u8) else {
            return;
        };
        let Some(track) = state.tracks.get(&(kanji_id, kind)) else {
            return;
        };
        let new_card = self
            .scheduler
            .review(kanji_id, kind, track.card.clone(), worst, now);
        if let Some(t) = state.tracks.get_mut(&(kanji_id, kind)) {
            t.card = new_card.clone();
        }

        if kind == TrackKind::Comprehension {
            let has_prod = state
                .tracks
                .contains_key(&(kanji_id, TrackKind::Production));
            if !has_prod
                && new_card.stability >= self.settings.production_gate_days
                && new_card.reps >= 2
            {
                state.tracks.insert(
                    (kanji_id, TrackKind::Production),
                    Track {
                        kanji_id,
                        kind: TrackKind::Production,
                        card: Scheduler::new_card(now),
                        introduced_at: now,
                    },
                );
            }
            self.maybe_unlock_level(state);
        }
    }

    /// If ≥ `clear_fraction` of the unlocked level's kanji have a mature comprehension track,
    /// unlock the next (lower-numbered) level.
    fn maybe_unlock_level(&self, state: &mut StudyState) {
        let level = state.unlocked_level;
        if level <= 1 {
            return;
        }
        let in_level: Vec<&KanjiMeta> = self
            .content
            .kanji
            .iter()
            .filter(|k| k.level == level)
            .collect();
        if in_level.is_empty() {
            return;
        }
        let mature = in_level
            .iter()
            .filter(|k| {
                state
                    .tracks
                    .get(&(k.id, TrackKind::Comprehension))
                    .map(|t| comprehension_mature(&t.card, self.settings.mature_stability_days))
                    .unwrap_or(false)
            })
            .count();
        if mature as f64 / in_level.len() as f64 >= self.settings.clear_fraction {
            state.unlocked_level = level - 1;
        }
    }
}

#[cfg(test)]
mod sim {
    use super::*;
    use chrono::Duration;

    /// Small synthetic curriculum: N5 has 3 kanji (k3 needs k1,k2); N4 has 2.
    fn content() -> ContentView {
        ContentView {
            kanji: vec![
                KanjiMeta {
                    id: 1,
                    level: 5,
                    intro_rank: 0,
                    prereq_kanji: vec![],
                },
                KanjiMeta {
                    id: 2,
                    level: 5,
                    intro_rank: 1,
                    prereq_kanji: vec![],
                },
                KanjiMeta {
                    id: 3,
                    level: 5,
                    intro_rank: 2,
                    prereq_kanji: vec![1, 2],
                },
                KanjiMeta {
                    id: 4,
                    level: 4,
                    intro_rank: 0,
                    prereq_kanji: vec![],
                },
                KanjiMeta {
                    id: 5,
                    level: 4,
                    intro_rank: 1,
                    prereq_kanji: vec![],
                },
            ],
        }
    }

    #[test]
    fn k3_introduced_only_after_its_prerequisites() {
        let content = content();
        let settings = Settings {
            new_per_day: 1,
            ..Default::default()
        };
        let engine = Engine::new(&content, settings);
        let mut state = StudyState {
            unlocked_level: 5,
            ..Default::default()
        };
        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        // Day 0: only k1 (rank 0) can be introduced (budget 1).
        let d0 = engine.introduce_new(&mut state, now);
        assert_eq!(d0, vec![1]);
        // k3 is not introducible yet (needs k2 too).
        assert!(!state.tracks.contains_key(&(3, TrackKind::Comprehension)));
    }

    #[test]
    fn multi_day_run_matures_activates_production_and_unlocks_n4() {
        let content = content();
        let settings = Settings {
            new_per_day: 2,
            ..Default::default()
        };
        let engine = Engine::new(&content, settings);
        let mut state = StudyState {
            unlocked_level: 5,
            ..Default::default()
        };
        let mut now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();

        let mut production_seen = false;
        let mut n4_introduced_while_n5_locked = false;

        for _day in 0..150 {
            engine.introduce_new(&mut state, now);
            for (kid, kind) in engine.due_items(&state, now) {
                engine.grade(&mut state, kid, kind, &[Rating::Good], now);
            }
            if state
                .tracks
                .keys()
                .any(|(_, k)| *k == TrackKind::Production)
            {
                production_seen = true;
            }
            // N4 must never be introduced while N5 is still the unlocked level.
            if state.unlocked_level == 5
                && state.tracks.contains_key(&(4, TrackKind::Comprehension))
            {
                n4_introduced_while_n5_locked = true;
            }
            now += Duration::days(1);
        }

        // All three N5 kanji introduced and matured.
        for id in [1, 2, 3] {
            let t = state
                .tracks
                .get(&(id, TrackKind::Comprehension))
                .unwrap_or_else(|| panic!("kanji {id} comprehension track missing"));
            assert!(
                comprehension_mature(&t.card, 21.0),
                "kanji {id} comprehension should be mature (stability {})",
                t.card.stability
            );
        }
        assert!(
            production_seen,
            "production track should activate after comprehension matures"
        );
        assert!(
            !n4_introduced_while_n5_locked,
            "N4 must not be introduced before it unlocks"
        );
        // N5 must have cleared (unlocked past 5); with only 5 kanji over 150 days it cascades
        // further, which is correct behaviour — assert progression, not an exact level.
        assert!(
            state.unlocked_level <= 4,
            "N5 should clear (unlock the next level) once ≥90% of N5 is mature; got level {}",
            state.unlocked_level
        );
        assert!(
            state.tracks.contains_key(&(4, TrackKind::Comprehension)),
            "an N4 kanji should be introduced after N5 unlocks"
        );
    }
}
