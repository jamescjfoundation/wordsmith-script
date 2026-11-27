// SPDX-License-Identifier: MIT OR Apache-2.0
//! A writer's statistics from Wordsmith Script 0.9.0 open in the app as it is now, on the core's
//! statistics (`jcj_shell::writing::stats`), and nothing in them is lost when they are saved.
//!
//! `tests/fixtures/stats-0.9.0.json` is a `stats.json` written by 0.9.0's own statistics types
//! and save, before Script moved onto the core's (`docs/fixtures.md`). The app opens it from the
//! settings folder as `Statistics::open` does at start-up, and writes it back as it does while the
//! writer works and at the end.

use std::path::PathBuf;

use jcj_shell::writing::stats::{FILE, Statistics};
use serde_json::Value;

const SAVED_BY_0_9_0: &str = include_str!("../../../tests/fixtures/stats-0.9.0.json");

/// A settings folder of the test's own.
fn folder(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wordsmith-script-stats-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn statistics_saved_by_0_9_0_keep_every_day_and_setting_through_a_save() {
    let dir = folder("0.9.0");
    std::fs::write(dir.join(FILE), SAVED_BY_0_9_0).unwrap();
    let (stats, problem) = Statistics::open(&dir, wordsmith_script_ui::workspace::fresh_stats());
    assert_eq!(problem, None, "the file reads as it is");
    assert!(!dir.join(format!("{FILE}.damaged")).exists());
    // The writer's settings are theirs, not the starting ones.
    assert!(stats.enabled);
    assert_eq!((stats.idle_minutes, stats.sprint_minutes), (5, 30));
    assert_eq!((stats.goals.pages_per_day, stats.goals.days_per_week, stats.goals.minutes_per_day), (2.5, 5, 45));
    assert_eq!(stats.days.len(), 10);

    stats.save(&dir).unwrap();
    let old: Value = serde_json::from_str(SAVED_BY_0_9_0).unwrap();
    let new: Value = serde_json::from_str(&std::fs::read_to_string(dir.join(FILE)).unwrap()).unwrap();
    // Every setting 0.9.0 wrote is written back with its value.
    for key in ["enabled", "idle_minutes", "sprint_minutes"] {
        assert_eq!(new[key], old[key], "{key}");
    }
    for key in ["pages_per_day", "days_per_week", "minutes_per_day"] {
        assert_eq!(new["goals"][key], old["goals"][key], "goals.{key}");
    }
    // Every day, with its words, pages, seconds and seconds by hour. 0.9.0 left the hours of a
    // day with no writing time empty; they are written as 24 zeros, the same time.
    let old_days = old["days"].as_object().unwrap();
    let new_days = new["days"].as_object().unwrap();
    assert_eq!(old_days.keys().collect::<Vec<_>>(), new_days.keys().collect::<Vec<_>>(), "the same days");
    for (day, was) in old_days {
        let now = &new_days[day];
        for key in ["words", "pages", "seconds"] {
            assert_eq!(now[key], was[key], "{day}: {key}");
        }
        let mut hours: Vec<u64> = was["hours"].as_array().unwrap().iter().map(|h| h.as_u64().unwrap()).collect();
        hours.resize(24, 0);
        let written: Vec<u64> = now["hours"].as_array().unwrap().iter().map(|h| h.as_u64().unwrap()).collect();
        assert_eq!(written, hours, "{day}: hours");
        assert_eq!(was["seconds"].as_u64(), Some(written.iter().sum::<u64>()), "{day}: the hours add up to the day");
    }
    // What was written reads back as the same statistics.
    assert_eq!(Statistics::open(&dir, Statistics::default()), (stats, None));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_writer_with_no_statistics_starts_with_scripts_goals() {
    let dir = folder("fresh");
    let (stats, problem) = Statistics::open(&dir, wordsmith_script_ui::workspace::fresh_stats());
    assert_eq!(problem, None);
    assert!(stats.enabled && stats.days.is_empty());
    // A page and 25 minutes a day, every day (SPEC-2 17).
    assert_eq!((stats.goals.pages_per_day, stats.goals.days_per_week, stats.goals.minutes_per_day), (1.0, 7, 25));
    assert_eq!(stats.sprint_minutes, 25);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_damaged_statistics_file_is_kept_aside_not_written_over() {
    let dir = folder("damaged");
    std::fs::write(dir.join(FILE), "{ not json").unwrap();
    let (stats, problem) = Statistics::open(&dir, wordsmith_script_ui::workspace::fresh_stats());
    assert!(problem.is_some_and(|p| p.contains("could not be read")));
    assert!(stats.days.is_empty());
    assert_eq!(std::fs::read_to_string(dir.join(format!("{FILE}.damaged"))).unwrap(), "{ not json");
    let _ = std::fs::remove_dir_all(&dir);
}
