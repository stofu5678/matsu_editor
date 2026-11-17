//! `matsu-commands`' [`Layout`] trait over `matsu-layout`'s [`DocumentLayout`].
//!
//! The trait stays in `matsu-commands` (it has no dependency on `matsu-layout`), and this adapter is
//! the "implementation in `matsu-ui`" that its documentation promises. Columns are the layout's
//! columns counted from the left edge of the screen, so the indent of a wrapped row is included,
//! and the goal column of the cursor means the same thing in every method.
//!
//! Both count in `X` ([`matsu_layout::X`], ADR-0034), so nothing is converted here.
//!
//! The context calls [`Layout::edited`] right after each edit and [`Layout::reset`] after an undo
//! or a redo, so the layout matches the document all through a command (ADR-0013 section 4).

use std::time::Duration;

use matsu_commands::context::TextChange;
use matsu_commands::layout::Layout;
use matsu_core::buffer::{Document, Edit, Pos, Result};
use matsu_layout::{DocumentLayout, X};

/// How long one frame may spend laying out the rest of a document.
pub const LAYOUT_BUDGET: Duration = Duration::from_millis(4);

/// A borrowed [`DocumentLayout`] as a command's [`Layout`].
pub struct RowLayout<'a>(pub &'a mut DocumentLayout);

impl Layout for RowLayout<'_> {
    fn tab_width(&self) -> usize {
        self.0.config().effective_tab_width() as usize
    }

    fn tab_x(&self) -> X {
        self.0.config().tab_x()
    }

    fn column_x(&self) -> X {
        self.0.config().width.to_x(1)
    }

    fn max_line_x(&self) -> X {
        self.0.config().max_x()
    }

    fn char_columns(&self, c: char) -> usize {
        self.0.config().width.columns(c) as usize
    }

    fn char_x(&self, c: char) -> X {
        self.0.config().width.width_of(c)
    }

    fn make_exact(&mut self, doc: &Document, line: usize) {
        let row = self.0.first_row_of_line(line);
        let _ = self.0.layout_rows(doc, row, 1);
    }

    fn row_index(&self, doc: &Document, pos: Pos) -> Result<usize> {
        Ok(self.0.pos_to_row(doc, pos)?.row)
    }

    fn row_start_at(&self, doc: &Document, row: usize) -> Result<Option<Pos>> {
        if row >= self.0.row_count() {
            return Ok(None);
        }
        Ok(Some(self.0.row(doc, row)?.start))
    }

    fn edited(&mut self, doc: &Document, change: &TextChange) {
        // `DocumentLayout::edited` follows one edit made right before `doc`, which is what the
        // context promises. If the index still disagrees, start again from the document.
        let edit = Edit {
            start: change.start,
            end: change.new_end,
        };
        if self.0.edited(doc, &edit).is_err() {
            self.reset(doc);
        }
    }

    fn reset(&mut self, doc: &Document) {
        // The first screenful is laid out at once and the rest in the following frames.
        if self.0.resync(doc).is_ok() {
            let _ = self.0.layout_for(doc, LAYOUT_BUDGET);
        }
    }

    fn display_x(&self, doc: &Document, pos: Pos) -> Result<X> {
        Ok(self.0.pos_to_row(doc, pos)?.x)
    }

    fn row_start(&self, doc: &Document, pos: Pos) -> Result<Pos> {
        let row = self.0.pos_to_row(doc, pos)?.row;
        Ok(self.0.row(doc, row)?.start)
    }

    fn row_end(&self, doc: &Document, pos: Pos) -> Result<Pos> {
        let row = self.0.pos_to_row(doc, pos)?.row;
        Ok(self.0.row(doc, row)?.end)
    }

    fn next_row_start(&self, doc: &Document, pos: Pos) -> Result<Option<Pos>> {
        let next = self.0.pos_to_row(doc, pos)?.row + 1;
        if next >= self.0.row_count() {
            return Ok(None);
        }
        Ok(Some(self.0.row(doc, next)?.start))
    }

    fn pos_at_x(&self, doc: &Document, row_pos: Pos, x: X) -> Result<Pos> {
        let row = self.0.pos_to_row(doc, row_pos)?.row;
        Ok(self.0.row_to_pos(doc, row, x)?.pos)
    }

    fn move_rows(&self, doc: &Document, pos: Pos, rows: isize, goal: X) -> Result<Pos> {
        Ok(self.0.move_rows(doc, pos, rows, goal)?.pos)
    }
}

#[cfg(test)]
mod tests {
    use matsu_commands::command::Arg;
    use matsu_commands::context::CommandContext;
    use matsu_commands::layout::PlainLayout;
    use matsu_commands::registry::Registry;
    use matsu_core::cursor::Cursor;
    use matsu_core::model::DocumentModel;
    use matsu_layout::{LayoutConfig, WidthRule, Widths, Wrap};

    use super::*;

    fn doc(s: &str) -> Document {
        Document::from_utf8(s.as_bytes().to_vec()).unwrap()
    }

    fn laid_out(d: &Document, cfg: LayoutConfig) -> DocumentLayout {
        DocumentLayout::laid_out(d, cfg).unwrap()
    }

    #[test]
    fn the_tab_width_is_the_effective_one_of_the_layout() {
        let d = doc("a\n");
        let mut layout = laid_out(
            &d,
            LayoutConfig {
                tab_width: 8,
                ..LayoutConfig::default()
            },
        );
        assert_eq!(RowLayout(&mut layout).tab_width(), 8);
        // a tab width that is not less than the wrap column falls back to 4
        let mut layout = laid_out(
            &d,
            LayoutConfig {
                tab_width: 8,
                wrap: Wrap::Columns(8),
                ..LayoutConfig::default()
            },
        );
        assert_eq!(RowLayout(&mut layout).tab_width(), 4);
    }

    /// Without wrapping the real layout must agree with the simple layout that the commands'
    /// own tests use (tab stops every 4 columns, full-width chars take 2).
    #[test]
    fn agrees_with_the_plain_layout_without_wrapping() {
        for per_column in PER_COLUMN {
            agrees_with_the_plain_layout(per_column);
        }
    }

    /// The column sizes that the commands must give the same result with (ADR-0034).
    const PER_COLUMN: [u32; 2] = [1, 64];

    fn scaled(per_column: u32) -> LayoutConfig {
        LayoutConfig {
            width: Widths::grid_scaled(WidthRule::default(), per_column),
            ..LayoutConfig::default()
        }
    }

    fn agrees_with_the_plain_layout(per_column: u32) {
        let d = doc("a\tb日c\r\n\r\nxyz\n");
        let mut layout = laid_out(&d, scaled(per_column));
        let real = RowLayout(&mut layout);
        let plain = PlainLayout::scaled(per_column);
        let x = |columns: u32| X::from_raw(columns * per_column);
        let positions = [
            Pos::new(0, 0),
            Pos::new(0, 1),
            Pos::new(0, 2),
            Pos::new(0, 3),
            Pos::new(0, 6),
            Pos::new(0, 7),
            Pos::new(1, 0),
            Pos::new(2, 2),
            Pos::new(3, 0),
        ];
        for pos in positions {
            assert_eq!(
                real.display_x(&d, pos).unwrap(),
                plain.display_x(&d, pos).unwrap(),
                "display_x {pos:?}"
            );
            assert_eq!(
                real.row_start(&d, pos).unwrap(),
                plain.row_start(&d, pos).unwrap(),
                "row_start {pos:?}"
            );
            assert_eq!(
                real.row_end(&d, pos).unwrap(),
                plain.row_end(&d, pos).unwrap(),
                "row_end {pos:?}"
            );
            assert_eq!(
                real.next_row_start(&d, pos).unwrap(),
                plain.next_row_start(&d, pos).unwrap(),
                "next_row_start {pos:?}"
            );
            for rows in [-2, -1, 1, 2, 9] {
                for goal in [0, 1, 4, 5, 6, 40] {
                    assert_eq!(
                        real.move_rows(&d, pos, rows, x(goal)).unwrap(),
                        plain.move_rows(&d, pos, rows, x(goal)).unwrap(),
                        "move_rows {pos:?} {rows} {goal}"
                    );
                }
            }
        }
        for col in [0, 1, 3, 4, 5, 6, 7, 99] {
            assert_eq!(
                real.pos_at_x(&d, Pos::new(0, 0), x(col)).unwrap(),
                plain.pos_at_x(&d, Pos::new(0, 0), x(col)).unwrap(),
                "pos_at_x {col}"
            );
        }
    }

    /// The rightmost `X` of a box or a free caret is the wrap column (`GetMaxLineLayout`).
    #[test]
    fn the_free_caret_goes_no_further_than_the_wrap_column() {
        use matsu_commands::edit_options::{EditOptions, with_options};

        let d = doc("abcdefgh\nabcdefghi\nxy");
        let cfg = LayoutConfig {
            wrap: Wrap::Columns(10),
            ..LayoutConfig::default()
        };
        let mut layout = laid_out(&d, cfg.clone());
        assert_eq!(RowLayout(&mut layout).max_line_x(), X::from_raw(10));
        let options = EditOptions {
            free_cursor: true,
            ..EditOptions::DEFAULT
        };
        let right = |line: usize, times: usize| {
            let mut model = DocumentModel::new(d.clone());
            let mut cursor = Cursor::new(Pos::new(line, d.line_len(line).unwrap().unwrap()));
            let mut layout = laid_out(&d, cfg.clone());
            let mut row_layout = RowLayout(&mut layout);
            let mut cx = CommandContext::new(&mut model, &mut cursor, &mut row_layout, None);
            for _ in 0..times {
                Registry::global()
                    .run("cursor_right", &mut cx, &[])
                    .unwrap();
            }
            drop(cx);
            (cursor.caret(), cursor.free_col())
        };
        with_options(options, || {
            // 8 characters: the caret goes into the gap (columns 8 and 9), then to the next line
            assert_eq!(right(0, 1), (Pos::new(0, 8), Some(9)));
            assert_eq!(right(0, 2), (Pos::new(1, 0), None));
            // 9 characters: one more column would reach the wrap column, so the caret goes to the
            // next line at once (`x_wrap + dx_default < GetMaxLineLayout()` is false)
            assert_eq!(right(1, 1), (Pos::new(2, 0), None));
        });
    }

    /// A box sort of more than 4096 wrapped lines is one big edit, after which the index of the
    /// layout only estimates the rows of those lines: the corners go back to their lines and rows
    /// all the same.
    #[test]
    fn a_box_sort_of_many_wrapped_lines_keeps_the_corners_on_their_lines() {
        use matsu_core::cursor::Cursor;

        const LINES: usize = 5000;
        // each line has two rows: "dddd" and "ab"
        let text: String = (0..LINES).map(|i| format!("{:04}ab\n", i)).collect();
        let cfg = LayoutConfig {
            wrap: Wrap::Columns(4),
            ..scaled(1)
        };
        let mut model = DocumentModel::new(doc(&text));
        let mut layout = laid_out(model.doc(), cfg);
        layout.layout_all(model.doc()).unwrap();
        // from the second row of line 100 to the second row of the last line, columns 0 to 2
        let mut cursor = Cursor::new(Pos::new(100, 4));
        cursor.begin_box(0, false);
        cursor.move_box_to(Pos::new(LINES - 1, 4), 2);
        {
            let mut adapter = RowLayout(&mut layout);
            let mut cx = CommandContext::new(&mut model, &mut cursor, &mut adapter, None);
            Registry::global().run("sort_desc", &mut cx, &[]).unwrap();
            cx.finish();
        }
        let ((a, a_col), (c, c_col)) = cursor.box_corners().expect("the box stays");
        assert_eq!((a, a_col), (Pos::new(100, 4), 0));
        assert_eq!((c, c_col), (Pos::new(LINES - 1, 6), 2));
        // keys "49", "48", ...: line 100 has the first line of the key "49" now
        assert_eq!(model.doc().line(100).unwrap().unwrap().text, "4900ab");
    }

    #[test]
    fn a_wrapped_line_has_several_rows() {
        let d = doc("abcdefghij\nxy");
        let cfg = LayoutConfig {
            wrap: Wrap::Columns(4),
            ..LayoutConfig::default()
        };
        let mut layout = laid_out(&d, cfg);
        let real = RowLayout(&mut layout);
        // abcd / efgh / ij / xy
        let p = Pos::new(0, 6);
        assert_eq!(real.row_start(&d, p).unwrap(), Pos::new(0, 4));
        assert_eq!(real.row_end(&d, p).unwrap(), Pos::new(0, 8));
        assert_eq!(real.display_x(&d, p).unwrap(), X::from_raw(2));
        assert_eq!(
            real.next_row_start(&d, p).unwrap(),
            Some(Pos::new(0, 8)),
            "the next row of the same line"
        );
        let x = X::from_raw;
        assert_eq!(real.move_rows(&d, p, 1, x(1)).unwrap(), Pos::new(0, 9));
        assert_eq!(real.move_rows(&d, p, -1, x(3)).unwrap(), Pos::new(0, 3));
        assert_eq!(real.move_rows(&d, p, 2, x(1)).unwrap(), Pos::new(1, 1));
        assert_eq!(real.next_row_start(&d, Pos::new(1, 1)).unwrap(), None);
    }

    /// Runs commands the way `EditorView` does, and checks after each that the layout kept up
    /// equals a fresh layout of the document.
    fn run_all(text: &str, wrap: Wrap, steps: &[(&str, Vec<Arg>)]) {
        for per_column in PER_COLUMN {
            run_all_scaled(text, wrap, steps, per_column);
        }
    }

    fn run_all_scaled(text: &str, wrap: Wrap, steps: &[(&str, Vec<Arg>)], per_column: u32) {
        let cfg = LayoutConfig {
            wrap,
            ..scaled(per_column)
        };
        let mut model = DocumentModel::new(doc(text));
        let mut cursor = Cursor::default();
        let mut layout = laid_out(model.doc(), cfg.clone());
        for (id, args) in steps {
            {
                let mut adapter = RowLayout(&mut layout);
                let mut cx = CommandContext::new(&mut model, &mut cursor, &mut adapter, None);
                Registry::global().run(id, &mut cx, args).unwrap();
                cx.finish();
            }
            layout.layout_all(model.doc()).unwrap();
            let fresh = laid_out(model.doc(), cfg.clone());
            assert_eq!(layout.row_count(), fresh.row_count(), "after {id}");
            let rows = layout.row_count();
            let a = layout.rows(model.doc(), 0, rows).unwrap();
            let b = fresh.rows(model.doc(), 0, rows).unwrap();
            let key = |r: &matsu_layout::RowInfo| (r.row, r.start, r.end, r.start_x, r.end_x);
            assert_eq!(
                a.iter().map(key).collect::<Vec<_>>(),
                b.iter().map(key).collect::<Vec<_>>(),
                "after {id}"
            );
        }
    }

    fn typed(c: char) -> (&'static str, Vec<Arg>) {
        ("input_char", vec![Arg::Char(c)])
    }

    #[test]
    fn typing_enter_and_undo_keep_the_layout_exact() {
        for wrap in [Wrap::None, Wrap::Columns(5)] {
            run_all(
                "abcdefgh\nxy\n",
                wrap,
                &[
                    typed('Z'),
                    typed('\r'),
                    typed('日'),
                    ("go_file_end", vec![]),
                    typed('q'),
                    ("delete_back", vec![]),
                    ("delete_line", vec![]),
                    ("undo", vec![]),
                    ("undo", vec![]),
                    ("redo", vec![]),
                    ("select_all", vec![]),
                    ("delete", vec![]),
                    ("undo", vec![]),
                ],
            );
        }
    }

    /// A command that edits and then moves vertically sees the rows of the edited text. With the
    /// layout of before the edit the caret would land on the wrong row.
    #[test]
    fn a_vertical_move_after_an_edit_in_one_command_sees_the_new_rows() {
        let cfg = LayoutConfig {
            wrap: Wrap::Columns(4),
            ..LayoutConfig::default()
        };
        let mut model = DocumentModel::new(doc("ab\ncd"));
        let mut cursor = Cursor::default();
        let mut layout = laid_out(model.doc(), cfg);
        {
            let mut adapter = RowLayout(&mut layout);
            let mut cx = CommandContext::new(&mut model, &mut cursor, &mut adapter, None);
            // "ab" becomes "abcdefgh": two rows, so the next row down is still the first line
            cx.insert_text("cdefgh").unwrap();
            cx.move_caret(Pos::new(0, 0), false).unwrap();
            cx.move_vertical(1, false).unwrap();
            cx.finish();
        }
        assert_eq!(cursor.caret(), Pos::new(0, 4));
    }

    /// The paragraph moves count lines of the document, not rows: a wrapped line is one line
    /// (Sakura converts the distance with `LogicToLayout` before `Cursor_UPDOWN`).
    #[test]
    fn paragraph_moves_count_lines_and_not_wrapped_rows() {
        let cfg = LayoutConfig {
            wrap: Wrap::Columns(4),
            ..LayoutConfig::default()
        };
        let mut model = DocumentModel::new(doc("abcdefghij\nkl\n\nmn"));
        let mut cursor = Cursor::default();
        let mut layout = laid_out(model.doc(), cfg);
        let mut run = |id: &str, cursor: &mut Cursor| {
            let mut adapter = RowLayout(&mut layout);
            let mut cx = CommandContext::new(&mut model, cursor, &mut adapter, None);
            Registry::global().run(id, &mut cx, &[]).unwrap();
            cx.finish();
        };
        // the first line has three rows; the target is five rows down
        run("gonextparagraph", &mut cursor);
        assert_eq!(cursor.caret(), Pos::new(3, 0));
        // from the last line, up to the first line of the paragraph above
        run("goprevparagraph", &mut cursor);
        assert_eq!(cursor.caret(), Pos::new(0, 0));
    }

    /// A box selection on wrapped rows: the box moves are by rows of the layout, columns count
    /// from the left edge of the row, and the caret may stand right of the text of a row.
    #[test]
    fn a_box_moves_by_the_rows_of_the_layout() {
        for per_column in PER_COLUMN {
            a_box_moves_by_the_rows(per_column);
        }
    }

    fn a_box_moves_by_the_rows(per_column: u32) {
        let cfg = LayoutConfig {
            wrap: Wrap::Columns(4),
            ..scaled(per_column)
        };
        // rows: "abcd" "ef" | "x" | "ghij"
        let mut model = DocumentModel::new(doc("abcdef\nx\nghij"));
        let mut cursor = Cursor::new(Pos::new(0, 1));
        let mut layout = laid_out(model.doc(), cfg);
        let mut run = |id: &str, cursor: &mut Cursor| {
            let mut adapter = RowLayout(&mut layout);
            let mut cx = CommandContext::new(&mut model, cursor, &mut adapter, None);
            Registry::global().run(id, &mut cx, &[]).unwrap();
            cx.finish();
        };
        // the cursor keeps `X`; the expected corners are in columns
        let per = per_column as usize;
        let corners = |c: &Cursor| {
            (
                c.caret(),
                c.box_anchor_col().map(|x| x / per),
                c.box_caret_col().map(|x| x / per),
            )
        };
        run("begin_box", &mut cursor);
        // down over the wrap: the second row of the line, then the short line
        run("cursor_down_box", &mut cursor);
        assert_eq!(corners(&cursor), (Pos::new(0, 5), Some(1), Some(1)));
        run("cursor_down_box", &mut cursor);
        run("cursor_right_box", &mut cursor);
        run("cursor_right_box", &mut cursor);
        // "x" is one column wide; the caret is in the empty space at column 3
        assert_eq!(corners(&cursor), (Pos::new(1, 1), Some(1), Some(3)));
        // left does not leave the row, and stops at its first column
        for _ in 0..8 {
            run("cursor_left_box", &mut cursor);
        }
        assert_eq!(corners(&cursor), (Pos::new(1, 0), Some(1), Some(0)));
        // down again: the goal column is the last one that was set by a horizontal move
        run("cursor_right_box", &mut cursor);
        run("cursor_down_box", &mut cursor);
        assert_eq!(corners(&cursor), (Pos::new(2, 1), Some(1), Some(1)));
        // the end of the file keeps the column
        run("go_file_top_box", &mut cursor);
        assert_eq!(corners(&cursor), (Pos::new(0, 1), Some(1), Some(1)));
        run("go_file_end_box", &mut cursor);
        assert_eq!(corners(&cursor), (Pos::new(2, 1), Some(1), Some(1)));
    }

    /// On a wrapped row the end of the row is the start of the next one: a box caret does not
    /// rest there, and the empty space exists only after the last row of a line.
    #[test]
    fn a_box_caret_does_not_rest_at_the_end_of_a_wrapped_row() {
        for per_column in PER_COLUMN {
            a_box_caret_stays_on_a_wrapped_row(per_column);
        }
    }

    fn a_box_caret_stays_on_a_wrapped_row(per_column: u32) {
        let cfg = LayoutConfig {
            wrap: Wrap::Columns(4),
            ..scaled(per_column)
        };
        // rows: "abcd" "ef"
        let mut model = DocumentModel::new(doc("abcdef\n"));
        let mut cursor = Cursor::new(Pos::new(0, 1));
        let mut layout = laid_out(model.doc(), cfg);
        let mut run = |id: &str, cursor: &mut Cursor| {
            let mut adapter = RowLayout(&mut layout);
            let mut cx = CommandContext::new(&mut model, cursor, &mut adapter, None);
            Registry::global().run(id, &mut cx, &[]).unwrap();
            cx.finish();
        };
        let per = per_column as usize;
        let at = |c: &Cursor| (c.caret(), c.box_caret_col().map(|x| x / per));
        run("begin_box", &mut cursor);
        run("cursor_right_box", &mut cursor);
        assert_eq!(at(&cursor), (Pos::new(0, 2), Some(2)));
        run("cursor_right_box", &mut cursor);
        assert_eq!(at(&cursor), (Pos::new(0, 3), Some(3)));
        // the last char of the row is as far as it goes; it stays on the row
        run("cursor_right_box", &mut cursor);
        assert_eq!(at(&cursor), (Pos::new(0, 3), Some(3)));
        run("cursor_left_box", &mut cursor);
        assert_eq!(at(&cursor), (Pos::new(0, 2), Some(2)));
        // down to the short row, and on to the end of the last row, where there is empty space
        run("cursor_right_box", &mut cursor);
        run("cursor_down_box", &mut cursor);
        assert_eq!(at(&cursor), (Pos::new(0, 6), Some(3)));
        for _ in 0..3 {
            run("cursor_right_box", &mut cursor);
        }
        // not further than the wrap column (`GetMaxLineLayout`)
        assert_eq!(at(&cursor), (Pos::new(0, 6), Some(4)));
        run("cursor_left_box", &mut cursor);
        assert_eq!(at(&cursor), (Pos::new(0, 6), Some(3)));
        // the end of a row: the last char of a wrapped one, the end of the text of the last one
        run("cursor_up_box", &mut cursor);
        run("go_line_end_box", &mut cursor);
        assert_eq!(at(&cursor), (Pos::new(0, 3), Some(3)));
        run("cursor_down_box", &mut cursor);
        run("go_line_end_box", &mut cursor);
        assert_eq!(at(&cursor), (Pos::new(0, 6), Some(2)));
    }
}
