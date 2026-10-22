//! Tests of the UME-only copies with LF (`copy_lf`) and with CR (`copy_cr`). Sakura has only
//! `F_COPY_CRLF`; these two behave like it (see `tests.rs`), with their own ending.

use crate::clipboard::ClipKind;
use crate::macro_names::{by_code, by_name};
use crate::testing::Editor;

fn clip(e: &Editor) -> (String, ClipKind) {
    let d = e.clipboard().expect("something on the clipboard");
    (d.text.clone(), d.kind)
}

#[test]
fn every_line_ending_of_the_selection_is_changed() {
    // CRLF, LF, CR and LS (an extended ending) in one selection.
    for (id, eol) in [("copy_lf", "\n"), ("copy_cr", "\r")] {
        let mut e = Editor::new("[a\r\nb\nc\rd\u{2028}e|f");
        e.run(id);
        assert_eq!(
            clip(&e),
            (format!("a{eol}b{eol}c{eol}d{eol}e"), ClipKind::Normal),
            "{id}"
        );
        // the selection stays, like `copy_crlf`
        assert!(e.cursor().has_selection());
    }
}

#[test]
fn without_a_selection_the_row_is_copied_as_a_line() {
    for (id, eol) in [("copy_lf", "\n"), ("copy_cr", "\r")] {
        let mut e = Editor::new("a|b\r\nc");
        e.run(id);
        assert_eq!(clip(&e), (format!("ab{eol}"), ClipKind::Line), "{id}");
        // the last row has no ending to change
        let mut e = Editor::new("ab\nc|d");
        e.run(id);
        assert_eq!(clip(&e), ("cd".to_owned(), ClipKind::Line), "{id}");
    }
}

#[test]
fn the_rows_of_a_box_end_with_the_ending_of_the_command() {
    // Sakura's copies of a box always end the rows with CRLF (`GetSelectedData` uses
    // `WCODE::CRLF`); the UME-only ones use their own ending.
    for (id, eol) in [("copy_crlf", "\r\n"), ("copy_lf", "\n"), ("copy_cr", "\r")] {
        let mut e = Editor::new("|abcd\nefgh\nijkl\n");
        e.run("begin_box")
            .run("cursor_right_box")
            .run("cursor_right_box")
            .run("cursor_down_box")
            .run("cursor_down_box")
            .run(id);
        assert_eq!(
            clip(&e),
            (format!("ab{eol}ef{eol}ij{eol}"), ClipKind::Box),
            "{id}"
        );
    }
}

#[test]
fn they_are_macro_commands_that_a_key_macro_records() {
    for (name, function, code) in [
        ("CopyLF", "F_COPY_LF", 32700),
        ("CopyCR", "F_COPY_CR", 32701),
    ] {
        let row = by_name(name).expect(name);
        assert_eq!((row.function, row.code), (function, code));
        assert!(row.args.is_empty() && row.result.is_none() && row.key_macro);
        assert_eq!(by_code(code).map(|r| r.name), Some(name));
    }
    // Sakura's spelling stays as it was.
    assert_eq!(by_name("CopyCRLF").map(|r| r.code), Some(30603));
}
