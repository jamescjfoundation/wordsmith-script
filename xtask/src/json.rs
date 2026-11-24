// SPDX-License-Identifier: MIT OR Apache-2.0
//! Just enough JSON to read `cargo metadata`. xtask has no dependencies (core decision 0003), so it
//! carries this instead of a JSON crate. Input is treated as untrusted: nesting is bounded and
//! anything outside RFC 8259 fails.

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// Numbers are kept as written; nothing here needs them as numbers.
    Number(String),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    /// The member `key` of an object.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// The elements of an array, or nothing for anything else.
    pub fn items(&self) -> &[Value] {
        match self {
            Value::Array(items) => items,
            _ => &[],
        }
    }

    /// The member `key` as a string, if it is one.
    pub fn str_of(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_str)
    }
}

const MAX_DEPTH: usize = 64;

pub fn parse(text: &str) -> Result<Value, String> {
    let mut p = Parser { bytes: text.as_bytes(), pos: 0 };
    p.space();
    let v = p.value(0)?;
    p.space();
    if p.pos != p.bytes.len() {
        return Err(format!("JSON: unexpected text at byte {}", p.pos));
    }
    Ok(v)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, b: u8) -> Result<(), String> {
        if self.peek() == Some(b) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("JSON: expected '{}' at byte {}", b as char, self.pos))
        }
    }

    fn literal(&mut self, word: &str, v: Value) -> Result<Value, String> {
        if self.bytes.get(self.pos..self.pos + word.len()) == Some(word.as_bytes()) {
            self.pos += word.len();
            Ok(v)
        } else {
            Err(format!("JSON: unexpected text at byte {}", self.pos))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err("JSON: nested too deeply".into());
        }
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().map(Value::String),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(format!("JSON: expected a value at byte {}", self.pos)),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, String> {
        self.expect(b'{')?;
        let mut members = Vec::new();
        self.space();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.space();
            let key = self.string()?;
            self.space();
            self.expect(b':')?;
            self.space();
            members.push((key, self.value(depth + 1)?));
            self.space();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(members));
                }
                _ => return Err(format!("JSON: expected ',' or '}}' at byte {}", self.pos)),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.space();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.space();
            items.push(self.value(depth + 1)?);
            self.space();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(format!("JSON: expected ',' or ']' at byte {}", self.pos)),
            }
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        while matches!(self.peek(), Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')) {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e| format!("JSON: {e}"))?;
        if text.parse::<f64>().is_err() {
            return Err(format!("JSON: bad number at byte {start}"));
        }
        Ok(Value::Number(text.to_owned()))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let digits = self.bytes.get(self.pos..self.pos + 4).ok_or("JSON: short \\u escape")?;
        let text = std::str::from_utf8(digits).map_err(|e| format!("JSON: {e}"))?;
        let n = u32::from_str_radix(text, 16).map_err(|_| format!("JSON: bad \\u escape at byte {}", self.pos))?;
        self.pos += 4;
        Ok(n)
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let start = self.pos;
            while !matches!(self.peek(), None | Some(b'"' | b'\\')) {
                if self.peek().is_some_and(|b| b < 0x20) {
                    return Err(format!("JSON: control character in a string at byte {}", self.pos));
                }
                self.pos += 1;
            }
            out.push_str(std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|e| format!("JSON: {e}"))?);
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let esc = self.peek().ok_or("JSON: string ended in an escape")?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi) {
                                self.expect(b'\\')?;
                                self.expect(b'u')?;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err("JSON: unpaired surrogate".into());
                                }
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            out.push(char::from_u32(code).ok_or("JSON: bad character escape")?);
                        }
                        other => return Err(format!("JSON: unknown escape \\{}", other as char)),
                    }
                }
                _ => return Err("JSON: unterminated string".into()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_cargo_metadata_writes() {
        let v = parse(r#" {"packages":[{"name":"a\"b","license":null,"n":-1.5e3,"ok":true,"u":"\u00e9\ud83d\ude00"}],"x":[]} "#).unwrap();
        let p = &v.get("packages").unwrap().items()[0];
        assert_eq!(p.str_of("name"), Some("a\"b"));
        assert_eq!(p.get("license"), Some(&Value::Null));
        assert_eq!(p.get("n"), Some(&Value::Number("-1.5e3".into())));
        assert_eq!(p.get("ok"), Some(&Value::Bool(true)));
        assert_eq!(p.str_of("u"), Some("é😀"));
        assert!(v.get("x").unwrap().items().is_empty());
    }

    #[test]
    fn refuses_what_is_not_json() {
        for bad in ["", "{", "[1,]", "{\"a\" 1}", "\"\\x\"", "tru", "[1] 2", "\"a\nb\"", "\"\\ud800\""] {
            assert!(parse(bad).is_err(), "{bad:?} parsed");
        }
        let deep = "[".repeat(100) + &"]".repeat(100);
        assert!(parse(&deep).is_err());
    }
}
