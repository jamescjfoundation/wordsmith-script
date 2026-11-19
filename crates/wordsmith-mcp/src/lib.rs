// Modified by the Wordsmith Script contributors.
//! Wordsmith Script's MCP server (Model Context Protocol, JSON-RPC 2.0 over stdio), from
//! WordCraft's.
//!
//! Two backends: [`Headless`] runs an in-process editing session (no window); [`backend::Remote`]
//! talks to a running app through its control channel, so agents can also click, type and
//! screenshot the real UI.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![forbid(unsafe_code)]

pub mod backend;
pub mod server;
pub mod tools;

pub use backend::{Backend, Remote};
pub use server::Server;

use std::sync::Arc;

use base64::Engine as _;
use serde_json::{Value, json};
use wordsmith_doc::StoryRef;
use wordsmith_engine::Session;
use wordsmith_engine::file_access::{self, AutomationRoots};

/// Methods that need the desktop app.
pub const NEEDS_APP: &str = "needs the desktop app with its control port open";

/// An in-process session.
pub struct Headless {
    pub session: Session,
    roots: Option<Arc<AutomationRoots>>,
}

impl Headless {
    /// A headless server over `session` (the screenplay layer passes a session with its full
    /// command registry).
    pub fn new(session: Session) -> Self {
        Headless { session, roots: None }
    }

    /// Confine every file the session's commands read and write to `roots`
    /// (`--automation-read-root … --automation-write-root …`).
    pub fn with_automation_roots(mut self, roots: Option<Arc<AutomationRoots>>) -> Self {
        self.roots = roots;
        self
    }
}

/// The document as agents read it: `document.inspect` when the registry has one (the screenplay
/// layer's, with elements), else the body text, paragraph count and selection.
fn inspect(session: &mut Session, params: Value) -> Result<Value, String> {
    if session.registry.get("document.inspect").is_some() {
        return session.run("document.inspect", &params).map_err(|e| e.to_string());
    }
    Ok(json!({
        "text": session.doc.plain_text(StoryRef::Body),
        "paragraphs": session.doc.paragraph_count(),
        "selection": {"anchor": wordsmith_engine::cmd::pos_json(&session.sel.anchor), "focus": wordsmith_engine::cmd::pos_json(&session.sel.focus)},
    }))
}

fn answer_on(session: &mut Session, method: &str, params: Value) -> Result<Value, String> {
    match method {
        "engine.execute" | "command" => {
            let id = params.get("command").and_then(Value::as_str).ok_or("missing `command`")?.to_string();
            let p = params.get("params").cloned().filter(|v| !v.is_null()).unwrap_or(json!({}));
            session.run(&id, &p).map_err(|e| e.to_string())
        }
        "engine.commands" => Ok(session.registry.describe()),
        "document.inspect" => inspect(session, params),
        "ui.render" | "render.page" => {
            let page = params.get("page").and_then(Value::as_u64).unwrap_or(1).max(1);
            let page = usize::try_from(page).unwrap_or(usize::MAX).saturating_sub(1);
            let scale = params.get("scale").and_then(Value::as_f64).unwrap_or(1.0).clamp(0.1, 4.0) as f32;
            let l = session.export_layout();
            let pg = l.pages.get(page).ok_or_else(|| format!("no page {}", page.saturating_add(1)))?;
            let img = wordsmith_render::render_page(&session.doc, pg, scale, &Default::default());
            let png = img.to_png();
            Ok(
                json!({"png": base64::engine::general_purpose::STANDARD.encode(&png), "width": img.width, "height": img.height, "pages": l.pages.len()}),
            )
        }
        m if m.starts_with("ui.") => Err(format!("`{m}` {NEEDS_APP}")),
        other => session.run(other, &params).map_err(|e| e.to_string()),
    }
}

impl Default for Headless {
    fn default() -> Self {
        Headless::new(Session::new(wordsmith_doc::Document::new()))
    }
}

impl Backend for Headless {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let roots = self.roots.clone();
        file_access::confine(roots.as_ref(), || answer_on(&mut self.session, method, params))
    }
    fn has_ui(&self) -> bool {
        false
    }
    fn describe(&self) -> String {
        "headless session (no window)".into()
    }
}

#[cfg(test)]
mod tests;
