// Modified by the Wordsmith Script contributors.
//! Local desktop recovery. Encoding and file I/O run on a worker, outside typing.
use crate::{CmdError, CmdResult, Session};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryInfo {
    pub id: String,
    pub name: String,
    pub saved_at: String,
}

#[cfg(not(target_arch = "wasm32"))]
mod desktop;
#[cfg(not(target_arch = "wasm32"))]
use desktop::{Applied, Request};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use desktop::{DocRecovery, Recovery, Slot};

impl Session {
    /// The desktop host supplies the profile folder and a thread-safe wakeup.
    /// Headless sessions and the browser never write recovery files implicitly.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enable_recovery(&mut self, root: std::path::PathBuf, wake: std::sync::Arc<dyn Fn() + Send + Sync>) -> Result<(), String> {
        self.recovery = Some(Recovery::start(root, wake)?);
        Ok(())
    }
    /// Copies left by an earlier run (or by other windows' documents that have closed), newest
    /// first. The copy this session's document keeps is not listed: it is not lost.
    pub fn recovery_documents(&self) -> Vec<RecoveryInfo> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(r) = &self.recovery {
            let held = |id: &str| self.recovery_slot.id.as_deref() == Some(id) || r.closing().any(|c| c == id);
            return r.documents.iter().filter(|d| !held(&d.id)).cloned().collect();
        }
        Vec::new()
    }
    pub fn recovery_ready(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(r) = &self.recovery {
            return r.ready;
        }
        false
    }
    pub fn recovery_busy(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(r) = &self.recovery {
            return r.busy;
        }
        false
    }
    pub fn recovery_error(&self) -> Option<&str> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(r) = &self.recovery {
            return r.error.as_deref();
        }
        None
    }
    /// What recovery reads from the document, with its slot to update. Nothing is cloned or
    /// laid out.
    #[cfg(not(target_arch = "wasm32"))]
    fn recovery_doc(&mut self) -> DocRecovery<'_> {
        DocRecovery {
            id: self.document_id(),
            wants_copy: self.dirty,
            rev: self.rev(),
            doc: &self.doc,
            path: self.path.as_deref(),
            slot: &mut self.recovery_slot,
        }
    }
    /// Called every frame: takes in the worker's answer, opens a restored copy, and queues the
    /// next job. A frame compares the document's revision and timer, and clones the document
    /// only when its copy is due.
    pub fn recovery_tick(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(mut r) = self.recovery.take() else { return };
            if let Some(e) = r.receive(false) {
                self.apply_recovery(&mut r, e);
            }
            self.open_held_restore(&mut r);
            if let Err(e) = r.tick(std::iter::once(self.recovery_doc()), std::time::Instant::now()) {
                r.error = Some(e);
            }
            if let Some(e) = &r.error {
                self.status = e.clone();
            }
            self.recovery = Some(r);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn apply_recovery(&mut self, r: &mut Recovery, e: desktop::Event) {
        match r.apply(e) {
            // Opened by the next tick, never in the middle of a command.
            Applied::Loaded(snapshot, request) => r.held = Some((snapshot, request)),
            applied => {
                let d = self.recovery_doc();
                if let Applied::Failed { document, .. } = &applied
                    && *document != d.id
                {
                    return;
                }
                d.slot.apply(&applied);
            }
        }
    }
    /// A restored copy replaces the untouched blank document that asked for it, so no work is
    /// ever replaced. Otherwise it waits for the front end to open it in a window of its own
    /// ([`Session::take_restored_document`]). The restored document keeps its copy in the
    /// record it came from.
    #[cfg(not(target_arch = "wasm32"))]
    fn open_held_restore(&mut self, r: &mut Recovery) {
        let Some((_, request)) = &r.held else { return };
        if !request.replace {
            return;
        }
        let request = *request;
        let Some((snapshot, _)) = r.held.take() else { return };
        let id = snapshot.info.id.clone();
        if request.document == self.document_id() && request.revision == self.rev() {
            self.set_document(snapshot.document);
            self.path = None;
        } else {
            r.error = Some("Recovery cancelled: the document changed while the copy was loading.".into());
            return;
        }
        self.dirty = true;
        self.recovery_slot.adopt(id, self.rev());
    }
    /// A restored copy that could not replace this session's document (it was not an untouched
    /// blank one), for the front end to open in a new window. The new window's session takes over
    /// the copy's record with [`Session::adopt_recovery_record`].
    pub fn take_restored_document(&mut self) -> Option<(String, wordsmith_doc::Document)> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(r) = &mut self.recovery
            && r.held.as_ref().is_some_and(|(_, request)| !request.replace)
            && let Some((snapshot, _)) = r.held.take()
        {
            let snapshot = *snapshot;
            return Some((snapshot.info.id, snapshot.document));
        }
        None
    }
    /// This session's document now keeps its copy in record `id` (a copy restored into a new
    /// window).
    pub fn adopt_recovery_record(&mut self, id: String) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let rev = self.rev();
            self.dirty = true;
            self.recovery_slot.adopt(id, rev);
        }
        #[cfg(target_arch = "wasm32")]
        let _ = id;
    }
    pub fn recovery_wait(&self) -> Option<std::time::Duration> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(r) = &self.recovery {
            return r.wait(std::iter::once((self.dirty, self.rev(), &self.recovery_slot)));
        }
        None
    }
    /// Waits for recovery to finish anything already under way, then deletes the copies no
    /// open document needs: closed documents', and this one's if it was saved since its last
    /// copy. A dirty document keeps its copy. Desktop hosts call this right before the process
    /// exits, the same as they wait out pending print jobs: otherwise a save just before close
    /// can race the process exiting before a now-redundant recovery copy actually gets removed,
    /// so the next launch wrongly offers to recover a document that was in fact saved cleanly.
    pub fn finish_recovery(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(mut r) = self.recovery.take() {
            if let Some(e) = r.receive(true) {
                self.apply_recovery(&mut r, e);
            }
            let mut discards: Vec<(String, Option<u64>)> = r.take_closed().into_iter().map(|id| (id, None)).collect();
            if !self.dirty
                && let Some(id) = self.recovery_slot.id.clone()
            {
                discards.push((id, Some(self.document_id())));
            }
            for (id, document) in discards {
                match r.discard_now(&id, document) {
                    Ok(e) => self.apply_recovery(&mut r, e),
                    Err(e) => r.error = Some(e),
                }
            }
            if let Some(e) = &r.error {
                log::warn!("recovery: {e}");
            }
            self.recovery = Some(r);
        }
    }
    /// The user discarded the document's unsaved changes without closing it (Don't Save
    /// when quitting): its recovery copy goes too, and another is made only after a further edit.
    pub fn discard_recovery_copy(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(r) = &mut self.recovery
            && let Some(id) = self.recovery_slot.id.take()
        {
            r.close(id);
            self.recovery_slot.revision = Some(self.rev());
        }
    }
    /// An untouched blank document: untitled, unchanged, no undo steps and no text. A restored
    /// copy may replace it.
    pub fn is_untouched_blank(&self) -> bool {
        self.path.is_none()
            && !self.dirty
            && !self.can_undo()
            && !self.can_redo()
            && self.doc.plain_text(wordsmith_doc::StoryRef::Body).trim().is_empty()
            && self.doc.paragraph_count() <= 1
    }
    pub(crate) fn recover(&mut self, v: &Value) -> CmdResult {
        let restore = v.get("restore");
        let discard = v.get("discard");
        if restore.is_some() && discard.is_some() {
            return Err(CmdError::Params("choose restore or discard".into()));
        }
        if restore.is_some() || discard.is_some() || v.get("refresh").is_some() {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let request = Request { document: self.document_id(), revision: self.rev(), replace: self.is_untouched_blank() };
                // Only listed copies: never the one this document keeps.
                let listed = self.recovery_documents();
                let known = |id: Option<&str>, what: &str| -> Result<String, CmdError> {
                    let id = id.ok_or_else(|| CmdError::Params(format!("{what} must be a recovery id")))?;
                    if listed.iter().any(|d| d.id == id) { Ok(id.to_string()) } else { Err(CmdError::Failed("no such recovery copy".into())) }
                };
                let r = self.recovery.as_mut().ok_or_else(|| CmdError::Disabled("local recovery requires the desktop host".into()))?;
                if let Some(id) = restore {
                    r.restore(&known(id.as_str(), "restore")?, request).map_err(CmdError::Failed)?;
                } else if let Some(id) = discard {
                    r.discard(&known(id.as_str(), "discard")?).map_err(CmdError::Failed)?;
                } else if v.get("refresh").and_then(Value::as_bool) == Some(true) {
                    r.refresh().map_err(CmdError::Failed)?;
                } else {
                    return Err(CmdError::Params("refresh must be true".into()));
                }
            }
            #[cfg(target_arch = "wasm32")]
            return Err(CmdError::Disabled("local recovery is available on desktop only".into()));
        }
        Ok(
            json!({"enabled": self.recovery_ready(), "busy": self.recovery_busy(), "documents": self.recovery_documents(), "error": self.recovery_error()}),
        )
    }
}
