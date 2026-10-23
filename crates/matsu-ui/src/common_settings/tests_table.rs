//! Tests of the editable list in the dialog (the list of 「ファイル名表示」) and of what the page
//! does to the windows, in GPUI's headless test windows.

use gpui::TestAppContext;

use super::Page;
use super::table::Op;
use super::tests::{Ini, cancel, ok, run, setup, type_in, with_dialog};

const LIST: &str = "file_name_list";

const FILE: &str = "[Other]\r\nx=1\r\n[Nickname]\r\nbShortPath=0\r\nnShortPathMaxWidth=60\r\n\
                    ArrNum=3\r\nFrom00=/zz/a/\r\nTo00=A:\r\nFrom01=/zz/b/\r\nTo01=B:\r\n\
                    From02=/zz/c/\r\nTo02=C:\r\n";

fn rows(cx: &mut TestAppContext) -> Vec<Vec<String>> {
    with_dialog(cx, |d, _, _| d.values().rows(LIST).to_vec())
}

fn firsts(cx: &mut TestAppContext) -> Vec<String> {
    rows(cx).into_iter().map(|mut r| r.remove(0)).collect()
}

fn selected(cx: &mut TestAppContext) -> Option<usize> {
    with_dialog(cx, |d, _, _| d.selected_row(LIST))
}

fn boxes(cx: &mut TestAppContext) -> (String, String) {
    with_dialog(cx, |d, _, dcx| {
        (
            d.table_input_text(LIST, 0, dcx).unwrap(),
            d.table_input_text(LIST, 1, dcx).unwrap(),
        )
    })
}

fn edit(cx: &mut TestAppContext, from: &str, to: &str) {
    with_dialog(cx, |d, window, dcx| {
        d.set_table_input(LIST, 0, from, window, dcx);
        d.set_table_input(LIST, 1, to, window, dcx);
    });
}

fn press(cx: &mut TestAppContext, op: Op) {
    with_dialog(cx, |d, window, dcx| d.table_op(LIST, op, window, dcx));
}

fn open_page(cx: &mut TestAppContext, text: &str) -> (Ini, gpui::AnyWindowHandle) {
    let (ini, handle) = setup(cx, text);
    run(cx, handle, "F_OPTION_FNAME");
    (ini, handle)
}

#[gpui::test]
fn the_list_shows_the_file_and_the_first_row_is_selected(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_page(cx, FILE);
    assert_eq!(firsts(cx), ["/zz/a/", "/zz/b/", "/zz/c/"]);
    assert_eq!(selected(cx), Some(0));
    // `SetData` selects the first row, and its texts are in the edit boxes.
    assert_eq!(boxes(cx), ("/zz/a/".to_owned(), "A:".to_owned()));
}

#[gpui::test]
fn a_file_without_a_list_shows_sakuras_seven_rows(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_page(cx, "");
    assert_eq!(rows(cx).len(), 7);
    assert_eq!(
        rows(cx)[0],
        vec!["%DeskTop%\\".to_owned(), "デスクトップ\\".to_owned()]
    );
    assert_eq!(rows(cx)[6][0], "%AppData%\\");
}

#[gpui::test]
fn an_empty_list_in_the_file_is_an_empty_list(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_page(cx, "[Nickname]\r\nArrNum=0\r\n");
    assert!(rows(cx).is_empty());
    assert_eq!(selected(cx), None);
    assert_eq!(boxes(cx), (String::new(), String::new()));
}

#[gpui::test]
fn a_click_on_a_row_puts_it_in_the_edit_boxes(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_page(cx, FILE);
    with_dialog(cx, |d, window, dcx| d.select_row(LIST, 2, window, dcx));
    assert_eq!(selected(cx), Some(2));
    assert_eq!(boxes(cx), ("/zz/c/".to_owned(), "C:".to_owned()));
    // A row that is not there is not selected.
    with_dialog(cx, |d, window, dcx| d.select_row(LIST, 9, window, dcx));
    assert_eq!(selected(cx), Some(2));
}

#[gpui::test]
fn the_buttons_change_the_rows(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_page(cx, FILE);
    // 更新: the selected row.
    edit(cx, "/zz/a2/", "A2:");
    press(cx, Op::Update);
    assert_eq!(firsts(cx), ["/zz/a2/", "/zz/b/", "/zz/c/"]);
    // 追加: at the end, and it is selected.
    edit(cx, "/zz/d/", "D:");
    press(cx, Op::Add);
    assert_eq!(firsts(cx), ["/zz/a2/", "/zz/b/", "/zz/c/", "/zz/d/"]);
    assert_eq!(selected(cx), Some(3));
    // 挿入: before the selected row.
    edit(cx, "/zz/x/", "X:");
    press(cx, Op::Insert);
    assert_eq!(
        firsts(cx),
        ["/zz/a2/", "/zz/b/", "/zz/c/", "/zz/x/", "/zz/d/"]
    );
    assert_eq!(selected(cx), Some(3));
    // 先頭, 下へ, 上へ, 最終 move the selected row; the edit boxes keep showing it.
    press(cx, Op::Top);
    assert_eq!(firsts(cx)[0], "/zz/x/");
    assert_eq!(selected(cx), Some(0));
    assert_eq!(boxes(cx), ("/zz/x/".to_owned(), "X:".to_owned()));
    press(cx, Op::Down);
    assert_eq!(firsts(cx)[..2], ["/zz/a2/", "/zz/x/"]);
    press(cx, Op::Up);
    assert_eq!(firsts(cx)[0], "/zz/x/");
    press(cx, Op::Last);
    assert_eq!(firsts(cx).last().map(String::as_str), Some("/zz/x/"));
    assert_eq!(selected(cx), Some(4));
    // 削除: the row that comes to its place is shown; at the end nothing is.
    press(cx, Op::Delete);
    assert_eq!(firsts(cx), ["/zz/a2/", "/zz/b/", "/zz/c/", "/zz/d/"]);
    assert_eq!(selected(cx), None);
    assert_eq!(boxes(cx), (String::new(), String::new()));
    with_dialog(cx, |d, window, dcx| d.select_row(LIST, 1, window, dcx));
    press(cx, Op::Delete);
    assert_eq!(firsts(cx), ["/zz/a2/", "/zz/c/", "/zz/d/"]);
    assert_eq!(selected(cx), Some(1));
    assert_eq!(boxes(cx).0, "/zz/c/");
}

#[gpui::test]
fn an_empty_first_box_adds_nothing_and_a_full_list_says_so(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_page(cx, FILE);
    edit(cx, "", "nothing");
    press(cx, Op::Add);
    press(cx, Op::Insert);
    assert_eq!(rows(cx).len(), 3);
    assert!(!cx.has_pending_prompt());
    // Up to 16 rows (`MAX_TRANSFORM_FILENAME`).
    for i in 3..16 {
        edit(cx, &format!("/zz/{i}/"), "");
        press(cx, Op::Add);
    }
    assert_eq!(rows(cx).len(), 16);
    edit(cx, "/zz/17/", "");
    press(cx, Op::Add);
    assert_eq!(rows(cx).len(), 16);
    let (message, _) = cx.pending_prompt().expect("the list is full");
    assert_eq!(message, "これ以上登録できません。");
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    // 更新 does not need room.
    press(cx, Op::Update);
    assert_eq!(firsts(cx).last().map(String::as_str), Some("/zz/17/"));
}

#[gpui::test]
fn the_edit_boxes_take_259_characters(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_page(cx, FILE);
    let long = "あ".repeat(300);
    edit(cx, &long, "");
    press(cx, Op::Update);
    assert_eq!(rows(cx)[0][0].chars().count(), 259);
}

#[gpui::test]
fn the_alt_keys_of_the_labels_use_the_buttons_and_the_boxes(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_page(cx, FILE);
    edit(cx, "/zz/new/", "N:");
    // 追加(&A)
    assert!(with_dialog(cx, |d, w, dcx| d.use_access_key('a', w, dcx)));
    assert_eq!(firsts(cx).last().map(String::as_str), Some("/zz/new/"));
    // 先頭(&T)
    assert!(with_dialog(cx, |d, w, dcx| d.use_access_key('t', w, dcx)));
    assert_eq!(firsts(cx)[0], "/zz/new/");
    // 削除(&D)
    assert!(with_dialog(cx, |d, w, dcx| d.use_access_key('d', w, dcx)));
    assert_eq!(rows(cx).len(), 3);
    // 置換前(&F) is a label of a box: the key is used.
    assert!(with_dialog(cx, |d, w, dcx| d.use_access_key('f', w, dcx)));
}

#[gpui::test]
fn ok_writes_the_list_and_the_rest_of_the_file_stays(cx: &mut TestAppContext) {
    let (ini, _handle) = open_page(cx, FILE);
    // Delete the middle row, change the last one, add one.
    with_dialog(cx, |d, window, dcx| d.select_row(LIST, 1, window, dcx));
    press(cx, Op::Delete);
    edit(cx, "/zz/c2/", "C2:");
    press(cx, Op::Update);
    edit(cx, "/zz/日本語/", "日:");
    press(cx, Op::Add);
    ok(cx);
    let p = ini.profile();
    assert_eq!(p.get("Nickname", "ArrNum"), Some("3"));
    assert_eq!(p.get("Nickname", "From00"), Some("/zz/a/"));
    assert_eq!(p.get("Nickname", "To00"), Some("A:"));
    assert_eq!(p.get("Nickname", "From01"), Some("/zz/c2/"));
    assert_eq!(p.get("Nickname", "To01"), Some("C2:"));
    assert_eq!(p.get("Nickname", "From02"), Some("/zz/日本語/"));
    assert_eq!(p.get("Nickname", "To02"), Some("日:"));
    // The rest of the file is as it was.
    assert_eq!(p.get("Nickname", "bShortPath"), Some("0"));
    assert_eq!(p.get("Nickname", "nShortPathMaxWidth"), Some("60"));
    assert_eq!(p.get("Other", "x"), Some("1"));
}

#[gpui::test]
fn ok_removes_the_keys_of_the_rows_that_are_gone(cx: &mut TestAppContext) {
    let (ini, _handle) = open_page(cx, FILE);
    press(cx, Op::Delete);
    press(cx, Op::Delete);
    press(cx, Op::Delete);
    ok(cx);
    let p = ini.profile();
    assert_eq!(p.get("Nickname", "ArrNum"), Some("0"));
    assert_eq!(p.get("Nickname", "From00"), None);
    assert_eq!(p.get("Nickname", "To02"), None);
    assert_eq!(p.get("Other", "x"), Some("1"));
}

#[gpui::test]
fn the_edit_boxes_are_not_the_list_until_a_button_is_pressed(cx: &mut TestAppContext) {
    let (ini, _handle) = open_page(cx, FILE);
    edit(cx, "/zz/typed/", "T:");
    ok(cx);
    // Sakura's `GetData` reads the list, not the boxes.
    let p = ini.profile();
    assert_eq!(p.get("Nickname", "From00"), Some("/zz/a/"));
    assert_eq!(p.get("Nickname", "ArrNum"), Some("3"));
}

#[gpui::test]
fn cancel_writes_nothing_and_the_windows_keep_their_names(cx: &mut TestAppContext) {
    let (ini, _handle) = open_page(cx, FILE);
    let bytes = ini.bytes();
    let before = cx.update(|app| crate::file_names::current(app));
    press(cx, Op::Delete);
    edit(cx, "/zz/e/", "E:");
    press(cx, Op::Add);
    type_in(cx, "short_path_max_width", "5");
    cancel(cx);
    assert_eq!(ini.bytes(), bytes);
    assert_eq!(cx.update(|app| crate::file_names::current(app)), before);
}

#[gpui::test]
fn ok_makes_the_windows_show_their_paths_with_the_new_list(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_page(cx, FILE);
    let shown = |cx: &mut TestAppContext, path: &str| {
        cx.update(|app| crate::file_names::current(app).transform(path, true))
    };
    // The file's settings are installed when the application starts.
    assert_eq!(shown(cx, "/zz/a/x.txt"), "A:x.txt");
    assert_eq!(shown(cx, "/zz/b/x.txt"), "B:x.txt");
    press(cx, Op::Delete);
    edit(cx, "/zz/new/", "N:");
    press(cx, Op::Add);
    ok(cx);
    assert_eq!(shown(cx, "/zz/a/x.txt"), "/zz/a/x.txt");
    assert_eq!(shown(cx, "/ZZ/NEW/x.txt"), "N:x.txt");
}

#[gpui::test]
fn the_short_form_follows_the_check_box_and_the_number(cx: &mut TestAppContext) {
    let (_ini, handle) = open_page(cx, FILE);
    let long = "/qq/one/two/three/four/file.txt";
    let shown = |cx: &mut TestAppContext| {
        cx.update(|app| crate::file_names::current(app).transform(long, true))
    };
    // The file turns it off.
    assert_eq!(shown(cx), long);
    super::tests::check(cx, "short_path", true);
    type_in(cx, "short_path_max_width", "20");
    ok(cx);
    assert_eq!(shown(cx), "/.../four/file.txt");
    run(cx, handle, "F_OPTION_FNAME");
    super::tests::check(cx, "short_path", false);
    ok(cx);
    assert_eq!(shown(cx), long);
}

#[gpui::test]
fn the_page_is_the_list_and_the_two_settings(cx: &mut TestAppContext) {
    let (_ini, _handle) = open_page(cx, "");
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::FileName);
    let ids: Vec<&str> = with_dialog(cx, |d, _, _| {
        d.page_controls().iter().map(|c| c.id).collect()
    });
    assert_eq!(ids, ["short_path", "short_path_max_width", LIST]);
    assert!(with_dialog(cx, |d, _, _| d
        .page_controls()
        .iter()
        .all(|c| c.live)));
}

#[gpui::test]
fn the_title_formats_of_the_window_page_make_the_title(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Common]\r\nszWinCaptionInactive=inactive $F\r\n");
    let view = cx
        .update(|app| crate::file_ops::windows::workspace_of(app, handle))
        .unwrap()
        .read_with(cx, |ws, _| ws.editor().clone());
    let titles = |cx: &mut TestAppContext| {
        view.read_with(cx, |v, app| {
            (v.title_for(true, app), v.title_for(false, app))
        })
    };
    // The file's formats are in use from the start; the active one is Sakura's default.
    let (active, inactive) = titles(cx);
    assert_eq!(active, crate::file_names::title::default_title("(無題)1"));
    assert_eq!(inactive, "inactive (無題)");
    // The boxes are live and show the file.
    run(cx, handle, "F_OPTION_WINDOW");
    assert!(with_dialog(cx, |d, _, _| d
        .page_controls()
        .iter()
        .filter(|c| c.id.starts_with("win_caption"))
        .all(|c| c.live)));
    assert_eq!(
        with_dialog(cx, |d, _, _| d
            .values()
            .text("win_caption_inactive")
            .to_owned()),
        "inactive $F"
    );
    type_in(cx, "win_caption_active", "[$f] $g.$b");
    ok(cx);
    assert_eq!(
        ini.profile().get("Common", "szWinCaptionActive"),
        Some("[$f] $g.$b")
    );
    assert_eq!(titles(cx), ("[(無題)] (無題).".to_owned(), inactive));
}

#[gpui::test]
fn the_folder_of_the_settings_file_is_a_meta_folder_from_the_start(cx: &mut TestAppContext) {
    // At startup the settings are installed before the application has them as a global: the
    // replacement of `%SAKURADATA%` must still find its folder.
    let (ini, _handle) = setup(
        cx,
        "[Nickname]\r\nbShortPath=0\r\nArrNum=1\r\nFrom00=%SAKURADATA%\\\r\nTo00=設定:\r\n",
    );
    let file = ini.path.to_string_lossy().into_owned();
    let shown = cx.update(|app| crate::file_names::current(app).transform(&file, true));
    assert_eq!(shown, "設定:matsu.ini");
}
