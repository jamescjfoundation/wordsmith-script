// SPDX-License-Identifier: MIT OR Apache-2.0
//! Files from the first build of Wordsmith Script (0.1.x) open in the app as rebuilt on
//! open-source-core (format 5) and come out the same: every one opens, saves in the new format,
//! reopens to the same script, and paginates exactly as 0.1.2 paginated it.
//!
//! `tests/fixtures/saved-by-0.1.2/` holds what Wordsmith Script 0.1.2 wrote (the last build
//! before the rebuild; never published, and its history was not kept: `docs/fixtures.md`) for
//! each golden script (`tests/golden/*.fountain`) and each format fixture (`tests/fixtures/v*.wss`):
//! the `.wss` file it saved, and its page plan (`.plan.txt`: whether the layout matched the plan,
//! each page's newest revision set, the page labels, and every page's pieces). The plans were
//! written by a test of 0.1.2's own code, as `format!("matches_plan={}\nrevs={:?}\nlabels={:?}\n{:#?}", ...)`.

use std::path::{Path, PathBuf};

use jcj_doc::Document;
use wordsmith_script_format::native;
use wordsmith_script_model::paginate::paginate;

const SAVED: [&str; 7] = ["everybodys-girl", "timid-teddy", "salt-of-vengeance", "v1-short", "v2-notes", "v3-revisions", "v4-styles"];

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/saved-by-0.1.2")
}

fn plan_of(doc: &Document) -> String {
    let pag = paginate(doc);
    format!("matches_plan={}\nrevs={:?}\nlabels={:?}\n{:#?}", pag.matches_plan, pag.page_revisions, pag.labels, pag.pages)
}

#[test]
fn every_0_1_file_opens_saves_reopens_and_paginates_identically() {
    let mut failures = Vec::new();
    for name in SAVED {
        let old = std::fs::read_to_string(dir().join(format!("{name}.wss"))).expect("0.1.2 file");
        assert_eq!(native::version_of(&old), Some(4), "{name}: saved by 0.1.2 in format 4");
        let opened = native::read(&old).unwrap_or_else(|e| panic!("{name}: {e}"));
        let saved = native::write(&opened).expect("saves");
        assert_eq!(native::version_of(&saved), Some(native::VERSION), "{name}: saved in the new format");
        let reopened = native::read(&saved).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(reopened.body, opened.body, "{name}: the same paragraphs after a save");
        assert_eq!(reopened.ext, opened.ext, "{name}: the same data after a save");
        assert_eq!(reopened.styles, opened.styles, "{name}: the same styles after a save");
        assert_eq!(native::write(&reopened).expect("saves"), saved, "{name}: saving again is stable");
        let want = std::fs::read_to_string(dir().join(format!("{name}.plan.txt"))).expect("0.1.2 plan");
        for (when, doc) in [("as opened", &opened), ("after a save", &reopened)] {
            let got = plan_of(doc);
            if got != want {
                let line = got.lines().zip(want.lines()).position(|(a, b)| a != b).unwrap_or(0);
                failures.push(format!(
                    "{name} {when}: pages differ from 0.1.2 at line {}\n  got:  {:?}\n  want: {:?}",
                    line + 1,
                    got.lines().nth(line),
                    want.lines().nth(line)
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn moved_settings_reach_the_core_keys() {
    let old = std::fs::read_to_string(dir().join("v2-notes.wss")).expect("0.1.2 file");
    assert!(old.contains("\"wordsmith.notes\""), "the fixture has notes under the old key");
    let doc = native::read(&old).expect("reads");
    assert!(doc.ext.contains("core.notes"), "notes are the core's now");
    assert!(!doc.ext.contains("wordsmith.notes"));
    assert_eq!(wordsmith_script_model::notes::Notes::of_doc(&doc).notes.len(), 1);
    let saved = native::write(&doc).expect("saves");
    assert!(saved.contains("\"ext\"") && !saved.contains("\"appData\""), "format 5 keeps extension data under `ext`");
}

#[test]
fn tracking_moves_to_the_documents_own_switch() {
    let old = std::fs::read_to_string(dir().join("v4-styles.wss")).expect("0.1.2 file");
    assert!(old.contains("\"wordsmith.tracking\": {\n      \"on\": false"), "the fixture has tracking off");
    let off = native::read(&old).expect("reads");
    assert!(!off.settings.track_changes);
    let on = old.replacen("\"wordsmith.tracking\": {\n      \"on\": false", "\"wordsmith.tracking\": {\n      \"on\": true", 1);
    let doc = native::read(&on).expect("reads");
    assert!(doc.settings.track_changes);
    assert!(!doc.ext.contains("wordsmith.tracking"));
    let back = native::read(&native::write(&doc).expect("saves")).expect("reads");
    assert!(back.settings.track_changes);
}
