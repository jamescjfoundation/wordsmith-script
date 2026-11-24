// SPDX-License-Identifier: MIT OR Apache-2.0
//! The time now, for tracked changes, notes and file names. The clock is the system's unless a
//! test (or a front end replaying a recording) fixes it for this thread with [`fix`].
//!
//! Before 0.2 Script kept the time in its own copy of the engine's session (`Session::now`); the
//! core's session has no clock of its own.

use std::cell::Cell;

thread_local! {
    static FIXED: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Unix seconds now: the fixed time, else the system clock.
pub fn now() -> u64 {
    FIXED.with(Cell::get).unwrap_or_else(jcj_engine::cmd::now_unix)
}

/// Fix the time this thread sees (`None`: the system clock again).
pub fn fix(at: Option<u64>) {
    FIXED.with(|c| c.set(at));
}
