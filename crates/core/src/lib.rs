//! MnemoKanji core — UI-agnostic domain logic.
//!
//! The two-track FSRS scheduler and the session engine (introductions, due selection, grading,
//! production activation, comprehension-gated level unlocking). Storage-agnostic: it operates on
//! an in-memory [`session::StudyState`] + [`session::ContentView`] that the data crate loads from
//! and persists to SQLite. See `docs/03-DESIGN.md`.

pub mod domain;
pub mod scheduler;
pub mod session;

pub use domain::{Card, Rating, State, Track, TrackKind};
pub use scheduler::{comprehension_mature, Scheduler, MATURE_STABILITY_DAYS};
pub use session::{ContentView, Engine, KanjiMeta, Settings, StudyState};

/// Crate version (from Cargo).
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!version().is_empty());
    }
}
