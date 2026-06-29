//! Core domain types shared by the scheduler and session engine.

use chrono::{DateTime, Utc};
pub use rs_fsrs::{Card, Rating, State};

/// A kanji is one concept but is scheduled on two independent FSRS tracks (docs/02 §C).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrackKind {
    /// Input skills: kanji→meaning, kanji→reading (gates level progression).
    Comprehension,
    /// Output skills: meaning→write, in-context cloze (runs in parallel, never gates).
    Production,
}

impl TrackKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TrackKind::Comprehension => "comprehension",
            TrackKind::Production => "production",
        }
    }
}

impl std::str::FromStr for TrackKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "comprehension" => Ok(TrackKind::Comprehension),
            "production" => Ok(TrackKind::Production),
            _ => Err(()),
        }
    }
}

/// One scheduled track for one kanji: the FSRS `Card` plus its identity.
#[derive(Clone, Debug)]
pub struct Track {
    pub kanji_id: i64,
    pub kind: TrackKind,
    pub card: Card,
    pub introduced_at: DateTime<Utc>,
}
