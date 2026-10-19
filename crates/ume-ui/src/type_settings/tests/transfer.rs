//! Tests of the functions that open the sheet at a page (`F_TYPE_*`), and of the インポート and
//! エクスポート of the regular expression keywords (`.rkw`).

use gpui::TestAppContext;

use super::*;
use crate::common_settings::model::Kind;

fn strings(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|t| (*t).to_owned()).collect()
}

fn rows(cx: &mut TestAppContext) -> Vec<Vec<String>> {
    with_dialog(cx, |d, _, _| d.values().rows(REGEX_LIST).to_vec())
}

/// The sheet of the basic type, opened at the page 正規表現キーワード.
fn open_regex_page(cx: &mut TestAppContext, text: &str) -> Ini {
    let (ini, a) = setup(cx, text);
    run(cx, a, "F_TYPE_REGEX_KEYWORD", &[]);
    ini
}

/// Presses the button `id` of the page the sheet shows.
fn press_button(cx: &mut TestAppContext, id: &'static str) {
    with_dialog(cx, |d, window, dcx| {
        let control = d
            .page_controls()
            .into_iter()
            .find(|c| c.id == id)
            .expect("the button");
        assert!(d.is_usable(&control), "{id} is usable");
        let Kind::Button { run } = control.kind else {
            panic!("{id} is not a button");
        };
        run(d, window, dcx);
    });
}

#[gpui::test]
fn each_function_of_a_page_opens_the_sheet_at_that_page(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    for (function, page) in [
        ("F_TYPE_SCREEN", 0),
        ("F_TYPE_COLOR", 1),
        ("F_TYPE_WINDOW", 2),
        ("F_TYPE_HELPER", 3),
        ("F_TYPE_REGEX_KEYWORD", 4),
        ("F_TYPE_KEYHELP", 5),
    ] {
        run(cx, handle, function, &[]);
        assert_eq!(
            with_dialog(cx, |d, _, _| d.page_index()),
            page,
            "{function}"
        );
        assert_eq!(
            with_dialog(cx, |d, _, _| d.values().int(TYPE_INDEX)),
            0,
            "{function}: the type of the document"
        );
    }
    cancel(cx);
}

#[gpui::test]
fn the_regex_keywords_go_to_a_file_and_come_back(cx: &mut TestAppContext) {
    let ini = open_regex_page(
        cx,
        "[Types(0)]\r\nRxKey[000]=URL,/a+/k\r\nRxKey[001]=RK2,m#b/c#ki\r\n",
    );
    let dir = ini.path.parent().unwrap().to_owned();
    let file = dir.join("words.rkw");
    assert!(with_dialog(cx, |d, window, dcx| d.regex_export_to(&file, window, dcx)));
    let text = String::from_utf8(std::fs::read(&file).unwrap()).unwrap();
    assert!(text.contains("// 正規表現キーワード Ver1\r\n"), "{text}");
    assert!(text.contains("RxKey[000]=URL,/a+/k\r\n"), "{text}");
    assert!(text.contains("RxKey[001]=RK2,m#b/c#ki\r\n"), "{text}");
    // Another list, then the file replaces it.
    with_dialog(cx, |d, _, dcx| {
        d.set_rows(REGEX_LIST, vec![strings(&["/z/k", "テキスト"])], dcx)
    });
    assert_eq!(rows(cx).len(), 1);
    assert!(with_dialog(cx, |d, window, dcx| d.regex_import_from(&file, window, dcx)));
    assert_eq!(
        rows(cx),
        [
            strings(&["/a+/k", "URL"]),
            strings(&["m#b/c#ki", "正規表現キーワード2"]),
        ]
    );
    // A file that is not there is not read, and the list stays.
    assert!(!with_dialog(cx, |d, window, dcx| d.regex_import_from(
        &dir.join("missing.rkw"),
        window,
        dcx
    )));
    assert_eq!(rows(cx).len(), 2);
    cancel(cx);
}

#[gpui::test]
fn the_regex_dialogs_list_the_rkw_files_and_remember_the_folder(cx: &mut TestAppContext) {
    let ini = open_regex_page(cx, "");
    crate::file_dialog::tests::asked();
    press_button(cx, REGEX_IMPORT);
    let request = crate::file_dialog::tests::asked().pop().expect("a dialog");
    let names: Vec<_> = request.filters.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        ["ユーザー指定", "テキストファイル", "すべてのファイル"]
    );
    assert_eq!(request.filters[0].patterns, ["*.rkw"]);
    assert_eq!(request.start.as_deref(), Some(ini.path.parent().unwrap()));
    let other = tempfile::tempdir().unwrap();
    let target = other.path().join("out.rkw");
    press_button(cx, REGEX_EXPORT);
    let request = crate::file_dialog::tests::asked().pop().expect("a dialog");
    assert_eq!(request.filters[0].patterns, ["*.rkw"]);
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    assert!(target.exists());
    press_button(cx, REGEX_IMPORT);
    let request = crate::file_dialog::tests::asked().pop().expect("a dialog");
    assert_eq!(request.start.as_deref(), Some(other.path()));
    cancel(cx);
}
