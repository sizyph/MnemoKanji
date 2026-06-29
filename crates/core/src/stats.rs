//! Progress/engagement helpers: mastery buckets and a humane study streak.
//!
//! The streak is "alive" if you studied today *or* yesterday (so missing the current day doesn't
//! instantly zero it). Engagement is informational — it never gates or alters the learning itself.

use chrono::NaiveDate;

use crate::domain::Card;

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
