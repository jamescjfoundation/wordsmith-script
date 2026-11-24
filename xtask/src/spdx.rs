// SPDX-License-Identifier: MIT OR Apache-2.0
//! A small SPDX license-expression evaluator for the attribution check.
//!
//! It answers one question: can we use this package under licenses that are
//! all on our allowlist? `A OR B` passes if either side passes, `A AND B`
//! only if both do. Input comes from `cargo tree`, so it is treated as
//! untrusted: length and nesting are bounded and unknown syntax fails.

/// License ids on the allowlist, spelled as SPDX ids (open-source-core decision 0001).
pub const ALLOWED: &[&str] =
    &["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "0BSD", "ISC", "OFL-1.1", "CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0", "Unlicense"];

/// Licenses outside the plain allowlist that were reviewed and approved
/// because WordCraft and its dependencies cannot build without them.
/// Reasons and conditions are in open-source-core decision 0001. Add to
/// this list only with a maintainer's approval recorded there.
pub const REVIEWED_EXCEPTIONS: &[&str] = &["Zlib", "Unicode-3.0", "IJG", "BSL-1.0", "CDLA-Permissive-2.0"];

/// `WITH` exceptions that only add permissions to an allowed license.
pub const ALLOWED_EXCEPTIONS: &[&str] = &["LLVM-exception"];

const MAX_LEN: usize = 1024;
const MAX_DEPTH: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Open,
    Close,
    Word(String),
}

fn tokenize(expr: &str) -> Result<Vec<Token>, String> {
    if expr.len() > MAX_LEN {
        return Err(format!("license expression longer than {MAX_LEN} bytes"));
    }
    let mut tokens = Vec::new();
    let mut word = String::new();
    let flush = |word: &mut String, tokens: &mut Vec<Token>| {
        if !word.is_empty() {
            tokens.push(Token::Word(std::mem::take(word)));
        }
    };
    for ch in expr.chars() {
        match ch {
            '(' => {
                flush(&mut word, &mut tokens);
                tokens.push(Token::Open);
            }
            ')' => {
                flush(&mut word, &mut tokens);
                tokens.push(Token::Close);
            }
            // Older crates write "MIT/Apache-2.0" meaning either.
            '/' => {
                flush(&mut word, &mut tokens);
                tokens.push(Token::Word("OR".to_owned()));
            }
            c if c.is_whitespace() => flush(&mut word, &mut tokens),
            c if c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '+' | ':') => word.push(c),
            c => return Err(format!("unexpected character {c:?} in license expression")),
        }
    }
    flush(&mut word, &mut tokens);
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek_word(&self) -> Option<&str> {
        match self.tokens.get(self.pos) {
            Some(Token::Word(w)) => Some(w.as_str()),
            _ => None,
        }
    }

    fn is_op(&self, op: &str) -> bool {
        self.peek_word().is_some_and(|w| w.eq_ignore_ascii_case(op))
    }

    fn or_expr(&mut self, depth: usize) -> Result<bool, String> {
        let mut ok = self.and_expr(depth)?;
        while self.is_op("OR") {
            self.pos += 1;
            let rhs = self.and_expr(depth)?;
            ok = ok || rhs;
        }
        Ok(ok)
    }

    fn and_expr(&mut self, depth: usize) -> Result<bool, String> {
        let mut ok = self.atom(depth)?;
        while self.is_op("AND") {
            self.pos += 1;
            let rhs = self.atom(depth)?;
            ok = ok && rhs;
        }
        Ok(ok)
    }

    fn atom(&mut self, depth: usize) -> Result<bool, String> {
        if depth >= MAX_DEPTH {
            return Err("license expression nested too deeply".to_owned());
        }
        match self.tokens.get(self.pos).cloned() {
            Some(Token::Open) => {
                self.pos += 1;
                let ok = self.or_expr(depth + 1)?;
                if self.tokens.get(self.pos) != Some(&Token::Close) {
                    return Err("unbalanced parentheses in license expression".to_owned());
                }
                self.pos += 1;
                Ok(ok)
            }
            Some(Token::Word(id)) => {
                if ["AND", "OR", "WITH"].iter().any(|op| id.eq_ignore_ascii_case(op)) {
                    return Err(format!("operator {id} where a license id was expected"));
                }
                self.pos += 1;
                let mut ok = ALLOWED.contains(&id.as_str()) || REVIEWED_EXCEPTIONS.contains(&id.as_str());
                if self.is_op("WITH") {
                    self.pos += 1;
                    match self.peek_word() {
                        Some(exc) => {
                            ok = ok && ALLOWED_EXCEPTIONS.contains(&exc);
                            self.pos += 1;
                        }
                        None => return Err("WITH without an exception id".to_owned()),
                    }
                }
                Ok(ok)
            }
            Some(Token::Close) => Err("unexpected ')' in license expression".to_owned()),
            None => Err("license expression ended early".to_owned()),
        }
    }
}

/// Returns `Ok(true)` if the expression can be satisfied with allowed
/// licenses only, `Ok(false)` if it cannot, and `Err` if it does not parse.
pub fn is_allowed(expr: &str) -> Result<bool, String> {
    let tokens = tokenize(expr)?;
    if tokens.is_empty() {
        return Err("empty license expression".to_owned());
    }
    let mut parser = Parser { tokens, pos: 0 };
    let ok = parser.or_expr(0)?;
    if parser.pos != parser.tokens.len() {
        return Err("trailing tokens in license expression".to_owned());
    }
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_ids() {
        assert_eq!(is_allowed("MIT"), Ok(true));
        assert_eq!(is_allowed("Apache-2.0"), Ok(true));
        assert_eq!(is_allowed("GPL-3.0-only"), Ok(false));
        assert_eq!(is_allowed("MPL-2.0"), Ok(false));
    }

    #[test]
    fn or_needs_one_side() {
        assert_eq!(is_allowed("MIT OR Apache-2.0"), Ok(true));
        assert_eq!(is_allowed("Apache-2.0 OR BSL-1.0"), Ok(true));
        assert_eq!(is_allowed("GPL-2.0-only OR LGPL-2.1-only"), Ok(false));
        assert_eq!(is_allowed("MIT/Apache-2.0"), Ok(true));
        assert_eq!(is_allowed("Unlicense OR MIT"), Ok(true));
    }

    #[test]
    fn and_needs_both_sides() {
        assert_eq!(is_allowed("(MIT OR Apache-2.0) AND MPL-2.0"), Ok(false));
        assert_eq!(is_allowed("MIT AND ISC"), Ok(true));
        assert_eq!(is_allowed("(MIT OR Apache-2.0) AND (BSD-3-Clause)"), Ok(true));
    }

    #[test]
    fn precedence_and_binds_tighter() {
        // GPL AND MIT OR ISC == (GPL AND MIT) OR ISC
        assert_eq!(is_allowed("GPL-3.0-only AND MIT OR ISC"), Ok(true));
        assert_eq!(is_allowed("ISC OR GPL-3.0-only AND MIT"), Ok(true));
        assert_eq!(is_allowed("MPL-2.0 OR GPL-3.0-only AND MIT"), Ok(false));
    }

    #[test]
    fn reviewed_exceptions_pass() {
        assert_eq!(is_allowed("Zlib"), Ok(true));
        assert_eq!(is_allowed("(MIT OR Apache-2.0) AND Unicode-3.0"), Ok(true));
        assert_eq!(is_allowed("(MIT OR Apache-2.0) AND IJG"), Ok(true));
        assert_eq!(is_allowed("BSL-1.0"), Ok(true));
        assert_eq!(is_allowed("Unicode-3.0 AND GPL-3.0-only"), Ok(false));
    }

    #[test]
    fn with_exceptions() {
        assert_eq!(is_allowed("Apache-2.0 WITH LLVM-exception"), Ok(true));
        assert_eq!(is_allowed("Apache-2.0 WITH LLVM-exception OR MIT"), Ok(true));
        assert_eq!(is_allowed("GPL-2.0-only WITH Classpath-exception-2.0"), Ok(false));
    }

    #[test]
    fn hostile_input_fails_cleanly() {
        assert!(is_allowed("").is_err());
        assert!(is_allowed("(MIT").is_err());
        assert!(is_allowed("MIT)").is_err());
        assert!(is_allowed("MIT OR").is_err());
        assert!(is_allowed("OR MIT").is_err());
        assert!(is_allowed("MIT; rm -rf").is_err());
        assert!(is_allowed("Apache-2.0 WITH").is_err());
        let deep = format!("{}MIT{}", "(".repeat(100), ")".repeat(100));
        assert!(is_allowed(&deep).is_err());
        let long = "MIT OR ".repeat(500) + "MIT";
        assert!(is_allowed(&long).is_err());
    }
}
