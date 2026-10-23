//! `F_CHGMOD_EOL`: sets the line ending that Enter inserts, by the macro's number for it. This is
//! the macro `ChgmodEOL(type)`; the menu and the keys use `F_CHGMOD_EOL_CRLF`, `F_CHGMOD_EOL_LF`
//! and `F_CHGMOD_EOL_CR`, which do the same.
//!
//! Sakura: `CViewCommander::Command_CHGMOD_EOL` in `sakura_core/cmd/CViewCommander_ModeChange.cpp`,
//! and the macro table of `sakura_core/macro/CSMacroMgr.cpp` (`ChgmodEOL`, one number). The macro's
//! number is not `EEolType`: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`, `case
//! F_CHGMOD_EOL`) converts 1 CRLF, 3 LF, 4 CR, 5 NEL, 6 LS, 7 PS, and any other number (0, 2, out
//! of range) becomes `EEolType::none`, which does nothing, without a message. The help
//! (`help/macro/S_ChgmodEOL.html`) lists 1, 3 and 4. The key macro records the same numbers
//! (`CMacro::AddLParam`).
//!
//! The setting is the document's: the status bar shows it (`CCaret::ShowCaretPosInfo`), Enter
//! and the line breaks the editor makes use it, and saving asks to unify mixed line endings to it
//! (`CCodeChecker`, `STR_CODECHECKER_EOL_UNIFY`). A plain save does not convert the lines. A
//! document that is opened or reopened starts again with the line ending of its first line, and a
//! new document with the one of the settings.

use crate::command::{Arg, Command, Result};
use crate::context::{CommandContext, eol_of_macro_value};
use crate::register_command;

#[derive(Default)]
pub struct ChgmodEol;

impl Command for ChgmodEol {
    const ID: &'static str = "chgmod_eol";
    const SAKURA: &'static str = "F_CHGMOD_EOL";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        let value = match args.first() {
            Some(Arg::Int(n)) => *n,
            // the macro reads a missing argument as 0
            None => 0,
            Some(_) => {
                return Err(crate::command::CommandError::BadArgs(
                    "chgmod_eol takes a number".to_owned(),
                ));
            }
        };
        if let Some(eol) = eol_of_macro_value(value) {
            cx.set_new_line(eol);
        }
        Ok(())
    }
}

register_command!(ChgmodEol);

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Eol;

    use crate::command::{Arg, CommandError};
    use crate::testing::Editor;

    fn run(e: &mut Editor, n: i64) {
        e.run_with("chgmod_eol", &[Arg::Int(n)]);
    }

    #[test]
    fn the_numbers_are_the_macro_values() {
        let mut e = Editor::new("ab|cd").with_new_line(Eol::Lf);
        for (n, eol) in [
            (1, Eol::CrLf),
            (3, Eol::Lf),
            (4, Eol::Cr),
            (5, Eol::Nel),
            (6, Eol::Ls),
            (7, Eol::Ps),
        ] {
            run(&mut e, n);
            assert_eq!(e.model().new_line(), eol, "{n}");
        }
    }

    #[test]
    fn a_number_that_is_not_valid_does_nothing() {
        let mut e = Editor::new("ab|cd").with_new_line(Eol::Cr);
        for n in [0, 2, 8, -1, 100] {
            run(&mut e, n);
            assert_eq!(e.model().new_line(), Eol::Cr, "{n}");
        }
        e.run("chgmod_eol");
        assert_eq!(e.model().new_line(), Eol::Cr);
        assert!(!e.changes().beep);
    }

    #[test]
    fn a_text_argument_is_a_mistake() {
        let mut e = Editor::new("ab|cd");
        let result = e.try_run("chgmod_eol", &[Arg::Text("lf".to_owned())]);
        assert!(matches!(result, Err(CommandError::BadArgs(_))));
    }

    #[test]
    fn enter_inserts_the_line_ending() {
        let mut e = Editor::new("ab|cd").with_new_line(Eol::CrLf);
        run(&mut e, 3);
        e.run_with("input_char", &[Arg::Char('\n')]);
        e.assert_marked("ab\n|cd");
        run(&mut e, 4);
        e.run_with("input_char", &[Arg::Char('\n')]);
        e.assert_marked("ab\n\r|cd");
    }

    #[test]
    fn it_is_not_an_edit_and_not_an_undo_step() {
        let mut e = Editor::new("ab|cd");
        run(&mut e, 3);
        assert!(!e.changes().text_changed());
        assert!(!e.model().is_modified());
        assert!(!e.model().history().can_undo());
    }
}
