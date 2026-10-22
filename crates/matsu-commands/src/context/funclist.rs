// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/cmd/CViewCommander_Bookmark.cpp, sakura_core/docplus/CFuncListManager.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2014, the authors named in the original files (genta, zenryaku, Moca,
//     aroka, hor, YAZAKI, little YOSHI, ryoji, syat and others)
//   Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (a UTF-16 buffer
// walked with `CDocLineMgr`, the results put in `CFuncInfoArr`, a Windows dialog) was rewritten in
// Rust on the lines of `LineSource`, the positions in the layout are left to the window, and the
// tests are new.
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! Function list marks (Sakura's 関数リストマーク): the lines of the functions that the last outline
//! analysis found, and the jumps to the next and the previous one.
//!
//! Sakura: `Command_FUNCLIST_NEXT` and `Command_FUNCLIST_PREV`
//! (`sakura_core/cmd/CViewCommander_Bookmark.cpp`), `CFuncListManager`
//! (`sakura_core/docplus/CFuncListManager.cpp`), and `CDlgFuncList::SetDocLineFuncList`
//! (`sakura_core/outline/CDlgFuncList.cpp`), which sets the marks when the outline is made.

use matsu_core::buffer::Pos;
use matsu_core::search::Direction;

use super::CommandContext;
use crate::command::Result;

/// How a jump to a function list mark ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FuncListJump {
    /// The caret is on the marked line now. `wrapped`: the search went on from the other end of
    /// the file (`STR_ERR_SRNEXT1`, `STR_ERR_SRPREV1` in the status bar).
    Found { wrapped: bool },
    /// There is no mark in that direction.
    NotFound,
}

impl CommandContext<'_> {
    /// `SetDocLineFuncList`: the marks are the lines (0 origin) of the items of the analysis; a
    /// line that does not exist gets none. The marks that were there go.
    pub fn set_funclist_marks(&mut self, lines: impl IntoIterator<Item = usize>) {
        let count = self.doc().line_count();
        let marks = self.model.funclist_marks_mut();
        marks.clear();
        for line in lines.into_iter().filter(|&l| l < count) {
            marks.set(line, true);
        }
    }

    /// `F_FUNCLIST_NEXT` and `F_FUNCLIST_PREV`: moves the caret to the start of the next (or
    /// previous) marked line, as a plain move (it extends the selection while the selection is
    /// locked). With `wrap` (Sakura's option 先頭（末尾）から再検索, `m_bSearchAll`) a search that
    /// finds nothing goes on from the other end of the file, and may end on the line of the caret.
    pub fn jump_to_funclist_mark(
        &mut self,
        direction: Direction,
        wrap: bool,
    ) -> Result<FuncListJump> {
        let line = self.caret().line;
        let marks = self.model.funclist_marks();
        let (found, wrapped) = match direction {
            Direction::Forward => match marks.next_after(line) {
                Some(l) => (Some(l), false),
                None if wrap => (marks.first(), true),
                None => (None, false),
            },
            Direction::Backward => match marks.prev_before(line) {
                Some(l) => (Some(l), false),
                None if wrap => (marks.last(), true),
                None => (None, false),
            },
        };
        let Some(target) = found else {
            return Ok(FuncListJump::NotFound);
        };
        let extend = self.extends(false);
        self.move_caret(Pos::new(target, 0), extend)?;
        Ok(FuncListJump::Found { wrapped })
    }
}

#[cfg(test)]
mod tests {
    use matsu_core::search::Direction;

    use super::FuncListJump;
    use crate::testing::Editor;

    fn marked(e: &Editor) -> Vec<usize> {
        e.model().funclist_marks().iter().collect()
    }

    fn jump(e: &mut Editor, direction: Direction, wrap: bool) -> FuncListJump {
        e.with_context(|cx| cx.jump_to_funclist_mark(direction, wrap).unwrap())
    }

    #[test]
    fn marks_replace_the_old_ones_and_skip_missing_lines() {
        let mut e = Editor::new("a\nb\nc|");
        e.with_context(|cx| cx.set_funclist_marks([0, 2, 9]));
        assert_eq!(marked(&e), [0, 2]);
        e.with_context(|cx| cx.set_funclist_marks([1]));
        assert_eq!(marked(&e), [1]);
    }

    #[test]
    fn jumps_to_the_next_and_the_previous_mark() {
        let mut e = Editor::new("a\nb\n  c\nd\ne|");
        e.with_context(|cx| cx.set_funclist_marks([1, 2, 4]));
        e.run("go_file_top");
        assert_eq!(
            jump(&mut e, Direction::Forward, false),
            FuncListJump::Found { wrapped: false }
        );
        e.assert_marked("a\n|b\n  c\nd\ne");
        // a move to the start of the line, not to the text
        assert_eq!(
            jump(&mut e, Direction::Forward, false),
            FuncListJump::Found { wrapped: false }
        );
        e.assert_marked("a\nb\n|  c\nd\ne");
        assert_eq!(
            jump(&mut e, Direction::Backward, false),
            FuncListJump::Found { wrapped: false }
        );
        e.assert_marked("a\n|b\n  c\nd\ne");
    }

    #[test]
    fn no_mark_in_that_direction_ends_without_a_move_unless_it_wraps() {
        let mut e = Editor::new("a\nb\nc|");
        e.with_context(|cx| cx.set_funclist_marks([1]));
        e.run("go_file_top");
        assert_eq!(
            jump(&mut e, Direction::Backward, false),
            FuncListJump::NotFound
        );
        e.assert_marked("|a\nb\nc");
        // wrapping goes on from the other end
        assert_eq!(
            jump(&mut e, Direction::Backward, true),
            FuncListJump::Found { wrapped: true }
        );
        e.assert_marked("a\n|b\nc");
        // from the marked line itself the search ends on it again
        assert_eq!(
            jump(&mut e, Direction::Forward, true),
            FuncListJump::Found { wrapped: true }
        );
        e.assert_marked("a\n|b\nc");
        // no marks at all
        e.with_context(|cx| cx.set_funclist_marks([]));
        assert_eq!(
            jump(&mut e, Direction::Forward, true),
            FuncListJump::NotFound
        );
    }

    #[test]
    fn marks_follow_the_lines_when_text_is_edited() {
        // the marks are on the line objects in Sakura, so they move with their lines
        let mut e = Editor::new("a\nb\nc|");
        e.with_context(|cx| cx.set_funclist_marks([2]));
        e.run("go_file_top");
        e.with_context(|cx| cx.insert_text("\n").unwrap());
        assert_eq!(marked(&e), [3]);
        // and a deleted line takes its mark along
        e.run("go_file_end").run("delete_line");
        assert!(marked(&e).is_empty());
    }
}
