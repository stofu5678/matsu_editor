//! What a key press means to the terminal (ADR-0028 point 9): GPUI's keystroke turned into a key for
//! `matsu_terminal::encode_key`, or into text.

use gpui::Keystroke;
use matsu_terminal::{Key, Modifiers};

/// What the terminal does with a key press.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Translated {
    /// A key to encode and write to the program.
    Key(Key, Modifiers),
    /// Typed text to write as UTF-8 (no IME: the composition is a later PR).
    Text(String),
    /// A key the terminal takes and does nothing with (paste, until there is one).
    Ignore,
    /// A key the terminal leaves to others: the menus and the window of macOS (Cmd).
    Pass,
}

/// The text a key produced, if it is printable.
fn printable(keystroke: &Keystroke) -> Option<&str> {
    keystroke
        .key_char
        .as_deref()
        .filter(|text| !text.is_empty() && !text.chars().any(char::is_control))
}

fn named_key(name: &str) -> Option<Key> {
    Some(match name {
        "enter" => Key::Enter,
        "backspace" => Key::Backspace,
        "tab" => Key::Tab,
        "escape" => Key::Escape,
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
        _ => {
            let number: u8 = name.strip_prefix('f')?.parse().ok()?;
            (1..=12).contains(&number).then_some(Key::F(number))?
        }
    })
}

/// Turns a key press into what the terminal does with it. `mac`: the platform key is Cmd, which belongs to
/// the menus and the window; elsewhere it is the Windows or Super key, which the OS keeps.
/// `prefer_character_input` is GPUI's hint that the character is wanted, not the key (AltGr and the like).
pub(super) fn translate(
    keystroke: &Keystroke,
    prefer_character_input: bool,
    mac: bool,
) -> Translated {
    let m = keystroke.modifiers;
    if m.platform {
        return Translated::Pass;
    }
    // Paste is kept free (ADR-0028 point 9: Ctrl+V on Windows and Linux): nothing is sent yet. Cmd+V on macOS
    // is passed above.
    if keystroke.key == "v" && m.control && !mac {
        return Translated::Ignore;
    }
    let mods = Modifiers {
        shift: m.shift,
        ctrl: m.control,
        alt: m.alt,
    };
    if let Some(key) = named_key(&keystroke.key) {
        return Translated::Key(key, mods);
    }
    let text = printable(keystroke);
    // AltGr is Ctrl+Alt on Windows: it types a character, which is sent as it is.
    if (prefer_character_input || (m.control && m.alt))
        && let Some(text) = text
    {
        return Translated::Text(text.to_owned());
    }
    let mut chars = keystroke.key.chars();
    let base = match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ if keystroke.key == "space" => Some(' '),
        _ => None,
    };
    if m.control {
        // Ctrl+letter is the control code of the letter, whatever the layout writes with Shift.
        return match base {
            Some(c) => Translated::Key(Key::Char(c), mods),
            None => Translated::Ignore,
        };
    }
    match (text, base) {
        // Alt+character is Meta: ESC and the character.
        (Some(text), _) if m.alt => {
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Translated::Key(Key::Char(c), mods),
                _ => Translated::Text(text.to_owned()),
            }
        }
        (Some(text), _) => Translated::Text(text.to_owned()),
        // A dead key, a modifier alone, a key the terminal has no use for.
        (None, _) => Translated::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Modifiers as GpuiModifiers;

    fn press(key: &str, key_char: Option<&str>, modifiers: GpuiModifiers) -> Keystroke {
        Keystroke {
            modifiers,
            key: key.to_owned(),
            key_char: key_char.map(str::to_owned),
        }
    }

    fn plain() -> GpuiModifiers {
        GpuiModifiers::default()
    }

    fn ctrl() -> GpuiModifiers {
        GpuiModifiers {
            control: true,
            ..plain()
        }
    }

    fn alt() -> GpuiModifiers {
        GpuiModifiers {
            alt: true,
            ..plain()
        }
    }

    #[test]
    fn typed_characters_are_text() {
        let a = press("a", Some("a"), plain());
        assert_eq!(translate(&a, false, false), Translated::Text("a".into()));
        let shifted = press(
            "a",
            Some("A"),
            GpuiModifiers {
                shift: true,
                ..plain()
            },
        );
        assert_eq!(
            translate(&shifted, false, false),
            Translated::Text("A".into())
        );
        let space = press("space", Some(" "), plain());
        assert_eq!(
            translate(&space, false, false),
            Translated::Text(" ".into())
        );
        // a character with no keystroke of its own (the layout's) is written as UTF-8
        let kana = press("あ", Some("あ"), plain());
        assert_eq!(
            translate(&kana, false, false),
            Translated::Text("あ".into())
        );
    }

    #[test]
    fn control_letters_are_keys_with_the_modifier() {
        let c = press("c", Some("c"), ctrl());
        assert_eq!(
            translate(&c, false, false),
            Translated::Key(
                Key::Char('c'),
                Modifiers {
                    ctrl: true,
                    ..Modifiers::NONE
                }
            )
        );
        let at = press("space", Some(" "), ctrl());
        assert!(matches!(
            translate(&at, false, false),
            Translated::Key(Key::Char(' '), _)
        ));
    }

    #[test]
    fn alt_letters_go_as_meta() {
        let b = press("b", Some("b"), alt());
        assert_eq!(
            translate(&b, false, false),
            Translated::Key(
                Key::Char('b'),
                Modifiers {
                    alt: true,
                    ..Modifiers::NONE
                }
            )
        );
    }

    #[test]
    fn altgr_types_its_character() {
        let both = GpuiModifiers {
            control: true,
            alt: true,
            ..plain()
        };
        let at = press("q", Some("@"), both);
        assert_eq!(translate(&at, false, false), Translated::Text("@".into()));
        let hinted = press("2", Some("\""), alt());
        assert_eq!(
            translate(&hinted, true, false),
            Translated::Text("\"".into())
        );
    }

    #[test]
    fn named_keys_carry_their_modifiers() {
        let up = press("up", None, plain());
        assert_eq!(
            translate(&up, false, false),
            Translated::Key(Key::Up, Modifiers::NONE)
        );
        let f5 = press("f5", None, ctrl());
        assert!(matches!(
            translate(&f5, false, false),
            Translated::Key(Key::F(5), Modifiers { ctrl: true, .. })
        ));
        // F13 and the like have no encoding
        assert_eq!(
            translate(&press("f13", None, plain()), false, false),
            Translated::Ignore
        );
        let enter = press("enter", Some("\n"), plain());
        assert_eq!(
            translate(&enter, false, false),
            Translated::Key(Key::Enter, Modifiers::NONE)
        );
    }

    #[test]
    fn paste_is_kept_free_and_cmd_is_left_to_the_menus() {
        let paste = press("v", Some("v"), ctrl());
        assert_eq!(translate(&paste, false, false), Translated::Ignore);
        let cmd = GpuiModifiers {
            platform: true,
            ..plain()
        };
        assert_eq!(
            translate(&press("v", Some("v"), cmd), false, true),
            Translated::Pass
        );
        assert_eq!(
            translate(&press("w", Some("w"), cmd), false, true),
            Translated::Pass
        );
        // Ctrl+V on macOS is the program's (a literal next character in shells)
        assert!(matches!(
            translate(&paste, false, true),
            Translated::Key(Key::Char('v'), _)
        ));
    }

    #[test]
    fn a_modifier_or_a_dead_key_alone_does_nothing() {
        assert_eq!(
            translate(&press("shift", None, plain()), false, false),
            Translated::Ignore
        );
        assert_eq!(
            translate(&press("a", None, plain()), false, false),
            Translated::Ignore
        );
    }
}
