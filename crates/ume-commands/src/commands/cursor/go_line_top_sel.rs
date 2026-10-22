//! `F_GOLINETOP_SEL`: moves the caret like `go_line_top` and extends the selection.
//!
//! Sakura: `CViewCommander::Command_GOLINETOP` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoLineTopSel;

impl Command for GoLineTopSel {
    const ID: &'static str = "go_line_top_sel";
    const SAKURA: &'static str = "F_GOLINETOP_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        cx.move_line_top_flags(true, Arg::first_int(args))
    }
}

register_command!(GoLineTopSel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("go_line_top_sel", "  ab|c", "  |ab[c");
        check("go_line_top_sel", "  |abc", "|  [abc");
        check("go_line_top_sel", "ab|c", "|ab[c");
    }

    #[test]
    fn flag_1_goes_to_the_line_start_whatever_the_caret_position() {
        use crate::command::Arg;
        use crate::testing::Editor;
        let mut e = Editor::new("  |abc");
        e.run_with("go_line_top_sel", &[Arg::Int(1)]);
        e.assert_marked("|  [abc");
        let mut e = Editor::new("  ab|c");
        e.run_with("go_line_top_sel", &[Arg::Int(1)]);
        e.assert_marked("|  ab[c");
    }
}
