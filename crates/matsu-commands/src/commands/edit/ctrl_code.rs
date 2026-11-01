//! `F_CTRL_CODE`: types one control code (`CtrlCode(n)` in a macro; the control code dialog runs it).
//!
//! Sakura: `CViewCommander::HandleCommand` calls `Command_WCHAR((wchar_t)lparam1, false)`
//! (`sakura_core/cmd/CViewCommander.cpp`). It is [`input_char`](super::input_char) with one
//! difference: the `false` is `bConvertEOL`, so CR and LF are inserted as they are, not as the
//! line ending of the document. The indentation of the line is copied after them as for Enter
//! (auto indent).
//!
//! Argument: the code, as `Arg::Int` (or one `Arg::Char`).

use super::input_char::type_char_as;
use crate::command::{Arg, Command, CommandError, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CtrlCode;

impl Command for CtrlCode {
    const ID: &'static str = "ctrl_code";
    const SAKURA: &'static str = "F_CTRL_CODE";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        let c = match args {
            [Arg::Int(n)] => u32::try_from(*n).ok().and_then(char::from_u32),
            [Arg::Char(c)] => Some(*c),
            _ => None,
        }
        .ok_or_else(|| CommandError::BadArgs("ctrl_code takes one code".to_owned()))?;
        type_char_as(cx, c, false)
    }
}

register_command!(CtrlCode);

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Eol;

    use super::*;
    use crate::testing::{Editor, check_with};

    const ID: &str = "ctrl_code";

    #[test]
    fn types_the_code_at_the_caret() {
        check_with(ID, &[Arg::Int(0x09)], "a|b", "a\t|b");
        check_with(ID, &[Arg::Int(0x1b)], "a|b", "a\u{1b}|b");
        check_with(ID, &[Arg::Int(0x7f)], "a|", "a\u{7f}|");
    }

    #[test]
    fn replaces_the_selection() {
        check_with(ID, &[Arg::Int(0x07)], "a[b|c", "a\u{7}|c");
    }

    #[test]
    fn cr_and_lf_are_not_converted_to_the_line_ending_of_the_document() {
        // `Command_WCHAR(c, false)`: a bare LF in a CRLF document stays a bare LF.
        let mut e = Editor::new("ab|cd").with_new_line(Eol::CrLf);
        e.run_with(ID, &[Arg::Int(0x0a)]).assert_marked("ab\n|cd");
    }

    #[test]
    fn the_indentation_is_copied_after_a_line_break() {
        let mut e = Editor::new("  ab|cd").with_new_line(Eol::Lf);
        e.run_with(ID, &[Arg::Int(0x0a)])
            .assert_marked("  ab\n  |cd");
    }

    #[test]
    fn a_bad_code_is_refused() {
        let mut e = Editor::new("|");
        assert!(e.try_run(ID, &[Arg::Int(-1)]).is_err());
        assert!(e.try_run(ID, &[]).is_err());
    }
}
