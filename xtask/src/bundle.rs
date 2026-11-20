// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask bundle`: a release bundle of the app and the command-line tool for the host
//! (decision 0029), in `dist/`:
//!
//! - macOS: `Wordsmith Script.app` (the app, the command-line tool, an Info.plist that opens
//!   `.wss` and Fountain files), signed ad hoc, on a disk image with the licenses and a link to
//!   Applications;
//! - Linux: a folder with `bin/`, a `.desktop` file and the licenses, as a `.tar.gz`;
//! - Windows: the two programs and the licenses, as a portable `.zip`.
//!
//! Files are named `wordsmith-script-<version>-<os>-<arch>[-portable].<ext>`, the names the
//! in-app update looks for (decision 0038). Nothing is uploaded or published.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{TaskResult, cargo};

/// Files every bundle carries beside the programs.
const DOCS: [&str; 5] = ["LICENSE-MIT", "LICENSE-APACHE", "NOTICE", "ATTRIBUTION.md", "README.md"];

fn version(root: &Path) -> Result<String, String> {
    let toml = std::fs::read_to_string(root.join("Cargo.toml")).map_err(|e| format!("Cargo.toml: {e}"))?;
    let in_workspace_package = toml.split("[workspace.package]").nth(1).ok_or("no [workspace.package] in Cargo.toml")?;
    let line = in_workspace_package.lines().find(|l| l.trim_start().starts_with("version")).ok_or("no workspace version")?;
    Ok(line.split('"').nth(1).ok_or("unreadable workspace version")?.to_string())
}

fn copy(from: &Path, to: &Path) -> TaskResult {
    if let Some(dir) = to.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::copy(from, to).map(|_| ()).map_err(|e| format!("{} → {}: {e}", from.display(), to.display()))
}

fn write(path: &Path, text: &str) -> TaskResult {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn run(cmd: &mut Command) -> TaskResult {
    let shown = format!("{cmd:?}");
    let status = cmd.status().map_err(|e| format!("{shown}: {e}"))?;
    if status.success() { Ok(()) } else { Err(format!("{shown} exited with {status}")) }
}

/// The Info.plist of the macOS app: its name, version, the two document types it opens.
pub fn info_plist(version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Wordsmith Script</string>
  <key>CFBundleDisplayName</key><string>Wordsmith Script</string>
  <key>CFBundleIdentifier</key><string>org.wordsmith-script.app</string>
  <key>CFBundleExecutable</key><string>wordsmith</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>{version}</string>
  <key>CFBundleVersion</key><string>{version}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>Wordsmith Script</string>
      <key>CFBundleTypeRole</key><string>Editor</string>
      <key>CFBundleTypeExtensions</key><array><string>wss</string></array>
    </dict>
    <dict>
      <key>CFBundleTypeName</key><string>Fountain screenplay</string>
      <key>CFBundleTypeRole</key><string>Editor</string>
      <key>CFBundleTypeExtensions</key><array><string>fountain</string><string>spmd</string></array>
    </dict>
  </array>
</dict>
</plist>
"#
    )
}

/// The Linux desktop entry.
pub fn desktop_entry() -> &'static str {
    "[Desktop Entry]
Type=Application
Name=Wordsmith Script
Comment=Write screenplays
Exec=wordsmith %f
Terminal=false
Categories=Office;WordProcessor;
MimeType=text/x-fountain;application/x-wordsmith-script;
"
}

/// The release file name for `os` and `arch` (Rust's names), as craft-update reads them:
/// `macos-aarch64`, `windows-x64`, `linux-x86_64`.
pub fn asset_name(version: &str, os: &str, arch: &str) -> String {
    match os {
        "macos" => format!("wordsmith-script-{version}-macos-{arch}.dmg"),
        "windows" => {
            let arch = match arch {
                "x86_64" => "x64",
                "aarch64" => "arm64",
                "x86" => "x86",
                other => other,
            };
            format!("wordsmith-script-{version}-windows-{arch}-portable.zip")
        }
        _ => format!("wordsmith-script-{version}-{os}-{arch}.tar.gz"),
    }
}

/// Strips an extension the bundle adds, for the staging folder's name.
fn stem(file: &str) -> &str {
    file.strip_suffix(".dmg").or_else(|| file.strip_suffix("-portable.zip")).or_else(|| file.strip_suffix(".tar.gz")).unwrap_or(file)
}

pub fn bundle(root: &Path) -> TaskResult {
    cargo(root, &["build", "--release", "--locked", "-p", "wordsmith", "-p", "wordsmith-cli"])?;
    let version = version(root)?;
    let release = root.join("target/release");
    let exe = std::env::consts::EXE_SUFFIX;
    let arch = std::env::consts::ARCH;
    let dist = root.join("dist");
    std::fs::create_dir_all(&dist).map_err(|e| format!("{}: {e}", dist.display()))?;
    let app_bin = release.join(format!("wordsmith{exe}"));
    let cli_bin = release.join(format!("wordsmith-cli{exe}"));
    let file = asset_name(&version, std::env::consts::OS, arch);
    let name = stem(&file).to_string();
    let out: PathBuf = dist.join(&file);
    let _ = std::fs::remove_file(&out);
    match std::env::consts::OS {
        "macos" => {
            let stage = dist.join(&name);
            let _ = std::fs::remove_dir_all(&stage);
            let app = stage.join("Wordsmith Script.app/Contents");
            copy(&app_bin, &app.join("MacOS/wordsmith"))?;
            copy(&cli_bin, &app.join("MacOS/wordsmith-cli"))?;
            write(&app.join("Info.plist"), &info_plist(&version))?;
            for d in DOCS {
                copy(&root.join(d), &stage.join(d))?;
            }
            // Ad hoc: the bundle is sealed, so the update's signature check can read it, but no
            // developer identity is claimed (a Developer ID is a stop rule, decision 0029).
            run(Command::new("codesign").args(["--force", "--deep", "--sign", "-"]).arg(stage.join("Wordsmith Script.app")))?;
            #[cfg(unix)]
            std::os::unix::fs::symlink("/Applications", stage.join("Applications")).map_err(|e| format!("Applications link: {e}"))?;
            run(Command::new("hdiutil")
                .args(["create", "-volname", "Wordsmith Script", "-fs", "HFS+", "-format", "UDZO", "-ov", "-srcfolder"])
                .arg(&stage)
                .arg(&out))?;
        }
        "windows" => {
            let stage = dist.join(&name);
            let _ = std::fs::remove_dir_all(&stage);
            copy(&app_bin, &stage.join("wordsmith.exe"))?;
            copy(&cli_bin, &stage.join("wordsmith-cli.exe"))?;
            for d in DOCS {
                copy(&root.join(d), &stage.join(d))?;
            }
            // Windows' own tar writes zip archives.
            run(Command::new("tar").current_dir(&dist).args(["-a", "-c", "-f"]).arg(&out).arg(&name))?;
        }
        _ => {
            let stage = dist.join(&name);
            let _ = std::fs::remove_dir_all(&stage);
            copy(&app_bin, &stage.join("bin/wordsmith"))?;
            copy(&cli_bin, &stage.join("bin/wordsmith-cli"))?;
            write(&stage.join("share/applications/wordsmith-script.desktop"), desktop_entry())?;
            for d in DOCS {
                copy(&root.join(d), &stage.join(d))?;
            }
            run(Command::new("tar").current_dir(&dist).arg("-czf").arg(&out).arg(&name))?;
        }
    }
    println!("\nxtask: bundle written to {}", out.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_describes_itself_and_its_documents() {
        let plist = info_plist("0.1.0");
        assert!(plist.contains("<string>0.1.0</string>") && plist.contains("<string>wss</string>") && plist.contains("<string>fountain</string>"));
        assert!(desktop_entry().contains("Exec=wordsmith %f"));
    }

    #[test]
    fn release_files_are_named_as_the_update_looks_for_them() {
        assert_eq!(asset_name("0.2.0", "macos", "aarch64"), "wordsmith-script-0.2.0-macos-aarch64.dmg");
        assert_eq!(asset_name("0.2.0", "windows", "x86_64"), "wordsmith-script-0.2.0-windows-x64-portable.zip");
        assert_eq!(asset_name("0.2.0-rc.1", "linux", "x86_64"), "wordsmith-script-0.2.0-rc.1-linux-x86_64.tar.gz");
        assert_eq!(stem(&asset_name("0.2.0", "windows", "aarch64")), "wordsmith-script-0.2.0-windows-arm64");
    }
}
