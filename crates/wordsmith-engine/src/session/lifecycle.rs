// Modified by the Wordsmith Script contributors.
use super::doc_state::{MAX_UI_REQUESTS, Selection};
use super::{Arc, DEFAULT_VIEW_WIDTH, Document, LayoutCache, Registry, Session, StoryRef, Value, ViewMode, next_document_id};

impl Session {
    /// Queue a request for the front end. A headless session never drains the queue, so only
    /// the newest [`MAX_UI_REQUESTS`] are kept.
    pub fn push_ui_request(&mut self, req: Value) {
        if self.ui_requests.len() >= MAX_UI_REQUESTS {
            self.ui_requests.remove(0);
        }
        self.ui_requests.push(req);
    }

    /// Start an editing session at the document beginning with empty history, the engine's own
    /// commands registered and page view.
    pub fn new(doc: Document) -> Self {
        Self::with_registry(doc, Arc::new(crate::cmd::registry()))
    }

    /// Start a session on `doc` that runs the commands in `registry` (the screenplay layer
    /// passes its full registry here).
    pub fn with_registry(mut doc: Document, registry: Arc<Registry>) -> Self {
        doc.ensure_nonempty();
        let sel = Selection::caret(doc.start_of(StoryRef::Body));
        Session {
            doc,
            sel,
            pending: None,
            path: None,
            file_name: None,
            dirty: false,
            registry,
            author: String::new(),
            now: 0,
            status: String::new(),
            view_mode: ViewMode::Print,
            view_width: DEFAULT_VIEW_WIDTH,
            goal_x: None,
            history: Vec::new(),
            undo_evicted: 0,
            redo: Vec::new(),
            typing_open: false,
            join_next: false,
            join_anchor: false,
            rev: 0,
            cache: LayoutCache::new(),
            layout: Default::default(),
            ui_requests: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            recovery: None,
            #[cfg(not(target_arch = "wasm32"))]
            recovery_slot: Default::default(),
            document_id: next_document_id(),
        }
    }
}
