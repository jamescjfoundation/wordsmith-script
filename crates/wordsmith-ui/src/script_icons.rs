// SPDX-License-Identifier: MIT OR Apache-2.0
//! Icons for screenplay work, our own, drawn in code on a 20 × 20 grid in two colors (line work
//! and an accent), beside the general icons copied from WordCraft (`crate::icons`). [`paint`]
//! tries ours first, then WordCraft's, which falls back to a lettered tile.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

struct Pen<'a> {
    p: &'a Painter,
    r: Rect,
    c: Color32,
    a: Color32,
}

impl Pen<'_> {
    fn s(&self) -> f32 {
        self.r.width().min(self.r.height()) / 20.0
    }
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        let s = self.s();
        let o = self.r.center() - vec2(10.0 * s, 10.0 * s);
        pos2(o.x + x * s, o.y + y * s)
    }
    fn stroke(&self, c: Color32) -> Stroke {
        Stroke::new(1.25 * self.s(), c)
    }
    fn line(&self, pts: &[(f32, f32)], c: Color32) {
        self.p.add(Shape::line(pts.iter().map(|&(x, y)| self.pt(x, y)).collect(), self.stroke(c)));
    }
    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, c: Color32) {
        self.p.rect_stroke(Rect::from_min_max(self.pt(x0, y0), self.pt(x1, y1)), 1.5, self.stroke(c), egui::StrokeKind::Middle);
    }
    fn frect(&self, x0: f32, y0: f32, x1: f32, y1: f32, c: Color32) {
        self.p.rect_filled(Rect::from_min_max(self.pt(x0, y0), self.pt(x1, y1)), 1.0, c);
    }
    fn fill(&self, pts: &[(f32, f32)], c: Color32) {
        self.p.add(Shape::convex_polygon(pts.iter().map(|&(x, y)| self.pt(x, y)).collect(), c, Stroke::NONE));
    }
    fn circle(&self, x: f32, y: f32, r: f32, c: Color32) {
        self.p.circle_stroke(self.pt(x, y), r * self.s(), self.stroke(c));
    }
    fn fcircle(&self, x: f32, y: f32, r: f32, c: Color32) {
        self.p.circle_filled(self.pt(x, y), r * self.s(), c);
    }
    /// A page with text lines from `x0` to `x1` at `ys`.
    fn page(&self) {
        self.rect(4.0, 2.5, 16.0, 17.5, self.c);
    }
    fn lines(&self, x0: f32, x1: f32, ys: &[f32], c: Color32) {
        for y in ys {
            self.line(&[(x0, *y), (x1, *y)], c);
        }
    }
    fn text(&self, x: f32, y: f32, size: f32, t: &str, c: Color32) {
        self.p.text(self.pt(x, y), egui::Align2::CENTER_CENTER, t, crate::theme::semibold(size * self.s()), c);
    }
}

/// Paint icon `name` into `r`: ours, else WordCraft's general set.
pub fn paint(p: &Painter, r: Rect, name: &str, c: Color32, accent: Color32) {
    let pen = Pen { p, r, c, a: accent };
    if !ours(&pen, name) {
        crate::icons::paint(p, r, name, c, accent);
    }
}

fn ours(pen: &Pen, name: &str) -> bool {
    let (c, a) = (pen.c, pen.a);
    match name {
        // Elements: where the text sits on a page.
        "el.sceneHeading" => {
            pen.page();
            pen.frect(6.0, 5.0, 14.0, 7.0, a);
            pen.lines(6.0, 14.0, &[10.0, 12.5, 15.0], c);
        }
        "el.action" => {
            pen.page();
            pen.lines(6.0, 14.0, &[6.0, 8.5, 11.0, 13.5], a);
        }
        "el.character" => {
            pen.page();
            pen.frect(8.5, 5.0, 11.5, 7.0, a);
            pen.lines(7.0, 13.0, &[10.0, 12.5], c);
        }
        "el.parenthetical" => {
            pen.page();
            pen.text(10.0, 10.0, 9.0, "( )", a);
        }
        "el.dialogue" => {
            pen.page();
            pen.lines(7.0, 13.0, &[6.0, 8.5, 11.0, 13.5], a);
        }
        "el.transition" => {
            pen.page();
            pen.frect(10.5, 13.0, 14.0, 15.0, a);
            pen.lines(6.0, 14.0, &[6.0, 8.5], c);
        }
        "el.shot" => {
            pen.rect(3.0, 6.0, 13.0, 14.0, c);
            pen.fill(&[(13.0, 10.0), (17.5, 7.0), (17.5, 13.0)], a);
        }
        "el.more" => {
            pen.page();
            for x in [7.0, 10.0, 13.0] {
                pen.fcircle(x, 10.0, 1.0, a);
            }
        }
        "el.sequence" => {
            pen.frect(3.0, 8.0, 7.0, 12.0, a);
            pen.frect(8.0, 8.0, 12.0, 12.0, a);
            pen.frect(13.0, 8.0, 17.0, 12.0, a);
            pen.line(&[(3.0, 15.0), (17.0, 15.0)], c);
        }
        "el.outline" => {
            pen.lines(4.0, 16.0, &[5.0], a);
            pen.lines(7.0, 16.0, &[9.0, 13.0], c);
            pen.lines(10.0, 16.0, &[17.0], c);
        }
        "el.reset" => {
            pen.page();
            pen.line(&[(7.0, 13.0), (13.0, 7.0)], a);
            pen.fill(&[(13.0, 7.0), (10.0, 7.0), (13.0, 10.0)], a);
        }
        "el.case" => pen.text(10.0, 10.5, 10.0, "Aa", a),
        "el.quotes" => pen.text(10.0, 11.0, 14.0, "“”", a),
        // Views.
        "view.page" => {
            pen.page();
            pen.lines(6.0, 14.0, &[6.0, 9.0, 12.0], c);
        }
        "view.normal" => pen.lines(3.0, 17.0, &[5.0, 8.0, 11.0, 14.0, 17.0], c),
        "view.speed" => {
            pen.lines(7.0, 17.0, &[5.0, 9.0, 13.0, 17.0], c);
            for y in [5.0, 9.0, 13.0, 17.0] {
                pen.fcircle(4.0, y, 1.0, a);
            }
        }
        "view.cards" => {
            for (x, y) in [(2.5, 3.0), (10.5, 3.0), (2.5, 11.0), (10.5, 11.0)] {
                pen.rect(x, y, x + 7.0, y + 6.0, c);
                pen.line(&[(x + 1.5, y + 2.0), (x + 5.5, y + 2.0)], a);
            }
        }
        "view.scenes" => {
            for y in [3.0, 8.5, 14.0] {
                pen.rect(3.0, y, 17.0, y + 4.0, c);
                pen.frect(3.0, y, 5.0, y + 4.0, a);
            }
        }
        "view.board" => {
            pen.frect(2.5, 3.0, 9.0, 8.0, a);
            pen.rect(11.0, 10.0, 17.5, 15.0, c);
            pen.rect(3.0, 12.0, 8.0, 17.0, c);
            pen.line(&[(9.0, 6.0), (13.0, 10.0)], c);
        }
        "view.outlineView" => {
            for (i, y) in [4.0, 9.0, 14.0].iter().enumerate() {
                let x = 3.0 + i as f32 * 3.0;
                pen.frect(x, *y, x + 9.0, y + 3.0, if i == 0 { a } else { c });
            }
        }
        "view.strip" => {
            pen.rect(2.5, 5.0, 17.5, 15.0, c);
            pen.frect(3.5, 6.0, 9.0, 9.5, a);
            pen.frect(9.5, 6.0, 16.5, 9.5, c);
            pen.frect(3.5, 10.5, 6.0, 14.0, a);
            pen.frect(6.5, 10.5, 12.0, 14.0, c);
            pen.frect(12.5, 10.5, 16.5, 14.0, a);
        }
        "view.tables" => {
            pen.rect(3.0, 4.0, 17.0, 16.0, c);
            pen.line(&[(3.0, 8.0), (17.0, 8.0)], a);
            pen.line(&[(3.0, 12.0), (17.0, 12.0)], c);
            pen.line(&[(8.0, 4.0), (8.0, 16.0)], c);
        }
        "view.navigator" => {
            pen.rect(2.5, 3.0, 17.5, 17.0, c);
            pen.frect(3.0, 3.5, 7.5, 16.5, a);
        }
        "view.inspector" => {
            pen.rect(2.5, 3.0, 17.5, 17.0, c);
            pen.frect(12.5, 3.5, 17.0, 16.5, a);
        }
        "view.split" => {
            pen.rect(2.5, 3.0, 17.5, 17.0, c);
            pen.line(&[(10.0, 3.0), (10.0, 17.0)], a);
        }
        "view.focus" => {
            pen.line(&[(3.0, 7.0), (3.0, 3.0), (7.0, 3.0)], c);
            pen.line(&[(13.0, 3.0), (17.0, 3.0), (17.0, 7.0)], c);
            pen.line(&[(17.0, 13.0), (17.0, 17.0), (13.0, 17.0)], c);
            pen.line(&[(7.0, 17.0), (3.0, 17.0), (3.0, 13.0)], c);
            pen.fcircle(10.0, 10.0, 2.5, a);
        }
        "view.typewriter" => {
            pen.lines(4.0, 16.0, &[5.0, 15.0], c);
            pen.frect(3.0, 8.5, 17.0, 11.5, a);
        }
        "view.dark" => {
            pen.fcircle(10.0, 10.0, 6.0, c);
            pen.fcircle(12.5, 8.0, 5.0, pen_bg(pen));
        }
        "view.highlight" => {
            pen.frect(3.0, 7.0, 17.0, 13.0, a);
            pen.lines(5.0, 15.0, &[10.0], c);
        }
        "view.zoomIn" => {
            pen.circle(8.5, 8.5, 5.0, c);
            pen.line(&[(12.0, 12.0), (17.0, 17.0)], c);
            pen.line(&[(6.0, 8.5), (11.0, 8.5)], a);
            pen.line(&[(8.5, 6.0), (8.5, 11.0)], a);
        }
        "view.zoomOut" => {
            pen.circle(8.5, 8.5, 5.0, c);
            pen.line(&[(12.0, 12.0), (17.0, 17.0)], c);
            pen.line(&[(6.0, 8.5), (11.0, 8.5)], a);
        }
        // Writing and going.
        "goto" => {
            pen.page();
            pen.fill(&[(8.0, 7.0), (14.0, 10.0), (8.0, 13.0)], a);
        }
        "bookmark" => pen.fill(&[(6.0, 3.0), (14.0, 3.0), (14.0, 17.0), (10.0, 13.5), (6.0, 17.0)], a),
        "note" => {
            pen.frect(4.0, 4.0, 16.0, 13.0, a);
            pen.fill(&[(7.0, 13.0), (11.0, 13.0), (7.0, 17.0)], a);
            pen.lines(6.5, 13.5, &[7.0, 10.0], pen_bg(pen));
        }
        "macro" => {
            pen.rect(3.0, 5.0, 17.0, 15.0, c);
            pen.fill(&[(8.0, 7.5), (13.0, 10.0), (8.0, 12.5)], a);
        }
        "scene.new" => {
            pen.page();
            pen.line(&[(10.0, 7.0), (10.0, 13.0)], a);
            pen.line(&[(7.0, 10.0), (13.0, 10.0)], a);
        }
        "pagebreak" => {
            pen.rect(4.0, 2.0, 16.0, 8.0, c);
            pen.rect(4.0, 12.0, 16.0, 18.0, c);
            pen.line(&[(2.0, 10.0), (18.0, 10.0)], a);
        }
        "dual" => {
            pen.page();
            pen.lines(5.5, 9.0, &[6.0, 8.5, 11.0], a);
            pen.lines(11.0, 14.5, &[6.0, 8.5, 11.0, 13.5], a);
        }
        "titlepage" => {
            pen.page();
            pen.lines(7.5, 12.5, &[8.0], a);
            pen.lines(8.5, 11.5, &[10.5], c);
        }
        // Production.
        "revision" => {
            pen.page();
            pen.text(14.0, 6.0, 10.0, "*", a);
            pen.lines(6.0, 12.0, &[9.0, 12.0], c);
        }
        "revision.next" => {
            pen.frect(3.0, 4.0, 9.0, 16.0, Color32::from_rgb(0x9C, 0xBF, 0xF0));
            pen.frect(11.0, 4.0, 17.0, 16.0, Color32::from_rgb(0xF6, 0xB8, 0xC8));
            pen.rect(3.0, 4.0, 17.0, 16.0, c);
        }
        "lock" => {
            pen.rect(5.0, 9.0, 15.0, 17.0, c);
            pen.line(&[(7.0, 9.0), (7.0, 6.0), (10.0, 3.5), (13.0, 6.0), (13.0, 9.0)], a);
        }
        "unlock" => {
            pen.rect(5.0, 9.0, 15.0, 17.0, c);
            pen.line(&[(7.0, 9.0), (7.0, 6.0), (10.0, 3.5), (13.0, 5.0)], a);
        }
        "scene.number" => {
            pen.page();
            pen.text(10.0, 10.0, 9.0, "12", a);
        }
        "omit" => {
            pen.page();
            pen.line(&[(5.0, 15.0), (15.0, 5.0)], a);
        }
        "track" => {
            pen.lines(4.0, 16.0, &[6.0, 14.0], c);
            pen.line(&[(4.0, 10.0), (16.0, 10.0)], Color32::from_rgb(0xC0, 0x39, 0x2B));
            pen.line(&[(4.0, 16.0), (12.0, 16.0)], a);
        }
        "compare" => {
            pen.rect(2.5, 3.0, 9.0, 15.0, c);
            pen.rect(11.0, 5.0, 17.5, 17.0, a);
            pen.line(&[(9.0, 9.0), (11.0, 9.0)], c);
        }
        "watermark" => {
            pen.page();
            pen.line(&[(6.0, 15.0), (14.0, 5.0)], a);
        }
        // Review and habits.
        "check" => {
            pen.page();
            pen.line(&[(6.5, 10.0), (9.0, 12.5), (14.0, 7.0)], a);
        }
        "report" => {
            pen.frect(4.0, 11.0, 7.0, 16.0, a);
            pen.frect(8.5, 7.0, 11.5, 16.0, c);
            pen.frect(13.0, 4.0, 16.0, 16.0, a);
        }
        "stats" => {
            pen.line(&[(3.0, 15.0), (7.0, 10.0), (11.0, 12.0), (17.0, 5.0)], a);
            pen.line(&[(3.0, 17.0), (17.0, 17.0)], c);
        }
        "sprint" => {
            pen.circle(10.0, 11.0, 6.0, c);
            pen.line(&[(10.0, 11.0), (10.0, 7.5)], a);
            pen.line(&[(8.0, 3.0), (12.0, 3.0)], c);
        }
        "characters" => {
            pen.circle(10.0, 7.0, 3.0, c);
            pen.line(&[(4.0, 17.0), (5.5, 13.0), (10.0, 11.5), (14.5, 13.0), (16.0, 17.0)], a);
        }
        "beat" => {
            pen.frect(3.0, 4.0, 13.0, 12.0, a);
            pen.rect(7.0, 8.0, 17.0, 16.0, c);
        }
        "link" => {
            pen.circle(6.0, 10.0, 2.5, c);
            pen.circle(14.0, 10.0, 2.5, c);
            pen.line(&[(8.5, 10.0), (11.5, 10.0)], a);
        }
        "arrange" => {
            for y in [4.0, 9.0, 14.0] {
                pen.frect(4.0, y, 12.0, y + 3.0, a);
            }
            pen.line(&[(3.0, 3.0), (3.0, 17.0)], c);
        }
        "export" => {
            pen.page();
            pen.line(&[(10.0, 14.0), (10.0, 6.0)], a);
            pen.fill(&[(7.0, 8.5), (13.0, 8.5), (10.0, 5.0)], a);
        }
        "template" => {
            pen.page();
            pen.frect(6.0, 5.0, 14.0, 8.0, a);
            pen.rect(6.0, 10.0, 14.0, 15.0, c);
        }
        "newtab" => {
            pen.rect(3.0, 6.0, 17.0, 16.0, c);
            pen.frect(3.0, 4.0, 9.0, 6.0, a);
        }
        _ => return false,
    }
    true
}

/// A color that reads as "cut out" over the line color: the opposite end of the scale.
fn pen_bg(pen: &Pen) -> Color32 {
    let l = u16::from(pen.c.r()) + u16::from(pen.c.g()) + u16::from(pen.c.b());
    if l > 382 { Color32::from_gray(30) } else { Color32::WHITE }
}

/// The icon a command shows on the ribbon, by its id.
pub fn for_command(id: &str) -> &'static str {
    match id {
        "element.sceneHeading" => "el.sceneHeading",
        "element.action" => "el.action",
        "element.character" => "el.character",
        "element.parenthetical" => "el.parenthetical",
        "element.dialogue" => "el.dialogue",
        "element.transition" => "el.transition",
        "element.shot" => "el.shot",
        "element.sequence" => "el.sequence",
        "element.outline1" | "element.outline2" | "element.outline3" | "element.summary" | "element.note" | "outline.show" => "el.outline",
        "element.reset" => "el.reset",
        "text.toggleCase" => "el.case",
        "typing.smartQuotes" => "el.quotes",
        "typing.autoCapitalize" => "el.case",
        "edit.cut" => "cut",
        "edit.copy" => "copy",
        "edit.undo" => "undo",
        "edit.redo" => "redo",
        "file.save" | "file.saveAs" => "save",
        "file.open" | "file.openPath" => "open",
        "file.print" | "view.print" | "file.printPreview" | "file.printRevised" => "print",
        "format.bold" => "bold",
        "format.italic" => "italic",
        "format.underline" => "underline",
        "format.plain" => "clear",
        "view.find" => "find",
        "view.goto" => "goto",
        "bookmark.add" | "view.bookmarks" => "bookmark",
        "view.note" | "note.next" | "note.previous" => "note",
        "view.macros" => "macro",
        "scene.insert" => "scene.new",
        "text.pageBreak" => "pagebreak",
        "dialogue.dual" => "dual",
        "view.titlePage" => "titlepage",
        "view.headerFooter" => "header",
        "view.watermark" => "watermark",
        "view.page" => "view.page",
        "view.normal" => "view.normal",
        "view.speed" => "view.speed",
        "view.cards" => "view.cards",
        "view.scenes" => "view.scenes",
        "view.board" => "view.board",
        "view.outline" => "view.outlineView",
        "view.strip" => "view.strip",
        "view.tables" => "view.tables",
        "view.focusMode" => "view.focus",
        "view.typewriter" => "view.typewriter",
        "view.darkTheme" | "view.extraDark" => "view.dark",
        "view.highlights" => "view.highlight",
        "view.zoomIn" => "view.zoomIn",
        "view.zoomOut" => "view.zoomOut",
        "revision.mode" | "revision.markRevised" | "revision.clearRevised" | "revision.gotoNext" | "revision.gotoPrevious" | "view.revisions" => {
            "revision"
        }
        "revision.next" => "revision.next",
        "lock.pages" | "lock.relock" | "lock.lockPage" => "lock",
        "lock.unlockAll" | "lock.unlockLevel" | "lock.unlockPage" => "unlock",
        "scene.showNumbers" | "scene.lockNumbers" | "scene.renumber" | "view.sceneNumbers" => "scene.number",
        "scene.omit" | "scene.recall" => "omit",
        "track.toggle" | "track.next" | "track.previous" => "track",
        "track.accept" | "track.acceptAll" => "accept",
        "track.reject" | "track.rejectAll" => "reject",
        "compare.withFile" => "compare",
        "view.checker" => "check",
        "view.reports" => "report",
        "view.stats" => "stats",
        "view.sprint" => "sprint",
        "view.sceneProperties" | "view.character" => "characters",
        "beat.add" => "beat",
        "beats.arrange" => "arrange",
        "storyboard.exportPdf" | "cards.exportPdf" => "export",
        "view.templates" | "file.saveAsTemplate" => "template",
        "file.newTab" => "newtab",
        "view.splitCards" | "view.splitBoard" | "view.splitOutline" | "view.splitScenes" | "view.splitScript" | "view.unsplit" => "view.split",
        _ => "",
    }
}
