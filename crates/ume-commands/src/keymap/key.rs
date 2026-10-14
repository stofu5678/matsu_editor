//! Keys, modifiers and chords, with Sakura's key names.
//!
//! The key set is the one of Sakura's key table (`KeyDataInit` in `sakura_core/func/CKeyBind.cpp`),
//! so that Sakura's tables, `sakura.ini` and `.key` files map one to one. Keys are named by what
//! Sakura calls them on a JIS keyboard; the UI crate converts its own key events to these.

use std::fmt;

use crate::sakura::KEY_BINDINGS;

/// Mouse actions that Sakura binds like keys (`VKEX_*` in `sakura_core/func/CKeyBind.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseAction {
    DoubleClick,
    RightClick,
    MiddleClick,
    LeftSideClick,
    RightSideClick,
    TripleClick,
    QuadrupleClick,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
}

impl MouseAction {
    /// In the order of `VKEX_*` (0x100, 0x101, ...).
    const ALL: [Self; 11] = [
        Self::DoubleClick,
        Self::RightClick,
        Self::MiddleClick,
        Self::LeftSideClick,
        Self::RightSideClick,
        Self::TripleClick,
        Self::QuadrupleClick,
        Self::WheelUp,
        Self::WheelDown,
        Self::WheelLeft,
        Self::WheelRight,
    ];
}

/// A key (or mouse action) of Sakura's key table.
///
/// `Letter` holds an uppercase ASCII letter, `Digit` a digit value 0-9 and `Function` a number
/// 1-24; use [`Key::letter`], [`Key::digit`] and [`Key::function`], which check this.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Letter(char),
    Digit(u8),
    Function(u8),
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
    Backspace,
    Tab,
    Enter,
    Escape,
    Space,
    /// `-`
    Minus,
    /// `^` (`'` on an English keyboard)
    Caret,
    /// `\` (the yen key)
    Yen,
    /// `@` (`` ` `` on an English keyboard)
    At,
    /// `[`
    LeftBracket,
    /// `;`
    Semicolon,
    /// `:`
    Colon,
    /// `]`
    RightBracket,
    Comma,
    Period,
    Slash,
    /// `_` (the backslash key of a JIS keyboard, `VK_OEM_102`)
    Underscore,
    /// `_` of a PC-98 keyboard
    UnderscorePc98,
    /// The menu key (`VK_APPS`).
    AppMenu,
    Mouse(MouseAction),
}

/// `(key, Sakura's key code)` for the keys that are not a family (letters, digits, F keys).
const NAMED_KEYS: &[(Key, u16)] = &[
    (Key::Up, 0x26),
    (Key::Down, 0x28),
    (Key::Left, 0x25),
    (Key::Right, 0x27),
    (Key::Home, 0x24),
    (Key::End, 0x23),
    (Key::PageUp, 0x21),
    (Key::PageDown, 0x22),
    (Key::Insert, 0x2d),
    (Key::Delete, 0x2e),
    (Key::Backspace, 0x08),
    (Key::Tab, 0x09),
    (Key::Enter, 0x0d),
    (Key::Escape, 0x1b),
    (Key::Space, 0x20),
    (Key::Minus, 0xbd),
    (Key::Caret, 0xde),
    (Key::Yen, 0xdc),
    (Key::At, 0xc0),
    (Key::LeftBracket, 0xdb),
    (Key::Semicolon, 0xbb),
    (Key::Colon, 0xba),
    (Key::RightBracket, 0xdd),
    (Key::Comma, 0xbc),
    (Key::Period, 0xbe),
    (Key::Slash, 0xbf),
    (Key::Underscore, 0xe2),
    (Key::UnderscorePc98, 0xdf),
    (Key::AppMenu, 0x5d),
];

/// Other spellings that [`Key::parse`] accepts, besides Sakura's names.
const ALIASES: &[(&str, Key)] = &[
    ("Delete", Key::Delete),
    ("Backspace", Key::Backspace),
    ("Insert", Key::Insert),
    ("Escape", Key::Escape),
    ("Return", Key::Enter),
    ("PageUp", Key::PageUp),
    ("PgUp", Key::PageUp),
    ("PageDown", Key::PageDown),
    ("PgDn", Key::PageDown),
    ("Up", Key::Up),
    ("Down", Key::Down),
    ("Left", Key::Left),
    ("Right", Key::Right),
    ("Minus", Key::Minus),
    ("Caret", Key::Caret),
    ("Backslash", Key::Yen),
    ("Yen", Key::Yen),
    ("Comma", Key::Comma),
    ("Period", Key::Period),
    ("Slash", Key::Slash),
    ("Semicolon", Key::Semicolon),
    ("Colon", Key::Colon),
    ("Menu", Key::AppMenu),
];

impl Key {
    pub fn letter(c: char) -> Option<Self> {
        c.is_ascii_alphabetic()
            .then(|| Self::Letter(c.to_ascii_uppercase()))
    }

    pub fn digit(n: u8) -> Option<Self> {
        (n <= 9).then_some(Self::Digit(n))
    }

    pub fn function(n: u8) -> Option<Self> {
        (1..=24).contains(&n).then_some(Self::Function(n))
    }

    /// Sakura's key code: a Windows virtual-key code, or `VKEX_*` (0x100 and above) for the
    /// mouse. It is what `sakura.ini` and `.key` files store. A key built with an invalid
    /// field (a lowercase letter, say) gets the code of the corrected key.
    pub fn code(self) -> u16 {
        match self {
            Self::Letter(c) => u16::from(c.to_ascii_uppercase() as u8),
            Self::Digit(n) => 0x30 + u16::from(n.min(9)),
            Self::Function(n) => 0x6f + u16::from(n.clamp(1, 24)),
            Self::Mouse(m) => {
                0x100
                    + MouseAction::ALL
                        .iter()
                        .position(|&a| a == m)
                        .map_or(0, |i| i as u16)
            }
            other => NAMED_KEYS
                .iter()
                .find(|(k, _)| *k == other)
                .map_or(0, |&(_, code)| code),
        }
    }

    /// The key for Sakura's key code; `None` if the key is not in Sakura's table.
    pub fn from_code(code: u16) -> Option<Self> {
        match code {
            0x30..=0x39 => Some(Self::Digit((code - 0x30) as u8)),
            0x41..=0x5a => Some(Self::Letter(code as u8 as char)),
            0x70..=0x87 => Some(Self::Function((code - 0x6f) as u8)),
            0x100..=0x10a => Some(Self::Mouse(MouseAction::ALL[usize::from(code - 0x100)])),
            _ => NAMED_KEYS.iter().find(|(_, c)| *c == code).map(|&(k, _)| k),
        }
    }

    /// Sakura's name for the key, as in the key table (`Del`, `BkSp`, `End(Help)`, ...).
    pub fn name(self) -> &'static str {
        let code = self.code();
        KEY_BINDINGS
            .iter()
            .find(|b| b.key_code == code)
            .map_or("?", |b| b.key_name)
    }

    /// The name without Sakura's parenthesized notes (`End`, `PgDn`, `^`).
    pub fn short_name(self) -> &'static str {
        let name = self.name();
        match name.find('(') {
            // "_(PC-98)" is a different key from "_"; keep its note.
            Some(i) if i > 0 && self != Self::UnderscorePc98 => &name[..i],
            _ => name,
        }
    }

    /// True for keys that type a character (so a modifier combination may be text input).
    pub fn is_printable(self) -> bool {
        matches!(
            self,
            Self::Letter(_)
                | Self::Digit(_)
                | Self::Space
                | Self::Minus
                | Self::Caret
                | Self::Yen
                | Self::At
                | Self::LeftBracket
                | Self::Semicolon
                | Self::Colon
                | Self::RightBracket
                | Self::Comma
                | Self::Period
                | Self::Slash
                | Self::Underscore
                | Self::UnderscorePc98
        )
    }

    /// Reads a key name: Sakura's names (as in [`Key::name`]), their short forms, and common
    /// English names (`Delete`, `PageUp`, `Up`, ...). Letters and names are case-insensitive.
    pub fn parse(s: &str) -> Option<Self> {
        let mut chars = s.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            if let Some(k) = Self::letter(c) {
                return Some(k);
            }
            if let Some(d) = c.to_digit(10) {
                return Self::digit(d as u8);
            }
        }
        if let Some(n) = s
            .strip_prefix(['F', 'f'])
            .and_then(|n| n.parse::<u8>().ok())
        {
            return Self::function(n);
        }
        let eq = |name: &str| name.eq_ignore_ascii_case(s);
        let by_name = KEY_BINDINGS.iter().find(|b| eq(b.key_name));
        if let Some(k) = by_name.and_then(|b| Self::from_code(b.key_code)) {
            return Some(k);
        }
        if let Some(&(_, k)) = ALIASES.iter().find(|(name, _)| eq(name)) {
            return Some(k);
        }
        // The short names: "End" for "End(Help)".
        KEY_BINDINGS
            .iter()
            .filter_map(|b| Self::from_code(b.key_code))
            .find(|k| eq(k.short_name()))
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Which modifier keys are held. `meta` is the Command key on macOS and the Windows/Super key
/// elsewhere; `alt` is Option on macOS; `ctrl` is Control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        shift: false,
        ctrl: false,
        alt: false,
        meta: false,
    };
    pub const SHIFT: Self = Self {
        shift: true,
        ..Self::NONE
    };
    pub const CTRL: Self = Self {
        ctrl: true,
        ..Self::NONE
    };
    pub const ALT: Self = Self {
        alt: true,
        ..Self::NONE
    };
    pub const META: Self = Self {
        meta: true,
        ..Self::NONE
    };

    pub const fn with_shift(self) -> Self {
        Self {
            shift: true,
            ..self
        }
    }

    pub const fn with_ctrl(self) -> Self {
        Self { ctrl: true, ..self }
    }

    pub const fn with_alt(self) -> Self {
        Self { alt: true, ..self }
    }

    pub const fn with_meta(self) -> Self {
        Self { meta: true, ..self }
    }

    /// Index into the 8 function codes of a Sakura key row (Shift 1, Ctrl 2, Alt 4;
    /// [`crate::sakura::modifier`]). `meta` is not part of it.
    pub fn sakura_index(self) -> usize {
        use crate::sakura::modifier;
        (if self.shift { modifier::SHIFT } else { 0 })
            | (if self.ctrl { modifier::CTRL } else { 0 })
            | (if self.alt { modifier::ALT } else { 0 })
    }

    /// The modifiers of a Sakura row index (no `meta`).
    pub fn from_sakura_index(index: usize) -> Self {
        use crate::sakura::modifier;
        Self {
            shift: index & modifier::SHIFT != 0,
            ctrl: index & modifier::CTRL != 0,
            alt: index & modifier::ALT != 0,
            meta: false,
        }
    }
}

/// How to write a chord.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayStyle {
    /// Sakura's style: `Shift+Ctrl+Alt+Key` (the order of `CKeyBind::GetKeyStrSub`), with
    /// `Cmd` for `meta`. Menus and settings on Windows and Linux.
    Sakura,
    /// Mac symbols in the order of Mac menus: `⌃⌥⇧⌘` and the key's short name.
    Mac,
}

/// A key with the modifiers held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub key: Key,
    pub mods: Modifiers,
}

impl Chord {
    pub const fn new(key: Key, mods: Modifiers) -> Self {
        Self { key, mods }
    }

    /// Writes the chord in the given style.
    pub fn display(self, style: DisplayStyle) -> String {
        let m = self.mods;
        let mut out = String::new();
        match style {
            DisplayStyle::Sakura => {
                for (on, name) in [
                    (m.shift, "Shift+"),
                    (m.ctrl, "Ctrl+"),
                    (m.alt, "Alt+"),
                    (m.meta, "Cmd+"),
                ] {
                    if on {
                        out.push_str(name);
                    }
                }
                out.push_str(self.key.name());
            }
            DisplayStyle::Mac => {
                for (on, name) in [(m.ctrl, "⌃"), (m.alt, "⌥"), (m.shift, "⇧"), (m.meta, "⌘")]
                {
                    if on {
                        out.push_str(name);
                    }
                }
                out.push_str(self.key.short_name());
            }
        }
        out
    }

    /// Reads `Ctrl+Shift+S`, `Alt+F4`, `Cmd+,` and so on: modifiers in any order and case
    /// (`Ctrl`/`Control`, `Alt`/`Option`, `Shift`, `Cmd`/`Command`/`Meta`/`Win`/`Super`), then
    /// the key (see [`Key::parse`]).
    pub fn parse(s: &str) -> Option<Self> {
        let (mods_part, key_part) = match s.rfind('+') {
            // A '+' at the end would be the key itself, which Sakura's table does not have.
            Some(i) if i + 1 < s.len() => (&s[..i], &s[i + 1..]),
            Some(_) => return None,
            None => ("", s),
        };
        let key = Key::parse(key_part.trim())?;
        let mut mods = Modifiers::NONE;
        if !mods_part.is_empty() {
            for part in mods_part.split('+') {
                match part.trim().to_ascii_lowercase().as_str() {
                    "shift" => mods.shift = true,
                    "ctrl" | "control" => mods.ctrl = true,
                    "alt" | "option" => mods.alt = true,
                    "cmd" | "command" | "meta" | "win" | "super" => mods.meta = true,
                    _ => return None,
                }
            }
        }
        Some(Self { key, mods })
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display(DisplayStyle::Sakura))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_of_sakura_s_table_round_trips() {
        for b in KEY_BINDINGS {
            let key = Key::from_code(b.key_code)
                .unwrap_or_else(|| panic!("no key for code {:#x} ({})", b.key_code, b.key_name));
            assert_eq!(key.code(), b.key_code, "{}", b.key_name);
            assert_eq!(key.name(), b.key_name);
            assert_eq!(Key::parse(b.key_name), Some(key), "{}", b.key_name);
            assert_eq!(Key::parse(key.short_name()), Some(key), "{}", b.key_name);
        }
    }

    #[test]
    fn parses_common_names() {
        assert_eq!(Key::parse("s"), Key::letter('S'));
        assert_eq!(Key::parse("f3"), Some(Key::Function(3)));
        assert_eq!(Key::parse("F24"), Some(Key::Function(24)));
        assert_eq!(Key::parse("F25"), None);
        assert_eq!(Key::parse("Delete"), Some(Key::Delete));
        assert_eq!(Key::parse("del"), Some(Key::Delete));
        assert_eq!(Key::parse("PageDown"), Some(Key::PageDown));
        assert_eq!(Key::parse("PgDn"), Some(Key::PageDown));
        assert_eq!(Key::parse("End"), Some(Key::End));
        assert_eq!(Key::parse("←"), Some(Key::Left));
        assert_eq!(Key::parse("\\"), Some(Key::Yen));
        assert_eq!(Key::parse("^"), Some(Key::Caret));
        assert_eq!(
            Key::parse("ダブルクリック"),
            Some(Key::Mouse(MouseAction::DoubleClick))
        );
        assert_eq!(Key::parse("nonsense"), None);
        assert_eq!(Key::parse(""), None);
    }

    #[test]
    fn short_names_drop_the_notes() {
        assert_eq!(Key::End.short_name(), "End");
        assert_eq!(Key::PageDown.short_name(), "PgDn");
        assert_eq!(Key::Caret.short_name(), "^");
        assert_eq!(Key::Underscore.short_name(), "_");
        assert_eq!(Key::UnderscorePc98.short_name(), "_(PC-98)");
    }

    #[test]
    fn chords_are_written_in_sakura_s_order() {
        // `CKeyBind::GetKeyStrSub` writes Shift, Ctrl, Alt, then the key name.
        let c = Chord::new(Key::Function(3), Modifiers::CTRL.with_shift());
        assert_eq!(c.to_string(), "Shift+Ctrl+F3");
        let c = Chord::new(Key::End, Modifiers::CTRL);
        assert_eq!(c.to_string(), "Ctrl+End(Help)");
        let c = Chord::new(Key::Letter('S'), Modifiers::META.with_shift());
        assert_eq!(c.display(DisplayStyle::Mac), "⇧⌘S");
        let c = Chord::new(Key::Left, Modifiers::CTRL.with_alt().with_meta());
        assert_eq!(c.display(DisplayStyle::Mac), "⌃⌥⌘←");
    }

    #[test]
    fn chords_parse_and_round_trip() {
        let c = Chord::parse("ctrl+shift+s").unwrap();
        assert_eq!(
            c,
            Chord::new(Key::Letter('S'), Modifiers::CTRL.with_shift())
        );
        assert_eq!(Chord::parse(&c.to_string()), Some(c));
        assert_eq!(
            Chord::parse("Option+Command+Left"),
            Some(Chord::new(Key::Left, Modifiers::ALT.with_meta()))
        );
        assert_eq!(
            Chord::parse("Ctrl+-"),
            Some(Chord::new(Key::Minus, Modifiers::CTRL))
        );
        assert_eq!(Chord::parse("Ctrl+"), None);
        assert_eq!(Chord::parse("Hyper+S"), None);
        assert_eq!(Chord::parse("Ctrl+Nope"), None);
        for b in KEY_BINDINGS {
            let key = Key::from_code(b.key_code).unwrap();
            for i in 0..8 {
                let chord = Chord::new(key, Modifiers::from_sakura_index(i));
                assert_eq!(Chord::parse(&chord.to_string()), Some(chord), "{chord}");
            }
        }
    }

    #[test]
    fn sakura_index_matches_the_modifier_bits() {
        assert_eq!(Modifiers::NONE.sakura_index(), 0);
        assert_eq!(Modifiers::CTRL.with_shift().sakura_index(), 3);
        assert_eq!(Modifiers::ALT.with_ctrl().sakura_index(), 6);
        assert_eq!(Modifiers::META.sakura_index(), 0);
        for i in 0..8 {
            assert_eq!(Modifiers::from_sakura_index(i).sakura_index(), i);
        }
    }
}
