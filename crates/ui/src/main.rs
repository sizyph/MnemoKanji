//! MnemoKanji — desktop app (Dioxus, system WebView).
//!
//! M3: dashboard + the four review modes wired to the core engine + seed, persisting every grade.
//! Comprehension track → recognition (kanji→meaning) + reading-in-context. Production track →
//! write (meaning→kanji, revealing animated strokes) + cloze. Plus the kanji-detail hub (with
//! editable mnemonics), a browse grid, a settings screen, one-tap undo, and a dev clock-skip so
//! the production track (active only after comprehension matures) is reachable without waiting.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Duration, Utc};
use dioxus::prelude::*;
use mnemokanji_core::{
    mastery, streak, ContentView, Engine, Mastery, Rating, Settings, StudyState, TrackKind,
};
use mnemokanji_data::{BrowseItem, ContentRepo, KanjiDetail, StateStore};

#[cfg(not(feature = "bundle-seed"))]
const SEED_DB: &str = "assets/seed.sqlite";
#[cfg(not(feature = "bundle-seed"))]
const USER_DB: &str = "user.sqlite";
const CSS: &str = include_str!("app.css");

/// Process-wide source of truth: the read-only seed, the writable user state, and settings.
struct Backend {
    content_repo: ContentRepo,
    content: ContentView,
    state_store: StateStore,
    state: StudyState,
    settings: Settings,
    /// Path to the writable user-state DB (for backup/restore).
    user_path: String,
    /// Dev-only simulated-time offset in days (lets the production track activate without waiting).
    clock_offset_days: i64,
}

impl Backend {
    fn now(&self) -> DateTime<Utc> {
        Utc::now() + Duration::days(self.clock_offset_days)
    }
}

static BACKEND: OnceLock<Mutex<Backend>> = OnceLock::new();

fn backend() -> std::sync::MutexGuard<'static, Backend> {
    BACKEND
        .get()
        .expect("backend initialized in main")
        .lock()
        .expect("backend lock")
}

/// Embedded N5 seed for release builds (extracted to the app-data dir at startup).
#[cfg(feature = "bundle-seed")]
const SEED_BYTES: &[u8] = include_bytes!("../../../assets/seed.sqlite");

#[cfg(feature = "bundle-seed")]
fn data_dir() -> std::path::PathBuf {
    let dir = dirs::data_dir()
        .unwrap_or_else(|| ".".into())
        .join("MnemoKanji");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Resolve the read-only seed DB: `$MNEMOKANJI_SEED`, else the embedded seed (release), else the
/// dev path `assets/seed.sqlite` relative to the working directory.
fn seed_path() -> String {
    if let Ok(p) = std::env::var("MNEMOKANJI_SEED") {
        return p;
    }
    #[cfg(feature = "bundle-seed")]
    {
        let path = data_dir().join("seed.sqlite");
        std::fs::write(&path, SEED_BYTES).expect("write bundled seed");
        path.to_string_lossy().into_owned()
    }
    #[cfg(not(feature = "bundle-seed"))]
    {
        SEED_DB.to_string()
    }
}

/// Resolve the writable user-state DB: `$MNEMOKANJI_USER_DB`, else the app-data dir (release), else
/// `user.sqlite` in the working directory (dev).
fn user_path() -> String {
    if let Ok(p) = std::env::var("MNEMOKANJI_USER_DB") {
        return p;
    }
    #[cfg(feature = "bundle-seed")]
    {
        data_dir()
            .join("user.sqlite")
            .to_string_lossy()
            .into_owned()
    }
    #[cfg(not(feature = "bundle-seed"))]
    {
        USER_DB.to_string()
    }
}

fn main() {
    let seed = seed_path();
    let user = user_path();

    let content_repo = ContentRepo::open(&seed).unwrap_or_else(|e| {
        panic!("open seed db {seed}: {e} (run scripts/fetch-sources.sh + cargo run -p mnemokanji-content)")
    });
    let content = content_repo.content_view().expect("load content view");
    let state_store = StateStore::open(&user).expect("open user state db");
    let state = state_store.load_state().expect("load user state");
    let (new_per_day, daily_review_cap) = state_store.load_settings().unwrap_or((10, 60));

    BACKEND
        .set(Mutex::new(Backend {
            content_repo,
            content,
            state_store,
            state,
            settings: Settings {
                new_per_day,
                daily_review_cap,
                ..Default::default()
            },
            user_path: user,
            clock_offset_days: 0,
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
    Settings,
    Stats,
}

#[derive(Clone, PartialEq)]
struct Dash {
    level: u8,
    total: usize,
    introduced: usize,
    due: usize,
    new_remaining: usize,
    offset_days: i64,
    streak: u32,
    reviews_today: usize,
}

fn compute_dash(b: &Backend) -> Dash {
    let now = b.now();
    let today = now.date_naive();
    let engine = Engine::new(&b.content, b.settings.clone());
    let level = b.state.unlocked_level;
    let dates = b.state_store.study_dates().unwrap_or_default();
    let (streak_days, _) = streak(&dates, today);
    let (reviews_today, _) = b.state_store.review_counts(today).unwrap_or((0, 0));
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
        offset_days: b.clock_offset_days,
        streak: streak_days,
        reviews_today,
    }
}

/// Snapshot for one-tap undo: the whole study state before a grade, plus the graded item.
#[derive(Clone)]
struct UndoSnapshot {
    state: StudyState,
    item: (i64, TrackKind),
}

#[derive(Clone, Copy)]
struct AppState {
    screen: Signal<Screen>,
    queue: Signal<Vec<(i64, TrackKind)>>,
    current: Signal<Option<(KanjiDetail, TrackKind)>>,
    revealed: Signal<bool>,
    detail: Signal<Option<KanjiDetail>>,
    browse: Signal<Vec<BrowseItem>>,
    undo: Signal<Option<UndoSnapshot>>,
    editing: Signal<bool>,
    edit_text: Signal<String>,
    tick: Signal<u32>,
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
        undo: use_signal(|| None),
        editing: use_signal(|| false),
        edit_text: use_signal(String::new),
        tick: use_signal(|| 0u32),
    };

    rsx! {
        style { dangerous_inner_html: CSS }
        div { class: "app",
            {match (s.screen)() {
                Screen::Dashboard => dashboard_view(s),
                Screen::Session => session_view(s),
                Screen::Browse => browse_view(s),
                Screen::Detail => detail_view(s),
                Screen::Settings => settings_view(s),
                Screen::Stats => stats_view(s),
            }}
        }
    }
}

fn dashboard_view(s: AppState) -> Element {
    let _ = (s.tick)(); // subscribe so dev skip-day / settings changes refresh the counts
    let d = compute_dash(&backend());
    let level = d.level;
    let due = d.due;
    let new_remaining = d.new_remaining;
    let learned = format!("{}/{}", d.introduced, d.total);
    let nothing = due == 0 && new_remaining == 0;
    let offset = d.offset_days;
    let streak = d.streak;
    let reviews_today = d.reviews_today;

    rsx! {
        div { class: "card",
            h1 { "MnemoKanji" }
            p { class: "sub", "JLPT N{level}" }
            if streak > 0 {
                div { class: "streak", "\u{1f525} {streak}-day streak \u{00b7} {reviews_today} today" }
            }
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
            div { class: "nav-row",
                button { class: "secondary", onclick: move |_| { let mut sc = s.screen; sc.set(Screen::Stats); }, "Progress" }
                button { class: "secondary", onclick: move |_| show_browse(s), "Browse" }
                button { class: "secondary", onclick: move |_| { let mut sc = s.screen; sc.set(Screen::Settings); }, "Settings" }
            }
            div { class: "devbar",
                span { "dev clock: +{offset}d" }
                button { class: "devbtn", onclick: move |_| skip_day(s), "skip 1 day \u{23e9}" }
            }
        }
    }
}

fn session_view(s: AppState) -> Element {
    let Some((k, kind)) = (s.current)() else {
        return rsx! { div { class: "card", p { "Loading\u{2026}" } } };
    };
    let left = (s.queue)().len();
    let is_revealed = (s.revealed)();
    let can_undo = (s.undo)().is_some();
    let mode_label = match kind {
        TrackKind::Comprehension => "recognise",
        TrackKind::Production => "write",
    };

    rsx! {
        div { class: "review",
            div { class: "topbar",
                button { class: "link", onclick: move |_| { let mut sc = s.screen; sc.set(Screen::Dashboard); }, "\u{2190} end" }
                span { class: "mode-tag", "{mode_label}" }
                span { class: "topbar-right",
                    if can_undo {
                        button { class: "link", onclick: move |_| undo_last(s), "\u{21b6} undo" }
                    }
                    span { class: "left", "{left} left" }
                }
            }
            {match kind {
                TrackKind::Comprehension => comprehension_body(s, &k, is_revealed),
                TrackKind::Production => production_body(s, &k, is_revealed),
            }}
        }
    }
}

/// Recognition (kanji → meaning) + reading shown in context. Prompt is a clean glyph.
fn comprehension_body(s: AppState, k: &KanjiDetail, revealed: bool) -> Element {
    let glyph = k.glyph.clone();
    let keyword = k.keyword.clone();
    let reading_str = dominant_reading(k);
    let examples: Vec<String> = k
        .vocab
        .iter()
        .take(2)
        .map(|v| format!("{} ({}) \u{2014} {}", v.surface, v.reading, v.gloss))
        .collect();
    let mnemonic = k.mnemonic.clone();

    rsx! {
        div { class: "glyph", "{glyph}" }
        if !revealed {
            button { class: "primary reveal", onclick: move |_| { let mut r = s.revealed; r.set(true); }, "Show answer" }
        } else {
            div { class: "answer",
                div { class: "keyword", "{keyword}" }
                if !reading_str.is_empty() { div { class: "reading", "{reading_str}" } }
                div { class: "examples", for ex in examples.iter() { div { class: "example", "{ex}" } } }
                if let Some(m) = mnemonic { div { class: "mnemonic", "{m}" } }
                {grade_buttons(s)}
            }
        }
    }
}

/// Production (meaning → write the kanji) + cloze. Prompt is the keyword; reveal shows the glyph
/// being drawn (animated strokes) and the filled-in example sentence.
fn production_body(s: AppState, k: &KanjiDetail, revealed: bool) -> Element {
    let keyword = k.keyword.clone();
    let reading_str = dominant_reading(k);
    let (cloze_q, cloze_a, cloze_en) = cloze(k);

    rsx! {
        div { class: "write-prompt",
            div { class: "write-kw", "{keyword}" }
            div { class: "write-hint", "write the kanji" }
            if let Some(q) = cloze_q.clone() { div { class: "cloze", "{q}" } }
        }
        if !revealed {
            button { class: "primary reveal", onclick: move |_| { let mut r = s.revealed; r.set(true); }, "Reveal" }
        } else {
            div { class: "answer",
                {stroke_svg(&k.stroke_paths)}
                div { class: "keyword", "{k.glyph}" }
                if !reading_str.is_empty() { div { class: "reading", "{reading_str}" } }
                if let Some(a) = cloze_a {
                    div { class: "example", "{a}" }
                    if let Some(en) = cloze_en { div { class: "sen", "{en}" } }
                }
                {grade_buttons(s)}
            }
        }
    }
}

fn grade_buttons(s: AppState) -> Element {
    rsx! {
        div { class: "grades",
            button { class: "grade again", onclick: move |_| do_grade(Rating::Again, s), "Again" }
            button { class: "grade hard", onclick: move |_| do_grade(Rating::Hard, s), "Hard" }
            button { class: "grade good", onclick: move |_| do_grade(Rating::Good, s), "Good" }
            button { class: "grade easy", onclick: move |_| do_grade(Rating::Easy, s), "Easy" }
        }
    }
}

fn dominant_reading(k: &KanjiDetail) -> String {
    k.readings
        .iter()
        .filter(|r| r.is_dominant)
        .map(|r| r.reading.clone())
        .collect::<Vec<_>>()
        .join("   ")
}

/// Build a cloze: a sentence with the first vocab word it contains blanked out.
/// Returns (blanked, full, english).
fn cloze(k: &KanjiDetail) -> (Option<String>, Option<String>, Option<String>) {
    for s in &k.sentences {
        if let Some(v) = k.vocab.iter().find(|v| s.jp.contains(&v.surface)) {
            return (
                Some(s.jp.replace(&v.surface, "\u{3000}____\u{3000}")),
                Some(s.jp.clone()),
                Some(s.en.clone()),
            );
        }
    }
    match k.sentences.first() {
        Some(s) => (Some(s.jp.clone()), Some(s.jp.clone()), Some(s.en.clone())),
        None => (None, None, None),
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
    let kid = k.id;
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
    let mnemonic = k.mnemonic.clone().unwrap_or_default();
    let mnemonic_for_edit = mnemonic.clone();
    let editing = (s.editing)();
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
                        for (t, dom) in on.iter() { span { class: if *dom { "reading-chip dom" } else { "reading-chip" }, "{t}" } }
                    } }
                    div { class: "row", span { class: "rk", "kun" } span { class: "rv",
                        for (t, dom) in kun.iter() { span { class: if *dom { "reading-chip dom" } else { "reading-chip" }, "{t}" } }
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
            div { class: "section",
                div { class: "mnemo-head",
                    h3 { "Mnemonic" }
                    if !editing {
                        button { class: "tiny-link", onclick: move |_| start_edit(s, mnemonic_for_edit.clone()), "edit" }
                    }
                }
                if editing {
                    textarea {
                        class: "edit-area",
                        value: "{s.edit_text}",
                        oninput: move |e| { let mut t = s.edit_text; t.set(e.value()); }
                    }
                    div { class: "edit-actions",
                        button { class: "secondary", onclick: move |_| save_mnemonic(s, kid), "Save" }
                        button { class: "link", onclick: move |_| { let mut ed = s.editing; ed.set(false); }, "Cancel" }
                    }
                } else if mnemonic.is_empty() {
                    p { class: "muted", "\u{2014}" }
                } else {
                    p { class: "mnemonic-text", "{mnemonic}" }
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
                    div { class: "sentence", div { class: "sjp", "{jp}" } div { class: "sen", "{en}" } }
                }
            }
        }
    }
}

fn settings_view(s: AppState) -> Element {
    let _ = (s.tick)();
    let (npd, cap) = {
        let g = backend();
        (g.settings.new_per_day, g.settings.daily_review_cap)
    };
    rsx! {
        div { class: "card",
            div { class: "topbar",
                button { class: "link", onclick: move |_| { let mut sc = s.screen; sc.set(Screen::Dashboard); }, "\u{2190} back" }
                span { class: "title", "Settings" }
            }
            div { class: "setting-row",
                label { "New kanji / day" }
                div { class: "stepper",
                    button { onclick: move |_| change_setting(s, -1, 0), "\u{2212}" }
                    span { "{npd}" }
                    button { onclick: move |_| change_setting(s, 1, 0), "+" }
                }
            }
            div { class: "setting-row",
                label { "Daily review cap" }
                div { class: "stepper",
                    button { onclick: move |_| change_setting(s, 0, -10), "\u{2212}" }
                    span { "{cap}" }
                    button { onclick: move |_| change_setting(s, 0, 10), "+" }
                }
            }
            div { class: "data-section",
                h3 { "Data" }
                div { class: "data-row",
                    button { class: "secondary", onclick: move |_| export_data(s), "Export backup" }
                    button { class: "secondary", onclick: move |_| import_data(s), "Import backup" }
                }
                p { class: "setting-note", "Back up or restore all your progress (a .sqlite file). Import replaces current progress." }
            }
            p { class: "setting-note", "Settings changes are saved and apply to the next session." }
        }
    }
}

fn stats_view(s: AppState) -> Element {
    let _ = (s.tick)();
    let (counts, total, cur, longest, today_n, total_n, level) = {
        let g = backend();
        let today = g.now().date_naive();
        let mut counts = [0usize; 4]; // new, learning, young, mature
        for ((_, k), t) in &g.state.tracks {
            if *k != TrackKind::Comprehension {
                continue;
            }
            let idx = match mastery(&t.card, g.settings.mature_stability_days) {
                Mastery::New => 0,
                Mastery::Learning => 1,
                Mastery::Young => 2,
                Mastery::Mature => 3,
            };
            counts[idx] += 1;
        }
        let total = g
            .content
            .kanji
            .iter()
            .filter(|kk| kk.level == g.state.unlocked_level)
            .count();
        let dates = g.state_store.study_dates().unwrap_or_default();
        let (cur, longest) = streak(&dates, today);
        let (today_n, total_n) = g.state_store.review_counts(today).unwrap_or((0, 0));
        (
            counts,
            total,
            cur,
            longest,
            today_n,
            total_n,
            g.state.unlocked_level,
        )
    };
    let [new_c, learn_c, young_c, mature_c] = counts;
    let mastered_pct = (mature_c * 100).checked_div(total).unwrap_or(0);
    let (bn, bl, by, bm) = (
        bar(new_c, total),
        bar(learn_c, total),
        bar(young_c, total),
        bar(mature_c, total),
    );

    rsx! {
        div { class: "card",
            div { class: "topbar",
                button { class: "link", onclick: move |_| { let mut sc = s.screen; sc.set(Screen::Dashboard); }, "\u{2190} back" }
                span { class: "title", "Progress \u{00b7} N{level}" }
            }
            div { class: "big-streak",
                div { class: "bs-num", "\u{1f525} {cur}" }
                div { class: "bs-lbl", "day streak \u{00b7} best {longest}" }
            }
            div { class: "mastery",
                div { class: "m-row", span { class: "m-lbl", "new" }, div { class: "m-bar", div { class: "m-fill new", style: "width:{bn}%" } }, span { class: "m-n", "{new_c}" } }
                div { class: "m-row", span { class: "m-lbl", "learning" }, div { class: "m-bar", div { class: "m-fill learning", style: "width:{bl}%" } }, span { class: "m-n", "{learn_c}" } }
                div { class: "m-row", span { class: "m-lbl", "young" }, div { class: "m-bar", div { class: "m-fill young", style: "width:{by}%" } }, span { class: "m-n", "{young_c}" } }
                div { class: "m-row", span { class: "m-lbl", "mature" }, div { class: "m-bar", div { class: "m-fill mature", style: "width:{bm}%" } }, span { class: "m-n", "{mature_c}" } }
            }
            p { class: "mastered-line", "{mature_c}/{total} mastered ({mastered_pct}%)" }
            div { class: "stats",
                div { class: "stat", div { class: "num", "{today_n}" } div { class: "lbl", "today" } }
                div { class: "stat", div { class: "num", "{total_n}" } div { class: "lbl", "reviews" } }
            }
        }
    }
}

fn bar(n: usize, total: usize) -> usize {
    (n * 100).checked_div(total).unwrap_or(0)
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

/// Load a kanji's detail and apply the user's mnemonic override, if any.
fn load_detail(id: i64) -> Option<KanjiDetail> {
    let g = backend();
    let mut d = g.content_repo.kanji_detail(id).ok()?;
    if let Some(user) = g.state_store.user_mnemonic(id) {
        d.mnemonic = Some(user);
    }
    Some(d)
}

fn skip_day(s: AppState) {
    {
        let mut g = backend();
        g.clock_offset_days += 1;
    }
    let mut tick = s.tick;
    tick += 1;
}

fn change_setting(s: AppState, d_npd: i64, d_cap: i64) {
    {
        let mut g = backend();
        g.settings.new_per_day = (g.settings.new_per_day as i64 + d_npd).clamp(1, 100) as usize;
        g.settings.daily_review_cap =
            (g.settings.daily_review_cap as i64 + d_cap).clamp(10, 500) as usize;
        let (n, c) = (g.settings.new_per_day, g.settings.daily_review_cap);
        let _ = g.state_store.save_settings(n, c);
    }
    let mut tick = s.tick;
    tick += 1;
}

/// Back up the user-state DB to a file the user chooses.
fn export_data(_s: AppState) {
    let src = backend().user_path.clone();
    if let Some(dest) = rfd::FileDialog::new()
        .set_file_name("mnemokanji-backup.sqlite")
        .add_filter("MnemoKanji backup", &["sqlite"])
        .save_file()
    {
        let _ = std::fs::copy(&src, dest);
    }
}

/// Restore the user-state DB from a backup file, replacing current progress.
fn import_data(s: AppState) {
    let Some(picked) = rfd::FileDialog::new()
        .add_filter("MnemoKanji backup", &["sqlite"])
        .pick_file()
    else {
        return;
    };
    let src = picked.to_string_lossy().into_owned();

    // Validate: a copy must open cleanly as a MnemoKanji state DB.
    let check = std::env::temp_dir().join("mnemokanji-import-check.sqlite");
    let valid =
        std::fs::copy(&src, &check).is_ok() && StateStore::open(&check.to_string_lossy()).is_ok();
    let _ = std::fs::remove_file(&check);
    if !valid {
        return;
    }

    {
        let mut g = backend();
        let dest = g.user_path.clone();
        // Release the lock on the destination file, copy the backup in, then reopen it.
        if let Ok(mem) = StateStore::open(":memory:") {
            g.state_store = mem;
        }
        let _ = std::fs::copy(&src, &dest);
        if let Ok(store) = StateStore::open(&dest) {
            g.state = store.load_state().unwrap_or_default();
            let (npd, cap) = store.load_settings().unwrap_or((10, 60));
            g.settings.new_per_day = npd;
            g.settings.daily_review_cap = cap;
            g.state_store = store;
        }
    }

    // Reset to a fresh dashboard.
    let mut screen = s.screen;
    let mut queue = s.queue;
    let mut undo = s.undo;
    let mut current = s.current;
    let mut tick = s.tick;
    queue.set(Vec::new());
    undo.set(None);
    current.set(None);
    tick += 1;
    screen.set(Screen::Dashboard);
}

fn start_edit(s: AppState, story: String) {
    let mut edit_text = s.edit_text;
    let mut editing = s.editing;
    edit_text.set(story);
    editing.set(true);
}

fn save_mnemonic(s: AppState, kid: i64) {
    let text = (s.edit_text)();
    {
        let mut g = backend();
        let _ = g.state_store.set_user_mnemonic(kid, &text);
    }
    let d = load_detail(kid);
    let mut detail = s.detail;
    let mut editing = s.editing;
    detail.set(d);
    editing.set(false);
}

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
    let d = load_detail(id);
    let mut detail = s.detail;
    let mut editing = s.editing;
    let mut screen = s.screen;
    editing.set(false);
    detail.set(d);
    screen.set(Screen::Detail);
}

fn start_session(s: AppState) {
    let q = {
        let mut g = backend();
        let now = g.now();
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
        engine.due_items(state, now)
    };
    let mut queue = s.queue;
    let mut undo = s.undo;
    queue.set(q);
    undo.set(None);
    load_current(s);
}

fn do_grade(rating: Rating, s: AppState) {
    let mut q = (s.queue)();
    if q.is_empty() {
        return;
    }
    let (kid, kind) = q.remove(0);
    let snapshot = {
        let mut g = backend();
        let now = g.now();
        let snap = g.state.clone();
        let Backend {
            content,
            state,
            state_store,
            settings,
            ..
        } = &mut *g;
        Engine::new(content, settings.clone()).grade(state, kid, kind, &[rating], now);
        let _ = state_store.save_state(state);
        let _ = state_store.log_review(kid, kind.as_str(), rating as u8, now);
        snap
    };
    let mut undo = s.undo;
    let mut queue = s.queue;
    undo.set(Some(UndoSnapshot {
        state: snapshot,
        item: (kid, kind),
    }));
    queue.set(q);
    load_current(s);
}

fn undo_last(s: AppState) {
    let Some(snap) = (s.undo)() else {
        return;
    };
    {
        let mut g = backend();
        g.state = snap.state;
        let Backend {
            state, state_store, ..
        } = &mut *g;
        let _ = state_store.save_state(state);
    }
    let mut q = (s.queue)();
    q.insert(0, snap.item);
    let mut queue = s.queue;
    let mut undo = s.undo;
    queue.set(q);
    undo.set(None);
    load_current(s);
}

/// Load the queue's head item (or return to the dashboard when the queue is empty).
fn load_current(s: AppState) {
    let q = (s.queue)();
    let loaded = q
        .first()
        .and_then(|(kid, kind)| load_detail(*kid).map(|d| (d, *kind)));
    let mut current = s.current;
    let mut revealed = s.revealed;
    let mut screen = s.screen;
    revealed.set(false);
    match loaded {
        Some(pair) => {
            current.set(Some(pair));
            screen.set(Screen::Session);
        }
        None => {
            current.set(None);
            screen.set(Screen::Dashboard);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mnemokanji_data::{SentenceItem, VocabItem};

    fn detail(vocab: &[(&str, &str, &str)], sentences: &[(&str, &str)]) -> KanjiDetail {
        KanjiDetail {
            id: 1,
            glyph: "\u{5b66}".into(),
            keyword: "study".into(),
            stroke_count: Some(8),
            meanings: vec![],
            readings: vec![],
            vocab: vocab
                .iter()
                .map(|(s, r, g)| VocabItem {
                    surface: (*s).into(),
                    reading: (*r).into(),
                    gloss: (*g).into(),
                })
                .collect(),
            sentences: sentences
                .iter()
                .map(|(jp, en)| SentenceItem {
                    jp: (*jp).into(),
                    en: (*en).into(),
                })
                .collect(),
            mnemonic: None,
            stroke_paths: vec![],
            components: vec![],
        }
    }

    #[test]
    fn cloze_blanks_the_matching_vocab_word() {
        let k = detail(
            &[(
                "\u{5b66}\u{6821}",
                "\u{304c}\u{3063}\u{3053}\u{3046}",
                "school",
            )],
            &[("\u{3053}\u{306e}\u{5b66}\u{6821}\u{3002}", "This school.")],
        );
        let (q, a, _) = cloze(&k);
        assert!(q.unwrap().contains("____"), "question should be blanked");
        assert!(
            !a.unwrap().contains("____"),
            "answer should be the full sentence"
        );
    }

    #[test]
    fn cloze_falls_back_to_full_sentence_when_no_vocab_matches() {
        let k = detail(
            &[("\u{72ac}", "\u{3044}\u{306c}", "dog")],
            &[("\u{732b}\u{3002}", "Cat.")],
        );
        let (q, _, _) = cloze(&k);
        assert_eq!(q.unwrap(), "\u{732b}\u{3002}");
    }
}
