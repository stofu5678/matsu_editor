//! Tests of the two lists of the type settings: the regular expression keywords (`RxKey[NNN]`)
//! and the keyword help dictionaries (`KDct[NN]`).

use std::path::PathBuf;

use gpui::TestAppContext;

use super::*;
use crate::common_settings::table::Op;

fn strings(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|t| (*t).to_owned()).collect()
}

fn rows_of(cx: &mut TestAppContext, id: &'static str) -> Vec<Vec<String>> {
    with_dialog(cx, |d, _, _| d.values().rows(id).to_vec())
}

fn press(cx: &mut TestAppContext, id: &'static str, op: Op) {
    with_dialog(cx, |d, window, dcx| d.table_op(id, op, window, dcx));
}

/// Types into the edit boxes of the list, one text for each column.
fn fill(cx: &mut TestAppContext, id: &'static str, texts: &[&str]) {
    with_dialog(cx, |d, window, dcx| {
        for (column, text) in texts.iter().enumerate() {
            d.set_table_input(id, column, text, window, dcx);
        }
    });
}

/// The sheet of the basic type, opened at `page`.
fn open_page(cx: &mut TestAppContext, text: &str, page: usize) -> Ini {
    let (ini, a) = setup(cx, text);
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(page, dcx));
    ini
}

/// The folder of the settings file, where the dictionaries of a test are.
fn ini_dir(ini: &Ini) -> PathBuf {
    ini.path.parent().unwrap().to_owned()
}

// ---- 正規表現キーワード ----

#[test]
fn the_color_of_a_regex_keyword_is_one_of_the_items_with_both_colors() {
    use super::super::pages::regex::{color_label, color_name, color_options};
    let options = color_options();
    let has = |label: &str| options.iter().any(|(l, _)| *l == label);
    // `SetData`: the items that have a text color and a background, in the order of the list.
    assert!(has("テキスト") && has("コメント") && has("URL") && has("正規表現キーワード10"));
    assert!(!has("カーソル") && !has("表示範囲(ミニマップ)"));
    assert!(options.windows(2).all(|w| w[0].1 < w[1].1));
    // The name of a color in the file and its label in the list.
    assert_eq!(color_label("RK3"), "正規表現キーワード3");
    assert_eq!(color_name("正規表現キーワード3"), "RK3");
    assert_eq!(color_label("URL"), "URL");
    // Something that is no color is 正規表現キーワード1 (`GetData`).
    assert_eq!(color_name("nothing"), "RK1");
    assert_eq!(color_label("nothing"), "正規表現キーワード1");
}

const REGEX_FILE: &str = "[Types(0)]\r\nbUseRegexKeyword=0\r\nRxKey[000]=URL,/a+/k\r\n\
                          RxKey[001]=RK2,m#b/c#ki\r\n";

#[gpui::test]
fn the_regex_list_shows_the_keywords_of_the_file_with_their_colors(cx: &mut TestAppContext) {
    let _ini = open_page(cx, REGEX_FILE, 4);
    assert_eq!(value(cx, USE_REGEX), Value::Bool(false));
    assert_eq!(
        rows_of(cx, REGEX_LIST),
        [
            strings(&["/a+/k", "URL"]),
            strings(&["m#b/c#ki", "正規表現キーワード2"]),
        ]
    );
    // The first row is selected and shown in the edit boxes (`SetData`).
    let first = with_dialog(cx, |d, _, dcx| {
        (
            d.selected_row(REGEX_LIST),
            d.table_input_text(REGEX_LIST, 0, dcx),
            d.table_input_text(REGEX_LIST, 1, dcx),
        )
    });
    assert_eq!(
        first,
        (Some(0), Some("/a+/k".to_owned()), Some("URL".to_owned()))
    );
    for id in [USE_REGEX, REGEX_LIST, REGEX_IMPORT, REGEX_EXPORT] {
        assert!(usable(cx, id), "{id}");
    }
}

#[gpui::test]
fn the_regex_buttons_edit_the_list_and_ok_writes_the_keywords(cx: &mut TestAppContext) {
    let ini = open_page(cx, REGEX_FILE, 4);
    check(cx, USE_REGEX, true);
    // 追加 puts the keyword at the end, with a color of the list.
    fill(cx, REGEX_LIST, &["/\\d+/k", "半角数値"]);
    press(cx, REGEX_LIST, Op::Add);
    // 挿入 puts it before the selected row (the one just added), with a color that is not in
    // the list.
    fill(cx, REGEX_LIST, &["m/x/ki", "nothing"]);
    press(cx, REGEX_LIST, Op::Insert);
    let shown: Vec<(String, String)> = rows_of(cx, REGEX_LIST)
        .into_iter()
        .map(|r| (r[0].clone(), r[1].clone()))
        .collect();
    let pair = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(
        shown,
        [
            pair("/a+/k", "URL"),
            pair("m#b/c#ki", "正規表現キーワード2"),
            pair("m/x/ki", "正規表現キーワード1"),
            pair("/\\d+/k", "半角数値"),
        ]
    );
    // 先頭 moves the selected row to the top, 削除 removes it.
    press(cx, REGEX_LIST, Op::Top);
    press(cx, REGEX_LIST, Op::Delete);
    assert_eq!(rows_of(cx, REGEX_LIST).len(), 3);
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "bUseRegexKeyword").as_deref(),
        Some("1")
    );
    assert_eq!(
        ini.get("Types(0)", "RxKey[000]").as_deref(),
        Some("URL,/a+/k")
    );
    assert_eq!(
        ini.get("Types(0)", "RxKey[001]").as_deref(),
        Some("RK2,m#b/c#ki")
    );
    assert_eq!(
        ini.get("Types(0)", "RxKey[002]").as_deref(),
        Some("NUM,/\\d+/k")
    );
    assert_eq!(ini.get("Types(0)", "RxKey[003]"), None);
    // The windows have the new keywords.
    let keywords = cx.update(|app| {
        crate::doc_types::with_user(app, |user| {
            let t = user.document_types().get(0).unwrap();
            (t.use_regex_keyword, t.regex_keywords.len())
        })
    });
    assert_eq!(keywords, (true, 3));
}

#[gpui::test]
fn a_deleted_keyword_is_gone_from_the_file_and_from_the_windows(cx: &mut TestAppContext) {
    let ini = open_page(cx, REGEX_FILE, 4);
    press(cx, REGEX_LIST, Op::Delete);
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "RxKey[000]").as_deref(),
        Some("RK2,m#b/c#ki")
    );
    // The first key that is gone is blank: it ends the list.
    assert_eq!(ini.get("Types(0)", "RxKey[001]").as_deref(), Some(""));
    assert_eq!(ini.get("Types(0)", "RxKey[002]"), None);
    let count = cx.update(|app| {
        crate::doc_types::with_user(app, |user| {
            user.document_types().get(0).unwrap().regex_keywords.len()
        })
    });
    assert_eq!(count, 1);
}

#[gpui::test]
fn the_keywords_a_built_in_type_starts_with_can_be_deleted(cx: &mut TestAppContext) {
    // テキスト (Types(1)) has regular expression keywords of its own that the file does not.
    let (ini, _a) = setup(cx, "");
    cx.update(|app| open(app, 1, Some(4), None)).unwrap();
    let started = rows_of(cx, REGEX_LIST).len();
    assert!(started >= 2, "{started}");
    // Delete the last one: the others stay.
    press(cx, REGEX_LIST, Op::Last);
    press(cx, REGEX_LIST, Op::Delete);
    ok(cx);
    let count = |cx: &mut TestAppContext| {
        cx.update(|app| {
            crate::doc_types::with_user(app, |user| {
                user.document_types().get(1).unwrap().regex_keywords.len()
            })
        })
    };
    assert_eq!(count(cx), started - 1);
    // Delete them all: none comes back.
    cx.update(|app| open(app, 1, Some(4), None)).unwrap();
    for _ in 0..started {
        press(cx, REGEX_LIST, Op::Delete);
    }
    assert!(rows_of(cx, REGEX_LIST).is_empty());
    ok(cx);
    assert_eq!(count(cx), 0);
    assert_eq!(ini.get("Types(1)", "RxKey[000]").as_deref(), Some(""));
    // and the same when the file is read again
    let again = ume_config::settings::DocumentTypes::from_profile(&ini.profile());
    assert!(again.get(1).unwrap().regex_keywords.is_empty());
}

#[gpui::test]
fn a_keyword_that_is_not_framed_or_is_there_already_is_refused(cx: &mut TestAppContext) {
    let _ini = open_page(cx, REGEX_FILE, 4);
    let before = rows_of(cx, REGEX_LIST);
    // `RegexKeyCheckSyntax`: it needs / and /k.
    fill(cx, REGEX_LIST, &["abc", "URL"]);
    press(cx, REGEX_LIST, Op::Add);
    fill(cx, REGEX_LIST, &["/abc/", "URL"]);
    press(cx, REGEX_LIST, Op::Add);
    // `CheckKeywordList`: the same keyword twice.
    fill(cx, REGEX_LIST, &["/a+/k", "コメント"]);
    press(cx, REGEX_LIST, Op::Add);
    assert_eq!(rows_of(cx, REGEX_LIST), before);
    // 更新 may keep the keyword of its own row and change the color,
    with_dialog(cx, |d, window, dcx| {
        d.select_row(REGEX_LIST, 0, window, dcx)
    });
    fill(cx, REGEX_LIST, &["/a+/k", "コメント"]);
    press(cx, REGEX_LIST, Op::Update);
    assert_eq!(rows_of(cx, REGEX_LIST)[0], strings(&["/a+/k", "コメント"]));
    // but not take the keyword of another row.
    fill(cx, REGEX_LIST, &["m#b/c#ki", "コメント"]);
    press(cx, REGEX_LIST, Op::Update);
    assert_eq!(rows_of(cx, REGEX_LIST)[0][0], "/a+/k");
    cancel(cx);
}

#[gpui::test]
fn the_regex_list_stops_at_a_hundred_keywords(cx: &mut TestAppContext) {
    let _ini = open_page(cx, "", 4);
    for i in 0..100 {
        fill(cx, REGEX_LIST, &[&format!("/w{i}/k"), "URL"]);
        press(cx, REGEX_LIST, Op::Add);
    }
    assert_eq!(rows_of(cx, REGEX_LIST).len(), 100);
    fill(cx, REGEX_LIST, &["/more/k", "URL"]);
    press(cx, REGEX_LIST, Op::Add);
    assert_eq!(rows_of(cx, REGEX_LIST).len(), 100);
    cancel(cx);
}

#[gpui::test]
fn the_regex_keywords_stay_when_another_page_changes(cx: &mut TestAppContext) {
    let ini = open_page(cx, REGEX_FILE, 4);
    with_dialog(cx, |d, _, dcx| d.select_index(0, dcx));
    type_in(cx, TAB_SPACE, "8");
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "RxKey[000]").as_deref(),
        Some("URL,/a+/k")
    );
    assert_eq!(
        ini.get("Types(0)", "RxKey[001]").as_deref(),
        Some("RK2,m#b/c#ki")
    );
}

// ---- キーワードヘルプ ----

/// A dictionary in UTF-8 with the BOM.
const DICT: &[u8] = b"\xEF\xBB\xBF\xE8\xBE\x9E\xE6\x9B\xB8, one\r\nword /// meaning\r\n";

#[gpui::test]
fn the_keyword_help_list_shows_the_file_and_the_options_are_greyed_out(cx: &mut TestAppContext) {
    let ini = open_page(
        cx,
        "[Types(0)]\r\nbUseKeyWordHelp=1\r\nbUseKeyHelpAllSearch=1\r\nnKeyHelpRMenuShowType=2\r\n\
         KDct[00]=1,Memo,dict.txt\r\nKDct[01]=0,,other.txt\r\n",
        5,
    );
    assert_eq!(value(cx, USE_KEYHELP), Value::Bool(true));
    assert_eq!(value(cx, KEYHELP_ALL_SEARCH), Value::Bool(true));
    assert_eq!(value(cx, KEYHELP_KEY_DISP), Value::Bool(false));
    assert_eq!(value(cx, KEYHELP_RMENU), Value::Int(2));
    assert_eq!(
        rows_of(cx, KEYHELP_LIST),
        [
            strings(&["dict.txt", "Memo", "○"]),
            strings(&["other.txt", "", "×"]),
        ]
    );
    assert!(usable(cx, KEYHELP_LIST));
    // The feature is not here: its settings and the buttons of the files are greyed out.
    for id in [
        USE_KEYHELP,
        KEYHELP_ALL_SEARCH,
        KEYHELP_KEY_DISP,
        KEYHELP_PREFIX,
        KEYHELP_RMENU,
        KEYHELP_IMPORT,
        KEYHELP_EXPORT,
    ] {
        assert!(!usable(cx, id), "{id}");
    }
    // Another page changes: the keys of the page are as the file had them.
    with_dialog(cx, |d, _, dcx| d.select_index(0, dcx));
    type_in(cx, TAB_SPACE, "8");
    ok(cx);
    assert_eq!(ini.get("Types(0)", "bUseKeyWordHelp").as_deref(), Some("1"));
    assert_eq!(
        ini.get("Types(0)", "KDct[00]").as_deref(),
        Some("1,Memo,dict.txt")
    );
    assert_eq!(
        ini.get("Types(0)", "KDct[01]").as_deref(),
        Some("0,,other.txt")
    );
    assert_eq!(
        ini.get("Types(0)", "nKeyHelpRMenuShowType").as_deref(),
        Some("2")
    );
}

#[gpui::test]
fn a_dictionary_is_checked_and_described_when_it_is_put_in_the_list(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    std::fs::write(ini_dir(&ini).join("jp.txt"), DICT).unwrap();
    // Shift_JIS without a BOM: "あ,い" and a second line
    std::fs::write(
        ini_dir(&ini).join("sj.txt"),
        b"\x82\xA0,\x82\xA2\r\nx /// y\r\n",
    )
    .unwrap();
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(5, dcx));
    assert!(rows_of(cx, KEYHELP_LIST).is_empty());
    // 挿入: a relative path starts at the folder of the settings file; the description is the
    // first line with the commas turned into dots, and the dictionary is used.
    fill(cx, KEYHELP_LIST, &["jp.txt", "", ""]);
    press(cx, KEYHELP_LIST, Op::Insert);
    assert_eq!(
        rows_of(cx, KEYHELP_LIST),
        [strings(&["jp.txt", "辞書. one", "○"])]
    );
    // A × typed in the box keeps the new dictionary off; it goes before the selected row.
    fill(cx, KEYHELP_LIST, &["sj.txt", "", "×"]);
    press(cx, KEYHELP_LIST, Op::Insert);
    assert_eq!(
        rows_of(cx, KEYHELP_LIST),
        [
            strings(&["sj.txt", "あ.い", "×"]),
            strings(&["jp.txt", "辞書. one", "○"]),
        ]
    );
    // A file that does not open, and a dictionary that is there already, are refused.
    let before = rows_of(cx, KEYHELP_LIST);
    fill(cx, KEYHELP_LIST, &["missing.txt", "", ""]);
    press(cx, KEYHELP_LIST, Op::Insert);
    fill(cx, KEYHELP_LIST, &["jp.txt", "", ""]);
    press(cx, KEYHELP_LIST, Op::Insert);
    assert_eq!(rows_of(cx, KEYHELP_LIST), before);
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "KDct[00]").as_deref(),
        Some("0,あ.い,sj.txt")
    );
    assert_eq!(
        ini.get("Types(0)", "KDct[01]").as_deref(),
        Some("1,辞書. one,jp.txt")
    );
    assert_eq!(ini.get("Types(0)", "KDct[02]"), None);
}

#[gpui::test]
fn updating_a_dictionary_reads_its_first_line_again(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "[Types(0)]\r\nKDct[00]=0,Old,d.txt\r\n");
    std::fs::write(ini_dir(&ini).join("d.txt"), DICT).unwrap();
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(5, dcx));
    // The dictionary keeps its own path (that is not a repeat) and is used again (`UPD`).
    fill(cx, KEYHELP_LIST, &["d.txt", "", ""]);
    press(cx, KEYHELP_LIST, Op::Update);
    assert_eq!(
        rows_of(cx, KEYHELP_LIST),
        [strings(&["d.txt", "辞書. one", "○"])]
    );
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "KDct[00]").as_deref(),
        Some("1,辞書. one,d.txt")
    );
}

#[gpui::test]
fn the_keyword_help_list_has_no_add_button_and_stops_at_twenty(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    std::fs::write(ini_dir(&ini).join("d.txt"), DICT).unwrap();
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(5, dcx));
    // `IDD_PROP_KEYHELP` has 挿入 but no 追加.
    fill(cx, KEYHELP_LIST, &["d.txt", "", ""]);
    press(cx, KEYHELP_LIST, Op::Add);
    assert!(rows_of(cx, KEYHELP_LIST).is_empty());
    for i in 0..20 {
        std::fs::write(ini_dir(&ini).join(format!("d{i}.txt")), DICT).unwrap();
        fill(cx, KEYHELP_LIST, &[&format!("d{i}.txt"), "", ""]);
        press(cx, KEYHELP_LIST, Op::Insert);
    }
    assert_eq!(rows_of(cx, KEYHELP_LIST).len(), 20);
    fill(cx, KEYHELP_LIST, &["d.txt", "", ""]);
    press(cx, KEYHELP_LIST, Op::Insert);
    assert_eq!(rows_of(cx, KEYHELP_LIST).len(), 20);
    cancel(cx);
}

#[gpui::test]
fn the_keyword_help_page_has_sakuras_defaults(cx: &mut TestAppContext) {
    let _ini = open_page(cx, "", 5);
    assert_eq!(value(cx, USE_KEYHELP), Value::Bool(false));
    // `KEYHELP_RMENU_TOP`
    assert_eq!(value(cx, KEYHELP_RMENU), Value::Int(1));
    assert!(rows_of(cx, KEYHELP_LIST).is_empty());
    cancel(cx);
}
