//! `F_DIFF_RESET`: removes the DIFF marks (差分の全解除).
//!
//! Sakura: `CViewCommander::Command_Diff_Reset` (`sakura_core/cmd/CViewCommander_Diff.cpp`), which
//! calls `CDiffLineMgr::ResetAllDiffMark` (`sakura_core/docplus/CDiffManager.cpp`).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct DiffReset;

impl Command for DiffReset {
    const ID: &'static str = "diff_reset";
    const SAKURA: &'static str = "F_DIFF_RESET";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.clear_diff_marks();
        Ok(())
    }
}

register_command!(DiffReset);

#[cfg(test)]
mod tests {
    use ume_core::diff::{DiffMark, Hunk, HunkKind};
    use ume_core::search::Direction;

    use crate::context::DiffJump;
    use crate::testing::Editor;

    fn marked_editor() -> Editor {
        let mut e = Editor::new("a\n日本\r\nc\nd|");
        let hunks = [
            Hunk {
                kind: HunkKind::Change,
                old: 1..2,
                new: 1..2,
            },
            Hunk {
                kind: HunkKind::Add,
                old: 3..3,
                new: 3..4,
            },
        ];
        assert!(e.with_context(|cx| cx.set_diff_marks(&hunks, false)));
        e
    }

    #[test]
    fn removes_every_mark() {
        let mut e = marked_editor();
        assert_eq!(e.model().diff_marks().get(1), Some(DiffMark::Change));
        assert_eq!(e.model().diff_marks().get(3), Some(DiffMark::Append));
        e.run("diff_reset");
        assert!(e.model().diff_marks().is_empty());
        assert!(!e.model().diff_marks().in_use());
        assert!(e.changes().bookmarks_changed);
        // the text and the caret stay
        e.assert_marked("a\n日本\r\nc\nd|");
    }

    #[test]
    fn is_not_an_edit() {
        let mut e = marked_editor();
        e.run("diff_reset");
        assert!(!e.changes().text_changed());
        assert!(!e.model().is_modified());
    }

    #[test]
    fn a_new_comparison_replaces_the_marks() {
        let mut e = marked_editor();
        let hunks = [Hunk {
            kind: HunkKind::Delete,
            old: 0..1,
            new: 0..0,
        }];
        // the old lines are not in the document: the gap is before its line 0
        assert!(e.with_context(|cx| cx.set_diff_marks(&hunks, false)));
        let marks: Vec<_> = e.model().diff_marks().iter().collect();
        assert_eq!(marks, [(0, DiffMark::Delete)]);
        // no hunks: nothing is marked and no DIFF is in use
        assert!(!e.with_context(|cx| cx.set_diff_marks(&[], false)));
        assert!(e.model().diff_marks().is_empty());
    }

    #[test]
    fn the_jumps_stop_at_the_marked_lines() {
        let mut e = marked_editor();
        let forward = |e: &mut Editor, wrap| {
            e.with_context(|cx| cx.jump_to_diff(Direction::Forward, wrap).unwrap())
        };
        let backward = |e: &mut Editor, wrap| {
            e.with_context(|cx| cx.jump_to_diff(Direction::Backward, wrap).unwrap())
        };
        // the caret is on the last marked line
        assert_eq!(forward(&mut e, false), DiffJump::NotFound);
        assert_eq!(backward(&mut e, false), DiffJump::Found { wrapped: false });
        e.assert_marked("a\n|日本\r\nc\nd");
        assert_eq!(backward(&mut e, false), DiffJump::NotFound);
        // with the option 先頭（末尾）から再検索 the search goes on from the other end
        assert_eq!(backward(&mut e, true), DiffJump::Found { wrapped: true });
        e.assert_marked("a\n日本\r\nc\n|d");
        assert_eq!(forward(&mut e, true), DiffJump::Found { wrapped: true });
        e.assert_marked("a\n|日本\r\nc\nd");
    }

    #[test]
    fn the_marks_follow_the_edits() {
        let mut e = marked_editor();
        let lines =
            |e: &Editor| -> Vec<usize> { e.model().diff_marks().iter().map(|(l, _)| l).collect() };
        e.run("go_file_top").run("delete_line");
        assert_eq!(lines(&e), [0, 2]);
        e.run("undo");
        assert_eq!(lines(&e), [1, 3]);
    }
}
