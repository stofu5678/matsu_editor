//! The preview tab (ADR-0024, `crate::preview`) in a window: opening it, the tab list, the menus,
//! the updates while the source is edited, the return to the source and the close.

use std::time::Duration;

use gpui::{AnyWindowHandle, Entity, TestAppContext};
use matsu_commands::keymap::KeyMap;

use super::content::MenuKind;
use super::tests::{run, type_text, window, workspace};
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::file_ops::windows::open_editor_window;
use crate::preview::text::{Format, MAX_BYTES};

fn open(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    run(cx, handle, "F_PREVIEW_TAB");
}

fn menus(cx: &mut TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        crate::app::set_key_map(cx, KeyMap::default());
        crate::app::refresh_menus(cx);
    });
}

/// The preview of the active tab.
fn preview_of(
    cx: &mut TestAppContext,
    ws: &Entity<Workspace>,
) -> Entity<crate::preview::PreviewView> {
    ws.read_with(cx, |ws, _| {
        ws.tab_content(ws.active_tab())
            .and_then(super::content::TabContent::preview)
            .expect("the active tab is a preview")
            .view()
            .clone()
    })
}

fn bar_names(cx: &mut TestAppContext, ws: &Entity<Workspace>) -> Vec<String> {
    let bar = ws.read_with(cx, |ws, _| ws.menu_bar.clone());
    bar.read_with(cx, |bar, cx| bar.menu_names(cx))
        .into_iter()
        .filter(|name| name != "MATSU Editor")
        .collect()
}

#[gpui::test]
fn the_command_opens_a_preview_next_to_the_source(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    menus(cx);
    type_text(cx, handle, "# 見出し");
    open(cx, handle);
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, cx| {
        assert_eq!(ws.tab_count(), 2);
        assert_eq!(ws.active_tab(), 1);
        let panel = ws.tab_panel(1).unwrap();
        assert_eq!(panel.menu_kind(), MenuKind::Preview);
        assert!(
            panel.caption(cx).starts_with("プレビュー: "),
            "{}",
            panel.caption(cx)
        );
        assert_eq!(panel.accessible_name(cx), panel.caption(cx));
        // It has no status bar, and closing it asks nothing.
        assert!(panel.status(cx).is_none());
        assert_eq!(panel.close_check(cx), super::content::CloseCheck::None);
        // The source is the editor tab; the preview is not one of the window's editors.
        assert_eq!(ws.tab_editors().len(), 1);
        assert_eq!(ws.all_views().len(), 1);
        assert!(ws.tab_editor(1).is_none());
    });
    // The menus of the window are the preview's, and Sakura's are back with the source.
    assert_eq!(bar_names(cx, &ws), vec!["プレビュー(P)".to_owned()]);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.activate_tab(0, window, cx))
        })
        .unwrap();
    assert!(bar_names(cx, &ws).iter().any(|n| n.contains("ファイル")));
}

#[gpui::test]
fn opening_it_again_goes_to_the_preview_that_there_is(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    open(cx, handle);
    let ws = workspace(cx, handle);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.activate_tab(0, window, cx))
        })
        .unwrap();
    open(cx, handle);
    ws.read_with(cx, |ws, _| {
        assert_eq!(ws.tab_count(), 2);
        assert_eq!(ws.active_tab(), 1);
    });
    // From the preview itself the command does nothing.
    open(cx, handle);
    ws.read_with(cx, |ws, _| assert_eq!(ws.tab_count(), 2));
}

#[gpui::test]
fn a_burst_of_edits_reads_the_text_once_after_the_pause(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    open(cx, handle);
    let ws = workspace(cx, handle);
    let preview = preview_of(cx, &ws);
    let (_, reads) = preview.read_with(cx, |p, _| p.shown());
    // Edit the source three times.
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.activate_tab(0, window, cx))
        })
        .unwrap();
    for text in ["a", "b", "c"] {
        type_text(cx, handle, text);
    }
    // Before the pause nothing is read; after it, once.
    cx.executor().advance_clock(Duration::from_millis(100));
    cx.run_until_parked();
    assert_eq!(preview.read_with(cx, |p, _| p.shown()), (0, reads));
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    assert_eq!(preview.read_with(cx, |p, _| p.shown()), (4, reads + 1));
}

#[gpui::test]
fn the_preview_returns_to_the_source_and_closes(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    open(cx, handle);
    let ws = workspace(cx, handle);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.preview_to_source(window, cx))
        })
        .unwrap();
    ws.read_with(cx, |ws, _| {
        assert_eq!(ws.active_tab(), 0);
        assert_eq!(ws.tab_count(), 2);
    });
    // Closing the preview (here from its own tab) leaves the source.
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| {
                ws.activate_tab(1, window, cx);
                ws.close_active_preview(window, cx);
            })
        })
        .unwrap();
    cx.run_until_parked();
    ws.read_with(cx, |ws, _| {
        assert_eq!(ws.tab_count(), 1);
        assert_eq!(ws.menu_kind(), MenuKind::Editor);
        assert!(ws.tab_content(0).unwrap().preview().is_none());
    });
}

#[gpui::test]
fn the_source_is_opened_again_when_its_tab_was_closed(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    open(cx, handle);
    let ws = workspace(cx, handle);
    // Close the tab of the source, so that the preview is the only tab.
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.close_tab_now(0, window, cx))
        })
        .unwrap();
    ws.read_with(cx, |ws, _| {
        assert_eq!(ws.tab_count(), 1);
        assert_eq!(ws.menu_kind(), MenuKind::Preview);
    });
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.preview_to_source(window, cx))
        })
        .unwrap();
    ws.read_with(cx, |ws, _| {
        assert_eq!(ws.tab_count(), 2);
        assert_eq!(ws.menu_kind(), MenuKind::Editor);
        assert_eq!(ws.tab_editors().len(), 1);
    });
}

#[gpui::test]
fn a_preview_does_not_move_to_another_window(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    open(cx, handle);
    let ws = workspace(cx, handle);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| {
                assert!(ws.take_tab(1, window, cx).is_none());
                assert_eq!(ws.tab_count(), 2);
            })
        })
        .unwrap();
}

#[gpui::test]
fn a_big_document_is_cut_at_the_cap_and_the_header_says_so(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    super::tests::tab_mode(cx, true, true);
    let big = "x".repeat(99) + "\n";
    let doc = SharedDocument::new(
        matsu_core::model::DocumentModel::new(
            matsu_core::buffer::Document::from_utf8(big.repeat(5_000).into_bytes()).unwrap(),
        ),
        None,
    );
    let handle = cx.update(|app| open_editor_window(app, doc, None)).unwrap();
    cx.run_until_parked();
    open(cx, handle);
    let ws = workspace(cx, handle);
    let preview = preview_of(cx, &ws);
    preview.read_with(cx, |p, _| {
        assert!(p.truncated());
        assert!(p.shown().0 <= MAX_BYTES);
        assert!(p.shown().0 > MAX_BYTES - 200);
    });
}

#[gpui::test]
fn the_format_can_be_changed_by_the_menu(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    open(cx, handle);
    let ws = workspace(cx, handle);
    let preview = preview_of(cx, &ws);
    // An untitled document starts as Markdown.
    assert_eq!(preview.read_with(cx, |p, _| p.format()), Format::Markdown);
    preview.update(cx, |p, cx| p.set_format(Format::Html, cx));
    assert_eq!(preview.read_with(cx, |p, _| p.format()), Format::Html);
}
