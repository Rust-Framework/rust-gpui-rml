//! Keyboard input handling for the terminal emulator.
//!
//! This module provides [`keystroke_to_bytes`], which converts GPUI keyboard
//! events into terminal escape sequences that can be written to the PTY.
//!
//! # Key Mappings
//!
//! ## Special Keys
//!
//! | Key | Sequence | Notes |
//! |-----|----------|-------|
//! | Enter | `\r` (0x0D) | Carriage return |
//! | Escape | `\x1b` (0x1B) | ESC |
//! | Backspace | `\x7f` (0x7F) | DEL |
//! | Tab | `\t` (0x09) | Horizontal tab |
//! | Shift+Tab | `\x1b[Z` | Backtab |
//! | Space | ` ` (0x20) | Space |
//! | Ctrl+Space | `\x00` | NUL |
//!
//! ## Arrow Keys
//!
//! Arrow key sequences depend on application cursor mode:
//!
//! | Key | Normal Mode | App Cursor Mode |
//! |-----|-------------|-----------------|
//! | Up | `\x1b[A` | `\x1bOA` |
//! | Down | `\x1b[B` | `\x1bOB` |
//! | Right | `\x1b[C` | `\x1bOC` |
//! | Left | `\x1b[D` | `\x1bOD` |
//!
//! ## Navigation Keys
//!
//! | Key | Sequence |
//! |-----|----------|
//! | Home | `\x1b[H` |
//! | End | `\x1b[F` |
//! | PageUp | `\x1b[5~` |
//! | PageDown | `\x1b[6~` |
//! | Insert | `\x1b[2~` |
//! | Delete | `\x1b[3~` |
//!
//! ## Function Keys
//!
//! | Key | Sequence |
//! |-----|----------|
//! | F1-F4 | `\x1bOP` - `\x1bOS` |
//! | F5-F12 | `\x1b[15~` - `\x1b[24~` |
//!
//! ## Control Combinations
//!
//! Ctrl+A through Ctrl+Z map to ASCII control characters 0x01-0x1A:
//!
//! | Combination | Byte |
//! |-------------|------|
//! | Ctrl+A | 0x01 |
//! | Ctrl+C | 0x03 (interrupt) |
//! | Ctrl+D | 0x04 (EOF) |
//! | Ctrl+Z | 0x1A (suspend) |
//!
//! ## Alt Combinations
//!
//! Alt+key sends ESC followed by the key: `\x1b` + key
//!
//! # Terminal Mode Effects
//!
//! The [`TermMode`] flags affect key sequences:
//!
//! - **APP_CURSOR**: Changes arrow key sequences from CSI to SS3 format
//!
//! # Example
//!
//! ```
//! use gpui::Keystroke;
//! use alacritty_terminal::term::TermMode;
//! use rml_ui_term::input::keystroke_to_bytes;
//!
//! // Enter key
//! let keystroke = Keystroke::parse("enter").unwrap();
//! assert_eq!(keystroke_to_bytes(&keystroke, TermMode::empty()), Some(b"\r".to_vec()));
//!
//! // Ctrl+C (interrupt)
//! let keystroke = Keystroke::parse("ctrl-c").unwrap();
//! assert_eq!(keystroke_to_bytes(&keystroke, TermMode::empty()), Some(vec![0x03]));
//! ```

use alacritty_terminal::term::TermMode;
use gpui::{Keystroke, Modifiers};

/// XKB / GPUI logical key names and their unshifted / shifted characters (US QWERTY).
///
/// GPUI often reports punctuation as multi-character `key` names (e.g. `"minus"`) with no
/// `key_char`, especially on non-US layouts or when keys are synthesized from bindings.
const PUNCTUATION_KEYS: &[(&str, &str, &str)] = &[
    ("minus", "-", "_"),
    ("underscore", "_", "_"),
    ("equal", "=", "+"),
    ("plus", "+", "+"),
    ("comma", ",", "<"),
    ("less", "<", "<"),
    ("period", ".", ">"),
    ("greater", ">", ">"),
    ("slash", "/", "?"),
    ("question", "?", "?"),
    ("semicolon", ";", ":"),
    ("colon", ":", ":"),
    ("apostrophe", "'", "\""),
    ("quotedbl", "\"", "\""),
    ("grave", "`", "~"),
    ("asciitilde", "~", "~"),
    ("bracketleft", "[", "{"),
    ("braceleft", "{", "{"),
    ("bracketright", "]", "}"),
    ("braceright", "}", "}"),
    ("backslash", "\\", "|"),
    ("bar", "|", "|"),
    ("exclam", "!", "!"),
    ("at", "@", "@"),
    ("numbersign", "#", "#"),
    ("dollar", "$", "$"),
    ("percent", "%", "%"),
    ("asciicircum", "^", "^"),
    ("ampersand", "&", "&"),
    ("asterisk", "*", "*"),
    ("parenleft", "(", "("),
    ("parenright", ")", ")"),
];

fn punctuation_bytes(key: &str, shift: bool) -> Option<Vec<u8>> {
    for (name, normal, shifted) in PUNCTUATION_KEYS {
        if key == *name || key == *normal {
            let text = if shift { shifted } else { normal };
            return Some(text.as_bytes().to_vec());
        }
    }
    None
}

fn is_typing_modifier_state(modifiers: &Modifiers) -> bool {
    !modifiers.control && !modifiers.alt && !modifiers.function
}

/// Printable bytes for keys that should go to the shell (not control/alt/meta chords).
fn printable_bytes(keystroke: &Keystroke) -> Option<Vec<u8>> {
    if !is_typing_modifier_state(&keystroke.modifiers) {
        return None;
    }

    if let Some(key_char) = &keystroke.key_char {
        if !key_char.is_empty() {
            return Some(key_char.as_bytes().to_vec());
        }
    }

    if let Some(bytes) = punctuation_bytes(keystroke.key.as_str(), keystroke.modifiers.shift) {
        return Some(bytes);
    }

    let key = keystroke.key.as_str();
    if key.len() == 1 {
        let ch = key.chars().next().unwrap();
        if ch.is_ascii() {
            let ch = if keystroke.modifiers.shift && ch.is_ascii_alphabetic() {
                ch.to_ascii_uppercase()
            } else {
                ch
            };
            return Some(vec![ch as u8]);
        }
        return None;
    }

    // Some layouts report the typed UTF-8 directly in `key`.
    if !keystroke.modifiers.shift
        && key.chars().all(|c| !c.is_control())
        && !matches!(
            key,
            "enter" | "escape" | "backspace" | "tab" | "space" | "delete" | "insert"
        )
    {
        return Some(key.as_bytes().to_vec());
    }

    None
}

fn control_bytes(keystroke: &Keystroke) -> Option<Vec<u8>> {
    if !keystroke.modifiers.control {
        return None;
    }

    let key = keystroke.key.as_str();

    // On US layouts Ctrl+punctuation often matches the shifted symbol (Ctrl+- → Ctrl+_ → 0x1f).
    if let Some(bytes) = punctuation_bytes(key, true) {
        if bytes.len() == 1 {
            return Some(vec![bytes[0] & 0x1f]);
        }
    }

    if key.len() == 1 {
        let ch = key.chars().next().unwrap();
        if ch.is_ascii() {
            return Some(vec![(ch as u8) & 0x1f]);
        }
    }

    if let Some(bytes) = punctuation_bytes(key, false) {
        if bytes.len() == 1 {
            return Some(vec![bytes[0] & 0x1f]);
        }
    }

    None
}

fn alt_bytes(keystroke: &Keystroke) -> Option<Vec<u8>> {
    if !keystroke.modifiers.alt {
        return None;
    }

    let payload = printable_bytes(keystroke).or_else(|| {
        let key = keystroke.key.as_str();
        if key.len() == 1 {
            let ch = key.chars().next().unwrap();
            if ch.is_ascii() {
                return Some(vec![ch as u8]);
            }
        }
        punctuation_bytes(key, keystroke.modifiers.shift)
    })?;

    let mut bytes = vec![b'\x1b'];
    bytes.extend(payload);
    Some(bytes)
}

/// Convert a GPUI keystroke to terminal escape sequence bytes.
///
/// This function translates GPUI keyboard events into the appropriate byte sequences
/// expected by terminal applications. It handles special keys, control characters,
/// and application cursor mode.
///
/// # Arguments
///
/// * `keystroke` - The GPUI keystroke to convert
/// * `mode` - The current terminal mode (affects arrow key sequences)
///
/// # Returns
///
/// An optional vector of bytes representing the terminal escape sequence.
/// Returns `None` if the keystroke should not produce any output.
///
/// # Examples
///
/// ```
/// use gpui::Keystroke;
/// use alacritty_terminal::term::TermMode;
/// use rml_ui_term::input::keystroke_to_bytes;
///
/// let keystroke = Keystroke::parse("enter").unwrap();
/// let bytes = keystroke_to_bytes(&keystroke, TermMode::empty());
/// assert_eq!(bytes, Some(b"\r".to_vec()));
/// ```
/// Returns true when the keystroke is a common terminal paste chord.
pub fn is_paste_keystroke(keystroke: &Keystroke) -> bool {
    let key = keystroke.key.as_str();
    keystroke.modifiers.control && keystroke.modifiers.shift && key == "v"
        || keystroke.modifiers.shift && key == "insert" && !keystroke.modifiers.control
        || keystroke.modifiers.control && key == "insert" && !keystroke.modifiers.shift
}

/// Normalize clipboard text for PTY input (LF → CR, strip stray NULs).
pub fn normalize_paste_bytes(text: &str) -> Vec<u8> {
    text.replace("\r\n", "\n")
        .replace('\n', "\r")
        .bytes()
        .filter(|&b| b != 0)
        .collect()
}

pub fn keystroke_to_bytes(keystroke: &Keystroke, mode: TermMode) -> Option<Vec<u8>> {
    // Handle special keys first
    match keystroke.key.as_str() {
        // Basic control characters
        "space" => {
            if keystroke.modifiers.control {
                return Some(b"\x00".to_vec()); // Ctrl+Space = NUL
            }
            return Some(b" ".to_vec());
        }
        "enter" => return Some(b"\r".to_vec()),
        "escape" => return Some(b"\x1b".to_vec()),
        "backspace" => return Some(b"\x7f".to_vec()),
        "tab" => {
            // Shift+Tab sends a different sequence
            if keystroke.modifiers.shift {
                return Some(b"\x1b[Z".to_vec());
            }
            return Some(b"\t".to_vec());
        }

        // Arrow keys - check APP_CURSOR mode
        "up" => {
            if mode.contains(TermMode::APP_CURSOR) {
                return Some(b"\x1bOA".to_vec());
            }
            return Some(b"\x1b[A".to_vec());
        }
        "down" => {
            if mode.contains(TermMode::APP_CURSOR) {
                return Some(b"\x1bOB".to_vec());
            }
            return Some(b"\x1b[B".to_vec());
        }
        "right" => {
            if mode.contains(TermMode::APP_CURSOR) {
                return Some(b"\x1bOC".to_vec());
            }
            return Some(b"\x1b[C".to_vec());
        }
        "left" => {
            if mode.contains(TermMode::APP_CURSOR) {
                return Some(b"\x1bOD".to_vec());
            }
            return Some(b"\x1b[D".to_vec());
        }

        // Navigation keys
        "home" => return Some(b"\x1b[H".to_vec()),
        "end" => return Some(b"\x1b[F".to_vec()),
        "pageup" => return Some(b"\x1b[5~".to_vec()),
        "pagedown" => return Some(b"\x1b[6~".to_vec()),
        "insert" => return Some(b"\x1b[2~".to_vec()),
        "delete" => return Some(b"\x1b[3~".to_vec()),

        // Function keys
        "f1" => return Some(b"\x1bOP".to_vec()),
        "f2" => return Some(b"\x1bOQ".to_vec()),
        "f3" => return Some(b"\x1bOR".to_vec()),
        "f4" => return Some(b"\x1bOS".to_vec()),
        "f5" => return Some(b"\x1b[15~".to_vec()),
        "f6" => return Some(b"\x1b[17~".to_vec()),
        "f7" => return Some(b"\x1b[18~".to_vec()),
        "f8" => return Some(b"\x1b[19~".to_vec()),
        "f9" => return Some(b"\x1b[20~".to_vec()),
        "f10" => return Some(b"\x1b[21~".to_vec()),
        "f11" => return Some(b"\x1b[23~".to_vec()),
        "f12" => return Some(b"\x1b[24~".to_vec()),

        _ => {}
    }

    if let Some(bytes) = control_bytes(keystroke) {
        return Some(bytes);
    }

    if let Some(bytes) = alt_bytes(keystroke) {
        return Some(bytes);
    }

    printable_bytes(keystroke)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enter_key() {
        let keystroke = Keystroke::parse("enter").unwrap();
        let bytes = keystroke_to_bytes(&keystroke, TermMode::empty());
        assert_eq!(bytes, Some(b"\r".to_vec()));
    }

    #[test]
    fn test_escape_key() {
        let keystroke = Keystroke::parse("escape").unwrap();
        let bytes = keystroke_to_bytes(&keystroke, TermMode::empty());
        assert_eq!(bytes, Some(b"\x1b".to_vec()));
    }

    #[test]
    fn test_backspace_key() {
        let keystroke = Keystroke::parse("backspace").unwrap();
        let bytes = keystroke_to_bytes(&keystroke, TermMode::empty());
        assert_eq!(bytes, Some(b"\x7f".to_vec()));
    }

    #[test]
    fn test_tab_key() {
        let keystroke = Keystroke::parse("tab").unwrap();
        let bytes = keystroke_to_bytes(&keystroke, TermMode::empty());
        assert_eq!(bytes, Some(b"\t".to_vec()));
    }

    #[test]
    fn test_shift_tab() {
        let keystroke = Keystroke::parse("shift-tab").unwrap();
        let bytes = keystroke_to_bytes(&keystroke, TermMode::empty());
        assert_eq!(bytes, Some(b"\x1b[Z".to_vec()));
    }

    #[test]
    fn test_arrow_keys_normal_mode() {
        let mode = TermMode::empty();

        let up = Keystroke::parse("up").unwrap();
        assert_eq!(keystroke_to_bytes(&up, mode), Some(b"\x1b[A".to_vec()));

        let down = Keystroke::parse("down").unwrap();
        assert_eq!(keystroke_to_bytes(&down, mode), Some(b"\x1b[B".to_vec()));

        let right = Keystroke::parse("right").unwrap();
        assert_eq!(keystroke_to_bytes(&right, mode), Some(b"\x1b[C".to_vec()));

        let left = Keystroke::parse("left").unwrap();
        assert_eq!(keystroke_to_bytes(&left, mode), Some(b"\x1b[D".to_vec()));
    }

    #[test]
    fn test_arrow_keys_app_cursor_mode() {
        let mode = TermMode::APP_CURSOR;

        let up = Keystroke::parse("up").unwrap();
        assert_eq!(keystroke_to_bytes(&up, mode), Some(b"\x1bOA".to_vec()));

        let down = Keystroke::parse("down").unwrap();
        assert_eq!(keystroke_to_bytes(&down, mode), Some(b"\x1bOB".to_vec()));

        let right = Keystroke::parse("right").unwrap();
        assert_eq!(keystroke_to_bytes(&right, mode), Some(b"\x1bOC".to_vec()));

        let left = Keystroke::parse("left").unwrap();
        assert_eq!(keystroke_to_bytes(&left, mode), Some(b"\x1bOD".to_vec()));
    }

    #[test]
    fn test_navigation_keys() {
        let mode = TermMode::empty();

        let home = Keystroke::parse("home").unwrap();
        assert_eq!(keystroke_to_bytes(&home, mode), Some(b"\x1b[H".to_vec()));

        let end = Keystroke::parse("end").unwrap();
        assert_eq!(keystroke_to_bytes(&end, mode), Some(b"\x1b[F".to_vec()));

        let pageup = Keystroke::parse("pageup").unwrap();
        assert_eq!(keystroke_to_bytes(&pageup, mode), Some(b"\x1b[5~".to_vec()));

        let pagedown = Keystroke::parse("pagedown").unwrap();
        assert_eq!(
            keystroke_to_bytes(&pagedown, mode),
            Some(b"\x1b[6~".to_vec())
        );

        let insert = Keystroke::parse("insert").unwrap();
        assert_eq!(keystroke_to_bytes(&insert, mode), Some(b"\x1b[2~".to_vec()));

        let delete = Keystroke::parse("delete").unwrap();
        assert_eq!(keystroke_to_bytes(&delete, mode), Some(b"\x1b[3~".to_vec()));
    }

    #[test]
    fn test_function_keys() {
        let mode = TermMode::empty();

        let f1 = Keystroke::parse("f1").unwrap();
        assert_eq!(keystroke_to_bytes(&f1, mode), Some(b"\x1bOP".to_vec()));

        let f2 = Keystroke::parse("f2").unwrap();
        assert_eq!(keystroke_to_bytes(&f2, mode), Some(b"\x1bOQ".to_vec()));

        let f5 = Keystroke::parse("f5").unwrap();
        assert_eq!(keystroke_to_bytes(&f5, mode), Some(b"\x1b[15~".to_vec()));

        let f12 = Keystroke::parse("f12").unwrap();
        assert_eq!(keystroke_to_bytes(&f12, mode), Some(b"\x1b[24~".to_vec()));
    }

    #[test]
    fn test_ctrl_combinations() {
        let mode = TermMode::empty();

        // Ctrl+A = 0x01
        let ctrl_a = Keystroke::parse("ctrl-a").unwrap();
        assert_eq!(keystroke_to_bytes(&ctrl_a, mode), Some(vec![0x01]));

        // Ctrl+C = 0x03
        let ctrl_c = Keystroke::parse("ctrl-c").unwrap();
        assert_eq!(keystroke_to_bytes(&ctrl_c, mode), Some(vec![0x03]));

        // Ctrl+Z = 0x1a
        let ctrl_z = Keystroke::parse("ctrl-z").unwrap();
        assert_eq!(keystroke_to_bytes(&ctrl_z, mode), Some(vec![0x1a]));

        // Ctrl+Space = 0x00
        let ctrl_space = Keystroke::parse("ctrl-space").unwrap();
        assert_eq!(keystroke_to_bytes(&ctrl_space, mode), Some(vec![0x00]));
    }

    #[test]
    fn test_alt_combinations() {
        let mode = TermMode::empty();

        // Alt+a sends ESC followed by 'a'
        let alt_a = Keystroke::parse("alt-a").unwrap();
        assert_eq!(keystroke_to_bytes(&alt_a, mode), Some(b"\x1ba".to_vec()));

        // Alt+x sends ESC followed by 'x'
        let alt_x = Keystroke::parse("alt-x").unwrap();
        assert_eq!(keystroke_to_bytes(&alt_x, mode), Some(b"\x1bx".to_vec()));
    }

    #[test]
    fn test_regular_characters() {
        let mode = TermMode::empty();

        let a = Keystroke::parse("a").unwrap();
        assert_eq!(keystroke_to_bytes(&a, mode), Some(b"a".to_vec()));

        let z = Keystroke::parse("z").unwrap();
        assert_eq!(keystroke_to_bytes(&z, mode), Some(b"z".to_vec()));

        let zero = Keystroke::parse("0").unwrap();
        assert_eq!(keystroke_to_bytes(&zero, mode), Some(b"0".to_vec()));
    }

    #[test]
    fn test_space_key() {
        let mode = TermMode::empty();

        let space = Keystroke::parse("space").unwrap();
        assert_eq!(keystroke_to_bytes(&space, mode), Some(b" ".to_vec()));
    }

    #[test]
    fn test_named_punctuation_keys() {
        let mode = TermMode::empty();

        let minus = Keystroke {
            modifiers: Default::default(),
            key: "minus".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&minus, mode), Some(b"-".to_vec()));

        let slash = Keystroke {
            modifiers: Default::default(),
            key: "slash".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&slash, mode), Some(b"/".to_vec()));
    }

    #[test]
    fn test_punctuation_with_key_char() {
        let mode = TermMode::empty();

        let hyphen = Keystroke {
            modifiers: Default::default(),
            key: "minus".into(),
            key_char: Some("-".into()),
        };
        assert_eq!(keystroke_to_bytes(&hyphen, mode), Some(b"-".to_vec()));
    }

    #[test]
    fn test_shifted_punctuation() {
        let mode = TermMode::empty();

        let mut shift = gpui::Modifiers::default();
        shift.shift = true;

        let minus = Keystroke {
            modifiers: shift,
            key: "-".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&minus, mode), Some(b"_".to_vec()));
    }

    #[test]
    fn test_ctrl_digit_and_punctuation() {
        let mode = TermMode::empty();

        let mut ctrl = gpui::Modifiers::default();
        ctrl.control = true;

        let ctrl_c = Keystroke {
            modifiers: ctrl,
            key: "c".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&ctrl_c, mode), Some(vec![0x03]));

        let ctrl_2 = Keystroke {
            modifiers: ctrl,
            key: "2".into(),
            key_char: None,
        };
        // ASCII control mapping: (b'2' & 0x1f) == 0x12
        assert_eq!(keystroke_to_bytes(&ctrl_2, mode), Some(vec![0x12]));

        let ctrl_minus = Keystroke {
            modifiers: ctrl,
            key: "-".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&ctrl_minus, mode), Some(vec![0x1f]));
    }

    #[test]
    fn test_alt_punctuation() {
        let mode = TermMode::empty();

        let mut alt = gpui::Modifiers::default();
        alt.alt = true;

        let alt_minus = Keystroke {
            modifiers: alt,
            key: "minus".into(),
            key_char: None,
        };
        assert_eq!(keystroke_to_bytes(&alt_minus, mode), Some(b"\x1b-".to_vec()));
    }

    #[test]
    fn test_paste_keystroke_detection() {
        let paste = Keystroke::parse("ctrl-shift-v").unwrap();
        assert!(is_paste_keystroke(&paste));

        let shift_ins = Keystroke::parse("shift-insert").unwrap();
        assert!(is_paste_keystroke(&shift_ins));
    }

    #[test]
    fn test_normalize_paste_bytes() {
        assert_eq!(normalize_paste_bytes("a\nb"), b"a\rb");
        assert_eq!(normalize_paste_bytes("a\r\nb"), b"a\rb");
    }

    #[test]
    fn test_common_punctuation_chars() {
        let mode = TermMode::empty();
        for (name, expected) in [
            ("equal", b"="),
            ("comma", b","),
            ("period", b"."),
            ("semicolon", b";"),
            ("apostrophe", b"'"),
            ("bracketleft", b"["),
            ("backslash", b"\\"),
            ("grave", b"`"),
        ] {
            let ks = Keystroke {
                modifiers: Default::default(),
                key: name.into(),
                key_char: None,
            };
            assert_eq!(
                keystroke_to_bytes(&ks, mode),
                Some(expected.to_vec()),
                "key name {name}"
            );
        }
    }
}
