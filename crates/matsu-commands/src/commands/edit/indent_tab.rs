//! `F_INDENT_TAB`: types a tab, or puts a tab before each selected line.
//!
//! Sakura: `CViewCommander::Command_INDENT( WCODE::TAB, INDENT_TAB )` in
//! `sakura_core/cmd/CViewCommander_Edit_advanced.cpp`; bound to Tab in
//! `sakura_core/func/CKeyBind.cpp`.
//!
//! * No selection, or a selection inside one row: types a tab in place of the selection
//!   (Sakura calls `Command_WCHAR`).
//! * Several rows: a tab goes before the text of every row that starts a line and has text. The
//!   rows are selected from their start to the start of the row after the last, caret at the end.
//!
//! * A box selection: a tab goes before the box in each row that has text there
//!   (`CommandContext::indent_box`).
//! * With the soft tab (the type's `m_bInsSpace`, off in Sakura's default types), the tab is the
//!   spaces up to the next tab stop, counted in display columns (a full-width char is 2). Each
//!   place that gets a tab gets its own number of spaces: the caret or the start of the selection,
//!   the start of each row (the tab width), or the left edge of the box in each row. Overwrite mode
//!   replaces the char at the caret, as a typed char does.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct IndentTab;

impl Command for IndentTab {
    const ID: &'static str = "indent_tab";
    const SAKURA: &'static str = "F_INDENT_TAB";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.indent_selection('\t')
    }
}

register_command!(IndentTab);

#[cfg(test)]
mod tests {
    use crate::testing::{Editor, check};

    #[test]
    fn types_a_tab_without_a_selection() {
        check("indent_tab", "ab|cd", "ab\t|cd");
        check("indent_tab", "|", "\t|");
        check("indent_tab", "ab\r\n|", "ab\r\n\t|");
    }

    #[test]
    fn a_selection_inside_one_row_is_replaced_by_a_tab() {
        check("indent_tab", "a[bc|d", "a\t|d");
        check("indent_tab", "a|bc[d", "a\t|d");
    }

    #[test]
    fn indents_every_row_of_a_selection_over_several_rows() {
        check("indent_tab", "a[b\ncd|e\nfg", "[\tab\n\tcde\n|fg");
        // the caret was before the anchor: it ends at the end all the same
        check("indent_tab", "a|b\ncd[e\nfg", "[\tab\n\tcde\n|fg");
    }

    #[test]
    fn a_selection_that_ends_at_column_0_leaves_that_row_alone() {
        check("indent_tab", "a[b\ncd\n|ef", "[\tab\n\tcd\n|ef");
        check("indent_tab", "[ab\ncd\n|", "[\tab\n\tcd\n|");
    }

    #[test]
    fn empty_rows_are_skipped() {
        check("indent_tab", "[ab\n\ncd|", "[\tab\n\n\tcd|");
        check("indent_tab", "[ab\r\n\r\ncd\r\n|", "[\tab\r\n\r\n\tcd\r\n|");
    }

    #[test]
    fn the_last_row_without_a_line_ending_is_indented() {
        check("indent_tab", "[ab\ncd|", "[\tab\n\tcd|");
    }

    #[test]
    fn the_selection_starts_at_the_start_of_the_first_row() {
        check("indent_tab", "ab\nc[d\ne|f", "ab\n[\tcd\n\tef|");
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("a[b\ncd|e");
        e.run("indent_tab").assert_marked("[\tab\n\tcde|");
        e.run("undo").assert_marked("a[b\ncd|e");
    }

    /// Runs `indent_tab` with the soft tab on, over `before`, at the tab width.
    fn soft(width: usize, before: &str, after: &str) {
        let mut e = Editor::new(before)
            .with_tab_width(width)
            .with_ins_space(true);
        e.run("indent_tab").assert_marked(after);
    }

    #[test]
    fn a_soft_tab_fills_up_to_the_next_tab_stop() {
        soft(4, "|", "    |");
        soft(4, "a|", "a   |");
        soft(4, "abc|", "abc |");
        soft(4, "abcd|", "abcd    |");
        soft(8, "ab|cd", "ab      |cd");
        soft(8, "abcdefg|", "abcdefg |");
    }

    #[test]
    fn a_soft_tab_in_overwrite_mode_keeps_the_columns_with_the_fix_mode() {
        // `DelCharForOverwrite` gets the tab char (one column): a full-width char that it
        // replaces leaves a space behind with `m_bOverWriteFixMode`
        let options = crate::edit_options::EditOptions {
            overwrite_fix_mode: true,
            ..Default::default()
        };
        crate::edit_options::with_options(options, || {
            let mut e = Editor::new("a|日b").with_tab_width(4).with_ins_space(true);
            e.set_overwrite(true);
            e.run("indent_tab").assert_marked("a   | b");
            let mut e = Editor::new("a|bc").with_tab_width(4).with_ins_space(true);
            e.set_overwrite(true);
            e.run("indent_tab").assert_marked("a   |c");
        });
        let mut e = Editor::new("a|日b").with_tab_width(4).with_ins_space(true);
        e.set_overwrite(true);
        e.run("indent_tab").assert_marked("a   |b");
    }

    #[test]
    fn a_soft_tab_counts_full_width_chars_as_two_columns() {
        soft(4, "あ|", "あ  |");
        soft(4, "あい|", "あい    |");
        soft(4, "aあ|", "aあ |");
        soft(8, "日本語|", "日本語  |");
        soft(4, "あ|い", "あ  |い");
    }

    #[test]
    fn a_soft_tab_on_a_later_line_counts_from_the_line_start() {
        soft(4, "ab\r\nc|d", "ab\r\nc   |d");
        soft(4, "ab\r\n|", "ab\r\n    |");
    }

    #[test]
    fn a_soft_tab_replaces_a_selection_inside_one_row() {
        soft(4, "a[bc|d", "a   |d");
        soft(4, "a|bc[d", "a   |d");
        // counted from the start of the selection, not from the caret
        soft(4, "ab[cd|e", "ab  |e");
        soft(4, "あ[い|う", "あ  |う");
    }

    #[test]
    fn a_soft_tab_indents_every_row_with_the_tab_width() {
        soft(4, "a[b\ncd|e\nfg", "[    ab\n    cde\n|fg");
        soft(8, "a[b\r\n\r\ncd|e", "[        ab\r\n\r\n        cde|");
        soft(4, "[あい\nう|", "[    あい\n    う|");
        soft(4, "[ab\ncd\n|", "[    ab\n    cd\n|");
    }

    #[test]
    fn a_soft_tab_in_overwrite_mode_takes_the_place_of_one_char() {
        let mut e = Editor::new("ab|cdef").with_ins_space(true);
        e.set_overwrite(true);
        e.run("indent_tab").assert_marked("ab  |def");
        // a full-width char is one char as well
        let mut e = Editor::new("a|いう").with_ins_space(true);
        e.set_overwrite(true);
        e.run("indent_tab").assert_marked("a   |う");
        // a line ending is never overwritten
        let mut e = Editor::new("ab|\r\ncd").with_ins_space(true);
        e.set_overwrite(true);
        e.run("indent_tab").assert_marked("ab  |\r\ncd");
        // with a selection the selection is replaced, and nothing else
        let mut e = Editor::new("a[b|cd").with_ins_space(true);
        e.set_overwrite(true);
        e.run("indent_tab").assert_marked("a   |cd");
    }

    #[test]
    fn a_tab_in_overwrite_mode_takes_the_place_of_one_char() {
        let mut e = Editor::new("ab|cd");
        e.set_overwrite(true);
        e.run("indent_tab").assert_marked("ab\t|d");
        let mut e = Editor::new("ab|\r\ncd");
        e.set_overwrite(true);
        e.run("indent_tab").assert_marked("ab\t|\r\ncd");
    }

    #[test]
    fn the_soft_tab_is_one_undo_step() {
        let mut e = Editor::new("a[b\ncd|e").with_ins_space(true);
        e.run("indent_tab").assert_marked("[    ab\n    cde|");
        e.run("undo").assert_marked("a[b\ncd|e");
        let mut e = Editor::new("ab|cd").with_ins_space(true);
        e.run("indent_tab").assert_marked("ab  |cd");
        e.run("undo").assert_marked("ab|cd");
    }

    #[test]
    fn indent_space_is_never_soft() {
        let mut e = Editor::new("ab|cd").with_ins_space(true);
        e.run("indent_space").assert_marked("ab |cd");
    }
}
