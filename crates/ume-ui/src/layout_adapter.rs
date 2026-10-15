//! `ume-commands`' [`Layout`] trait over `ume-layout`'s [`DocumentLayout`].
//!
//! The trait stays in `ume-commands` (it has no dependency on `ume-layout`), and this adapter is
//! the "implementation in `ume-ui`" that its documentation promises. Columns are the layout's
//! columns counted from the left edge of the screen, so the indent of a wrapped row is included,
//! and the goal column of the cursor means the same thing in every method.
//!
//! The context calls [`Layout::edited`] right after each edit and [`Layout::reset`] after an undo
//! or a redo, so the layout matches the document all through a command (ADR-0013 section 4).

use std::time::Duration;

use ume_commands::context::TextChange;
use ume_commands::layout::Layout;
use ume_core::buffer::{Document, Edit, Pos, Result};
use ume_layout::DocumentLayout;

/// How long one frame may spend laying out the rest of a document.
pub const LAYOUT_BUDGET: Duration = Duration::from_millis(4);

/// A borrowed [`DocumentLayout`] as a command's [`Layout`].
pub struct RowLayout<'a>(pub &'a mut DocumentLayout);

impl Layout for RowLayout<'_> {
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

    fn display_col(&self, doc: &Document, pos: Pos) -> Result<usize> {
        Ok(self.0.pos_to_row(doc, pos)?.column as usize)
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

    fn pos_at_col(&self, doc: &Document, row_pos: Pos, col: usize) -> Result<Pos> {
        let row = self.0.pos_to_row(doc, row_pos)?.row;
        let col = u32::try_from(col).unwrap_or(u32::MAX);
        Ok(self.0.row_to_pos(doc, row, col)?.pos)
    }

    fn move_rows(&self, doc: &Document, pos: Pos, rows: isize, goal_col: usize) -> Result<Pos> {
        let goal = u32::try_from(goal_col).unwrap_or(u32::MAX);
        Ok(self.0.move_rows(doc, pos, rows, goal)?.pos)
    }
}

#[cfg(test)]
mod tests {
    use ume_commands::command::Arg;
    use ume_commands::context::CommandContext;
    use ume_commands::layout::PlainLayout;
    use ume_commands::registry::Registry;
    use ume_core::cursor::Cursor;
    use ume_core::model::DocumentModel;
    use ume_layout::{LayoutConfig, Wrap};

    use super::*;

    fn doc(s: &str) -> Document {
        Document::from_utf8(s.as_bytes().to_vec()).unwrap()
    }

    fn laid_out(d: &Document, cfg: LayoutConfig) -> DocumentLayout {
        DocumentLayout::laid_out(d, cfg).unwrap()
    }

    /// Without wrapping the real layout must agree with the simple layout that the commands'
    /// own tests use (tab stops every 4 columns, full-width chars take 2).
    #[test]
    fn agrees_with_the_plain_layout_without_wrapping() {
        let d = doc("a\tb日c\r\n\r\nxyz\n");
        let mut layout = laid_out(&d, LayoutConfig::default());
        let real = RowLayout(&mut layout);
        let plain = PlainLayout::default();
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
                real.display_col(&d, pos).unwrap(),
                plain.display_col(&d, pos).unwrap(),
                "display_col {pos:?}"
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
                        real.move_rows(&d, pos, rows, goal).unwrap(),
                        plain.move_rows(&d, pos, rows, goal).unwrap(),
                        "move_rows {pos:?} {rows} {goal}"
                    );
                }
            }
        }
        for col in [0, 1, 3, 4, 5, 6, 7, 99] {
            assert_eq!(
                real.pos_at_col(&d, Pos::new(0, 0), col).unwrap(),
                plain.pos_at_col(&d, Pos::new(0, 0), col).unwrap(),
                "pos_at_col {col}"
            );
        }
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
        assert_eq!(real.display_col(&d, p).unwrap(), 2);
        assert_eq!(
            real.next_row_start(&d, p).unwrap(),
            Some(Pos::new(0, 8)),
            "the next row of the same line"
        );
        assert_eq!(real.move_rows(&d, p, 1, 1).unwrap(), Pos::new(0, 9));
        assert_eq!(real.move_rows(&d, p, -1, 3).unwrap(), Pos::new(0, 3));
        assert_eq!(real.move_rows(&d, p, 2, 1).unwrap(), Pos::new(1, 1));
        assert_eq!(real.next_row_start(&d, Pos::new(1, 1)).unwrap(), None);
    }

    /// Runs commands the way `EditorView` does, and checks after each that the layout kept up
    /// equals a fresh layout of the document.
    fn run_all(text: &str, wrap: Wrap, steps: &[(&str, Vec<Arg>)]) {
        let cfg = LayoutConfig {
            wrap,
            ..LayoutConfig::default()
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
            let key =
                |r: &ume_layout::RowInfo| (r.row, r.start, r.end, r.start_column, r.end_column);
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
}
