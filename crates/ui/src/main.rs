//! MnemoKanji — desktop app (Dioxus, system WebView).
//!
//! M3 slice 1: dashboard + recognition review loop, wired to the core engine + seed, persisting
//! every grade. Slice 2: stroke-order animation, the kanji-detail hub, a browse grid, and an
//! enriched reveal (reading-in-context + mnemonic). Other test modes + anti-burnout land next.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use chrono::Utc;
use dioxus::prelude::*;
use mnemokanji_core::{ContentView, Engine, Rating, Settings, StudyState, TrackKind};
use mnemokanji_data::{BrowseItem, ContentRepo, KanjiDetail, StateStore};

const SEED_DB: &str = "assets/seed.sqlite";
const USER_DB: &str = "user.sqlite";
const CSS: &str = include_str!("app.css");

/// Process-wide source of truth: the read-only seed, the writable user state, and settings.
struct Backend {
    content_repo: ContentRepo,
    content: ContentView,
    state_store: StateStore,
    state: StudyState,
    settings: Settings,
}

static BACKEND: OnceLock<Mutex<Backend>> = OnceLock::new();

fn backend() -> std::sync::MutexGuard<'static, Backend> {
    BACKEND
        .get()
        .expect("backend initialized in main")
        .lock()
        .expect("backend lock")
}

fn main() {
    let seed = std::env::var("MNEMOKANJI_SEED").unwrap_or_else(|_| SEED_DB.into());
    let user = std::env::var("MNEMOKANJI_USER_DB").unwrap_or_else(|_| USER_DB.into());

    let content_repo = ContentRepo::open(&seed).unwrap_or_else(|e| {
        panic!("open seed db {seed}: {e} (run scripts/fetch-sources.sh + cargo run -p mnemokanji-content)")
    });
    let content = content_repo.content_view().expect("load content view");
    let state_store = StateStore::open(&user).expect("open user state db");
    let state = state_store.load_state().expect("load user state");

    BACKEND
        .set(Mutex::new(Backend {
            content_repo,
            content,
            state_store,
            state,
            settings: Settings::default(),
        }))
        .map_err(|_| ())
        .expect("set backend once");

    dioxus::launch(App);
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Dashboard,
    Session,
    Browse,
    Detail,
}

#[derive(Clone, PartialEq)]
struct Dash {
    level: u8,
    total: usize,
    introduced: usize,
    due: usize,
    new_remaining: usize,
}

fn compute_dash(b: &Backend) -> Dash {
    let now = Utc::now();
    let engine = Engine::new(&b.content, b.settings.clone());
    let level = b.state.unlocked_level;
    Dash {
        level,
        total: b.content.kanji.iter().filter(|k| k.level == level).count(),
        introduced: b
            .state
            .tracks
            .keys()
            .filter(|(_, k)| *k == TrackKind::Comprehension)
            .count(),
        due: engine.due_items(&b.state, now).len(),
        new_remaining: engine.new_remaining_today(&b.state, now),
    }
}

#[derive(Clone, Copy)]
struct AppState {
    screen: Signal<Screen>,
    queue: Signal<Vec<(i64, TrackKind)>>,
    current: Signal<Option<KanjiDetail>>,
    revealed: Signal<bool>,
    detail: Signal<Option<KanjiDetail>>,
    browse: Signal<Vec<BrowseItem>>,
}

#[component]
fn App() -> Element {
    let s = AppState {
        screen: use_signal(|| Screen::Dashboard),
        queue: use_signal(Vec::new),
        current: use_signal(|| None),
        revealed: use_signal(|| false),
        detail: use_signal(|| None),
        browse: use_signal(Vec::new),
    };

    rsx! {
        style { dangerous_inner_html: CSS }
        div { class: "app",
            {match (s.screen)() {
                Screen::Dashboard => dashboard_view(s),
                Screen::Session => session_view(s),
                Screen::Browse => browse_view(s),
                Screen::Detail => detail_view(s),
            }}
        }
    }
}

fn dashboard_view(s: AppState) -> Element {
    let d = compute_dash(&backend());
    let level = d.level;
    let due = d.due;
    let new_remaining = d.new_remaining;
    let learned = format!("{}/{}", d.introduced, d.total);
    let nothing = due == 0 && new_remaining == 0;

    rsx! {
        div { class: "card",
            h1 { "MnemoKanji" }
            p { class: "sub", "JLPT N{level}" }
            div { class: "stats",
                div { class: "stat", div { class: "num", "{due}" } div { class: "lbl", "due" } }
                div { class: "stat", div { class: "num", "{new_remaining}" } div { class: "lbl", "new today" } }
                div { class: "stat", div { class: "num", "{learned}" } div { class: "lbl", "learned" } }
            }
            if nothing {
                p { class: "done", "All caught up for now \u{2728}" }
            } else {
                button { class: "primary", onclick: move |_| start_session(s), "Start session" }
            }
            button { class: "secondary", onclick: move |_| show_browse(s), "Browse N{level}" }
        }
    }
}

fn session_view(s: AppState) -> Element {
    let Some(k) = (s.current)() else {
        return rsx! { div { class: "card", p { "Loading\u{2026}" } } };
    };
    let glyph = k.glyph.clone();
    let keyword = k.keyword.clone();
    let left = (s.queue)().len();
    let is_revealed = (s.revealed)();
    let reading_str = k
        .readings
        .iter()
        .filter(|r| r.is_dominant)
        .map(|r| r.reading.clone())
        .collect::<Vec<_>>()
        .join("   ");
    let examples: Vec<String> = k
        .vocab
        .iter()
        .take(2)
        .map(|v| format!("{} ({}) \u{2014} {}", v.surface, v.reading, v.gloss))
        .collect();
    let mnemonic = k.mnemonic.clone();

    rsx! {
        div { class: "review",
            div { class: "topbar",
                button { class: "link", onclick: move |_| { let mut sc = s.screen; sc.set(Screen::Dashboard); }, "\u{2190} end" }
                span { class: "left", "{left} left" }
            }
            // Recognition prompt: a clean glyph only (modality segregation).
            div { class: "glyph", "{glyph}" }
            if !is_revealed {
                button { class: "primary reveal", onclick: move |_| { let mut r = s.revealed; r.set(true); }, "Show answer" }
            } else {
                div { class: "answer",
                    div { class: "keyword", "{keyword}" }
                    if !reading_str.is_empty() {
                        div { class: "reading", "{reading_str}" }
                    }
                    // Reading practiced in context, not in isolation.
                    div { class: "examples",
                        for ex in examples.iter() {
                            div { class: "example", "{ex}" }
                        }
                    }
                    if let Some(m) = mnemonic {
                        div { class: "mnemonic", "{m}" }
                    }
                    div { class: "grades",
                        button { class: "grade again", onclick: move |_| do_grade(Rating::Again, s), "Again" }
                        button { class: "grade hard", onclick: move |_| do_grade(Rating::Hard, s), "Hard" }
                        button { class: "grade good", onclick: move |_| do_grade(Rating::Good, s), "Good" }
                        button { class: "grade easy", onclick: move |_| do_grade(Rating::Easy, s), "Easy" }
                    }
                }
            }
        }
    }
}

fn browse_view(s: AppState) -> Element {
    let introduced: HashSet<i64> = {
        let g = backend();
        g.state
            .tracks
            .keys()
            .filter(|(_, k)| *k == TrackKind::Comprehension)
            .map(|(id, _)| *id)
            .collect()
    };
    let items: Vec<(i64, String, String, &'static str)> = (s.browse)()
        .into_iter()
        .map(|it| {
            let cls = if introduced.contains(&it.id) {
                "k-cell learned"
            } else {
                "k-cell"
            };
            (it.id, it.glyph, it.keyword, cls)
        })
        .collect();

    rsx! {
        div { class: "browse",
            div { class: "topbar",
                button { class: "link", onclick: move |_| { let mut sc = s.screen; sc.set(Screen::Dashboard); }, "\u{2190} back" }
                span { class: "title", "Browse" }
            }
            div { class: "grid",
                for (id, glyph, kw, cls) in items {
                    button { key: "{id}", class: cls, onclick: move |_| show_detail(id, s),
                        div { class: "cell-glyph", "{glyph}" }
                        div { class: "cell-kw", "{kw}" }
                    }
                }
            }
        }
    }
}

fn detail_view(s: AppState) -> Element {
    let Some(k) = (s.detail)() else {
        return rsx! { div { class: "card", p { "Loading\u{2026}" } } };
    };
    let glyph = k.glyph.clone();
    let keyword = k.keyword.clone();
    let strokes = k.stroke_paths.len();
    let on: Vec<(String, bool)> = k
        .readings
        .iter()
        .filter(|r| r.kind == "on")
        .map(|r| (r.reading.clone(), r.is_dominant))
        .collect();
    let kun: Vec<(String, bool)> = k
        .readings
        .iter()
        .filter(|r| r.kind == "kun")
        .map(|r| (r.reading.clone(), r.is_dominant))
        .collect();
    let meanings = k.meanings.join(", ");
    let components: Vec<(String, String)> = k
        .components
        .iter()
        .map(|c| {
            (
                c.glyph.clone(),
                c.actor.clone().unwrap_or_else(|| "\u{2014}".into()),
            )
        })
        .collect();
    let mnemonic = k.mnemonic.clone();
    let vocab: Vec<(String, String, String)> = k
        .vocab
        .iter()
        .map(|v| (v.surface.clone(), v.reading.clone(), v.gloss.clone()))
        .collect();
    let sentences: Vec<(String, String)> = k
        .sentences
        .iter()
        .map(|x| (x.jp.clone(), x.en.clone()))
        .collect();

    rsx! {
        div { class: "detail",
            div { class: "topbar",
                button { class: "link", onclick: move |_| { let mut sc = s.screen; sc.set(Screen::Browse); }, "\u{2190} back" }
                span { class: "title", "{glyph}  \u{00b7}  {keyword}" }
            }
            div { class: "detail-grid",
                div { class: "stroke-box",
                    {stroke_svg(&k.stroke_paths)}
                    div { class: "stroke-count", "{strokes} strokes" }
                }
                div { class: "facts",
                    div { class: "row", span { class: "rk", "on" } span { class: "rv",
                        for (t, dom) in on.iter() {
                            span { class: if *dom { "reading-chip dom" } else { "reading-chip" }, "{t}" }
                        }
                    } }
                    div { class: "row", span { class: "rk", "kun" } span { class: "rv",
                        for (t, dom) in kun.iter() {
                            span { class: if *dom { "reading-chip dom" } else { "reading-chip" }, "{t}" }
                        }
                    } }
                    div { class: "row", span { class: "rk", "mean" } span { class: "rv plain", "{meanings}" } }
                }
            }
            div { class: "section",
                h3 { "Components" }
                div { class: "components",
                    for (g, actor) in components.iter() {
                        div { class: "component", span { class: "cglyph", "{g}" } span { class: "cactor", "{actor}" } }
                    }
                }
            }
            if let Some(m) = mnemonic {
                div { class: "section",
                    h3 { "Mnemonic" }
                    p { class: "mnemonic-text", "{m}" }
                }
            }
            div { class: "section",
                h3 { "Vocabulary" }
                for (surface, reading, gloss) in vocab.iter() {
                    div { class: "vocab-row",
                        span { class: "vsurface", "{surface}" }
                        span { class: "vreading", "{reading}" }
                        span { class: "vgloss", "{gloss}" }
                    }
                }
            }
            div { class: "section",
                h3 { "Examples" }
                for (jp, en) in sentences.iter() {
                    div { class: "sentence",
                        div { class: "sjp", "{jp}" }
                        div { class: "sen", "{en}" }
                    }
                }
            }
        }
    }
}

/// An animated stroke-order SVG from KanjiVG path data (CSS draws each stroke in turn).
fn stroke_svg(paths: &[String]) -> Element {
    let timed: Vec<(String, String, usize)> = paths
        .iter()
        .enumerate()
        .map(|(i, d)| {
            (
                d.clone(),
                format!("animation-delay: {:.2}s", i as f64 * 0.35),
                i,
            )
        })
        .collect();
    rsx! {
        svg { class: "strokes", "viewBox": "0 0 109 109",
            for (d, style, i) in timed {
                path { key: "{i}", class: "stroke", d: "{d}", style: "{style}" }
            }
        }
    }
}

// --- actions ---

fn show_browse(s: AppState) {
    let list = {
        let g = backend();
        g.content_repo.browse("N5").unwrap_or_default()
    };
    let mut browse = s.browse;
    let mut screen = s.screen;
    browse.set(list);
    screen.set(Screen::Browse);
}

fn show_detail(id: i64, s: AppState) {
    let d = {
        let g = backend();
        g.content_repo.kanji_detail(id).ok()
    };
    let mut detail = s.detail;
    let mut screen = s.screen;
    detail.set(d);
    screen.set(Screen::Detail);
}

fn start_session(s: AppState) {
    let now = Utc::now();
    {
        let mut g = backend();
        let Backend {
            content,
            state,
            state_store,
            settings,
            ..
        } = &mut *g;
        let engine = Engine::new(content, settings.clone());
        engine.introduce_new(state, now);
        let _ = state_store.save_state(state);
    }
    let q = {
        let g = backend();
        Engine::new(&g.content, g.settings.clone()).due_items(&g.state, now)
    };
    let mut queue = s.queue;
    queue.set(q);
    load_current(s);
}

fn do_grade(rating: Rating, s: AppState) {
    let mut q = (s.queue)();
    if q.is_empty() {
        return;
    }
    let (kid, kind) = q.remove(0);
    {
        let now = Utc::now();
        let mut g = backend();
        let Backend {
            content,
            state,
            state_store,
            settings,
            ..
        } = &mut *g;
        Engine::new(content, settings.clone()).grade(state, kid, kind, &[rating], now);
        let _ = state_store.save_state(state);
    }
    let mut queue = s.queue;
    queue.set(q);
    load_current(s);
}

/// Load the queue's head item (or return to the dashboard when the queue is empty).
fn load_current(s: AppState) {
    let q = (s.queue)();
    let detail = {
        let g = backend();
        q.first()
            .and_then(|(kid, _)| g.content_repo.kanji_detail(*kid).ok())
    };
    let mut current = s.current;
    let mut revealed = s.revealed;
    let mut screen = s.screen;
    revealed.set(false);
    match detail {
        Some(k) => {
            current.set(Some(k));
            screen.set(Screen::Session);
        }
        None => {
            current.set(None);
            screen.set(Screen::Dashboard);
        }
    }
}
