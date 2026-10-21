//! `F_CHGMOD_EOL_LS`: Enter inserts LS in this document (入力改行コード指定(LS)).
//!
//! Sakura: `CEditWnd::OnNotify` (`NM_RCLICK` on the line ending of the status bar), in
//! `sakura_core/window/CEditWnd.cpp`, offers it only when `m_bEnableExtEol` is on; it runs
//! `F_CHGMOD_EOL` with `EEolType` 5 (`sakura_core/basis/CEol.h`); see `chgmod_eol`.

use ume_core::buffer::Eol;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct ChgmodEolLs;

impl Command for ChgmodEolLs {
    const ID: &'static str = "chgmod_eol_ls";
    const SAKURA: &'static str = "F_CHGMOD_EOL_LS";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.set_new_line(Eol::Ls);
        Ok(())
    }
}

register_command!(ChgmodEolLs);

#[cfg(test)]
mod tests {
    use ume_core::buffer::Eol;

    use crate::testing::Editor;

    #[test]
    fn sets_the_line_ending_of_enter() {
        let mut e = Editor::new("日本|語").with_new_line(Eol::Lf);
        e.run("chgmod_eol_ls");
        assert_eq!(e.model().new_line(), Eol::Ls);
    }

    #[test]
    fn leaves_the_text_alone() {
        let mut e = Editor::new("a\nb|").with_new_line(Eol::Lf);
        e.run("chgmod_eol_ls");
        e.assert_marked("a\nb|");
        assert!(!e.model().is_modified());
    }
}
