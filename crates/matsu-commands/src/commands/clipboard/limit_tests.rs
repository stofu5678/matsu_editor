//! The limit of a copy and a cut (ADR-0020, chapter 5 and 9-8): a selection of `copy_limit()` bytes
//! or more is refused with a message, nothing is put on the clipboard and a cut deletes nothing.
//! The limit is made small here (`with_copy_limit`), so that no test needs 256 MiB.

use matsu_core::buffer::Pos;

use crate::clipboard::{ClipKind, MAX_COPY_BYTES, copy_limit, with_copy_limit};
use crate::testing::Editor;

/// The whole text of the document.
fn text(e: &Editor) -> String {
    let doc = e.model().doc();
    doc.text_between(Pos::new(0, 0), doc.end_pos().unwrap())
        .unwrap()
}

/// The message box and the beep of what the last command did, if it was refused.
fn refusal(e: &Editor) -> Option<String> {
    let changes = e.changes();
    let message = changes.message_box.clone();
    assert_eq!(changes.beep, message.is_some(), "a refusal beeps");
    message
}

#[test]
fn the_limit_is_256_mib() {
    assert_eq!(MAX_COPY_BYTES, 256 * 1024 * 1024);
    assert_eq!(copy_limit(), MAX_COPY_BYTES);
    with_copy_limit(10, || assert_eq!(copy_limit(), 10));
    assert_eq!(copy_limit(), MAX_COPY_BYTES);
}

#[test]
fn a_selection_under_the_limit_is_copied_and_cut_as_before() {
    with_copy_limit(8, || {
        let mut e = Editor::new("a[bcdefg|h");
        e.run("copy");
        assert_eq!(e.clipboard().unwrap().text, "bcdefg");
        assert!(refusal(&e).is_none());
        e.run("cut").assert_marked("a|h");
        assert_eq!(e.clipboard().unwrap().text, "bcdefg");
    });
}

#[test]
fn a_copy_over_the_limit_is_refused_with_a_message_and_copies_nothing() {
    with_copy_limit(8, || {
        for id in ["copy", "copy_crlf", "copy_lf", "copy_cr", "copy_addcrlf"] {
            let mut e = Editor::new("a[bcdefghi|j").with_clipboard("old", ClipKind::Normal);
            e.run(id).assert_marked("a[bcdefghi|j");
            let message = refusal(&e).unwrap_or_else(|| panic!("{id} was not refused"));
            assert!(message.contains("コピーできません"), "{message}");
            assert!(message.contains("上限は 1 MB"), "{message}");
            assert_eq!(e.clipboard().unwrap().text, "old", "{id}");
        }
    });
}

#[test]
fn a_cut_over_the_limit_is_refused_and_deletes_nothing() {
    with_copy_limit(8, || {
        let mut e = Editor::new("a[bcdefghi|j").with_clipboard("old", ClipKind::Normal);
        e.run("cut").assert_marked("a[bcdefghi|j");
        let message = refusal(&e).expect("refused");
        assert!(message.contains("切り取りできません"), "{message}");
        assert!(message.contains("文字は削除していません"), "{message}");
        assert_eq!(text(&e), "abcdefghij");
        assert_eq!(e.clipboard().unwrap().text, "old");
        assert!(!e.changes().text_changed());
    });
}

#[test]
fn a_cut_to_the_end_of_the_line_over_the_limit_deletes_nothing() {
    with_copy_limit(8, || {
        let mut e = Editor::new("a|bcdefghij\nk");
        e.run("line_cut_to_end");
        assert!(refusal(&e).is_some());
        assert_eq!(text(&e), "abcdefghij\nk");
    });
}

#[test]
fn the_limit_counts_the_bytes_of_the_file_not_the_chars() {
    // 3 chars of 3 bytes in UTF-8: 9 bytes
    with_copy_limit(9, || {
        let mut e = Editor::new("[日本語|");
        e.run("copy");
        assert!(refusal(&e).is_some());
    });
    with_copy_limit(10, || {
        let mut e = Editor::new("[日本語|");
        e.run("copy");
        assert!(refusal(&e).is_none());
        assert_eq!(e.clipboard().unwrap().text, "日本語");
    });
}

#[test]
fn the_copy_of_a_row_without_a_selection_has_the_limit_too() {
    with_copy_limit(8, || {
        let mut e = Editor::new("abcdefg|hij\nk");
        e.run("copy");
        assert!(refusal(&e).is_some());
        assert!(e.clipboard().is_none());
        let mut e = Editor::new("abcdefg|hij\nk");
        e.run("cut_line");
        assert!(refusal(&e).is_some());
        assert_eq!(text(&e), "abcdefghij\nk");
    });
}

#[test]
fn copy_lines_over_the_limit_are_refused_and_the_selection_stays() {
    with_copy_limit(9, || {
        for id in [
            "copy_lines",
            "copy_lines_with_line_number",
            "copy_lines_as_passage",
        ] {
            let mut e = Editor::new("ab\nc[de\nfghij|\nklmn");
            e.run(id).assert_marked("ab\nc[de\nfghij|\nklmn");
            let message = refusal(&e).unwrap_or_else(|| panic!("{id} was not refused"));
            assert!(message.contains("コピーできません"), "{message}");
            assert!(e.clipboard().is_none(), "{id}");
        }
    });
}

#[test]
fn a_box_is_counted_by_the_text_of_its_rows() {
    // a box of 2 columns over 4 rows: 8 bytes, though the lines are longer
    let mut e = Editor::new("|abcdefgh\nabcdefgh\nabcdefgh\nabcdefgh");
    e.run("begin_box")
        .run("cursor_right_box")
        .run("cursor_right_box");
    for _ in 0..3 {
        e.run("cursor_down_box");
    }
    with_copy_limit(8, || {
        e.run("copy");
        assert!(refusal(&e).is_some());
        assert!(e.clipboard().is_none());
        e.run("cut");
        assert!(refusal(&e).is_some());
        assert!(text(&e).starts_with("abcdefgh\n"));
    });
    with_copy_limit(9, || {
        e.run("copy");
        assert!(refusal(&e).is_none());
        assert_eq!(e.clipboard().unwrap().text, "ab\r\nab\r\nab\r\nab\r\n");
        assert_eq!(e.clipboard().unwrap().kind, ClipKind::Box);
    });
}

#[test]
fn the_message_says_the_size_and_the_limit_in_megabytes() {
    let message = crate::clipboard::too_big_message(300 * 1024 * 1024, false);
    assert!(message.contains("300 MB"), "{message}");
    assert!(message.contains("上限は 256 MB"), "{message}");
    assert!(message.contains("コピーできません"), "{message}");
    let cut = crate::clipboard::too_big_message(MAX_COPY_BYTES, true);
    assert!(cut.contains("切り取りできません"), "{cut}");
}
