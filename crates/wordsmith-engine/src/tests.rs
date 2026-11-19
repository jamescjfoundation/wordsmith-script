// SPDX-License-Identifier: MIT OR Apache-2.0
//! The run loop's guarantees, checked with small test commands. Upstream's engine tests drive
//! Word commands we did not copy, so these are new (see docs/decisions/0006-engine-trim.md).

use std::sync::Arc;

use serde_json::{Value, json};
use wordsmith_doc::props::CharProps;
use wordsmith_doc::{Document, StoryRef};

use crate::{CmdError, CmdResult, CommandSpec, Session, cmd, p};

/// Types `text` (default "x") at the caret, as typing does.
fn type_text(s: &mut Session, v: &Value) -> CmdResult {
    let text = p::str(v, "text").unwrap_or("x");
    let at = s.sel.focus.clone();
    let end = s.doc.insert_text(&at, text, &CharProps::default())?;
    s.sel = crate::Selection::caret(end);
    cmd::sel_result(s)
}

/// Appends "!" to the document, then fails.
fn edit_then_fail(s: &mut Session, _: &Value) -> CmdResult {
    let at = s.doc.end_of(StoryRef::Body);
    s.doc.insert_text(&at, "!", &CharProps::default())?;
    Err(CmdError::Failed("refused after editing".into()))
}

/// Appends "!" to the document, then panics.
fn edit_then_panic(s: &mut Session, _: &Value) -> CmdResult {
    let at = s.doc.end_of(StoryRef::Body);
    s.doc.insert_text(&at, "!", &CharProps::default())?;
    panic!("boom")
}

fn test_registry() -> Arc<crate::Registry> {
    Arc::new(cmd::registry_with(vec![
        CommandSpec::new("text.insert", "Typing", "Test", type_text),
        CommandSpec::new("test.append", "Append", "Test", |s, v| {
            let at = s.doc.end_of(StoryRef::Body);
            s.doc.insert_text(&at, p::str(v, "text").unwrap_or("+"), &CharProps::default())?;
            Ok(Value::Null)
        }),
        CommandSpec::new("test.noop", "Nothing", "Test", |_, _| Ok(Value::Null)),
        CommandSpec::new("test.fail", "Fail", "Test", edit_then_fail),
        CommandSpec::new("test.panic", "Panic", "Test", edit_then_panic),
        CommandSpec::new("test.disabled", "Disabled", "Test", |_, _| Ok(Value::Null)).when(|_| Some("never")),
        CommandSpec::new("caret.home", "Home", "Test", |s, _| {
            s.sel = crate::Selection::caret(s.doc.start_of(StoryRef::Body));
            Ok(Value::Null)
        })
        .pure(),
    ]))
}

fn session(text: &str) -> Session {
    let mut s = Session::with_registry(Document::from_text(text), test_registry());
    s.sel = crate::Selection::caret(s.doc.end_of(StoryRef::Body));
    s
}

fn body(s: &Session) -> String {
    s.doc.plain_text(StoryRef::Body)
}

#[test]
fn registry_lists_engine_commands_and_finds_shortcuts() {
    let r = cmd::registry();
    for id in ["edit.undo", "edit.redo", "edit.undoTo", "edit.undoHistory", "file.recover"] {
        assert!(r.get(id).is_some(), "{id} is registered");
    }
    assert_eq!(r.by_shortcut("mod+z").map(|s| s.id), Some("edit.undo"));
    assert_eq!(r.by_shortcut("Mod+Shift+Z").map(|s| s.id), Some("edit.redo"));
    assert_eq!(r.describe().as_array().map(Vec::len), Some(r.all().len()));
}

#[test]
fn a_later_spec_replaces_an_engine_command_with_the_same_id() {
    let r = cmd::registry_with(vec![CommandSpec::new("edit.undo", "Mine", "Edit", |_, _| Ok(Value::Null))]);
    assert_eq!(r.get("edit.undo").map(|s| s.label), Some("Mine"));
    assert_eq!(r.all().iter().filter(|s| s.id == "edit.undo").count(), 1);
}

#[test]
fn unknown_and_disabled_commands_change_nothing() {
    let mut s = session("Hello");
    assert_eq!(s.run("no.such", &json!({})), Err(CmdError::Unknown("no.such".into())));
    assert!(matches!(s.run("test.disabled", &json!({})), Err(CmdError::Disabled(_))));
    assert!(!s.can_undo());
    assert!(!s.dirty);
}

#[test]
fn an_edit_is_one_undo_step_and_undo_redo_round_trip() {
    let mut s = session("Hello");
    s.run("test.append", &json!({"text": " world"})).unwrap();
    assert_eq!(body(&s), "Hello world");
    assert!(s.dirty);
    assert_eq!(s.undo_label(), Some("Append"));
    s.run("edit.undo", &json!({})).unwrap();
    assert_eq!(body(&s), "Hello");
    assert_eq!(s.redo_label(), Some("Append"));
    s.run("edit.redo", &json!({})).unwrap();
    assert_eq!(body(&s), "Hello world");
}

#[test]
fn typing_collapses_into_one_step_until_the_caret_moves() {
    let mut s = session("");
    for c in ["a", "b", "c"] {
        s.run("text.insert", &json!({"text": c})).unwrap();
    }
    assert_eq!(body(&s), "abc");
    assert_eq!(s.undo_depth(), 1);
    assert_eq!(s.undo_label(), Some("Typing"));
    s.run("caret.home", &json!({})).unwrap();
    s.run("text.insert", &json!({"text": "X"})).unwrap();
    assert_eq!(s.undo_depth(), 2);
    s.run("edit.undo", &json!({})).unwrap();
    assert_eq!(body(&s), "abc");
    s.run("edit.undo", &json!({})).unwrap();
    assert_eq!(body(&s), "");
}

#[test]
fn a_failed_command_leaves_no_trace() {
    let mut s = session("Hello");
    s.run("test.append", &json!({})).unwrap();
    s.run("edit.undo", &json!({})).unwrap();
    let rev = s.rev();
    let dirty = s.dirty;
    let err = s.run("test.fail", &json!({}));
    assert_eq!(err, Err(CmdError::Failed("refused after editing".into())));
    assert_eq!(body(&s), "Hello");
    assert_eq!(s.rev(), rev);
    assert_eq!(s.dirty, dirty);
    assert_eq!(s.undo_depth(), 0);
    // The redo stack survives a failed command.
    assert_eq!(s.redo_label(), Some("Append"));
    assert_eq!(s.status, "refused after editing");
}

#[test]
fn a_panicking_command_is_an_error_and_is_rolled_back() {
    let mut s = session("Hello");
    let err = s.run("test.panic", &json!({}));
    assert!(matches!(&err, Err(CmdError::Failed(m)) if m.contains("test.panic panicked: boom")), "{err:?}");
    assert_eq!(body(&s), "Hello");
    assert!(!s.can_undo());
    // The session still works afterwards.
    s.run("test.append", &json!({})).unwrap();
    assert_eq!(body(&s), "Hello+");
}

#[test]
fn a_command_that_changes_nothing_is_not_an_edit() {
    let mut s = session("Hello");
    s.run("test.append", &json!({})).unwrap();
    s.run("edit.undo", &json!({})).unwrap();
    s.dirty = false;
    let rev = s.rev();
    s.run("test.noop", &json!({})).unwrap();
    assert!(!s.dirty);
    assert_eq!(s.rev(), rev);
    assert!(!s.can_undo());
    assert_eq!(s.redo_label(), Some("Append"), "a no-op keeps the redo stack");
}

#[test]
fn joined_commands_undo_as_one_step() {
    let mut s = session("");
    s.run("test.append", &json!({"text": "1"})).unwrap();
    s.join_next_undo();
    s.run("test.append", &json!({"text": "2"})).unwrap();
    s.join_next_undo();
    s.run("test.append", &json!({"text": "3"})).unwrap();
    assert_eq!(body(&s), "123");
    assert_eq!(s.undo_depth(), 1);
    s.run("edit.undo", &json!({})).unwrap();
    assert_eq!(body(&s), "");
}

#[test]
fn undo_to_takes_several_steps_and_checks_its_count() {
    let mut s = session("");
    for t in ["a", "b", "c"] {
        s.run("test.append", &json!({"text": t})).unwrap();
    }
    assert_eq!(s.undo_history(), vec!["Append", "Append", "Append"]);
    let r = s.run("edit.undoTo", &json!({"count": 2})).unwrap();
    assert_eq!(r["undone"], 2);
    assert_eq!(body(&s), "a");
    assert!(matches!(s.run("edit.undoTo", &json!({"count": 0})), Err(CmdError::Params(_))));
    assert!(matches!(s.run("edit.undoTo", &json!({})), Err(CmdError::Params(_))));
    // More than the depth undoes everything there is.
    let r = s.run("edit.undoTo", &json!({"count": u64::MAX})).unwrap();
    assert_eq!(r["undone"], 1);
    assert!(matches!(s.run("edit.undoTo", &json!({"count": 1})), Err(CmdError::Disabled(_))));
}

#[test]
fn the_undo_stack_is_bounded() {
    let mut s = session("");
    for _ in 0..(crate::doc_state::MAX_UNDO + 10) {
        s.run("test.append", &json!({"text": "."})).unwrap();
    }
    assert_eq!(s.undo_depth(), crate::doc_state::MAX_UNDO);
    assert_eq!(s.undo_steps_so_far(), (crate::doc_state::MAX_UNDO + 10) as u64);
}

#[test]
fn set_document_starts_a_new_history() {
    let mut s = session("Hello");
    s.run("test.append", &json!({})).unwrap();
    let id = s.document_id();
    s.set_document(Document::from_text("Fresh"));
    assert_ne!(s.document_id(), id);
    assert!(!s.can_undo() && !s.can_redo());
    assert!(!s.dirty);
    assert_eq!(body(&s), "Fresh");
}

#[test]
fn the_layout_is_cached_per_revision_and_view() {
    let mut s = session("Hello");
    let a = s.layout();
    let b = s.layout();
    assert!(Arc::ptr_eq(&a, &b), "same revision and view: same layout");
    s.run("test.append", &json!({})).unwrap();
    let c = s.layout();
    assert!(!Arc::ptr_eq(&b, &c), "an edit lays out again");
    s.view_mode = wordsmith_layout::ViewMode::Draft;
    let d = s.layout();
    s.view_mode = wordsmith_layout::ViewMode::Print;
    let e = s.layout();
    assert!(Arc::ptr_eq(&c, &e), "the other view's layout is kept");
    assert!(!Arc::ptr_eq(&d, &e));
}

#[test]
fn ui_requests_are_bounded() {
    let mut s = session("");
    for i in 0..(crate::doc_state::MAX_UI_REQUESTS + 5) {
        s.push_ui_request(json!(i));
    }
    assert_eq!(s.ui_requests.len(), crate::doc_state::MAX_UI_REQUESTS);
    assert_eq!(s.ui_requests.first(), Some(&json!(5)));
}

#[test]
fn recover_without_a_desktop_host_reports_disabled() {
    let mut s = session("");
    let r = s.run("file.recover", &json!({})).unwrap();
    assert_eq!(r["enabled"], false);
    assert!(matches!(s.run("file.recover", &json!({"refresh": true})), Err(CmdError::Disabled(_))));
    assert!(matches!(s.run("file.recover", &json!({"restore": "a", "discard": "b"})), Err(CmdError::Params(_))));
}
