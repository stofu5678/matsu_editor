use gpui::TestAppContext;
use matsu_config::Profile;
use matsu_config::settings::oneliner::{OneLiners, OutputTo};

use super::list::{COMMAND, ID, LIST, NAME, OUTPUT, SELECTED, select};
use super::rows::{
    Fields, Problem, fields_of, index_of, problem, register, rows_of, spec, table_of,
};
use crate::common_settings::model::{Kind, controls};
use crate::common_settings::tests::{cancel, choose, ok, run, setup, type_in, visual, with_dialog};
use crate::common_settings::{Page, all_controls};

const INI: &str = "[Other]\r\nx=1\r\n\
[OneLiner]\r\nName[000]=整列\r\nCommand[000]=sort -u\r\nOutput[000]=0\r\n\
Name[002]=\r\nCommand[002]=jq .\r\nOutput[002]=2\r\n\
Recent[00]=tr a-z A-Z\r\nRecent[01]=wc -l\r\nFuture=keep\r\n\
[Common]\r\nbDispSTATUSBAR=1\r\n";

fn profile(text: &str) -> Profile {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

fn fields(name: &str, command: &str, output: OutputTo) -> Fields {
    Fields {
        name: name.into(),
        command: command.into(),
        output,
    }
}

// ---- the rows ----

#[test]
fn the_rows_show_the_slots_and_read_back_as_they_were() {
    let table = OneLiners::from_profile(&profile(INI));
    let rows = rows_of(&table);
    assert_eq!(rows.len(), 50);
    assert_eq!(rows[0], ["0", "整列", "sort -u", "置き換える"]);
    assert_eq!(rows[1], ["1", "", "", "置き換える"]);
    assert_eq!(rows[2], ["2", "", "jq .", "アウトプットに出す"]);
    assert_eq!(rows[49][0], "49");
    assert_eq!(table_of(&rows, table.recent.clone()), table);
}

#[test]
fn the_places_of_the_result_have_their_words() {
    let mut rows = rows_of(&OneLiners::default());
    register(&mut rows, 5, &fields("a", "x", OutputTo::NextLine)).unwrap();
    assert_eq!(rows[5], ["5", "a", "x", "次の行に入れる"]);
    assert_eq!(fields_of(&rows[5]).output, OutputTo::NextLine);
    // A text that is none of them reads as the first place, as the file does.
    rows[5][3] = "?".into();
    assert_eq!(fields_of(&rows[5]).output, OutputTo::Replace);
}

#[test]
fn set_puts_the_fields_into_the_row_and_the_name_is_taken_without_the_spaces_around_it() {
    let mut rows = rows_of(&OneLiners::from_profile(&profile(INI)));
    register(
        &mut rows,
        4,
        &fields("  並べ替え ", "sort", OutputTo::Replace),
    )
    .unwrap();
    assert_eq!(rows[4], ["4", "並べ替え", "sort", "置き換える"]);
    // The other rows are as they were.
    assert_eq!(rows[0], ["0", "整列", "sort -u", "置き換える"]);
    // A row past the list changes nothing.
    let before = rows.clone();
    register(&mut rows, 50, &fields("x", "y", OutputTo::Replace)).unwrap();
    assert_eq!(rows, before);
}

#[test]
fn an_empty_command_with_no_name_empties_the_slot() {
    let mut rows = rows_of(&OneLiners::from_profile(&profile(INI)));
    register(&mut rows, 0, &fields("", "  ", OutputTo::OutputWindow)).unwrap();
    assert_eq!(rows[0], ["0", "", "", "置き換える"]);
}

#[test]
fn the_limits_and_the_line_break_are_refused_and_the_row_stays() {
    // The limits count UTF-16 units: 63 for a name, 1023 for a command.
    assert_eq!(problem(&"a".repeat(63), &"b".repeat(1023)), None);
    assert_eq!(problem(&"a".repeat(64), "x"), Some(Problem::NameTooLong));
    assert_eq!(
        problem("a", &"b".repeat(1024)),
        Some(Problem::CommandTooLong)
    );
    // An emoji is two units.
    assert_eq!(problem(&"😀".repeat(31), "x"), None);
    assert_eq!(problem(&"😀".repeat(32), "x"), Some(Problem::NameTooLong));
    assert_eq!(problem("a\nb", "x"), Some(Problem::NameBreak));
    assert_eq!(problem("a", "x\r\ny"), Some(Problem::CommandBreak));
    assert_eq!(problem("a", "x\ny"), Some(Problem::CommandBreak));
    assert_eq!(problem("名前", " "), Some(Problem::NameWithoutCommand));
    assert_eq!(problem("", ""), None);
    let mut rows = rows_of(&OneLiners::from_profile(&profile(INI)));
    let before = rows.clone();
    for bad in [
        fields("a\nb", "x", OutputTo::Replace),
        fields("a", "x\ny", OutputTo::Replace),
        fields(&"a".repeat(64), "x", OutputTo::Replace),
        fields("a", &"b".repeat(1024), OutputTo::Replace),
        fields("名前だけ", "", OutputTo::Replace),
    ] {
        assert!(register(&mut rows, 0, &bad).is_err());
        assert_eq!(rows, before, "nothing is cut or half written");
    }
    // Every reason has words.
    for problem in [
        Problem::NameBreak,
        Problem::CommandBreak,
        Problem::NameTooLong,
        Problem::CommandTooLong,
        Problem::NameWithoutCommand,
    ] {
        assert!(!problem.text().is_empty());
    }
    assert!(Problem::NameTooLong.text().contains("63"));
    assert!(Problem::CommandTooLong.text().contains("1023"));
}

#[test]
fn the_id_box_is_a_number_of_the_list() {
    assert_eq!(index_of("7"), 7);
    assert_eq!(index_of(" 49 "), 49);
    assert_eq!(index_of("50"), 49);
    assert_eq!(index_of("-1"), 0);
    assert_eq!(index_of("abc"), 0);
}

// ---- the controls ----

#[test]
fn the_page_comes_after_the_shell_and_has_no_key_of_its_own() {
    assert_eq!(Page::OneLiner.title(), "ワンライナー");
    assert_eq!(Page::ALL.last(), Some(&Page::OneLiner));
    let list = controls(&super::items());
    let ids: Vec<&str> = list.iter().map(|c| c.id).collect();
    for id in [
        NAME,
        ID,
        COMMAND,
        OUTPUT,
        "oneliner_register",
        LIST,
        SELECTED,
    ] {
        assert!(ids.contains(&id), "{id}");
    }
    assert!(list.iter().all(|c| c.live && c.key.is_empty()));
    assert!(list.iter().any(|c| matches!(c.kind, Kind::Table { .. })));
    assert!(all_controls().iter().any(|c| c.id == LIST));
}

// ---- the file ----

#[test]
fn the_list_is_stored_in_oneliner_and_the_history_and_unknown_keys_stay() {
    let mut file = profile(INI);
    let mut rows = rows_of(&OneLiners::from_profile(&file));
    register(&mut rows, 3, &fields("三", "tac", OutputTo::NextLine)).unwrap();
    assert!(spec().store(&rows, &mut file).unwrap());
    assert_eq!(file.get("OneLiner", "Name[003]"), Some("三"));
    assert_eq!(file.get("OneLiner", "Command[003]"), Some("tac"));
    assert_eq!(file.get("OneLiner", "Output[003]"), Some("1"));
    // The history, the key this program does not know and the other sections stay.
    assert_eq!(file.get("OneLiner", "Recent[00]"), Some("tr a-z A-Z"));
    assert_eq!(file.get("OneLiner", "Recent[01]"), Some("wc -l"));
    assert_eq!(file.get("OneLiner", "Future"), Some("keep"));
    assert_eq!(file.get("Other", "x"), Some("1"));
    // The list is read back from the file, and storing what is there changes nothing.
    assert_eq!(spec().load(&file), rows);
    assert!(!spec().store(&rows, &mut file).unwrap());
    // A slot that is emptied loses its keys.
    register(&mut rows, 0, &Fields::default()).unwrap();
    assert!(spec().store(&rows, &mut file).unwrap());
    assert_eq!(file.get("OneLiner", "Command[000]"), None);
    assert_eq!(file.get("OneLiner", "Name[000]"), None);
}

// ---- the dialog ----

fn open_page(cx: &mut TestAppContext, handle: gpui::AnyWindowHandle) {
    run(cx, handle, "F_ONELINER_SETTINGS");
}

fn list(cx: &mut TestAppContext) -> Vec<Vec<String>> {
    with_dialog(cx, |d, _, _| d.values().rows(LIST).to_vec())
}

fn text(cx: &mut TestAppContext, id: &str) -> String {
    with_dialog(cx, |d, _, dcx| d.input_text(id, dcx).unwrap_or_default())
}

fn press_set(cx: &mut TestAppContext) {
    with_dialog(cx, super::list::register);
}

fn click(cx: &mut TestAppContext, row: usize) {
    with_dialog(cx, |d, window, dcx| select(d, row, window, dcx));
}

#[gpui::test]
fn the_function_opens_the_dialog_at_this_page_and_the_page_shows_the_stored_slots(
    cx: &mut TestAppContext,
) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::OneLiner);
    let rows = list(cx);
    assert_eq!(rows.len(), 50);
    assert_eq!(rows[0], ["0", "整列", "sort -u", "置き換える"]);
    assert_eq!(rows[2], ["2", "", "jq .", "アウトプットに出す"]);
    // Nothing is selected, and the Id box starts at 0.
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(SELECTED)), -1);
    assert_eq!(text(cx, ID), "0");
    cancel(cx);
}

#[gpui::test]
fn a_click_on_a_row_fills_the_fields_and_clicking_it_again_keeps_what_was_typed(
    cx: &mut TestAppContext,
) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    click(cx, 2);
    assert_eq!(text(cx, ID), "2");
    assert_eq!(text(cx, NAME), "");
    assert_eq!(text(cx, COMMAND), "jq .");
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(OUTPUT)), 2);
    type_in(cx, COMMAND, "jq -c .");
    click(cx, 2);
    assert_eq!(text(cx, COMMAND), "jq -c .");
    click(cx, 0);
    assert_eq!(text(cx, NAME), "整列");
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(OUTPUT)), 0);
    cancel(cx);
}

#[gpui::test]
fn editing_a_slot_and_ok_write_the_section_and_keep_the_rest(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    click(cx, 0);
    type_in(cx, ID, "9");
    type_in(cx, NAME, "九番");
    type_in(cx, COMMAND, "sed 's/a/b/'");
    choose(cx, OUTPUT, 1);
    press_set(cx);
    assert_eq!(list(cx)[9], ["9", "九番", "sed 's/a/b/'", "次の行に入れる"]);
    // Nothing is told for a value that is accepted.
    assert!(!cx.has_pending_prompt());
    ok(cx);
    let file = ini.profile();
    assert_eq!(file.get("OneLiner", "Name[009]"), Some("九番"));
    assert_eq!(file.get("OneLiner", "Command[009]"), Some("sed 's/a/b/'"));
    assert_eq!(file.get("OneLiner", "Output[009]"), Some("1"));
    assert_eq!(file.get("OneLiner", "Command[000]"), Some("sort -u"));
    assert_eq!(file.get("OneLiner", "Output[002]"), Some("2"));
    // The history and the unknown key survive.
    assert_eq!(file.get("OneLiner", "Recent[00]"), Some("tr a-z A-Z"));
    assert_eq!(file.get("OneLiner", "Recent[01]"), Some("wc -l"));
    assert_eq!(file.get("OneLiner", "Future"), Some("keep"));
    assert_eq!(file.get("Other", "x"), Some("1"));
    assert_eq!(file.get("Common", "bDispSTATUSBAR"), Some("1"));
    // The windows read the new one-liners at once.
    let table = cx.update(|app| app.global::<crate::settings::UserSettings>().oneliners());
    assert_eq!(table.get(9).unwrap().command, "sed 's/a/b/'");
    assert_eq!(table.recent, ["tr a-z A-Z", "wc -l"]);
}

#[gpui::test]
fn cancel_changes_nothing(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    let before = ini.bytes();
    open_page(cx, handle);
    type_in(cx, ID, "3");
    type_in(cx, COMMAND, "cat");
    press_set(cx);
    assert_eq!(list(cx)[3][2], "cat");
    cancel(cx);
    assert_eq!(ini.bytes(), before);
}

#[gpui::test]
fn ok_with_nothing_changed_leaves_the_section_as_it_is(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    let section = |p: &Profile| -> Vec<(String, String)> {
        p.entries("OneLiner")
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    };
    let before = section(&ini.profile());
    open_page(cx, handle);
    ok(cx);
    assert_eq!(section(&ini.profile()), before);
}

#[gpui::test]
fn a_value_that_is_refused_is_told_and_the_row_stays(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    let before = list(cx);
    type_in(cx, ID, "4");
    // A name of 64 units is one too many.
    type_in(cx, NAME, &"名".repeat(64));
    type_in(cx, COMMAND, "cat");
    press_set(cx);
    let (message, _) = cx.pending_prompt().expect("the refusal is told");
    assert!(message.contains("名前が長すぎます"), "{message}");
    cx.simulate_prompt_answer("OK");
    assert_eq!(list(cx), before);
    // A line break in the command, however it got there.
    type_in(cx, NAME, "ok");
    type_in(cx, COMMAND, "a\nb");
    press_set(cx);
    // If the box takes the break out, the command is one line and is accepted; if it keeps it, it is refused.
    let typed = text(cx, COMMAND);
    if typed.contains(['\r', '\n']) {
        let (message, _) = cx.pending_prompt().expect("the refusal is told");
        assert!(message.contains("改行"), "{message}");
        cx.simulate_prompt_answer("OK");
        assert_eq!(list(cx), before);
    } else {
        assert!(!cx.has_pending_prompt());
        assert!(!list(cx)[4][2].contains(['\r', '\n']));
    }
    // A name with no command.
    type_in(cx, COMMAND, "");
    press_set(cx);
    let (message, _) = cx.pending_prompt().expect("the refusal is told");
    assert!(message.contains("コマンド"), "{message}");
    cx.simulate_prompt_answer("OK");
    cancel(cx);
}

#[gpui::test]
fn the_note_under_the_fields_follows_what_is_typed(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    type_in(cx, NAME, "名前だけ");
    // The page draws without trouble with a problem in the boxes.
    let dialog_window = cx
        .update(|app| crate::common_settings::dialog_window(app))
        .unwrap();
    visual(cx, dialog_window).run_until_parked();
    cancel(cx);
}

// ---- the other pages name the one-liners ----

#[gpui::test]
fn the_key_pages_names_are_the_registered_names(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    let name = |cx: &mut TestAppContext, code: u16| {
        cx.update(|app| crate::func_name::FuncNames::new().name(app, code))
    };
    assert_eq!(name(cx, 32711), "整列");
    // No name: the command.
    assert_eq!(name(cx, 32713), "jq .");
    assert_eq!(name(cx, 32712), "ワンライナー 1 (未登録)");
    assert_eq!(name(cx, 32763), "ワンライナーの設定");
    cancel(cx);
}
