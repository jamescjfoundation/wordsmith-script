// SPDX-License-Identifier: MIT OR Apache-2.0
//! The README's pictures, drawn by the app's own offscreen renderer (`wordsmith_script_ui::snapshot`):
//! `cargo xtask screenshots` runs this with the folder to write them to.

use std::path::{Path, PathBuf};

use egui::Vec2;
use jcj_shell::workspace::Layout;
use jcj_shell::{Page, Shell};
use serde_json::json;
use wordsmith_script_ui::Editor;
use wordsmith_script_ui::editor::View;
use wordsmith_script_ui::ribbon::tab;
use wordsmith_script_ui::structure::Structure;

const SIZE: Vec2 = Vec2::new(1280.0, 820.0);
const PPP: f32 = 1.5;

fn main() {
    let out = std::env::args().nth(1).map_or_else(|| PathBuf::from("docs/images"), PathBuf::from);
    if let Err(e) = run(&out) {
        eprintln!("screenshots: {e}");
        std::process::exit(1);
    }
}

/// An editor on a public-domain sample with the features the pictures show.
fn sample() -> Result<Editor, String> {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("../script-format/tests/golden/everybodys-girl.fountain");
    let text = std::fs::read_to_string(&golden).map_err(|e| format!("{}: {e}", golden.display()))?;
    let doc = wordsmith_script_format::fountain::read(&text)?.doc;
    let mut session = wordsmith_script_ui::new_session();
    session.set_document(doc);
    let mut ed = Editor::new(session);
    ed.mac = false;
    let run = |ed: &mut Editor, id: &str, params: serde_json::Value| {
        ed.run(id, params);
        if !ed.message.is_empty() {
            eprintln!("{id}: {}", ed.message);
        }
    };
    run(&mut ed, "titlePage.set", json!({"credit": "Adapted by", "authors": "A. Van Buren Powell", "draftDate": "Blue revision"}));
    run(&mut ed, "lock.pages", json!({}));
    run(&mut ed, "revision.next", json!({}));
    // A revision on the first page.
    ed.session.sel = jcj_engine::Selection::caret(jcj_doc::Pos { off: 0, path: jcj_doc::Path::top(2), ..ed.session.sel.focus.clone() });
    run(&mut ed, "text.insert", json!({"text": "Morning light. "}));
    // Beats and lanes for the story board and the outline.
    run(&mut ed, "outline.template", json!({"name": "three-act"}));
    let beats = [
        ("Florence at the shop", "She trims hats she can never own.", 1, 40.0, 40.0),
        ("Brickdust Row", "The boarding house, the gang, the rent.", 1, 260.0, 40.0),
        ("Blinker's offer", "A rich young man and a carriage ride.", 2, 480.0, 40.0),
        ("Coney Island", "The walking beam, the moonlight, the question.", 2, 480.0, 200.0),
        ("Bill's jealousy", "The gang leader watches from the rail.", 2, 260.0, 200.0),
        ("Home", "Florence chooses.", 3, 700.0, 120.0),
    ];
    for (i, (title, body, lane, x, y)) in beats.iter().enumerate() {
        run(&mut ed, "beat.add", json!({"title": title, "body": body, "lane": lane, "x": x, "y": y}));
        if i > 0 {
            run(&mut ed, "beat.link", json!({"from": i, "to": i + 1}));
        }
    }
    ed.session.sel = jcj_engine::Selection::caret(jcj_doc::Pos { off: 0, path: jcj_doc::Path::top(4), ..ed.session.sel.focus.clone() });
    Ok(ed)
}

/// Open panels `open` (and no others).
fn panels(shell: &mut Shell, open: &[&str]) {
    shell.layout = Layout::with_open(&wordsmith_script_ui::sidebar::panels(), open);
}

fn shot(shell: &mut Shell, ed: &mut Editor, out: &Path, name: &str) -> Result<(), String> {
    let pic = wordsmith_script_ui::snapshot::render(shell, ed, SIZE, PPP, Vec::new(), 4);
    let path = out.join(format!("{name}.png"));
    std::fs::write(&path, pic.to_png()).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("{}", path.display());
    Ok(())
}

fn run(out: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut ed = sample()?;
    let mut shell = wordsmith_script_ui::shell();
    let s = &mut shell;
    ed.set_view(View::Page);
    panels(s, &["scenes", "scene"]);
    shot(s, &mut ed, out, "page-view")?;
    s.chrome.tab = tab::PRODUCTION.into();
    panels(s, &["scenes", "revisions"]);
    shot(s, &mut ed, out, "ribbon-production")?;
    s.chrome.tab = tab::REVIEW.into();
    panels(s, &["characters", "element"]);
    shot(s, &mut ed, out, "ribbon-review")?;
    s.open_file_page(Page::New);
    shot(s, &mut ed, out, "file-page")?;
    s.chrome.file_page = None;
    s.chrome.tab = tab::WRITE.into();
    panels(s, &["scenes"]);
    ed.structure = Some(Structure::Cards);
    shot(s, &mut ed, out, "index-cards")?;
    ed.structure = Some(Structure::Board);
    shot(s, &mut ed, out, "story-board")?;
    ed.structure = Some(Structure::Outline);
    shot(s, &mut ed, out, "outline")?;
    ed.structure = None;
    s.mode = jcj_shell::theme::Mode::Dark;
    ed.set_view(View::Draft);
    shot(s, &mut ed, out, "draft-view-dark")?;
    Ok(())
}
