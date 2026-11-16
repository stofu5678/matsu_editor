//! Sending to a generative AI in mode b (ADR-0035, 4): `F_AI_ASK` opens the window, and 「送る」 runs a destination on
//! the instruction and the target text, with real shells as stand-ins for the AI tool. Nothing here runs an AI tool or
//! sends anything anywhere. The window itself is tested in `crate::ai_ask`. The names in the settings are ASCII: the
//! test settings are read as a file of the system's code page, as `matsu.ini` is.

use gpui::TestAppContext;
use matsu_config::settings::ai::F_AI_ASK;
use matsu_core::buffer::Pos;

use super::ai::{open, press, records, wait_idle};
use super::{COPY, FAIL, Win, wait_until};

/// A table of destinations: `(name, command, output)`, with `Output` as `[AI]` numbers it (0 replace, 1 next line, 2 the
/// log tab only).
fn ai_ini(slots: &[(&str, &str, i32)]) -> String {
    let mut text = String::from("[AI]\r\n");
    for (i, (name, command, output)) in slots.iter().enumerate() {
        text.push_str(&format!(
            "Name[{i:03}]={name}\r\nCommand[{i:03}]={command}\r\nOutput[{i:03}]={output}\r\nAskBeforeSend[{i:03}]=1\r\n"
        ));
    }
    text
}

fn ask(w: &Win, cx: &mut TestAppContext) {
    w.call_code(cx, F_AI_ASK);
}

fn wait_window(w: &Win, cx: &mut TestAppContext) {
    wait_until(cx, "the window of mode b", |cx| {
        w.view.read_with(cx, |view, _| view.ai_ask.is_some())
    });
}

fn is_open(w: &Win, cx: &mut TestAppContext) -> bool {
    w.view.read_with(cx, |view, _| view.ai_ask.is_some())
}

/// Types `instruction` into the window and presses 「送る」 (field -> list -> 送る).
fn send(w: &Win, cx: &mut TestAppContext, instruction: &str) {
    w.cx(cx).simulate_input(instruction);
    press(w, cx, "tab tab enter");
}

#[gpui::test]
fn the_window_opens_with_the_first_destination_and_nothing_runs(cx: &mut TestAppContext) {
    let (_dir, w) = open(
        cx,
        &ai_ini(&[("translate", COPY, 2), ("summary", COPY, 2)]),
        "abc\n",
    );
    ask(&w, cx);
    wait_window(&w, cx);
    w.view.read_with(cx, |view, app| {
        let dialog = view.ai_ask.as_ref().unwrap().read(app);
        assert_eq!(dialog.selected_index(), 0);
        let names: Vec<_> = dialog
            .request()
            .destinations
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        assert_eq!(names, ["translate", "summary"]);
        assert_eq!(dialog.request().material.lines, 1);
    });
    assert!(!w.running(cx));
    assert!(records(&w, cx).is_empty(), "nothing is logged until 送る");
    press(&w, cx, "escape");
    wait_idle(&w, cx);
}

#[gpui::test]
fn the_destination_used_last_is_chosen_the_next_time(cx: &mut TestAppContext) {
    let (_dir, w) = open(
        cx,
        &ai_ini(&[("translate", COPY, 2), ("summary", COPY, 2)]),
        "abc\n",
    );
    ask(&w, cx);
    wait_window(&w, cx);
    w.cx(cx).simulate_input("訳して");
    // the list: Down chooses the second one
    press(&w, cx, "tab down tab enter");
    wait_idle(&w, cx);
    assert_eq!(records(&w, cx)[0].name, "summary");

    ask(&w, cx);
    wait_window(&w, cx);
    w.view.read_with(cx, |view, app| {
        assert_eq!(view.ai_ask.as_ref().unwrap().read(app).selected_index(), 1);
    });
    press(&w, cx, "escape");
    wait_idle(&w, cx);
}

#[gpui::test]
fn only_destinations_that_have_a_command_are_offered(cx: &mut TestAppContext) {
    let text = "[AI]\r\nName[000]=empty\r\nCommand[000]=\r\nName[002]=second\r\nCommand[002]=cat\r\nOutput[002]=2\r\n";
    let (_dir, w) = open(cx, text, "abc\n");
    ask(&w, cx);
    wait_window(&w, cx);
    w.view.read_with(cx, |view, app| {
        let dialog = view.ai_ask.as_ref().unwrap().read(app);
        assert_eq!(dialog.request().destinations.len(), 1);
        assert_eq!(dialog.selected_index(), 2, "the number of its slot");
    });
    press(&w, cx, "escape");
    wait_idle(&w, cx);
}

#[gpui::test]
fn without_a_destination_nothing_opens_and_the_status_bar_says_so(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "", "abc\n");
    ask(&w, cx);
    wait_idle(&w, cx);
    assert!(!is_open(&w, cx));
    assert!(
        w.notice(cx)
            .is_some_and(|n| n.contains("登録されていません"))
    );
}

#[gpui::test]
fn an_empty_target_sends_nothing_and_says_so(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(&[("t", COPY, 2)]), "");
    ask(&w, cx);
    wait_idle(&w, cx);
    assert!(!is_open(&w, cx));
    assert!(
        w.notice(cx)
            .is_some_and(|n| n.contains("送る文字がありません"))
    );
    assert!(records(&w, cx).is_empty());
}

#[gpui::test]
fn only_the_log_tab_destinations_are_offered_for_a_document_that_cannot_be_edited(
    cx: &mut TestAppContext,
) {
    let (_dir, w) = open(
        cx,
        &ai_ini(&[("replace", COPY, 0), ("logonly", COPY, 2)]),
        "abc\n",
    );
    w.view_mode(cx);
    ask(&w, cx);
    wait_window(&w, cx);
    w.view.read_with(cx, |view, app| {
        let dialog = view.ai_ask.as_ref().unwrap().read(app);
        let names: Vec<_> = dialog
            .request()
            .destinations
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        assert_eq!(names, ["logonly"]);
    });
    press(&w, cx, "escape");
    wait_idle(&w, cx);

    // with only a destination that changes the document, nothing opens
    let (_dir, w) = open(cx, &ai_ini(&[("replace", COPY, 0)]), "abc\n");
    w.view_mode(cx);
    ask(&w, cx);
    wait_idle(&w, cx);
    assert!(!is_open(&w, cx));
    assert!(w.notice(cx).is_some_and(|n| n.contains("編集できない")));
}

#[gpui::test]
fn not_sending_runs_nothing_and_changes_nothing(cx: &mut TestAppContext) {
    for keys in ["escape", "tab tab tab enter"] {
        let (_dir, w) = open(cx, &ai_ini(&[("t", COPY, 0)]), "abc\n");
        ask(&w, cx);
        wait_window(&w, cx);
        w.cx(cx).simulate_input("訳して");
        press(&w, cx, keys);
        wait_idle(&w, cx);
        assert_eq!(w.text(cx), "abc\n", "{keys}");
        assert!(!is_open(&w, cx), "{keys}");
        assert!(!w.running(cx), "{keys}");
        assert!(records(&w, cx).is_empty(), "{keys}: nothing was logged");
        assert!(
            w.notice(cx).is_some_and(|n| n.contains("送らない")),
            "{keys}: {:?}",
            w.notice(cx)
        );
    }
}

#[gpui::test]
fn an_empty_instruction_runs_nothing(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(&[("t", COPY, 0)]), "abc\n");
    ask(&w, cx);
    wait_window(&w, cx);
    press(&w, cx, "tab tab enter");
    cx.run_until_parked();
    assert!(is_open(&w, cx), "the window stays");
    assert!(records(&w, cx).is_empty());
    assert_eq!(w.text(cx), "abc\n");
    press(&w, cx, "escape");
    wait_idle(&w, cx);
}

#[gpui::test]
fn the_standard_input_is_the_instruction_a_blank_line_a_separator_and_the_text(
    cx: &mut TestAppContext,
) {
    // a CRLF document, a selection of two lines and an instruction of two lines
    let (_dir, w) = open(
        cx,
        &ai_ini(&[("t", COPY, 2)]),
        "one\r\ntwo\r\nthree\r\nfour\r\n",
    );
    w.select(cx, Pos::new(1, 0), Pos::new(3, 0));
    ask(&w, cx);
    wait_window(&w, cx);
    send(&w, cx, "  Translate.\r\nKeep the tone.  ");
    wait_idle(&w, cx);
    let records = records(&w, cx);
    assert_eq!(records.len(), 1);
    let record = &records[0];
    let expected = "Translate.\nKeep the tone.\n\n---\ntwo\nthree\n";
    assert_eq!(record.input, expected, "what the log says was sent");
    let (output, stderr, code) = record.end.clone().expect("the record has ended");
    assert_eq!(
        output.replace("\r\n", "\n"),
        expected,
        "what the process received"
    );
    assert_eq!((stderr.as_str(), code), ("", Some(0)));
    // 「ログのタブだけ」: the document is as it was
    assert_eq!(w.text(cx), "one\r\ntwo\r\nthree\r\nfour\r\n");
}

#[gpui::test]
fn the_log_records_mode_b_with_its_instruction_and_the_destination(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(&[("translate", COPY, 2)]), "abc\n");
    ask(&w, cx);
    wait_window(&w, cx);
    send(&w, cx, "英語に");
    wait_idle(&w, cx);
    let record = &records(&w, cx)[0];
    assert_eq!(record.name, "translate");
    assert_eq!(record.command_line, COPY);
    assert_eq!(record.instruction.as_deref(), Some("英語に"));
}

#[gpui::test]
fn the_whole_document_is_the_text_when_nothing_is_selected(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(&[("t", COPY, 2)]), "abc\ndef\n");
    ask(&w, cx);
    wait_window(&w, cx);
    send(&w, cx, "x");
    wait_idle(&w, cx);
    assert_eq!(records(&w, cx)[0].input, "x\n\n---\nabc\ndef\n");
}

#[gpui::test]
fn a_box_selection_is_the_text_of_the_box(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(&[("t", COPY, 2)]), "abcd\nefgh\nijkl\n");
    w.view.update(cx, |view, _| {
        view.cursor.move_to(Pos::new(0, 1), false);
        view.cursor.begin_box(64, false);
        view.cursor.move_box_to(Pos::new(2, 3), 3 * 64);
    });
    ask(&w, cx);
    wait_window(&w, cx);
    send(&w, cx, "x");
    wait_idle(&w, cx);
    assert_eq!(records(&w, cx)[0].input, "x\n\n---\nbc\nfg\njk\n");
}

#[gpui::test]
fn the_output_kinds_put_the_result_where_mode_a_puts_it(cx: &mut TestAppContext) {
    // 置き換える: the selection becomes the result
    let (_dir, w) = open(cx, &ai_ini(&[("t", COPY, 0)]), "abc\ndef\nghi\n");
    w.select(cx, Pos::new(1, 0), Pos::new(2, 0));
    ask(&w, cx);
    wait_window(&w, cx);
    send(&w, cx, "i");
    wait_idle(&w, cx);
    assert_eq!(w.text(cx), "abc\ni\n\n---\ndef\nghi\n");
    // one undo step
    w.undo(cx);
    assert_eq!(w.text(cx), "abc\ndef\nghi\n");

    // 次の行に入れる: after the selected line
    let (_dir, w) = open(cx, &ai_ini(&[("t", COPY, 1)]), "abc\ndef\n");
    w.select(cx, Pos::new(0, 0), Pos::new(1, 0));
    ask(&w, cx);
    wait_window(&w, cx);
    send(&w, cx, "i");
    wait_idle(&w, cx);
    assert_eq!(w.text(cx), "abc\ni\n\n---\nabc\ndef\n");

    // ログのタブだけ: the document is not changed
    let (_dir, w) = open(cx, &ai_ini(&[("t", COPY, 2)]), "abc\n");
    ask(&w, cx);
    wait_window(&w, cx);
    send(&w, cx, "i");
    wait_idle(&w, cx);
    assert_eq!(w.text(cx), "abc\n");
    assert_eq!(records(&w, cx).len(), 1);
}

#[gpui::test]
fn a_failing_command_changes_nothing_and_is_logged(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(&[("t", FAIL, 0)]), "abc\n");
    ask(&w, cx);
    wait_window(&w, cx);
    send(&w, cx, "i");
    wait_idle(&w, cx);
    assert_eq!(w.text(cx), "abc\n");
    let (_, stderr, code) = records(&w, cx)[0].end.clone().unwrap();
    assert_eq!(code, Some(3));
    assert!(stderr.contains("oops"));
}

#[gpui::test]
fn a_document_that_changed_during_the_question_is_not_sent(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(&[("t", COPY, 0)]), "abc\n");
    ask(&w, cx);
    wait_window(&w, cx);
    w.cx(cx).simulate_input("i");
    w.view.update(cx, |view, cx| {
        view.document()
            .clone()
            .update(cx, |shared, cx| shared.set_view_mode(true, cx));
    });
    press(&w, cx, "tab tab enter");
    wait_idle(&w, cx);
    assert_eq!(w.text(cx), "abc\n");
    assert!(records(&w, cx).is_empty(), "nothing was started");
    assert!(w.notice(cx).is_some_and(|n| n.contains("変わった")));
}

#[gpui::test]
fn the_instructions_are_kept_newest_first_without_duplicates(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(&[("t", COPY, 2)]), "abc\n");
    for instruction in ["a", "b", "a", "c"] {
        ask(&w, cx);
        wait_window(&w, cx);
        send(&w, cx, instruction);
        wait_idle(&w, cx);
    }
    assert_eq!(
        cx.update(|app| crate::ai_ask::history::items(app)),
        ["c", "a", "b"].map(str::to_owned)
    );
    // the next window offers them
    ask(&w, cx);
    wait_window(&w, cx);
    w.view.read_with(cx, |view, app| {
        let dialog = view.ai_ask.as_ref().unwrap().read(app);
        assert_eq!(dialog.request().history, ["c", "a", "b"].map(str::to_owned));
    });
    press(&w, cx, "escape");
    wait_idle(&w, cx);
}

#[gpui::test]
fn at_most_twenty_instructions_are_offered(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ai_ini(&[("t", COPY, 2)]), "abc\n");
    for n in 0..crate::ai_ask::history::MAX_HISTORY + 2 {
        cx.update(|app| crate::ai_ask::history::remember(app, &format!("instruction {n}"), 0));
    }
    ask(&w, cx);
    wait_window(&w, cx);
    w.view.read_with(cx, |view, app| {
        let history = &view.ai_ask.as_ref().unwrap().read(app).request().history;
        assert_eq!(history.len(), 20);
        assert_eq!(history[0], "instruction 21");
        assert_eq!(history[19], "instruction 2");
    });
    press(&w, cx, "escape");
    wait_idle(&w, cx);
}

#[gpui::test]
fn the_instructions_are_never_written_to_a_file(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("matsu.ini");
    std::fs::write(&file, ai_ini(&[("t", COPY, 2)])).unwrap();
    let user = crate::settings::UserSettings::from_profile(
        matsu_config::Profile::load(&file).unwrap(),
        Some(file.clone()),
    );
    cx.update(|app| {
        crate::history::install(app, &user);
        app.set_global(user);
    });
    cx.update(crate::view::EditorView::bind_keys);
    let snapshot = |dir: &std::path::Path| {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| {
                let path = e.unwrap().path();
                let bytes = std::fs::read(&path).unwrap_or_default();
                (path, bytes)
            })
            .collect();
        entries.sort();
        entries
    };
    let (_doc_dir, w) = super::open_in(cx, "abc\n");
    // opening the document wrote its place into the list of the recent files: that is not of the instruction
    let before = snapshot(dir.path());
    ask(&w, cx);
    wait_window(&w, cx);
    send(&w, cx, "秘密の指示 SECRET-INSTRUCTION");
    wait_idle(&w, cx);
    assert_eq!(records(&w, cx).len(), 1, "it was sent");
    // the settings file and its folder are as they were, and nothing holds the instruction
    let after = snapshot(dir.path());
    assert_eq!(before, after);
    for (path, bytes) in &after {
        assert!(
            !String::from_utf8_lossy(bytes).contains("SECRET-INSTRUCTION"),
            "{}",
            path.display()
        );
    }
}
