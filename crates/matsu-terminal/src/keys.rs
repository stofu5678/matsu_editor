//! Key encoding: the bytes a terminal sends to the program for a key press (xterm form, ADR-0028).
//!
//! The types are UI independent; the UI maps its own key events to [`Key`] and [`Modifiers`]. A printable
//! character is given as the character the key produces (with Shift already applied, so `A` and not `a`).
//! The Kitty keyboard protocol is not supported yet.
//!
//! The rules follow xterm's defaults (`ctlseqs`, "PC-Style Function Keys"): a modified cursor or function
//! key is `CSI 1 ; <m> <final>` or `CSI <n> ; <m> ~`, where `m = 1 + shift + 2 * alt + 4 * ctrl`; Alt on a
//! character or on Enter, Backspace, Tab and Escape is an ESC prefix.

use crate::screen::Modes;

/// A key, without its modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// A printable character, as the key produces it.
    Char(char),
    Enter,
    Backspace,
    Tab,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    /// A function key, `F(1)` to `F(12)`.
    F(u8),
}

/// The modifier keys held with a key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        shift: false,
        ctrl: false,
        alt: false,
    };

    /// xterm's modifier parameter, `1 + shift + 2 * alt + 4 * ctrl` (1 means no modifier).
    fn parameter(self) -> u8 {
        1 + u8::from(self.shift) + 2 * u8::from(self.alt) + 4 * u8::from(self.ctrl)
    }

    fn any(self) -> bool {
        self.shift || self.ctrl || self.alt
    }
}

/// The bytes to write to the terminal for a key, or `None` when the key has no encoding (a function key
/// outside F1 to F12, a control character given as a `Char`).
///
/// `modes` is the terminal's current [`Modes`]: in application cursor mode the unmodified arrows, Home and
/// End are `SS3` sequences.
pub fn encode_key(key: Key, mods: Modifiers, modes: &Modes) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    match key {
        Key::Char(c) => {
            if c.is_control() {
                return None;
            }
            if mods.alt {
                out.push(0x1b);
            }
            match mods.ctrl.then(|| control_code(c)).flatten() {
                Some(code) => out.push(code),
                None => {
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
            }
        }
        Key::Enter => {
            if mods.alt {
                out.push(0x1b);
            }
            // Ctrl+Enter is a line feed (as in Windows Terminal), so programs can tell it from Enter.
            out.push(if mods.ctrl { b'\n' } else { b'\r' });
        }
        Key::Backspace => {
            if mods.alt {
                out.push(0x1b);
            }
            out.push(if mods.ctrl { 0x08 } else { 0x7f });
        }
        Key::Tab => {
            if mods.alt {
                out.push(0x1b);
            }
            if mods.shift {
                out.extend_from_slice(b"\x1b[Z");
            } else {
                out.push(b'\t');
            }
        }
        Key::Escape => {
            if mods.alt {
                out.push(0x1b);
            }
            out.push(0x1b);
        }
        Key::Up => cursor_key(&mut out, b'A', mods, modes),
        Key::Down => cursor_key(&mut out, b'B', mods, modes),
        Key::Right => cursor_key(&mut out, b'C', mods, modes),
        Key::Left => cursor_key(&mut out, b'D', mods, modes),
        Key::Home => cursor_key(&mut out, b'H', mods, modes),
        Key::End => cursor_key(&mut out, b'F', mods, modes),
        Key::Insert => tilde_key(&mut out, 2, mods),
        Key::Delete => tilde_key(&mut out, 3, mods),
        Key::PageUp => tilde_key(&mut out, 5, mods),
        Key::PageDown => tilde_key(&mut out, 6, mods),
        Key::F(n @ 1..=4) => {
            let final_byte = b'P' + (n - 1);
            if mods.any() {
                out.extend_from_slice(format!("\x1b[1;{}", mods.parameter()).as_bytes());
                out.push(final_byte);
            } else {
                out.extend_from_slice(&[0x1b, b'O', final_byte]);
            }
        }
        Key::F(n @ 5..=12) => {
            // xterm's numbers: F5 15, F6 17, F7 18, F8 19, F9 20, F10 21, F11 23, F12 24.
            const NUMBERS: [u8; 8] = [15, 17, 18, 19, 20, 21, 23, 24];
            tilde_key(&mut out, NUMBERS[usize::from(n - 5)], mods);
        }
        Key::F(_) => return None,
    }
    Some(out)
}

/// The control code of Ctrl with a character, as xterm sends it (`Ctrl+A` is 0x01, `Ctrl+[` is ESC).
fn control_code(c: char) -> Option<u8> {
    match c {
        'a'..='z' => Some(c as u8 - b'a' + 1),
        'A'..='Z' => Some(c as u8 - b'A' + 1),
        '@' | ' ' => Some(0x00),
        '[' => Some(0x1b),
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '^' => Some(0x1e),
        '_' => Some(0x1f),
        '?' => Some(0x7f),
        _ => None,
    }
}

/// Arrows, Home and End: `CSI <final>`, `SS3 <final>` in application cursor mode, `CSI 1 ; <m> <final>` with
/// modifiers.
fn cursor_key(out: &mut Vec<u8>, final_byte: u8, mods: Modifiers, modes: &Modes) {
    if mods.any() {
        out.extend_from_slice(format!("\x1b[1;{}", mods.parameter()).as_bytes());
    } else if modes.app_cursor {
        out.extend_from_slice(b"\x1bO");
    } else {
        out.extend_from_slice(b"\x1b[");
    }
    out.push(final_byte);
}

/// Insert, Delete, Page Up and Down, F5 to F12: `CSI <n> ~`, `CSI <n> ; <m> ~` with modifiers.
fn tilde_key(out: &mut Vec<u8>, number: u8, mods: Modifiers) {
    if mods.any() {
        out.extend_from_slice(format!("\x1b[{number};{}~", mods.parameter()).as_bytes());
    } else {
        out.extend_from_slice(format!("\x1b[{number}~").as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CTRL: Modifiers = Modifiers {
        ctrl: true,
        ..Modifiers::NONE
    };
    const ALT: Modifiers = Modifiers {
        alt: true,
        ..Modifiers::NONE
    };
    const SHIFT: Modifiers = Modifiers {
        shift: true,
        ..Modifiers::NONE
    };
    const CTRL_ALT: Modifiers = Modifiers {
        ctrl: true,
        alt: true,
        shift: false,
    };

    fn enc(key: Key, mods: Modifiers) -> Vec<u8> {
        encode_key(key, mods, &Modes::default()).expect("encoded")
    }

    fn enc_app(key: Key, mods: Modifiers) -> Vec<u8> {
        let modes = Modes {
            app_cursor: true,
            ..Modes::default()
        };
        encode_key(key, mods, &modes).expect("encoded")
    }

    #[test]
    fn printable_characters_are_utf8() {
        assert_eq!(enc(Key::Char('a'), Modifiers::NONE), b"a");
        assert_eq!(enc(Key::Char('A'), SHIFT), b"A");
        assert_eq!(enc(Key::Char('あ'), Modifiers::NONE), "あ".as_bytes());
        assert_eq!(enc(Key::Char('😀'), Modifiers::NONE), "😀".as_bytes());
    }

    #[test]
    fn control_characters_given_as_char_have_no_encoding() {
        assert_eq!(
            encode_key(Key::Char('\n'), Modifiers::NONE, &Modes::default()),
            None
        );
    }

    #[test]
    fn ctrl_with_letters_is_a_control_code() {
        assert_eq!(enc(Key::Char('a'), CTRL), [0x01]);
        assert_eq!(enc(Key::Char('c'), CTRL), [0x03]);
        assert_eq!(enc(Key::Char('Z'), CTRL), [0x1a]);
        assert_eq!(enc(Key::Char('d'), CTRL), [0x04]);
    }

    #[test]
    fn ctrl_with_symbols() {
        assert_eq!(enc(Key::Char(' '), CTRL), [0x00]);
        assert_eq!(enc(Key::Char('@'), CTRL), [0x00]);
        assert_eq!(enc(Key::Char('['), CTRL), [0x1b]);
        assert_eq!(enc(Key::Char('\\'), CTRL), [0x1c]);
        assert_eq!(enc(Key::Char(']'), CTRL), [0x1d]);
        assert_eq!(enc(Key::Char('_'), CTRL), [0x1f]);
        assert_eq!(enc(Key::Char('?'), CTRL), [0x7f]);
    }

    #[test]
    fn ctrl_with_a_character_without_a_code_sends_the_character() {
        assert_eq!(enc(Key::Char('1'), CTRL), b"1");
        assert_eq!(enc(Key::Char('あ'), CTRL), "あ".as_bytes());
    }

    #[test]
    fn alt_is_an_escape_prefix() {
        assert_eq!(enc(Key::Char('b'), ALT), b"\x1bb");
        assert_eq!(enc(Key::Char('あ'), ALT), "\x1bあ".as_bytes());
        assert_eq!(enc(Key::Char('b'), CTRL_ALT), [0x1b, 0x02]);
        assert_eq!(enc(Key::Enter, ALT), b"\x1b\r");
        assert_eq!(enc(Key::Backspace, ALT), b"\x1b\x7f");
        assert_eq!(enc(Key::Escape, ALT), b"\x1b\x1b");
        assert_eq!(enc(Key::Tab, ALT), b"\x1b\t");
    }

    #[test]
    fn enter_backspace_tab_escape() {
        assert_eq!(enc(Key::Enter, Modifiers::NONE), b"\r");
        assert_eq!(enc(Key::Enter, SHIFT), b"\r");
        assert_eq!(enc(Key::Enter, CTRL), b"\n");
        assert_eq!(enc(Key::Backspace, Modifiers::NONE), [0x7f]);
        assert_eq!(enc(Key::Backspace, SHIFT), [0x7f]);
        assert_eq!(enc(Key::Backspace, CTRL), [0x08]);
        assert_eq!(enc(Key::Tab, Modifiers::NONE), b"\t");
        assert_eq!(enc(Key::Tab, SHIFT), b"\x1b[Z");
        assert_eq!(enc(Key::Escape, Modifiers::NONE), [0x1b]);
    }

    #[test]
    fn arrows_normal_and_application_mode() {
        assert_eq!(enc(Key::Up, Modifiers::NONE), b"\x1b[A");
        assert_eq!(enc(Key::Down, Modifiers::NONE), b"\x1b[B");
        assert_eq!(enc(Key::Right, Modifiers::NONE), b"\x1b[C");
        assert_eq!(enc(Key::Left, Modifiers::NONE), b"\x1b[D");
        assert_eq!(enc_app(Key::Up, Modifiers::NONE), b"\x1bOA");
        assert_eq!(enc_app(Key::Left, Modifiers::NONE), b"\x1bOD");
    }

    #[test]
    fn modified_arrows_ignore_application_mode() {
        assert_eq!(enc(Key::Up, SHIFT), b"\x1b[1;2A");
        assert_eq!(enc(Key::Up, ALT), b"\x1b[1;3A");
        assert_eq!(enc(Key::Right, CTRL), b"\x1b[1;5C");
        assert_eq!(enc(Key::Left, CTRL_ALT), b"\x1b[1;7D");
        assert_eq!(enc_app(Key::Right, CTRL), b"\x1b[1;5C");
        let all = Modifiers {
            shift: true,
            ctrl: true,
            alt: true,
        };
        assert_eq!(enc(Key::Down, all), b"\x1b[1;8B");
    }

    #[test]
    fn home_and_end() {
        assert_eq!(enc(Key::Home, Modifiers::NONE), b"\x1b[H");
        assert_eq!(enc(Key::End, Modifiers::NONE), b"\x1b[F");
        assert_eq!(enc_app(Key::Home, Modifiers::NONE), b"\x1bOH");
        assert_eq!(enc_app(Key::End, Modifiers::NONE), b"\x1bOF");
        assert_eq!(enc(Key::Home, SHIFT), b"\x1b[1;2H");
        assert_eq!(enc(Key::End, CTRL), b"\x1b[1;5F");
    }

    #[test]
    fn tilde_keys() {
        assert_eq!(enc(Key::Insert, Modifiers::NONE), b"\x1b[2~");
        assert_eq!(enc(Key::Delete, Modifiers::NONE), b"\x1b[3~");
        assert_eq!(enc(Key::PageUp, Modifiers::NONE), b"\x1b[5~");
        assert_eq!(enc(Key::PageDown, Modifiers::NONE), b"\x1b[6~");
        assert_eq!(enc(Key::Delete, CTRL), b"\x1b[3;5~");
        assert_eq!(enc(Key::PageUp, SHIFT), b"\x1b[5;2~");
        assert_eq!(enc_app(Key::Delete, Modifiers::NONE), b"\x1b[3~");
    }

    #[test]
    fn function_keys() {
        let expected: [&[u8]; 12] = [
            b"\x1bOP",
            b"\x1bOQ",
            b"\x1bOR",
            b"\x1bOS",
            b"\x1b[15~",
            b"\x1b[17~",
            b"\x1b[18~",
            b"\x1b[19~",
            b"\x1b[20~",
            b"\x1b[21~",
            b"\x1b[23~",
            b"\x1b[24~",
        ];
        for (i, want) in expected.iter().enumerate() {
            let n = u8::try_from(i + 1).expect("small");
            assert_eq!(enc(Key::F(n), Modifiers::NONE), *want, "F{n}");
        }
    }

    #[test]
    fn modified_function_keys() {
        assert_eq!(enc(Key::F(1), SHIFT), b"\x1b[1;2P");
        assert_eq!(enc(Key::F(4), CTRL), b"\x1b[1;5S");
        assert_eq!(enc(Key::F(5), SHIFT), b"\x1b[15;2~");
        assert_eq!(enc(Key::F(12), CTRL_ALT), b"\x1b[24;7~");
    }

    #[test]
    fn function_keys_out_of_range_have_no_encoding() {
        let modes = Modes::default();
        assert_eq!(encode_key(Key::F(0), Modifiers::NONE, &modes), None);
        assert_eq!(encode_key(Key::F(13), Modifiers::NONE, &modes), None);
    }
}
