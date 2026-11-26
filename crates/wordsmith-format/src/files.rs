// SPDX-License-Identifier: MIT OR Apache-2.0
//! Scripts on disk: opening, importing, saving, exporting and backups. Every write is the core's
//! atomic write (`jcj_project::files`), so a failed or interrupted write never leaves a script
//! half-written, and every path passes the automation file gateway (`jcj_engine::file_access`).
//! Decision 0020.
//!
//! Backups are copies of the saved file in a backups folder, named
//! `<name> <YYYY-MM-DD HH.MM.SS> UTC.wss`; the oldest beyond the number to keep are removed.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use jcj_doc::Document;
use jcj_engine::file_access;

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
            let text = import::rtf_plain_text(&read_bytes(path)?).map_err(|_| format!("{}: not an RTF file", shown(path)))?;
            Ok(import::looks_like_script(&text))
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

/// The time part of a backup's name: `2026-11-19 14.03.22 UTC` (the core's backups,
/// `jcj_project::files`).
pub fn stamp(at: SystemTime) -> String {
    jcj_project::files::stamp(at)
}

/// The backups of the script named `stem` in `dir`, oldest first.
pub fn backups_of(dir: &Path, stem: &str) -> Vec<PathBuf> {
    jcj_project::files::backups_of(dir, Path::new(&format!("{stem}.wss")))
}

/// Copy the script just saved at `saved` into `dir` as a timestamped backup
/// (`<name> <stamp>[ n].wss`), then remove the oldest backups of the same script beyond `keep`
/// (1 to [`MAX_BACKUPS`]). Returns the new backup's path. The core's backups
/// (`jcj_project::files::backup`), named as Script's were.
pub fn backup(saved: &Path, dir: &Path, keep: usize, at: SystemTime) -> Result<PathBuf, String> {
    jcj_project::files::backup(saved, dir, keep.clamp(1, MAX_BACKUPS), at).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use jcj_doc::para_block;
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
        // Names that only look like backups are not taken for them.
        std::fs::write(backups.join("Draft 2026-11-19 15.03.22 UTC x.wss"), b"x").unwrap();
        std::fs::write(backups.join("Draft2 2026-11-19 15.03.22 UTC.wss"), b"x").unwrap();
        assert_eq!(backups_of(&backups, "Draft").len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
