//! MnemoKanji data — SQLite storage layer.
//!
//! Two databases (docs/03-DESIGN §7): the read-only bundled **seed** (kanji/readings/vocab/…,
//! exposed to the engine as a [`mnemokanji_core::ContentView`]) and a writable per-user **state**
//! store (FSRS tracks + progress). The seed is never mutated; only user state is.

pub mod content;
pub mod state;

pub use content::ContentRepo;
pub use state::StateStore;
