// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask brand`: `assets/brand/` holds the brand kit's files, as the kit gave them.
//!
//! The logos, the icons and the seven palette colors are the Wordsmith brand kit's (the
//! `brand/` folder of the wordsmith repository; project decision 0007, Route F of the project
//! map). The kit's tool writes Script's copies into `assets/brand/` and leaves a record beside
//! them, `kit.json`: the kit commit they were taken at, and every file the folder holds, each
//! *carried* (a kit file, byte for byte, with its SHA-256), *own* (Script writes it) or *derived*
//! (Script makes it from a carried file).
//!
//! This module is Script's own, and asks two questions of that record:
//!
//! - `cargo xtask brand`, in the gate, reads only files. Is the folder what the record says: each
//!   carried file with the recorded checksum, each own and derived file present, no file the
//!   record does not name, and the theme's `palette` the kit's seven colors?
//! - `cargo xtask brand --kit [<file>]`, outside the gate. Is the record still what the kit
//!   gives? It reads Script's entry in the kit's published list, `brand/apps.json` on the wordsmith
//!   repository's `main` (with `curl`, or from `<file>`), and fails when the two differ. The
//!   workflow `brand-kit.yml` asks it every week.
//!
//! A failure of either is mended with the kit's tool, never by hand ([`RETAKE`]).

// core-candidate: open-source-core#47
// Wordsmith Manuscript and Wordsmith Book hold the same two checks for their own folders; one
// check of "files carried from elsewhere, with a record" would serve all three.

use std::path::Path;
use std::process::Command;

use crate::shared::json::{self, Value};
use crate::shared::package::{files_under, sha256_of};
use crate::shared::util::{TaskResult, report};

/// The brand folder, from the repository's root, and the record in it.
const FOLDER: &str = "assets/brand";
const RECORD: &str = "kit.json";
/// Script's key in the kit's list.
const APP: &str = "script";
/// The kit's list of what each app carries, as published.
const KIT_LIST: &str = "https://raw.githubusercontent.com/jamescjfoundation/wordsmith/main/brand/apps.json";
/// How the copies are taken again.
const RETAKE: &str = "check the wordsmith repository out beside this one and run `npm run brand -- --app script` in its website/wiki";
/// How many colors the kit's palette has.
const PALETTE: usize = 7;

/// A kit file the folder carries.
#[derive(Debug, PartialEq, Eq)]
struct Carried {
    file: String,
    /// Where it is in the kit.
    from: String,
    sha256: String,
}

/// Script's part of a record or of the kit's list: what the folder is to hold.
#[derive(Debug, PartialEq, Eq)]
struct Listing {
    carried: Vec<Carried>,
    /// The files Script writes or derives, each with which of the two it is.
    ours: Vec<(String, &'static str)>,
    theme: Option<String>,
    /// The palette's colors, upper case, sorted.
    palette: Vec<String>,
}

/// The colors of a JSON object's members, upper case and sorted. The names are not compared:
/// the kit says "app-color" where a theme says "ribbon-green".
fn colors(palette: Option<&Value>) -> Vec<String> {
    let mut found = Vec::new();
    if let Some(Value::Object(members)) = palette {
        found.extend(members.iter().filter_map(|(_, color)| color.as_str()).map(str::to_uppercase));
    }
    found.sort();
    found
}

impl Listing {
    /// Read an app's entry: the `app` member of a record, or the app's member of the kit's list.
    fn read(entry: &Value) -> Result<Listing, String> {
        let list = |key: &str| entry.get(key).map(Value::items).unwrap_or_default();
        let mut carried = Vec::new();
        for one in list("carried") {
            let (Some(file), Some(sha256)) = (one.str_of("file"), one.str_of("sha256")) else {
                return Err("a carried file has no `file` or no `sha256`".to_owned());
            };
            carried.push(Carried { file: file.to_owned(), from: one.str_of("from").unwrap_or("the kit").to_owned(), sha256: sha256.to_owned() });
        }
        let mut ours = Vec::new();
        for (key, kind) in [("own", "Script's own"), ("derived", "made here from a kit file")] {
            ours.extend(list(key).iter().filter_map(|one| one.str_of("file")).map(|file| (file.to_owned(), kind)));
        }
        Ok(Listing { carried, ours, theme: entry.str_of("theme").map(str::to_owned), palette: colors(entry.get("palette")) })
    }

    /// The listing a record's text holds.
    fn of_record(text: &str) -> Result<Listing, String> {
        let record = json::parse(text).map_err(|e| format!("{FOLDER}/{RECORD} cannot be read: {e}"))?;
        let entry = record.get("app").ok_or_else(|| format!("{FOLDER}/{RECORD} has no `app` list"))?;
        Listing::read(entry).map_err(|e| format!("{FOLDER}/{RECORD}: {e}"))
    }

    /// Whether the listing names `file`, in any of its three kinds.
    fn names(&self, file: &str) -> bool {
        self.carried.iter().any(|c| c.file == file) || self.ours.iter().any(|(name, _)| name == file)
    }
}

/// The folder as the check sees it: which files are there, a file's checksum, a file's text.
struct Folder<'a> {
    present: &'a [String],
    checksum: &'a dyn Fn(&str) -> Result<String, String>,
    text: &'a dyn Fn(&str) -> Option<String>,
}

/// Where the folder is not what the record's listing says, a sentence each.
fn folder_problems(listing: &Listing, folder: &Folder) -> Vec<String> {
    let here = |file: &str| folder.present.iter().any(|p| p == file);
    let mut out = Vec::new();
    if listing.carried.is_empty() {
        out.push(format!("{FOLDER}/{RECORD} lists no carried file"));
    }
    for Carried { file, from, sha256 } in &listing.carried {
        if !here(file) {
            out.push(format!("{FOLDER}/{file} is missing: it is the kit's {from}"));
            continue;
        }
        match (folder.checksum)(file) {
            Ok(found) if found == *sha256 => {}
            Ok(_) => out.push(format!("{FOLDER}/{file} differs from the kit's {from} as it was taken. A kit file is never changed here")),
            Err(e) => out.push(e),
        }
    }
    for (file, kind) in &listing.ours {
        if !here(file) {
            out.push(format!("{FOLDER}/{file} is missing: the record lists it as {kind}"));
        }
    }
    for file in folder.present {
        if file != RECORD && !listing.names(file) {
            out.push(format!(
                "{FOLDER}/{file} is not in the record. A brand file comes from the kit; a file of Script's own is added to the kit's list first"
            ));
        }
    }
    out.extend(theme_problem(listing, folder));
    out
}

/// The theme is Script's own file but for its palette, which holds the kit's colors.
fn theme_problem(listing: &Listing, folder: &Folder) -> Option<String> {
    let Some(theme) = &listing.theme else {
        return Some(format!("{FOLDER}/{RECORD} names no theme file"));
    };
    if listing.palette.len() != PALETTE {
        return Some(format!("{FOLDER}/{RECORD}: the palette has {} colors, where the kit has {PALETTE}", listing.palette.len()));
    }
    let Some(text) = (folder.text)(theme) else {
        return Some(format!("{FOLDER}/{theme}, the theme, cannot be read"));
    };
    let tokens = match json::parse(&text) {
        Ok(tokens) => tokens,
        Err(e) => return Some(format!("{FOLDER}/{theme}: {e}")),
    };
    let found = colors(tokens.get("palette"));
    (found != listing.palette)
        .then(|| format!("{FOLDER}/{theme}: its palette is {}, and the kit's colors are {}", found.join(" "), listing.palette.join(" ")))
}

/// Where the kit's published list has moved on from the record, a sentence each.
fn kit_moves(record: &str, kit_list: &str) -> Result<Vec<String>, String> {
    let taken = Listing::of_record(record)?;
    let list = json::parse(kit_list).map_err(|e| format!("the kit's list: {e}"))?;
    let entry = list.get("apps").and_then(|apps| apps.get(APP)).ok_or_else(|| format!("the kit's list has no entry for {APP}"))?;
    let now = Listing::read(entry).map_err(|e| format!("the kit's list: {e}"))?;
    let mut out = Vec::new();
    for kit in &now.carried {
        match taken.carried.iter().find(|c| c.file == kit.file) {
            None => out.push(format!("{}: the kit gives this file now, and it has not been taken", kit.file)),
            Some(have) if have.sha256 != kit.sha256 => out.push(format!("{}: the kit's file changed after it was taken", kit.file)),
            Some(_) => {}
        }
    }
    for have in &taken.carried {
        if !now.carried.iter().any(|c| c.file == have.file) {
            out.push(format!("{}: the kit's list no longer gives this file to Script", have.file));
        }
    }
    if taken.palette != now.palette {
        out.push("the palette: the kit's colors changed".to_owned());
    }
    if out.is_empty() && taken != now {
        out.push("the kit's list differs in Script's own or derived files, or in the theme's name".to_owned());
    }
    Ok(out)
}

/// The kit's list as published.
fn published_list() -> Result<String, String> {
    let fetched = Command::new("curl")
        .args(["--fail", "--silent", "--show-error", "--location", "--max-time", "60", KIT_LIST])
        .output()
        .map_err(|e| format!("curl could not be run to read {KIT_LIST}: {e}"))?;
    if !fetched.status.success() {
        return Err(format!("{KIT_LIST} could not be read: {}", String::from_utf8_lossy(&fetched.stderr).trim()));
    }
    String::from_utf8(fetched.stdout).map_err(|_| format!("{KIT_LIST} is not text"))
}

/// Add how to mend it to a failed check's message.
fn with_remedy(result: TaskResult, why: &str) -> TaskResult {
    result.map_err(|e| format!("{e}\n{why} To take the kit's files again: {RETAKE}."))
}

/// `cargo xtask brand [--kit [<file>]]`.
pub fn task(root: &Path, args: &[String]) -> TaskResult {
    let folder = root.join(FOLDER);
    let record = std::fs::read_to_string(folder.join(RECORD))
        .map_err(|e| format!("{FOLDER}/{RECORD} cannot be read ({e}). The kit's tool writes it: {RETAKE}."))?;
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let kit_list = match args.as_slice() {
        [] => {
            let listing = Listing::of_record(&record)?;
            let present = files_under(&folder)?;
            let found = folder_problems(
                &listing,
                &Folder {
                    present: &present,
                    checksum: &|file| sha256_of(&folder.join(file)),
                    text: &|file| std::fs::read_to_string(folder.join(file)).ok(),
                },
            );
            let ok = format!("{} files in {FOLDER} are the brand kit's or Script's own, and the theme holds the kit's palette", present.len());
            return with_remedy(report(found, &ok), "The folder is not what its record says.");
        }
        ["--kit"] => published_list()?,
        ["--kit", file] => std::fs::read_to_string(file).map_err(|e| format!("{file} cannot be read: {e}"))?,
        _ => return Err("usage: cargo xtask brand [--kit [<the kit's brand/apps.json>]]".to_owned()),
    };
    let ok = format!("the brand kit's list for {APP} is what {FOLDER}/{RECORD} records");
    with_remedy(report(kit_moves(&record, &kit_list)?, &ok), "The brand kit has moved since Script's copies were taken.")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An app's entry, as the record and the kit's list both hold it.
    const ENTRY: &str = r##"{
        "carried": [
          { "file": "icon.png", "from": "brand/app/icon.png", "sha256": "11" },
          { "file": "mark.svg", "from": "brand/app/mark.svg", "sha256": "22" }
        ],
        "own": [ { "file": "tokens.json", "what": "the theme" } ],
        "derived": [ { "file": "mask.png", "from": ["mark.svg"], "by": "a tool" } ],
        "theme": "tokens.json",
        "palette": { "ink": "#1C2340", "paper": "#FAFAF7", "app-color": "#2E9473", "app-color-text": "#1F7358", "margin-blue": "#3B4FA8", "slate": "#5A6080", "night": "#12162B" }
      }"##;
    /// A theme with the same colors under its own names, one written in lower case.
    const THEME: &str = r##"{ "palette": { "iron-gall-ink": "#1c2340", "page-white": "#FAFAF7", "ribbon-green": "#2E9473", "ribbon-green-text": "#1F7358", "margin-blue": "#3B4FA8", "slate": "#5A6080", "night": "#12162B" } }"##;
    const FILES: [&str; 5] = ["icon.png", "kit.json", "mark.svg", "mask.png", "tokens.json"];

    fn record(entry: &str) -> String {
        format!(r#"{{ "kit": {{ "repository": "example/kit", "commit": "abc" }}, "app": {entry} }}"#)
    }

    fn kit_list(entry: &str) -> String {
        format!(r#"{{ "apps": {{ "book": {{}}, "script": {entry} }} }}"#)
    }

    /// The problems of a folder holding `files`, where `changed` has another checksum than
    /// recorded and the theme's text is `theme`.
    fn problems(files: &[&str], changed: &str, theme: &str) -> Vec<String> {
        let listing = Listing::of_record(&record(ENTRY)).unwrap();
        let present: Vec<String> = files.iter().map(|f| (*f).to_owned()).collect();
        folder_problems(
            &listing,
            &Folder {
                present: &present,
                checksum: &|file| {
                    let recorded = listing.carried.iter().find(|c| c.file == file).map(|c| c.sha256.clone()).ok_or(format!("{file}: not carried"))?;
                    Ok(if file == changed { "ff".to_owned() } else { recorded })
                },
                text: &|file| (file == "tokens.json").then(|| theme.to_owned()),
            },
        )
    }

    #[test]
    fn a_folder_that_is_what_the_record_says_passes() {
        assert_eq!(problems(&FILES, "", THEME), Vec::<String>::new());
    }

    #[test]
    fn a_kit_file_changed_here_fails() {
        let found = problems(&FILES, "mark.svg", THEME);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("assets/brand/mark.svg differs from the kit's brand/app/mark.svg"), "{found:?}");
    }

    #[test]
    fn a_missing_file_fails_whatever_its_kind() {
        for (gone, says) in
            [("icon.png", "it is the kit's brand/app/icon.png"), ("mask.png", "made here from a kit file"), ("tokens.json", "Script's own")]
        {
            let left: Vec<&str> = FILES.iter().copied().filter(|f| *f != gone).collect();
            let found = problems(&left, "", THEME);
            assert!(found.iter().any(|p| p.contains(&format!("{gone} is missing")) && p.contains(says)), "{gone}: {found:?}");
        }
    }

    #[test]
    fn a_file_the_record_does_not_name_fails() {
        let mut files = FILES.to_vec();
        files.push("sketch.png");
        let found = problems(&files, "", THEME);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("assets/brand/sketch.png is not in the record"), "{found:?}");
    }

    #[test]
    fn a_theme_off_the_palette_fails() {
        let found = problems(&FILES, "", &THEME.replace("#2E9473", "#2E9474"));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("tokens.json: its palette is") && found[0].contains("#2E9474"), "{found:?}");
        assert!(problems(&FILES, "", "not json")[0].starts_with("assets/brand/tokens.json: "));
    }

    #[test]
    fn a_record_that_cannot_be_used_says_why() {
        assert!(Listing::of_record("{").unwrap_err().contains("cannot be read"));
        assert!(Listing::of_record("{}").unwrap_err().contains("has no `app` list"));
        let no_sum = record(r#"{ "carried": [ { "file": "icon.png" } ] }"#);
        assert!(Listing::of_record(&no_sum).unwrap_err().contains("no `sha256`"));
        // A record with no palette and no theme is a record of nothing.
        let bare = Listing::of_record(&record(r#"{ "carried": [] }"#)).unwrap();
        let found = folder_problems(&bare, &Folder { present: &[], checksum: &|_| Ok(String::new()), text: &|_| None });
        assert!(found.iter().any(|p| p.contains("lists no carried file")) && found.iter().any(|p| p.contains("names no theme file")), "{found:?}");
    }

    #[test]
    fn the_kits_list_is_held_against_the_record() {
        let ours = record(ENTRY);
        assert_eq!(kit_moves(&ours, &kit_list(ENTRY)), Ok(Vec::new()));
        let changed = ENTRY.replace(r#""sha256": "22""#, r#""sha256": "33""#);
        assert_eq!(kit_moves(&ours, &kit_list(&changed)).unwrap(), ["mark.svg: the kit's file changed after it was taken"]);
        let added = ENTRY.replace(r#"{ "file": "icon.png""#, r#"{ "file": "new.svg", "from": "x", "sha256": "44" }, { "file": "icon.png""#);
        assert_eq!(kit_moves(&ours, &kit_list(&added)).unwrap(), ["new.svg: the kit gives this file now, and it has not been taken"]);
        // The other way round: the record has a file the kit stopped giving.
        assert_eq!(kit_moves(&record(&added), &kit_list(ENTRY)).unwrap(), ["new.svg: the kit's list no longer gives this file to Script"]);
        assert_eq!(kit_moves(&ours, &kit_list(&ENTRY.replace("#12162B", "#12162C"))).unwrap(), ["the palette: the kit's colors changed"]);
        let renamed = ENTRY.replace(r#""file": "mask.png""#, r#""file": "mask-2.png""#);
        assert_eq!(kit_moves(&ours, &kit_list(&renamed)).unwrap().len(), 1);
        assert!(kit_moves(&ours, r#"{ "apps": {} }"#).unwrap_err().contains("no entry for script"));
    }

    /// The folder in this repository is what its record says.
    #[test]
    fn the_folder_in_this_repository_passes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        assert_eq!(task(&root, &[]), Ok(()));
    }
}
