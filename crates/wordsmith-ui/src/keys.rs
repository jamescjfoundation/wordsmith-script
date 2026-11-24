// SPDX-License-Identifier: MIT OR Apache-2.0
//! Key presses to shortcut strings, the portable form commands declare (`Mod+Shift+Left`,
//! `Enter`, `Mod+3`). `Mod` is Command on macOS and Ctrl elsewhere. On macOS the platform's own
//! text-editing keys are translated to the portable ones first: Option+Arrow moves by word,
//! Command+Left/Right goes to the line's ends, Command+Up/Down to the script's.

use egui::{Key, Modifiers};

/// The portable name of `key`, if it can be part of a shortcut.
pub fn key_name(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::ArrowLeft => "Left",
        Key::ArrowRight => "Right",
        Key::ArrowUp => "Up",
        Key::ArrowDown => "Down",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::Enter => "Enter",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::Delete => "Delete",
        Key::Escape => "Escape",
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        _ => return None,
    })
}

/// The shortcut string for a key press: modifiers in the order `Mod`, `Ctrl` (macOS only,
/// where Control is not `Mod`), `Alt`, `Shift`, then the key. `mac` applies macOS's own keys.
pub fn shortcut(key: Key, m: Modifiers, mac: bool) -> Option<String> {
    let name = key_name(key)?;
    let (mut cmd, mut alt, shift) = (m.command, m.alt, m.shift);
    let ctrl = mac && m.ctrl;
    let mut name = name;
    if mac {
        match (cmd, alt, name) {
            // Option+Left/Right: by word.
            (false, true, "Left" | "Right") => {
                alt = false;
                cmd = true;
            }
            // Command+Left/Right: the line's ends.
            (true, false, "Left") => {
                cmd = false;
                name = "Home";
            }
            (true, false, "Right") => {
                cmd = false;
                name = "End";
            }
            // Command+Up/Down: the script's ends.
            (true, false, "Up") => name = "Home",
            (true, false, "Down") => name = "End",
            _ => {}
        }
    }
    let mut out = String::new();
    for (on, part) in [(cmd, "Mod+"), (ctrl, "Ctrl+"), (alt, "Alt+"), (shift, "Shift+")] {
        if on {
            out.push_str(part);
        }
    }
    out.push_str(name);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(command: bool, alt: bool, shift: bool) -> Modifiers {
        Modifiers { alt, ctrl: command, shift, mac_cmd: false, command }
    }

    #[test]
    fn portable_shortcuts() {
        assert_eq!(shortcut(Key::Enter, Modifiers::NONE, false).as_deref(), Some("Enter"));
        assert_eq!(shortcut(Key::Num3, m(true, false, false), false).as_deref(), Some("Mod+3"));
        assert_eq!(shortcut(Key::Z, m(true, false, true), false).as_deref(), Some("Mod+Shift+Z"));
        assert_eq!(shortcut(Key::L, m(true, true, false), false).as_deref(), Some("Mod+Alt+L"));
        assert_eq!(shortcut(Key::ArrowLeft, m(true, false, true), false).as_deref(), Some("Mod+Shift+Left"));
        assert_eq!(shortcut(Key::Space, Modifiers::NONE, false), None);
    }

    #[test]
    fn macos_text_keys_become_portable_ones() {
        let opt = Modifiers { alt: true, ..Modifiers::NONE };
        let cmd = Modifiers { command: true, mac_cmd: true, ..Modifiers::NONE };
        let cmd_shift = Modifiers { shift: true, ..cmd };
        assert_eq!(shortcut(Key::ArrowLeft, opt, true).as_deref(), Some("Mod+Left"));
        assert_eq!(shortcut(Key::ArrowRight, Modifiers { shift: true, ..opt }, true).as_deref(), Some("Mod+Shift+Right"));
        assert_eq!(shortcut(Key::ArrowLeft, cmd, true).as_deref(), Some("Home"));
        assert_eq!(shortcut(Key::ArrowRight, cmd_shift, true).as_deref(), Some("Shift+End"));
        assert_eq!(shortcut(Key::ArrowUp, cmd, true).as_deref(), Some("Mod+Home"));
        assert_eq!(shortcut(Key::ArrowDown, cmd_shift, true).as_deref(), Some("Mod+Shift+End"));
        assert_eq!(shortcut(Key::Num1, cmd, true).as_deref(), Some("Mod+1"));
        // Control on a Mac is its own modifier, not Mod.
        assert_eq!(shortcut(Key::A, Modifiers { ctrl: true, ..Modifiers::NONE }, true).as_deref(), Some("Ctrl+A"));
    }

    #[test]
    fn every_registry_shortcut_can_be_typed() {
        // Each command's shortcut splits into known modifiers and a key this module names.
        let r = wordsmith_script::registry();
        let keys: Vec<&str> = [
            Key::ArrowLeft,
            Key::ArrowRight,
            Key::ArrowUp,
            Key::ArrowDown,
            Key::Home,
            Key::End,
            Key::Enter,
            Key::Tab,
            Key::Backspace,
            Key::Delete,
            Key::Num0,
            Key::Num9,
            Key::A,
            Key::N,
            Key::Y,
            Key::Z,
        ]
        .into_iter()
        .filter_map(key_name)
        .collect();
        for spec in r.all().iter().filter(|s| !s.shortcut.is_empty()) {
            for alt in spec.shortcut.split(" / ") {
                let key = alt.rsplit('+').next().unwrap_or("");
                assert!(keys.contains(&key) || key.len() == 1 || key.starts_with('F'), "{}: {alt}", spec.id);
                for part in alt.split('+').rev().skip(1) {
                    assert!(["Mod", "Ctrl", "Alt", "Shift"].contains(&part), "{}: {alt}", spec.id);
                }
            }
        }
    }
}
