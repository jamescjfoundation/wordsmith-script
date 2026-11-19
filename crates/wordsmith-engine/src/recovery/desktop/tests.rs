// Modified by the Wordsmith Script contributors.
use super::*;
use crate::Session;
use serde_json::json;
use wordsmith_doc::StoryRef;

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("wordsmith-recovery-test-{}", new_id().unwrap()));
        private_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
/// A store reopened on `root` whose scan reports an unreadable record. The record's lock can be
/// briefly held by a process another test is starting (a forked child holds the descriptor
/// until it execs), and a record that can't be locked is skipped, not reported: so try again
/// for up to a second rather than read a skipped record as a clean scan.
fn reopen_reporting_unreadable(root: &std::path::Path) -> Store {
    for _ in 0..50 {
        let mut store = Store::new(root.to_path_buf()).unwrap();
        if store.scan().is_err() {
            return store;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("the scan never reported the unreadable record");
}
/// `store.load(id)`, tried again for up to a second while the record can't be locked: a process
/// another test is starting holds the lock for a moment (see [`reopen_reporting_unreadable`]).
fn load_retrying(store: &mut Store, id: &str) -> Snapshot {
    let mut last = String::new();
    for _ in 0..50 {
        match store.load(id) {
            Ok(snapshot) => return snapshot,
            Err(e) => last = e,
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("the record never loaded: {last}");
}
fn snapshot(id: &str, text: &str) -> Snapshot {
    Snapshot {
        version: 1,
        info: RecoveryInfo { id: id.into(), name: "Synthetic draft".into(), saved_at: crate::cmd::now_iso() },
        generation: 0,
        document: Document::from_text(text),
    }
}
fn settle(s: &mut Session) {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        s.recovery_tick();
        if s.recovery_ready() && !s.recovery_busy() {
            break;
        }
        assert!(Instant::now() < end, "recovery did not finish");
        std::thread::sleep(Duration::from_millis(2));
    }
}
/// The commands these tests drive: replace the body text, and type at the caret. The
/// screenplay layer has its own; the engine registers neither.
fn test_registry() -> Arc<crate::Registry> {
    use crate::{CommandSpec, p};
    use wordsmith_doc::props::CharProps;
    Arc::new(crate::cmd::registry_with(vec![
        CommandSpec::new("document.setText", "Set Text", "Test", |s, v| {
            let doc = Document::from_text(p::str(v, "text").unwrap_or(""));
            s.doc.body = doc.body;
            s.clamp_selection();
            Ok(serde_json::Value::Null)
        }),
        CommandSpec::new("text.insert", "Typing", "Test", |s, v| {
            let at = s.sel.focus.clone();
            let end = s.doc.insert_text(&at, p::str(v, "text").unwrap_or(""), &CharProps::default())?;
            s.sel = crate::Selection::caret(end);
            Ok(serde_json::Value::Null)
        }),
    ]))
}
fn session(root: &Path) -> Session {
    let mut s = Session::with_registry(Document::new(), test_registry());
    s.enable_recovery(root.to_owned(), Arc::new(|| {})).unwrap();
    settle(&mut s);
    s
}
/// The document's interval has passed.
fn due_now(s: &mut Session) {
    s.recovery_slot.due = Instant::now();
}
fn save_now(s: &mut Session) {
    due_now(s);
    s.recovery_tick();
    settle(s);
}
/// The records in the profile folder: the copies on disk.
fn records(root: &Path) -> Vec<String> {
    let mut ids: Vec<String> = fs::read_dir(root)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    ids.sort();
    ids
}

/// #3140: a save that lands right before the app closes can race the process exiting before
/// the now-redundant recovery copy is actually removed, so the next launch wrongly offers to
/// recover a document that was in fact saved cleanly. `finish_recovery` (desktop hosts call it
/// on exit, the same as they wait for pending print jobs) must discard it synchronously
/// instead, with no regular tick in between — simulating closing right after Save, not after
/// however long it'd take the next normal autosave tick to notice and clean up.
#[test]
fn finish_recovery_discards_its_own_copy_for_a_document_saved_right_before_exit() {
    let profile = Temp::new();
    let mut s = session(&profile.0);
    s.run("document.setText", &json!({"text": "About to be saved"})).unwrap();
    save_now(&mut s);
    assert_eq!(records(&profile.0).len(), 1, "a recovery copy exists for the dirty document");
    // A real Save does this, then the app can close on the very next frame, with no regular
    // recovery_tick() in between to notice `!s.dirty` and queue the discard itself.
    s.dirty = false;
    assert_eq!(records(&profile.0).len(), 1, "confirms the race: setting dirty=false alone (what Save does) doesn't discard anything by itself");
    s.finish_recovery();
    assert!(records(&profile.0).is_empty(), "finish_recovery should have discarded the now-redundant copy before returning");
}

/// `tick` resets `self.id` the moment the session's document no longer matches what it last
/// saw (so it never discards under a stale identity); `finish_recovery` must guard the same
/// way. Switching documents and exiting immediately, with no regular tick in between to reset
/// that identity, must not let the *new* (clean, blank) document's state discard the
/// *previous* document's still-live recovery copy.
#[test]
fn finish_recovery_does_not_discard_a_previous_document_s_copy_after_switching() {
    let profile = Temp::new();
    let mut s = session(&profile.0);
    s.run("document.setText", &json!({"text": "The document the user is still working on"})).unwrap();
    save_now(&mut s);
    assert_eq!(records(&profile.0).len(), 1, "a recovery copy exists for the dirty document");
    s.set_document(Document::new()); // the document is replaced, with no tick() in between
    assert!(!s.dirty, "a freshly switched-to document starts clean");
    s.finish_recovery();
    assert_eq!(records(&profile.0).len(), 1, "replacing the document must not discard the previous one's recovery copy");
    assert_eq!(s.recovery_documents().len(), 1, "no open document keeps it: it is listed");
}

#[test]
fn recovery_commands_only_address_known_profile_records() {
    use crate::file_access::{AutomationRoots, confine};
    let profile = Temp::new();
    let files = Temp::new();
    let original = files.0.join("original.txt");
    fs::write(&original, b"External synthetic original").unwrap();
    let roots = AutomationRoots::new(Some(&files.0), Some(&files.0)).unwrap().unwrap();
    let mut s = session(&profile.0);
    s.run("document.setText", &json!({"text": "Recoverable synthetic draft"})).unwrap();
    save_now(&mut s);
    drop(s);
    let mut s = session(&profile.0);
    let id = s.recovery_documents()[0].id.clone();
    confine(Some(&roots), || {
        // A command cannot use a file path or choose a different storage folder.
        for bad in [original.to_string_lossy().into_owned(), "../original.txt".into()] {
            for action in ["restore", "discard"] {
                let err = s.run("file.recover", &json!({action: bad, "root": files.0})).unwrap_err();
                assert!(err.to_string().contains("no such recovery copy"), "{err}");
            }
        }
        assert_eq!(s.recovery_documents().len(), 1);
        s.run("file.recover", &json!({"restore": id})).unwrap();
        settle(&mut s);
        assert!(s.path.is_none());
        assert!(s.dirty);
        assert_eq!(s.doc.plain_text(StoryRef::Body), "Recoverable synthetic draft");
        // The restored document keeps its copy there: it isn't listed, and can't be discarded.
        assert!(s.recovery_documents().is_empty());
        let err = s.run("file.recover", &json!({"discard": id})).unwrap_err();
        assert!(err.to_string().contains("no such recovery copy"), "{err}");
        settle(&mut s);
        assert_eq!(records(&profile.0), std::slice::from_ref(&id));
    });
    assert_eq!(fs::read(&original).unwrap(), b"External synthetic original");
    assert_eq!(fs::read_dir(&files.0).unwrap().count(), 1);
}

#[test]
fn failed_write_keeps_last_valid_generation_and_discovery_falls_back() {
    let t = Temp::new();
    let id = new_id().unwrap();
    let mut store = Store::new(t.0.clone()).unwrap();
    store.save(snapshot(&id, "First valid copy")).unwrap();
    let first = store.entries[&id].path.clone();
    let bytes = fs::read(&first).unwrap();
    // Simulate a final rename failure without relying on permissions (root can bypass them).
    let obstruction = t.0.join(&id).join("snapshot-00000000000000000002.json");
    fs::create_dir(&obstruction).unwrap();
    assert!(store.save(snapshot(&id, "Failed copy")).is_err());
    assert_eq!(fs::read(&first).unwrap(), bytes);
    fs::remove_dir(obstruction).unwrap();
    store.save(snapshot(&id, "Second valid copy")).unwrap();
    let second = store.entries[&id].path.clone();
    fs::write(second, b"{truncated").unwrap();
    drop(store);
    let mut reopened = reopen_reporting_unreadable(&t.0);
    assert_eq!(load_retrying(&mut reopened, &id).document.plain_text(StoryRef::Body), "First valid copy");
    // Never reuse the corrupt generation's name or prune the valid fallback.
    reopened.save(snapshot(&id, "Third valid copy")).unwrap();
    assert_eq!(fs::read(first).unwrap(), bytes);
    assert_eq!(Store::files(&t.0.join(&id)).unwrap().len(), 2);
}

#[test]
fn size_limit_and_lossy_values_fail_without_committing() {
    let mut out = Limited { inner: Vec::new(), remaining: 3, exceeded: false };
    assert!(out.write_all(b"four").is_err());
    assert!(out.inner.is_empty());
    assert!(out.exceeded);
    let t = Temp::new();
    let id = new_id().unwrap();
    let mut store = Store::new(t.0.clone()).unwrap();
    store.save(snapshot(&id, "valid")).unwrap();
    let first = store.entries[&id].path.clone();
    let bytes = fs::read(&first).unwrap();
    let mut invalid = snapshot(&id, "invalid");
    invalid.document.last_section.page_w = f32::NAN;
    assert!(store.save(invalid).is_err_and(|e| e.permanent));
    assert_eq!(fs::read(first).unwrap(), bytes);
    assert_eq!(Store::files(&t.0.join(id)).unwrap().len(), 1);
}

/// Copies are written with picture bytes in base64 (#586 follow-up); those written before kept
/// them as arrays of numbers and are still listed and restored, bytes intact.
#[test]
fn copies_with_byte_arrays_written_before_still_restore() {
    let t = Temp::new();
    let id = new_id().unwrap();
    let mut s = snapshot(&id, "Synthetic draft with a picture");
    s.generation = 1;
    s.document.media.insert("media/image1.png".into(), Arc::new(vec![0, 1, 127, 128, 254, 255]));
    let mut value = serde_json::to_value(&s).unwrap();
    assert_eq!(value["document"]["media"]["media/image1.png"], "AAF/gP7/");
    value["document"]["media"]["media/image1.png"] = json!([0, 1, 127, 128, 254, 255]);
    private_dir(&t.0.join(&id)).unwrap();
    fs::write(t.0.join(&id).join("snapshot-00000000000000000001.json"), serde_json::to_vec(&value).unwrap()).unwrap();
    let mut store = Store::new(t.0.clone()).unwrap();
    store.scan().unwrap();
    assert_eq!(store.inventory().len(), 1);
    assert_eq!(store.load(&id).unwrap().document, s.document);
}

#[test]
fn snapshot_that_cannot_be_copied_is_not_retried_until_the_document_changes() {
    // #2625: a failure caused by the snapshot itself (here a value the format cannot keep;
    // in the field, the 256 MiB limit) used to be rewritten every interval while dirty.
    let t = Temp::new();
    let mut s = session(&t.0);
    s.run("document.setText", &json!({"text": "Synthetic draft"})).unwrap();
    s.doc.last_section.page_w = f32::NAN;
    save_now(&mut s);
    assert!(s.recovery_error().is_some());
    assert!(s.recovery_wait().is_none());
    due_now(&mut s);
    s.recovery_tick();
    assert!(!s.recovery_busy(), "the unchanged document was snapshotted again");
    assert!(s.recovery_error().is_some());
    // An edit tries again.
    s.doc.last_section.page_w = 612.0;
    s.run("text.insert", &json!({"text": "!"})).unwrap();
    save_now(&mut s);
    assert!(s.recovery_error().is_none());
    assert_eq!(records(&t.0).len(), 1);
}

#[test]
fn failed_pruning_cannot_push_a_record_beyond_the_discovery_limit() {
    let t = Temp::new();
    let id = new_id().unwrap();
    let mut store = Store::new(t.0.clone()).unwrap();
    store.save(snapshot(&id, "Last valid copy")).unwrap();
    let first = store.entries[&id].path.clone();
    let bytes = fs::read(&first).unwrap();
    // A failed cleanup must not allow the next write to exceed the reader's bound.
    for n in 2..=MAX_FILES {
        fs::write(t.0.join(&id).join(format!("snapshot-{n:020}.json")), b"{truncated").unwrap();
    }
    assert!(store.save(snapshot(&id, "New copy")).is_err());
    assert_eq!(fs::read(first).unwrap(), bytes);
    drop(store);
    let mut reopened = reopen_reporting_unreadable(&t.0);
    assert_eq!(load_retrying(&mut reopened, &id).document.plain_text(StoryRef::Body), "Last valid copy");
}

#[test]
fn failed_first_write_does_not_consume_a_record_slot() {
    let t = Temp::new();
    let mut store = Store::new(t.0.clone()).unwrap();
    let mut invalid = snapshot(&new_id().unwrap(), "Synthetic draft");
    invalid.document.last_section.page_w = f32::NAN;
    assert!(store.save(invalid).is_err());
    assert!(store.directories().unwrap().is_empty());
}

#[test]
fn discovery_reclaims_a_crashed_first_write_but_keeps_valid_records() {
    let t = Temp::new();
    let id = new_id().unwrap();
    let empty = new_id().unwrap();
    let mut store = Store::new(t.0.clone()).unwrap();
    store.save(snapshot(&id, "Valid synthetic draft")).unwrap();
    drop(store);
    let dir = t.0.join(&empty);
    private_dir(&dir).unwrap();
    fs::write(dir.join(format!("pending-{}.tmp", new_id().unwrap())), b"{unfinished").unwrap();
    let mut reopened = Store::new(t.0.clone()).unwrap();
    reopened.scan().unwrap();
    assert!(!dir.exists());
    assert_eq!(load_retrying(&mut reopened, &id).document.plain_text(StoryRef::Body), "Valid synthetic draft");
}

#[test]
fn live_records_are_locked_and_discard_releases_storage() {
    let t = Temp::new();
    let id = new_id().unwrap();
    let mut owner = Store::new(t.0.clone()).unwrap();
    owner.save(snapshot(&id, "live")).unwrap();
    let mut other = Store::new(t.0.clone()).unwrap();
    other.scan().unwrap();
    assert!(other.inventory().is_empty());
    assert!(other.discard(&id).is_err());
    assert!(other.discard("../outside").is_err());
    drop(owner);
    other.scan().unwrap();
    assert_eq!(other.inventory().len(), 1);
    other.discard(&id).unwrap();
    assert!(!t.0.join(id).exists(), "discard must release the record slot");
}

#[test]
fn new_draft_restarts_restores_unsaved_and_explicitly_discards() {
    let t = Temp::new();
    let mut s = session(&t.0);
    s.run("text.insert", &json!({"text": "Unsaved synthetic draft"})).unwrap();
    save_now(&mut s);
    assert!(s.recovery_wait().is_none(), "unchanged documents must not schedule idle work");
    let expected = s.doc.clone();
    drop(s);
    let mut s = session(&t.0);
    let id = s.recovery_documents()[0].id.clone();
    s.run("file.recover", &json!({"restore": id})).unwrap();
    settle(&mut s);
    assert_eq!(s.doc, expected);
    assert!(s.dirty && s.path.is_none());
    assert!(s.recovery_documents().is_empty(), "the restored document keeps the copy");
    // A crash leaves the restored document's copy, in the same record.
    drop(s);
    let mut s = session(&t.0);
    assert_eq!(s.recovery_documents().len(), 1);
    s.run("file.recover", &json!({"discard": id})).unwrap();
    settle(&mut s);
    save_now(&mut s);
    assert!(s.recovery_documents().is_empty());
    assert!(records(&t.0).is_empty());
}

#[test]
fn delayed_restore_cannot_replace_new_edits() {
    let t = Temp::new();
    let mut s = session(&t.0);
    s.run("text.insert", &json!({"text": "Saved draft"})).unwrap();
    save_now(&mut s);
    drop(s);
    let mut s = session(&t.0);
    let id = s.recovery_documents()[0].id.clone();
    s.run("file.recover", &json!({"restore": id})).unwrap();
    s.run("text.insert", &json!({"text": "New work"})).unwrap();
    settle(&mut s);
    assert_eq!(s.doc.plain_text(StoryRef::Body), "New work");
    assert!(s.recovery_error().unwrap().contains("document changed"));
}

/// Removing a record's emptied folder waits out a file Windows hasn't finished deleting (another
/// process, such as an antivirus scanner, still has it open, so the folder isn't empty yet: os
/// error 145), which made saving with a password, and discarding a closed document's copy, fail
/// now and then. It doesn't wait on other errors, nor forever.
#[test]
fn removing_a_record_folder_waits_for_files_still_being_deleted() {
    let not_empty = || io::Error::from(io::ErrorKind::DirectoryNotEmpty);
    let mut pending = 3;
    let removed = remove_record_dir(Path::new("record"), |_| {
        if pending > 0 {
            pending -= 1;
            Err(not_empty())
        } else {
            Ok(())
        }
    });
    assert!(removed.is_ok(), "{removed:?}");
    assert_eq!(pending, 0);
    let mut calls = 0;
    let stuck = remove_record_dir(Path::new("record"), |_| {
        calls += 1;
        Err(not_empty())
    });
    assert_eq!(stuck.map_err(|e| e.kind()), Err(io::ErrorKind::DirectoryNotEmpty));
    assert_eq!(calls, REMOVE_RETRIES + 1);
    let mut calls = 0;
    let denied = remove_record_dir(Path::new("record"), |_| {
        calls += 1;
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    });
    assert_eq!(denied.map_err(|e| e.kind()), Err(io::ErrorKind::PermissionDenied));
    assert_eq!(calls, 1);
}
