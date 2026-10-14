//! Settings for how Sakura's key table is read on each OS, and the macOS tables behind them.
//!
//! ADR-0008 decided that on macOS Ctrl is read as Cmd and Alt as Option, with a setting that
//! keeps Ctrl, and left four points open. The defaults below are a proposal for them; each
//! one can be switched off by a field of [`KeyMapOptions`].

use super::key::{Key, Modifiers};

/// Which OS's rules to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// Windows and Linux: Sakura's table as it is. The meta key (Windows/Super) is not used.
    Other,
    MacOs,
}

impl Platform {
    /// The OS this program was built for.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Other
        }
    }
}

/// What the Control key does on macOS while Ctrl is read as Cmd.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKey {
    /// macOS's Emacs-style text keys (Control+A line start, Control+E line end, ...); see
    /// `EMACS_KEYS`.
    Emacs,
    /// Not bound; left to the OS.
    Ignore,
}

/// Settings of the key map. The fields other than `platform` only matter on macOS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyMapOptions {
    pub platform: Platform,
    /// Read Ctrl of Sakura's table as Cmd (ADR-0008). Off keeps Ctrl: Sakura's table works as
    /// on Windows, with Control as Ctrl. Option is Alt in both cases.
    pub ctrl_as_cmd: bool,
    /// Give Cmd chords that clash with macOS's standard keys their Mac meaning
    /// (`STANDARD_KEYS`): Cmd+W closes, Cmd+Left/Right go to the line start/end, and so on.
    /// Needs a row of Sakura's table to be unchanged by the user (see [`super::KeyMap`]).
    pub standard_keys: bool,
    /// What Control does while `ctrl_as_cmd` is on. (Off: Control is Ctrl.)
    pub control_key: ControlKey,
    /// Option+Left/Right move by word, as in macOS (`OPTION_ARROWS`). Off keeps Sakura's
    /// Alt+arrows (box selection), which ADR-0008 maps to Option.
    pub option_arrows_by_word: bool,
    /// Never read Option+character as a command, so that the character is typed (for example
    /// Option+¥ for a backslash, or dead keys of other layouts). Off keeps Sakura's Alt
    /// bindings on the letters, digits and symbols (Alt+D sort, Alt+; date, ...).
    pub option_text_input: bool,
}

impl KeyMapOptions {
    /// The proposed defaults for `platform`.
    pub fn for_platform(platform: Platform) -> Self {
        Self {
            platform,
            ctrl_as_cmd: true,
            standard_keys: true,
            control_key: ControlKey::Emacs,
            option_arrows_by_word: false,
            option_text_input: false,
        }
    }
}

impl Default for KeyMapOptions {
    fn default() -> Self {
        Self::for_platform(Platform::current())
    }
}

/// A chord of the keyboard (all four modifiers are physical) that macOS gives its own meaning.
/// `function` is a name from `FUNCTIONS` (`F_...`); `None` leaves the chord to the OS or the
/// menu bar (hide, minimize).
pub(super) type OverlayEntry = (Key, Modifiers, Option<&'static str>);

const CMD: Modifiers = Modifiers::META;
const CMD_SHIFT: Modifiers = Modifiers::META.with_shift();
const OPT: Modifiers = Modifiers::ALT;
const OPT_SHIFT: Modifiers = Modifiers::ALT.with_shift();
const CTL: Modifiers = Modifiers::CTRL;

/// Cmd chords where reading Ctrl as Cmd clashes with macOS's standard keys, and the keys that
/// Sakura leaves free where macOS has a meaning. The UI PoC used the Mac meaning too
/// (`docs/poc/ui-framework-result.md`).
///
/// * Cmd+W: Sakura selects the word (double click does that too); Mac closes the window.
/// * Cmd+Left/Right/Up/Down: Sakura moves by word / scrolls; Mac goes to the line or file ends.
/// * Cmd+Backspace: Sakura deletes a word; Mac deletes to the line start.
/// * Cmd+H, Cmd+Option+H, Cmd+M: hide and minimize belong to the app menu. (Sakura: scroll the
///   caret line to the center; window arrangement.)
/// * Cmd+Q: quit (Sakura: list the key bindings). Cmd+, : settings (Sakura: Ctrl+6).
/// * Cmd+Shift+Z: redo (Sakura has Ctrl+Y, which stays as Cmd+Y).
/// * Cmd+G / Cmd+Shift+G: find next / previous, the Mac habit for Sakura's F3 / Shift+F3,
///   which need fn on a Mac keyboard. Sakura's Ctrl+G (Grep) moves to Cmd+Shift+F.
/// * Option+Backspace / Option+Delete: delete by word; Sakura leaves them free.
pub(super) static STANDARD_KEYS: &[OverlayEntry] = &[
    (Key::Letter('W'), CMD, Some("F_WINCLOSE")),
    (Key::Letter('Q'), CMD, Some("F_EXITALLEDITORS")),
    (Key::Comma, CMD, Some("F_OPTION")),
    (Key::Letter('Z'), CMD_SHIFT, Some("F_REDO")),
    (Key::Letter('G'), CMD, Some("F_SEARCH_NEXT")),
    (Key::Letter('G'), CMD_SHIFT, Some("F_SEARCH_PREV")),
    (Key::Letter('F'), CMD_SHIFT, Some("F_GREP_DIALOG")),
    (Key::Letter('H'), CMD, None),
    (Key::Letter('H'), CMD.with_alt(), None),
    (Key::Letter('M'), CMD, None),
    (Key::Left, CMD, Some("F_GOLINETOP")),
    (Key::Left, CMD_SHIFT, Some("F_GOLINETOP_SEL")),
    (Key::Right, CMD, Some("F_GOLINEEND")),
    (Key::Right, CMD_SHIFT, Some("F_GOLINEEND_SEL")),
    (Key::Up, CMD, Some("F_GOFILETOP")),
    (Key::Up, CMD_SHIFT, Some("F_GOFILETOP_SEL")),
    (Key::Down, CMD, Some("F_GOFILEEND")),
    (Key::Down, CMD_SHIFT, Some("F_GOFILEEND_SEL")),
    (Key::Backspace, CMD, Some("F_LineDeleteToStart")),
    (Key::Backspace, OPT, Some("F_WordDeleteToStart")),
    (Key::Delete, OPT, Some("F_WordDeleteToEnd")),
];

/// Option+arrows by word, as in macOS (`option_arrows_by_word`). Sakura binds Alt+arrows to
/// box selection.
pub(super) static OPTION_ARROWS: &[OverlayEntry] = &[
    (Key::Left, OPT, Some("F_WORDLEFT")),
    (Key::Left, OPT_SHIFT, Some("F_WORDLEFT_SEL")),
    (Key::Right, OPT, Some("F_WORDRIGHT")),
    (Key::Right, OPT_SHIFT, Some("F_WORDRIGHT_SEL")),
];

/// macOS's Emacs-style text keys on Control (`ControlKey::Emacs`). Only the ones that have a
/// Sakura function. Control+K cuts to the line end, like Sakura's Ctrl+K.
pub(super) static EMACS_KEYS: &[OverlayEntry] = &[
    (Key::Letter('A'), CTL, Some("F_GOLINETOP")),
    (Key::Letter('E'), CTL, Some("F_GOLINEEND")),
    (Key::Letter('F'), CTL, Some("F_RIGHT")),
    (Key::Letter('B'), CTL, Some("F_LEFT")),
    (Key::Letter('N'), CTL, Some("F_DOWN")),
    (Key::Letter('P'), CTL, Some("F_UP")),
    (Key::Letter('D'), CTL, Some("F_DELETE")),
    (Key::Letter('H'), CTL, Some("F_DELETE_BACK")),
    (Key::Letter('K'), CTL, Some("F_LineCutToEnd")),
    (Key::Letter('V'), CTL, Some("F_1PageDown")),
];
