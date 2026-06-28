//! MnemoKanji core — UI-agnostic domain logic.
//!
//! This crate will hold the domain model, the FSRS-driven two-track scheduler, the session
//! engine, and the frequency-weighted topological learning order. See `docs/03-DESIGN.md`.
//!
//! M0 scaffold: a version helper + a smoke test, so the workspace and CI have something real to
//! build and run. Real logic lands in M2.

/// Returns the crate version (from Cargo). Placeholder wiring exercised by the UI in M0.
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
