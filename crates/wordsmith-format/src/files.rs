// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scripts on disk: opening, importing, saving, exporting and backups. Every write goes through
//! craft-atomic, so a failed or interrupted write never leaves a script half-written, and every
//! path passes the automation file gateway (`wordsmith_engine::file_access`). Decision 0020.
//!
//! Backups are copies of the saved file in a backups folder, named
//! `<name> <YYYY-MM-DD HH.MM.SS> UTC.wss`; the oldest beyond the number to keep are removed.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use wordsmith_doc::Document;
use wordsmith_engine::file_access;

use crate::{export, fountain, import, native};

/// Largest file opened or imported, bytes.
pub const MAX_FILE_BYTES: u64 = 64 << 20;
/// Backups kept by default, and at most (SPEC: 1,000 by default, up to 10,000).
pub const DEFAULT_BACKUPS: usize = 1_000;
pub const MAX_BACKUPS: usize = 10_000;

/// The kinds of file a script is read from or written to, by extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `.wss`, the native format.
    Native,
    Fountain,
    Text,
    Rtf,
    Pdf,
    Html,
}

impl Kind {
    /// The kind a path's extension names, if any.
    pub fn of(path: &Path) -> Option<Kind> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Some(match ext.as_str() {
            // `.wst` is a native file saved as a template.
            "wss" | "wst" => Kind::Native,
            "fountain" | "spmd" => Kind::Fountain,
            "txt" | "text" => Kind::Text,
            "rtf" => Kind::Rtf,
            "pdf" => Kind::Pdf,
            "html" | "htm" => Kind::Html,
            _ => return None,
        })
    }

    /// The extension written for this kind.
    pub fn extension(self) -> &'static str {
        match self {
            Kind::Native => crate::NATIVE_EXTENSION,
            Kind::Fountain => "fountain",
            Kind::Text => "txt",
            Kind::Rtf => "rtf",
            Kind::Pdf => "pdf",
            Kind::Html => "html",
        }
    }

    /// A name for file dialogs.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Native => "Wordsmith Script",
            Kind::Fountain => "Fountain",
            Kind::Text => "Plain Text",
            Kind::Rtf => "Rich Text (RTF)",
            Kind::Pdf => "PDF",
            Kind::Html => "Web Page (HTML)",
        }
    }

    /// Whether importing this kind asks if the text is a script.
    pub fn asks_if_script(self) -> bool {
        matches!(self, Kind::Text | Kind::Rtf)
    }
}

/// `path` with `kind`'s extension, unless it already has it.
pub fn with_extension(path: &Path, kind: Kind) -> PathBuf {
    if Kind::of(path) == Some(kind) { path.to_path_buf() } else { path.with_extension(kind.extension()) }
}

fn shown(path: &Path) -> String {
    path.display().to_string()
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let text = shown(path);
    file_access::check_read(&text)?;
    let meta = std::fs::metadata(path).map_err(|e| format!("{text}: {e}"))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(format!("{text}: the file is larger than {} MB", MAX_FILE_BYTES >> 20));
    }
    std::fs::read(path).map_err(|e| format!("{text}: {e}"))
}

fn read_text(path: &Path) -> Result<String, String> {
    let bytes = read_bytes(path)?;
    // Text files from other programs may not be UTF-8; damaged bytes become U+FFFD.
    Ok(String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))
}

/// Write `bytes` to `path` atomically, through the file gateway. Every file a command writes goes
/// through here (or [`wordsmith_script::export::write_file`], which this calls).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    wordsmith_script::export::write_file(&shown(path), bytes)
}

/// Open a script file: the native format or Fountain.
pub fn open(path: &Path) -> Result<Document, String> {
    match Kind::of(path) {
        Some(Kind::Native) => native::read(&read_text(path)?).map_err(|e| format!("{}: {e}", shown(path))),
        Some(Kind::Fountain) => fountain::read(&read_text(path)?).map(|f| f.doc).map_err(|e| format!("{}: {e}", shown(path))),
        Some(other) => Err(format!("{}: {} files are imported, not opened", shown(path), other.label())),
        None => Err(format!("{}: not a script file this app reads", shown(path))),
    }
}

/// Import a file into a new script: plain text or RTF (`as_script` is the answer to "is this a
/// script?"), a text-based PDF, or Fountain.
pub fn import_file(path: &Path, as_script: bool) -> Result<Document, String> {
    let named = |e: import::ImportError| format!("{}: {e}", shown(path));
    match Kind::of(path) {
        Some(Kind::Text) => import::text(&read_text(path)?, as_script).map_err(named),
        Some(Kind::Rtf) => import::rtf(&read_bytes(path)?, as_script).map_err(named),
        Some(Kind::Pdf) => import::pdf(&read_bytes(path)?).map_err(named),
        Some(Kind::Fountain | Kind::Native) => open(path),
        Some(Kind::Html) | None => Err(format!("{}: not a file this app imports", shown(path))),
    }
}

/// The default answer to "is this a script?" for a plain-text or RTF file.
pub fn guess_is_script(path: &Path) -> Result<bool, String> {
    match Kind::of(path) {
        Some(Kind::Rtf) => {
            let doc = craft_rtf::read(&read_bytes(path)?).map_err(|_| format!("{}: not an RTF file", shown(path)))?;
            let text: Vec<String> = doc.paras().map(craft_rtf::Para::text).collect();
            Ok(import::looks_like_script(&text.join("\n\n")))
        }
        _ => Ok(import::looks_like_script(&read_text(path)?)),
    }
}

/// Save `doc` in the native format at `path`.
pub fn save(path: &Path, doc: &Document) -> Result<(), String> {
    let text = native::write(doc).map_err(|e| format!("{}: {e}", shown(path)))?;
    write_atomic(path, text.as_bytes())
}

/// Export `doc` as `kind` to `path`.
pub fn export_to(path: &Path, doc: &Document, kind: Kind) -> Result<(), String> {
    let bytes = match kind {
        Kind::Native => native::write(doc).map_err(|e| format!("{}: {e}", shown(path)))?.into_bytes(),
        Kind::Fountain => fountain::write(doc).into_bytes(),
        Kind::Text => export::text(doc).into_bytes(),
        Kind::Rtf => export::rtf(doc).into_bytes(),
        Kind::Html => export::html(doc).into_bytes(),
        Kind::Pdf => wordsmith_script::export::pdf(doc)?,
    };
    write_atomic(path, &bytes)
}

/// The time part of a backup's name: `2026-11-19 14.03.22 UTC`.
pub fn stamp(at: SystemTime) -> String {
    let secs = at.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, mo, d, h, mi, s) = wordsmith_script::dates::civil(secs);
    format!("{y:04}-{mo:02}-{d:02} {h:02}.{mi:02}.{s:02} UTC")
}

/// Whether `file` is a backup of a script named `stem` (`<stem> <stamp>[ n].wss`).
fn is_backup_of(file: &str, stem: &str) -> bool {
    let Some(rest) = file.strip_prefix(stem).and_then(|r| r.strip_prefix(' ')) else { return false };
    let Some(rest) = rest.strip_suffix(".wss") else { return false };
    let b = rest.as_bytes();
    // "YYYY-MM-DD HH.MM.SS UTC", then possibly " n" for a second backup in the same second.
    b.len() >= 23
        && rest.get(..23).is_some_and(|s| {
            s.char_indices().all(|(i, c)| match i {
                4 | 7 => c == '-',
                10 | 19 => c == ' ',
                13 | 16 => c == '.',
                20..=22 => "UTC".chars().nth(i - 20) == Some(c),
                _ => c.is_ascii_digit(),
            })
        })
        && rest
            .get(23..)
            .is_some_and(|tail| tail.is_empty() || tail.strip_prefix(' ').is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())))
}

/// The backups of `stem` in `dir`, oldest first.
pub fn backups_of(dir: &Path, stem: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut found: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            is_backup_of(&name, stem).then(|| (name, e.path()))
        })
        .collect();
    // The stamp sorts as text; a numbered copy comes after the one without a number.
    found.sort_by(|a, b| {
        let key = |s: &str| {
            let tail = s.strip_suffix(".wss").unwrap_or(s);
            let (base, n) = match tail.get(stem.len() + 24..) {
                Some(n) if !n.is_empty() => (tail.get(..stem.len() + 24).unwrap_or(tail).to_string(), n.trim().parse::<u64>().unwrap_or(0)),
                _ => (tail.to_string(), 0),
            };
            (base, n)
        };
        key(&a.0).cmp(&key(&b.0))
    });
    found.into_iter().map(|(_, p)| p).collect()
}

/// Copy the script just saved at `saved` into `dir` as a timestamped backup, then remove the
/// oldest backups of the same script beyond `keep` (1 to [`MAX_BACKUPS`]). Returns the new
/// backup's path.
pub fn backup(saved: &Path, dir: &Path, keep: usize, at: SystemTime) -> Result<PathBuf, String> {
    let keep = keep.clamp(1, MAX_BACKUPS);
    let stem = saved.file_stem().and_then(|s| s.to_str()).filter(|s| !s.is_empty()).unwrap_or("Untitled");
    let bytes = read_bytes(saved)?;
    file_access::check_write(&shown(dir))?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", shown(dir)))?;
    let base = format!("{stem} {}", stamp(at));
    let mut written = None;
    for n in 0..100u32 {
        let name = if n == 0 { format!("{base}.wss") } else { format!("{base} {}.wss", n + 1) };
        let path = dir.join(name);
        file_access::check_write(&shown(&path))?;
        match craft_atomic::write_new(&path, &bytes) {
            Ok(()) => {
                written = Some(path);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("{}: {e}", shown(&path))),
        }
    }
    let written = written.ok_or_else(|| format!("{}: too many backups in one second", shown(dir)))?;
    let all = backups_of(dir, stem);
    if all.len() > keep {
        for old in all.get(..all.len() - keep).unwrap_or_default() {
            if *old != written {
                let _ = std::fs::remove_file(old);
            }
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use wordsmith_doc::para_block;
    use wordsmith_script::Elements;
    use wordsmith_script::template::{new_script, paragraph};

    use super::*;

    /// A fresh folder for one test.
    fn temp_dir(name: &str) -> PathBuf {
        let n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("wordsmith-files-{name}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn script() -> Document {
        let mut doc = new_script();
        doc.core.title = "Test".into();
        doc.body = [("SceneHeading", "INT. KITCHEN - NIGHT"), ("Action", "Rain."), ("Character", "ANNA"), ("Dialogue", "Hello.")]
            .iter()
            .map(|(e, t)| para_block(paragraph(e, t)))
            .collect();
        doc
    }

    fn lines(doc: &Document) -> Vec<(String, String)> {
        let e = Elements::of_doc(doc);
        doc.body.iter().filter_map(|b| b.as_para()).map(|p| (e.of_para(p).id.clone(), p.text.clone())).collect()
    }

    #[test]
    fn kinds_by_extension() {
        assert_eq!(Kind::of(Path::new("a.WSS")), Some(Kind::Native));
        assert_eq!(Kind::of(Path::new("a.spmd")), Some(Kind::Fountain));
        assert_eq!(Kind::of(Path::new("a.docx")), None);
        assert_eq!(with_extension(Path::new("dir/Draft"), Kind::Native), Path::new("dir/Draft.wss"));
        assert_eq!(with_extension(Path::new("Draft.wss"), Kind::Native), Path::new("Draft.wss"));
        assert_eq!(with_extension(Path::new("Draft.v2"), Kind::Fountain), Path::new("Draft.fountain"));
    }

    #[test]
    fn save_open_export_and_import() {
        let dir = temp_dir("roundtrip");
        let doc = script();
        let path = dir.join("Draft.wss");
        save(&path, &doc).unwrap();
        assert_eq!(lines(&open(&path).unwrap()), lines(&doc));
        for kind in [Kind::Fountain, Kind::Text, Kind::Rtf, Kind::Pdf] {
            let out = dir.join(format!("Draft.{}", kind.extension()));
            export_to(&out, &doc, kind).unwrap();
            assert_eq!(lines(&import_file(&out, true).unwrap()), lines(&doc), "{kind:?}");
        }
        export_to(&dir.join("Draft.html"), &doc, Kind::Html).unwrap();
        assert!(std::fs::read_to_string(dir.join("Draft.html")).unwrap().starts_with("<!DOCTYPE html>"));
        assert!(guess_is_script(&dir.join("Draft.txt")).unwrap());
        assert!(guess_is_script(&dir.join("Draft.rtf")).unwrap());
        assert!(open(&dir.join("Draft.txt")).unwrap_err().contains("imported, not opened"));
        assert!(open(&dir.join("missing.wss")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_save_leaves_the_old_file() {
        let dir = temp_dir("failed");
        let path = dir.join("Draft.wss");
        save(&path, &script()).unwrap();
        let before = std::fs::read(&path).unwrap();
        // Writing into a folder that does not exist fails without touching anything else.
        assert!(save(&dir.join("no/such/folder/Draft.wss"), &script()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stamps_are_utc_dates() {
        assert_eq!(stamp(UNIX_EPOCH), "1970-01-01 00.00.00 UTC");
        assert_eq!(stamp(UNIX_EPOCH + Duration::from_secs(951_782_400)), "2000-02-29 00.00.00 UTC");
        assert_eq!(stamp(UNIX_EPOCH + Duration::from_secs(1_795_100_602)), "2026-11-19 15.03.22 UTC");
    }

    #[test]
    fn backups_are_named_kept_and_pruned() {
        let dir = temp_dir("backups");
        let saved = dir.join("Draft.wss");
        save(&saved, &script()).unwrap();
        let backups = dir.join("Backups");
        let t0 = UNIX_EPOCH + Duration::from_secs(1_795_100_602);
        let first = backup(&saved, &backups, 3, t0).unwrap();
        assert_eq!(first.file_name().and_then(|n| n.to_str()), Some("Draft 2026-11-19 15.03.22 UTC.wss"));
        assert_eq!(std::fs::read(&first).unwrap(), std::fs::read(&saved).unwrap());
        // Two in the same second get a number; never overwritten.
        let second = backup(&saved, &backups, 3, t0).unwrap();
        assert_eq!(second.file_name().and_then(|n| n.to_str()), Some("Draft 2026-11-19 15.03.22 UTC 2.wss"));
        backup(&saved, &backups, 3, t0 + Duration::from_secs(60)).unwrap();
        let newest = backup(&saved, &backups, 3, t0 + Duration::from_secs(120)).unwrap();
        let kept = backups_of(&backups, "Draft");
        assert_eq!(kept.len(), 3, "the oldest is removed");
        assert!(!kept.contains(&first) && kept.contains(&second) && kept.last() == Some(&newest));
        // Other scripts' backups and other files are left alone.
        std::fs::write(backups.join("Other 2026-11-19 15.03.22 UTC.wss"), b"x").unwrap();
        std::fs::write(backups.join("Draft notes.txt"), b"x").unwrap();
        backup(&saved, &backups, 1, t0 + Duration::from_secs(180)).unwrap();
        assert_eq!(backups_of(&backups, "Draft").len(), 1);
        assert!(backups.join("Other 2026-11-19 15.03.22 UTC.wss").exists() && backups.join("Draft notes.txt").exists());
        assert!(!is_backup_of("Draft 2026-11-19 15.03.22 UTC x.wss", "Draft"));
        assert!(!is_backup_of("Draft2 2026-11-19 15.03.22 UTC.wss", "Draft"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
