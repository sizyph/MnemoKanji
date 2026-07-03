//! Progress/engagement helpers: mastery buckets and a humane study streak.
//!
//! The streak is "alive" if you studied today *or* yesterday (so missing the current day doesn't
//! instantly zero it). Engagement is informational — it never gates or alters the learning itself.

use std::collections::HashSet;

use chrono::NaiveDate;

use crate::domain::{Card, TrackKind};
use crate::session::StudyState;

/// How well-learned a track is, by FSRS stability (cf. Anki's young/mature split at ~21 days).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mastery {
    New,
    Learning,
    Young,
    Mature,
}

impl Mastery {
    pub fn as_str(self) -> &'static str {
        match self {
            Mastery::New => "new",
            Mastery::Learning => "learning",
            Mastery::Young => "young",
            Mastery::Mature => "mature",
        }
    }
}

/// Classify a card's mastery. `mature_days` is the level-clear stability threshold (≈21).
pub fn mastery(card: &Card, mature_days: f64) -> Mastery {
    if card.reps == 0 {
        Mastery::New
    } else if card.stability < 7.0 {
        Mastery::Learning
    } else if card.stability < mature_days {
        Mastery::Young
    } else {
        Mastery::Mature
    }
}

/// Number of introduced kanji: comprehension tracks whose kanji exists in the content.
/// `active` is [`crate::ContentView::id_set`]; orphan tracks (quarantined) never count.
pub fn introduced_count(state: &StudyState, active: &HashSet<i64>) -> usize {
    state
        .tracks
        .keys()
        .filter(|(id, k)| *k == TrackKind::Comprehension && active.contains(id))
        .count()
}

/// Mastery-bucket counts `[new, learning, young, mature]` over the active comprehension
/// tracks. Orphan tracks (id not in `active`) are quarantined out, like everywhere else.
pub fn mastery_counts(state: &StudyState, active: &HashSet<i64>, mature_days: f64) -> [usize; 4] {
    let mut counts = [0usize; 4];
    for ((id, kind), t) in &state.tracks {
        if *kind != TrackKind::Comprehension || !active.contains(id) {
            continue;
        }
        counts[mastery(&t.card, mature_days) as usize] += 1;
    }
    counts
}

/// Current and longest study streak (in days) from a sorted, unique list of study dates.
/// The current streak counts back from the most recent date only if that date is today or
/// yesterday relative to `today`.
pub fn streak(days_sorted_unique: &[NaiveDate], today: NaiveDate) -> (u32, u32) {
    let days = days_sorted_unique;
    if days.is_empty() {
        return (0, 0);
    }

    // Longest consecutive run anywhere in the history.
    let (mut longest, mut run) = (1u32, 1u32);
    for w in days.windows(2) {
        if w[0].succ_opt() == Some(w[1]) {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 1;
        }
    }

    // Current run, ending at the most recent date, but only "alive" if that's today or yesterday.
    let last = *days.last().unwrap();
    let alive = last == today || Some(last) == today.pred_opt();
    let current = if alive {
        let mut c = 1u32;
        let mut prev = last;
        for &d in days.iter().rev().skip(1) {
            if d.succ_opt() == Some(prev) {
                c += 1;
                prev = d;
            } else {
                break;
            }
        }
        c
    } else {
        0
    };

    (current, longest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn empty_history_is_zero() {
        assert_eq!(streak(&[], d(2026, 6, 29)), (0, 0));
    }

    /// Progress counts quarantine orphan tracks (id not in content) — same invariant as
    /// the engine's scheduling/budget filters, pinned here for the UI-facing counts.
    #[test]
    fn progress_counts_quarantine_orphans() {
        use crate::domain::Track;
        use crate::scheduler::Scheduler;
        use chrono::{DateTime, Utc};

        let now = DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap();
        let active: HashSet<i64> = [19968, 19971].into_iter().collect();
        let mut state = StudyState::default();
        // One active mature, one active new, one orphan mature (999 ∉ active).
        for (id, stability, reps) in [(19968, 30.0, 5), (19971, 0.0, 0), (999, 30.0, 5)] {
            let mut card = Scheduler::new_card(now);
            card.stability = stability;
            card.reps = reps;
            state.tracks.insert(
                (id, TrackKind::Comprehension),
                Track {
                    kanji_id: id,
                    kind: TrackKind::Comprehension,
                    card,
                    introduced_at: now,
                },
            );
        }
        // A production track never counts as "introduced".
        state.tracks.insert(
            (19968, TrackKind::Production),
            Track {
                kanji_id: 19968,
                kind: TrackKind::Production,
                card: Scheduler::new_card(now),
                introduced_at: now,
            },
        );

        assert_eq!(introduced_count(&state, &active), 2, "orphan excluded");
        assert_eq!(
            mastery_counts(&state, &active, 21.0),
            [1, 0, 0, 1],
            "one new + one mature; the mature orphan must not count"
        );
    }

    #[test]
    fn current_streak_alive_today() {
        let days = [d(2026, 6, 27), d(2026, 6, 28), d(2026, 6, 29)];
        assert_eq!(streak(&days, d(2026, 6, 29)), (3, 3));
    }

    #[test]
    fn current_streak_alive_yesterday_not_broken_yet() {
        // studied through yesterday; today not yet — streak still alive.
        let days = [d(2026, 6, 27), d(2026, 6, 28)];
        assert_eq!(streak(&days, d(2026, 6, 29)).0, 2);
    }

    #[test]
    fn current_breaks_after_a_gap_but_longest_remembers() {
        let days = [d(2026, 6, 1), d(2026, 6, 2), d(2026, 6, 3), d(2026, 6, 28)];
        // today is the 30th: last study (28th) is two days ago -> current 0; longest run was 3.
        assert_eq!(streak(&days, d(2026, 6, 30)), (0, 3));
    }
}
