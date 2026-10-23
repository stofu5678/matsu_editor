//! `F_GOLINETOP`: moves the caret to the first char that is not indentation, or to the start of the row if it is already there.
//!
//! Sakura: `CViewCommander::Command_GOLINETOP` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.
//!
//! Sakura passes the selection lock (`m_bSelectingLock`) as `bSelect` here: while the lock is on
//! (after `F_BEGIN_SEL` or `F_BEGIN_BOX`) the move extends the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct GoLineTop;

impl Command for GoLineTop {
    const ID: &'static str = "go_line_top";
    const SAKURA: &'static str = "F_GOLINETOP";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        cx.move_line_top_flags(false, Arg::first_int(args))
    }
}

register_command!(GoLineTop);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("go_line_top", "  ab|c", "  |abc");
        check("go_line_top", "  |abc", "|  abc");
        check("go_line_top", "|  abc", "  |abc");
        check("go_line_top", "x\n  ab|c", "x\n  |abc");
        check("go_line_top", "x\r\n\tab|c", "x\r\n\t|abc");
        check("go_line_top", "\u{3000}ab|c", "\u{3000}|abc");
        check("go_line_top", "  | ", "|   ");
        check("go_line_top", "|", "|");
        check("go_line_top", "abc|", "|abc");
        check("go_line_top", "[ab|c", "|abc");
    }

    #[test]
    fn the_macro_flags() {
        use crate::command::Arg;
        use crate::testing::Editor;
        // Sakura: `Command_GOLINETOP`, flag 1 = go to the start whatever the caret position
        // (a caret at the first char does not toggle), 4 = select, 8 = line unit.
        let run = |before: &str, flags: i64, after: &str| {
            let mut e = Editor::new(before);
            e.run_with("go_line_top", &[Arg::Int(flags)]);
            e.assert_marked(after);
        };
        run("  ab|c", 1, "|  abc");
        run("  |abc", 1, "|  abc");
        run("|  abc", 1, "|  abc");
        run("x\n\tab|c", 1, "x\n|\tabc");
        run("  ab|c", 0, "  |abc");
        run("  ab|c", 4, "  |ab[c");
        run("  ab|c", 5, "|  ab[c");
        run("  |abc", 4, "|  [abc");
        run("  ab|c", 8, "  |abc");
        run("  ab|c", 9, "|  abc");
        // bits that are not defined are ignored
        run("  ab|c", 2, "  |abc");
        // a missing argument is 0
        let mut e = Editor::new("  ab|c");
        e.run("go_line_top");
        e.assert_marked("  |abc");
    }
}
