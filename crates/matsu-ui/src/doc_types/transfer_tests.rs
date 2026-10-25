//! インポート and エクスポート of a type in the type list: the files, the question, and the result in
//! the settings file.

use gpui::{TestAppContext, VisualTestContext};
use matsu_config::Profile;
use matsu_config::settings::type_file;

use super::list_tests::{Ini, labels, open_list, select, selected, setup, types};
use super::tests::view_of;
use super::transfer::ColorChoice;

/// A type file as Sakura writes it, by hand, with the keyword file of its set.
const SAKURA_FILE: &str = "; タイプ別設定 Ver1\r\n\r\n\
    [Info]\r\nszVersion=2.4.2.0\r\nvStructureVersion=183\r\n\r\n\
    [TypeEx]\r\nszKeyword[1]=日本語の単語\r\nszKeywordCase[1]=1\r\nszKeywordFile[1]=日本語の単語.kwd\r\n\r\n\
    [Types]\r\nid=7\r\nnInts=9,90,0,2,-1,-1,0,0,0,0,0,0\r\nszTypeName=日本語タイプ\r\nszTypeExts=jp,nihongo\r\n\
    szLineComment=＃\r\nnLineCommentColumn=-1\r\nC[CMT]=1,0,0000ff,ffffff,0\r\n";

fn utf8(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// Writes the Sakura file and its keyword file in the folder of the test.
fn write_sakura_files(ini: &Ini) -> std::path::PathBuf {
    let file = ini.folder().join("jp.ini");
    std::fs::write(&file, utf8(SAKURA_FILE)).unwrap();
    std::fs::write(
        ini.folder().join("日本語の単語.kwd"),
        utf8("// 日本語の単語 キーワード定義ファイル\r\n// CASE=True\r\n\r\nりんご\r\n"),
    )
    .unwrap();
    file
}

#[gpui::test]
fn a_type_file_from_sakura_is_added_after_the_question(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    let file = write_sakura_files(&ini);
    assert!(list.update_in(cx, |l, window, lcx| l.import_chosen(&file, window, lcx)));
    cx.run_until_parked();
    // The question is open; the list has not changed yet. 新規追加 is chosen, and the colors come from the file.
    let (add, color, choices) = list.read_with(cx, |l, _| {
        let q = l.ascertain.as_ref().expect("the question");
        (q.add, q.color, q.choices().to_vec())
    });
    assert!(add);
    assert_eq!(color, 0);
    assert_eq!(choices[0].0, ColorChoice::AsFile);
    assert_eq!(choices[0].1, "--そのままインポート--");
    // The types follow (`name (extensions)`), then the color files of the folder: none here.
    assert_eq!(choices[1], (ColorChoice::OfType(0), "基本".to_owned()));
    assert_eq!(choices.len(), 1 + 36);
    assert_eq!(labels(&list, cx).len(), 36);
    list.update_in(cx, |l, window, lcx| l.ascertain_ok(window, lcx));
    cx.run_until_parked();
    assert!(list.read_with(cx, |l, _| l.ascertain.is_none()));
    let shown = labels(&list, cx);
    assert_eq!(shown.len(), 37);
    assert_eq!(shown[36], "日本語タイプ ( jp,nihongo )");
    assert_eq!(selected(&list, cx), 36);
    let t = types(&ini).get(36).unwrap().clone();
    assert_eq!(t.line_comments[0].delimiter, "＃");
    assert_eq!(ini.get("Other", "nTypesCount").as_deref(), Some("37"));
    // Its keyword set came with its words.
    let sets = types(&ini);
    let set = sets.keyword_sets_of(36);
    assert_eq!(set[0].1.name, "日本語の単語");
    assert!(set[0].1.contains("りんご"));
    // The report names the files (an information is open).
    assert!(cx.has_pending_prompt());
}

#[gpui::test]
fn a_type_file_can_replace_the_selected_type_with_the_colors_of_another(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    let file = write_sakura_files(&ini);
    select(&list, cx, 2);
    list.update_in(cx, |l, window, lcx| l.import_chosen(&file, window, lcx));
    cx.run_until_parked();
    // 基本's colors (the second choice is the first type), and the second radio button.
    list.update(cx, |l, _| {
        let q = l.ascertain.as_mut().unwrap();
        q.add = false;
        q.color = 1;
    });
    let basic_text = types(&ini).get(0).unwrap().colors.get("CMT").unwrap().text;
    list.update_in(cx, |l, window, lcx| l.ascertain_ok(window, lcx));
    cx.run_until_parked();
    let after = types(&ini);
    assert_eq!(after.len(), 36);
    let t = after.get(2).unwrap();
    assert_eq!(t.type_name, "日本語タイプ");
    // The colors are 基本's, the file's flag of showing stays.
    assert_eq!(t.colors.get("CMT").unwrap().text, basic_text);
    assert!(t.colors.get("CMT").unwrap().disp);
    assert_eq!(selected(&list, cx), 2);
    assert_eq!(labels(&list, cx)[2], "日本語タイプ ( jp,nihongo )");
}

#[gpui::test]
fn the_colors_may_come_from_a_color_file_next_to_the_type_file(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    let file = write_sakura_files(&ini);
    let mut colors = matsu_config::settings::ColorSet::default();
    colors.get_mut("CMT").unwrap().text = 0x12_3456;
    std::fs::write(
        ini.folder().join("mine.col"),
        type_file::render_colors(&colors),
    )
    .unwrap();
    std::fs::write(ini.folder().join("broken.col"), b"not a color file").unwrap();
    list.update_in(cx, |l, window, lcx| l.import_chosen(&file, window, lcx));
    cx.run_until_parked();
    let choices = list.read_with(cx, |l, _| l.ascertain.as_ref().unwrap().choices().to_vec());
    let files: Vec<_> = choices
        .iter()
        .filter(|(c, _)| matches!(c, ColorChoice::File(_)))
        .map(|(_, text)| text.as_str())
        .collect();
    assert_eq!(files, ["File -- broken.col", "File -- mine.col"]);
    let at = choices
        .iter()
        .position(|(c, _)| *c == ColorChoice::File("mine.col".into()))
        .unwrap();
    list.update(cx, |l, _| l.ascertain.as_mut().unwrap().color = at);
    list.update_in(cx, |l, window, lcx| l.ascertain_ok(window, lcx));
    cx.run_until_parked();
    let t = types(&ini).get(36).unwrap().clone();
    assert_eq!(t.colors.get("CMT").unwrap().text, 0x12_3456);
    // A file that is not a color file gives the colors of 基本.
    cx.simulate_prompt_answer("OK");
    list.update_in(cx, |l, window, lcx| l.import_chosen(&file, window, lcx));
    cx.run_until_parked();
    let choices = list.read_with(cx, |l, _| l.ascertain.as_ref().unwrap().choices().to_vec());
    let broken = choices
        .iter()
        .position(|(c, _)| *c == ColorChoice::File("broken.col".into()))
        .unwrap();
    list.update(cx, |l, _| l.ascertain.as_mut().unwrap().color = broken);
    list.update_in(cx, |l, window, lcx| l.ascertain_ok(window, lcx));
    cx.run_until_parked();
    let basic = types(&ini).get(0).unwrap().colors.get("CMT").unwrap().text;
    assert_eq!(
        types(&ini).get(37).unwrap().colors.get("CMT").unwrap().text,
        basic
    );
}

#[gpui::test]
fn the_import_does_not_replace_another_type_when_the_target_is_gone(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    let file = write_sakura_files(&ini);
    select(&list, cx, 20);
    list.update_in(cx, |l, window, lcx| l.import_chosen(&file, window, lcx));
    cx.run_until_parked();
    list.update(cx, |l, _| l.ascertain.as_mut().unwrap().add = false);
    // The type named in the question is deleted (by id 20) before the answer.
    list.update_in(cx, |l, window, lcx| l.delete_now(20, window, lcx));
    cx.run_until_parked();
    let names_before = types(&ini);
    list.update_in(cx, |l, window, lcx| l.ascertain_ok(window, lcx));
    cx.run_until_parked();
    assert_eq!(types(&ini), names_before);
    assert!(cx.has_pending_prompt());
}

#[gpui::test]
fn a_file_that_is_not_a_type_file_is_refused(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    let file = ini.folder().join("other.ini");
    std::fs::write(&file, utf8("[Types]\r\nszTypeName=A\r\n")).unwrap();
    assert!(!list.update_in(cx, |l, window, lcx| l.import_chosen(&file, window, lcx)));
    cx.run_until_parked();
    assert!(cx.has_pending_prompt());
    assert!(list.read_with(cx, |l, _| l.ascertain.is_none()));
    cx.simulate_prompt_answer("OK");
    // A file that is not there.
    let missing = ini.folder().join("missing.ini");
    assert!(!list.update_in(cx, |l, window, lcx| l.import_chosen(&missing, window, lcx)));
    cx.run_until_parked();
    assert!(cx.has_pending_prompt());
    assert_eq!(labels(&list, cx).len(), 36);
}

#[gpui::test]
fn another_version_of_the_structure_asks_whether_to_go_on(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    let file = ini.folder().join("old.ini");
    std::fs::write(
        &file,
        utf8(&SAKURA_FILE.replace("vStructureVersion=183", "vStructureVersion=100")),
    )
    .unwrap();
    assert!(list.update_in(cx, |l, window, lcx| l.import_chosen(&file, window, lcx)));
    cx.run_until_parked();
    assert!(cx.has_pending_prompt());
    assert!(list.read_with(cx, |l, _| l.ascertain.is_none()));
    // いいえ ends the import.
    cx.simulate_prompt_answer("いいえ(N)");
    cx.run_until_parked();
    assert!(list.read_with(cx, |l, _| l.ascertain.is_none()));
    list.update_in(cx, |l, window, lcx| l.import_chosen(&file, window, lcx));
    cx.run_until_parked();
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    assert!(list.read_with(cx, |l, _| l.ascertain.is_some()));
    // キャンセル of the question changes nothing.
    list.update(cx, |l, lcx| l.ascertain_cancel(lcx));
    assert_eq!(labels(&list, cx).len(), 36);
}

#[gpui::test]
fn a_type_is_exported_with_its_keyword_files_and_imported_again(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    select(&list, cx, 2);
    let target = ini.folder().join("C／C++.ini");
    assert!(list.update_in(cx, |l, window, lcx| l.export_to(&target, window, lcx)));
    cx.run_until_parked();
    let text = String::from_utf8(std::fs::read(&target).unwrap()[3..].to_vec()).unwrap();
    assert!(text.starts_with("; タイプ別設定 Ver1"), "{text}");
    assert!(text.contains("vStructureVersion=183"));
    assert!(text.contains("szKeywordFile[1]=C／C++.kwd"));
    // The keyword file is next to it, and is a keyword definition file.
    let words = std::fs::read(ini.folder().join("C／C++.kwd")).unwrap();
    assert!(words.starts_with(&[0xEF, 0xBB, 0xBF]));
    assert!(String::from_utf8(words).unwrap().contains("// CASE=True"));
    // The settings file did not change.
    assert_eq!(ini.get("Other", "nTypesCount"), None);
    // Import it as a new type.
    cx.simulate_prompt_answer("OK");
    list.update_in(cx, |l, window, lcx| l.import_chosen(&target, window, lcx));
    cx.run_until_parked();
    list.update_in(cx, |l, window, lcx| l.ascertain_ok(window, lcx));
    cx.run_until_parked();
    let after = types(&ini);
    assert_eq!(after.len(), 37);
    let (a, b) = (after.get(2).unwrap(), after.get(36).unwrap());
    assert_eq!(a.type_name, b.type_name);
    assert_eq!(a.type_exts, b.type_exts);
    assert_eq!(a.block_comments, b.block_comments);
    assert_eq!(a.keyword_set_idx, b.keyword_set_idx);
    assert_ne!(a.id, b.id);
    // Two types have the same name: Sakura does not ask.
    assert_eq!(after.index_of_name("C/C++"), Some(2));
}

#[gpui::test]
fn the_export_button_asks_for_the_name_of_the_type_and_the_import_folder(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    select(&list, cx, 13);
    crate::file_dialog::tests::asked();
    list.update_in(cx, |l, window, lcx| l.export(window, lcx));
    cx.run_until_parked();
    let request = crate::file_dialog::tests::asked().pop().expect("a dialog");
    assert_eq!(request.name.as_deref(), Some("Python.ini"));
    assert_eq!(request.filters[0].patterns, ["*.ini"]);
    assert_eq!(request.start.as_deref(), Some(ini.folder()));
    let target = ini.folder().join("py.ini");
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    assert!(target.exists());
    // The import dialog lists the same kind of files.
    list.update_in(cx, |l, window, lcx| l.import(window, lcx));
    cx.run_until_parked();
    let request = crate::file_dialog::tests::asked().pop().expect("a dialog");
    assert_eq!(request.filters[0].patterns, ["*.ini"]);
}

#[gpui::test]
fn a_type_file_in_shift_jis_is_read(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(
        "; タイプ別設定 Ver1\r\n[Info]\r\nvStructureVersion=183\r\n[Types]\r\nszTypeName=和文\r\nszTypeExts=wa\r\n",
    );
    let file = ini.folder().join("sjis.ini");
    std::fs::write(&file, &bytes).unwrap();
    list.update_in(cx, |l, window, lcx| l.import_chosen(&file, window, lcx));
    cx.run_until_parked();
    list.update_in(cx, |l, window, lcx| l.ascertain_ok(window, lcx));
    cx.run_until_parked();
    assert_eq!(labels(&list, cx)[36], "和文 ( wa )");
    let profile: Profile = ini.profile();
    assert_eq!(profile.get("Types(36)", "szTypeName"), Some("和文"));
}
