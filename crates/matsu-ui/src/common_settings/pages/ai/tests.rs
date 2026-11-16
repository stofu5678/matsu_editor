use gpui::TestAppContext;
use matsu_config::Profile;
use matsu_config::settings::ai::{AiDestinations, AiOutput};

use super::rows::{
    Fields, TEMPLATE_OPTIONS, TEMPLATES, apply_template, fields_of, register, rows_of, spec,
    table_of, template_of,
};
use super::{ASK, COMMAND, ID, LIST, NAME, OUTPUT, SELECTED, TEMPLATE, select};
use crate::common_settings::model::{Kind, controls};
use crate::common_settings::pages::oneliner::{Problem, index_of, problem};
use crate::common_settings::tests::{
    cancel, check, choose, ok, run, setup, type_in, visual, with_dialog,
};
use crate::common_settings::{Page, all_controls};

const INI: &str = "[Other]\r\nx=1\r\n\
[AI]\r\nName[000]=英訳\r\nCommand[000]=claude -p\r\nOutput[000]=1\r\nAskBeforeSend[000]=1\r\n\
Name[002]=\r\nCommand[002]=cat\r\nOutput[002]=2\r\nAskBeforeSend[002]=0\r\nFuture=keep\r\n\
[OneLiner]\r\nName[000]=整列\r\nCommand[000]=sort -u\r\nOutput[000]=0\r\n\
[Common]\r\nbDispSTATUSBAR=1\r\n";

fn profile(text: &str) -> Profile {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

fn fields(name: &str, command: &str, output: AiOutput, ask: bool) -> Fields {
    Fields {
        name: name.into(),
        command: command.into(),
        output,
        ask,
    }
}

// ---- the rows ----

#[test]
fn the_rows_show_the_destinations_and_read_back_as_they_were() {
    let table = AiDestinations::from_profile(&profile(INI));
    let rows = rows_of(&table);
    assert_eq!(rows.len(), 50);
    assert_eq!(
        rows[0],
        ["0", "英訳", "claude -p", "次の行に入れる", "する"]
    );
    // An empty slot shows the defaults of the file: the next line, and asking.
    assert_eq!(rows[1], ["1", "", "", "次の行に入れる", "する"]);
    assert_eq!(rows[2], ["2", "", "cat", "ログのタブだけ", "しない"]);
    assert_eq!(table_of(&rows), table);
}

#[test]
fn the_three_places_of_the_result_have_their_words() {
    let mut rows = rows_of(&AiDestinations::default());
    for (output, word) in [
        (AiOutput::Replace, "置き換える"),
        (AiOutput::NextLine, "次の行に入れる"),
        (AiOutput::LogOnly, "ログのタブだけ"),
    ] {
        register(&mut rows, 5, &fields("a", "x", output, true)).unwrap();
        assert_eq!(rows[5][3], word);
        assert_eq!(fields_of(&rows[5]).output, output);
    }
    // A text that is none of them reads as the next line, as the file does.
    rows[5][3] = "?".into();
    assert_eq!(fields_of(&rows[5]).output, AiOutput::NextLine);
}

#[test]
fn set_puts_the_fields_in_and_an_empty_command_empties_the_slot() {
    let mut rows = rows_of(&AiDestinations::from_profile(&profile(INI)));
    register(
        &mut rows,
        4,
        &fields("  要約 ", "codex exec -", AiOutput::Replace, false),
    )
    .unwrap();
    assert_eq!(
        rows[4],
        ["4", "要約", "codex exec -", "置き換える", "しない"]
    );
    assert_eq!(
        rows[0],
        ["0", "英訳", "claude -p", "次の行に入れる", "する"]
    );
    register(&mut rows, 0, &Fields::default()).unwrap();
    assert_eq!(rows[0], ["0", "", "", "次の行に入れる", "する"]);
    // A row past the list changes nothing.
    let before = rows.clone();
    register(&mut rows, 50, &fields("x", "y", AiOutput::Replace, true)).unwrap();
    assert_eq!(rows, before);
}

#[test]
fn the_name_and_command_limits_are_the_files_and_nothing_is_cut() {
    // 63 units for a name, 1023 for a command; an emoji is two units.
    assert_eq!(problem(&"a".repeat(63), &"b".repeat(1023)), None);
    let mut rows = rows_of(&AiDestinations::from_profile(&profile(INI)));
    let before = rows.clone();
    for (bad, why) in [
        (
            fields(&"a".repeat(64), "x", AiOutput::Replace, true),
            Problem::NameTooLong,
        ),
        (
            fields(&"😀".repeat(32), "x", AiOutput::Replace, true),
            Problem::NameTooLong,
        ),
        (
            fields("a", &"b".repeat(1024), AiOutput::Replace, true),
            Problem::CommandTooLong,
        ),
        (
            fields("a\nb", "x", AiOutput::Replace, true),
            Problem::NameBreak,
        ),
        (
            fields("a", "x\ny", AiOutput::Replace, true),
            Problem::CommandBreak,
        ),
        (
            fields("名前だけ", "", AiOutput::Replace, true),
            Problem::NameWithoutCommand,
        ),
    ] {
        assert_eq!(register(&mut rows, 0, &bad), Err(why));
        assert_eq!(rows, before);
    }
}

#[test]
fn the_id_box_is_a_number_of_the_list() {
    assert_eq!(index_of("7"), 7);
    assert_eq!(index_of("50"), 49);
    assert_eq!(index_of("abc"), 0);
}

// ---- the templates ----

#[test]
fn the_templates_carry_no_instruction_and_read_the_prompt_from_stdin() {
    assert_eq!(TEMPLATE_OPTIONS.len(), TEMPLATES.len() + 1);
    for (i, template) in TEMPLATES.iter().enumerate() {
        let number = i32::try_from(i + 1).unwrap();
        assert_eq!(template_of(number), Some(template));
        assert_eq!(TEMPLATE_OPTIONS[i + 1], (template.label, number));
        assert!(!template.command.contains(['\r', '\n']));
        // No instruction argument and no way for the text to reach an argument.
        assert!(!template.command.contains("ここに指示"));
        for bad in ["$(", "`", "<", "\"\" \""] {
            assert!(!template.command.contains(bad), "{bad}");
        }
        assert_eq!(problem(template.label, template.command), None);
    }
    assert_eq!(template_of(0), None);
    assert_eq!(template_of(3), None);
    assert_eq!(template_of(-1), None);
    let command = |n| template_of(n).unwrap().command;
    // Claude Code: `-p` with no prompt argument reads the standard input; the tools are off.
    assert!(command(1).starts_with("claude -p "));
    assert!(command(1).ends_with("--permission-mode dontAsk"));
    assert!(command(1).contains("--tools \"\""));
    assert!(command(1).contains("--no-session-persistence"));
    // Codex CLI: `-` is the prompt from the standard input.
    assert!(command(2).starts_with("codex exec "));
    assert!(command(2).ends_with(" -"));
    assert!(command(2).contains("--sandbox read-only"));
    assert!(command(2).contains("--ephemeral"));
    // Ollama is not offered: its documents do not show a prompt on the standard input.
    assert!(TEMPLATES.iter().all(|t| t.label != "Ollama"));
}

#[test]
fn a_template_sets_the_command_and_the_next_line_and_leaves_the_question() {
    let template = template_of(1).unwrap();
    let got = apply_template(&fields("", "old", AiOutput::Replace, false), template);
    assert_eq!(got.command, template.command);
    assert_eq!(got.output, AiOutput::NextLine);
    assert_eq!(got.name, "Claude Code");
    assert!(!got.ask);
    let got = apply_template(&fields("要約", "", AiOutput::LogOnly, true), template);
    assert_eq!(got.name, "要約");
    assert!(got.ask);
}

// ---- the controls ----

#[test]
fn the_page_comes_last_and_has_no_key_of_its_own() {
    assert_eq!(Page::Ai.title(), "生成 AI");
    assert_eq!(Page::ALL.last(), Some(&Page::Ai));
    assert_eq!(Page::ALL[Page::ALL.len() - 2], Page::OneLiner);
    let list = controls(&super::items());
    let ids: Vec<&str> = list.iter().map(|c| c.id).collect();
    for id in [
        NAME,
        ID,
        COMMAND,
        OUTPUT,
        ASK,
        TEMPLATE,
        "ai_register",
        "ai_put_template",
        LIST,
        SELECTED,
    ] {
        assert!(ids.contains(&id), "{id}");
    }
    assert!(list.iter().all(|c| c.live && c.key.is_empty()));
    assert!(list.iter().any(|c| matches!(c.kind, Kind::Table { .. })));
    assert!(all_controls().iter().any(|c| c.id == LIST));
}

#[test]
fn the_page_explains_the_two_modes() {
    // The explanation is a label of the page (ADR-0035, decision 7).
    assert!(super::items().iter().any(|item| matches!(
        item,
        crate::common_settings::model::Item::Label(t)
            if t.contains("モード a") && t.contains("モード b") && t.contains("標準入力")
    )));
}

// ---- the file ----

#[test]
fn the_list_is_stored_in_ai_and_unknown_keys_and_other_sections_stay() {
    let mut file = profile(INI);
    let mut rows = rows_of(&AiDestinations::from_profile(&file));
    register(&mut rows, 3, &fields("三", "tac", AiOutput::LogOnly, false)).unwrap();
    assert!(spec().store(&rows, &mut file).unwrap());
    assert_eq!(file.get("AI", "Name[003]"), Some("三"));
    assert_eq!(file.get("AI", "Command[003]"), Some("tac"));
    assert_eq!(file.get("AI", "Output[003]"), Some("2"));
    assert_eq!(file.get("AI", "AskBeforeSend[003]"), Some("0"));
    assert_eq!(file.get("AI", "Future"), Some("keep"));
    assert_eq!(file.get("Other", "x"), Some("1"));
    // The one-liners are not touched.
    assert_eq!(file.get("OneLiner", "Command[000]"), Some("sort -u"));
    // Read back, and storing what is there changes nothing.
    assert_eq!(spec().load(&file), rows);
    assert!(!spec().store(&rows, &mut file).unwrap());
    register(&mut rows, 0, &Fields::default()).unwrap();
    assert!(spec().store(&rows, &mut file).unwrap());
    assert_eq!(file.get("AI", "Command[000]"), None);
    assert_eq!(file.get("AI", "AskBeforeSend[000]"), None);
}

// ---- the dialog ----

fn open_page(cx: &mut TestAppContext, handle: gpui::AnyWindowHandle) {
    run(cx, handle, "F_OPTION");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Ai, dcx));
}

fn list(cx: &mut TestAppContext) -> Vec<Vec<String>> {
    with_dialog(cx, |d, _, _| d.values().rows(LIST).to_vec())
}

fn text(cx: &mut TestAppContext, id: &str) -> String {
    with_dialog(cx, |d, _, dcx| d.input_text(id, dcx).unwrap_or_default())
}

fn press_set(cx: &mut TestAppContext) {
    with_dialog(cx, super::register);
}

fn click(cx: &mut TestAppContext, row: usize) {
    with_dialog(cx, |d, window, dcx| select(d, row, window, dcx));
}

fn checked(cx: &mut TestAppContext, id: &str) -> bool {
    with_dialog(cx, |d, _, _| d.values().bool(id))
}

fn section(p: &Profile, name: &str) -> Vec<(String, String)> {
    p.entries(name)
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

#[gpui::test]
fn the_page_shows_the_stored_destinations_and_a_click_fills_the_fields(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::Ai);
    assert_eq!(list(cx)[0][1], "英訳");
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(SELECTED)), -1);
    click(cx, 2);
    assert_eq!(text(cx, ID), "2");
    assert_eq!(text(cx, COMMAND), "cat");
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(OUTPUT)), 2);
    assert!(!checked(cx, ASK));
    // Clicking it again keeps what was typed.
    type_in(cx, COMMAND, "cat -n");
    click(cx, 2);
    assert_eq!(text(cx, COMMAND), "cat -n");
    click(cx, 0);
    assert_eq!(text(cx, NAME), "英訳");
    assert!(checked(cx, ASK));
    cancel(cx);
}

#[gpui::test]
fn ok_writes_ai_and_keeps_the_rest(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    type_in(cx, ID, "9");
    type_in(cx, NAME, "要約");
    type_in(cx, COMMAND, "codex exec -");
    choose(cx, OUTPUT, 0);
    check(cx, ASK, false);
    press_set(cx);
    assert_eq!(
        list(cx)[9],
        ["9", "要約", "codex exec -", "置き換える", "しない"]
    );
    assert!(!cx.has_pending_prompt());
    ok(cx);
    let file = ini.profile();
    assert_eq!(file.get("AI", "Name[009]"), Some("要約"));
    assert_eq!(file.get("AI", "Command[009]"), Some("codex exec -"));
    assert_eq!(file.get("AI", "Output[009]"), Some("0"));
    assert_eq!(file.get("AI", "AskBeforeSend[009]"), Some("0"));
    assert_eq!(file.get("AI", "Command[000]"), Some("claude -p"));
    assert_eq!(file.get("AI", "Future"), Some("keep"));
    assert_eq!(file.get("OneLiner", "Command[000]"), Some("sort -u"));
    assert_eq!(file.get("Common", "bDispSTATUSBAR"), Some("1"));
    // The windows read the new destinations at once.
    let table = cx.update(|app| {
        app.global::<crate::settings::UserSettings>()
            .shared_ai_destinations()
    });
    assert_eq!(table.get(9).unwrap().command, "codex exec -");
    assert!(!table.get(9).unwrap().ask_before_send);
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
    let before = (
        section(&ini.profile(), "AI"),
        section(&ini.profile(), "OneLiner"),
    );
    open_page(cx, handle);
    ok(cx);
    let after = (
        section(&ini.profile(), "AI"),
        section(&ini.profile(), "OneLiner"),
    );
    assert_eq!(after, before);
}

#[gpui::test]
fn a_value_that_is_refused_is_told_and_the_row_stays(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    let before = list(cx);
    type_in(cx, ID, "4");
    type_in(cx, NAME, &"名".repeat(64));
    type_in(cx, COMMAND, "cat");
    press_set(cx);
    let (message, _) = cx.pending_prompt().expect("the refusal is told");
    assert!(message.contains("名前が長すぎます"), "{message}");
    cx.simulate_prompt_answer("OK");
    assert_eq!(list(cx), before);
    type_in(cx, NAME, "ok");
    type_in(cx, COMMAND, "a".repeat(1024).as_str());
    press_set(cx);
    let (message, _) = cx.pending_prompt().expect("the refusal is told");
    assert!(message.contains("コマンドが長すぎます"), "{message}");
    cx.simulate_prompt_answer("OK");
    assert_eq!(list(cx), before);
    // A name with no command.
    type_in(cx, COMMAND, "");
    press_set(cx);
    let (message, _) = cx.pending_prompt().expect("the refusal is told");
    assert!(message.contains("コマンド"), "{message}");
    cx.simulate_prompt_answer("OK");
    cancel(cx);
}

#[gpui::test]
fn the_template_button_fills_the_fields_and_cancel_writes_nothing(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    let before = ini.bytes();
    open_page(cx, handle);
    // Nothing chosen: nothing changes.
    type_in(cx, COMMAND, "keep me");
    with_dialog(cx, super::put_template);
    assert_eq!(text(cx, COMMAND), "keep me");
    for number in 1..=2 {
        choose(cx, TEMPLATE, number);
        with_dialog(cx, super::put_template);
        let template = template_of(number).unwrap();
        assert_eq!(text(cx, COMMAND), template.command);
        assert_eq!(text(cx, NAME), template.label);
        assert_eq!(with_dialog(cx, |d, _, _| d.values().int(OUTPUT)), 1);
        type_in(cx, NAME, "");
    }
    press_set(cx);
    assert!(!cx.has_pending_prompt());
    assert_eq!(list(cx)[0][3], "次の行に入れる");
    cancel(cx);
    assert_eq!(ini.bytes(), before);
}

#[gpui::test]
fn ok_writes_a_template_destination(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    type_in(cx, ID, "5");
    choose(cx, TEMPLATE, 2);
    with_dialog(cx, super::put_template);
    press_set(cx);
    ok(cx);
    let file = ini.profile();
    assert_eq!(file.get("AI", "Name[005]"), Some("Codex CLI"));
    assert_eq!(
        file.get("AI", "Command[005]"),
        Some(template_of(2).unwrap().command)
    );
    assert_eq!(file.get("AI", "Output[005]"), Some("1"));
    assert_eq!(file.get("AI", "AskBeforeSend[005]"), Some("1"));
}

#[gpui::test]
fn the_note_under_the_fields_draws_with_a_problem_in_the_boxes(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    type_in(cx, NAME, "名前だけ");
    let dialog_window = cx
        .update(|app| crate::common_settings::dialog_window(app))
        .unwrap();
    visual(cx, dialog_window).run_until_parked();
    cancel(cx);
}

#[gpui::test]
fn a_refused_value_does_not_change_the_rows_and_a_good_one_changes_one(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    let before = list(cx);
    type_in(cx, ID, "4");
    type_in(cx, NAME, "名前だけ");
    type_in(cx, COMMAND, "");
    press_set(cx);
    cx.simulate_prompt_answer("OK");
    assert_eq!(list(cx), before);
    type_in(cx, COMMAND, "cat");
    press_set(cx);
    let after = list(cx);
    assert_eq!(after[4][2], "cat");
    assert!(
        after
            .iter()
            .zip(&before)
            .enumerate()
            .all(|(i, (a, b))| i == 4 || a == b)
    );
    cancel(cx);
}
