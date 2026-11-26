// SPDX-License-Identifier: MIT OR Apache-2.0
//! The hostile-input corpus (no input may crash the app): truncated, bit-flipped and
//! random files in every format this app reads, through every reader and importer, and the
//! scripts that do read through pagination. Any panic fails the test. Deterministic: the same
//! corpus every run.

use std::path::Path;

use wordsmith_script_format::{fountain, import, native};

/// A small deterministic generator (xorshift).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Valid files of every format, made from a golden script.
fn seeds() -> Vec<(&'static str, Vec<u8>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let text = std::fs::read_to_string(dir.join("timid-teddy.fountain")).expect("golden");
    let doc = fountain::read(&text).expect("reads").doc;
    let mut s = wordsmith_script_model::session_for(doc.clone());
    // Exercise every mark the native format carries.
    let _ = s.run("revision.next", &serde_json::json!({}));
    let _ = s.run("text.insert", &serde_json::json!({"text": "Revised {brace} *star*."}));
    let _ = s.run("note.add", &serde_json::json!({"text": "A note."}));
    let _ = s.run("scene.lockNumbers", &serde_json::json!({"lock": true}));
    let _ = s.run("lock.pages", &serde_json::json!({}));
    let _ = s.run("beat.add", &serde_json::json!({"title": "Beat"}));
    vec![
        ("fountain", text.into_bytes()),
        ("wss", native::write(&s.doc).unwrap().into_bytes()),
        ("txt", wordsmith_script_format::export::text(&doc).into_bytes()),
        ("rtf", wordsmith_script_format::export::rtf(&doc).into_bytes()),
        ("pdf", wordsmith_script_model::export::pdf_of(&wordsmith_script_model::paginate::paginate(&doc)).expect("pdf")),
    ]
}

/// Mutants of `seed`: truncations, byte flips, spliced junk.
fn mutants(seed: &[u8], rng: &mut Rng, n: usize) -> Vec<Vec<u8>> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut m = seed.to_vec();
        match i % 4 {
            0 => m.truncate(rng.below(seed.len() + 1)),
            1 => {
                for _ in 0..1 + rng.below(16) {
                    let at = rng.below(m.len());
                    if let Some(b) = m.get_mut(at) {
                        *b ^= 1 << rng.below(8);
                    }
                }
            }
            2 => {
                let at = rng.below(m.len() + 1);
                let junk: Vec<u8> = (0..rng.below(64)).map(|_| rng.next() as u8).collect();
                m.splice(at..at, junk);
            }
            _ => {
                let (a, b) = (rng.below(m.len() + 1), rng.below(m.len() + 1));
                let (a, b) = (a.min(b), a.max(b));
                m.drain(a..b);
            }
        }
        out.push(m);
    }
    out
}

/// Every reader on `bytes`; scripts that read are paginated too (every `paginate_every`th).
fn read_all(bytes: &[u8], paginate: bool) {
    let text = String::from_utf8_lossy(bytes);
    let mut docs = Vec::new();
    if let Ok(f) = fountain::read(&text) {
        docs.push(f.doc);
    }
    if let Ok(d) = native::read(&text) {
        docs.push(d);
    }
    for as_script in [true, false] {
        if let Ok(d) = import::text(&text, as_script) {
            docs.push(d);
        }
        if let Ok(d) = import::rtf(bytes, as_script) {
            docs.push(d);
        }
    }
    let _ = import::looks_like_script(&text);
    if paginate {
        for d in docs.iter().take(2) {
            let _ = wordsmith_script_model::paginate::paginate(d);
        }
    }
}

#[test]
fn random_bytes_never_crash_a_reader() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for i in 0..300 {
        let len = rng.below(4096);
        let bytes: Vec<u8> = (0..len).map(|_| rng.next() as u8).collect();
        read_all(&bytes, i % 25 == 0);
        let _ = import::pdf(&bytes);
    }
}

#[test]
fn damaged_files_of_every_format_never_crash() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for (kind, seed) in seeds() {
        let n = if kind == "pdf" { 120 } else { 200 };
        for (i, m) in mutants(&seed, &mut rng, n).into_iter().enumerate() {
            if kind == "pdf" {
                let _ = import::pdf(&m);
            } else {
                read_all(&m, i % 20 == 0);
            }
        }
    }
}

#[test]
fn hostile_shapes_never_crash() {
    let big_line = "A".repeat(200_000);
    let deep_braces = "{n1}{r0}{p3}".repeat(5_000);
    let cases: Vec<String> = vec![
        String::new(),
        "\u{feff}".into(),
        "\0\0\0".into(),
        big_line.clone(),
        format!("INT. {big_line}"),
        format!("@{big_line}\n{big_line}"),
        "\n".repeat(100_000),
        "Wordsmith Script 3\n=== settings ===\n{}\n=== script ===\n@Action\n".to_string() + &deep_braces,
        "Wordsmith Script 3\n=== settings ===\n{\"appData\": {\"wordsmith.storyboard\": {\"beats\": [{\"id\": -1}]}, \"wordsmith.locks\": {\"pages\": [{\"id\": 1}]}}}\n=== script ===\n".into(),
        "Wordsmith Script 99\n".into(),
        "{\\rtf1 ".to_string() + &"{".repeat(10_000),
        "%PDF-1.7\n".to_string() + &"1 0 obj << /Kids [1 0 R] >> endobj\n".repeat(100),
        "Title:\n".to_string() + &"    x\n".repeat(10_000),
        "[[".repeat(10_000),
        "/*".repeat(10_000),
        "*_".repeat(50_000),
    ];
    for c in &cases {
        read_all(c.as_bytes(), c.len() < 50_000);
        let _ = import::pdf(c.as_bytes());
    }
}
