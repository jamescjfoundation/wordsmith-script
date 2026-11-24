// SPDX-License-Identifier: MIT OR Apache-2.0
//! The workspace (SPEC-2 2, 8, 17, 19): document tabs, a split view, the start window and the
//! template chooser, writing statistics with goals and a sprint timer, and keys of the writer's
//! own. Statistics belong to the writer, not the script: they are kept beside the preferences
//! (`stats.json`), never in a script.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use egui::{Color32, Context, RichText, Stroke, Ui, pos2, vec2};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_script::dates::day_number;

use crate::editor::Editor;
use crate::structure::Structure;

/// Most recent scripts listed.
pub const MAX_RECENT: usize = 12;
/// Most days of statistics kept (about ten years).
pub const MAX_DAYS: usize = 3_700;
/// The sprint timer's default length, minutes (decision 0036).
pub const SPRINT_MINUTES: u32 = 25;

/// A second view beside (or below) the main one: what it shows (`None`: the script).
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
pub fn word_count(doc: &wordsmith_doc::Document) -> usize {
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

/// The tab bar, when more than one script is open.
pub fn tab_bar(ed: &mut Editor, ui: &mut Ui) {
    let names = ed.tab_names();
    if names.len() < 2 {
        return;
    }
    let mut switch = None;
    let mut close = false;
    egui::Panel::top("tabs").show(ui, |ui| {
        ui.horizontal(|ui| {
            for (i, (name, dirty)) in names.iter().enumerate() {
                let label = if *dirty { format!("{name} •") } else { name.clone() };
                if ui.selectable_label(i == ed.active_tab, label).clicked() {
                    switch = Some(i);
                }
                if i == ed.active_tab && ui.small_button("×").on_hover_text("Close this script").clicked() {
                    close = true;
                }
            }
            if ui.small_button("+").on_hover_text("New script in a tab").clicked() {
                switch = Some(usize::MAX);
            }
        });
    });
    match switch {
        Some(usize::MAX) => ed.new_tab(),
        Some(i) => ed.switch_tab(i),
        None => {}
    }
    if close {
        ed.close_tab(false);
    }
}

/// The split view's second pane.
pub fn split_panel(ed: &mut Editor, ui: &mut Ui) {
    let Some(split) = ed.split else { return };
    // Both panes showing the same thing is no split.
    if split.pane == ed.structure {
        return;
    }
    let show = |ed: &mut Editor, ui: &mut Ui| match split.pane {
        None => ed.script_pane(ui),
        Some(Structure::Cards) => crate::structure::cards(ed, ui),
        Some(Structure::Scenes) => crate::structure::scene_view(ed, ui),
        Some(Structure::Board) => crate::board::view(ed, ui),
        Some(Structure::Outline) => crate::outline::view(ed, ui),
    };
    if split.below {
        egui::Panel::bottom("split").resizable(true).default_size(320.0).show(ui, |ui| show(ed, ui));
    } else {
        egui::Panel::right("split").resizable(true).default_size(520.0).show(ui, |ui| show(ed, ui));
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

/// The start window: recent scripts, templates, and a few first steps.
fn start_window(ed: &mut Editor, ctx: &Context) {
    if !ed.tools.start {
        return;
    }
    let mut open = true;
    let mut act: Option<(&'static str, Value)> = None;
    let mut template: Option<String> = None;
    egui::Window::new("Welcome").open(&mut open).default_width(620.0).collapsible(false).show(ctx, |ui| {
        ui.columns(2, |cols| {
            cols[0].heading("Recent scripts");
            if ed.recent.is_empty() {
                cols[0].label("Scripts you open or save show here.");
            }
            for r in ed.recent.clone() {
                let name = std::path::Path::new(&r).file_name().and_then(|n| n.to_str()).unwrap_or(&r).to_string();
                if cols[0].link(name).on_hover_text(&r).clicked() {
                    act = Some(("file.openPath", json!({"path": r})));
                }
            }
            cols[0].add_space(8.0);
            if cols[0].button("Open…").clicked() {
                act = Some(("file.open", json!({})));
            }
            cols[1].heading("Start from");
            template_tree(&mut cols[1], &mut template);
        });
        ui.separator();
        ui.collapsing("First steps", |ui| {
            for tip in [
                "Return starts the next element and Tab changes this one: a heading leads to action, a name to dialogue.",
                "Type INT, press Tab, type the place, press Tab, type the time of day.",
                "Mod+1 to Mod+9 pick an element; the status bar says what Return and Tab will make.",
                "View › Index Cards, Story Board and Outline show the same script in other ways.",
                "Production › Revision Mode marks every change in the color of the current set.",
                "Tools › Reports counts scenes, locations and characters.",
            ] {
                ui.label(format!("• {tip}"));
            }
        });
        let mut show = ed.tools.show_start_on_launch;
        if ui.checkbox(&mut show, "Show this window when the app starts").changed() {
            ed.tools.show_start_on_launch = show;
        }
    });
    if let Some(id) = template {
        act = Some(("file.newFromTemplate", json!({"template": id})));
    }
    if let Some((cmd, params)) = act {
        open = false;
        if cmd == "file.newFromTemplate" && ed.session.dirty {
            // Unsaved work stays: the template opens in a tab of its own.
            ed.new_tab();
        }
        ed.run(cmd, params);
    }
    ed.tools.start = open;
}

/// The built-in templates as a tree of categories; a click picks one.
pub fn template_tree(ui: &mut Ui, picked: &mut Option<String>) {
    let mut category = "";
    for t in &wordsmith_script::templates::TEMPLATES {
        if t.category != category {
            category = t.category;
            ui.add_space(4.0);
            ui.label(RichText::new(category).strong());
        }
        if ui.link(format!("  {}", t.name)).on_hover_text(t.about).clicked() {
            *picked = Some(t.id.to_string());
        }
    }
}

fn templates_window(ed: &mut Editor, ctx: &Context) {
    if !ed.tools.templates {
        return;
    }
    let mut open = true;
    let mut picked = None;
    egui::Window::new("New from Template").open(&mut open).resizable(false).show(ctx, |ui| {
        template_tree(ui, &mut picked);
        ui.separator();
        ui.label("Templates of your own (File › Save as Template) open like any script.");
    });
    if let Some(id) = picked {
        open = false;
        if ed.session.dirty {
            ed.new_tab();
        }
        ed.run("file.newFromTemplate", json!({"template": id}));
    }
    ed.tools.templates = open;
}

/// The keys window: every command, its key, and the writer's own.
fn keys_window(ed: &mut Editor, ctx: &Context) {
    if ed.tools.keys.is_none() || ed.chrome.backstage == Some(crate::backstage::Page::Keys) {
        return;
    }
    let mut open = true;
    egui::Window::new("Keys").open(&mut open).default_size([560.0, 480.0]).show(ctx, |ui| keys_body(ed, ui));
    if !open {
        ed.tools.keys = None;
    }
}

/// Every command with its key and the writer's own, in a window or on the File page.
pub fn keys_body(ed: &mut Editor, ui: &mut Ui) {
    let (mut filter, mut capturing) = ed.tools.keys.take().unwrap_or_default();
    let mut set: Option<(String, String)> = None;
    let mut clear: Option<String> = None;
    let specs: Vec<(&'static str, &'static str, &'static str, &'static str)> =
        ed.session.registry.all().iter().filter(|s| s.params == "{}").map(|s| (s.id, s.label, s.location, s.shortcut)).collect();
    ui.horizontal(|ui| {
        ui.label("Find");
        ui.text_edit_singleline(&mut filter);
    });
    if let Some(id) = &capturing {
        ui.colored_label(Color32::from_rgb(200, 120, 30), format!("Press the new key for {id} (Escape to cancel)"));
        let pressed = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*key, *modifiers)),
                _ => None,
            })
        });
        if let Some((key, m)) = pressed {
            if key == egui::Key::Escape {
                capturing = None;
            } else if let Some(sc) = crate::keys::shortcut(key, m, ed.mac) {
                set = Some((sc, id.clone()));
                capturing = None;
            }
        }
    }
    ui.separator();
    egui::ScrollArea::vertical().id_salt("keys-list").show(ui, |ui| {
        egui::Grid::new("keys").striped(true).num_columns(4).show(ui, |ui| {
            let f = filter.to_lowercase();
            for (id, label, loc, default) in &specs {
                if !f.is_empty() && !format!("{id} {label} {loc}").to_lowercase().contains(&f) {
                    continue;
                }
                ui.label(*label).on_hover_text(*id);
                ui.label(RichText::new(*default).weak());
                let mine: Vec<String> = ed.keys.iter().filter(|(_, c)| c.as_str() == *id).map(|(k, _)| k.clone()).collect();
                ui.label(mine.join(", "));
                ui.horizontal(|ui| {
                    if ui.small_button("Set…").clicked() {
                        capturing = Some((*id).to_string());
                    }
                    if !mine.is_empty() && ui.small_button("Reset").clicked() {
                        clear = Some((*id).to_string());
                    }
                });
                ui.end_row();
            }
        });
    });
    if let Some((sc, id)) = set {
        ed.keys.insert(sc, id);
    }
    if let Some(id) = clear {
        ed.keys.retain(|_, c| *c != id);
    }
    ed.tools.keys = Some((filter, capturing));
}

pub fn show(ed: &mut Editor, ctx: &Context) {
    stats_window(ed, ctx);
    sprint_window(ed, ctx);
    start_window(ed, ctx);
    templates_window(ed, ctx);
    keys_window(ed, ctx);
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
        craft_atomic::atomic_write(&dir.join(NAME), text.as_bytes()).map_err(|e| e.to_string())
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
