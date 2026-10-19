//! The encoding, the BOM and the line ending inside the save dialog itself
//! (`file_dialog::save_fields`). The dialog of the OS is answered by the test
//! (`file_dialog::tests::answer_with`); the second dialog of ours is the fallback for a dialog
//! that has no such fields.

use std::path::{Path, PathBuf};

use gpui::{TestAppContext, VisualTestContext};
use ume_core::encoding::Encoding;
use ume_platform::dialog::{Chosen, Extra, ExtraValue, Kind, Request};

use super::*;
use crate::file_dialog::tests::{answer_with, asked, forget_answer};

/// Forgets the prepared answer when the test ends, even when it fails.
struct Reset;

impl Drop for Reset {
    fn drop(&mut self) {
        forget_answer();
    }
}

fn index_of(encoding: Encoding) -> usize {
    crate::file_ops::save_as_dialog::encoding_choices()
        .iter()
        .position(|e| *e == encoding)
        .unwrap()
}

/// The dialog of the OS answers a save dialog with `path` and the values `extras` makes from
/// what was asked.
fn answer_save(path: &Path, extras: impl Fn(&Request) -> Vec<ExtraValue> + 'static) {
    let path = path.to_owned();
    answer_with(move |request| {
        (request.kind == Kind::SaveFile).then(|| {
            Some(Chosen {
                paths: vec![path.clone()],
                extras: extras(request),
            })
        })
    });
}

fn selected(request: &Request, field: usize) -> usize {
    match &request.extras[field] {
        Extra::Combo { selected, .. } => *selected,
        other => panic!("not a list: {other:?}"),
    }
}

fn checked(request: &Request, field: usize) -> (bool, bool) {
    match &request.extras[field] {
        Extra::Check {
            checked, enabled, ..
        } => (*checked, *enabled),
        other => panic!("not a check box: {other:?}"),
    }
}

fn save_as(f: &Fixture, cx: &mut VisualTestContext) -> Request {
    asked();
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    let mut requests = asked();
    assert_eq!(requests.len(), 1, "{requests:?}");
    requests.pop().unwrap()
}

#[gpui::test]
fn the_fields_start_with_the_format_of_the_document(cx: &mut TestAppContext) {
    let _reset = Reset;
    let dir = TempDir::new();
    let (f, cx) = utf8_fixture(cx, &dir, b"\xEF\xBB\xBFabc");
    let request = save_as(&f, cx);
    assert_eq!(request.kind, Kind::SaveFile);
    assert_eq!(request.extras.len(), 3);
    assert_eq!(selected(&request, 0), index_of(Encoding::Utf8));
    assert_eq!(checked(&request, 1), (true, true), "the BOM of the file");
    assert_eq!(selected(&request, 2), 0, "変換なし");
    assert!(request.follow.is_some());
}

#[gpui::test]
fn the_answer_of_the_dialog_decides_the_encoding_the_bom_and_the_line_ending(
    cx: &mut TestAppContext,
) {
    let _reset = Reset;
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, &[NIHONGO_UTF8, b"\r\nabc"].concat());
    // Shift_JIS, no BOM, LF
    answer_save(&target, |_| {
        vec![
            ExtraValue::Combo(index_of(Encoding::ShiftJis)),
            ExtraValue::Check(false),
            ExtraValue::Combo(2),
        ]
    });
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    assert!(!f.dialog_open(cx), "the second dialog does not appear");
    assert!(!cx.has_pending_prompt());
    assert_eq!(read(&target), [SJIS_NIHONGO, b"\nabc"].concat());
    assert_eq!(f.format(cx), "SJIS  LF");

    // The next dialog starts with what was chosen: Shift_JIS, no BOM, LF.
    let request = save_as_with_cancel(&f, cx);
    assert_eq!(selected(&request, 0), index_of(Encoding::ShiftJis));
    assert_eq!(checked(&request, 1), (false, false));
    assert_eq!(selected(&request, 2), 2);
}

/// A Save As that the user cancels in the dialog; what was asked.
fn save_as_with_cancel(f: &Fixture, cx: &mut VisualTestContext) -> Request {
    answer_with(|_| Some(None));
    let request = save_as(f, cx);
    forget_answer();
    request
}

#[gpui::test]
fn a_bom_is_written_when_the_box_is_checked_for_an_encoding_that_has_one(cx: &mut TestAppContext) {
    let _reset = Reset;
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"abc");
    answer_save(&target, |_| {
        vec![
            ExtraValue::Combo(index_of(Encoding::Utf8)),
            ExtraValue::Check(true),
            ExtraValue::Combo(0),
        ]
    });
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    assert_eq!(read(&target), b"\xEF\xBB\xBFabc");
    assert_eq!(f.format(cx), format!("UTF-8 BOM付  {}", os_eol_name()));
}

#[gpui::test]
fn a_box_left_checked_is_ignored_for_an_encoding_without_a_bom(cx: &mut TestAppContext) {
    // The portal cannot uncheck the box when the encoding changes.
    let _reset = Reset;
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"\xEF\xBB\xBFabc");
    answer_save(&target, |_| {
        vec![
            ExtraValue::Combo(index_of(Encoding::ShiftJis)),
            ExtraValue::Check(true),
            ExtraValue::Combo(0),
        ]
    });
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    assert_eq!(read(&target), b"abc");
    assert_eq!(f.format(cx), format!("SJIS  {}", os_eol_name()));
}

#[gpui::test]
fn utf16_with_a_bom_is_written_from_the_dialog(cx: &mut TestAppContext) {
    let _reset = Reset;
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"a\r\nb");
    answer_save(&target, |_| {
        vec![
            ExtraValue::Combo(index_of(Encoding::Utf16Le)),
            ExtraValue::Check(true),
            ExtraValue::Combo(2),
        ]
    });
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    assert_eq!(read(&target), b"\xFF\xFEa\x00\n\x00b\x00");
}

#[gpui::test]
fn cancelling_the_dialog_writes_nothing(cx: &mut TestAppContext) {
    let _reset = Reset;
    let dir = TempDir::new();
    let (f, cx) = utf8_fixture(cx, &dir, b"abc");
    answer_with(|_| Some(None));
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    assert!(!f.dialog_open(cx));
    assert!(!cx.has_pending_prompt());
    assert_eq!(f.title(cx), "a.txt - UME Editor", "still the old file");
}

#[gpui::test]
fn an_untitled_document_gets_the_fields_in_its_dialog_too(cx: &mut TestAppContext) {
    let _reset = Reset;
    let dir = TempDir::new();
    let target = dir.path("n.txt");
    let (f, cx) = setup(cx);
    cx.simulate_input("hi");
    answer_save(&target, |request| {
        assert_eq!(request.extras.len(), 3);
        vec![
            ExtraValue::Combo(index_of(Encoding::Utf16Le)),
            ExtraValue::Check(true),
            ExtraValue::Combo(0),
        ]
    });
    f.run(cx, "F_FILESAVE", &[]);
    assert!(!f.dialog_open(cx));
    assert_eq!(read(&target), b"\xFF\xFEh\x00i\x00");
}

#[gpui::test]
fn a_macro_that_gives_the_encoding_gets_a_dialog_without_the_fields(cx: &mut TestAppContext) {
    let _reset = Reset;
    let dir = TempDir::new();
    let target: PathBuf = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"abc");
    answer_save(&target, |request| {
        assert!(request.extras.is_empty(), "{:?}", request.extras);
        assert!(request.follow.is_none());
        Vec::new()
    });
    f.run(
        cx,
        "F_FILESAVEAS_DIALOG",
        &[Arg::Text(String::new()), Arg::Int(SJIS)],
    );
    assert!(!f.dialog_open(cx));
    assert_eq!(read(&target), b"abc");
    assert_eq!(f.format(cx), format!("SJIS  {}", os_eol_name()));
}

#[gpui::test]
fn a_dialog_without_the_fields_is_followed_by_the_dialog_of_ours(cx: &mut TestAppContext) {
    // The macOS panel of an old system, or GPUI's dialog: no values come back.
    let _reset = Reset;
    let dir = TempDir::new();
    let target = dir.path("b.txt");
    let (f, cx) = utf8_fixture(cx, &dir, b"abc");
    answer_save(&target, |_| Vec::new());
    f.run(cx, "F_FILESAVEAS_DIALOG", &[]);
    assert!(f.dialog_open(cx));
    f.keys(cx, "enter");
    assert!(!f.dialog_open(cx));
    assert_eq!(read(&target), b"abc");
}
