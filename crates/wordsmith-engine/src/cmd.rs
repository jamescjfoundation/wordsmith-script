// Modified by the Wordsmith Script contributors.
//! Command implementations, one module per area. Each module exposes `specs()`.
//!
//! The engine registers only its own commands: undo and redo, and recovery. The screenplay
//! layer adds the rest with [`registry_with`].

use serde_json::{Value, json};
use wordsmith_doc::Pos;

use crate::{CmdError, CmdResult, CommandSpec, Registry, Session, p};

/// The engine's own registry: undo, redo and recovery.
pub fn registry() -> Registry {
    Registry::new(specs())
}

/// The engine's commands followed by `more` (the screenplay layer's). A later command with the
/// same id as an earlier one replaces it.
pub fn registry_with(more: Vec<CommandSpec>) -> Registry {
    let mut all = specs();
    for spec in more {
        all.retain(|s| s.id != spec.id);
        all.push(spec);
    }
    Registry::new(all)
}

/// The engine's own commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("edit.undo", "Undo", "Edit", |s, _| {
            s.undo();
            sel_result(s)
        })
        .key("Mod+Z")
        .pure(),
        CommandSpec::new("edit.undoTo", "Undo Actions", "Edit", undo_to)
            .params(r#"{"count": n (1 = the newest step, like edit.undo; n undoes the newest n steps at once; at most the undo depth)}"#)
            .when(|s| if s.can_undo() { None } else { Some("there is nothing to undo") })
            .pure(),
        CommandSpec::new("edit.undoHistory", "Undo History", "Edit", |s, _| {
            Ok(json!({"undo": s.undo_history(), "depth": s.undo_depth(), "redo": s.redo_label()}))
        })
        .pure(),
        CommandSpec::new("edit.redo", "Redo", "Edit", |s, _| {
            s.redo();
            sel_result(s)
        })
        .key("Mod+Y / Mod+Shift+Z")
        .pure(),
        CommandSpec::new("file.recover", "Recover Unsaved Scripts", "File", |s, v| s.recover(v))
            .params(r#"{"restore"?: recoveryId, "discard"?: recoveryId, "refresh"?: true}; desktop only; returns {enabled,busy,documents,error}; restore loads asynchronously without a file path"#)
            .pure(),
    ]
}

fn undo_to(s: &mut Session, v: &Value) -> CmdResult {
    let n = p::u64(v, "count").ok_or_else(|| CmdError::Params("`count` (whole number, 1 or more) is required".into()))?;
    if n == 0 {
        return Err(CmdError::Params("`count` must be 1 or more".into()));
    }
    let n = usize::try_from(n).unwrap_or(usize::MAX).min(s.undo_depth());
    let undone = s.undo_steps(n);
    let mut r = sel_result(s)?;
    if let Some(m) = r.as_object_mut() {
        m.insert("undone".into(), json!(undone));
        m.insert("depth".into(), json!(s.undo_depth()));
    }
    Ok(r)
}

pub fn pos_json(p: &Pos) -> Value {
    serde_json::to_value(p).unwrap_or(Value::Null)
}

/// The standard result: the selection after the command.
pub fn sel_result(s: &Session) -> CmdResult {
    Ok(json!({"anchor": pos_json(&s.sel.anchor), "focus": pos_json(&s.sel.focus)}))
}

/// ISO-8601 timestamp (UTC, seconds) for `secs` since the Unix epoch.
pub fn iso_from_unix_secs(secs: u64) -> String {
    use chrono::Datelike;
    let t = secs % 86_400;
    let hms = format!("{:02}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60);
    // Within chrono's range, read the date from it; past it, keep the proleptic Gregorian answer
    // `civil_from_days` gives rather than falling back to the epoch.
    let (y, m, d) = match i64::try_from(secs).ok().and_then(|s| chrono::DateTime::from_timestamp(s, 0)) {
        Some(dt) => (i64::from(dt.year()), dt.month(), dt.day()),
        None => civil_from_days((secs / 86_400) as i64),
    };
    // `{y:04}` rather than `%Y`, which writes a `+` before years past 9999.
    format!("{y:04}-{m:02}-{d:02}T{hms}Z")
}

/// Seconds since the Unix epoch (UTC): the system clock, or the browser's on the web.
pub fn now_unix() -> u64 {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }
    // `SystemTime::now()` panics on wasm32-unknown-unknown, so ask the browser's clock.
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        let ms = js_sys::Date::now();
        // Clamp a NaN or pre-epoch clock to 0 rather than fail.
        if ms.is_finite() && ms > 0.0 { (ms / 1000.0) as u64 } else { 0 }
    }
}

/// ISO-8601 timestamp (UTC, seconds).
pub fn now_iso() -> String {
    iso_from_unix_secs(now_unix())
}

/// Days since 1970-01-01 → (year, month, day).
///
/// `NaiveDate` only covers about ±262143 years, but this is reached by the system clock (and by
/// [`iso_from_unix_secs`] for a `u64` seconds count past it), so it must stay exact for every
/// `z`. The Gregorian calendar repeats every 400 years (146_097 days): chrono converts the day
/// within its cycle and whole cycles go back into the year (an i128 so i64::MIN/MAX can't
/// overflow). 1970-01-01 is `num_days_from_ce` 719_163, and 0..146_097 days from CE day 0 always
/// fit chrono. The result matches the proleptic Gregorian count the old hand-rolled code gave.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    use chrono::Datelike;
    let (cycles, rest) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    match i32::try_from(rest + 719_163).ok().and_then(chrono::NaiveDate::from_num_days_from_ce_opt) {
        Some(d) => {
            let y = i128::from(d.year()) + i128::from(cycles) * 400;
            (i64::try_from(y).unwrap_or(if y < 0 { i64::MIN } else { i64::MAX }), d.month(), d.day())
        }
        // Unreachable: `rest + 719_163` is 719_163..865_259, always a valid CE day.
        None => (1970, 1, 1),
    }
}

#[cfg(test)]
mod iso_tests {
    use super::{civil_from_days, iso_from_unix_secs};

    #[test]
    fn formats_epoch_and_known_timestamps() {
        assert_eq!(iso_from_unix_secs(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_from_unix_secs(1_760_082_785), "2025-10-10T07:53:05Z");
    }

    #[test]
    fn formats_leap_day_and_year_end() {
        assert_eq!(iso_from_unix_secs(1_709_164_800), "2024-02-29T00:00:00Z");
        assert_eq!(iso_from_unix_secs(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(iso_from_unix_secs(1_735_689_599), "2024-12-31T23:59:59Z");
    }

    #[test]
    fn formats_dates_outside_chronos_range() {
        // Year 10000 is still inside chrono's range: a plain zero-padded year, no leading `+`.
        assert_eq!(iso_from_unix_secs(253_402_300_800), "10000-01-01T00:00:00Z");
        // Past chrono's range, keep the proleptic Gregorian date instead of the Unix epoch.
        assert_eq!(iso_from_unix_secs(10_000_000_000_000), "318857-05-20T17:46:40Z");
    }

    #[test]
    fn civil_from_days_is_exact_beyond_chronos_range() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_734), (2026, 10, 8));
        // Past `NaiveDate`'s ±262143 the 400-year cycles carry the year; values from the
        // pre-chrono formula, both directions, both sides of the epoch.
        assert_eq!(civil_from_days(95_026_602), (262_144, 1, 1));
        assert_eq!(civil_from_days(99_000_000), (273_022, 10, 18));
        assert_eq!(civil_from_days(-96_465_964), (-262_145, 3, 1));
        assert_eq!(civil_from_days(-99_000_000), (-269_083, 3, 17));
        // The extremes must not overflow or panic (i128 intermediates).
        let _ = (civil_from_days(i64::MAX), civil_from_days(i64::MIN));
    }
}
