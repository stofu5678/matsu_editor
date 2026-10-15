//! Keeps a view's [`DocumentLayout`] equal to the document after a command (ADR-0013 section 4).
//!
//! `DocumentLayout::edited` wants the one `Edit` that just happened, with the document right
//! after it. A command may edit several times (or undo, which swaps the whole document), and by
//! the time `ume-ui` hears of it the document is past the first edit. So a single edit is followed
//! incrementally, and anything else starts the layout again from the document, which lays out the
//! first screenful at once and the rest in the background (`DocumentLayout::layout_for`).

use std::time::Duration;

use ume_commands::context::Changes;
use ume_core::buffer::{Document, Edit, Result};
use ume_layout::DocumentLayout;

/// How long one frame may spend laying out the rest of a document.
pub const LAYOUT_BUDGET: Duration = Duration::from_millis(4);

/// Brings `layout` up to date after a command that made `changes` and left `doc`.
pub fn apply_changes(layout: &mut DocumentLayout, doc: &Document, changes: &Changes) -> Result<()> {
    match changes.edits.as_slice() {
        [] if !changes.replaced_all => Ok(()),
        [one] if !changes.replaced_all => layout.edited(
            doc,
            &Edit {
                start: one.start,
                end: one.new_end,
            },
        ),
        _ => {
            layout.resync(doc)?;
            layout.layout_for(doc, LAYOUT_BUDGET).map(|_| ())
        }
    }
}

#[cfg(test)]
mod tests {
    use ume_commands::command::Arg;
    use ume_commands::context::CommandContext;
    use ume_commands::registry::Registry;
    use ume_core::buffer::Pos;
    use ume_core::cursor::Cursor;
    use ume_core::model::DocumentModel;
    use ume_layout::{LayoutConfig, Wrap};

    use super::*;
    use crate::layout_adapter::RowLayout;

    fn model(s: &str) -> DocumentModel {
        DocumentModel::new(Document::from_utf8(s.as_bytes().to_vec()).unwrap())
    }

    /// Runs commands the way `EditorView` does, and checks after each that the layout kept up
    /// equals a fresh layout of the document.
    fn run_all(text: &str, wrap: Wrap, steps: &[(&str, Vec<Arg>)]) {
        let cfg = LayoutConfig {
            wrap,
            ..LayoutConfig::default()
        };
        let mut model = model(text);
        let mut cursor = Cursor::default();
        let mut layout = DocumentLayout::laid_out(model.doc(), cfg.clone()).unwrap();
        for (id, args) in steps {
            let changes = {
                let adapter = RowLayout(&layout);
                let mut cx = CommandContext::new(&mut model, &mut cursor, &adapter, None);
                Registry::global().run(id, &mut cx, args).unwrap();
                cx.finish()
            };
            apply_changes(&mut layout, model.doc(), &changes).unwrap();
            layout.layout_all(model.doc()).unwrap();
            let fresh = DocumentLayout::laid_out(model.doc(), cfg.clone()).unwrap();
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

    #[test]
    fn a_command_without_edits_leaves_the_layout_alone() {
        let mut model = model("ab\ncd");
        let mut cursor = Cursor::default();
        let mut layout = DocumentLayout::laid_out(model.doc(), LayoutConfig::default()).unwrap();
        let adapter = RowLayout(&layout);
        let mut cx = CommandContext::new(&mut model, &mut cursor, &adapter, None);
        Registry::global().run("cursor_down", &mut cx, &[]).unwrap();
        let changes = cx.finish();
        assert!(!changes.text_changed());
        apply_changes(&mut layout, model.doc(), &changes).unwrap();
        assert_eq!(cursor.caret(), Pos::new(1, 0));
    }
}
