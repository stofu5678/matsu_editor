use gpui::TestAppContext;
use matsu_config::Profile;
use matsu_config::settings::oneliner::{OneLiners, OutputTo};

use super::list::{AI, ASK, COMMAND, ID, LIST, NAME, OUTPUT, SELECTED, TEMPLATE, select};
use super::rows::{
    Fields, Problem, TEMPLATE_OPTIONS, TEMPLATES, apply_template, fields_of, index_of, problem,
    register, rows_of, spec, table_of, template_of,
};
use crate::common_settings::model::{Kind, controls};
use crate::common_settings::tests::{
    cancel, check, choose, ok, run, setup, type_in, visual, with_dialog,
};
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
        ..Fields::default()
    }
}

// ---- the rows ----

#[test]
fn the_rows_show_the_slots_and_read_back_as_they_were() {
    let table = OneLiners::from_profile(&profile(INI));
    let rows = rows_of(&table);
    assert_eq!(rows.len(), 50);
    assert_eq!(rows[0], ["0", "整列", "sort -u", "置き換える", "", ""]);
    assert_eq!(rows[1], ["1", "", "", "置き換える", "", ""]);
    assert_eq!(rows[2], ["2", "", "jq .", "アウトプットに出す", "", ""]);
    assert_eq!(rows[49][0], "49");
    assert_eq!(table_of(&rows, table.recent.clone()), table);
}

#[test]
fn the_places_of_the_result_have_their_words() {
    let mut rows = rows_of(&OneLiners::default());
    register(&mut rows, 5, &fields("a", "x", OutputTo::NextLine)).unwrap();
    assert_eq!(rows[5], ["5", "a", "x", "次の行に入れる", "", ""]);
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
    assert_eq!(rows[4], ["4", "並べ替え", "sort", "置き換える", "", ""]);
    // The other rows are as they were.
    assert_eq!(rows[0], ["0", "整列", "sort -u", "置き換える", "", ""]);
    // A row past the list changes nothing.
    let before = rows.clone();
    register(&mut rows, 50, &fields("x", "y", OutputTo::Replace)).unwrap();
    assert_eq!(rows, before);
}

#[test]
fn an_empty_command_with_no_name_empties_the_slot() {
    let mut rows = rows_of(&OneLiners::from_profile(&profile(INI)));
    register(&mut rows, 0, &fields("", "  ", OutputTo::OutputWindow)).unwrap();
    assert_eq!(rows[0], ["0", "", "", "置き換える", "", ""]);
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
    assert_eq!(rows[0], ["0", "整列", "sort -u", "置き換える", "", ""]);
    assert_eq!(rows[2], ["2", "", "jq .", "アウトプットに出す", "", ""]);
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
    assert_eq!(
        list(cx)[9],
        ["9", "九番", "sed 's/a/b/'", "次の行に入れる", "", ""]
    );
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
    assert_eq!(name(cx, 32763), "ワンライナーの設定...");
    cancel(cx);
}

// ---- generative AI (ADR-0030) ----

fn ai_fields(ai: bool, ask: bool) -> Fields {
    Fields {
        ai,
        ask,
        ..fields("", "cat", OutputTo::Replace)
    }
}

#[test]
fn the_ai_mark_and_the_question_are_in_the_rows_and_read_back() {
    let mut rows = rows_of(&OneLiners::default());
    register(&mut rows, 0, &ai_fields(true, true)).unwrap();
    register(&mut rows, 1, &ai_fields(true, false)).unwrap();
    // Not asking without the mark is kept in the row, so the page does not lose it.
    register(&mut rows, 2, &ai_fields(false, false)).unwrap();
    register(&mut rows, 3, &ai_fields(false, true)).unwrap();
    assert_eq!(rows[0][4..], ["○", "する"]);
    assert_eq!(rows[1][4..], ["○", "しない"]);
    assert_eq!(rows[2][4..], ["", "しない"]);
    assert_eq!(rows[3][4..], ["", ""]);
    for (i, (ai, ask)) in [(true, true), (true, false), (false, false), (false, true)]
        .into_iter()
        .enumerate()
    {
        assert_eq!(fields_of(&rows[i]), ai_fields(ai, ask), "{i}");
    }
    let table = table_of(&rows, Vec::new());
    assert!(table.slots[0].ai && table.slots[0].ask_before_send);
    assert!(table.slots[1].ai && !table.slots[1].ask_before_send);
    // An emptied slot has the defaults.
    register(&mut rows, 0, &Fields::default()).unwrap();
    assert_eq!(rows[0][4..], ["", ""]);
}

#[test]
fn the_templates_turn_the_tools_off_and_put_the_text_through_stdin() {
    assert_eq!(TEMPLATE_OPTIONS.len(), TEMPLATES.len() + 1);
    for (i, template) in TEMPLATES.iter().enumerate() {
        let number = i32::try_from(i + 1).unwrap();
        assert_eq!(template_of(number), Some(template));
        assert_eq!(TEMPLATE_OPTIONS[i + 1], (template.label, number));
        // One line, the instruction is a placeholder to replace, and the limits of the file hold.
        assert!(!template.command.contains(['\r', '\n']));
        assert!(template.command.contains("ここに指示を書く"));
        assert_eq!(problem(template.label, template.command), None);
    }
    assert_eq!(template_of(0), None);
    assert_eq!(template_of(4), None);
    assert_eq!(template_of(-1), None);
    let command = |n| template_of(n).unwrap().command;
    assert!(command(1).starts_with("claude -p "));
    assert!(command(1).contains("--tools \"\""));
    assert!(command(1).contains("--no-session-persistence"));
    // `--tools` takes many values: the instruction must not follow it directly.
    assert!(!command(1).contains("--tools \"\" \"ここ"));
    assert!(command(2).starts_with("codex exec "));
    assert!(command(2).contains("--sandbox read-only"));
    assert!(command(2).contains("--ephemeral"));
    assert!(command(3).starts_with("ollama run "));
    // The text is never an argument: no template reads a file or substitutes a command.
    for n in 1..=3 {
        assert!(!command(n).contains("$("), "{n}");
        assert!(!command(n).contains('`'), "{n}");
        assert!(!command(n).contains('<'), "{n}");
    }
}

#[test]
fn a_template_sets_the_command_the_next_line_and_the_ai_mark() {
    let template = template_of(1).unwrap();
    let got = apply_template(&ai_fields(false, false), template);
    assert_eq!(got.command, template.command);
    assert_eq!(got.output, OutputTo::NextLine);
    assert!(got.ai);
    // A name is given when there is none, and kept when there is one; the question is not touched.
    assert_eq!(got.name, "Claude Code");
    assert!(!got.ask);
    let named = Fields {
        name: "要約".into(),
        ..ai_fields(false, true)
    };
    let got = apply_template(&named, template);
    assert_eq!(got.name, "要約");
    assert!(got.ask);
}

#[test]
fn the_page_has_the_check_boxes_and_the_templates() {
    let list = controls(&super::items());
    for id in [AI, ASK, TEMPLATE, "oneliner_put_template"] {
        assert!(list.iter().any(|c| c.id == id), "{id}");
    }
    assert!(list.iter().all(|c| c.live && c.key.is_empty()));
}

fn ask_is_usable(cx: &mut TestAppContext) -> bool {
    with_dialog(cx, |d, _, _| {
        let control = controls(&super::items())
            .into_iter()
            .find(|c| c.id == ASK)
            .unwrap();
        d.is_usable(&control)
    })
}

fn checked(cx: &mut TestAppContext, id: &str) -> bool {
    with_dialog(cx, |d, _, _| d.values().bool(id))
}

#[gpui::test]
fn the_question_can_be_set_only_for_an_ai_one_liner(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    assert!(!checked(cx, AI));
    assert!(checked(cx, ASK), "asking is the default");
    assert!(!ask_is_usable(cx));
    check(cx, AI, true);
    assert!(ask_is_usable(cx));
    check(cx, AI, false);
    assert!(!ask_is_usable(cx));
    cancel(cx);
}

const AI_INI: &str = "[OneLiner]\r\nName[000]=尋ねる\r\nCommand[000]=claude -p\r\nOutput[000]=1\r\n\
AI[000]=1\r\nAskBeforeSend[000]=0\r\nName[001]=\r\nCommand[001]=sort\r\nOutput[001]=0\r\nFuture=keep\r\n";

#[gpui::test]
fn a_click_shows_the_ai_boxes_of_the_row_and_ok_writes_them(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, AI_INI);
    open_page(cx, handle);
    click(cx, 0);
    assert!(checked(cx, AI));
    assert!(!checked(cx, ASK));
    click(cx, 1);
    assert!(!checked(cx, AI));
    assert!(checked(cx, ASK));
    // Make slot 1 an AI one-liner that asks, and slot 0 one that does not need the AI mark.
    check(cx, AI, true);
    press_set(cx);
    assert_eq!(list(cx)[1][4..], ["○", "する"]);
    click(cx, 0);
    check(cx, AI, false);
    press_set(cx);
    ok(cx);
    let file = ini.profile();
    assert_eq!(file.get("OneLiner", "AI[001]"), Some("1"));
    assert_eq!(file.get("OneLiner", "AskBeforeSend[001]"), None);
    assert_eq!(file.get("OneLiner", "AI[000]"), None);
    // The question that was turned off stays in the file.
    assert_eq!(file.get("OneLiner", "AskBeforeSend[000]"), Some("0"));
    assert_eq!(file.get("OneLiner", "Future"), Some("keep"));
    let table = cx.update(|app| app.global::<crate::settings::UserSettings>().oneliners());
    assert!(table.get(1).unwrap().ai && table.get(1).unwrap().ask_before_send);
}

#[gpui::test]
fn ok_with_nothing_changed_leaves_the_ai_keys_as_they_are(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, AI_INI);
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
fn the_template_button_fills_the_fields_and_cancel_writes_nothing(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    let before = ini.bytes();
    open_page(cx, handle);
    // Nothing chosen: nothing changes.
    type_in(cx, COMMAND, "keep me");
    with_dialog(cx, super::list::put_template);
    assert_eq!(text(cx, COMMAND), "keep me");
    assert!(!checked(cx, AI));
    for number in 1..=3 {
        choose(cx, TEMPLATE, number);
        with_dialog(cx, super::list::put_template);
        let template = template_of(number).unwrap();
        assert_eq!(text(cx, COMMAND), template.command);
        assert_eq!(text(cx, NAME), template.label);
        assert_eq!(with_dialog(cx, |d, _, _| d.values().int(OUTPUT)), 1);
        assert!(checked(cx, AI));
        assert!(ask_is_usable(cx));
        type_in(cx, NAME, "");
    }
    // 設定 puts it in the list; the file is written only by OK.
    press_set(cx);
    assert!(!cx.has_pending_prompt());
    assert_eq!(list(cx)[0][4..], ["○", "する"]);
    assert_eq!(list(cx)[0][3], "次の行に入れる");
    cancel(cx);
    assert_eq!(ini.bytes(), before);
}

#[gpui::test]
fn ok_writes_a_template_one_liner_with_the_ai_mark(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    type_in(cx, ID, "5");
    choose(cx, TEMPLATE, 2);
    with_dialog(cx, super::list::put_template);
    press_set(cx);
    ok(cx);
    let file = ini.profile();
    assert_eq!(file.get("OneLiner", "Name[005]"), Some("Codex CLI"));
    assert_eq!(
        file.get("OneLiner", "Command[005]"),
        Some(template_of(2).unwrap().command)
    );
    assert_eq!(file.get("OneLiner", "Output[005]"), Some("1"));
    assert_eq!(file.get("OneLiner", "AI[005]"), Some("1"));
    assert_eq!(file.get("OneLiner", "AskBeforeSend[005]"), None);
}
