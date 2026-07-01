//! Thin wrapper over `rs-fsrs` (FSRS-4.5) with MnemoKanji's parameters and conventions.
//!
//! Config per docs/03-DESIGN §3 / docs/06-RESEARCH §7: desired retention 0.90, fuzz ON (the crate
//! default is off; fuzz de-clusters bulk-added kanji), seeded per (kanji, track) for determinism.

use chrono::{DateTime, Utc};
use rs_fsrs::{Card, Parameters, Rating, Seed, FSRS};

use crate::domain::TrackKind;

/// Comprehension is "mature" (counts toward clearing a level) at this stability — with `reps >= 2`
/// so a single confident `Easy` first answer (init stability ≈ 15.5) can't trip the gate.
pub const MATURE_STABILITY_DAYS: f64 = 21.0;

#[derive(Clone)]
pub struct Scheduler {
    params: Parameters,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(0.9)
    }
}

impl Scheduler {
    /// `desired_retention` is the target recall probability (the challenge dial); 0.9 is the
    /// recommended default.
    pub fn new(desired_retention: f64) -> Self {
        // Parameters::default() already gives maximum_interval 36500, enable_short_term true, and
        // the published FSRS-4.5 weights. We set retention and flip fuzz on.
        Self {
            params: Parameters {
                request_retention: desired_retention,
                enable_fuzz: true,
                ..Default::default()
            },
        }
    }

    /// A fresh card, due immediately at `now` (introduction includes the first test).
    pub fn new_card(now: DateTime<Utc>) -> Card {
        Card {
            due: now,
            last_review: now,
            ..Card::new()
        }
    }

    /// Apply one rating and return the updated card. Fuzz is seeded per (kanji, kind) so cards
    /// added on the same day don't pile onto the same future day forever.
    pub fn review(
        &self,
        kanji_id: i64,
        kind: TrackKind,
        card: Card,
        rating: Rating,
        now: DateTime<Utc>,
    ) -> Card {
        let mut params = self.params.clone();
        params.seed = Seed::new(format!("{kanji_id}:{}", kind.as_str()));
        FSRS::new(params).next(card, now, rating).card
    }
}

/// Whether a comprehension card has reached the maturity gate.
pub fn comprehension_mature(card: &Card, mature_days: f64) -> bool {
    card.stability >= mature_days && card.reps >= 2
}
