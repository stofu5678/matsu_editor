//! The AI log tab (ADR-0030, `crate::ai_log`) in a window: opened behind the document, one only, records
//! begun and ended through the API, the 50 limit, the menus, the close, and the copy.

use std::time::Duration;

use chrono::TimeZone as _;
use gpui::{AnyWindowHandle, Entity, TestAppContext};
use matsu_commands::keymap::KeyMap;

use super::content::{CloseCheck, MenuKind};
use super::tests::{type_text, window, workspace};
use crate::ai_log::records::MAX_RECORDS;
use crate::ai_log::{self, AiLogView, Finished, RecordHandle, Started};
use crate::app::Workspace;

fn started(name: &str, command_line: &str) -> Started {
    Started {
        name: name.to_owned(),
        command_line: command_line.to_owned(),
        input: "こんにちは\r\n世界".to_owned(),
        input_size: None,
        started_at: chrono::Local
            .with_ymd_and_hms(2026, 11, 13, 10, 0, 0)
            .unwrap(),
    }
}

fn answer(text: &str) -> Finished {
    Finished {
        output: text.to_owned(),
        stderr: "warning".to_owned(),
        exit_code: Some(0),
        elapsed: Duration::from_millis(500),
    }
}

fn begin(cx: &mut TestAppContext, handle: AnyWindowHandle, started: Started) -> RecordHandle {
    let ws = workspace(cx, handle);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ai_log::begin(ws, started, window, cx))
        })
        .unwrap()
}

fn open(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    let ws = workspace(cx, handle);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ai_log::open(ws, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

fn log_view(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> Entity<AiLogView> {
    ws.read_with(cx, |ws, _| {
        ws.ai_log_of_window().expect("the window has the log")
    })
}

fn activate(cx: &mut TestAppContext, handle: AnyWindowHandle, index: usize) {
    let ws = workspace(cx, handle);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.activate_tab(index, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

fn menus(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::app::set_key_map(cx, KeyMap::default());
        crate::app::refresh_menus(cx);
    });
}

#[gpui::test]
fn a_record_opens_the_tab_behind_the_document(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    type_text(cx, handle, "document");
    let _record = begin(cx, handle, started("要約", "claude -p"));
    cx.run_until_parked();
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, cx| {
        assert_eq!(ws.tab_count(), 2);
        // The document keeps the focus.
        assert_eq!(ws.active_tab(), 0);
        assert!(ws.tab_content(0).unwrap().is_editor());
        let panel = ws.tab_panel(1).unwrap();
        assert_eq!(panel.caption(cx), "AI ログ");
        assert_eq!(panel.accessible_name(cx), "AI ログ");
        assert_eq!(panel.menu_kind(), MenuKind::AiLog);
        assert!(panel.status(cx).is_none());
        assert_eq!(panel.close_check(cx), CloseCheck::None);
        // The log is not one of the window's editors, so nothing that walks the editors (the autosave of
        // unsaved documents, the session) sees it.
        assert_eq!(ws.tab_editors().len(), 1);
        assert_eq!(ws.all_views().len(), 1);
        assert!(ws.tab_editor(1).is_none());
    });
    let view = log_view(cx, &ws);
    view.read_with(cx, |view, _| {
        let records: Vec<_> = view.log().records().collect();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "要約");
        assert_eq!(records[0].command_line.as_str(), "claude -p");
        assert!(records[0].result.is_none());
    });
}

#[gpui::test]
fn the_window_has_one_log_and_opening_it_goes_there(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    let _a = begin(cx, handle, started("a", "one"));
    let _b = begin(cx, handle, started("b", "two"));
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, _| {
        assert_eq!(ws.tab_count(), 2);
        assert_eq!(ws.active_tab(), 0);
    });
    assert_eq!(
        log_view(cx, &ws).read_with(cx, |v, _| v.log().len()),
        2,
        "both records are in the one log"
    );
    // Asking for the log takes the focus to it, and again changes nothing.
    open(cx, handle);
    open(cx, handle);
    ws.read_with(cx, |ws, _| {
        assert_eq!(ws.tab_count(), 2);
        assert_eq!(ws.active_tab(), 1);
    });
    // A record begun while the log is shown does not move the user.
    activate(cx, handle, 0);
    let _c = begin(cx, handle, started("c", "three"));
    ws.read_with(cx, |ws, _| {
        assert_eq!(ws.tab_count(), 2);
        assert_eq!(ws.active_tab(), 0);
    });
}

#[gpui::test]
fn a_record_is_ended_in_place_by_its_handle(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    let first = begin(cx, handle, started("a", "one"));
    let _second = begin(cx, handle, started("b", "two"));
    let ws = workspace(cx, handle);
    let view = log_view(cx, &ws);
    cx.update(|app| first.finish(answer("the answer"), app));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        let records: Vec<_> = view.log().records().collect();
        // Order does not change, and only the first has ended.
        assert_eq!(records[0].name, "a");
        let result = records[0].result.as_ref().expect("ended");
        assert_eq!(result.output.as_str(), "the answer");
        assert_eq!(result.stderr.as_str(), "warning");
        assert_eq!(result.exit_code, Some(0));
        assert_eq!(result.elapsed, Duration::from_millis(500));
        assert!(records[1].result.is_none());
    });
}

#[gpui::test]
fn the_end_of_a_run_whose_tab_was_closed_is_dropped(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    let record = begin(cx, handle, started("a", "one"));
    let ws = workspace(cx, handle);
    activate(cx, handle, 1);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.close_active_own_tab(window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    ws.read_with(cx, |ws, _| {
        assert_eq!(ws.tab_count(), 1);
        assert!(
            ws.ai_log_of_window().is_none(),
            "the records went with the tab"
        );
    });
    // No panic, nothing comes back.
    cx.update(|app| record.finish(answer("late"), app));
    // A new record opens a new, empty log.
    let _again = begin(cx, handle, started("b", "two"));
    let view = log_view(cx, &ws);
    view.read_with(cx, |view, _| {
        assert_eq!(view.log().len(), 1);
        assert_eq!(view.log().records().next().unwrap().name, "b");
    });
}

#[gpui::test]
fn the_log_keeps_the_last_fifty_records(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    let handles: Vec<RecordHandle> = (0..MAX_RECORDS + 2)
        .map(|n| begin(cx, handle, started(&format!("run{n}"), "cmd")))
        .collect();
    let ws = workspace(cx, handle);
    let view = log_view(cx, &ws);
    view.read_with(cx, |view, _| {
        assert_eq!(view.log().len(), MAX_RECORDS);
        assert_eq!(view.log().records().next().unwrap().name, "run2");
    });
    // The end of one that was dropped does nothing; the end of the last one lands on it.
    cx.update(|app| handles[0].finish(answer("late"), app));
    cx.update(|app| handles[MAX_RECORDS + 1].finish(answer("last"), app));
    view.read_with(cx, |view, _| {
        let last = view.log().records().last().unwrap();
        assert_eq!(last.result.as_ref().unwrap().output.as_str(), "last");
        assert!(view.log().records().all(|r| r.name != "run0"));
    });
}

#[gpui::test]
fn the_menus_of_the_window_are_the_logs_while_it_is_active(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    menus(cx);
    let _record = begin(cx, handle, started("a", "one"));
    let ws = workspace(cx, handle);
    let names = |cx: &mut TestAppContext| {
        let bar = ws.read_with(cx, |ws, _| ws.menu_bar.clone());
        bar.read_with(cx, |bar, cx| bar.menu_names(cx))
            .into_iter()
            .filter(|name| name != "MATSU Editor")
            .collect::<Vec<_>>()
    };
    // Behind the document: Sakura's menus stay.
    assert!(names(cx).iter().any(|n| n.contains("ファイル")));
    open(cx, handle);
    assert_eq!(names(cx), vec!["AI ログ(L)".to_owned()]);
    activate(cx, handle, 0);
    assert!(names(cx).iter().any(|n| n.contains("ファイル")));
}

/// Copying: the selected text reaches the clipboard, and a command line can be pasted from there.
#[gpui::test]
fn the_selected_text_is_copied_to_the_clipboard(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    let command = "claude -p --tools \"\" --no-session-persistence";
    let record = begin(cx, handle, started("要約", command));
    cx.update(|app| record.finish(answer("答え"), app));
    open(cx, handle);
    let ws = workspace(cx, handle);
    let view = log_view(cx, &ws);
    cx.update(|app| app.write_to_clipboard(gpui::ClipboardItem::new_string("before".to_owned())));
    // Nothing is selected: the clipboard stays.
    super::tests::visual(cx, handle).dispatch_action(crate::ai_log::Copy);
    assert_eq!(clipboard(cx), Some("before".to_owned()));
    view.update(cx, |view, cx| view.select_all(cx));
    cx.run_until_parked();
    let selected = view.read_with(cx, |view, cx| view.selected_text(cx));
    assert!(selected.contains(command), "{selected:?}");
    assert!(selected.contains("答え"));
    super::tests::visual(cx, handle).dispatch_action(crate::ai_log::Copy);
    let copied = clipboard(cx).expect("text on the clipboard");
    assert_eq!(copied, selected);
}

fn clipboard(cx: &mut TestAppContext) -> Option<String> {
    cx.read_from_clipboard().and_then(|item| item.text())
}
