//! `F_CHGMOD_EOL_PS`: Enter inserts PS in this document (入力改行コード指定(PS)).
//!
//! Sakura: `CEditWnd::OnNotify` (`NM_RCLICK` on the line ending of the status bar), in
//! `sakura_core/window/CEditWnd.cpp`, offers it only when `m_bEnableExtEol` is on; it runs
//! `F_CHGMOD_EOL` with `EEolType` 6 (`sakura_core/basis/CEol.h`); see `chgmod_eol`.

use matsu_core::buffer::Eol;

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct ChgmodEolPs;

impl Command for ChgmodEolPs {
    const ID: &'static str = "chgmod_eol_ps";
    const SAKURA: &'static str = "F_CHGMOD_EOL_PS";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.set_new_line(Eol::Ps);
        Ok(())
    }
}

register_command!(ChgmodEolPs);

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Eol;

    use crate::testing::Editor;

    #[test]
    fn sets_the_line_ending_of_enter() {
        let mut e = Editor::new("日本|語").with_new_line(Eol::Lf);
        e.run("chgmod_eol_ps");
        assert_eq!(e.model().new_line(), Eol::Ps);
    }

    #[test]
    fn leaves_the_text_alone() {
        let mut e = Editor::new("a\nb|").with_new_line(Eol::Lf);
        e.run("chgmod_eol_ps");
        e.assert_marked("a\nb|");
        assert!(!e.model().is_modified());
    }
}
