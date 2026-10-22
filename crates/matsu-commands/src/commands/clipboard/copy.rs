//! `F_COPY`. Sakura: `CViewCommander::Command_COPY` in `sakura_core/cmd/CViewCommander_Clipboard.cpp`. Copies the selection. Without a selection it copies the row and marks it as a line copy, so that a paste puts it at the start of a row
//! (`bEnableNoSelectCopy` and `bEnableLineModePaste` are on by default). The selection stays
//! (the setting that drops it after a copy is off by default), but the selection lock goes
//! (`m_bSelectingLock = false` in `Command_COPY`), so the next plain move drops the selection.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct Copy;

impl Command for Copy {
    const ID: &'static str = "copy";
    const SAKURA: &'static str = "F_COPY";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.copy(None, false)
    }
}

register_command!(Copy);

#[cfg(test)]
mod tests {
    use crate::testing::Editor;

    const TEXT: &str = "abcdefghij\nABCDEFGHIJ\n0123456789\nshort\n\nlast\n";

    fn box_copy() -> Editor {
        let mut e = Editor::new(&format!("|{TEXT}"));
        e.run("begin_box")
            .run("cursor_right_box")
            .run("cursor_right_box");
        e.run("cursor_down_box").run("copy");
        e
    }

    #[test]
    fn releases_the_selection_lock_but_keeps_the_selection() {
        let e = box_copy();
        assert!(!e.cursor().selecting_lock());
        assert!(e.cursor().has_selection());
        let mut e = Editor::new("a|bc");
        e.run("begin_sel").run("cursor_right").run("copy");
        assert!(!e.cursor().selecting_lock());
        e.run("cursor_right");
        assert!(!e.cursor().has_selection());
    }

    #[test]
    fn a_box_copy_is_pasted_as_a_box_after_a_plain_move() {
        // the case `box-copy-paste-text` of the Sakura oracle
        let mut e = box_copy();
        assert_eq!(e.clipboard().map(|c| c.text.as_str()), Some("ab\r\nAB\r\n"));
        e.run("cursor_down").run("cursor_down");
        assert!(!e.cursor().has_selection());
        e.run("paste");
        assert_eq!(
            e.marked().replace(['|', '['], ""),
            "abcdefghij\nABCDEFGHIJ\n0123456789\nshabort\n  AB\nlast\n"
        );
    }

    #[test]
    fn a_box_copy_is_added_at_the_end_by_pastebox_after_a_plain_move() {
        // the case `box-copy-pastebox`: nothing is removed
        let mut e = box_copy();
        e.run("go_file_end").run("pastebox");
        let text = e.marked().replace(['|', '['], "");
        assert!(text.starts_with(TEXT), "{text:?}");
        assert_eq!(text.matches("ab").count(), 2, "{text:?}");
    }
}
