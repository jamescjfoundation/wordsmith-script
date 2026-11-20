// Modified by the Wordsmith Script contributors.
//! The editing session: document, selection, undo/redo, layout, view state.

use std::sync::Arc;

use serde_json::Value;
use wordsmith_doc::props::CharProps;
use wordsmith_doc::{Document, Pos, StoryRef};
use wordsmith_layout::{DocLayout, LayoutCache, LayoutOptions, ViewMode};

use crate::{CmdError, Registry};

mod history;
mod layout;
mod lifecycle;
mod query;
mod run;
mod snapshot;

use crate::doc_state;
use crate::snapshot as snapshot_types;
use doc_state::Undo;
pub use doc_state::{Selection, UNDO_LIST_MAX};
use snapshot_types::LayoutSlots;

/// Width of the continuous (normal and speed) views, in points, when the front end has not set
/// one: the page's text width, so lines break where they do on paper.
pub const DEFAULT_VIEW_WIDTH: f32 = 6.0 * 72.0;

/// An editing session: one document, its selection, undo history, cached layout and the
/// command registry every action goes through.
pub struct Session {
    pub doc: Document,
    pub sel: Selection,
    /// Formatting picked with a collapsed caret, applied to the next typed text.
    pub pending: Option<CharProps>,
    pub path: Option<std::path::PathBuf>,
    /// Name given to a document not saved yet: what Save suggests. A file name, not document
    /// metadata, so it is not part of the document or its undo history.
    pub file_name: Option<String>,
    pub dirty: bool,
    pub registry: Arc<Registry>,
    /// Author name for notes and revisions.
    pub author: String,
    /// The time now (Unix seconds), set by the front end; 0 when it has no clock. Tracked
    /// changes record it.
    pub now: u64,
    /// Last message for the status bar / agents.
    pub status: String,
    /// How the document is laid out for the editor: page view is [`ViewMode::Print`]; the
    /// continuous views use [`ViewMode::Draft`] at [`Self::view_width`].
    pub view_mode: ViewMode,
    /// Width of the continuous views, points.
    pub view_width: f32,
    /// Page x the caret tries to keep on Up/Down. Every other command clears it.
    pub goal_x: Option<f32>,
    /// Undo and redo steps. Shared so `run` can snapshot both stacks cheaply (a pointer per
    /// step) and put them back exactly when a command fails.
    history: Vec<Arc<Undo>>,
    /// Undo steps dropped so far because of the undo limit.
    undo_evicted: u64,
    redo: Vec<Arc<Undo>>,
    /// Typing is coalesced into one undo step until something else happens.
    typing_open: bool,
    /// The next mutating command joins the previous undo step (later frames of a drag).
    join_next: bool,
    /// Whether the undo group has a step of its own for the next joined command to merge
    /// into. A command that changed nothing leaves the group without one, so the next
    /// command in the group starts the step the group needs.
    join_anchor: bool,
    rev: u64,
    cache: LayoutCache,
    layout: LayoutSlots,
    /// Requests from commands to the UI (open a dialog, scroll…), drained by the front end.
    pub ui_requests: Vec<Value>,
    /// The recovery worker the document's copy is kept through (desktop hosts only).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) recovery: Option<crate::recovery::Recovery>,
    /// The document's recovery copy.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) recovery_slot: crate::recovery::Slot,
    /// Which document this is, unique in the process: changes when [`Session::set_document`]
    /// replaces it, never on an edit.
    document_id: u64,
}

/// A new document id, unique in the process.
pub(super) fn next_document_id() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
