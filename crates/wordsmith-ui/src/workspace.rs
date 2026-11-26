// SPDX-License-Identifier: MIT OR Apache-2.0
//! The workspace (SPEC-2 2, 8, 17, 19, decision 0036): a script in each tab, a split view, and
//! writing statistics with goals and a sprint timer.
//!
//! The tab strip and the two panes with their divider are the core's (`jcj_shell::split`); which
//! scripts are open and what the second pane shows are Script's. Statistics belong to the writer,
//! not the script: they are kept beside the preferences (`stats.json`), never in a script.
//!
// core-candidate: open-source-core#41. The statistics below (words, pages and writing time by
// day and hour, counted only upward, goals in pages and minutes, the statistics window, a
// sprint's pages) are general. The core's `jcj_shell::writing` keeps net words per day, a
// session target and a sprint; Script moves onto it once it holds these and reads `stats.json`.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use egui::{Color32, Context, RichText, Stroke, Ui, pos2, vec2};
use jcj_shell::split::{TabEvent, tabs};
use serde::{Deserialize, Serialize};
use wordsmith_script::dates::day_number;

use crate::editor::Editor;
use crate::structure::Structure;

/// Most days of statistics kept (about ten years).
pub const MAX_DAYS: usize = 3_700;
/// The sprint timer's default length, minutes (decision 0036).
pub const SPRINT_MINUTES: u32 = 25;

/// A second view beside (or below) the main one: what it shows (`None`: the script). The panes
/// and the divider are the core's ([`panes`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Split {
    pub pane: Option<Structure>,
    pub below: bool,
}

/// One day's writing.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Day {
    pub words: u64,
    /// Pages added (whole pages, as the script grew).
    pub pages: u64,
    /// Seconds spent writing.
    pub seconds: u64,
    /// Seconds by hour of the day (UTC).
    pub hours: Vec<u64>,
}

/// Goals: pages on so many days a week, and minutes a day.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Goals {
    pub pages_per_day: f32,
    /// Days a week the page goal is for.
    pub days_per_week: u32,
    pub minutes_per_day: u32,
}

impl Default for Goals {
    fn default() -> Self {
        // SPEC-2 17's beginner suggestion: a page and 25 minutes a day.
        Goals { pages_per_day: 1.0, days_per_week: 7, minutes_per_day: 25 }
    }
}

/// The writer's statistics, kept between runs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StatsData {
    pub enabled: bool,
    /// Writing time stops counting after this many minutes without input.
    pub idle_minutes: u32,
    pub sprint_minutes: u32,
    pub goals: Goals,
    /// By date, `YYYY-MM-DD` (UTC).
    pub days: BTreeMap<String, Day>,
}

impl Default for StatsData {
    fn default() -> Self {
        StatsData { enabled: true, idle_minutes: 3, sprint_minutes: SPRINT_MINUTES, goals: Goals::default(), days: BTreeMap::new() }
    }
}

impl StatsData {
    /// Bring values read from a file within bounds.
    pub fn repaired(mut self) -> Self {
        self.idle_minutes = self.idle_minutes.clamp(1, 120);
        self.sprint_minutes = self.sprint_minutes.clamp(1, 240);
        self.goals.pages_per_day = if self.goals.pages_per_day.is_finite() { self.goals.pages_per_day.clamp(0.0, 100.0) } else { 1.0 };
        self.goals.days_per_week = self.goals.days_per_week.clamp(1, 7);
        self.goals.minutes_per_day = self.goals.minutes_per_day.min(24 * 60);
        while self.days.len() > MAX_DAYS {
            let Some(first) = self.days.keys().next().cloned() else { break };
            self.days.remove(&first);
        }
        for d in self.days.values_mut() {
            d.hours.resize(24, 0);
        }
        self
    }
}

/// A sprint under way, or just ended.
#[derive(Clone, Debug, PartialEq)]
pub struct Sprint {
    pub ends: Instant,
    pub minutes: u32,
    pub start_words: usize,
    pub start_pages: usize,
    /// Set when the time is up: words and pages written.
    pub result: Option<(i64, i64)>,
}

/// Statistics, the sprint, and what was counted last.
#[derive(Default)]
pub struct Stats {
    pub data: StatsData,
    /// Changed since last saved.
    pub dirty: bool,
    pub sprint: Option<Sprint>,
    /// The statistics window: metric ("words", "pages", "time") and range ("day", "week",
    /// "month", "year").
    pub window: Option<(&'static str, &'static str)>,
    last_counts: Option<(u64, usize, usize)>,
    last_tick: Option<Instant>,
    last_input: Option<Instant>,
    /// Writing time not yet a whole second, milliseconds.
    carry_ms: u64,
    /// The script's words now, and the revision they were counted at (the status bar shows them).
    pub words_now: (u64, usize),
}

impl Stats {
    /// Forget the last counts (another script came into view), so its words do not count.
    pub fn forget_counts(&mut self) {
        self.last_counts = None;
    }
}

/// Words in a script.
pub fn word_count(doc: &jcj_doc::Document) -> usize {
    doc.body.iter().filter_map(|b| b.as_para()).map(|p| p.text.split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count()).sum()
}

/// Count writing: words and pages added and time spent, once a frame.
pub fn track(ed: &mut Editor) {
    let now = Instant::now();
    let since = ed.stats.last_tick.map_or(Duration::ZERO, |t| now - t).min(Duration::from_secs(5));
    ed.stats.last_tick = Some(now);
    let rev = ed.session.rev();
    let changed = ed.stats.last_counts.is_none_or(|(r, _, _)| r != rev);
    if changed {
        ed.stats.last_input = Some(now);
    }
    if ed.stats.words_now.0 != rev || rev == 0 {
        ed.stats.words_now = (rev, word_count(&ed.session.doc));
    }
    // The sprint ends.
    if let Some(sp) = ed.stats.sprint.as_mut()
        && sp.result.is_none()
        && now >= sp.ends
    {
        let words = word_count(&ed.session.doc) as i64;
        let pages = ed.paginated().pages.len() as i64;
        if let Some(sp) = ed.stats.sprint.as_mut() {
            sp.result = Some((words - sp.start_words as i64, pages - sp.start_pages as i64));
        }
    }
    if !ed.stats.data.enabled {
        return;
    }
    let secs = crate::now_secs();
    let date = wordsmith_script::reports_more::date(secs);
    if date.is_empty() {
        return;
    }
    let idle = Duration::from_secs(u64::from(ed.stats.data.idle_minutes.max(1)) * 60);
    let active = ed.stats.last_input.is_some_and(|t| now - t < idle);
    let hour = ((secs % 86_400) / 3_600) as usize;
    if changed {
        let words = ed.stats.words_now.1;
        let pages = ed.paginated().pages.len();
        if let Some((_, w0, p0)) = ed.stats.last_counts {
            let day = ed.stats.data.days.entry(date.clone()).or_default();
            day.words += words.saturating_sub(w0) as u64;
            day.pages += pages.saturating_sub(p0) as u64;
            ed.stats.dirty = true;
        }
        ed.stats.last_counts = Some((rev, words, pages));
    }
    if active && !since.is_zero() {
        ed.stats.carry_ms += since.as_millis() as u64;
        let whole = ed.stats.carry_ms / 1_000;
        if whole > 0 {
            ed.stats.carry_ms %= 1_000;
            let day = ed.stats.data.days.entry(date).or_default();
            day.hours.resize(24, 0);
            day.seconds += whole;
            if let Some(h) = day.hours.get_mut(hour) {
                *h += whole;
            }
            ed.stats.dirty = true;
        }
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

fn stats_window(ed: &mut Editor, ctx: &Context) {
    let Some((mut metric, mut range)) = ed.stats.window else { return };
    let mut open = true;
    let today = wordsmith_script::reports_more::date(crate::now_secs());
    let today_n = day_number(&today).unwrap_or(0);
    let data = ed.stats.data.clone();
    let value = |d: &Day| match metric {
        "pages" => d.pages as f32,
        "time" => d.seconds as f32 / 60.0,
        _ => d.words as f32,
    };
    let by_day: BTreeMap<i64, &Day> = data.days.iter().filter_map(|(k, d)| day_number(k).map(|n| (n, d))).collect();
    let wrote = |n: i64| by_day.get(&n).is_some_and(|d| d.words > 0 || d.seconds >= 60);
    // Streaks of days with writing.
    let mut current = 0;
    let mut n = today_n;
    while wrote(n) || (n == today_n && wrote(n - 1)) {
        if wrote(n) {
            current += 1;
        }
        n -= 1;
    }
    let mut longest = 0;
    let mut run = 0;
    let mut last = None;
    for (&k, d) in &by_day {
        if d.words > 0 || d.seconds >= 60 {
            run = if last == Some(k - 1) { run + 1 } else { 1 };
            longest = longest.max(run);
            last = Some(k);
        }
    }
    let mut weekdays = [0f32; 7];
    let mut hours = [0u64; 24];
    for (&k, d) in &by_day {
        // 1970-01-01 was a Thursday.
        weekdays[((k + 3).rem_euclid(7)) as usize] += value(d);
        for (h, s) in d.hours.iter().enumerate().take(24) {
            hours[h] += s;
        }
    }
    const DAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    let peak_day = weekdays.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).filter(|(_, v)| **v > 0.0).map_or("—", |(i, _)| DAYS[i]);
    let peak_hour = hours
        .iter()
        .enumerate()
        .max_by_key(|(_, s)| **s)
        .filter(|(_, s)| **s > 0)
        .map(|(h, _)| format!("{h:02}:00 to {:02}:00 UTC", (h + 1) % 24))
        .unwrap_or_else(|| "—".into());
    let (span, bars): (i64, i64) = match range {
        "day" => (1, 14),
        "month" => (30, 12),
        "year" => (365, 5),
        _ => (7, 12),
    };
    let sums: Vec<f32> = (0..bars)
        .rev()
        .map(|b| {
            let end = today_n - b * span;
            (end - span + 1..=end).map(|k| by_day.get(&k).map_or(0.0, |d| value(d))).sum()
        })
        .collect();
    let goal_per_day = match metric {
        "pages" => data.goals.pages_per_day * data.goals.days_per_week as f32 / 7.0,
        "time" => data.goals.minutes_per_day as f32,
        _ => 0.0,
    };
    let goal = goal_per_day * span as f32;
    let average = sums.last().copied().unwrap_or(0.0) / span as f32;
    egui::Window::new("Writing Statistics").open(&mut open).default_width(520.0).show(ctx, |ui| {
        ui.horizontal(|ui| {
            for (label, v) in [
                ("Current streak", format!("{current} days")),
                ("Longest streak", format!("{longest} days")),
                ("Best day", peak_day.to_string()),
                ("Best hours", peak_hour.clone()),
            ] {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(label).small());
                        ui.strong(v);
                    });
                });
            }
        });
        ui.horizontal(|ui| {
            for (k, l) in [("words", "Words"), ("pages", "Pages"), ("time", "Minutes")] {
                if ui.selectable_label(metric == k, l).clicked() {
                    metric = k;
                }
            }
            ui.separator();
            for (k, l) in [("day", "Days"), ("week", "Weeks"), ("month", "Months"), ("year", "Years")] {
                if ui.selectable_label(range == k, l).clicked() {
                    range = k;
                }
            }
        });
        ui.label(format!("Average a day this {range}: {average:.1}"));
        // The bars, with the goal as a line across them.
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width().min(500.0), 160.0), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let top = sums.iter().copied().fold(goal.max(1.0), f32::max);
        let w = rect.width() / bars as f32;
        for (i, v) in sums.iter().enumerate() {
            let h = rect.height() * (v / top);
            let r = egui::Rect::from_min_max(
                pos2(rect.min.x + i as f32 * w + 2.0, rect.max.y - h),
                pos2(rect.min.x + (i + 1) as f32 * w - 2.0, rect.max.y),
            );
            painter.rect_filled(r, 2.0, Color32::from_rgb(90, 140, 210));
        }
        if goal > 0.0 {
            let y = rect.max.y - rect.height() * (goal / top);
            painter.line_segment([pos2(rect.min.x, y), pos2(rect.max.x, y)], Stroke::new(1.5, Color32::from_rgb(220, 120, 40)));
        }
        ui.separator();
        let g = &mut ed.stats.data.goals;
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Goals:");
            changed |= ui.add(egui::DragValue::new(&mut g.pages_per_day).range(0.0..=100.0).speed(0.25).suffix(" pages")).changed();
            ui.label("on");
            changed |= ui.add(egui::DragValue::new(&mut g.days_per_week).range(1..=7).suffix(" days a week")).changed();
            changed |= ui.add(egui::DragValue::new(&mut g.minutes_per_day).range(0..=1440).suffix(" minutes a day")).changed();
        });
        ui.horizontal(|ui| {
            changed |= ui.checkbox(&mut ed.stats.data.enabled, "Keep statistics").changed();
            ui.label("Stop counting time after");
            changed |= ui.add(egui::DragValue::new(&mut ed.stats.data.idle_minutes).range(1..=120).suffix(" idle minutes")).changed();
        });
        if changed {
            ed.stats.dirty = true;
        }
    });
    ed.stats.window = open.then_some((metric, range));
}

fn sprint_window(ed: &mut Editor, ctx: &Context) {
    let Some(sp) = ed.stats.sprint.clone() else { return };
    let Some((words, pages)) = sp.result else {
        ctx.request_repaint_after(Duration::from_secs(1));
        return;
    };
    let mut done = false;
    egui::Window::new("Sprint Finished").collapsible(false).resizable(false).show(ctx, |ui| {
        ui.label(format!("{} minutes: {words} words and {pages} pages written.", sp.minutes));
        if ui.button("OK").clicked() {
            done = true;
        }
    });
    if done {
        ed.stats.sprint = None;
    }
}

/// Start a sprint of the set length, or stop the one under way.
pub fn toggle_sprint(ed: &mut Editor) {
    if ed.stats.sprint.as_ref().is_some_and(|s| s.result.is_none()) {
        ed.stats.sprint = None;
        return;
    }
    let minutes = ed.stats.data.sprint_minutes.clamp(1, 240);
    let words = word_count(&ed.session.doc);
    let pages = ed.paginated().pages.len();
    ed.stats.sprint = Some(Sprint {
        ends: Instant::now() + Duration::from_secs(u64::from(minutes) * 60),
        minutes,
        start_words: words,
        start_pages: pages,
        result: None,
    });
}

/// The sprint's time left, for the status bar.
pub fn sprint_left(ed: &Editor) -> Option<Duration> {
    ed.stats.sprint.as_ref().filter(|s| s.result.is_none()).map(|s| s.ends.saturating_duration_since(Instant::now()))
}

pub fn show(ed: &mut Editor, ctx: &Context) {
    stats_window(ed, ctx);
    sprint_window(ed, ctx);
}

/// The statistics file beside the preferences.
#[cfg(not(target_arch = "wasm32"))]
pub mod file {
    use super::StatsData;

    pub const NAME: &str = "stats.json";

    /// The statistics in `dir`, or new ones.
    pub fn load(dir: &std::path::Path) -> StatsData {
        let path = dir.join(NAME);
        let text = match std::fs::metadata(&path) {
            Ok(m) if m.len() <= 8 << 20 => std::fs::read_to_string(&path).ok(),
            _ => None,
        };
        text.and_then(|t| serde_json::from_str::<StatsData>(&t).ok()).unwrap_or_default().repaired()
    }

    pub fn save(dir: &std::path::Path, data: &StatsData) -> Result<(), String> {
        let text = serde_json::to_string(data).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        jcj_project::files::write_atomic(&dir.join(NAME), text.as_bytes()).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_count_days() {
        assert_eq!(day_number("1970-01-01"), Some(0));
        assert_eq!(day_number("2000-03-01").map(|d| d - day_number("2000-02-28").unwrap_or(0)), Some(2));
        assert_eq!(day_number("nope"), None);
    }

    #[test]
    fn hostile_stats_are_bounded() {
        let mut d = StatsData { idle_minutes: 0, sprint_minutes: 10_000, ..StatsData::default() };
        d.goals.pages_per_day = f32::NAN;
        for i in 0..4_000 {
            d.days.insert(format!("{:04}-01-01", 1000 + i), Day::default());
        }
        let d = d.repaired();
        assert_eq!((d.idle_minutes, d.sprint_minutes, d.goals.pages_per_day), (1, 240, 1.0));
        assert!(d.days.len() <= MAX_DAYS);
        assert!(d.days.values().all(|x| x.hours.len() == 24));
    }
}
