//! `F_IME_CHAR`: types one char that the IME sent (`CharIme(n)` in a macro).
//!
//! Sakura: `CViewCommander::Command_IME_CHAR( WORD wChar )` in
//! `sakura_core/cmd/CViewCommander_Edit.cpp`. `WM_IME_CHAR` sends it (`CEditView.cpp`), but only
//! in overwrite mode: in insert mode the window takes the whole string of `WM_IME_COMPOSITION`
//! (see the comment above the function). The argument is one UTF-16 unit of the Unicode build.
//!
//! * A value below 0x100 (`0 == (wChar & 0xff00)`) is `Command_WCHAR( wChar & 0xff )`, which is
//!   [`input_char`](super::input_char).
//! * Otherwise the char replaces a selection (a box: `Command_INDENT`, the char goes before the
//!   box in every row), or, in overwrite mode with nothing selected, the char at the caret
//!   (`DelCharForOverwrite`); else it is inserted. That is what `input_char` does for any char
//!   that is not a line break, so both go through the same code.
//!
//! Not done: the beep while the mouse selects (`IsMouseSelecting`; a command has no mouse
//! state) and hiding the mouse pointer (`m_bHideMouse`).
//!
//! Known difference (type B of ADR-0018): Sakura inserts a value in the surrogate range as a
//! lone UTF-16 unit, and the two halves of a pair, sent one after the other, join in its
//! buffer. Text here is UTF-8, which has no lone halves, so such a value is refused. The IME of
//! this program hands over the whole string (`record_typed_text`), where a pair is one char.
//!
//! Argument: the value, as `Arg::Int`; it is cut to 16 bits (`(WORD)lparam1`).

use super::input_char::type_char;
use crate::command::{Arg, Command, CommandError, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct ImeChar;

impl Command for ImeChar {
    const ID: &'static str = "ime_char";
    const SAKURA: &'static str = "F_IME_CHAR";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        let bad = |what: &str| CommandError::BadArgs(format!("ime_char: {what}"));
        let [Arg::Int(n)] = args else {
            return Err(bad("takes one number"));
        };
        // `(WORD)lparam1`: the value is cut to 16 bits, as the cast does. Below 0x100 the value
        // is the same after `wChar & 0xff`.
        let unit = (*n & 0xffff) as u32;
        let c = char::from_u32(unit)
            .ok_or_else(|| bad("a half of a surrogate pair cannot be typed alone"))?;
        type_char(cx, c)
    }
}

register_command!(ImeChar);

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Eol;

    use crate::command::{Arg, CommandError};
    use crate::testing::{Editor, check_with};

    const ID: &str = "ime_char";

    #[test]
    fn types_a_full_width_char() {
        check_with(ID, &[Arg::Int(0x65E5)], "a|b", "a日|b");
        check_with(ID, &[Arg::Int(0x3042)], "|", "あ|");
    }

    #[test]
    fn a_value_below_0x100_is_a_typed_char() {
        // `Command_WCHAR( wChar & 0xff )`
        check_with(ID, &[Arg::Int(0x41)], "a|b", "aA|b");
        check_with(ID, &[Arg::Int(0xE9)], "|", "é|");
        let mut e = Editor::new("ab|cd").with_new_line(Eol::Lf);
        e.run_with(ID, &[Arg::Int(0x0d)]).assert_marked("ab\n|cd");
    }

    #[test]
    fn the_value_is_cut_to_16_bits() {
        check_with(ID, &[Arg::Int(0x1_3042)], "|", "あ|");
    }

    #[test]
    fn replaces_the_selection() {
        check_with(ID, &[Arg::Int(0x65E5)], "a[bc|d", "a日|d");
    }

    #[test]
    fn overwrites_in_overwrite_mode() {
        let mut e = Editor::new("a|bc");
        e.set_overwrite(true);
        e.run_with(ID, &[Arg::Int(0x65E5)]).assert_marked("a日|c");
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("ab|cd");
        e.run_with(ID, &[Arg::Int(0x65E5)]);
        e.run("undo").assert_marked("ab|cd");
    }

    #[test]
    fn a_lone_surrogate_and_bad_arguments_are_refused() {
        for args in [
            &[][..],
            &[Arg::Char('a')],
            &[Arg::Int(0xD83D)],
            &[Arg::Int(0xDE00)],
        ] {
            let err = Editor::new("|").try_run(ID, args).unwrap_err();
            assert!(matches!(err, CommandError::BadArgs(_)), "{args:?}");
        }
    }
}
