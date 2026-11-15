//! What a key press means to the terminal (ADR-0028 point 9): GPUI's keystroke turned into a key for
//! `matsu_terminal::encode_key`, or into text.

use gpui::Keystroke;
use matsu_terminal::{Key, Modifiers};

/// What the terminal does with a key press.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Translated {
    /// A key to encode and write to the program.
    Key(Key, Modifiers),
    /// Typed text. The key does not write it: the platform delivers the text to the input handler
    /// (`view::input`), which writes it as UTF-8. That path also carries the IME and the dead keys (a
    /// dead key's accent must wait for the next key), so a key with text is left unhandled.
    Text(String),
    /// Paste the clipboard (Ctrl+V and Ctrl+Shift+V; Cmd+V on macOS).
    Paste,
    /// Copy the selection and leave the key unsent (Ctrl+Shift+C; Cmd+C on macOS).
    Copy,
    /// Ctrl+C: copy when there is a selection, else the key goes to the program (the interrupt).
    CopyOrKey(Key, Modifiers),
    /// Scroll the view through the scrollback: a key of the terminal view, never the program's.
    Scroll(ScrollKey),
    /// A key the terminal takes and does nothing with.
    Ignore,
    /// A key the terminal leaves to others: the menus and the window of macOS (Cmd).
    Pass,
}

/// The keys that scroll the view back and forward (Windows Terminal's: Ctrl+Shift with the arrow and page
/// keys, and Home and End for the ends). They work on every platform, also while the program has the mouse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScrollKey {
    /// One line towards the older lines (Ctrl+Shift+Up).
    LineUp,
    LineDown,
    /// A page towards the older lines (Ctrl+Shift+PageUp).
    PageUp,
    PageDown,
    /// The oldest line (Ctrl+Shift+Home).
    Top,
    /// The live screen (Ctrl+Shift+End).
    Bottom,
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

/// The capital of `c` when it has one letter for it; `c` itself otherwise (a digit, a symbol, `ß`).
fn upper(c: char) -> char {
    let mut capitals = c.to_uppercase();
    match (capitals.next(), capitals.next()) {
        (Some(capital), None) => capital,
        _ => c,
    }
}

/// Turns a key press into what the terminal does with it. `mac`: the platform key is Cmd, which belongs to
/// the menus and the window; elsewhere it is the Windows or Super key, which the OS keeps.
/// `prefer_character_input` is GPUI's hint that the character is wanted, not the key (AltGr and the like).
/// [`translate_with`] without the setting "Option as Meta".
#[cfg(test)]
fn translate(keystroke: &Keystroke, prefer_character_input: bool, mac: bool) -> Translated {
    translate_with(keystroke, prefer_character_input, mac, false)
}

/// [`translate`] with the setting "Option as Meta" (`[Terminal]` `OptionAsMeta`): on macOS, Option+key sends ESC
/// and the key as it is on the keyboard, not the character that Option makes of it (Terminal.app's "Use Option
/// as Meta key" and iTerm2's "Esc+"). It does nothing elsewhere: `option_as_meta` counts only with `mac`.
pub(super) fn translate_with(
    keystroke: &Keystroke,
    prefer_character_input: bool,
    mac: bool,
    option_as_meta: bool,
) -> Translated {
    let m = keystroke.modifiers;
    if m.platform {
        // Cmd+C and Cmd+V on macOS (ADR-0028 point 9); the other Cmd keys belong to the menus. Elsewhere the
        // platform key is the Windows or Super key.
        if mac && !m.control && !m.alt && !m.shift {
            match keystroke.key.as_str() {
                "c" => return Translated::Copy,
                "v" => return Translated::Paste,
                _ => {}
            }
        }
        return Translated::Pass;
    }
    let mods = Modifiers {
        shift: m.shift,
        ctrl: m.control,
        alt: m.alt,
    };
    if m.control && m.shift && !m.alt {
        let scroll = match keystroke.key.as_str() {
            "up" => Some(ScrollKey::LineUp),
            "down" => Some(ScrollKey::LineDown),
            "pageup" => Some(ScrollKey::PageUp),
            "pagedown" => Some(ScrollKey::PageDown),
            "home" => Some(ScrollKey::Top),
            "end" => Some(ScrollKey::Bottom),
            _ => None,
        };
        if let Some(key) = scroll {
            return Translated::Scroll(key);
        }
    }
    // Windows and Linux, as in Windows Terminal (ADR-0028 point 9): Ctrl+V pastes, Ctrl+C copies when
    // something is selected and else is the program's, and Ctrl+Shift+C and Ctrl+Shift+V are the copy and
    // the paste that never are the program's. AltGr (Ctrl+Alt) types a character: AltGr+V stays one. On
    // macOS Ctrl+C and Ctrl+V are the program's.
    if !mac && m.control && !m.alt {
        match keystroke.key.as_str() {
            "v" => return Translated::Paste,
            "c" if m.shift => return Translated::Copy,
            "c" => return Translated::CopyOrKey(Key::Char('c'), mods),
            _ => {}
        }
    }
    if let Some(key) = named_key(&keystroke.key) {
        return Translated::Key(key, mods);
    }
    let text = printable(keystroke);
    let mut chars = keystroke.key.chars();
    let base = match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ if keystroke.key == "space" => Some(' '),
        _ => None,
    };
    // Option as Meta: ESC and the key under the finger (`keystroke.key` is the key without Option, uppercased
    // for Shift). A dead key (Option+E makes an accent that waits for a letter) has no text, but is a key too.
    if mac
        && option_as_meta
        && m.alt
        && !m.control
        && let Some(c) = base
    {
        let c = if m.shift { upper(c) } else { c };
        return Translated::Key(Key::Char(c), mods);
    }
    // AltGr is Ctrl+Alt on Windows: it types a character, which is sent as it is.
    if (prefer_character_input || (m.control && m.alt))
        && let Some(text) = text
    {
        return Translated::Text(text.to_owned());
    }
    if m.control {
        // Ctrl+letter is the control code of the letter, whatever the layout writes with Shift.
        return match base {
            Some(c) => Translated::Key(Key::Char(c), mods),
            None => Translated::Ignore,
        };
    }
    match (text, base) {
        // On macOS Option types the character it gives (Option+B is a letter of its own): Terminal.app's and
        // iTerm2's default. The setting `OptionAsMeta` makes it Meta (above).
        (Some(text), _) if m.alt && mac => Translated::Text(text.to_owned()),
        // Alt+character is Meta elsewhere, as in Windows Terminal and xterm: ESC and the character.
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
        let c = press("a", Some("a"), ctrl());
        assert_eq!(
            translate(&c, false, false),
            Translated::Key(
                Key::Char('a'),
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
    fn option_types_its_character_on_macos_and_alt_is_meta_elsewhere() {
        // macOS gives a character for Option+B; it is typed as it is, without ESC
        let option_b = press("b", Some("\u{222b}"), alt());
        assert_eq!(
            translate(&option_b, false, true),
            Translated::Text("\u{222b}".into())
        );
        // Windows and Linux: Alt+B is ESC b
        let alt_b = press("b", Some("b"), alt());
        assert_eq!(
            translate(&alt_b, false, false),
            Translated::Key(
                Key::Char('b'),
                Modifiers {
                    alt: true,
                    ..Modifiers::NONE
                }
            )
        );
        assert_eq!(
            matsu_terminal::encode_key(
                Key::Char('b'),
                Modifiers {
                    alt: true,
                    ..Modifiers::NONE
                },
                &matsu_terminal::Modes::default()
            ),
            Some(b"\x1bb".to_vec())
        );
    }

    #[test]
    fn option_as_meta_sends_escape_and_the_key_on_macos() {
        let meta = |key: &str, key_char: Option<&str>, modifiers| {
            translate_with(&press(key, key_char, modifiers), false, true, true)
        };
        let escape_and = |c: char, shift: bool| {
            Translated::Key(
                Key::Char(c),
                Modifiers {
                    alt: true,
                    shift,
                    ..Modifiers::NONE
                },
            )
        };
        // Option+B types an integral sign without the setting, ESC b with it.
        assert_eq!(meta("b", Some("\u{222b}"), alt()), escape_and('b', false));
        // A dead key (Option+E waits for a letter, so it has no text) is Meta too.
        assert_eq!(meta("e", None, alt()), escape_and('e', false));
        // Shift gives the capital; a digit and a symbol stay as the key is.
        let shifted = GpuiModifiers {
            alt: true,
            shift: true,
            ..plain()
        };
        assert_eq!(meta("b", Some("\u{ec}"), shifted), escape_and('B', true));
        assert_eq!(meta("1", Some("\u{2044}"), shifted), escape_and('1', true));
        // The space bar.
        assert_eq!(meta("space", Some("\u{a0}"), alt()), escape_and(' ', false));
        // The bytes: ESC and the key.
        let bytes = |t: Translated| match t {
            Translated::Key(key, mods) => {
                matsu_terminal::encode_key(key, mods, &matsu_terminal::Modes::default())
            }
            other => panic!("{other:?}"),
        };
        assert_eq!(
            bytes(meta("b", Some("\u{222b}"), alt())),
            Some(b"\x1bb".to_vec())
        );
        assert_eq!(
            bytes(meta("b", Some("\u{ec}"), shifted)),
            Some(b"\x1bB".to_vec())
        );
        // A key without Option is not changed, and neither is Cmd.
        assert_eq!(meta("b", Some("b"), plain()), Translated::Text("b".into()));
        // Named keys already carry Alt.
        assert!(matches!(
            meta("left", None, alt()),
            Translated::Key(Key::Left, Modifiers { alt: true, .. })
        ));
    }

    #[test]
    fn option_as_meta_does_nothing_without_the_setting_or_off_macos() {
        let option_b = press("b", Some("\u{222b}"), alt());
        // The setting is off: Option types its character (the default of the setting).
        assert_eq!(
            translate_with(&option_b, false, true, false),
            Translated::Text("\u{222b}".into())
        );
        // Not macOS: Alt is Meta already, and the setting does not change how.
        let alt_b = press("b", Some("b"), alt());
        assert_eq!(
            translate_with(&alt_b, false, false, true),
            translate_with(&alt_b, false, false, false)
        );
        // Not macOS and a character the layout makes with Alt is still its text when asked for it.
        assert_eq!(
            translate_with(&option_b, true, false, true),
            Translated::Text("\u{222b}".into())
        );
    }

    #[test]
    fn upper_is_one_capital_or_the_character() {
        assert_eq!(upper('b'), 'B');
        assert_eq!(upper('1'), '1');
        assert_eq!(upper('\u{df}'), '\u{df}');
        assert_eq!(upper('\u{e9}'), '\u{c9}');
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
        // AltGr+V is a character of the layout, not paste
        let both_v = GpuiModifiers {
            control: true,
            alt: true,
            ..plain()
        };
        let v = press("v", Some("@"), both_v);
        assert_eq!(translate(&v, false, false), Translated::Text("@".into()));
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
    fn copy_and_paste_are_ctrl_c_and_v_on_windows_and_linux_and_cmd_on_macos() {
        let ctrl_key = |c: &str| press(c, Some(c), ctrl());
        let ctrl_shift = GpuiModifiers {
            control: true,
            shift: true,
            ..plain()
        };
        let cmd = GpuiModifiers {
            platform: true,
            ..plain()
        };
        // Windows and Linux
        assert_eq!(translate(&ctrl_key("v"), false, false), Translated::Paste);
        assert_eq!(
            translate(&press("v", Some("V"), ctrl_shift), false, false),
            Translated::Paste
        );
        assert_eq!(
            translate(&ctrl_key("c"), false, false),
            Translated::CopyOrKey(
                Key::Char('c'),
                Modifiers {
                    ctrl: true,
                    ..Modifiers::NONE
                }
            )
        );
        assert_eq!(
            translate(&press("c", Some("C"), ctrl_shift), false, false),
            Translated::Copy
        );
        // Cmd+V and Cmd+C are the keys of the menus elsewhere: the Windows or Super key
        assert_eq!(
            translate(&press("v", Some("v"), cmd), false, false),
            Translated::Pass
        );
        // macOS
        assert_eq!(
            translate(&press("v", Some("v"), cmd), false, true),
            Translated::Paste
        );
        assert_eq!(
            translate(&press("c", Some("c"), cmd), false, true),
            Translated::Copy
        );
        assert_eq!(
            translate(&press("w", Some("w"), cmd), false, true),
            Translated::Pass
        );
        // Ctrl+C and Ctrl+V on macOS are the program's (the interrupt; a literal next character in shells)
        for letter in ["c", "v"] {
            assert!(matches!(
                translate(&ctrl_key(letter), false, true),
                Translated::Key(Key::Char(_), _)
            ));
        }
        // AltGr+V is a character of the layout, not paste
        let altgr = GpuiModifiers {
            control: true,
            alt: true,
            ..plain()
        };
        assert_eq!(
            translate(&press("v", Some("@"), altgr), false, false),
            Translated::Text("@".into())
        );
    }

    #[test]
    fn ctrl_shift_with_the_arrow_page_home_and_end_keys_scroll_the_view() {
        let both = GpuiModifiers {
            control: true,
            shift: true,
            ..plain()
        };
        for (key, want) in [
            ("up", ScrollKey::LineUp),
            ("down", ScrollKey::LineDown),
            ("pageup", ScrollKey::PageUp),
            ("pagedown", ScrollKey::PageDown),
            ("home", ScrollKey::Top),
            ("end", ScrollKey::Bottom),
        ] {
            // on every platform
            for mac in [false, true] {
                assert_eq!(
                    translate(&press(key, None, both), false, mac),
                    Translated::Scroll(want),
                    "{key} mac={mac}"
                );
            }
        }
        // the same keys with another modifier are the program's
        assert!(matches!(
            translate(&press("pageup", None, ctrl()), false, false),
            Translated::Key(Key::PageUp, _)
        ));
        let shift = GpuiModifiers {
            shift: true,
            ..plain()
        };
        assert!(matches!(
            translate(&press("pageup", None, shift), false, false),
            Translated::Key(Key::PageUp, _)
        ));
        let with_alt = GpuiModifiers { alt: true, ..both };
        assert!(matches!(
            translate(&press("up", None, with_alt), false, false),
            Translated::Key(Key::Up, _)
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
