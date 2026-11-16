//! Alt+letter in a dialog: the access keys that the labels show (`実行(E)`, `アウトプットに出す(O)`).
//!
//! In a Windows dialog, Alt with the letter of a label uses that control: a button is pressed, a check
//! box switched, a radio button chosen, and the label of a field moves the focus to the field
//! (`IsDialogMessage`). The letter is matched without case, and the key is the dialog's, wherever the
//! focus is: from inside a text field too, where the letter is not typed. Our dialogs draw their
//! controls themselves, so they have to do that; this module is what they share:
//!
//! * [`letter_of`] reads the letter out of a key event (and leaves alone what is not an access key:
//!   AltGr, an IME in the middle of a composition, a key that makes a character);
//! * [`pick`] finds the control of a letter among the ones a dialog shows, and says when two controls
//!   share it;
//! * [`OnAccessKey::on_access_key`] puts the dialog's handler on its panel, ahead of the controls inside
//!   it, so that a text field does not see the key and the main window's menu bar (which opens on
//!   Alt+letter when the text has the focus) does not either.
//!
//! The dialogs write the key after the text of the label, as Sakura's resource file does (`実行(E)`); the
//! settings dialogs use `&E` (`crate::common_settings::model::parse_label`). Either way the key is a
//! single letter or digit, lowercased here.

use gpui::{App, InteractiveElement, KeyDownEvent, Window};

/// The access key that `label` writes after its text: `e` for `実行(E)`, `o` for `開く(O)...`.
pub(crate) fn hint_of(label: &str) -> Option<char> {
    crate::a11y::key_hint(label).map(|key| key.to_ascii_lowercase())
}

/// The letter of an Alt+letter key event, lowercased, or none when the event is not an access key. The
/// letter is one character: a letter, a digit or a sign (`.`). Shift does not matter. Control (AltGr
/// on Windows is Control+Alt), the platform key, an IME that is composing and a key that is a typed
/// character are not access keys.
pub(crate) fn letter_of(event: &KeyDownEvent) -> Option<char> {
    let keystroke = &event.keystroke;
    let m = keystroke.modifiers;
    if !m.alt
        || m.control
        || m.platform
        || m.function
        || event.prefer_character_input
        || keystroke.is_ime_in_progress()
    {
        return None;
    }
    let mut chars = keystroke.key.chars();
    let (Some(letter), None) = (chars.next(), chars.next()) else {
        return None;
    };
    Some(letter.to_lowercase().next().unwrap_or(letter))
}

/// A control of a dialog as far as its access key goes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Entry<T> {
    /// The key the label shows.
    pub key: Option<char>,
    /// The control is shown and not greyed out: a control that is not is skipped, so its letter can be
    /// another control's (the 登録 box of the one-liner dialog replaces the buttons).
    pub usable: bool,
    /// What the dialog does with the key.
    pub what: T,
}

impl<T> Entry<T> {
    /// A control whose key is not at the end of the label (`選択始点(1)挿入`).
    pub(crate) fn keyed(key: char, usable: bool, what: T) -> Self {
        Self {
            key: Some(key),
            usable,
            what,
        }
    }

    /// A control whose label ends with the key (`実行(E)`).
    pub(crate) fn labelled(label: &str, usable: bool, what: T) -> Self {
        Self {
            key: hint_of(label),
            usable,
            what,
        }
    }
}

/// What a letter finds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Found<T> {
    /// No usable control has the letter.
    Nothing,
    /// One does.
    One(T),
    /// Several do. A Windows dialog moves the focus from one to the next and uses none; the controls
    /// of ours cannot take the focus from outside, so the dialog does nothing, and still takes the key.
    Several,
}

/// The control that `letter` (lowercase, as [`letter_of`] gives it) is the key of.
pub(crate) fn pick<T: Copy>(letter: char, entries: &[Entry<T>]) -> Found<T> {
    let mut found = Found::Nothing;
    for entry in entries {
        if entry.key != Some(letter) || !entry.usable {
            continue;
        }
        found = match found {
            Found::Nothing => Found::One(entry.what),
            _ => return Found::Several,
        };
    }
    found
}

/// `.on_access_key(handler)` on the panel of a dialog: Alt+letter goes to `handler` before the controls
/// inside the panel see it. The handler returns whether the letter was one of the dialog's; if so the
/// key stops there (no letter is typed into a text field, no menu of the window behind opens).
pub(crate) trait OnAccessKey: InteractiveElement + Sized {
    fn on_access_key(
        self,
        handler: impl Fn(char, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.capture_key_down(move |event, window, cx| {
            if let Some(letter) = letter_of(event)
                && handler(letter, window, cx)
            {
                cx.stop_propagation();
            }
        })
    }
}

impl<E: InteractiveElement> OnAccessKey for E {}

#[cfg(test)]
mod tests {
    use gpui::{Keystroke, Modifiers};

    use super::*;

    fn event(key: &str, modifiers: Modifiers) -> KeyDownEvent {
        KeyDownEvent {
            keystroke: Keystroke {
                modifiers,
                key: key.to_owned(),
                key_char: None,
            },
            is_held: false,
            prefer_character_input: false,
        }
    }

    const ALT: Modifiers = Modifiers {
        alt: true,
        control: false,
        shift: false,
        platform: false,
        function: false,
    };

    #[test]
    fn the_key_of_a_label_is_the_letter_in_parentheses_without_case() {
        assert_eq!(hint_of("実行(E)"), Some('e'));
        assert_eq!(hint_of("登録...(T)"), Some('t'));
        assert_eq!(hint_of("選択文字(0)"), Some('0'));
        assert_eq!(hint_of("履歴"), None);
        assert_eq!(hint_of("置換"), None);
    }

    #[test]
    fn alt_with_a_letter_is_an_access_key_whatever_the_case_and_shift() {
        assert_eq!(letter_of(&event("e", ALT)), Some('e'));
        assert_eq!(letter_of(&event("E", ALT)), Some('e'));
        let shifted = Modifiers { shift: true, ..ALT };
        assert_eq!(letter_of(&event("e", shifted)), Some('e'));
        assert_eq!(letter_of(&event("3", ALT)), Some('3'));
        assert_eq!(letter_of(&event(".", ALT)), Some('.'));
    }

    #[test]
    fn what_is_not_alt_with_one_character_is_not_an_access_key() {
        assert_eq!(letter_of(&event("e", Modifiers::none())), None);
        assert_eq!(letter_of(&event("enter", ALT)), None);
        assert_eq!(letter_of(&event("f4", ALT)), None);
        // AltGr is Control+Alt
        let alt_gr = Modifiers {
            control: true,
            ..ALT
        };
        assert_eq!(letter_of(&event("e", alt_gr)), None);
        let platform = Modifiers {
            platform: true,
            ..ALT
        };
        assert_eq!(letter_of(&event("e", platform)), None);
        // a typed character
        let mut typed = event("e", ALT);
        typed.prefer_character_input = true;
        assert_eq!(letter_of(&typed), None);
    }

    fn entry(label: &str, usable: bool, what: u8) -> Entry<u8> {
        Entry::labelled(label, usable, what)
    }

    #[test]
    fn a_letter_finds_the_one_usable_control_that_has_it() {
        let entries = [
            entry("実行(E)", true, 1),
            entry("登録...(T)", true, 2),
            entry("置き換える(R)", false, 3),
            entry("履歴", true, 4),
        ];
        assert_eq!(pick('e', &entries), Found::One(1));
        assert_eq!(pick('t', &entries), Found::One(2));
        // greyed out, no key, no such key
        assert_eq!(pick('r', &entries), Found::Nothing);
        assert_eq!(pick('q', &entries), Found::Nothing);
    }

    #[test]
    fn a_letter_that_two_usable_controls_share_finds_neither() {
        let entries = [
            entry("該当行マーク(M)", true, 1),
            entry("メッセージを表示(M)", true, 2),
            entry("三つ目(M)", false, 3),
        ];
        assert_eq!(pick('m', &entries), Found::Several);
        // with one of them gone the other is the one
        let entries = [
            entry("該当行マーク(M)", true, 1),
            entry("メッセージを表示(M)", false, 2),
        ];
        assert_eq!(pick('m', &entries), Found::One(1));
    }
}
