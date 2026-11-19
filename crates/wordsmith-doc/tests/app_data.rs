// SPDX-License-Identifier: MIT OR Apache-2.0
//! `Document::app_data`: application data that lives in the document, so undo, equality and
//! saving all see it. Added by Wordsmith Script.

use std::sync::Arc;

use serde_json::json;
use wordsmith_doc::Document;

#[test]
fn app_data_counts_as_content() {
    let a = Document::from_text("Same text");
    let mut b = a.clone();
    assert!(a.same_content(&b));
    b.app_data.insert("wordsmith.elements".into(), Arc::new(json!({"version": 1})));
    assert!(!a.same_content(&b), "a change to app data is a change to the document");
    assert_ne!(a, b);
    let c = b.clone();
    assert!(b.same_content(&c));
    assert!(Arc::ptr_eq(&b.app_data["wordsmith.elements"], &c.app_data["wordsmith.elements"]), "cloning shares the values");
}

#[test]
fn app_data_round_trips_and_is_left_out_when_empty() {
    let plain = Document::from_text("x");
    let text = serde_json::to_string(&plain).unwrap();
    assert!(!text.contains("appData"), "an empty map is not written");

    let mut doc = Document::from_text("x");
    doc.app_data.insert("k".into(), Arc::new(json!({"list": [1, 2, 3], "name": "INT."})));
    let text = serde_json::to_string(&doc).unwrap();
    assert!(text.contains("appData"));
    let back: Document = serde_json::from_str(&text).unwrap();
    assert!(back.same_content(&doc));
    assert_eq!(back.app_data["k"]["name"], "INT.");
}

#[test]
fn a_document_without_app_data_still_reads() {
    let doc = Document::from_text("old file");
    let mut v = serde_json::to_value(&doc).unwrap();
    v.as_object_mut().unwrap().remove("appData");
    let back: Document = serde_json::from_value(v).unwrap();
    assert!(back.app_data.is_empty());
}
