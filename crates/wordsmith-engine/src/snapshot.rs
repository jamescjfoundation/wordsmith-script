// Modified by the Wordsmith Script contributors.
use crate::doc_state::{Selection, Undo};
use std::sync::Arc;

use wordsmith_doc::Document;
use wordsmith_layout::{DocLayout, ViewMode};

/// What `run` keeps to take a failed mutating command back: the document, the selection, the
/// undo and redo stacks, the open typing group and the undo steps dropped so far.
pub(super) type RunSnapshot = (Document, Selection, Vec<Arc<Undo>>, Vec<Arc<Undo>>, bool, u64);

/// A layout made and what it was made for: (revision, width, view, layout).
pub(super) type LayoutEntry = (u64, f32, ViewMode, Arc<DocLayout>);

/// The layouts last made, newest first. Two, so two views of one document (page view and normal
/// view side by side) each find theirs as they draw in turn.
pub(super) type LayoutSlots = [Option<LayoutEntry>; 2];
