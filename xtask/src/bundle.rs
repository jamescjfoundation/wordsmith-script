// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask bundle`: a release bundle of the app and the command-line tool for the host
//! (decision 0029), in `dist/`:
//!
//! - macOS: `Wordsmith Script.app` (the app, the command-line tool, an Info.plist that opens
//!   `.wss` and Fountain files, the app icon and the licenses in `Contents/Resources`), signed
//!   ad hoc, on a disk image with the licenses and a link to Applications;
//! - Linux: a folder with `bin/`, a `.desktop` file, the app icon and the licenses, as a `.tar.gz`;
//! - Windows: the two programs and the licenses, as a portable `.zip`.
//!
//! "The licenses" are the project's own (`DOCS`) and a `licenses/` folder with every
//! dependency's license text (`THIRD-PARTY-LICENSES.txt`, decision 0040) and the bundled fonts'
//! OFL files. The bundle stops if any of them is missing.
//!
//! Files are named `wordsmith-script-<version>-<os>-<arch>[-portable].<ext>`, the names the
//! in-app update looks for (decision 0038). Nothing is uploaded or published.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::licenses;
use crate::util::{TaskResult, cargo};

/// Files every bundle carries beside the programs.
const DOCS: [&str; 5] = ["LICENSE-MIT", "LICENSE-APACHE", "NOTICE", "ATTRIBUTION.md", "README.md"];

/// The app icon's sizes in `assets/brand/` (`icon-<size>.png`), and the macOS icon type of each.
const ICON_SIZES: [(u32, &[u8; 4]); 3] = [(256, b"ic08"), (512, b"ic09"), (1024, b"ic10")];

/// The name the icon is installed under (the desktop entry's `Icon=` and the app's id).
const ICON_NAME: &str = "wordsmith-script";

/// The app icon at `size` pixels.
fn icon_png(root: &Path, size: u32) -> Result<Vec<u8>, String> {
    let path = root.join(format!("assets/brand/icon-{size}.png"));
    std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// A macOS icon file holding the PNGs as they are: the `icns` header, then one
/// (type, length, data) entry per size. Lengths count the 8-byte headers.
pub fn icns(images: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let total = 8 + images.iter().map(|(_, png)| 8 + png.len()).sum::<usize>();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&(total as u32).to_be_bytes());
    for (kind, png) in images {
        out.extend_from_slice(*kind);
        out.extend_from_slice(&((8 + png.len()) as u32).to_be_bytes());
        out.extend_from_slice(png);
    }
    out
}

/// The folder in every bundle with the third-party license texts.
const LICENSES_DIR: &str = "licenses";

/// The license texts of the bundled fonts: `assets/fonts/OFL-*.txt`, sorted.
fn font_licenses(root: &Path) -> Result<Vec<PathBuf>, String> {
    let fonts = root.join("assets/fonts");
    let entries = std::fs::read_dir(&fonts).map_err(|e| format!("{}: {e}", fonts.display()))?;
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("OFL-") && n.ends_with(".txt")))
        .collect();
    found.sort();
    if found.is_empty() {
        return Err(format!("no OFL-*.txt font licenses in {}", fonts.display()));
    }
    Ok(found)
}

/// Puts the licenses in `dir`: `DOCS`, and `licenses/` with `third_party` and the font licenses.
fn legal_files(root: &Path, dir: &Path, third_party: &str, fonts: &[PathBuf]) -> TaskResult {
    for d in DOCS {
        copy(&root.join(d), &dir.join(d))?;
    }
    write(&dir.join(LICENSES_DIR).join(licenses::FILE_NAME), third_party)?;
    for font in fonts {
        let name = font.file_name().ok_or_else(|| format!("{}: no file name", font.display()))?;
        copy(font, &dir.join(LICENSES_DIR).join(name))?;
    }
    Ok(())
}

/// Fails unless `dir` carries every license file a bundle must ship.
pub fn check_legal_files(dir: &Path, fonts: &[PathBuf]) -> TaskResult {
    let mut missing = Vec::new();
    for d in DOCS {
        if !dir.join(d).is_file() {
            missing.push(d.to_owned());
        }
    }
    let third_party = dir.join(LICENSES_DIR).join(licenses::FILE_NAME);
    match std::fs::read_to_string(&third_party) {
        Ok(text) if text.starts_with("Third-party licenses") => {}
        _ => missing.push(format!("{LICENSES_DIR}/{}", licenses::FILE_NAME)),
    }
    for font in fonts {
        let name = font.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !dir.join(LICENSES_DIR).join(&name).is_file() {
            missing.push(format!("{LICENSES_DIR}/{name}"));
        }
    }
    if missing.is_empty() { Ok(()) } else { Err(format!("{} lacks {}", dir.display(), missing.join(", "))) }
}

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
  <key>CFBundleIconFile</key><string>wordsmith-script</string>
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
Icon=wordsmith-script
StartupWMClass=wordsmith-script
Terminal=false
Categories=Office;WordProcessor;
MimeType=text/x-fountain;application/x-wordsmith-script;
"
}

/// The release file name for `os` and `arch` (Rust's names), as the in-app update reads them:
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

/// Where Cargo puts release builds: `CARGO_TARGET_DIR` (relative to the workspace) if set.
fn release_dir(root: &Path, target_dir: Option<OsString>) -> PathBuf {
    match target_dir.filter(|d| !d.is_empty()) {
        Some(dir) => root.join(dir).join("release"),
        None => root.join("target/release"),
    }
}

/// Strips an extension the bundle adds, for the staging folder's name.
fn stem(file: &str) -> &str {
    file.strip_suffix(".dmg").or_else(|| file.strip_suffix("-portable.zip")).or_else(|| file.strip_suffix(".tar.gz")).unwrap_or(file)
}

pub fn bundle(root: &Path) -> TaskResult {
    cargo(root, &["build", "--release", "--locked", "-p", "wordsmith", "-p", "wordsmith-cli"])?;
    let third_party = licenses::third_party_licenses(root, crate::SHIPPED, crate::PRODUCT)?;
    let fonts = font_licenses(root)?;
    let version = version(root)?;
    let release = release_dir(root, std::env::var_os("CARGO_TARGET_DIR"));
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
            let mut images = Vec::new();
            for (size, kind) in ICON_SIZES {
                images.push((kind, icon_png(root, size)?));
            }
            let icon = app.join(format!("Resources/{ICON_NAME}.icns"));
            std::fs::create_dir_all(app.join("Resources")).map_err(|e| format!("{}: {e}", icon.display()))?;
            std::fs::write(&icon, icns(&images)).map_err(|e| format!("{}: {e}", icon.display()))?;
            // Inside the app too, so the notices stay with it once it is copied out of the disk image.
            legal_files(root, &app.join("Resources"), &third_party, &fonts)?;
            check_legal_files(&app.join("Resources"), &fonts)?;
            legal_files(root, &stage, &third_party, &fonts)?;
            check_legal_files(&stage, &fonts)?;
            // Ad hoc: the bundle is sealed, so the update's signature check can read it, but no
            // developer identity is claimed (a Developer ID needs a paid account, decisions 0029 and 0042).
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
            legal_files(root, &stage, &third_party, &fonts)?;
            check_legal_files(&stage, &fonts)?;
            // Windows' own tar writes zip archives.
            run(Command::new("tar").current_dir(&dist).args(["-a", "-c", "-f"]).arg(&out).arg(&name))?;
        }
        _ => {
            let stage = dist.join(&name);
            let _ = std::fs::remove_dir_all(&stage);
            copy(&app_bin, &stage.join("bin/wordsmith"))?;
            copy(&cli_bin, &stage.join("bin/wordsmith-cli"))?;
            write(&stage.join("share/applications/wordsmith-script.desktop"), desktop_entry())?;
            for (size, _) in ICON_SIZES {
                let icon = stage.join(format!("share/icons/hicolor/{size}x{size}/apps/{ICON_NAME}.png"));
                std::fs::create_dir_all(icon.parent().unwrap_or(&stage)).map_err(|e| format!("{}: {e}", icon.display()))?;
                std::fs::write(&icon, icon_png(root, size)?).map_err(|e| format!("{}: {e}", icon.display()))?;
            }
            legal_files(root, &stage, &third_party, &fonts)?;
            check_legal_files(&stage, &fonts)?;
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
        assert!(plist.contains(&format!("<key>CFBundleIconFile</key><string>{ICON_NAME}</string>")));
        assert!(desktop_entry().contains(&format!("Icon={ICON_NAME}\n")));
    }

    #[test]
    fn the_icon_sizes_are_in_the_repository_and_pack_into_an_icns() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut images = Vec::new();
        for (size, kind) in ICON_SIZES {
            let png = icon_png(&root, size).unwrap();
            assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
            // The IHDR width and height.
            assert_eq!(u32::from_be_bytes([png[16], png[17], png[18], png[19]]), size);
            assert_eq!(u32::from_be_bytes([png[20], png[21], png[22], png[23]]), size);
            images.push((kind, png));
        }
        let file = icns(&images);
        assert_eq!(&file[..4], b"icns");
        assert_eq!(u32::from_be_bytes([file[4], file[5], file[6], file[7]]) as usize, file.len());
        assert_eq!(&file[8..12], b"ic08");
        let first = u32::from_be_bytes([file[12], file[13], file[14], file[15]]) as usize;
        assert_eq!(&file[8 + first..8 + first + 4], b"ic09");
    }

    #[test]
    fn a_bundle_without_its_licenses_is_refused() {
        let base = std::env::temp_dir().join(format!("xtask-bundle-legal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (root, stage) = (base.join("root"), base.join("stage"));
        for d in DOCS {
            write(&root.join(d), d).unwrap();
        }
        write(&root.join("assets/fonts/OFL-Test.txt"), "SIL Open Font License").unwrap();
        write(&root.join("assets/fonts/Test-Regular.ttf"), "").unwrap();
        let fonts = font_licenses(&root).unwrap();
        assert_eq!(fonts, [root.join("assets/fonts/OFL-Test.txt")]);

        legal_files(&root, &stage, "Third-party licenses\n...", &fonts).unwrap();
        check_legal_files(&stage, &fonts).unwrap();

        std::fs::remove_file(stage.join(LICENSES_DIR).join(licenses::FILE_NAME)).unwrap();
        let err = check_legal_files(&stage, &fonts).unwrap_err();
        assert!(err.contains("THIRD-PARTY-LICENSES.txt"), "{err}");
        // An empty or wrong file counts as missing too.
        write(&stage.join(LICENSES_DIR).join(licenses::FILE_NAME), "").unwrap();
        assert!(check_legal_files(&stage, &fonts).is_err());

        legal_files(&root, &stage, "Third-party licenses\n...", &fonts).unwrap();
        std::fs::remove_file(stage.join("licenses/OFL-Test.txt")).unwrap();
        assert!(check_legal_files(&stage, &fonts).unwrap_err().contains("OFL-Test.txt"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn release_files_are_named_as_the_update_looks_for_them() {
        assert_eq!(asset_name("0.2.0", "macos", "aarch64"), "wordsmith-script-0.2.0-macos-aarch64.dmg");
        assert_eq!(asset_name("0.2.0", "windows", "x86_64"), "wordsmith-script-0.2.0-windows-x64-portable.zip");
        assert_eq!(asset_name("0.2.0-rc.1", "linux", "x86_64"), "wordsmith-script-0.2.0-rc.1-linux-x86_64.tar.gz");
        assert_eq!(stem(&asset_name("0.2.0", "windows", "aarch64")), "wordsmith-script-0.2.0-windows-arm64");
    }

    #[test]
    fn programs_are_taken_from_cargos_target_folder() {
        let root = Path::new("/repo");
        assert_eq!(release_dir(root, None), root.join("target/release"));
        assert_eq!(release_dir(root, Some(OsString::new())), root.join("target/release"));
        assert_eq!(release_dir(root, Some("out".into())), root.join("out/release"));
        let shared = std::env::temp_dir().join("shared-target");
        assert_eq!(release_dir(root, Some(shared.clone().into_os_string())), shared.join("release"));
    }
}
