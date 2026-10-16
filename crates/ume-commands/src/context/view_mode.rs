//! Tests of view mode (`F_VIEWMODE`): while it is on, every edit is refused.
//!
//! The mode itself belongs to the window (`ume-ui`); the context only refuses. Sakura's list of
//! functions that view mode greys out is `EIsModificationForbidden` in
//! `sakura_core/doc/CEditDoc.cpp`.

#[cfg(test)]
mod tests {
    use crate::clipboard::ClipKind;
    use crate::command::{Arg, CommandError};
    use crate::testing::Editor;

    #[test]
    fn edits_beep_and_change_nothing() {
        // A command may select before it edits (word_delete selects the word); the text is what
        // must stay.
        let plain = |e: &Editor| e.marked().replace(['|', '['], "");
        for id in [
            "delete",
            "delete_back",
            "word_delete",
            "delete_line",
            "duplicate_line",
            "indent_tab",
            "to_upper",
            "paste",
            "cut",
            "cut_line",
            "line_cut_to_start",
            "line_cut_to_end",
            "sort_asc",
        ] {
            let mut e = Editor::new("ef|gh\nabcd\n").with_clipboard("x", ClipKind::Normal);
            if id == "cut" || id == "to_upper" || id == "sort_asc" {
                e.run("select_all");
            }
            let text = plain(&e);
            e.set_view_mode(true);
            let result = e.try_run(id, &[]);
            assert!(
                matches!(result, Err(CommandError::Unavailable(_))),
                "{id}: {result:?}"
            );
            assert!(e.changes().beep, "{id}");
            assert!(!e.changes().text_changed(), "{id}");
            assert_eq!(plain(&e), text, "{id}");
            assert!(!e.model().is_modified(), "{id}");
            // a refused cut must not replace what the clipboard has
            assert_eq!(e.clipboard().map(|c| c.text.as_str()), Some("x"), "{id}");
        }
        let mut e = Editor::new("ab|cd\nefgh");
        let text = plain(&e);
        e.set_view_mode(true);
        for c in ['x', '\r', 'あ'] {
            assert!(e.try_run("input_char", &[Arg::Char(c)]).is_err());
            assert_eq!(plain(&e), text);
        }
        assert!(!e.model().is_modified());
    }

    #[test]
    fn a_selection_is_not_replaced() {
        let mut e = Editor::new("a[bc|d");
        e.set_view_mode(true);
        assert!(e.try_run("input_char", &[Arg::Char('x')]).is_err());
        assert!(e.try_run("delete", &[]).is_err());
        e.assert_marked("a[bc|d");
    }

    #[test]
    fn undo_redo_and_the_typing_mode_are_refused_too() {
        let mut e = Editor::new("ab|cd");
        e.run_with("input_char", &[Arg::Char('X')]).run("undo");
        e.assert_marked("ab|cd");
        e.set_view_mode(true);
        // Sakura lists F_UNDO, F_REDO and F_CHGMOD_INS as forbidden.
        for id in ["redo", "undo", "toggle_overwrite_mode"] {
            assert!(matches!(
                e.try_run(id, &[]),
                Err(CommandError::Unavailable(_))
            ));
            assert!(e.changes().beep, "{id}");
        }
        e.assert_marked("ab|cd");
        assert!(!e.overwrite());
        // Back to normal: the redo that was refused is still there.
        e.set_view_mode(false);
        e.run("redo").assert_marked("abX|cd");
    }

    #[test]
    fn moves_selections_and_copy_still_work() {
        let mut e = Editor::new("|ab\ncd\nef");
        e.set_view_mode(true);
        e.run("cursor_right").assert_marked("a|b\ncd\nef");
        e.run("cursor_down_sel").assert_marked("a[b\nc|d\nef");
        e.run("copy");
        assert_eq!(e.clipboard().map(|c| c.text.as_str()), Some("b\nc"));
        e.run("select_all").run("go_file_top");
        e.assert_marked("|ab\ncd\nef");
        assert!(!e.changes().beep);
    }

    #[test]
    fn japanese_text_and_crlf_are_left_alone() {
        let mut e = Editor::new("日本|語\r\nテキスト");
        e.set_view_mode(true);
        assert!(e.try_run("input_char", &[Arg::Char('の')]).is_err());
        assert!(e.try_run("delete_back", &[]).is_err());
        e.assert_marked("日本|語\r\nテキスト");
    }
}
