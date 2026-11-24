// SPDX-License-Identifier: MIT OR Apache-2.0
//! Calendar dates for Unix times, and back. Every conversion goes through chrono (the engine's
//! `civil_from_days`, which stays exact past chrono's range), so there is one calendar in the app.
//! All dates are UTC.

use wordsmith_engine::cmd::civil_from_days;

/// Days from 1970-01-01 to 0001-01-01's count in chrono (`num_days_from_ce`).
const UNIX_EPOCH_CE_DAYS: i64 = 719_163;

/// `secs` since 1970 as (year, month, day, hour, minute, second), UTC.
pub fn civil(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let (y, m, d) = civil_from_days((secs / 86_400) as i64);
    let rem = secs % 86_400;
    (y, m, d, (rem / 3600) as u32, (rem / 60 % 60) as u32, (rem % 60) as u32)
}

/// `secs` since 1970 as `YYYY-MM-DD`, UTC.
pub fn ymd(secs: u64) -> String {
    let (y, m, d, ..) = civil(secs);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Days since 1970-01-01 for a `YYYY-MM-DD` date, if it is a real date.
pub fn day_number(date: &str) -> Option<i64> {
    let mut it = date.split('-');
    let (y, m, d) = (it.next()?.parse::<i32>().ok()?, it.next()?.parse::<u32>().ok()?, it.next()?.parse::<u32>().ok()?);
    if it.next().is_some() {
        return None;
    }
    use chrono::Datelike;
    chrono::NaiveDate::from_ymd_opt(y, m, d).map(|n| i64::from(n.num_days_from_ce()) - UNIX_EPOCH_CE_DAYS)
}

/// The English name of month `m` (1 to 12), or empty.
pub fn month_name(m: u32) -> &'static str {
    const MONTHS: [&str; 12] =
        ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    m.checked_sub(1).and_then(|i| MONTHS.get(i as usize)).copied().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_back() {
        assert_eq!(civil(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(civil(1_795_100_602), (2026, 11, 19, 15, 3, 22));
        assert_eq!(ymd(951_782_400), "2000-02-29");
        assert_eq!(day_number("1970-01-01"), Some(0));
        assert_eq!(day_number("2000-03-01").zip(day_number("2000-02-28")).map(|(a, b)| a - b), Some(2));
        assert_eq!(day_number("2026-02-30"), None);
        assert_eq!(day_number("nope"), None);
        assert_eq!(day_number("2026-01-01-01"), None);
        for secs in [0, 86_399, 86_400, 951_782_400, 1_795_100_602, 4_102_444_800] {
            assert_eq!(day_number(&ymd(secs)), Some((secs / 86_400) as i64), "{secs}");
        }
        assert_eq!((month_name(1), month_name(12), month_name(0), month_name(13)), ("January", "December", "", ""));
    }
}
