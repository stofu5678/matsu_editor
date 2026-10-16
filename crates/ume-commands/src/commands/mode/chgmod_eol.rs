//! `F_CHGMOD_EOL`: sets the line ending that Enter inserts, by Sakura's number for it. This is the
//! macro `ChgmodEOL(type)`; the menu and the keys use `F_CHGMOD_EOL_CRLF`, `F_CHGMOD_EOL_LF` and
//! `F_CHGMOD_EOL_CR`, which call it.
//!
//! Sakura: `CViewCommander::Command_CHGMOD_EOL` in `sakura_core/cmd/CViewCommander_ModeChange.cpp`,
//! and the macro table of `sakura_core/macro/CSMacroMgr.cpp` (`ChgmodEOL`, one number). The number
//! is `EEolType` (`sakura_core/basis/CEol.h`): 1 CRLF, 2 LF, 3 CR, 4 NEL, 5 LS, 6 PS. A number that
//! `CEol::IsValid` refuses (0 and anything out of range) does nothing, without a message.
//!
//! The setting is the document's: the status bar shows it (`CCaret::ShowCaretPosInfo`), Enter
//! and the line breaks the editor makes use it, and saving asks to unify mixed line endings to it
//! (`CCodeChecker`, `STR_CODECHECKER_EOL_UNIFY`). A plain save does not convert the lines. A
//! document that is opened or reopened starts again with the line ending of its first line, and a
//! new document with the one of the settings.

use crate::command::{Arg, Command, Result};
use crate::context::{CommandContext, eol_of_type};
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
        if let Some(eol) = eol_of_type(value) {
            cx.set_new_line(eol);
        }
        Ok(())
    }
}

register_command!(ChgmodEol);

#[cfg(test)]
mod tests {
    use ume_core::buffer::Eol;

    use crate::command::{Arg, CommandError};
    use crate::testing::Editor;

    fn run(e: &mut Editor, n: i64) {
        e.run_with("chgmod_eol", &[Arg::Int(n)]);
    }

    #[test]
    fn the_numbers_are_sakuras_eol_types() {
        let mut e = Editor::new("ab|cd").with_new_line(Eol::Lf);
        for (n, eol) in [
            (1, Eol::CrLf),
            (2, Eol::Lf),
            (3, Eol::Cr),
            (4, Eol::Nel),
            (5, Eol::Ls),
            (6, Eol::Ps),
        ] {
            run(&mut e, n);
            assert_eq!(e.model().new_line(), eol, "{n}");
        }
    }

    #[test]
    fn a_number_that_is_not_valid_does_nothing() {
        let mut e = Editor::new("ab|cd").with_new_line(Eol::Cr);
        for n in [0, 7, -1, 100] {
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
        run(&mut e, 2);
        e.run_with("input_char", &[Arg::Char('\n')]);
        e.assert_marked("ab\n|cd");
        run(&mut e, 3);
        e.run_with("input_char", &[Arg::Char('\n')]);
        e.assert_marked("ab\n\r|cd");
    }

    #[test]
    fn it_is_not_an_edit_and_not_an_undo_step() {
        let mut e = Editor::new("ab|cd");
        run(&mut e, 2);
        assert!(!e.changes().text_changed());
        assert!(!e.model().is_modified());
        assert!(!e.model().history().can_undo());
    }
}
