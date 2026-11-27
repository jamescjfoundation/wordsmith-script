// SPDX-License-Identifier: MIT OR Apache-2.0
//! The workspace (SPEC-2 2, 8, 17, 19, decision 0036): a script in each tab, a split view, and
//! writing statistics with goals and a sprint timer.
//!
//! The tab strip and the two panes with their divider are the core's (`jcj_shell::split`); which
//! scripts are open and what the second pane shows are Script's. Statistics belong to the writer,
//! not the script: they are kept beside the preferences (`stats.json`), never in a script.
//!
//! The statistics are the core's (`jcj_shell::writing::stats`): the days, the goals, the counting
//! (words and pages added, never taken away, and writing time while input keeps coming) and the
//! statistics window. What is Script's is what it counts (its word rule and its pages), its
//! starting goals ([`fresh_stats`]), its days and hours in UTC, and the sprint's window.

use std::time::{Duration, Instant, SystemTime};

use egui::{Context, Ui};
use jcj_shell::split::{TabEvent, tabs};
use jcj_shell::writing::stats::{Goals, Metric, Moment, Range, Statistics, StatsView, Tracker, Unit, statistics_window};
use jcj_shell::writing::{Counting, Writing};

use crate::editor::Editor;
use crate::structure::Structure;

/// The sprint timer's default length, minutes (decision 0036).
pub const SPRINT_MINUTES: u32 = jcj_shell::writing::stats::SPRINT_MINUTES;
/// How often the statistics are written while they change.
pub const SAVE_INTERVAL: Duration = Duration::from_secs(60);

/// A second view beside (or below) the main one: what it shows (`None`: the script). The panes
/// and the divider are the core's ([`panes`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Split {
    pub pane: Option<Structure>,
    pub below: bool,
}

/// The statistics of a writer who has none yet: SPEC-2 17's beginner goals, a page and 25
/// minutes a day.
pub fn fresh_stats() -> Statistics {
    Statistics { goals: Goals { pages_per_day: 1.0, days_per_week: 7, minutes_per_day: 25, words_per_day: 0 }, ..Statistics::default() }
}

/// The statistics window as Script opens it: its unit is the page, it shows words by weeks first,
/// and its hours are UTC, as Script counts them.
pub fn stats_view() -> StatsView {
    StatsView { metric: Metric::Words, range: Range::Weeks, zone: "UTC".into(), ..StatsView::new(Unit::Pages) }
}

/// The day and hour writing counts toward: Script's clock (which tests fix), in UTC. `None`
/// before the clock is read.
pub fn now_moment() -> Option<Moment> {
    let secs = crate::now_secs();
    (secs > 0).then(|| Moment::of(SystemTime::UNIX_EPOCH + Duration::from_secs(secs), 0))
}

/// A sprint's result, once its time is up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SprintResult {
    pub minutes: u32,
    /// Words written in it (deleting takes none away).
    pub words: i64,
    /// Pages the script grew by.
    pub pages: i64,
}

/// Statistics, the sprint, and what was counted last.
pub struct Stats {
    /// The writer's statistics (the core's), kept in `stats.json`.
    pub data: Statistics,
    /// Changed since last saved.
    pub dirty: bool,
    /// When the statistics were last written.
    pub saved: Option<Instant>,
    /// Counts writing into [`Stats::data`].
    pub tracker: Tracker,
    /// The statistics window.
    pub view: StatsView,
    /// The session's words, counted upward, for the sprint.
    pub writing: Writing,
    /// The sprint's result, once its time is up, until the writer closes it.
    pub sprint_result: Option<SprintResult>,
    /// The revision last counted; `None`: the next count is the base.
    last_rev: Option<u64>,
    /// The script's words now, and the revision they were counted at (the status bar shows them).
    pub words_now: (u64, usize),
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            data: fresh_stats(),
            dirty: false,
            saved: None,
            tracker: Tracker::new(),
            view: stats_view(),
            writing: Writing::new(0).counting(Counting::UpOnly),
            sprint_result: None,
            last_rev: None,
            words_now: (0, 0),
        }
    }
}

impl Stats {
    /// Forget the last counts (another script came into view), so its words do not count.
    pub fn forget_counts(&mut self) {
        self.last_rev = None;
        self.tracker.forget();
    }
}

/// Words in a script.
pub fn word_count(doc: &jcj_doc::Document) -> usize {
    doc.body.iter().filter_map(|b| b.as_para()).map(|p| p.text.split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count()).sum()
}

/// Count writing, once a frame: words and pages added when the script changed, and the time
/// spent while input keeps coming (the core's [`Tracker`]); the sprint's end.
pub fn track(ed: &mut Editor) {
    let now = Instant::now();
    let rev = ed.session.rev();
    if ed.stats.words_now.0 != rev || rev == 0 {
        ed.stats.words_now = (rev, word_count(&ed.session.doc));
    }
    let moment = now_moment();
    if ed.stats.last_rev != Some(rev) {
        let words = ed.stats.words_now.1;
        let pages = ed.paginated().pages.len();
        let stats = &mut ed.stats;
        // The first count after another script came into view is only the base.
        if stats.last_rev.is_none() {
            stats.writing.rebase(words);
        } else if let Some(m) = moment {
            stats.writing.observe(words, m.day);
        }
        if let Some(m) = moment {
            stats.dirty |= stats.tracker.count(&mut stats.data, words, Some(pages), m, now);
        }
        stats.last_rev = Some(rev);
    }
    if let Some(m) = moment {
        let stats = &mut ed.stats;
        stats.dirty |= stats.tracker.tick(&mut stats.data, m, now);
    }
    // The sprint ends.
    if let Some(sp) = ed.stats.writing.sprint
        && ed.stats.sprint_result.is_none()
        && sp.finished(now)
    {
        let pages = ed.paginated().pages.len();
        let stats = &mut ed.stats;
        let minutes = u32::try_from(sp.length.as_secs() / 60).unwrap_or(u32::MAX);
        stats.sprint_result =
            Some(SprintResult { minutes, words: stats.writing.sprint_words(), pages: stats.writing.sprint_pages(pages).unwrap_or(0) });
        stats.writing.sprint = None;
    }
}

/// The tab bar, when more than one script is open: the core's tab strip (`jcj_shell::split::tabs`)
/// with a button for a new tab at its end. A script with unsaved changes has a dot after its
/// name.
pub fn tab_bar(ed: &mut Editor, ui: &mut Ui) {
    let names = ed.tab_names();
    if names.len() < 2 {
        return;
    }
    let titles: Vec<String> = names.iter().map(|(name, dirty)| if *dirty { format!("{name} •") } else { name.clone() }).collect();
    let mut event = None;
    let mut new = false;
    egui::Panel::top("tabs").frame(egui::Frame::NONE).show(ui, |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(4.0);
            new = ui.small_button("+").on_hover_text("New script in a tab").clicked();
            event = tabs(ui, egui::Id::unique("script-tabs"), &titles, ed.active_tab);
        });
    });
    if let Some(event) = event {
        tab_event(ed, event);
    }
    if new {
        ed.new_tab();
    }
}

/// What a click on the tab strip does: a tab shows its script; its close button closes it.
/// Closing asks about unsaved changes, so that script is shown first; when another script's tab
/// closes at once, the writer stays on the script they were in.
pub fn tab_event(ed: &mut Editor, event: TabEvent) {
    match event {
        TabEvent::Select(i) => ed.switch_tab(i),
        TabEvent::Close(i) => {
            let was = ed.active_tab;
            ed.switch_tab(i);
            if ed.close_tab(false) && was != i {
                ed.switch_tab(if was > i { was - 1 } else { was });
            }
        }
    }
}

/// The main view and, when the view is split, the second pane beside or below it, with the
/// core's divider between them (`jcj_shell::split::split`). Focus mode shows the main view alone.
pub fn panes(ed: &mut Editor, ui: &mut Ui) {
    // Both panes showing the same thing is no split.
    let second = ed.split.filter(|sp| !ed.focus_mode && sp.pane != ed.structure);
    let how = match second {
        None => jcj_shell::split::Split::None,
        Some(sp) if sp.below => jcj_shell::split::Split::Stacked,
        Some(_) => jcj_shell::split::Split::Side,
    };
    let main = ed.structure;
    let mut share = ed.split_share;
    // The two panes are drawn one after the other, each with the editor.
    let both = std::cell::RefCell::new(&mut *ed);
    jcj_shell::split::split(
        ui,
        egui::Id::unique("script-split"),
        how,
        &mut share,
        |ui| {
            if let Ok(mut ed) = both.try_borrow_mut() {
                pane(&mut ed, ui, main, false);
            }
        },
        |ui| {
            if let (Some(sp), Ok(mut ed)) = (second, both.try_borrow_mut()) {
                pane(&mut ed, ui, sp.pane, true);
            }
        },
    );
    ed.split_share = share;
}

/// One pane: a structure view, or the script (`None`).
fn pane(ed: &mut Editor, ui: &mut Ui, what: Option<Structure>, second: bool) {
    match what {
        Some(Structure::Cards) => crate::structure::cards(ed, ui),
        Some(Structure::Scenes) => crate::structure::scene_view(ed, ui),
        Some(Structure::Board) => crate::board::view(ed, ui),
        Some(Structure::Outline) => crate::outline::view(ed, ui),
        None if second => ed.script_pane(ui),
        None => {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| ed.page_view(ui));
        }
    }
}

/// Set the split view: `pane` beside (or below) the main view; the same again unsplits.
pub fn set_split(ed: &mut Editor, what: &str) {
    let pane = |p: &str| match p {
        "cards" => Some(Some(Structure::Cards)),
        "scenes" => Some(Some(Structure::Scenes)),
        "board" => Some(Some(Structure::Board)),
        "outline" => Some(Some(Structure::Outline)),
        "script" => Some(None),
        _ => None,
    };
    match what {
        "off" => ed.split = None,
        "swap" => {
            if let Some(sp) = ed.split.as_mut() {
                std::mem::swap(&mut sp.pane, &mut ed.structure);
            }
        }
        "below" => {
            if let Some(sp) = ed.split.as_mut() {
                sp.below = !sp.below;
            }
        }
        p => {
            if let Some(pane) = pane(p) {
                ed.split = Some(Split { pane, below: ed.split.is_some_and(|s| s.below) });
            }
        }
    }
}

fn sprint_window(ed: &mut Editor, ctx: &Context) {
    if ed.stats.writing.sprint.is_some() {
        ctx.request_repaint_after(Duration::from_secs(1));
    }
    let Some(result) = ed.stats.sprint_result else { return };
    let mut done = false;
    egui::Window::new("Sprint Finished").collapsible(false).resizable(false).show(ctx, |ui| {
        ui.label(format!("{} minutes: {} words and {} pages written.", result.minutes, result.words, result.pages));
        if ui.button("OK").clicked() {
            done = true;
        }
    });
    if done {
        ed.stats.sprint_result = None;
    }
}

/// Start a sprint of the set length, or stop the one under way.
pub fn toggle_sprint(ed: &mut Editor) {
    if ed.stats.writing.sprint.take().is_some() {
        return;
    }
    ed.stats.sprint_result = None;
    let minutes = ed.stats.data.sprint_minutes.clamp(1, jcj_shell::writing::stats::MAX_SPRINT_MINUTES);
    let pages = ed.paginated().pages.len();
    ed.stats.writing.start_sprint_with_pages(Duration::from_secs(u64::from(minutes) * 60), Instant::now(), pages);
}

/// The sprint's time left, for the status bar.
pub fn sprint_left(ed: &Editor) -> Option<Duration> {
    ed.stats.writing.sprint.map(|s| s.remaining(Instant::now()))
}

/// Open the statistics window, showing words by weeks.
pub fn open_stats(ed: &mut Editor) {
    ed.stats.view = StatsView { open: true, ..stats_view() };
}

/// The statistics window (the core's) and the sprint's result.
pub fn show(ed: &mut Editor, ctx: &Context) {
    if let Some(m) = now_moment() {
        let stats = &mut ed.stats;
        stats.dirty |= statistics_window(ctx, &mut stats.data, &mut stats.view, m.day);
    }
    sprint_window(ed, ctx);
}

/// Write the statistics into the settings folder `dir` when they changed: at most once every
/// [`SAVE_INTERVAL`], or now with `force`.
pub fn save_stats(ed: &mut Editor, dir: &std::path::Path, force: bool) -> Result<(), String> {
    let stats = &mut ed.stats;
    if !stats.dirty || (!force && stats.saved.is_some_and(|t| t.elapsed() < SAVE_INTERVAL)) {
        return Ok(());
    }
    stats.data.save(dir)?;
    stats.dirty = false;
    stats.saved = Some(Instant::now());
    Ok(())
}
