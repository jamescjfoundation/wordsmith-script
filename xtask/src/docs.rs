// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask docs`: what the instructions and records point at is there.
//!
//! The rules that send work from one repository to another are prose, and prose rots silently: a
//! file moves, a record is renumbered, a marker never gets its issue. This task reads the Markdown
//! files Git tracks or would add (and, for the third rule, the Rust files) and fails on:
//!
//! 1. **A relative link that leads nowhere.** The target of a Markdown link, an image, a
//!    reference definition or an `href`/`src` attribute must be a tracked file or folder, and a
//!    `#fragment` on a link to a Markdown file must be one of its headings (as GitHub names them)
//!    or an explicit `id`. Addresses with a scheme and paths that start with `/` are not read.
//! 2. **A decision reference that names no record.** "core decision 0011" must be a record in the
//!    core's `docs/decisions/`; a bare "decision 0011" must be a record in the repository's own.
//!    The grammar is on [`decision_refs`].
//! 3. **A `core-candidate` marker without an issue.** The comment that starts `// core-candidate:`
//!    must name the core issue that tracks it (`#12`, `open-source-core#12`, or the issue's
//!    address), so the list of code waiting to move is the core's issue list.
//!
//! Finished plans and published reviews under `docs/history/` are kept as written and not read.
//! Fenced code blocks are not read, and code spans are not read for links.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::sync;
use super::util::{TaskResult, git_output, report};

/// Folders that are never read.
const SKIP_DIRS: &[&str] = &[".git", ".private", "target", "node_modules"];
/// Finished plans and published reviews: history, kept as written.
const HISTORY: &str = "docs/history/";
/// The marker lines of the blocks an app carries from the core.
const BLOCK_BEGIN: &str = "<!-- BEGIN open-source-core:";
const BLOCK_END: &str = "<!-- END open-source-core:";
/// What starts a marker comment.
const MARKER: &str = "core-candidate:";
/// Capitalised words that may stand directly before "decision" without being an owner's name.
const ORDINARY: &[&str] = &[
    "After",
    "Also",
    "Amends",
    "And",
    "As",
    "Before",
    "By",
    "Extends",
    "Follows",
    "From",
    "In",
    "Or",
    "Per",
    "Qualifies",
    "Read",
    "Replaces",
    "See",
    "Since",
    "Supersedes",
    "That",
    "The",
    "This",
    "Under",
    "Until",
    "When",
    "With",
];

/// What to check a folder against.
pub struct Rules<'a> {
    /// The folder a "core decision NNNN" must name a record in.
    pub core_decisions: &'a Path,
    /// The folder a bare "decision NNNN" must name a record in; `None` leaves that rule out.
    pub own_decisions: Option<&'a Path>,
    /// Folders whose Markdown is written for another repository (the core's `template/`): a bare
    /// reference there is not this repository's.
    pub not_own: &'a [&'a str],
    /// Whether `core-candidate` markers are checked.
    pub markers: bool,
}

/// What a run read and found.
pub struct Found {
    pub failures: Vec<String>,
    pub markdown: usize,
    pub markers: usize,
}

/// `cargo xtask docs` in an app: against the pinned core release.
pub fn app_task(root: &Path) -> TaskResult {
    let core = sync::locate_core(root)?;
    let own = root.join("docs/decisions");
    let rules =
        Rules { core_decisions: &core.join("docs/decisions"), own_decisions: own.is_dir().then_some(own.as_path()), not_own: &[], markers: true };
    finish(check(root, &rules)?, true)
}

/// `cargo xtask docs` in the core, or `cargo xtask docs <dir>` on any folder (an app, or a
/// repository without Cargo, which has no other way to be checked).
// An app's xtask never calls this; allowed here so an app's `mod docs;` needs no attribute.
#[allow(dead_code)]
pub fn core_task(core: &Path, dir: Option<&str>) -> TaskResult {
    let decisions = core.join("docs/decisions");
    match dir {
        None => {
            let rules = Rules { core_decisions: &decisions, own_decisions: Some(&decisions), not_own: &["template/"], markers: false };
            finish(check(core, &rules)?, false)
        }
        Some(dir) => {
            let root = Path::new(dir);
            if !root.is_dir() {
                return Err(format!("{dir} is not a folder"));
            }
            let own = root.join("docs/decisions");
            let rules = Rules { core_decisions: &decisions, own_decisions: own.is_dir().then_some(own.as_path()), not_own: &[], markers: true };
            finish(check(root, &rules)?, true)
        }
    }
}

fn finish(found: Found, markers: bool) -> TaskResult {
    let about_markers = if markers {
        format!("{} core-candidate marker(s) name an issue", found.markers)
    } else {
        "no core-candidate markers to check here (they mark app code)".to_owned()
    };
    report(found.failures, &format!("{} Markdown files: links and decision references resolve; {about_markers}", found.markdown))
}

/// Read every tracked Markdown and Rust file under `root` and collect what breaks a rule.
pub fn check(root: &Path, rules: &Rules) -> Result<Found, String> {
    let files = tracked(root);
    let known = Known::new(&files);
    let core_records = records(rules.core_decisions).map_err(|e| format!("the core's decision records: {e}"))?;
    let own_records = match rules.own_decisions {
        Some(dir) => Some(records(dir).map_err(|e| format!("{}: {e}", dir.display()))?),
        None => None,
    };
    let mut found = Found { failures: Vec::new(), markdown: 0, markers: 0 };
    let mut anchors: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in files.iter().filter(|p| is_markdown(p) && !skipped(p)) {
        let Ok(text) = std::fs::read_to_string(root.join(path)) else { continue };
        found.markdown += 1;
        for (line, target) in links(&text) {
            if let Some(problem) = link_problem(root, path, &text, &target, &known, &mut anchors) {
                found.failures.push(format!("{path}:{line}: link `{target}` {problem}"));
            }
        }
        let own_here = !rules.not_own.iter().any(|dir| path.starts_with(dir));
        let carried = carried_lines(&text);
        for r in decision_refs(&text) {
            let in_block = carried.get(r.line.saturating_sub(1)).copied().unwrap_or(false);
            match r.owner {
                Owner::Core if !core_records.contains(&r.number) => {
                    found.failures.push(format!("{path}:{}: \"core decision {}\" names no record in the core's docs/decisions/", r.line, r.number));
                }
                Owner::Own if own_here && !in_block => {
                    if let Some(own) = &own_records
                        && !own.contains(&r.number)
                    {
                        found.failures.push(format!(
                            "{path}:{}: \"decision {}\" names no record in docs/decisions/ here (a record of the core is \"core decision {}\")",
                            r.line, r.number, r.number
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    if rules.markers {
        for path in files.iter().filter(|p| p.ends_with(".rs") && !skipped(p) && !p.starts_with("xtask/")) {
            let Ok(text) = std::fs::read_to_string(root.join(path)) else { continue };
            let (all, bad) = markers(&text);
            found.markers += all;
            for line in bad {
                found.failures.push(format!(
                    "{path}:{line}: this core-candidate marker names no core issue (write `// {MARKER} open-source-core#N ...`, opening the issue first)"
                ));
            }
        }
    }
    Ok(found)
}

// ---- which files ------------------------------------------------------------------------------

/// The files Git tracks under `root`, and the new ones it does not ignore, which the next commit
/// may add (paths with `/`); or every file when `root` is not a checkout or holds none of these
/// (a scratch copy).
fn tracked(root: &Path) -> Vec<String> {
    let listed: Vec<String> = git_output(root, &["ls-files", "-z", "--cached", "--others", "--exclude-standard"])
        .map(|out| out.split('\0').filter(|p| !p.is_empty() && root.join(p).exists()).map(str::to_owned).collect())
        .unwrap_or_default();
    if !listed.is_empty() {
        return listed;
    }
    let mut out = Vec::new();
    walk(root, "", 0, &mut out);
    out.sort();
    out
}

fn walk(root: &Path, rel: &str, depth: usize, out: &mut Vec<String>) {
    if depth > 24 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root.join(rel)) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
        if entry.path().is_dir() {
            if !SKIP_DIRS.contains(&name.as_str()) {
                walk(root, &path, depth + 1, out);
            }
        } else {
            out.push(path);
        }
    }
}

fn is_markdown(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".md")
}

fn skipped(path: &str) -> bool {
    path.starts_with(HISTORY) || path.split('/').any(|part| SKIP_DIRS.contains(&part))
}

/// Every tracked file, and every folder that holds one.
struct Known {
    files: BTreeSet<String>,
    dirs: BTreeSet<String>,
}

impl Known {
    fn new(files: &[String]) -> Self {
        let mut dirs = BTreeSet::new();
        for file in files {
            let mut dir = file.as_str();
            while let Some((parent, _)) = dir.rsplit_once('/') {
                if !dirs.insert(parent.to_owned()) {
                    break;
                }
                dir = parent;
            }
        }
        Known { files: files.iter().cloned().collect(), dirs }
    }
    fn has(&self, path: &str) -> bool {
        path.is_empty() || self.files.contains(path) || self.dirs.contains(path)
    }
}

// ---- reading Markdown -------------------------------------------------------------------------

/// The lines of `text`, with every line of a fenced code block (and its fence lines) emptied.
fn prose(text: &str) -> Vec<&str> {
    let mut fence: Option<(char, usize)> = None;
    let mut out = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        let mark = t.chars().next().filter(|c| *c == '`' || *c == '~');
        let run = mark.map_or(0, |m| t.chars().take_while(|c| *c == m).count());
        match (fence, mark) {
            (None, Some(m)) if run >= 3 => {
                fence = Some((m, run));
                out.push("");
            }
            (Some((m, n)), Some(c)) if c == m && run >= n && t.trim_end().chars().all(|x| x == m) => {
                fence = None;
                out.push("");
            }
            (Some(_), _) => out.push(""),
            (None, _) => out.push(line),
        }
    }
    out
}

/// `line` with its code spans replaced by spaces, so that what is quoted as code is not read.
fn without_code_spans(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < chars.len() {
        if chars.get(i) != Some(&'`') {
            if let Some(c) = chars.get(i) {
                out.push(*c);
            }
            i += 1;
            continue;
        }
        let run = chars.iter().skip(i).take_while(|c| **c == '`').count();
        // The closing run is the next run of exactly the same length.
        let mut j = i + run;
        let mut close = None;
        while j < chars.len() {
            if chars.get(j) == Some(&'`') {
                let n = chars.iter().skip(j).take_while(|c| **c == '`').count();
                if n == run {
                    close = Some(j + n);
                    break;
                }
                j += n;
            } else {
                j += 1;
            }
        }
        match close {
            Some(end) => {
                out.extend(std::iter::repeat_n(' ', end - i));
                i = end;
            }
            None => {
                out.extend(std::iter::repeat_n('`', run));
                i += run;
            }
        }
    }
    out
}

/// Every link target in `text`, with the number of the line it is on.
fn links(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (i, line) in prose(text).into_iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        for target in link_targets(&without_code_spans(line)) {
            out.push((i + 1, target));
        }
    }
    out
}

/// The targets of the inline links and images, the reference definition and the `href` and `src`
/// attributes on one line.
fn link_targets(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some((_, after)) = rest.split_once("](") {
        if let Some(target) = inline_target(after) {
            out.push(target);
        }
        rest = after;
    }
    // `[label]: target`, at the start of a line. `[^1]: text` is a footnote.
    let t = line.trim_start();
    if line.len() - t.len() <= 3
        && let Some(def) = t.strip_prefix('[').filter(|d| !d.starts_with('^'))
        && let Some((label, target)) = def.split_once("]:")
        && !label.contains(']')
        && let Some(first) = target.split_whitespace().next()
    {
        out.push(first.trim_start_matches('<').trim_end_matches('>').to_owned());
    }
    for attr in ["href=\"", "src=\"", "href='", "src='"] {
        let quote = if attr.ends_with('"') { '"' } else { '\'' };
        let mut rest = line;
        while let Some((_, after)) = rest.split_once(attr) {
            if let Some((value, _)) = after.split_once(quote) {
                out.push(value.to_owned());
            }
            rest = after;
        }
    }
    out
}

/// The target of an inline link, given the text after its `](`. `None` when the line does not
/// close the link, which is prose that happens to hold `](`.
fn inline_target(after: &str) -> Option<String> {
    let after = after.trim_start();
    if let Some(angled) = after.strip_prefix('<') {
        return angled.split_once('>').map(|(target, _)| target.to_owned());
    }
    let mut depth = 0usize;
    for (i, c) in after.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => return after.get(..i).map(str::to_owned),
            ')' => depth -= 1,
            // A title follows: `(target "title")`.
            c if c.is_whitespace() => return after.get(i..).filter(|rest| rest.contains(')')).and_then(|_| after.get(..i)).map(str::to_owned),
            _ => {}
        }
    }
    None
}

/// The path and fragment of a target that points into the repository; `None` for an address with
/// a scheme, a path from a site's root, and an empty target.
fn local_target(target: &str) -> Option<(String, Option<String>)> {
    let target = target.trim();
    if target.is_empty() || target.starts_with('/') {
        return None;
    }
    // A scheme: letters, digits, `+`, `.`, `-`, then a colon, before any `/` or `#`.
    if let Some((scheme, _)) = target.split_once(':')
        && scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
    {
        return None;
    }
    let (path, fragment) = match target.split_once('#') {
        Some((path, fragment)) => (path, Some(percent_decoded(fragment))),
        None => (target, None),
    };
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    Some((percent_decoded(path), fragment))
}

fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(b) = bytes.get(i) {
        let hex = (*b == b'%').then(|| text.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())).flatten();
        match hex {
            Some(value) => {
                out.push(value);
                i += 3;
            }
            None => {
                out.push(*b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `path`, written in the file `from`, as a path from the repository's root. `None` when it
/// climbs out of the repository.
fn resolved(from: &str, path: &str) -> Option<String> {
    let mut parts: Vec<&str> = from.rsplit_once('/').map_or_else(Vec::new, |(dir, _)| dir.split('/').collect());
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

/// What is wrong with one link target, if anything.
fn link_problem(
    root: &Path,
    from: &str,
    text: &str,
    target: &str,
    known: &Known,
    anchors: &mut BTreeMap<String, BTreeSet<String>>,
) -> Option<String> {
    let (path, fragment) = local_target(target)?;
    let to = if path.is_empty() {
        from.to_owned()
    } else {
        let Some(to) = resolved(from, &path) else {
            return Some("points outside the repository; give the full address".to_owned());
        };
        to
    };
    if !known.has(&to) {
        return Some(format!("leads nowhere: no tracked file or folder `{to}`"));
    }
    let fragment = fragment.filter(|f| !f.is_empty())?;
    if !is_markdown(&to) {
        return None;
    }
    if !anchors.contains_key(&to) {
        let target_text = if to == from { text.to_owned() } else { std::fs::read_to_string(root.join(&to)).unwrap_or_default() };
        anchors.insert(to.clone(), anchors_of(&target_text));
    }
    let names = anchors.get(&to)?;
    if names.contains(&fragment) || names.contains(&fragment.to_lowercase()) {
        return None;
    }
    Some(format!("names no heading or id `#{fragment}` in `{to}`"))
}

/// The names a `#fragment` may use in a Markdown file: each heading as GitHub names it, and every
/// explicit `id` or `name` attribute.
fn anchors_of(text: &str) -> BTreeSet<String> {
    let lines = prose(text);
    let mut out = BTreeSet::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for (i, line) in lines.iter().enumerate() {
        for attr in ["id=\"", "name=\"", "id='", "name='"] {
            let quote = if attr.ends_with('"') { '"' } else { '\'' };
            let mut rest = *line;
            while let Some((_, after)) = rest.split_once(attr) {
                if let Some((value, _)) = after.split_once(quote) {
                    out.insert(value.to_owned());
                }
                rest = after;
            }
        }
        let heading = atx_heading(line).or_else(|| {
            // A line underlined with `=` or `-`.
            let under = lines.get(i + 1)?.trim();
            let underlined = under.len() >= 2 && (under.chars().all(|c| c == '=') || under.chars().all(|c| c == '-'));
            let t = line.trim();
            (underlined && !t.is_empty() && !t.starts_with(['#', '-', '*', '>', '|'])).then_some(t)
        });
        let Some(heading) = heading else { continue };
        let slug = slug(heading);
        let count = seen.entry(slug.clone()).or_insert(0);
        out.insert(if *count == 0 { slug.clone() } else { format!("{slug}-{count}") });
        *count += 1;
    }
    out
}

/// The text of a `#` heading line.
fn atx_heading(line: &str) -> Option<&str> {
    let t = line.trim_start();
    if line.len() - t.len() > 3 {
        return None;
    }
    let hashes = t.chars().take_while(|c| *c == '#').count();
    let rest = t.get(hashes..)?;
    if !(1..=6).contains(&hashes) || !(rest.is_empty() || rest.starts_with(char::is_whitespace)) {
        return None;
    }
    Some(rest.trim().trim_end_matches('#').trim_end())
}

/// A heading as GitHub names it in an address: its text without markup, in lower case, keeping
/// letters, digits, `-` and `_`, with each space made a `-`.
fn slug(heading: &str) -> String {
    let mut out = String::new();
    for c in plain_text(heading).trim().chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            out.extend(c.to_lowercase());
        } else if c == ' ' {
            out.push('-');
        }
    }
    out
}

/// Heading text as it is shown: a link is its text, an image its description, a tag nothing.
fn plain_text(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let after = rest.get(c.len_utf8()..).unwrap_or("");
        match c {
            '<' => match after.split_once('>') {
                Some((tag, tail)) if !tag.contains('<') => rest = tail,
                _ => {
                    out.push(c);
                    rest = after;
                }
            },
            '[' => match after.split_once(']') {
                Some((label, tail)) if tail.starts_with('(') => {
                    out.push_str(&plain_text(label));
                    rest = tail.split_once(')').map_or("", |(_, tail)| tail);
                }
                _ => rest = after,
            },
            '!' if after.starts_with('[') => rest = after,
            _ => {
                out.push(c);
                rest = after;
            }
        }
    }
    out
}

// ---- decision references ----------------------------------------------------------------------

/// Whose record a reference means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// The core's: "core decision 0011".
    Core,
    /// This repository's: a bare "decision 0011".
    Own,
    /// Somebody else's, which this task cannot read: "project decision 0003", "Script's decision
    /// 0042".
    Other,
}

/// One number in a decision reference.
#[derive(Debug, PartialEq, Eq)]
pub struct Ref {
    pub line: usize,
    pub owner: Owner,
    pub number: String,
}

/// The decision references in a Markdown text, outside fenced code blocks.
///
/// A reference is the word "decision" or "decisions" (any case), white space (a line break
/// counts), then one or more four-digit numbers, each plain or as link text (`[0011](address)`),
/// joined by a comma, "and", "or" or "to": "decisions 0004, 0011 and 0013". A number that runs on
/// into more digits, a date or a version is not one. Numbers in file names and addresses are not
/// references, since no "decision " stands before them.
///
/// The word directly before says whose record. Only white space, the `[` of a link or emphasis
/// marks may stand between that word and "decision"; a bracket or quote before the word itself is
/// taken off it:
///
/// - "core", "core's", "open-source-core", "open-source-core's": the core's;
/// - "project", "project's", any word ending in "'s", or a capitalised word that is not one of a
///   few ordinary ones (`ORDINARY`), read as a name ("Script decision 0042"): somebody else's;
/// - anything else, or punctuation or a bracket in between ("on open-source-core (decision
///   0044)"): this repository's own.
pub fn decision_refs(text: &str) -> Vec<Ref> {
    let text = prose(text).join("\n");
    let lower = text.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = lower.get(from..).and_then(|rest| rest.find("decision")) {
        let start = from + found;
        from = start + "decision".len();
        let before = text.get(..start).unwrap_or("");
        if before.chars().next_back().is_some_and(char::is_alphanumeric) {
            continue;
        }
        let after = lower.get(from..).unwrap_or("");
        let after = after.strip_prefix('s').unwrap_or(after);
        let numbers = numbers(after);
        if numbers.is_empty() {
            continue;
        }
        let line = before.matches('\n').count() + 1;
        let owner = owner(before);
        out.extend(numbers.into_iter().map(|number| Ref { line, owner, number }));
    }
    out
}

/// The numbers of a reference, given the text after "decision" or "decisions".
fn numbers(after: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = after.trim_start();
    if rest.len() == after.len() {
        return out;
    }
    while let Some((number, tail)) = number(rest) {
        out.push(number);
        let tail = tail.trim_start();
        let (comma, tail) = match tail.strip_prefix(',') {
            Some(t) => (true, t.trim_start()),
            None => (false, tail),
        };
        let word = ["and", "or", "to"].iter().find_map(|w| tail.strip_prefix(w).filter(|t| t.starts_with(char::is_whitespace)));
        rest = match (word, comma) {
            (Some(t), _) => t.trim_start(),
            (None, true) => tail,
            (None, false) => break,
        };
    }
    out
}

/// A four-digit number at the start of `text`, plain or as link text, and what follows it.
fn number(text: &str) -> Option<(String, &str)> {
    let (linked, t) = match text.strip_prefix('[') {
        Some(t) => (true, t),
        None => (false, text),
    };
    let digits = t.get(..4).filter(|d| d.bytes().all(|b| b.is_ascii_digit()))?;
    let after = t.get(4..)?;
    let mut next = after.chars();
    match next.next() {
        Some(c) if c.is_ascii_digit() => return None,
        Some('-' | '.' | '/' | ':') if next.next().is_some_and(|c| c.is_ascii_digit()) => return None,
        _ => {}
    }
    if !linked {
        return Some((digits.to_owned(), after));
    }
    let after = after.strip_prefix(']')?;
    let after = match after.strip_prefix('(') {
        Some(address) => address.split_once(')').map(|(_, tail)| tail)?,
        None => after,
    };
    Some((digits.to_owned(), after))
}

/// Whose record a reference means, from the text before its "decision".
fn owner(before: &str) -> Owner {
    const MARKUP: &[char] = &['[', '*', '_', '`'];
    const OPENING: &[char] = &['(', '[', '"', '\'', '\u{201c}', '\u{2018}', '*', '_', '`'];
    let before = before.trim_end_matches(MARKUP);
    if !before.ends_with(char::is_whitespace) {
        return Owner::Own;
    }
    let word = before.trim_end().rsplit(char::is_whitespace).next().unwrap_or("").trim_start_matches(OPENING);
    let lower = word.to_lowercase().replace('\u{2019}', "'");
    match lower.as_str() {
        "core" | "core's" | "open-source-core" | "open-source-core's" => Owner::Core,
        "project" | "project's" => Owner::Other,
        _ if lower.ends_with("'s") => Owner::Other,
        _ if word.chars().next().is_some_and(char::is_uppercase)
            && word.chars().all(|c| c.is_alphabetic() || c == '-')
            && !ORDINARY.contains(&word) =>
        {
            Owner::Other
        }
        _ => Owner::Own,
    }
}

/// For each line of `text`, whether it is inside a block the core owns (`open-source-core`
/// markers): the references there are the core's text, written from the core's side.
fn carried_lines(text: &str) -> Vec<bool> {
    let mut inside = false;
    text.lines()
        .map(|line| {
            let t = line.trim();
            if t.starts_with(BLOCK_BEGIN) {
                inside = true;
            }
            let here = inside;
            if t.starts_with(BLOCK_END) {
                inside = false;
            }
            here
        })
        .collect()
}

/// The numbers of the decision records in `dir`: files named `NNNN-title.md`.
fn records(dir: &Path) -> Result<BTreeSet<String>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let number = name.get(..4).filter(|n| n.bytes().all(|b| b.is_ascii_digit()))?;
            name.get(4..).filter(|rest| rest.starts_with(['-', '.'])).map(|_| number.to_owned())
        })
        .collect())
}

// ---- core-candidate markers -------------------------------------------------------------------

/// How many `core-candidate` markers a Rust file holds, and the lines of those that name no
/// issue. A marker is a comment line holding `core-candidate:`; the comment lines that follow it
/// belong to it, up to the next marker.
fn markers(text: &str) -> (usize, Vec<usize>) {
    let lines: Vec<&str> = text.lines().collect();
    let is_comment = |line: &str| line.trim_start().starts_with("//");
    let mut all = 0;
    let mut bad = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if !is_comment(line) || !line.contains(MARKER) {
            continue;
        }
        all += 1;
        let mut comment = String::from(*line);
        for next in lines.iter().skip(i + 1).take(8) {
            if !is_comment(next) || next.contains(MARKER) {
                break;
            }
            comment.push('\n');
            comment.push_str(next);
        }
        if !names_issue(&comment) {
            bad.push(i + 1);
        }
    }
    (all, bad)
}

/// Whether a comment names an issue: `#12`, or an address with `/issues/12`.
fn names_issue(comment: &str) -> bool {
    ["#", "/issues/"].iter().any(|mark| {
        let mut rest = comment;
        while let Some((_, after)) = rest.split_once(mark) {
            if after.starts_with(|c: char| c.is_ascii_digit()) {
                return true;
            }
            rest = after;
        }
        false
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_link_targets_and_leaves_code_alone() {
        assert_eq!(
            link_targets("see [the map](project/README.md#route-a) and ![a picture](docs/a b.png \"title\")"),
            ["project/README.md#route-a", "docs/a"]
        );
        assert_eq!(link_targets("[x](<docs/a b.md>) [y](docs/f(1).md)"), ["docs/a b.md", "docs/f(1).md"]);
        assert_eq!(link_targets("[guide]: docs/guide.md \"The guide\""), ["docs/guide.md"]);
        assert!(link_targets("[^1]: a footnote, not a link").is_empty());
        assert_eq!(link_targets("<a href=\"docs/x.md\"><img src='docs/y.png'></a>"), ["docs/x.md", "docs/y.png"]);
        // Prose that holds `](` without closing it is not a link.
        assert!(link_targets("an array index a[i](x").is_empty());
        // A code span is quoted text.
        assert!(link_targets(&without_code_spans("write `[text](missing.md)` to link")).is_empty());
        assert_eq!(without_code_spans("a `b` c ``d ` e`` f `g"), "a     c           f `g");
        // A fenced block is not read; the line after it is.
        let text = "one\n```sh\n[x](nowhere.md)\n```\n[y](there.md)\n";
        assert_eq!(links(text), [(5, "there.md".to_owned())]);
    }

    #[test]
    fn tells_local_targets_from_addresses() {
        assert_eq!(local_target("docs/a.md#top"), Some(("docs/a.md".to_owned(), Some("top".to_owned()))));
        assert_eq!(local_target("#route-a"), Some((String::new(), Some("route-a".to_owned()))));
        assert_eq!(local_target("docs/a%20b.md?raw=true"), Some(("docs/a b.md".to_owned(), None)));
        for skipped in ["https://example.invalid/x", "mailto:someone@example.invalid", "/wiki/script", "", "app://x"] {
            assert_eq!(local_target(skipped), None, "{skipped}");
        }
        assert_eq!(resolved("docs/guide/a.md", "../format.md"), Some("docs/format.md".to_owned()));
        assert_eq!(resolved("README.md", "./docs/"), Some("docs".to_owned()));
        assert_eq!(resolved("docs/a.md", "../../other/README.md"), None);
    }

    #[test]
    fn names_headings_as_github_does() {
        let text = "# The `project/` folder\n\n## Route A: a core fix, found from an app\n\n<a id=\"route-b\"></a>\n### [Linked](x.md) & more\n\n\
                    ## Twice\n\n## Twice\n\nUnderlined\n----------\n\n```\n## not a heading\n```\n";
        let names = anchors_of(text);
        for name in ["the-project-folder", "route-a-a-core-fix-found-from-an-app", "route-b", "linked--more", "twice", "twice-1", "underlined"] {
            assert!(names.contains(name), "{name} is missing from {names:?}");
        }
        assert!(!names.contains("not-a-heading"));
        assert_eq!(atx_heading("#hashtag"), None);
        assert_eq!(atx_heading("## Closed ##"), Some("Closed"));
    }

    #[test]
    fn reads_decision_references() {
        let refs = |text: &str| decision_refs(text).into_iter().map(|r| (r.owner, r.number)).collect::<Vec<_>>();
        let n = |owner, number: &str| (owner, number.to_owned());
        assert_eq!(refs("see core decision 0011 and decision 0003."), [n(Owner::Core, "0011"), n(Owner::Own, "0003")]);
        assert_eq!(refs("Decisions 0001, 0003 and 0004 hold"), [n(Owner::Own, "0001"), n(Owner::Own, "0003"), n(Owner::Own, "0004")]);
        // Linked numbers, across a line break, as the carried blocks write them.
        let linked = "once it has (core decisions\n  [0004](https://example.invalid/0004-x.md)\n  and\n  [0011](https://example.invalid/0011-y.md)).";
        assert_eq!(refs(linked), [n(Owner::Core, "0004"), n(Owner::Core, "0011")]);
        assert_eq!(refs("([core decision 0015](https://example.invalid/d/0015-x.md))"), [n(Owner::Core, "0015")]);
        assert_eq!(refs("the core's decision 0003, open-source-core's decision 0012"), [n(Owner::Core, "0003"), n(Owner::Core, "0012")]);
        // Somebody else's records are not ours to check.
        for other in
            ["project decision 0003", "([project decision 0003](x))", "Script's decision 0042", "(Script decision 0044: x)", "\"Book decision 0003\""]
        {
            assert_eq!(refs(other).first().map(|r| r.0), Some(Owner::Other), "{other}");
        }
        // An ordinary capitalised word, or punctuation in between, leaves it our own.
        assert_eq!(refs("Qualifies decision 0002 (rule 3)"), [n(Owner::Own, "0002")]);
        assert_eq!(refs("the fallback, decision 0003"), [n(Owner::Own, "0003")]);
        assert_eq!(refs("built on open-source-core (decision 0044); see (decision 0002)"), [n(Owner::Own, "0044"), n(Owner::Own, "0002")]);
        assert_eq!(refs("the core [decision 0003](x) and **core decision 0001**"), [n(Owner::Core, "0003"), n(Owner::Core, "0001")]);
        // Not references: a file name, an address, a date, a longer number, a word that only ends alike.
        for not in
            ["docs/decisions/0003-x.md", "a decision 2026-11-26", "decision 00031", "indecision 0003", "decision made", "```\ndecision 0003\n```"]
        {
            assert!(refs(not).is_empty(), "{not}");
        }
        assert_eq!(decision_refs("a\nb decision 0007").first().map(|r| r.line), Some(2));
    }

    #[test]
    fn a_marker_must_name_an_issue() {
        let text = "\
// core-candidate: open-source-core#22 (a table widget)
fn a() {}
//! core-candidate: every app needs this,
//! and it has an issue (core issue #20).
/// core-candidate: the shell has no file dialogs yet.
/// Every app needs them.
fn b() {}
// core-candidate: https://example.invalid/open-source-core/issues/7
let s = \"core-candidate: in a string is not a marker\";
";
        assert_eq!(markers(text), (4, vec![5]));
        assert!(names_issue("see #3") && !names_issue("# heading") && !names_issue("issue number three"));
    }

    #[test]
    fn checks_a_folder() {
        let tmp = std::env::temp_dir().join(format!("jcj-docs-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let (core, app) = (tmp.join("core"), tmp.join("app"));
        let put = |root: &Path, file: &str, text: &str| {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        put(&core, "docs/decisions/0011-releases.md", "# Releases\n");
        put(&app, "docs/decisions/0003-print.md", "# Print\n\nSee [the guide](../guide.md#saving) and decision 0003.\n");
        put(&app, "docs/guide.md", "# Guide\n\n## Saving\n");
        put(&app, "docs/history/old-plan.md", "[gone](gone.md) and decision 0999\n");
        put(&app, "src/lib.rs", "// core-candidate: open-source-core#4\n// core-candidate: no issue yet\n");
        put(
            &app,
            "AGENTS.md",
            "# Notes\n\n[ok](docs/guide.md) [gone](docs/gone.md) [no heading](docs/guide.md#printing) [out](../other/x.md)\n\n\
             <!-- BEGIN open-source-core: app -->\ndecision 0011 is the core's own way to say it\n<!-- END open-source-core: app -->\n\n\
             core decision 0011, core decision 0099, decision 0003, decision 0042, project decision 0001.\n",
        );
        let decisions = app.join("docs/decisions");
        let rules = Rules { core_decisions: &core.join("docs/decisions"), own_decisions: Some(&decisions), not_own: &[], markers: true };
        let found = check(&app, &rules).unwrap();
        assert_eq!((found.markdown, found.markers), (3, 2));
        let has = |part: &str| found.failures.iter().any(|f| f.contains(part));
        assert_eq!(found.failures.len(), 6, "{:#?}", found.failures);
        assert!(has("AGENTS.md:3: link `docs/gone.md` leads nowhere"));
        assert!(has("names no heading or id `#printing`"));
        assert!(has("points outside the repository"));
        assert!(has("\"core decision 0099\" names no record in the core's"));
        assert!(has("AGENTS.md:9: \"decision 0042\" names no record in docs/decisions/ here"));
        assert!(has("src/lib.rs:2: this core-candidate marker names no core issue"));
        // Without the bare rule (a repository with no records of its own) only the rest is said.
        let rules = Rules { core_decisions: &core.join("docs/decisions"), own_decisions: None, not_own: &[], markers: false };
        assert_eq!(check(&app, &rules).unwrap().failures.len(), 4);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
