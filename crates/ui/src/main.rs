//! MnemoKanji — desktop app (Dioxus, system WebView).
//!
//! M3 slice 1: load the real N5 seed + user state, show a dashboard, and run the recognition
//! review loop (kanji → meaning) end-to-end through the core engine, persisting every grade.
//! Other modes, the kanji-detail hub, and the anti-burnout polish land in later M3 slices.

use std::sync::{Mutex, OnceLock};

use chrono::Utc;
use dioxus::prelude::*;
use mnemokanji_core::{ContentView, Engine, Rating, Settings, StudyState, TrackKind};
use mnemokanji_data::{ContentRepo, KanjiDetail, StateStore};

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

    let content_repo = ContentRepo::open(&seed)
        .unwrap_or_else(|e| panic!("open seed db {seed}: {e} (run scripts/fetch-sources.sh + cargo run -p mnemokanji-content)"));
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

#[component]
fn App() -> Element {
    let screen = use_signal(|| Screen::Dashboard);
    let dash = use_signal(|| compute_dash(&backend()));
    let queue = use_signal(Vec::<(i64, TrackKind)>::new);
    let current = use_signal(|| None::<KanjiDetail>);
    let revealed = use_signal(|| false);

    rsx! {
        style { dangerous_inner_html: CSS }
        div { class: "app",
            {match screen() {
                Screen::Dashboard => dashboard_view(screen, dash, queue, current, revealed),
                Screen::Session => session_view(screen, dash, queue, current, revealed),
            }}
        }
    }
}

fn dashboard_view(
    screen: Signal<Screen>,
    dash: Signal<Dash>,
    queue: Signal<Vec<(i64, TrackKind)>>,
    current: Signal<Option<KanjiDetail>>,
    revealed: Signal<bool>,
) -> Element {
    let d = dash();
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
                button {
                    class: "primary",
                    onclick: move |_| start_session(screen, dash, queue, current, revealed),
                    "Start session"
                }
            }
        }
    }
}

fn session_view(
    screen: Signal<Screen>,
    dash: Signal<Dash>,
    queue: Signal<Vec<(i64, TrackKind)>>,
    current: Signal<Option<KanjiDetail>>,
    revealed: Signal<bool>,
) -> Element {
    let Some(k) = current() else {
        return rsx! { div { class: "card", p { "Loading\u{2026}" } } };
    };
    let glyph = k.glyph.clone();
    let keyword = k.keyword.clone();
    let left = queue().len();
    let is_revealed = revealed();
    let reading_str = k
        .readings
        .iter()
        .filter(|r| r.is_dominant)
        .map(|r| r.reading.clone())
        .collect::<Vec<_>>()
        .join("   ");
    let all_readings = k
        .readings
        .iter()
        .map(|r| format!("{}:{}", r.kind, r.reading))
        .collect::<Vec<_>>()
        .join("   ");

    rsx! {
        div { class: "review",
            div { class: "topbar",
                button { class: "link", onclick: move |_| { let mut s = screen; s.set(Screen::Dashboard); }, "\u{2190} end" }
                span { class: "left", "{left} left" }
            }
            // Recognition prompt: a clean glyph only (modality segregation).
            div { class: "glyph", "{glyph}" }
            if !is_revealed {
                button {
                    class: "primary reveal",
                    onclick: move |_| { let mut r = revealed; r.set(true); },
                    "Show answer"
                }
            } else {
                div { class: "answer",
                    div { class: "keyword", "{keyword}" }
                    if !reading_str.is_empty() {
                        div { class: "reading", "{reading_str}" }
                    }
                    div { class: "all-readings", "{all_readings}" }
                    div { class: "grades",
                        button { class: "grade again", onclick: move |_| do_grade(Rating::Again, screen, dash, queue, current, revealed), "Again" }
                        button { class: "grade hard", onclick: move |_| do_grade(Rating::Hard, screen, dash, queue, current, revealed), "Hard" }
                        button { class: "grade good", onclick: move |_| do_grade(Rating::Good, screen, dash, queue, current, revealed), "Good" }
                        button { class: "grade easy", onclick: move |_| do_grade(Rating::Easy, screen, dash, queue, current, revealed), "Easy" }
                    }
                }
            }
        }
    }
}

fn start_session(
    screen: Signal<Screen>,
    dash: Signal<Dash>,
    queue: Signal<Vec<(i64, TrackKind)>>,
    current: Signal<Option<KanjiDetail>>,
    revealed: Signal<bool>,
) {
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
    let mut queue = queue;
    queue.set(q);
    load_current(screen, dash, queue, current, revealed);
}

fn do_grade(
    rating: Rating,
    screen: Signal<Screen>,
    dash: Signal<Dash>,
    queue: Signal<Vec<(i64, TrackKind)>>,
    current: Signal<Option<KanjiDetail>>,
    revealed: Signal<bool>,
) {
    let mut q = queue();
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
    let mut queue = queue;
    queue.set(q);
    load_current(screen, dash, queue, current, revealed);
}

/// Refresh the dashboard counts and load the queue's head item (or return to the dashboard).
fn load_current(
    mut screen: Signal<Screen>,
    mut dash: Signal<Dash>,
    queue: Signal<Vec<(i64, TrackKind)>>,
    mut current: Signal<Option<KanjiDetail>>,
    mut revealed: Signal<bool>,
) {
    let q = queue();
    let (d, detail) = {
        let g = backend();
        let d = compute_dash(&g);
        let detail = q
            .first()
            .and_then(|(kid, _)| g.content_repo.kanji_detail(*kid).ok());
        (d, detail)
    };
    dash.set(d);
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
