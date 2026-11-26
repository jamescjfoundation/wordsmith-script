// SPDX-License-Identifier: MIT OR Apache-2.0
//! The xtask modules every app shares, under one name.
//!
//! An app's `xtask/src/main.rs` declares this module and nothing else of the core's:
//! `mod shared;`. The list of shared modules is here, so a core release that adds one needs no
//! edit in an app. The files sit beside this one in `xtask/src/`; a module names another as
//! `super::name`.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.
//! Change it in the core, never in an app.

// Each side uses a part: the core's own xtask the core's half of `sync` and `docs`, an app's the
// app's half and the gate.
#![allow(dead_code)]

#[path = "applayers.rs"]
pub mod applayers;
#[path = "assets.rs"]
pub mod assets;
#[path = "changed.rs"]
pub mod changed;
#[path = "corepin.rs"]
pub mod corepin;
#[path = "docs.rs"]
pub mod docs;
#[path = "gate.rs"]
pub mod gate;
#[path = "json.rs"]
pub mod json;
#[path = "licenses.rs"]
pub mod licenses;
#[path = "link.rs"]
pub mod link;
#[path = "lintroots.rs"]
pub mod lintroots;
#[path = "privacy.rs"]
pub mod privacy;
#[path = "spdx.rs"]
pub mod spdx;
#[path = "sync.rs"]
pub mod sync;
#[path = "util.rs"]
pub mod util;
