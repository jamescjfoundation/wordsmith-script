// SPDX-License-Identifier: MIT OR Apache-2.0
//! Native file dialogs for the editor's file commands (Open, Save As, the exports): each dialog
//! the editor asks for ([`wordsmith_ui::editor::FileAsk`]) is shown over the window, and its answer
//! runs the command it was asked for.
//!
// core-candidate: the shell has no file dialogs yet. Every app needs them, so they belong in
// `jcj-shell` (over the dialogs the core's upstream layer already carries), answered on a worker
// as the old interface's were; until then the window shows them here, modally.

use wordsmith_ui::Editor;

/// Show the dialogs the editor asked for this frame over `parent`, and hand back each answer.
pub fn answer<W: rfd_handles::Handles>(editor: &mut Editor, parent: &W) {
    for ask in std::mem::take(&mut editor.file_asks) {
        let mut dialog = rfd::FileDialog::new().set_title(&ask.title).set_parent(parent);
        for (name, exts) in &ask.filters {
            let exts: Vec<&str> = exts.iter().map(String::as_str).collect();
            dialog = dialog.add_filter(name, &exts);
        }
        if let Some(name) = &ask.file_name {
            dialog = dialog.set_file_name(name);
        }
        let path = if ask.save { dialog.save_file() } else { dialog.pick_file() };
        editor.answer_file(ask.then, path);
    }
}

/// What a dialog's parent window must offer.
mod rfd_handles {
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

    /// A window a dialog can sit over.
    pub trait Handles: HasWindowHandle + HasDisplayHandle {}

    impl<T: HasWindowHandle + HasDisplayHandle> Handles for T {}
}
