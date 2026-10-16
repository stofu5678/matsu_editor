//! `F_CHGMOD_EOL_LF`: Enter inserts LF in this document (入力改行コード指定(LF)).
//!
//! Sakura: `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp` runs
//! `F_CHGMOD_EOL` with `EEolType::line_feed`; see `chgmod_eol`.

use ume_core::buffer::Eol;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct ChgmodEolLf;

impl Command for ChgmodEolLf {
    const ID: &'static str = "chgmod_eol_lf";
    const SAKURA: &'static str = "F_CHGMOD_EOL_LF";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.set_new_line(Eol::Lf);
        Ok(())
    }
}

register_command!(ChgmodEolLf);

#[cfg(test)]
mod tests {
    use ume_core::buffer::Eol;

    use crate::command::Arg;
    use crate::testing::Editor;

    #[test]
    fn sets_the_line_ending_of_enter() {
        let mut e = Editor::new("日本|語").with_new_line(Eol::CrLf);
        e.run("chgmod_eol_lf");
        assert_eq!(e.model().new_line(), Eol::Lf);
        e.run_with("input_char", &[Arg::Char('\n')]);
        e.assert_marked("日本\n|語");
    }

    #[test]
    fn leaves_the_text_alone() {
        let mut e = Editor::new("a\r\nb|").with_new_line(Eol::CrLf);
        e.run("chgmod_eol_lf");
        e.assert_marked("a\r\nb|");
        assert!(!e.model().is_modified());
    }
}
