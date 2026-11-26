// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask layers` and `cargo xtask app-layers`: what an app's packages may depend on.
//!
//! **`layers`** holds the rule every app shares: an app reaches upstream only through the core's
//! `jcj-` interfaces, never `jcj-upstream`, `wordcraft-*` or `craft-*`.
//!
//! **`app-layers`** holds the app's own order, and reads it from the table the app's `AGENTS.md`
//! shows to people and agents. There is no second copy in code, so the two cannot differ: the
//! table is the rule. It sits between two marker lines, each a line of its own:
//!
//! ```text
//! <!-- BEGIN layer table (cargo xtask app-layers reads it) -->
//! | Layer | Packages | Holds | May use |
//! | --- | --- | --- | --- |
//! | 1 | `app-model` | The model | `jcj-doc`, `jcj-entries` (only here) |
//! | 2 | `app-format` | The files | the model, `jcj-project` |
//! | 3 | `app-ui`, `apps/*` | The interface and the programs | everything below, `jcj-shell` |
//! <!-- END layer table -->
//! ```
//!
//! How it is read:
//!
//! - **Rows are layers, lowest first.** With a `Layer` column, rows that carry the same number
//!   are one layer, and the numbers never go down.
//! - **The packages column** (its heading has "package", "crate" or "program" in it) names
//!   workspace packages in backticks: by package name, by folder from the root
//!   (`crates/app-model`), by the folder's last part, or every package under a folder
//!   (`apps/*`). Every package of the workspace but `xtask` is in exactly one row.
//! - **The last column whose heading has "use" or "depend" in it** names, in backticks, the
//!   crates from outside the workspace that the row's packages may use. A crate is allowed from
//!   the layer that names it upwards. `(only here)` after a name keeps it to that row's
//!   packages. Words outside backticks are for the reader, and so is one of the app's own
//!   packages named there, as long as it is under the row.
//! - Other columns are not read.
//!
//! What fails: a package in no row or in two; a name in the table that is no package; a package
//! that depends on one of its own layer or a higher one; and a package that uses a *watched*
//! crate its row and the rows under it do not allow. Watched are the core's crates (`jcj-`),
//! the crates of the app's providers (core decision 0015), and every crate the table names.
//! Any other library is not this check's business.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.
//! Core decisions: <https://github.com/jamescjfoundation/open-source-core/tree/main/docs/decisions>.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::link::{self, Provider};
use super::util::{self, TaskResult, report, workspace_members};

/// The file that holds the table, and how its two marker lines start.
pub const FILE: &str = "AGENTS.md";
pub const TABLE_BEGIN: &str = "<!-- BEGIN layer table";
pub const TABLE_END: &str = "<!-- END layer table";
/// After a crate's name: only the packages of that row may use it.
const ONLY_HERE: &str = "(only here)";
/// The gate itself: it is in no layer.
const GATE: &str = "xtask";

/// Apps reach upstream only through the core's `jcj-` interfaces (the `app-and-core` block,
/// "Things agents must not do").
pub fn upstream_free(root: &Path) -> TaskResult {
    let members = workspace_members(root)?;
    let names: BTreeSet<&str> = members.iter().map(|m| m.name.as_str()).collect();
    let mut failures = Vec::new();
    for member in members.iter().filter(|m| m.name != GATE) {
        for dep in util::direct_deps(root, &member.name)? {
            if names.contains(dep.as_str()) {
                continue;
            }
            if dep.starts_with("wordcraft-") || dep.starts_with("craft-") || dep == "jcj-upstream" {
                failures.push(format!("{} depends on {dep}; use the core's jcj- interfaces", member.name));
            }
        }
    }
    report(failures, "the app uses only the core's jcj- interfaces")
}

/// One row of the table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Its layer, counted from 1.
    pub layer: usize,
    /// The names in its packages column, as written.
    pub packages: Vec<String>,
    /// The names in its "may use" column, and whether each is kept to this row.
    pub uses: Vec<(String, bool)>,
}

/// The cells of a table line. `\|` is a bar inside a cell, and so is a bar inside backticks.
fn cells(line: &str) -> Vec<String> {
    let line = line.trim().trim_start_matches('|');
    let mut out = Vec::new();
    let mut cell = String::new();
    let (mut in_code, mut escaped) = (false, false);
    for c in line.chars() {
        match c {
            '|' if !in_code && !escaped => out.push(std::mem::take(&mut cell).trim().to_owned()),
            '`' => {
                in_code = !in_code;
                cell.push(c);
            }
            _ => cell.push(c),
        }
        escaped = c == '\\' && !escaped;
    }
    if !cell.trim().is_empty() {
        out.push(cell.trim().to_owned());
    }
    out
}

/// The names a cell holds in backticks, and whether `(only here)` follows each.
fn code_spans(cell: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut rest = cell;
    while let Some(start) = rest.find('`') {
        let after = &rest[start + 1..];
        let Some(len) = after.find('`') else { break };
        let name = after[..len].trim();
        rest = &after[len + 1..];
        if !name.is_empty() {
            out.push((name.to_owned(), rest.trim_start().starts_with(ONLY_HERE)));
        }
    }
    out
}

/// The rows of the layer table in `text` (an app's `AGENTS.md`).
pub fn parse_table(text: &str) -> Result<Vec<Row>, String> {
    let mut lines = text.lines().skip_while(|l| !l.trim().starts_with(TABLE_BEGIN));
    if lines.next().is_none() {
        return Err(format!(
            "{FILE} has no layer table: the app's layers go in a Markdown table between the lines `{TABLE_BEGIN} (cargo xtask app-layers reads it) -->` \
             and `{TABLE_END} -->`. The core's template/AGENTS.md shows one"
        ));
    }
    let mut table: Vec<&str> = Vec::new();
    let mut closed = false;
    for line in lines {
        if line.trim().starts_with(TABLE_END) {
            closed = true;
            break;
        }
        if line.trim().starts_with('|') {
            table.push(line);
        }
    }
    if !closed {
        return Err(format!("{FILE}: the layer table has no closing line `{TABLE_END} -->`"));
    }
    let mut table = table.into_iter();
    let header: Vec<String> = table.next().map(cells).unwrap_or_default().iter().map(|h| h.to_lowercase()).collect();
    let column = |words: &[&str]| header.iter().rposition(|h| words.iter().any(|w| h.contains(w)));
    let layer_column = header.iter().position(|h| h == "layer");
    let packages_column = header.iter().position(|h| ["package", "crate", "program"].iter().any(|w| h.contains(w)));
    let (Some(packages_column), Some(uses_column)) = (packages_column, column(&["use", "depend"])) else {
        return Err(format!(
            "{FILE}: the layer table needs a column of packages (a heading with \"package\" or \"crate\") and a column of what they may use \
             (a heading with \"use\" or \"depend\"); its headings are: {}",
            header.join(" | ")
        ));
    };
    let mut rows: Vec<Row> = Vec::new();
    let mut last_number: Option<u64> = None;
    for line in table.skip(1) {
        let cells = cells(line);
        let cell = |i: usize| cells.get(i).map(String::as_str).unwrap_or_default();
        let packages: Vec<String> = code_spans(cell(packages_column)).into_iter().map(|(name, _)| name).collect();
        if packages.is_empty() {
            return Err(format!("{FILE}: a row of the layer table names no package in backticks: {}", line.trim()));
        }
        let layer = match layer_column {
            None => rows.len() + 1,
            Some(i) => {
                let number: u64 =
                    cell(i).parse().map_err(|_| format!("{FILE}: the layer `{}` of a row of the layer table is not a number", cell(i)))?;
                let previous = rows.last().map_or(0, |r| r.layer);
                match last_number {
                    Some(last) if number < last => {
                        return Err(format!("{FILE}: the layer table goes from layer {last} down to {number}; rows are layers, lowest first"));
                    }
                    Some(last) if number == last => previous,
                    _ => previous + 1,
                }
            }
        };
        if let Some(i) = layer_column {
            last_number = cell(i).parse().ok();
        }
        rows.push(Row { layer, packages, uses: code_spans(cell(uses_column)) });
    }
    if rows.is_empty() {
        return Err(format!("{FILE}: the layer table has no rows"));
    }
    Ok(rows)
}

/// The packages a name in the table stands for: a package's name, its folder from the root, the
/// last part of that folder, or `folder/*` for every package under a folder.
fn named<'a>(name: &str, members: &'a [(String, String)]) -> Vec<&'a str> {
    let name = name.trim_end_matches('/');
    if let Some(folder) = name.strip_suffix("/*") {
        let prefix = format!("{folder}/");
        return members.iter().filter(|(_, dir)| dir.starts_with(&prefix)).map(|(n, _)| n.as_str()).collect();
    }
    let exact: Vec<&str> = members.iter().filter(|(n, dir)| n == name || dir == name).map(|(n, _)| n.as_str()).collect();
    if !exact.is_empty() {
        return exact;
    }
    members.iter().filter(|(_, dir)| dir.rsplit('/').next() == Some(name)).map(|(n, _)| n.as_str()).collect()
}

/// Where the table allows one outside crate.
#[derive(Debug, Default)]
struct Allowed {
    /// The lowest layer that names it without `(only here)`.
    from: Option<usize>,
    /// The packages of the rows that name it with `(only here)`.
    only: BTreeSet<String>,
}

/// What breaks the table. `members` are the workspace's packages (name, folder from the root),
/// `deps` each package's direct dependencies, and `watched` the crates checked beyond the
/// core's and the ones the table names (the app's providers' crates).
pub fn problems(rows: &[Row], members: &[(String, String)], deps: &BTreeMap<String, Vec<String>>, watched: &BTreeSet<String>) -> Vec<String> {
    let mut out = Vec::new();
    let is_member = |name: &str| members.iter().any(|(n, _)| n == name);
    // Each package's layer.
    let mut layer_of: BTreeMap<&str, usize> = BTreeMap::new();
    let mut row_members: Vec<Vec<&str>> = Vec::new();
    for row in rows {
        let mut here = Vec::new();
        for name in &row.packages {
            let found = named(name, members);
            if found.is_empty() {
                out.push(format!("the layer table names `{name}`, which is no package of this workspace"));
            } else if found.len() > 1 && !name.ends_with("/*") {
                out.push(format!("the layer table names `{name}`, which could be {}: write the package's name", found.join(" or ")));
            }
            for package in found {
                if layer_of.insert(package, row.layer).is_some() {
                    out.push(format!("{package} is in two rows of the layer table"));
                }
                here.push(package);
            }
        }
        row_members.push(here);
    }
    for (name, _) in members.iter().filter(|(name, _)| name != GATE) {
        if !layer_of.contains_key(name.as_str()) {
            out.push(format!("{name} is in no row of the layer table in {FILE}"));
        }
    }
    // What each row allows from outside.
    let mut allowed: BTreeMap<&str, Allowed> = BTreeMap::new();
    for (row, here) in rows.iter().zip(&row_members) {
        for (name, only) in &row.uses {
            let own = named(name, members);
            if !own.is_empty() {
                // One of the app's own packages, named for the reader: the order decides, and the
                // table may not contradict it.
                for package in own.into_iter().filter(|p| layer_of.get(p).is_some_and(|l| *l >= row.layer)) {
                    out.push(format!("the layer table lets layer {} use {package}, which is not under it", row.layer));
                }
                continue;
            }
            let entry = allowed.entry(name.as_str()).or_default();
            if *only {
                entry.only.extend(here.iter().map(|p| (*p).to_owned()));
            } else {
                entry.from = Some(entry.from.map_or(row.layer, |from| from.min(row.layer)));
            }
        }
    }
    // Every dependency against it.
    for (package, layer) in &layer_of {
        for dep in deps.get(*package).map(Vec::as_slice).unwrap_or_default() {
            if dep.as_str() == *package || dep == GATE {
                continue;
            }
            if is_member(dep) {
                if let Some(theirs) = layer_of.get(dep.as_str()).filter(|theirs| *theirs >= layer) {
                    out.push(format!("{package} (layer {layer}) depends on {dep} (layer {theirs}): a package uses only what is under it"));
                }
                continue;
            }
            let rule = allowed.get(dep.as_str());
            if !(dep.starts_with("jcj-") || watched.contains(dep) || rule.is_some()) {
                continue;
            }
            match rule {
                Some(rule) if rule.from.is_some_and(|from| from <= *layer) || rule.only.contains(*package) => {}
                Some(Allowed { from: Some(from), .. }) => {
                    out.push(format!("{package} (layer {layer}) uses {dep}, which the layer table allows from layer {from}"))
                }
                Some(rule) => {
                    let only: Vec<&str> = rule.only.iter().map(String::as_str).collect();
                    out.push(format!("{package} uses {dep}, which the layer table keeps to {}", only.join(", ")));
                }
                None => out.push(format!("{package} (layer {layer}) uses {dep}, which no row of the layer table lists")),
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// `cargo xtask app-layers`: the workspace against the layer table in `AGENTS.md`.
pub fn task(root: &Path, providers: &[Provider]) -> TaskResult {
    let text = std::fs::read_to_string(root.join(FILE)).map_err(|e| format!("cannot read {FILE}: {e}"))?;
    let rows = parse_table(&text)?;
    let mut members = Vec::new();
    let mut deps = BTreeMap::new();
    for member in workspace_members(root)? {
        let dir = member.dir.strip_prefix(root).unwrap_or(&member.dir).to_string_lossy().replace('\\', "/");
        if member.name != GATE {
            deps.insert(member.name.clone(), util::direct_deps(root, &member.name)?);
        }
        members.push((member.name, dir));
    }
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
    let watched: BTreeSet<String> = providers.iter().flat_map(|p| link::git_dependencies(&manifest, p.git)).collect();
    let layers = rows.iter().map(|r| r.layer).max().unwrap_or(0);
    report(
        problems(&rows, &members, &deps, &watched),
        &format!("{} packages in {layers} layers use only what the table in {FILE} puts under them", deps.len()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "# Rules\n\nSome text with a | bar.\n\n\
        <!-- BEGIN layer table (cargo xtask app-layers reads it) -->\n\
        | Crate | Layer | Holds | May depend on |\n\
        | --- | --- | --- | --- |\n\
        | `app-model` | 1 | The model; no files | `jcj-doc`, `jcj-entries` (only here), `serde` |\n\
        | `crates/app-format`, `design` | 2 | Files \\| folders | the model, `other-format`, `jcj-project` |\n\
        | `app-ui` | 3 | Views | everything below, `jcj-shell`, `window` |\n\
        | `apps/*` | 4 | The programs | the crates above |\n\
        <!-- END layer table -->\n\n| Not | this |\n| --- | --- |\n| `x` | `y` |\n";

    fn members() -> Vec<(String, String)> {
        [
            ("app-model", "crates/app-model"),
            ("app-format", "crates/app-format"),
            ("app-design", "crates/design"),
            ("app-ui", "crates/app-ui"),
            ("app", "apps/app"),
            ("app-cli", "apps/cli"),
            ("xtask", "xtask"),
        ]
        .iter()
        .map(|(n, d)| ((*n).to_owned(), (*d).to_owned()))
        .collect()
    }

    fn deps(of: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        of.iter().map(|(n, d)| ((*n).to_owned(), d.iter().map(|s| (*s).to_owned()).collect())).collect()
    }

    fn check(of: &[(&str, &[&str])]) -> Vec<String> {
        let watched: BTreeSet<String> = ["other-format".to_owned(), "other-model".to_owned()].into_iter().collect();
        problems(&parse_table(TABLE).unwrap(), &members(), &deps(of), &watched)
    }

    #[test]
    fn reads_the_table_between_the_marker_lines() {
        let rows = parse_table(TABLE).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].packages, ["app-model"]);
        assert_eq!(rows[0].uses, [("jcj-doc".to_owned(), false), ("jcj-entries".to_owned(), true), ("serde".to_owned(), false)]);
        assert_eq!(rows[1].packages, ["crates/app-format", "design"]);
        assert_eq!(rows[1].uses.len(), 2);
        assert_eq!(rows.iter().map(|r| r.layer).collect::<Vec<_>>(), [1, 2, 3, 4]);
        assert_eq!(rows[3].packages, ["apps/*"]);
        assert!(rows[3].uses.is_empty());
    }

    #[test]
    fn rows_with_one_number_are_one_layer_and_numbers_never_go_down() {
        let table = |rows: &str| format!("{TABLE_BEGIN} -->\n| Layer | Package | Uses |\n| --- | --- | --- |\n{rows}{TABLE_END} -->\n");
        let rows = parse_table(&table("| 1 | `a` | |\n| 3 | `b` | |\n| 3 | `c` | `typeset` (only here) |\n| 5 | `d` | |\n")).unwrap();
        assert_eq!(rows.iter().map(|r| r.layer).collect::<Vec<_>>(), [1, 2, 2, 3]);
        assert!(parse_table(&table("| 2 | `a` | |\n| 1 | `b` | |\n")).unwrap_err().contains("down to 1"));
        assert!(parse_table(&table("| one | `a` | |\n")).unwrap_err().contains("not a number"));
        assert!(parse_table(&table("| 1 | a | |\n")).unwrap_err().contains("names no package"));
        assert!(parse_table(&table("")).unwrap_err().contains("no rows"));
        // Without a Layer column every row is its own layer.
        let plain = format!("{TABLE_BEGIN} -->\n| Crate | Depends on |\n| --- | --- |\n| `a` | |\n| `b` | |\n{TABLE_END} -->\n");
        assert_eq!(parse_table(&plain).unwrap().iter().map(|r| r.layer).collect::<Vec<_>>(), [1, 2]);
    }

    #[test]
    fn a_table_that_cannot_be_read_says_what_is_missing() {
        assert!(parse_table("# No table here\n").unwrap_err().contains("has no layer table"));
        assert!(parse_table(&format!("{TABLE_BEGIN} -->\n| Crate | Uses |\n| --- | --- |\n| `a` | |\n")).unwrap_err().contains("no closing line"));
        let err = parse_table(&format!("{TABLE_BEGIN} -->\n| Name | Holds |\n| --- | --- |\n| `a` | x |\n{TABLE_END} -->\n")).unwrap_err();
        assert!(err.contains("needs a column of packages"), "{err}");
    }

    #[test]
    fn a_workspace_that_follows_the_table_passes() {
        let found = check(&[
            ("app-model", &["jcj-doc", "jcj-entries", "serde", "unwatched"]),
            ("app-format", &["app-model", "jcj-doc", "jcj-project", "other-format"]),
            ("app-design", &["app-model", "serde"]),
            ("app-ui", &["app-format", "app-design", "app-model", "jcj-shell", "jcj-doc", "window"]),
            ("app", &["app-ui", "jcj-shell"]),
            ("app-cli", &["app-format", "jcj-project"]),
        ]);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn layers_point_down() {
        let found = check(&[("app-model", &["app-format"]), ("app-format", &["app-design"]), ("app", &["app-cli"])]);
        assert_eq!(
            found,
            [
                "app (layer 4) depends on app-cli (layer 4): a package uses only what is under it",
                "app-format (layer 2) depends on app-design (layer 2): a package uses only what is under it",
                "app-model (layer 1) depends on app-format (layer 2): a package uses only what is under it",
            ]
        );
    }

    #[test]
    fn an_outside_crate_is_used_from_its_row_upwards() {
        let found = check(&[
            // Too low; kept to another row; a core crate and a provider's crate no row lists.
            ("app-model", &["jcj-shell", "window", "other-model"]),
            ("app-format", &["jcj-entries", "jcj-board"]),
            // A library the table does not name is not checked, wherever it is used.
            ("app-design", &["regex"]),
        ]);
        assert_eq!(
            found,
            [
                "app-format (layer 2) uses jcj-board, which no row of the layer table lists",
                "app-format uses jcj-entries, which the layer table keeps to app-model",
                "app-model (layer 1) uses jcj-shell, which the layer table allows from layer 3",
                "app-model (layer 1) uses other-model, which no row of the layer table lists",
                "app-model (layer 1) uses window, which the layer table allows from layer 3",
            ]
        );
    }

    #[test]
    fn the_table_and_the_workspace_name_the_same_packages() {
        let mut more = members();
        more.push(("app-new".to_owned(), "crates/app-new".to_owned()));
        let found = problems(&parse_table(TABLE).unwrap(), &more, &BTreeMap::new(), &BTreeSet::new());
        assert_eq!(found, ["app-new is in no row of the layer table in AGENTS.md"]);

        let fewer: Vec<(String, String)> = members().into_iter().filter(|(n, _)| n != "app-design").collect();
        let found = problems(&parse_table(TABLE).unwrap(), &fewer, &BTreeMap::new(), &BTreeSet::new());
        assert_eq!(found, ["the layer table names `design`, which is no package of this workspace"]);

        let twice = TABLE.replace("| `app-ui` | 3 |", "| `app-ui`, `app-model` | 3 |");
        let found = problems(&parse_table(&twice).unwrap(), &members(), &BTreeMap::new(), &BTreeSet::new());
        assert_eq!(found, ["app-model is in two rows of the layer table"]);

        // A row may name the app's own packages for the reader, but not one that is not under it.
        let upside_down = TABLE.replace("the model, `other-format`", "`app-model`, `app-ui`, `other-format`");
        let found = problems(&parse_table(&upside_down).unwrap(), &members(), &BTreeMap::new(), &BTreeSet::new());
        assert_eq!(found, ["the layer table lets layer 2 use app-ui, which is not under it"]);
    }
}
