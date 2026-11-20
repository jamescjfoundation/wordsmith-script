// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screenplay layer for Wordsmith Script.
//!
//! Every screenplay rule from `docs/SPEC.md` lives here: the element registry and its paragraph
//! styles ([`element`], [`template`]), scene-heading settings ([`scene_heading`]), scenes derived
//! from Scene Headings ([`scenes`]), and, as M1 continues, the Return and Tab rules and the
//! screenplay commands.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod autocomplete;
pub mod bookmarks;
pub mod checker;
pub mod commands;
pub mod data;
pub mod element;
pub mod export;
pub mod header_footer;
pub mod locks;
pub mod notes;
pub mod outline;
pub mod paginate;
pub mod reports;
pub mod revisions;
pub mod scene_heading;
pub mod scene_numbers;
pub mod scenes;
pub mod spelling;
pub mod storyboard;
pub mod template;
pub mod title_page;
pub mod typing;

pub use commands::{new_session, registry, session_for};
pub use element::{Casing, ElementDef, ElementKind, Elements};
pub use scene_heading::SceneHeadingSettings;
pub use scenes::{Scene, scenes};
pub use template::new_script;

/// Product name shown to people.
pub const PRODUCT_NAME: &str = "Wordsmith Script";

/// Version of this crate, from Cargo.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests;
