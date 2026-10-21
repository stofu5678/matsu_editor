//! `F_GOLINEEND_SEL`: moves the caret to the end of the row and extends the selection.
//!
//! Sakura: `CViewCommander::Command_GOLINEEND` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoLineEndSel;

impl Command for GoLineEndSel {
    const ID: &'static str = "go_line_end_sel";
    const SAKURA: &'static str = "F_GOLINEEND_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        cx.move_line_end_flags(true, Arg::first_int(args))
    }
}

register_command!(GoLineEndSel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("go_line_end_sel", "ab|cd", "ab[cd|");
        check("go_line_end_sel", "ab|cd\r\nef", "ab[cd|\r\nef");
    }

    #[test]
    fn the_flag_8_is_accepted() {
        use crate::command::Arg;
        use crate::testing::Editor;
        let mut e = Editor::new("a|bc\nd");
        e.run_with("go_line_end_sel", &[Arg::Int(8)]);
        e.assert_marked("a[bc|\nd");
    }
}
