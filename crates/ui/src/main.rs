//! MnemoKanji — desktop/mobile app (Dioxus, system WebView renderer).
//!
//! M0 scaffold: a hello-world window that also exercises the workspace wiring by reading the
//! core crate's version. The review loop, screens, and state land from M3 on. See `docs/03-DESIGN.md`.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let core_version = mnemokanji_core::version();
    rsx! {
        main {
            style: "font-family: system-ui, sans-serif; max-width: 40rem; margin: 0 auto; padding: 3rem 2rem;",
            h1 { style: "margin-bottom: 0.25rem;", "MnemoKanji" }
            p { style: "color: #555; margin-top: 0;", "Learn every JLPT kanji — efficiently." }
            p {
                style: "color: #999; font-size: 0.85rem;",
                "core v{core_version} · scaffold (M0)"
            }
        }
    }
}
