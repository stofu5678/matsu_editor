//! `F_BOOKMARK_SET`: sets or clears the bookmark of the line of the caret (Ctrl+F2). With a
//! selection over more than one row, every line from the one the selection starts in to the one
//! it ends in (an end at the start of a line includes that line) is turned over, line by line.
//!
//! Sakura: `CViewCommander::Command_BOOKMARK_SET` (`sakura_core/cmd/CViewCommander_Bookmark.cpp`);
//! the help is `help/sakura/res/HLP000205.html`. Bookmarks are not part of undo, and are not an
//! edit of the text (the document stays unmodified).

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct BookmarkSet;

impl Command for BookmarkSet {
    const ID: &'static str = "bookmark_set";
    const SAKURA: &'static str = "F_BOOKMARK_SET";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.toggle_bookmarks()
    }
}

register_command!(BookmarkSet);

#[cfg(test)]
mod tests {
    use ume_core::buffer::{Document, Pos};
    use ume_core::cursor::Cursor;
    use ume_core::model::DocumentModel;

    use crate::context::{CommandContext, TextChange};
    use crate::layout::{Layout, PlainLayout};
    use crate::testing::Editor;

    fn marked(e: &Editor) -> Vec<usize> {
        e.model().bookmarks().iter().collect()
    }

    #[test]
    fn toggles_the_line_of_the_caret() {
        let mut e = Editor::new("ab\n日本|語\r\ncd");
        e.run("bookmark_set");
        assert_eq!(marked(&e), [1]);
        assert!(e.changes().bookmarks_changed);
        e.assert_marked("ab\n日本|語\r\ncd");
        e.run("bookmark_set");
        assert!(marked(&e).is_empty());
    }

    #[test]
    fn marks_the_last_line_and_ignores_the_row_after_a_final_line_ending() {
        let mut e = Editor::new("a\nb|");
        e.run("bookmark_set");
        assert_eq!(marked(&e), [1]);
        // the empty row after the final line break is not a line
        let mut e = Editor::new("a\nb\n|");
        e.run("bookmark_set");
        assert!(marked(&e).is_empty());
        // and an empty document has no line
        let mut e = Editor::new("|");
        e.run("bookmark_set");
        assert!(marked(&e).is_empty());
    }

    #[test]
    fn a_selection_turns_over_each_line_from_the_first_to_the_last() {
        // line 2 is marked first; selecting lines 1 to 3 turns 1 and 3 on and 2 off
        let mut e = Editor::new("ab\ncd\nef|\ngh\nij");
        e.run("bookmark_set");
        e.run("go_file_top")
            .run("cursor_down")
            .run("cursor_right")
            .run("cursor_down_sel")
            .run("cursor_down_sel");
        assert!(e.cursor().has_selection());
        e.run("bookmark_set");
        assert_eq!(marked(&e), [1, 3]);
        // the selection stays
        assert!(e.cursor().has_selection());
    }

    #[test]
    fn a_selection_that_ends_at_the_start_of_a_line_includes_that_line() {
        let mut e = Editor::new("[a\nb\n|c\nd");
        e.run("bookmark_set");
        assert_eq!(marked(&e), [0, 1, 2]);
        let mut e = Editor::new("a|\nb\nc[\nd");
        e.run("bookmark_set");
        assert_eq!(marked(&e), [0, 1, 2]);
    }

    #[test]
    fn a_selection_inside_one_row_marks_the_line_of_the_caret() {
        let mut e = Editor::new("ab\nc[de|f\ngh");
        e.run("bookmark_set");
        assert_eq!(marked(&e), [1]);
    }

    #[test]
    fn a_box_selection_marks_the_lines_of_its_corners() {
        let mut e = Editor::new("|abc\ndef\nghi\njkl");
        e.run("begin_box").run("cursor_right").run("cursor_down");
        e.run("cursor_down");
        e.run("bookmark_set");
        assert_eq!(marked(&e), [0, 1, 2]);
        assert!(e.cursor().is_box());
        // a box that covers nothing marks the line of the caret
        let mut e = Editor::new("abc\nd|ef");
        e.run("begin_box").run("bookmark_set");
        assert_eq!(marked(&e), [1]);
    }

    #[test]
    fn a_bookmark_follows_its_line() {
        let mut e = Editor::new("a\nb|\nc");
        e.run("bookmark_set").run("go_file_top");
        e.run("insert_date"); // an edit on line 0 only: nothing moves
        assert_eq!(marked(&e), [1]);
        e.run("go_file_top").run("delete_line");
        assert_eq!(marked(&e), [0]);
        e.run("delete_line"); // the bookmarked line goes, and its bookmark
        assert!(marked(&e).is_empty());
    }

    #[test]
    fn undo_does_not_bring_back_a_bookmark_of_a_deleted_line() {
        let mut e = Editor::new("a\nb|\nc");
        e.run("bookmark_set").run("delete_line");
        assert!(marked(&e).is_empty());
        e.run("undo");
        e.assert_marked("a\nb|\nc");
        assert!(marked(&e).is_empty());
    }

    #[test]
    fn crlf_and_lone_cr_lines_count_once() {
        let mut e = Editor::new("a\r\nb\rc\nd|");
        e.run("bookmark_set").run("go_file_top").run("bookmark_set");
        assert_eq!(marked(&e), [0, 3]);
    }

    /// A layout that wraps every line after three ASCII chars (rows of "abc", "def", ...).
    struct Wrap3(PlainLayout);

    type R<T> = ume_core::buffer::Result<T>;

    impl Layout for Wrap3 {
        fn edited(&mut self, doc: &Document, change: &TextChange) {
            self.0.edited(doc, change);
        }
        fn reset(&mut self, doc: &Document) {
            self.0.reset(doc);
        }
        fn display_col(&self, _: &Document, pos: Pos) -> R<usize> {
            Ok(pos.col % 3)
        }
        fn row_start(&self, _: &Document, pos: Pos) -> R<Pos> {
            Ok(Pos::new(pos.line, pos.col / 3 * 3))
        }
        fn row_end(&self, doc: &Document, pos: Pos) -> R<Pos> {
            self.0.row_end(doc, pos)
        }
        fn next_row_start(&self, doc: &Document, pos: Pos) -> R<Option<Pos>> {
            self.0.next_row_start(doc, pos)
        }
        fn pos_at_col(&self, doc: &Document, row_pos: Pos, col: usize) -> R<Pos> {
            self.0.pos_at_col(doc, row_pos, col)
        }
        fn move_rows(&self, doc: &Document, pos: Pos, rows: isize, goal: usize) -> R<Pos> {
            self.0.move_rows(doc, pos, rows, goal)
        }
        fn tab_width(&self) -> usize {
            4
        }
    }

    fn run_wrapped(text: &str, anchor: Pos, caret: Pos) -> Vec<usize> {
        let mut model = DocumentModel::new(Document::from_utf8(text.as_bytes().to_vec()).unwrap());
        let mut cursor = Cursor::default();
        cursor.select(anchor, caret);
        let mut layout = Wrap3(PlainLayout::default());
        let mut cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None);
        cx.toggle_bookmarks().unwrap();
        model.bookmarks().iter().collect()
    }

    #[test]
    fn a_selection_over_two_rows_of_one_wrapped_line_marks_that_line_once() {
        // rows "abc" "def" of line 0; the selection starts and ends in line 0 on different rows
        assert_eq!(
            run_wrapped("abcdef\nxyz", Pos::new(0, 1), Pos::new(0, 4)),
            [0]
        );
        // on one row the caret decides, as for no selection
        assert_eq!(
            run_wrapped("abcdef\nxyz", Pos::new(0, 0), Pos::new(0, 2)),
            [0]
        );
        assert_eq!(
            run_wrapped("abcdef\nxyz", Pos::new(1, 0), Pos::new(1, 2)),
            [1]
        );
        // two rows of line 0 to a row of line 1
        assert_eq!(
            run_wrapped("abcdef\nxyz\nuvw", Pos::new(0, 4), Pos::new(2, 1)),
            [0, 1, 2]
        );
    }
}
