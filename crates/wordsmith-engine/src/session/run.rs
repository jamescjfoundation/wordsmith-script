// Modified by the Wordsmith Script contributors.
use super::doc_state::Selection;
use super::snapshot_types::RunSnapshot;
use super::{CharProps, CmdError, Document, Session, Value};
use crate::CommandSpec;

/// Commands that move the caret a line up or down and keep [`Session::goal_x`].
fn is_vertical(id: &str) -> bool {
    matches!(id, "caret.up" | "caret.down" | "select.up" | "select.down" | "caret.pageUp" | "caret.pageDown" | "select.pageUp" | "select.pageDown")
}

struct SuccessContext<'a> {
    spec: &'a CommandSpec,
    before_doc: &'a Option<RunSnapshot>,
    pending_before: &'a Option<Option<CharProps>>,
    document_id_before: u64,
    dirty_before: Option<bool>,
    rev_restore: Option<u64>,
    pushed: bool,
    sel_before: &'a Selection,
}

impl Session {
    /// Whether the command changed nothing: the document, the selection and the formatting
    /// waiting at the caret are as they were before it ran.
    ///
    /// A command that changes nothing is not an edit: it adds no undo step, marks no
    /// unsaved changes, forces no relayout and leaves the redo stack alone (#23).
    fn unchanged(&self, doc: &Document, sel: &Selection, pending: &Option<CharProps>) -> bool {
        doc.same_content(&self.doc) && *sel == self.sel && *pending == self.pending
    }

    /// Run a command by id with JSON params (the single entry point for the UI, CLI, MCP and
    /// control channel). Mutating commands are undoable; a failed command leaves the document as
    /// it was; a panic inside a command becomes an error.
    pub fn run(&mut self, id: &str, params: &Value) -> Result<Value, CmdError> {
        let join = std::mem::take(&mut self.join_next);
        let reg = self.registry.clone();
        let Some(spec) = reg.get(id) else { return Err(CmdError::Unknown(id.to_string())) };
        if let Some(why) = (spec.enabled)(self) {
            return Err(CmdError::Disabled(format!("{id}: {why}")));
        }
        // The undo stacks are snapshotted whole, not as a length: commands run nested commands,
        // whose checkpoints, undos and redos (and evictions at the limit) must be put back too.
        let before_doc: Option<RunSnapshot> = if spec.mutates {
            Some((self.doc.clone(), self.sel.clone(), self.history.clone(), self.redo.clone(), self.typing_open, self.undo_evicted))
        } else {
            None
        };
        // The formatting waiting at the caret (⌘B with nothing selected): an undo step restores
        // it, so a command that only changes it is an edit.
        let pending_before = spec.mutates.then(|| self.pending.clone());
        // Which document is open. A command that replaces the document (open, new) starts a new
        // undo history: it is not an edit of this document, whatever the new document holds.
        let document_id_before = self.document_id;
        // Whether the document has unsaved changes, and the revision its layout is keyed on: a
        // command that changes nothing leaves both as they were, and a failed one leaves no
        // trace.
        let dirty_before = spec.mutates.then_some(self.dirty);
        let rev_restore = spec.mutates.then_some(self.rev);
        let typing = spec.id == "text.insert";
        if spec.mutates && !typing {
            self.typing_open = false;
        }
        // Whether this command pushed an undo step (a no-op command's step is dropped below,
        // leaving its undo group without one).
        let mut pushed = false;
        if spec.mutates {
            let label = if typing { "Typing" } else { spec.label };
            // A joined command merges into its group's step; a group whose first command
            // changed nothing has none, so the next command in it starts one (#4001).
            if !join || self.history.is_empty() || !self.join_anchor {
                pushed = self.checkpoint(label);
            }
        } else if spec.id.starts_with("caret.") {
            self.typing_open = false;
        }
        let sel_before = self.sel.clone();
        // Up and Down keep the column the caret started in; anything else forgets it.
        if !is_vertical(id) {
            self.goal_x = None;
        }
        let run = spec.run;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(self, params)));
        let result = match result {
            Ok(r) => r,
            Err(p) => {
                let msg = p.downcast_ref::<&str>().map(ToString::to_string).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_default();
                Err(CmdError::Failed(format!("{id} panicked: {msg}")))
            }
        };
        match &result {
            Ok(_) => self.finish_success(SuccessContext {
                spec,
                before_doc: &before_doc,
                pending_before: &pending_before,
                document_id_before,
                dirty_before,
                rev_restore,
                pushed,
                sel_before: &sel_before,
            }),
            Err(e) => {
                if let Some((d, s, h, r, t, ev)) = before_doc {
                    self.doc = d;
                    self.sel = s;
                    self.history = h;
                    self.redo = r;
                    self.typing_open = t;
                    // The steps the command pushed out are back, so they no longer count as gone.
                    self.undo_evicted = ev;
                    // Its step is gone with them: its undo group has none to merge into.
                    if pushed {
                        self.join_anchor = false;
                    }
                }
                if let Some(dirty) = dirty_before {
                    self.dirty = dirty;
                }
                if let Some(rev) = rev_restore {
                    self.restore_rev(rev);
                }
                self.status = e.to_string();
            }
        }
        result
    }

    fn finish_success(&mut self, context: SuccessContext<'_>) {
        let SuccessContext { spec, before_doc, pending_before, document_id_before, dirty_before, rev_restore, pushed, sel_before } = context;
        if spec.mutates {
            self.doc.ensure_nonempty();
            // Whether the command left the document, the selection and the formatting
            // waiting at the caret as they were. A command that replaces the document is not an
            // edit of this one, whatever the new document holds: `document_id` catches it
            // (through `set_document`, which starts a new history the old one must not
            // overwrite).
            let unchanged = |s: &Session| match (before_doc, pending_before) {
                (Some((doc, sel, ..)), Some(pending)) => s.document_id == document_id_before && s.unchanged(doc, sel, pending),
                _ => false,
            };
            if unchanged(self) {
                // A command that changed nothing is not an edit (#23): the step the dispatch
                // pushed is put back, so the redo stack survives and no step is evicted. The
                // group it belonged to is left without a step of its own, so the next command in
                // it starts one.
                if let Some((_, _, history, redo, typing_open, evicted)) = before_doc {
                    self.history = history.clone();
                    self.redo = redo.clone();
                    self.typing_open = *typing_open;
                    self.undo_evicted = *evicted;
                }
                if let Some(dirty) = dirty_before {
                    self.dirty = dirty;
                }
                if let Some(rev) = rev_restore {
                    self.restore_rev(rev);
                }
                if pushed {
                    self.join_anchor = false;
                }
            } else if self.document_id == document_id_before {
                self.touch();
            }
        }
        self.clamp_selection();
        // A selection command that moves the insertion point ends the typing group, like
        // caret movement: typing, Select, typing are two undo steps (#419).
        if !spec.mutates && self.sel != *sel_before {
            self.typing_open = false;
        }
    }
}
