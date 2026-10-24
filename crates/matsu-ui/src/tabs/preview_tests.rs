//! The preview tab (ADR-0024, `crate::preview`) in a window: opening it, the tab list, the menus,
//! the updates while the source is edited, the return to the source and the close.

use std::time::Duration;

use gpui::{AnyWindowHandle, AppContext as _, Entity, TestAppContext};
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

/// The tab bar paints a tab for the source and one for the preview, grouped or not.
fn both_tabs_are_painted(cx: &mut TestAppContext, handle: AnyWindowHandle) {
    let visual = super::tests::visual(cx, handle);
    visual.run_until_parked();
    for index in 0..2 {
        assert!(
            visual
                .debug_bounds(Box::leak(format!("tab-{index}").into_boxed_str()))
                .is_some(),
            "tab {index} is not painted"
        );
    }
}

#[gpui::test]
fn the_bar_has_a_tab_for_the_preview(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    open(cx, handle);
    both_tabs_are_painted(cx, handle);
}

#[gpui::test]
fn the_bar_has_a_tab_for_the_preview_also_when_the_windows_are_not_grouped(
    cx: &mut TestAppContext,
) {
    let handle = window(cx, true);
    super::tests::tab_mode(cx, true, false);
    open(cx, handle);
    both_tabs_are_painted(cx, handle);
}

#[gpui::test]
fn a_document_over_the_source_limit_is_not_previewed(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    super::tests::tab_mode(cx, true, true);
    let doc = SharedDocument::new(
        matsu_core::model::DocumentModel::new(
            matsu_core::buffer::Document::from_utf8(
                "z".repeat(crate::preview::text::MAX_SOURCE_BYTES + 1)
                    .into_bytes(),
            )
            .unwrap(),
        ),
        None,
    );
    let handle = cx.update(|app| open_editor_window(app, doc, None)).unwrap();
    cx.run_until_parked();
    open(cx, handle);
    // Refused before the tab and its hidden view are made, with a notice in the status bar.
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, cx| {
        assert_eq!(ws.tab_count(), 1);
        let notice = ws.editor().read(cx).notice.clone().unwrap_or_default();
        assert!(notice.contains("プレビューしません"), "{notice}");
    });
}

/// A click of `button` on the element painted with `selector`.
fn click(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    selector: &'static str,
    button: gpui::MouseButton,
) {
    let visual = super::tests::visual(cx, handle);
    let at = visual.debug_bounds(selector).expect("painted").center();
    visual.simulate_mouse_down(at, button, gpui::Modifiers::default());
    visual.simulate_mouse_up(at, button, gpui::Modifiers::default());
    visual.run_until_parked();
}

#[gpui::test]
fn the_close_button_of_the_bar_closes_the_preview_without_a_question(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    // The source has changes; the preview has nothing to save.
    type_text(cx, handle, "changed");
    open(cx, handle);
    click(cx, handle, "tab-bar-close", gpui::MouseButton::Left);
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, _| {
        assert_eq!(ws.tab_count(), 1);
        assert!(ws.tab_content(0).unwrap().preview().is_none());
    });
    assert_eq!(super::tests::windows(cx), 1);
}

#[gpui::test]
fn the_middle_button_on_the_tab_of_a_preview_closes_it(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    type_text(cx, handle, "changed");
    open(cx, handle);
    click(cx, handle, "tab-1", gpui::MouseButton::Middle);
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, _| assert_eq!(ws.tab_count(), 1));
    assert_eq!(super::tests::windows(cx), 1);
}

#[gpui::test]
fn the_next_window_function_works_from_a_preview(cx: &mut TestAppContext) {
    // The wheel over the bar and the keys of the bar run this function on the view of the window.
    let handle = window(cx, true);
    open(cx, handle);
    run(cx, handle, "F_NEXTWINDOW");
    let ws = workspace(cx, handle);
    ws.read_with(cx, |ws, _| assert_eq!(ws.active_tab(), 0));
}

#[gpui::test]
fn the_list_of_the_tabs_opens_over_a_preview(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    open(cx, handle);
    click(cx, handle, "tab-list-button", gpui::MouseButton::Left);
    let ws = workspace(cx, handle);
    // The list belongs to the hidden view; the window draws it over the preview.
    ws.read_with(cx, |ws, cx| {
        assert!(ws.editor().read(cx).tab_list.is_some());
    });
}

#[gpui::test]
fn closing_the_list_of_the_tabs_gives_the_focus_back_to_the_preview(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    open(cx, handle);
    click(cx, handle, "tab-list-button", gpui::MouseButton::Left);
    super::tests::visual(cx, handle).simulate_keystrokes("escape");
    cx.run_until_parked();
    let ws = workspace(cx, handle);
    let focused = handle
        .update(cx, |_, window, app| {
            ws.read(app).active_focus(app).is_focused(window)
        })
        .unwrap();
    assert!(focused, "the preview has the focus");
    ws.read_with(cx, |ws, cx| {
        assert!(ws.editor().read(cx).tab_list.is_none())
    });
}

#[gpui::test]
fn the_source_in_another_window_is_not_opened_again(cx: &mut TestAppContext) {
    let first = window(cx, true);
    open(cx, first);
    let second = cx
        .update(|app| open_editor_window(app, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    let a = workspace(cx, first);
    let b = workspace(cx, second);
    // The source tab of the first window moves to the second; its preview stays.
    let set = first
        .update(cx, |_, window, app| {
            a.update(app, |ws, cx| ws.take_tab(0, window, cx))
        })
        .unwrap()
        .expect("the source tab");
    second
        .update(cx, |_, window, app| {
            b.update(app, |ws, cx| {
                ws.insert_tab(set, usize::MAX, false, window, cx)
            })
        })
        .unwrap();
    cx.run_until_parked();
    let before = b.read_with(cx, |ws, _| ws.tab_count());
    first
        .update(cx, |_, window, app| {
            a.update(app, |ws, cx| ws.preview_to_source(window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    // No second editor for the document in either window.
    a.read_with(cx, |ws, _| assert_eq!(ws.tab_count(), 1));
    b.read_with(cx, |ws, _| assert_eq!(ws.tab_count(), before));
}

/// A document of the file `path` with the text `# a`.
fn file_document(path: &std::path::Path) -> SharedDocument {
    SharedDocument::new(
        matsu_core::model::DocumentModel::new(
            matsu_core::buffer::Document::from_utf8(b"# a\n".to_vec()).unwrap(),
        ),
        Some(path.to_path_buf()),
    )
}

#[gpui::test]
fn the_source_opened_again_is_found_by_its_file(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    super::tests::tab_mode(cx, true, true);
    let path = std::env::temp_dir().join(format!("matsu-preview-{}.md", std::process::id()));
    std::fs::write(&path, "# a\n").unwrap();
    let handle = cx
        .update(|app| open_editor_window(app, file_document(&path), None))
        .unwrap();
    cx.run_until_parked();
    open(cx, handle);
    let ws = workspace(cx, handle);
    // The source tab is closed, and the same file is opened as a new document in a new tab.
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| {
                let document = cx.new(|_| file_document(&path));
                let editor = cx.new(|cx| crate::view::EditorView::new(document, cx));
                ws.insert_tab(
                    crate::panes::PaneSet::new(editor),
                    usize::MAX,
                    false,
                    window,
                    cx,
                );
                ws.close_tab_now(0, window, cx);
            })
        })
        .unwrap();
    cx.run_until_parked();
    let (before, preview_active) = ws.read_with(cx, |ws, _| {
        let active = ws.tab_content(ws.active_tab()).unwrap();
        (ws.tab_count(), active.preview().is_some())
    });
    assert_eq!(before, 2);
    assert!(preview_active);
    handle
        .update(cx, |_, window, app| {
            ws.update(app, |ws, cx| ws.preview_to_source(window, cx))
        })
        .unwrap();
    // No second editor for the file: the tab that is there is shown, and the preview follows its
    // document now.
    ws.read_with(cx, |ws, cx| {
        assert_eq!(ws.tab_count(), before);
        assert!(ws.tab_content(ws.active_tab()).unwrap().preview().is_none());
        let source = ws.editor().read(cx).document().clone();
        let previews: Vec<_> = (0..ws.tab_count())
            .filter_map(|i| {
                ws.tab_content(i)
                    .and_then(super::content::TabContent::preview)
            })
            .collect();
        assert_eq!(previews.len(), 1);
        assert!(previews[0].view().read(cx).document() == &source);
    });
    let _ = std::fs::remove_file(&path);
}

#[gpui::test]
fn closing_the_group_closes_the_previews_with_their_documents(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    open(cx, handle);
    // The source tab is active again; the group closes from there.
    run(cx, handle, "F_GROUPCLOSE");
    cx.run_until_parked();
    assert_eq!(super::tests::windows(cx), 0);
}
