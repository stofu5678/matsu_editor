//! Tests of the one-liners in GPUI's headless test window: the flow with real shells (`/bin/sh` on macOS and Linux,
//! Windows PowerShell or PowerShell 7 on Windows, the shell tab's default), the functions, the menus and the dialog.
//! The shell and its arguments are tested in `matsu_oneliner`, the text rules in `matsu_core::oneliner`.

use std::path::Path;
use std::time::Duration;

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use matsu_config::Profile;
use matsu_config::settings::oneliner::{
    F_ONELINER_DIALOG, F_ONELINER_LIST, MAX_ONELINER, OutputTo, function_of_index,
};
use matsu_core::buffer::Pos;

use crate::document::SharedDocument;
use crate::file_ops::windows::{LoadRequest, editors, open_editor_window};
use crate::settings::UserSettings;
use crate::view::EditorView;

/// Makes the text upper case.
const UPPER: &str = if cfg!(windows) {
    "$input | ForEach-Object { $_.ToUpper() }"
} else {
    "tr a-z A-Z"
};
/// Copies the input to the output.
const COPY: &str = if cfg!(windows) { "$input" } else { "cat" };
/// Writes `oops` to the standard error and ends with 3.
const FAIL: &str = if cfg!(windows) {
    "[Console]::Error.WriteLine('oops'); exit 3"
} else {
    "echo oops 1>&2; exit 3"
};
/// Copies the input and writes `warn` to the standard error.
const WARN: &str = if cfg!(windows) {
    "$input; [Console]::Error.WriteLine('warn')"
} else {
    "cat; echo warn 1>&2"
};
/// Runs for half a minute.
const LONG: &str = if cfg!(windows) {
    "Start-Sleep 30"
} else {
    "sleep 30"
};

struct Win {
    handle: AnyWindowHandle,
    view: Entity<EditorView>,
}

impl Win {
    fn call_code(&self, cx: &mut TestAppContext, code: u16) {
        let view = self.view.clone();
        self.handle
            .update(cx, |_, window, app| {
                view.update(app, |view, cx| view.run_function(code, &[], window, cx))
            })
            .unwrap();
        cx.run_until_parked();
    }

    fn slot(&self, cx: &mut TestAppContext, n: usize) {
        self.call_code(cx, function_of_index(n).unwrap());
    }

    /// The text with its line breaks as they are.
    fn text(&self, cx: &mut TestAppContext) -> String {
        self.view.read_with(cx, |view, app| {
            let doc = view.document().read(app).doc().clone();
            let mut text = String::new();
            for line in doc.lines(0..doc.line_count()) {
                let line = line.unwrap();
                text.push_str(&line.text);
                if let Some(eol) = line.eol {
                    text.push_str(eol.as_str());
                }
            }
            text
        })
    }

    fn output(&self, cx: &mut TestAppContext) -> String {
        self.view.read_with(cx, |view, _| {
            view.macros.output.text().replace("\r\n", "\n")
        })
    }

    fn running(&self, cx: &mut TestAppContext) -> bool {
        self.view
            .read_with(cx, |view, _| view.support.exec_running.is_some())
    }

    fn select(&self, cx: &mut TestAppContext, from: Pos, to: Pos) {
        self.view.update(cx, |view, _| view.cursor.select(from, to));
    }

    fn selection(&self, cx: &mut TestAppContext) -> Option<(Pos, Pos)> {
        self.view.read_with(cx, |view, _| view.cursor().selection())
    }

    fn notice(&self, cx: &mut TestAppContext) -> Option<String> {
        self.view.read_with(cx, |view, _| view.notice.clone())
    }

    fn view_mode(&self, cx: &mut TestAppContext) {
        self.view.update(cx, |view, cx| {
            view.document()
                .clone()
                .update(cx, |shared, cx| shared.set_view_mode(true, cx));
        });
    }

    fn cx<'a>(&self, cx: &'a mut TestAppContext) -> &'a mut VisualTestContext {
        VisualTestContext::from_window(self.handle, cx).into_mut()
    }

    fn prompt(&self, cx: &mut TestAppContext) -> Option<String> {
        self.cx(cx).pending_prompt().map(|(message, _)| message)
    }

    fn dialog(&self, cx: &mut TestAppContext) -> Option<Entity<super::dialog::OneLinerDialog>> {
        self.view
            .read_with(cx, |view, _| view.support.oneliner_dialog.clone())
    }

    /// The table in the settings now.
    fn undo(&self, cx: &mut TestAppContext) {
        self.view
            .update(cx, |view, cx| view.run_command("undo", &[], cx));
        cx.run_until_parked();
    }
}

fn ini(slots: &[(&str, &str, i32)], recent: &[&str]) -> String {
    let mut text = String::from("[OneLiner]\r\n");
    for (i, (name, command, output)) in slots.iter().enumerate() {
        text.push_str(&format!(
            "Name[{i:03}]={name}\r\nCommand[{i:03}]={command}\r\nOutput[{i:03}]={output}\r\n"
        ));
    }
    for (i, line) in recent.iter().enumerate() {
        text.push_str(&format!("Recent[{i:02}]={line}\r\n"));
    }
    text
}

fn install_settings(cx: &mut TestAppContext, text: &str, path: Option<&Path>) {
    let profile = Profile::from_bytes(text.as_bytes());
    cx.update(|app| {
        app.set_global(UserSettings::from_profile(
            profile,
            path.map(Path::to_owned),
        ));
    });
}

/// A window with `text` as a file in a folder of its own, after the settings are installed.
fn open(cx: &mut TestAppContext, settings: &str, text: &str) -> (tempfile::TempDir, Win) {
    cx.update(gpui_component::init);
    install_settings(cx, settings, None);
    open_in(cx, text)
}

fn open_in(cx: &mut TestAppContext, text: &str) -> (tempfile::TempDir, Win) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.txt");
    std::fs::write(&path, text).unwrap();
    let request = LoadRequest {
        path,
        encoding: None,
    };
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), Some(request)))
        .unwrap();
    cx.run_until_parked();
    let view = cx.update(|app| {
        editors(app)
            .into_iter()
            .find(|(h, _)| *h == handle)
            .map(|(_, view)| view)
            .unwrap()
    });
    (dir, Win { handle, view })
}

fn wait_until(
    cx: &mut TestAppContext,
    what: &str,
    mut done: impl FnMut(&mut TestAppContext) -> bool,
) {
    cx.executor().allow_parking();
    let begun = std::time::Instant::now();
    while begun.elapsed() < Duration::from_secs(60) {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if done(cx) {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("never happened: {what}");
}

/// Runs slot `n` and waits for the end of the run.
fn run(cx: &mut TestAppContext, w: &Win, n: usize) {
    w.slot(cx, n);
    wait_until(cx, "the end of the run", |cx| !w.running(cx));
    cx.run_until_parked();
}

fn one(command: &str, output: i32) -> String {
    ini(&[("test", command, output)], &[])
}

// ---- the flow ----

#[gpui::test]
fn a_selection_is_replaced_and_the_new_text_is_selected(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(UPPER, 0), "abc\ndef\nghi\n");
    w.select(cx, Pos::new(1, 0), Pos::new(1, 3));
    run(cx, &w, 0);
    assert_eq!(w.text(cx), "abc\nDEF\nghi\n");
    let (from, to) = w.selection(cx).expect("the new text is selected");
    assert_eq!(from, Pos::new(1, 0));
    assert_eq!((to.line, to.col), (1, 3));
}

#[gpui::test]
fn the_whole_document_is_replaced_when_nothing_is_selected(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(UPPER, 0), "abc\ndef\n");
    run(cx, &w, 0);
    assert_eq!(w.text(cx), "ABC\nDEF\n");
    assert_eq!(w.selection(cx), Some((Pos::new(0, 0), Pos::new(2, 0))));
}

#[gpui::test]
fn the_result_is_one_undo_step(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(UPPER, 0), "abc\ndef\n");
    run(cx, &w, 0);
    assert_eq!(w.text(cx), "ABC\nDEF\n");
    w.undo(cx);
    assert_eq!(w.text(cx), "abc\ndef\n");
}

#[gpui::test]
fn a_crlf_document_gets_the_result_with_crlf(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(UPPER, 0), "abc\r\ndef\r\n");
    run(cx, &w, 0);
    assert_eq!(w.text(cx), "ABC\r\nDEF\r\n");
}

#[gpui::test]
fn the_last_line_break_that_the_input_lacked_is_not_added(cx: &mut TestAppContext) {
    // the shell adds a line break after the last line
    let (_dir, w) = open(cx, &one(UPPER, 0), "abc");
    run(cx, &w, 0);
    assert_eq!(w.text(cx), "ABC");
}

#[gpui::test]
fn next_line_puts_the_result_after_the_selected_lines(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(UPPER, 1), "abc\ndef\nghi\n");
    // lines 0 and 1, up to the start of line 2
    w.select(cx, Pos::new(0, 0), Pos::new(2, 0));
    run(cx, &w, 0);
    assert_eq!(w.text(cx), "abc\ndef\nABC\nDEF\nghi\n");
    assert_eq!(w.selection(cx), Some((Pos::new(2, 0), Pos::new(4, 0))));
    w.undo(cx);
    assert_eq!(w.text(cx), "abc\ndef\nghi\n");
}

#[gpui::test]
fn next_line_for_the_whole_document_goes_to_its_end(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(UPPER, 1), "abc\ndef");
    run(cx, &w, 0);
    // the last line had no line break: one is added before the result
    assert_eq!(w.text(cx), "abc\ndef\nABC\nDEF\n");
}

#[gpui::test]
fn next_line_in_a_crlf_document_uses_crlf(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(UPPER, 1), "abc\r\ndef\r\n");
    w.select(cx, Pos::new(0, 0), Pos::new(0, 2));
    run(cx, &w, 0);
    assert_eq!(w.text(cx), "abc\r\nAB\r\ndef\r\n");
}

#[gpui::test]
fn next_line_with_no_result_changes_nothing(cx: &mut TestAppContext) {
    let nothing = if cfg!(windows) { "$null" } else { "true" };
    let (_dir, w) = open(cx, &one(nothing, 1), "abc");
    run(cx, &w, 0);
    // not even the line break that the last line lacks
    assert_eq!(w.text(cx), "abc");
}

#[gpui::test]
fn an_external_command_waits_while_a_one_liner_is_starting(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "", "abc\n");
    w.view
        .update(cx, |view, _| view.support.oneliner_starting = true);
    // 外部コマンド実行 with the output taken (0x01 and 0x02): refused as for a second external command
    let view = w.view.clone();
    w.handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| {
                view.run_function(
                    matsu_commands::sakura::FUNCTIONS
                        .iter()
                        .find(|f| f.name == "F_EXECMD")
                        .unwrap()
                        .code,
                    &[
                        matsu_commands::command::Arg::Text("echo hi".into()),
                        matsu_commands::command::Arg::Int(3),
                    ],
                    window,
                    cx,
                )
            })
        })
        .unwrap();
    wait_until(cx, "the box", |cx| w.prompt(cx).is_some());
    let message = w.prompt(cx).unwrap();
    assert!(message.contains("別のコマンドを実行中です"), "{message}");
    w.cx(cx).simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert!(!w.running(cx));
    assert_eq!(w.text(cx), "abc\n");
}

#[gpui::test]
fn the_output_window_shows_the_result_and_the_document_stays(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(UPPER, 2), "abc\n");
    run(cx, &w, 0);
    assert_eq!(w.text(cx), "abc\n");
    let output = w.output(cx);
    assert!(output.contains("ABC"), "{output}");
    assert!(output.contains("#CmdLine  : "), "{output}");
    assert!(w.view.read_with(cx, |v, _| v.macros.output.is_open()));
}

#[gpui::test]
fn a_failing_one_liner_changes_nothing_and_shows_the_code_and_the_error(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(FAIL, 0), "abc\n");
    run(cx, &w, 0);
    assert_eq!(w.text(cx), "abc\n");
    let output = w.output(cx);
    assert!(output.contains("oops"), "{output}");
    assert!(output.contains("終了コード: 3"), "{output}");
    assert!(w.prompt(cx).is_none());
}

#[gpui::test]
fn the_error_output_of_a_one_liner_that_ends_with_0_is_shown_and_the_result_applied(
    cx: &mut TestAppContext,
) {
    let (_dir, w) = open(cx, &one(WARN, 0), "abc\n");
    run(cx, &w, 0);
    assert_eq!(w.text(cx), "abc\n");
    let output = w.output(cx);
    assert!(output.contains("warn"), "{output}");
    assert!(output.contains("終了コード: 0"), "{output}");
    // and with a result that differs
    let (_dir, w) = open(cx, &one(&format!("{UPPER}; {}", warn_only()), 0), "abc\n");
    run(cx, &w, 0);
    assert_eq!(w.text(cx), "ABC\n");
    assert!(w.output(cx).contains("warn"));
}

fn warn_only() -> &'static str {
    if cfg!(windows) {
        "[Console]::Error.WriteLine('warn')"
    } else {
        "echo warn 1>&2"
    }
}

#[gpui::test]
fn cancelling_stops_the_run_and_changes_nothing(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(LONG, 0), "keep\n");
    w.slot(cx, 0);
    wait_until(cx, "the box", |cx| w.running(cx));
    // typing does not reach the text
    w.cx(cx).simulate_input("zzz");
    cx.run_until_parked();
    w.cx(cx).simulate_keystrokes("escape");
    wait_until(cx, "the end", |cx| !w.running(cx));
    assert_eq!(w.text(cx), "keep\n");
    assert!(w.notice(cx).is_some_and(|n| n.contains("中断")));
}

#[gpui::test]
fn a_document_in_view_mode_refuses_replace_and_next_line_but_not_the_output_window(
    cx: &mut TestAppContext,
) {
    let settings = ini(&[("r", UPPER, 0), ("n", UPPER, 1), ("o", UPPER, 2)], &[]);
    let (_dir, w) = open(cx, &settings, "abc\n");
    w.view_mode(cx);
    w.slot(cx, 0);
    w.slot(cx, 1);
    assert!(!w.running(cx));
    assert_eq!(w.text(cx), "abc\n");
    assert!(w.notice(cx).is_some());
    run(cx, &w, 2);
    assert!(w.output(cx).contains("ABC"));
    assert_eq!(w.text(cx), "abc\n");
}

#[gpui::test]
fn a_box_selection_is_the_input_and_the_result_goes_in_once(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(UPPER, 0), "abcd\nefgh\nijkl\n");
    w.view.update(cx, |view, _| {
        view.cursor.move_to(Pos::new(0, 1), false);
        view.cursor.begin_box(1, false);
        view.cursor.move_box_to(Pos::new(1, 3), 3);
    });
    run(cx, &w, 0);
    // the box ("bc", "fg") is replaced by the result ("BC\nFG") once, at the caret
    // (the text of a box has a line break after each row, so the result has one too)
    assert_eq!(w.text(cx), "aBC\nFG\nd\neh\nijkl\n");
}

#[gpui::test]
fn a_document_that_changed_meanwhile_gets_nothing(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(LONG.replace("30", "1").as_str(), 0), "abc\n");
    w.slot(cx, 0);
    wait_until(cx, "the box", |cx| w.running(cx));
    // an edit that gets past the box (a macro, a plugin)
    w.view.update(cx, |view, cx| {
        view.run_with(cx, |c| c.insert_text("X")).unwrap();
    });
    wait_until(cx, "the end", |cx| !w.running(cx));
    assert_eq!(w.text(cx), "Xabc\n");
    assert!(w.output(cx).contains("文書が変わった"), "{}", w.output(cx));
}

#[gpui::test]
fn a_one_liner_that_cannot_start_is_told_in_a_box(cx: &mut TestAppContext) {
    // a shell that does not exist
    let settings = format!("{}[Terminal]\r\nShell=/no/such/shell-qzx\r\n", one(COPY, 0));
    let (_dir, w) = open(cx, &settings, "abc\n");
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
    let message = w.prompt(cx).unwrap();
    assert!(
        message.contains("ワンライナーの実行に失敗しました。"),
        "{message}"
    );
    w.cx(cx).simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert_eq!(w.text(cx), "abc\n");
    assert!(!w.running(cx));
}

// ---- the functions ----

#[gpui::test]
fn an_unregistered_slot_does_nothing(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &one(UPPER, 0), "abc\n");
    w.slot(cx, 7);
    assert!(!w.running(cx));
    assert_eq!(w.text(cx), "abc\n");
    assert!(
        w.notice(cx)
            .is_some_and(|n| n.contains("登録されていません"))
    );
}

#[gpui::test]
fn a_registered_slot_runs_from_a_macro_function_name(cx: &mut TestAppContext) {
    use matsu_commands::sakura::macro_functions::MACRO_COMMANDS;
    let (_dir, w) = open(cx, &one(UPPER, 0), "abc\n");
    let row = MACRO_COMMANDS
        .iter()
        .find(|m| m.name == "OneLiner0")
        .expect("a macro function for slot 0");
    assert_eq!(row.function, "F_ONELINER_0");
    assert!(row.key_macro);
    w.call_code(cx, row.code);
    wait_until(cx, "the end of the run", |cx| !w.running(cx));
    assert_eq!(w.text(cx), "ABC\n");
    // there is no macro that adds or changes one
    assert!(
        MACRO_COMMANDS
            .iter()
            .all(|m| !m.name.starts_with("OneLiner") || m.function.starts_with("F_ONELINER_"))
    );
}

// ---- the menus ----

#[gpui::test]
fn the_list_expands_to_the_registered_names(cx: &mut TestAppContext) {
    use crate::menu::{Lists, MenuAction};
    let (_dir, _w) = open(cx, "", "");
    let table = matsu_config::settings::oneliner::OneLiners::from_profile(&Profile::from_bytes(
        ini(&[("up", "tr a-z A-Z", 0), ("", "cat", 1)], &[]).as_bytes(),
    ));
    let list = super::OneLinerList(std::sync::Arc::new(table));
    let items = list.items(F_ONELINER_LIST);
    let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
    // a one-liner without a name shows its command
    // and the way to the log of what was sent to an AI is the last item (ADR-0030)
    assert_eq!(labels, ["up", "cat", "AI ログを表示"]);
    assert_eq!(
        items[1].action,
        MenuAction::Function(function_of_index(1).unwrap())
    );
    assert!(list.items(0).is_empty());
}

#[test]
fn the_default_right_click_menu_has_the_oneliner_submenu_once() {
    use matsu_config::settings::CustomMenus;
    let menus = CustomMenus::default();
    let count = menus
        .get(0)
        .unwrap()
        .items
        .iter()
        .filter(|i| i.function == F_ONELINER_LIST)
        .count();
    assert_eq!(count, 1);
}

#[test]
fn a_saved_right_click_menu_gets_the_submenu_of_version_3_once() {
    use matsu_config::settings::CustomMenus;
    use matsu_config::settings::custom_menu::MATSU_VERSION_KEY;
    let by_name = |_: &str| None;
    // a menu of version 2: the encodings, no one-liners
    let mut profile = Profile::new();
    CustomMenus::default()
        .write_to_profile(&mut profile, None, None)
        .unwrap();
    let mut all = CustomMenus::from_profile(&profile, &by_name)
        .menus()
        .to_vec();
    all[0].items.retain(|i| i.function != F_ONELINER_LIST);
    let menus = CustomMenus::from_menus(all);
    let mut saved = Profile::new();
    menus.write_to_profile(&mut saved, None, None).unwrap();
    saved.set_i32("CustMenu", MATSU_VERSION_KEY, 2).unwrap();
    let read = CustomMenus::from_profile(&saved, &by_name);
    let items = &read.get(0).unwrap().items;
    let at = items
        .iter()
        .position(|i| i.function == F_ONELINER_LIST)
        .expect("the submenu is added");
    assert_eq!(items[at - 1].function, 32709, "after the last encoding");
    // written with version 3, it is not added again
    let mut again = Profile::new();
    read.write_to_profile(&mut again, None, None).unwrap();
    matsu_config::settings::CustomMenus::write_matsu_version(&mut again).unwrap();
    let twice = CustomMenus::from_profile(&again, &by_name);
    assert_eq!(
        twice
            .get(0)
            .unwrap()
            .items
            .iter()
            .filter(|i| i.function == F_ONELINER_LIST)
            .count(),
        1
    );
}

#[test]
fn the_right_click_menu_shows_the_list_as_a_submenu_with_the_dialog_and_the_settings() {
    use crate::custom_menu::model::{Context, entries};
    use crate::menu::{Entry, NoLists};
    let menus = matsu_config::settings::CustomMenus::default();
    let keymap = matsu_commands::keymap::KeyMap::default();
    let table = matsu_config::settings::oneliner::OneLiners::from_profile(&Profile::from_bytes(
        ini(&[("up", "tr a-z A-Z", 0)], &[]).as_bytes(),
    ));
    let list = super::OneLinerList(std::sync::Arc::new(table.clone()));
    let available = |_: u16| true;
    let rows = entries(
        0,
        &Context {
            menus: &menus,
            macros: &Default::default(),
            oneliners: &table,
            keymap: &keymap,
            available: &available,
            lists: &list,
            parentheses: true,
        },
    );
    let Some(Entry::Submenu { label, entries }) = rows
        .iter()
        .find(|e| matches!(e, Entry::Submenu { label, .. } if label.starts_with("ワンライナー")))
    else {
        panic!("no submenu in {rows:?}");
    };
    assert!(label.starts_with("ワンライナー"));
    let shape: Vec<String> = entries
        .iter()
        .map(|e| match e {
            Entry::Item(i) => i.label.clone(),
            Entry::Separator => "-".to_owned(),
            Entry::Submenu { label, .. } => label.clone(),
        })
        .collect();
    assert_eq!(shape.len(), 5, "{shape:?}");
    assert_eq!(shape[0], "up");
    assert_eq!(shape[1], "AI ログを表示");
    assert_eq!(shape[2], "-");
    assert!(
        shape[3].starts_with("ワンライナーを入力して実行"),
        "{shape:?}"
    );
    assert!(shape[4].starts_with("ワンライナーの設定"), "{shape:?}");
    // and with none registered the submenu says so
    let empty = Default::default();
    let rows = entries_with_none(&menus, &keymap, &empty);
    assert!(rows.contains(&"(なし)".to_owned()), "{rows:?}");
    let _ = NoLists;
}

fn entries_with_none(
    menus: &matsu_config::settings::CustomMenus,
    keymap: &matsu_commands::keymap::KeyMap,
    table: &matsu_config::settings::oneliner::OneLiners,
) -> Vec<String> {
    use crate::custom_menu::model::{Context, entries};
    use crate::menu::{Entry, NoLists};
    let available = |_: u16| true;
    let rows = entries(
        0,
        &Context {
            menus,
            macros: &Default::default(),
            oneliners: table,
            keymap,
            available: &available,
            lists: &NoLists,
            parentheses: true,
        },
    );
    rows.iter()
        .find_map(|e| match e {
            Entry::Submenu { label, entries } if label.starts_with("ワンライナー") => Some(
                entries
                    .iter()
                    .map(|e| match e {
                        Entry::Item(i) => i.label.clone(),
                        _ => "-".to_owned(),
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

#[test]
fn the_main_menu_has_the_submenu_under_the_tool_menu() {
    use crate::menu::{Entry, Registered, main_menu};
    let keymap = matsu_commands::keymap::KeyMap::default();
    let menus = main_menu(&keymap, &Registered);
    let tool = menus
        .iter()
        .find(|m| m.label.starts_with("ツール"))
        .unwrap();
    let at = tool
        .entries
        .iter()
        .position(
            |e| matches!(e, Entry::Submenu { label, .. } if label.starts_with("ワンライナー")),
        )
        .expect("the submenu");
    let Entry::Submenu { entries, .. } = &tool.entries[at] else {
        unreachable!()
    };
    // (なし), a separator, the dialog and the settings
    assert_eq!(entries.len(), 4);
    // right after 外部コマンド実行
    let Entry::Item(before) = &tool.entries[at - 1] else {
        panic!()
    };
    assert!(
        before.label.starts_with("外部コマンド実行"),
        "{}",
        before.label
    );
}

#[test]
fn a_saved_main_menu_gets_the_submenu_of_version_5_once() {
    use matsu_config::settings::MainMenu;
    use matsu_config::settings::main_menu::MATSU_VERSION_KEY;
    let default = crate::menu::default_main_menu();
    let by_name = |_: &str| None;
    // a saved menu of version 4: written without the submenu
    let mut old = default.clone();
    let node = old
        .rows
        .iter()
        .position(|r| r.name == "ワンライナー")
        .expect("the submenu is in the default menu");
    // the node, the list, a separator, the dialog and the settings
    old.rows.drain(node..node + 5);
    let mut profile = Profile::new();
    old.write_to_profile(&mut profile, None, None, Some(2))
        .unwrap();
    profile.set_i32("MainMenu", MATSU_VERSION_KEY, 4).unwrap();
    let read = MainMenu::from_profile(&profile, &by_name, default);
    let count = |m: &MainMenu| {
        m.rows
            .iter()
            .filter(|r| r.function == F_ONELINER_LIST)
            .count()
    };
    assert_eq!(count(&read), 1);
    assert!(read.rows.iter().any(|r| r.function == F_ONELINER_DIALOG));
    // written again, it is not added a second time
    let mut again = Profile::new();
    read.write_to_profile(&mut again, None, None, Some(2))
        .unwrap();
    let twice = MainMenu::from_profile(&again, &by_name, default);
    assert_eq!(count(&twice), 1);
}

// ---- the dialog ----

fn open_dialog(w: &Win, cx: &mut TestAppContext) -> Entity<super::dialog::OneLinerDialog> {
    w.call_code(cx, F_ONELINER_DIALOG);
    w.dialog(cx).expect("the dialog is open")
}

fn fill(w: &Win, cx: &mut TestAppContext, line: &str) {
    let dialog = w.dialog(cx).unwrap();
    let line = line.to_owned();
    w.handle
        .update(cx, |_, window, app| {
            dialog.update(app, |d, cx| {
                d.line.update(cx, |s, cx| s.set_value(line, window, cx))
            })
        })
        .unwrap();
}

#[gpui::test]
fn the_dialog_starts_with_the_history_newest_first(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ini(&[], &["newest", "older"]), "");
    let dialog = open_dialog(&w, cx);
    let (rows, line) =
        dialog.read_with(cx, |d, app| (d.history_rows().to_vec(), d.choice(app).line));
    assert_eq!(rows, ["newest", "older"]);
    assert_eq!(line, "newest");
}

#[gpui::test]
fn the_dialog_runs_the_line_and_adds_it_to_the_history(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ini(&[], &["older"]), "abc\n");
    let dialog = open_dialog(&w, cx);
    fill(&w, cx, UPPER);
    w.handle
        .update(cx, |_, window, app| {
            dialog.update(app, |d, cx| d.ok(window, cx))
        })
        .unwrap();
    wait_until(cx, "the end of the run", |cx| {
        !w.running(cx) && w.text(cx) == "ABC\n"
    });
    let recent = cx.update(|app| super::table(app).recent);
    assert_eq!(recent, [UPPER, "older"]);
    assert!(w.dialog(cx).is_none());
}

#[gpui::test]
fn a_line_run_again_moves_to_the_front_of_the_history(cx: &mut TestAppContext) {
    let (_dir, _w) = open(cx, &ini(&[], &["a", "b", "c"]), "");
    cx.update(|app| super::store::remember(app, "c"));
    assert_eq!(cx.update(|app| super::table(app).recent), ["c", "a", "b"]);
}

#[gpui::test]
fn cancelling_the_dialog_runs_and_remembers_nothing(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, &ini(&[], &["older"]), "abc\n");
    open_dialog(&w, cx);
    fill(&w, cx, UPPER);
    w.cx(cx).simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(w.dialog(cx).is_none());
    assert!(!w.running(cx));
    assert_eq!(w.text(cx), "abc\n");
    assert_eq!(cx.update(|app| super::table(app).recent), ["older"]);
}

#[gpui::test]
fn registering_takes_the_first_free_slot(cx: &mut TestAppContext) {
    // slot 0 is taken, slot 1 is free, slot 2 is taken
    let mut text = ini(&[("zero", "cat", 0)], &[]);
    text.push_str("Name[002]=two\r\nCommand[002]=cat\r\nOutput[002]=1\r\n");
    let (_dir, w) = open(cx, &text, "");
    let dialog = open_dialog(&w, cx);
    fill(&w, cx, "sort -u");
    w.handle
        .update(cx, |_, window, app| {
            dialog.update(app, |d, cx| {
                d.output = OutputTo::OutputWindow;
                d.start_registering(window, cx);
                assert!(d.registering);
                d.name.update(cx, |s, cx| s.set_value("sorted", window, cx));
                d.register(cx);
            })
        })
        .unwrap();
    let table = cx.update(|app| super::table(app));
    let one = &table.slots[1];
    assert_eq!(
        (one.name.as_str(), one.command.as_str(), one.output),
        ("sorted", "sort -u", OutputTo::OutputWindow)
    );
    let message = dialog.read_with(cx, |d, _| d.message.clone());
    assert_eq!(message.as_deref(), Some("ワンライナー 1 に登録しました。"));
    // the dialog stays open
    assert!(w.dialog(cx).is_some());
}

#[gpui::test]
fn registering_says_so_when_all_the_slots_are_taken(cx: &mut TestAppContext) {
    let slots: Vec<(String, String, i32)> = (0..MAX_ONELINER)
        .map(|i| (format!("n{i}"), "cat".to_owned(), 0))
        .collect();
    let refs: Vec<(&str, &str, i32)> = slots
        .iter()
        .map(|(n, c, o)| (n.as_str(), c.as_str(), *o))
        .collect();
    let (_dir, w) = open(cx, &ini(&refs, &[]), "");
    let dialog = open_dialog(&w, cx);
    fill(&w, cx, "sort");
    w.handle
        .update(cx, |_, window, app| {
            dialog.update(app, |d, cx| {
                d.start_registering(window, cx);
                d.register(cx);
            })
        })
        .unwrap();
    let message = dialog.read_with(cx, |d, _| d.message.clone()).unwrap();
    assert!(message.contains("すべて使われています"), "{message}");
    assert_eq!(cx.update(|app| super::table(app)).slots.len(), MAX_ONELINER);
    assert_eq!(cx.update(|app| super::table(app)).slots[0].command, "cat");
}

#[gpui::test]
fn the_history_and_a_registration_are_written_to_the_settings_file(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("matsu.ini");
    std::fs::write(&file, "[Other]\r\nkeep=1\r\n").unwrap();
    let user = UserSettings::from_profile(Profile::load(&file).unwrap(), Some(file.clone()));
    cx.update(|app| {
        crate::history::install(app, &user);
        app.set_global(user);
    });
    cx.update(|app| {
        super::store::remember(app, "sort -u");
        super::store::register(app, "s", "sort -u", OutputTo::NextLine).unwrap();
    });
    let saved = String::from_utf8_lossy(&std::fs::read(&file).unwrap()).into_owned();
    assert!(saved.contains("Recent[00]=sort -u"), "{saved}");
    assert!(saved.contains("Command[000]=sort -u"), "{saved}");
    assert!(saved.contains("Output[000]=1"), "{saved}");
    assert!(saved.contains("keep=1"), "{saved}");
}

#[gpui::test]
fn a_document_that_cannot_be_edited_offers_only_the_output_window(cx: &mut TestAppContext) {
    let (_dir, w) = open(cx, "", "abc\n");
    w.view_mode(cx);
    let dialog = open_dialog(&w, cx);
    let output = dialog.read_with(cx, |d, _| d.output);
    assert_eq!(output, OutputTo::OutputWindow);
}

mod ai;
