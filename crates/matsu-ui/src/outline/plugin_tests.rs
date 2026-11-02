//! Tests of the outline of a plugin in the pane (the jack `Outline`, ADR-0026): a plugin with a plug of
//! the jack runs on the real `matsu-script` (skipped when that program is not built next to the tests),
//! and the pane shows what the plug gave, or says why it shows nothing.

use std::fs;
use std::path::Path;
use std::time::Duration;

use gpui::TestAppContext;
use matsu_commands::command::Arg;
use matsu_config::plugins::{DEFAULT_LANGUAGE, SearchDirs};
use matsu_outline::OutlineKind;

use super::tests::{active_view, labels, pane, rows, run, run_with, window_with};
use crate::macros::plugin_call::tests::script_host;
use crate::macros::plugin_call::{set_test_interpreter, set_test_limit, shutdown};
use crate::plugin_outline::tests::{FOLDER, ID, Place};

/// The place for a test, or `None` (the test is skipped) when the program that serves the plugins is
/// not built.
fn begin(name: &str, script: &str) -> Option<Place> {
    let host = script_host()?;
    set_test_interpreter(Some(host));
    Some(Place::new(name, script))
}

fn finish(cx: &mut TestAppContext) {
    set_test_interpreter(None);
    set_test_limit(None);
    cx.update(|app| shutdown(app, ID));
}

/// Lets the window and the plugin process run until the pane has its result.
fn wait_for_the_result(cx: &mut TestAppContext, handle: gpui::AnyWindowHandle) {
    let pane = pane(cx, handle).expect("the pane is open");
    for _ in 0..1500 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if !pane.read_with(cx, |p, _| p.is_analyzing()) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the pane did not get its result");
}

fn open_outline(cx: &mut TestAppContext, handle: gpui::AnyWindowHandle) {
    cx.executor().allow_parking();
    run(cx, handle, "F_OUTLINE");
    wait_for_the_result(cx, handle);
}

fn title(cx: &mut TestAppContext, handle: gpui::AnyWindowHandle) -> String {
    pane(cx, handle)
        .unwrap()
        .read_with(cx, |p, _| p.title().to_owned())
}

fn message(cx: &mut TestAppContext, handle: gpui::AnyWindowHandle) -> Option<String> {
    pane(cx, handle)
        .unwrap()
        .read_with(cx, |p, _| p.message().map(str::to_owned))
}

fn open_all_nodes(cx: &mut TestAppContext, handle: gpui::AnyWindowHandle) {
    let pane = pane(cx, handle).unwrap();
    loop {
        let rows = pane.read_with(cx, |p, _| p.rows().to_vec());
        let Some(row) = rows.iter().position(|r| r.has_children && !r.expanded) else {
            return;
        };
        pane.update(cx, |p, cx| p.set_node(row, true, cx));
    }
}

#[gpui::test]
fn the_items_the_levels_and_the_title_of_the_plug_are_in_the_pane(cx: &mut TestAppContext) {
    let Some(place) = begin(
        "tree",
        &fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../matsu-plugin/samples/plugins/HelloOutline/outline.js"),
        )
        .unwrap(),
    ) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let handle = window_with(
        cx,
        "# A\ntext\n## B\n### C\n# D\n",
        "a.zzz",
        place.user(Place::code(), true, true),
    );
    open_outline(cx, handle);
    assert_eq!(title(cx, handle), "Headings (sample plugin)");
    assert_eq!(message(cx, handle), None);
    open_all_nodes(cx, handle);
    let all = rows(cx, handle);
    assert_eq!(
        all.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
        ["A", "B", "C", "D"]
    );
    // AddFuncInfo2: the low bits of the last argument are the depth
    assert_eq!(
        all.iter().map(|r| r.depth).collect::<Vec<_>>(),
        [0, 1, 2, 0]
    );
    assert_eq!(
        pane(cx, handle)
            .unwrap()
            .read_with(cx, |p, _| p.outline_type()),
        OutlineKind::Plugin(Place::code())
    );
    finish(cx);
}

#[gpui::test]
fn a_list_shows_the_lines_and_the_columns_the_plug_gave_in_the_order_of_the_lines(
    cx: &mut TestAppContext,
) {
    let Some(place) = begin(
        "list",
        "Outline.SetListType(300);\n\
         Outline.AddFuncInfo(3, 1, 'third', 0);\n\
         Outline.AddFuncInfo('1', '2', 'first', 0);\n\
         Outline.AddFuncInfo3(2, 1, 'elsewhere', 0, 'other.txt');\n",
    ) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let handle = window_with(
        cx,
        "a\nb\nc\n",
        "a.zzz",
        place.user(Place::code(), true, true),
    );
    open_outline(cx, handle);
    // `OUTLINE_LIST` has no title of its own
    assert_eq!(title(cx, handle), "");
    let all = rows(cx, handle);
    assert_eq!(
        all.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
        ["first", "elsewhere", "third"]
    );
    let columns: Vec<_> = all.iter().map(|r| r.columns.clone().unwrap()).collect();
    // the numbers may come as text, as they do from WSH (`_wtoi`)
    assert_eq!((columns[0][0].as_str(), columns[0][1].as_str()), ("1", "2"));
    assert_eq!(columns[2][0], "3");
    finish(cx);
}

#[gpui::test]
fn the_labels_of_a_class_tree_are_the_ones_the_plug_set(cx: &mut TestAppContext) {
    // the example of the manual (`help/plugin/Text/implementation03.html`)
    let Some(place) = begin(
        "labels",
        "Outline.SetListType(200);\n\
         Outline.SetLabel(2, ' メソッド');\n\
         Outline.SetLabel(10, ' モジュール');\n\
         Outline.AddFuncInfo(1, 1, 'Foo', 10);\n\
         Outline.AddFuncInfo(3, 1, 'Foo::Bar::Hoge', 2);\n\
         Outline.AddFuncInfo(5, 1, 'Foo::Bar::Fuga', 2);\n",
    ) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let handle = window_with(
        cx,
        "a\nb\nc\nd\ne\n",
        "a.zzz",
        place.user(Place::code(), true, true),
    );
    open_outline(cx, handle);
    open_all_nodes(cx, handle);
    let all = labels(cx, handle);
    assert_eq!(all[0], "Foo モジュール", "{all:?}");
    assert!(all.iter().any(|l| l == "Bar クラス"), "{all:?}");
    assert!(all.iter().any(|l| l == "Hoge メソッド"), "{all:?}");
    finish(cx);
}

#[gpui::test]
fn the_plug_is_called_when_the_outline_is_analyzed_and_not_for_a_key(cx: &mut TestAppContext) {
    let Some(place) = begin(
        "calls",
        "console.log('[ran]'); Outline.SetListType(300); Outline.AddFuncInfo(1, 1, 'x', 0);",
    ) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let handle = window_with(cx, "abc\n", "a.zzz", place.user(Place::code(), true, true));
    let ran = |cx: &mut TestAppContext| {
        active_view(cx, handle)
            .read_with(cx, |v, _| v.macro_output_text())
            .matches("[ran]")
            .count()
    };
    assert_eq!(ran(cx), 0, "opening a window runs nothing");
    open_outline(cx, handle);
    assert_eq!(ran(cx), 1);
    // typing makes the result old; it does not call the plug again
    for _ in 0..3 {
        active_view(cx, handle).update(cx, |view, cx| {
            view.run_with(cx, |c| c.insert_text("x").map(|_| ()))
                .unwrap();
            cx.notify();
        });
        cx.run_until_parked();
    }
    for _ in 0..20 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(ran(cx), 1, "a key does not call the plug");
    assert!(pane(cx, handle).unwrap().read_with(cx, |p, _| p.is_stale()));
    // the update button (`F_OUTLINE` with the action 1) analyzes again, and calls it
    run_with(cx, handle, "F_OUTLINE", &[Arg::Int(1)]);
    wait_for_the_result(cx, handle);
    assert_eq!(ran(cx), 2);
    finish(cx);
}

#[gpui::test]
fn a_plug_that_throws_leaves_an_empty_list_and_the_reason(cx: &mut TestAppContext) {
    let Some(place) = begin(
        "throws",
        "Outline.AddFuncInfo(1, 1, 'lost', 0); throw new Error('boom at the plug');",
    ) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let handle = window_with(cx, "a\n", "a.zzz", place.user(Place::code(), true, true));
    open_outline(cx, handle);
    // no built-in analysis in its place, and nothing of what the plug gave before it failed
    assert!(rows(cx, handle).is_empty());
    let text = message(cx, handle).expect("the reason is in the pane");
    assert!(text.contains("Outl"), "{text}");
    assert!(text.contains("boom at the plug"), "{text}");
    // the output panel has the failure too
    let output = active_view(cx, handle).read_with(cx, |v, _| v.macro_output_text());
    assert!(output.contains("boom at the plug"), "{output}");
    // the pane is whole: it analyzes again
    run_with(cx, handle, "F_OUTLINE", &[Arg::Int(1)]);
    wait_for_the_result(cx, handle);
    assert!(message(cx, handle).is_some());
    finish(cx);
}

#[gpui::test]
fn a_plug_that_runs_out_of_time_leaves_an_empty_list_and_the_reason(cx: &mut TestAppContext) {
    let Some(place) = begin("slow", "Outline.AddFuncInfo(1, 1, 'lost', 0); for (;;) {}") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    // (the limit of the jack is 10 seconds: `plugin_host::LIMITS`)
    set_test_limit(Some(Duration::from_millis(1500)));
    let handle = window_with(cx, "a\n", "a.zzz", place.user(Place::code(), true, true));
    open_outline(cx, handle);
    assert!(rows(cx, handle).is_empty());
    let text = message(cx, handle).expect("the reason is in the pane");
    assert!(text.contains("時間切れ"), "{text}");
    // the window works: the text can be edited
    active_view(cx, handle).update(cx, |view, cx| {
        view.run_with(cx, |c| c.insert_text("x").map(|_| ()))
            .unwrap();
        cx.notify();
    });
    cx.run_until_parked();
    let first_line = active_view(cx, handle).read_with(cx, |v, app| {
        v.document().read(app).doc().line(0).unwrap().unwrap().text
    });
    assert!(first_line.contains('x'), "{first_line:?}");
    finish(cx);
}

#[test]
fn the_limit_of_the_jack_is_ten_seconds() {
    assert_eq!(
        matsu_plugin::plugin_host::limit_of("Outline"),
        Duration::from_secs(10)
    );
}

#[gpui::test]
fn an_outline_of_a_plugin_that_is_off_is_the_text_topics_and_its_number_stays(
    cx: &mut TestAppContext,
) {
    let Some(place) = begin("off", "Outline.AddFuncInfo(1, 1, 'x', 0);") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    // the switch is off: Sakura does not load the plugin, so the number is no outline of a plugin and the
    // text topics are made (`Command_FUNCLIST`, `default:`)
    let handle = window_with(
        cx,
        "■ head\n",
        "a.zzz",
        place.user(Place::code(), false, true),
    );
    open_outline(cx, handle);
    assert_eq!(title(cx, handle), "テキスト トピックツリー");
    assert!(labels(cx, handle).iter().any(|l| l.contains("head")));
    finish(cx);
}

#[gpui::test]
fn a_plugin_the_user_did_not_agree_to_does_not_run(cx: &mut TestAppContext) {
    let Some(place) = begin(
        "noconsent",
        "console.log('[ran]'); Outline.AddFuncInfo(1, 1, 'x', 0);",
    ) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let handle = window_with(
        cx,
        "■ head\n",
        "a.zzz",
        place.user(Place::code(), true, false),
    );
    open_outline(cx, handle);
    assert_eq!(title(cx, handle), "テキスト トピックツリー");
    assert!(!active_view(cx, handle).read_with(cx, |v, _| v.macro_output_text().contains("[ran]")));
    finish(cx);
}

#[gpui::test]
fn a_file_read_by_ranges_has_no_outline_of_a_plugin_either(cx: &mut TestAppContext) {
    use crate::document::SharedDocument;
    use crate::file_ops::windows::open_editor_window;
    use crate::tabs::settings::TabSettings;
    use matsu_core::file::{OpenOptions, open_file};

    let Some(place) = begin(
        "huge",
        "console.log('[ran]'); Outline.AddFuncInfo(1, 1, 'x', 0);",
    ) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.update(gpui_component::init);
    cx.update(|cx| {
        cx.set_global(place.user(Place::code(), true, true));
        cx.set_global(TabSettings {
            show_bar: true,
            grouped: true,
            new_window: false,
        })
    });
    let path = place.dir.join("big.zzz");
    fs::write(&path, "a\nb\n").unwrap();
    let options = OpenOptions {
        huge_file_len: Some(1),
        ..OpenOptions::default()
    };
    let file = open_file(&path, &options).unwrap();
    assert!(file.is_ranged());
    let shared = SharedDocument::from_text_file(file);
    let handle = cx
        .update(|cx| open_editor_window(cx, shared, None))
        .unwrap();
    cx.run_until_parked();
    open_outline(cx, handle);
    assert!(rows(cx, handle).is_empty());
    assert_eq!(
        message(cx, handle).as_deref(),
        Some("巨大なファイルなので、アウトライン解析は使えません")
    );
    // the plug did not run
    assert!(!active_view(cx, handle).read_with(cx, |v, _| v.macro_output_text().contains("[ran]")));
    finish(cx);
}

#[gpui::test]
fn the_outline_object_is_not_there_for_a_command(cx: &mut TestAppContext) {
    // a plug of another jack cannot use the object: the call is an error of the editor
    let Some(place) = begin("command", "Outline.AddFuncInfo(1, 1, 'x', 0);") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let dirs = SearchDirs::new(&place.settings(), None);
    let def = dirs.load_def(FOLDER, DEFAULT_LANGUAGE).unwrap();
    let mut api = crate::macros::plugin_call::DefApi::new(&dirs, FOLDER, def, 1);
    use matsu_plugin::plugin_host::PluginApi as _;
    let error = api
        .call(
            "Outline.AddFuncInfo",
            &[1.into(), 1.into(), "x".into(), 0.into()],
        )
        .unwrap_err();
    assert!(error.message.contains("Outline"), "{}", error.message);
    set_test_interpreter(None);
    let _ = cx;
}
