//! `F_CHGMOD_EOL_CRLF`: Enter inserts CR LF in this document (入力改行コード指定(CRLF)).
//!
//! Sakura: `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp` runs
//! `F_CHGMOD_EOL` with `EEolType::cr_and_lf`; see `chgmod_eol`.

use ume_core::buffer::Eol;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct ChgmodEolCrlf;

impl Command for ChgmodEolCrlf {
    const ID: &'static str = "chgmod_eol_crlf";
    const SAKURA: &'static str = "F_CHGMOD_EOL_CRLF";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.set_new_line(Eol::CrLf);
        Ok(())
    }
}

register_command!(ChgmodEolCrlf);

#[cfg(test)]
mod tests {
    use ume_core::buffer::Eol;

    use crate::command::Arg;
    use crate::testing::Editor;

    #[test]
    fn sets_the_line_ending_of_enter() {
        let mut e = Editor::new("日本|語").with_new_line(Eol::Lf);
        e.run("chgmod_eol_crlf");
        assert_eq!(e.model().new_line(), Eol::CrLf);
        e.run_with("input_char", &[Arg::Char('\n')]);
        e.assert_marked("日本\r\n|語");
    }

    #[test]
    fn leaves_the_text_alone() {
        let mut e = Editor::new("a\nb|").with_new_line(Eol::Lf);
        e.run("chgmod_eol_crlf");
        e.assert_marked("a\nb|");
        assert!(!e.model().is_modified());
    }
}
