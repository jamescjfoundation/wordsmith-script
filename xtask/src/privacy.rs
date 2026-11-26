// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask privacy`: nothing personal and no secrets in a public repository.
//!
//! The repositories are public, history included ("Working in the open" in the core plan). This
//! check fails on:
//!
//! - email addresses other than no-reply and example addresses;
//! - local machine paths (a home folder);
//! - things that look like keys or tokens;
//! - commits whose author or committer email is not a no-reply address.
//!
//! It scans every tracked file (`--staged`: only the lines a pending commit adds, for a pre-commit
//! hook). A line that must keep a match on purpose carries the marker `privacy:allow`.
//!
//! Owned by open-source-core (`template/xtask/src/`); copied into apps by `cargo xtask sync`.

use std::path::Path;

use super::util::{TaskResult, git_output, report};

/// Files larger than this are not scanned (they are data, checked by review).
const MAX_SCAN_BYTES: u64 = 4 * 1024 * 1024;

/// The marker that lets one line keep a match on purpose.
const ALLOW_MARKER: &str = concat!("privacy", ":allow");

/// Email domains that are fine to publish.
const ALLOWED_EMAIL_SUFFIXES: &[&str] = &[
    "@users.noreply.github.com",
    "@noreply.github.com",
    "noreply@github.com",
    "noreply@anthropic.com",
    "@example.com",
    "@example.org",
    "@example.net",
    "@example.invalid",
];

/// Home folders that are not a person's: CI runners and documentation placeholders.
const ALLOWED_HOMES: &[&str] = &["runner", "user", "you", "me", "example", "USER", "username", "<user>"];

/// Starts of well-known secret formats. Written split so this file does not match itself.
const SECRET_PREFIXES: &[&str] = &[
    concat!("gh", "p_"),
    concat!("gh", "o_"),
    concat!("gh", "s_"),
    concat!("gh", "u_"),
    concat!("github", "_pat_"),
    concat!("sk", "-ant-"),
    concat!("sk", "_live_"),
    concat!("xo", "xb-"),
    concat!("xo", "xp-"),
    concat!("-----BEGIN ", "PRIVATE KEY"),
    concat!("-----BEGIN ", "RSA PRIVATE KEY"),
    concat!("-----BEGIN ", "OPENSSH PRIVATE KEY"),
];

/// Problems found on one line of text.
pub fn line_problems(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    if line.contains(ALLOW_MARKER) {
        return out;
    }
    for email in emails_in(line) {
        if !ALLOWED_EMAIL_SUFFIXES.iter().any(|s| email.ends_with(s)) {
            out.push(format!("email address `{email}`"));
        }
    }
    for home in home_paths_in(line) {
        out.push(format!("local path `{home}`"));
    }
    for p in SECRET_PREFIXES {
        if let Some(i) = line.find(p) {
            // A prefix followed by a long token-like tail; a bare mention in prose is fine.
            let tail: String = line[i + p.len()..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
            if tail.len() >= 16 || p.starts_with("-----") {
                out.push(format!("something that looks like a secret (`{p}...`)"));
            }
        }
    }
    if let Some(i) = line.find("AKIA") {
        let tail: String = line[i + 4..].chars().take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit()).collect();
        if tail.len() >= 16 {
            out.push("something that looks like a cloud access key".to_owned());
        }
    }
    out
}

fn is_email_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-')
}

/// Email-shaped words: `local@domain.tld`. // privacy:allow
fn emails_in(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if *c != '@' {
            continue;
        }
        let mut s = i;
        while s > 0 && chars.get(s - 1).is_some_and(|c| is_email_char(*c)) {
            s -= 1;
        }
        let mut e = i + 1;
        while chars.get(e).is_some_and(|c| is_email_char(*c)) {
            e += 1;
        }
        let local: String = chars.get(s..i).map(|c| c.iter().collect()).unwrap_or_default();
        let domain: String = chars.get(i + 1..e).map(|c| c.iter().collect()).unwrap_or_default();
        let domain = domain.trim_end_matches('.');
        let Some((_, tld)) = domain.rsplit_once('.') else { continue };
        if local.is_empty() || tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
            continue;
        }
        // `name@1.2.3` (a version requirement) or a git ssh remote are not emails.
        if local == "git" {
            continue;
        }
        out.push(format!("{local}@{domain}"));
    }
    out
}

/// Home-folder paths: `/home/<name>`, `/Users/<name>`, `C:\Users\<name>`.
fn home_paths_in(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for prefix in ["/home/", "/Users/", "\\Users\\"] {
        let mut from = 0;
        while let Some(i) = line.get(from..).and_then(|s| s.find(prefix)) {
            let start = from + i + prefix.len();
            let name: String = line[start..].chars().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '<' | '>')).collect();
            from = start;
            if name.is_empty() || name.starts_with('<') || ALLOWED_HOMES.contains(&name.as_str()) {
                continue;
            }
            out.push(format!("{}{name}", prefix));
        }
    }
    out
}

fn is_text(bytes: &[u8]) -> bool {
    !bytes.iter().take(8192).any(|b| *b == 0)
}

/// Scan every tracked file.
pub fn scan_tree(root: &Path) -> Result<Vec<String>, String> {
    let files = git_output(root, &["ls-files", "-z"])?;
    let mut failures = Vec::new();
    let mut scanned = 0usize;
    for rel in files.split('\0').filter(|f| !f.is_empty()) {
        let path = root.join(rel);
        let Ok(meta) = std::fs::metadata(&path) else { continue };
        if !meta.is_file() || meta.len() > MAX_SCAN_BYTES {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("cannot read {rel}: {e}"))?;
        if !is_text(&bytes) {
            continue;
        }
        scanned += 1;
        let text = String::from_utf8_lossy(&bytes);
        for (n, line) in text.lines().enumerate() {
            for p in line_problems(line) {
                failures.push(format!("{rel}:{}: {p}", n + 1));
            }
        }
    }
    println!("scanned {scanned} tracked text files");
    Ok(failures)
}

/// Scan only the lines a pending commit adds (`git diff --cached`).
pub fn scan_staged(root: &Path) -> Result<Vec<String>, String> {
    let diff = git_output(root, &["diff", "--cached", "--unified=0", "--no-color", "--no-ext-diff"])?;
    let mut failures = Vec::new();
    let mut file = String::new();
    for line in diff.lines() {
        if let Some(f) = line.strip_prefix("+++ b/") {
            file = f.to_owned();
        } else if let Some(added) = line.strip_prefix('+').filter(|_| !line.starts_with("+++")) {
            for p in line_problems(added) {
                failures.push(format!("{file}: {p}"));
            }
        }
    }
    Ok(failures)
}

/// Every author and committer email in the history (or in `range`) must be a no-reply address.
pub fn scan_identities(root: &Path, range: Option<&str>) -> Result<Vec<String>, String> {
    let mut args = vec!["log", "--format=%H %ae %ce"];
    if let Some(r) = range {
        args.push(r);
    }
    let log = match git_output(root, &args) {
        Ok(log) => log,
        // A repository without commits yet has nothing to check.
        Err(e) if e.contains("does not have any commits") => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut failures = Vec::new();
    for line in log.lines() {
        let mut parts = line.split(' ');
        let (Some(sha), Some(author), Some(committer)) = (parts.next(), parts.next(), parts.next()) else { continue };
        for (who, email) in [("author", author), ("committer", committer)] {
            if !ALLOWED_EMAIL_SUFFIXES.iter().any(|s| email.ends_with(s)) {
                failures.push(format!("commit {}: {who} email is not a no-reply address", sha.get(..10).unwrap_or(sha)));
            }
        }
    }
    Ok(failures)
}

/// `cargo xtask privacy [--staged] [--range <git range>]`.
pub fn task(root: &Path, args: &[String]) -> TaskResult {
    let staged = args.iter().any(|a| a == "--staged");
    let range = args.iter().position(|a| a == "--range").and_then(|i| args.get(i + 1)).map(String::as_str);
    let mut failures = if staged { scan_staged(root)? } else { scan_tree(root)? };
    if !staged {
        failures.extend(scan_identities(root, range)?);
    }
    report(failures, "no personal details or secrets found")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_personal_emails_but_not_noreply_ones() {
        assert_eq!(line_problems("contact: someone@company.co.uk").len(), 1); // privacy:allow
        assert!(line_problems("123+name@users.noreply.github.com").is_empty());
        assert!(line_problems("Co-Authored-By: Claude <noreply@anthropic.com>").is_empty());
        assert!(line_problems("a@example.com").is_empty());
        assert!(line_problems("git@github.com:org/repo.git").is_empty());
        assert!(line_problems("serde = \"1\" # x@1.2").is_empty());
        assert!(line_problems(&format!("person@company.com {ALLOW_MARKER}")).is_empty()); // privacy:allow
    }

    #[test]
    fn finds_home_folders() {
        assert_eq!(line_problems("see /home/alice/project").len(), 1); // privacy:allow
        assert_eq!(line_problems("C:\\Users\\bob\\file").len(), 1);
        assert_eq!(line_problems("/Users/carol/x").len(), 1); // privacy:allow
        assert!(line_problems("/home/runner/work").is_empty());
        assert!(line_problems("~/project and /usr/home").is_empty());
    }

    #[test]
    fn finds_secrets() {
        let token = format!("{}{}", concat!("gh", "p_"), "a".repeat(36));
        assert_eq!(line_problems(&format!("token = \"{token}\"")).len(), 1);
        assert!(line_problems(concat!("the gh", "p_ prefix is a GitHub token")).is_empty());
        assert_eq!(line_problems(concat!("-----BEGIN ", "PRIVATE KEY-----")).len(), 1);
        assert_eq!(line_problems(&format!("AKIA{}", "ABCDEFGHIJKLMNOP")).len(), 1);
    }
}
