//! The one-liners that send to a generative AI (ADR-0030): the confirmation, the working folder and the log, with
//! real shells as stand-ins for the AI tool. Nothing here runs an AI tool or sends anything anywhere.

use gpui::{Entity, TestAppContext};
use matsu_config::settings::oneliner::OutputTo;
use matsu_core::buffer::Pos;

use super::{COPY, FAIL, LONG, UPPER, Win, ini, run, wait_until};
use crate::ai_confirm::AiConfirmDialog;
use crate::app::Workspace;

/// A window as the app has it: with the key bindings of the editor, among them the one that lets Tab reach the
/// dialogs (`Root` binds it to a focus move otherwise).
fn open(cx: &mut TestAppContext, settings: &str, text: &str) -> (tempfile::TempDir, Win) {
    cx.update(gpui_component::init);
    cx.update(crate::view::EditorView::bind_keys);
    super::install_settings(cx, settings, None);
    super::open_in(cx, text)
}

/// A table of one AI one-liner named `test`.
fn ai_ini(command: &str, output: OutputTo, ask: bool) -> String {
    // `ini` ends with the last line of the section, so the keys can follow it
    let mut text = ini(&[("test", command, output.number())], &[]);
    text.push_str("AI[000]=1\r\n");
    if !ask {
        text.push_str("AskBeforeSend[000]=0\r\n");
    }
    text
}

/// What the log of the window has for one record.
#[derive(Debug)]
struct Rec {
    name: String,
    command_line: String,
    input: String,
    /// The answer, the stderr and the exit code, once the record has ended.
    end: Option<(String, String, Option<i32>)>,
}

fn workspace(w: &Win, cx: &mut TestAppContext) -> Option<Entity<Workspace>> {
    w.view
        .read_with(cx, |view, _| view.split_host.clone())
        .and_then(|host| host.upgrade())
}

fn records(w: &Win, cx: &mut TestAppContext) -> Vec<Rec> {
    let Some(ws) = workspace(w, cx) else {
        return Vec::new();
    };
    let Some(log) = ws.read_with(cx, |ws, _| ws.ai_log_of_window()) else {
        return Vec::new();
    };
    log.read_with(cx, |view, _| {
        view.log()
            .records()
            .map(|record| Rec {
                name: record.name.clone(),
                command_line: record.command_line.as_str().to_owned(),
                input: record.input.as_str().to_owned(),
                end: record.result.as_ref().map(|r| {
                    (
                        r.output.as_str().to_owned(),
                        r.stderr.as_str().to_owned(),
                        r.exit_code,
                    )
                }),
            })
            .collect()
    })
}

fn confirm(w: &Win, cx: &mut TestAppContext) -> Option<Entity<AiConfirmDialog>> {
    w.view.read_with(cx, |view, _| view.ai_confirm.clone())
}

fn wait_confirm(w: &Win, cx: &mut TestAppContext) -> Entity<AiConfirmDialog> {
    wait_until(cx, "the confirmation window", |cx| confirm(w, cx).is_some());
    confirm(w, cx).unwrap()
}

fn press(w: &Win, cx: &mut TestAppContext, keys: &str) {
    w.cx(cx).simulate_keystrokes(keys);
    cx.run_until_parked();
}

fn ask_before_send(cx: &mut TestAppContext) -> bool {
    cx.update(|app| super::super::table(app)).slots[0].ask_before_send
}

fn wait_idle(w: &Win, cx: &mut TestAppContext) {
    wait_until(cx, "the end of the run", |cx| {
        !w.running(cx) && !w.view.read_with(cx, |v, _| v.support.oneliner_starting)
    });
    cx.run_until_parked();
}

#[gpui::test]
fn the_window_is_asked_with_the_name_the_command_line_and_the_range(cx: &mut TestAppContext) {
    let (_dir, w) = open(
        cx,
        &ai_ini(UPPER, OutputTo::NextLine, true),
        "abc\ndef\nghi\n",
    );
    w.select(cx, Pos::new(1, 0), Pos::new(2, 3));
    w.slot(cx, 0);
    let dialog = wait_confirm(&w, cx);
    dialog.read_with(cx, |d, _| {
        let request = d.request();
        assert_eq!(request.name, "test");
        assert_eq!(request.command_line, UPPER);
        assert_eq!(request.preview.lines, 2);
        assert_eq!(request.preview.chars, "def\nghi".chars().count());
        let text: Vec<_> = request
            .preview
            .head
            .iter()
            .map(|l| l.text.as_str())
            .collect();
        assert_eq!(text, ["def", "ghi"]);
    });
    // nothing has run yet and nothing is logged
    assert!(!w.running(cx));
    assert!(records(&w, cx).is_empty());
    press(&w, cx, "escape");
    wait_idle(&w, cx);
}

#[gpui::test]
fn the_window_is_asked_over_the_whole_document_when_nothing_is_selected(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(UPPER, OutputTo::NextLine, true), "abc\ndef\n");
    w.slot(cx, 0);
    let dialog = wait_confirm(&w, cx);
    dialog.read_with(cx, |d, _| {
        assert_eq!(d.request().preview.lines, 2);
        assert_eq!(d.request().preview.chars, "abc\ndef\n".len());
    });
    press(&w, cx, "escape");
    wait_idle(&w, cx);
}

#[gpui::test]
fn not_sending_runs_nothing_and_changes_nothing(cx: &mut TestAppContext) {
    for keys in ["escape", "tab enter"] {
        let (_dir, w) = open(cx, &ai_ini(UPPER, OutputTo::Replace, true), "abc\n");
        w.slot(cx, 0);
        wait_confirm(&w, cx);
        press(&w, cx, keys);
        wait_idle(&w, cx);
        assert_eq!(w.text(cx), "abc\n", "{keys}");
        assert!(confirm(&w, cx).is_none());
        assert!(
            w.notice(cx).is_some_and(|n| n.contains("送らない")),
            "{keys}: {:?}",
            w.notice(cx)
        );
        // the log says that nothing was sent
        let records = records(&w, cx);
        assert_eq!(records.len(), 1, "{keys}");
        let (output, stderr, code) = records[0].end.clone().expect("the record has ended");
        assert_eq!((output.as_str(), code), ("", None));
        assert!(stderr.contains("送りませんでした"), "{stderr}");
        // asking stays on
        assert!(ask_before_send(cx));
    }
}

#[gpui::test]
fn sending_runs_and_records(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(UPPER, OutputTo::NextLine, true), "abc\ndef\n");
    w.select(cx, Pos::new(0, 0), Pos::new(1, 0));
    w.slot(cx, 0);
    wait_confirm(&w, cx);
    press(&w, cx, "enter");
    wait_idle(&w, cx);
    // 次の行に入れる: after the selected line
    assert_eq!(w.text(cx), "abc\nABC\ndef\n");
    let records = records(&w, cx);
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.name, "test");
    assert_eq!(record.command_line, UPPER);
    assert_eq!(record.input, "abc\n");
    let (output, stderr, code) = record.end.clone().expect("the record has ended");
    assert_eq!(output.replace("\r\n", "\n"), "ABC\n");
    assert_eq!((stderr.as_str(), code), ("", Some(0)));
    // asking stays on when the box was not checked
    assert!(ask_before_send(cx));
}

#[gpui::test]
fn the_log_tab_opens_behind_the_document(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(COPY, OutputTo::NextLine, false), "abc\n");
    w.slot(cx, 0);
    wait_idle(&w, cx);
    let ws = workspace(&w, cx).unwrap();
    ws.read_with(cx, |ws, _| {
        assert_eq!(ws.tab_count(), 2);
        assert_eq!(ws.active_tab(), 0, "the document keeps the focus");
        assert!(ws.ai_log_of_window().is_some());
    });
}

#[gpui::test]
fn dont_ask_again_writes_the_setting_and_the_next_run_does_not_ask(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(UPPER, OutputTo::NextLine, true), "abc\n");
    w.slot(cx, 0);
    wait_confirm(&w, cx);
    // 送る -> (Shift+Tab) the check box, Space toggles it, Tab back to 送る
    press(&w, cx, "shift-tab space tab enter");
    wait_idle(&w, cx);
    assert_eq!(w.text(cx), "abc\nABC\n");
    assert!(!ask_before_send(cx));
    assert_eq!(records(&w, cx).len(), 1);

    // the next run goes at once
    w.slot(cx, 0);
    wait_idle(&w, cx);
    assert!(confirm(&w, cx).is_none());
    assert_eq!(w.text(cx), "abc\nABC\nABC\n");
    assert_eq!(records(&w, cx).len(), 2);
}

#[gpui::test]
fn dont_ask_again_is_written_to_the_settings_file(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("matsu.ini");
    std::fs::write(&file, ai_ini(UPPER, OutputTo::NextLine, true)).unwrap();
    let user = crate::settings::UserSettings::from_profile(
        matsu_config::Profile::load(&file).unwrap(),
        Some(file.clone()),
    );
    cx.update(|app| {
        crate::history::install(app, &user);
        app.set_global(user);
    });
    cx.update(crate::view::EditorView::bind_keys);
    let (_doc_dir, w) = super::open_in(cx, "abc\n");
    w.slot(cx, 0);
    wait_confirm(&w, cx);
    press(&w, cx, "shift-tab space tab enter");
    wait_idle(&w, cx);
    let saved = String::from_utf8_lossy(&std::fs::read(&file).unwrap()).into_owned();
    assert!(saved.contains("AskBeforeSend[000]=0"), "{saved}");
    assert!(saved.contains("AI[000]=1"), "{saved}");
}

#[gpui::test]
fn not_sending_with_the_box_checked_keeps_asking(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(UPPER, OutputTo::NextLine, true), "abc\n");
    w.slot(cx, 0);
    wait_confirm(&w, cx);
    // check the box, then 送らない
    press(&w, cx, "shift-tab space tab tab enter");
    wait_idle(&w, cx);
    assert_eq!(w.text(cx), "abc\n");
    assert!(ask_before_send(cx));
}

#[gpui::test]
fn a_document_that_changed_during_the_question_is_not_run(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(UPPER, OutputTo::Replace, true), "abc\n");
    w.slot(cx, 0);
    wait_confirm(&w, cx);
    w.view.update(cx, |view, cx| {
        view.document()
            .clone()
            .update(cx, |shared, cx| shared.set_view_mode(true, cx));
    });
    press(&w, cx, "enter");
    wait_idle(&w, cx);
    assert_eq!(w.text(cx), "abc\n");
    assert!(records(&w, cx).is_empty(), "nothing was started");
    assert!(w.notice(cx).is_some_and(|n| n.contains("変わった")));
}

#[gpui::test]
fn without_asking_it_runs_at_once_and_records(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(UPPER, OutputTo::Replace, false), "abc\n");
    run(cx, &w, 0);
    assert!(confirm(&w, cx).is_none());
    assert_eq!(w.text(cx), "ABC\n");
    let records = records(&w, cx);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].input, "abc\n");
    assert_eq!(records[0].end.as_ref().unwrap().2, Some(0));
}

#[gpui::test]
fn the_working_folder_is_a_new_empty_temporary_folder_that_is_removed(cx: &mut TestAppContext) {
    let command = if cfg!(windows) {
        "(Get-Location).Path; (Get-ChildItem -Force | Measure-Object).Count"
    } else {
        "pwd; ls -A | wc -l"
    };
    let (dir, w) = open(cx, &ai_ini(command, OutputTo::OutputWindow, false), "abc\n");
    run(cx, &w, 0);
    let (output, _, code) = records(&w, cx)[0].end.clone().unwrap();
    assert_eq!(code, Some(0));
    let mut lines = output.lines();
    let folder = std::path::PathBuf::from(lines.next().unwrap().trim());
    assert_eq!(lines.next().unwrap().trim(), "0", "the folder is empty");
    assert!(
        folder
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("matsu-ai-"),
        "{folder:?}"
    );
    assert_ne!(
        folder.canonicalize().ok(),
        dir.path().canonicalize().ok(),
        "not the document's folder"
    );
    assert!(!folder.exists(), "removed after the run: {folder:?}");
}

#[gpui::test]
fn a_plain_one_liner_keeps_the_document_folder_and_neither_asks_nor_logs(cx: &mut TestAppContext) {
    let command = if cfg!(windows) {
        "(Get-Location).Path"
    } else {
        "pwd"
    };
    let (dir, w) = open(cx, &ini(&[("plain", command, 2)], &[]), "abc\n");
    run(cx, &w, 0);
    assert!(confirm(&w, cx).is_none());
    assert!(records(&w, cx).is_empty());
    let ws = workspace(&w, cx).unwrap();
    ws.read_with(cx, |ws, _| {
        assert!(ws.ai_log_of_window().is_none());
        assert_eq!(ws.tab_count(), 1);
    });
    let output = w.output(cx);
    let name = dir
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(output.contains(&name), "{output}");
}

#[gpui::test]
fn a_failing_run_is_recorded_with_its_stderr_and_code(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(FAIL, OutputTo::NextLine, false), "abc\n");
    run(cx, &w, 0);
    // a non-zero exit code changes nothing
    assert_eq!(w.text(cx), "abc\n");
    let (output, stderr, code) = records(&w, cx)[0].end.clone().unwrap();
    assert_eq!(output, "");
    assert!(stderr.contains("oops"), "{stderr}");
    assert_eq!(code, Some(3));
}

#[gpui::test]
fn a_run_that_cannot_start_is_recorded(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(COPY, OutputTo::NextLine, false), "abc\n");
    cx.update(|app| {
        let terminal = matsu_config::terminal::TerminalSettings::from_values(
            "/no/such/shell-qzx",
            1000,
            false,
        );
        crate::terminal::settings::set(app, terminal);
    });
    w.slot(cx, 0);
    wait_until(cx, "the box", |cx| w.prompt(cx).is_some());
    w.cx(cx).simulate_prompt_answer("OK");
    wait_idle(&w, cx);
    assert_eq!(w.text(cx), "abc\n");
    let records = records(&w, cx);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].input, "abc\n");
    let (output, stderr, code) = records[0].end.clone().expect("the record has ended");
    assert_eq!((output.as_str(), code), ("", None));
    assert!(!stderr.is_empty());
}

#[gpui::test]
fn cancelling_a_run_is_recorded(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(LONG, OutputTo::NextLine, false), "abc\n");
    w.slot(cx, 0);
    wait_until(cx, "the box", |cx| w.running(cx));
    // running: the record is open
    let running = records(&w, cx);
    assert_eq!(running.len(), 1);
    assert!(running[0].end.is_none());
    w.cx(cx).simulate_keystrokes("escape");
    wait_idle(&w, cx);
    assert_eq!(w.text(cx), "abc\n");
    let (_, stderr, _) = records(&w, cx)[0]
        .end
        .clone()
        .expect("the record has ended");
    assert!(stderr.contains("キャンセル"), "{stderr}");
}

#[gpui::test]
fn the_log_has_the_output_as_the_command_wrote_it_for_every_place(cx: &mut TestAppContext) {
    let command = if cfg!(windows) {
        "[Console]::Out.Write('answer')"
    } else {
        "printf answer"
    };
    for (place, expected) in [
        (OutputTo::NextLine, "abc\nanswer\n"),
        (OutputTo::Replace, "answer"),
        (OutputTo::OutputWindow, "abc\n"),
    ] {
        let (_dir, w) = open(cx, &ai_ini(command, place, false), "abc\n");
        run(cx, &w, 0);
        let (output, _, _) = records(&w, cx)[0].end.clone().unwrap();
        assert_eq!(
            output, "answer",
            "{place:?}: no line break is added or dropped"
        );
        assert_eq!(w.text(cx).replace("\r\n", "\n"), expected, "{place:?}");
    }
}

#[gpui::test]
fn the_status_bar_says_where_the_run_is_recorded(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(LONG, OutputTo::NextLine, false), "abc\n");
    w.slot(cx, 0);
    wait_until(cx, "the box", |cx| w.running(cx));
    let notice = w.notice(cx).unwrap_or_default();
    assert!(notice.contains("AI ログを表示"), "{notice}");
    w.cx(cx).simulate_keystrokes("escape");
    wait_idle(&w, cx);
}

#[gpui::test]
fn the_function_show_ai_log_opens_and_goes_to_the_log(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(COPY, OutputTo::NextLine, false), "abc\n");
    let code = matsu_config::settings::oneliner::F_AI_LOG_SHOW;
    w.call_code(cx, code);
    let ws = workspace(&w, cx).unwrap();
    ws.read_with(cx, |ws, _| {
        assert!(ws.ai_log_of_window().is_some());
        assert_eq!(ws.active_tab(), 1, "the log is shown");
    });
}

#[test]
fn the_oneliner_submenu_always_ends_with_the_ai_log_item() {
    use crate::menu::{Lists as _, MenuAction};
    use matsu_config::settings::oneliner::{F_AI_LOG_SHOW, F_ONELINER_LIST, OneLiners};
    // with no one-liner registered the item is the whole list; with some, it is the last
    let none = crate::oneliner::OneLinerList(std::sync::Arc::new(OneLiners::default()));
    let items = none.items(F_ONELINER_LIST);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].label, "AI ログを表示");
    assert_eq!(items[0].action, MenuAction::Function(F_AI_LOG_SHOW));
    assert!(items[0].enabled);
}

// ---- Tab in the dialogs of the flow, on a window with the key bindings of the app ----

#[gpui::test]
fn tab_stays_inside_the_input_dialog_on_a_real_window(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "", "abc\n");
    super::open_dialog(&w, cx);
    // The dialog's controls are tab stops of `Root`'s own Tab handling (no key listener of the dialog is needed): the
    // focus goes round the field and the buttons and never into the text behind, and no Tab is typed into it.
    for key in ["tab", "tab", "tab", "tab", "shift-tab", "tab", "tab", "tab"] {
        press(&w, cx, key);
        let editor_focused = w
            .handle
            .update(cx, |_, window, app| {
                w.view.read(app).focus_handle.is_focused(window)
            })
            .unwrap();
        assert!(!editor_focused, "{key}: the focus left the dialog");
        assert!(w.dialog(cx).is_some(), "{key}");
    }
    assert_eq!(w.text(cx), "abc\n");
}
