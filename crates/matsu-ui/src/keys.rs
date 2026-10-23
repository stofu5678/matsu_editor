//! GPUI keystrokes to the chords of `matsu-commands`' key map.

use gpui::Keystroke;
use matsu_commands::keymap::{Chord, Key, Modifiers};

/// The chord for a keystroke, or `None` for a key that the key map has no name for (a lone
/// modifier, media keys, a letter of another script that has no ASCII equivalent).
///
/// GPUI names a key by the character printed on it ([`Keystroke::key`], lowercase, the ASCII
/// equivalent on layouts without ASCII keys). Sakura's table is by Windows virtual-key code, whose
/// names follow a Japanese keyboard (`Caret` is the `^` key, `At` the `@` key). The punctuation
/// below is matched by the character, so the keys of a US keyboard map by what is printed on
/// them: `=` stands for the `^` key of the JIS layout (`VK_OEM_7`), and so on. That is right for
/// the keys that have the same character on both layouts and approximate for the rest; the real
/// mapping needs the scan code, which GPUI does not give.
pub fn chord_of(keystroke: &Keystroke) -> Option<Chord> {
    let key = key_named(&keystroke.key)?;
    let m = keystroke.modifiers;
    Some(Chord {
        key,
        mods: Modifiers {
            shift: m.shift,
            ctrl: m.control,
            alt: m.alt,
            meta: m.platform,
        },
    })
}

fn key_named(name: &str) -> Option<Key> {
    let mut chars = name.chars();
    let first = chars.next()?;
    if chars.next().is_none() {
        return single(first);
    }
    Some(match name {
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "insert" => Key::Insert,
        "delete" => Key::Delete,
        "backspace" => Key::Backspace,
        "tab" => Key::Tab,
        "enter" => Key::Enter,
        "escape" => Key::Escape,
        "space" => Key::Space,
        "menu" => Key::AppMenu,
        _ => {
            let n: u8 = name.strip_prefix('f')?.parse().ok()?;
            return Key::function(n);
        }
    })
}

fn single(c: char) -> Option<Key> {
    if let Some(d) = c.to_digit(10) {
        return Key::digit(u8::try_from(d).ok()?);
    }
    if let Some(k) = Key::letter(c) {
        return Some(k);
    }
    Some(match c {
        ' ' => Key::Space,
        '-' => Key::Minus,
        '^' | '=' => Key::Caret,
        '\\' => Key::Yen,
        '@' | '`' => Key::At,
        '[' => Key::LeftBracket,
        ';' => Key::Semicolon,
        ':' | '\'' => Key::Colon,
        ']' => Key::RightBracket,
        ',' => Key::Comma,
        '.' => Key::Period,
        '/' => Key::Slash,
        '_' => Key::Underscore,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(s: &str) -> Option<Chord> {
        chord_of(&Keystroke::parse(s).unwrap())
    }

    #[test]
    fn letters_digits_and_function_keys() {
        assert_eq!(
            chord("ctrl-z"),
            Some(Chord {
                key: Key::Letter('Z'),
                mods: Modifiers::CTRL
            })
        );
        assert_eq!(chord("5").unwrap().key, Key::Digit(5));
        assert_eq!(chord("f12").unwrap().key, Key::Function(12));
        assert_eq!(chord("f1").unwrap().key, Key::Function(1));
    }

    #[test]
    fn named_keys_and_modifiers() {
        assert_eq!(chord("backspace").unwrap().key, Key::Backspace);
        assert_eq!(chord("pageup").unwrap().key, Key::PageUp);
        let c = chord("ctrl-shift-home").unwrap();
        assert_eq!(c.key, Key::Home);
        assert_eq!(c.mods, Modifiers::CTRL.with_shift());
        let c = chord("alt-left").unwrap();
        assert_eq!(c.mods, Modifiers::ALT);
        assert_eq!(chord("cmd-s").unwrap().mods, Modifiers::META);
        assert_eq!(chord("space").unwrap().key, Key::Space);
    }

    #[test]
    fn punctuation_is_matched_by_the_character() {
        assert_eq!(chord("-").unwrap().key, Key::Minus);
        assert_eq!(chord(";").unwrap().key, Key::Semicolon);
        assert_eq!(chord("/").unwrap().key, Key::Slash);
    }

    #[test]
    fn unknown_keys_have_no_chord() {
        assert_eq!(chord("f25"), None);
        assert_eq!(chord("ๆ"), None);
        assert_eq!(chord("capslock"), None);
    }
}
