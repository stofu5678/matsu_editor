//! The tab bar (`CTabWnd`, `sakura_core/window/CTabWnd.cpp`; the help is
//! `help/sakura/res/HLP000150.html`).
//!
//! * Each tab shows the caption of its document (`GetTabName`, [`Workspace::caption_of`]).
//! * A click makes the tab active (`OnTabLButtonUp`); the close button and a click with the
//!   middle button close the document with the question about saving (`F_WINCLOSE`,
//!   `OnTabLButtonUp`, `OnTabMButtonUp`); a double click on the bar beside the tabs makes a new
//!   document (`OnLButtonDblClk`: `F_FILENEW`); the button at the right end lists the documents
//!   (`TabListMenu`; here it is the window list, `F_WINLIST`).
//! * A tab can be dragged to another place of the bar (`ReorderTab`). Dragging a tab out of the
//!   window to make a new one (`SeparateGroup` on a drop outside the bar) needs the OS to move a
//!   drag across windows, which GPUI cannot do: `F_TAB_SEPARATE` does it.
//!
//! With the tabs grouped (Sakura's tab mode) the bar shows the tabs of this window. With the bar
//! shown but the windows not grouped ("タブをまとめない") every window has one document, and the
//! bar of each window lists the documents of all windows: a click brings that window to the
//! front.

use gpui::{
    AnyElement, AnyWindowHandle, AppContext as _, AsyncWindowContext, Context, Entity,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px, rgb,
};
use ume_commands::sakura::FUNCTIONS;

use super::{flows, settings};
use crate::app::Workspace;
use crate::file_ops::close::confirm_and_close_window;
use crate::file_ops::windows::listed_workspaces;
use crate::view::EditorView;

/// A tab of the bar.
#[derive(Clone)]
struct Entry {
    handle: AnyWindowHandle,
    view: Entity<EditorView>,
    caption: String,
    active: bool,
    /// The position among the tabs of this window, when it is one of them.
    tab: Option<usize>,
}

/// What is dragged when a tab moves: its position in the bar.
#[derive(Clone)]
struct DraggedTab {
    from: usize,
    caption: String,
}

impl Render for DraggedTab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_0p5()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0x888888))
            .text_size(px(12.))
            .text_color(rgb(0x1e1e1e))
            .child(self.caption.clone())
    }
}

fn function_code(name: &str) -> u16 {
    FUNCTIONS
        .iter()
        .find(|f| f.name == name)
        .map_or(0, |f| f.code)
}

/// Closes the document `view` of the window `handle` with the question about saving, from a flow
/// that may run in another window.
pub(crate) async fn close_document(
    cx: &mut AsyncWindowContext,
    handle: AnyWindowHandle,
    view: &Entity<EditorView>,
) -> bool {
    let weak = view.downgrade();
    if cx.window_handle() == handle {
        return confirm_and_close_window(&weak, cx).await;
    }
    let Ok(mut other) = handle.update(cx, |_, window, app| window.to_async(app)) else {
        return false;
    };
    confirm_and_close_window(&weak, &mut other).await
}

impl Workspace {
    /// Runs a function of Sakura on the active view once this event is handled: the function may
    /// look at the tabs of every window, which cannot happen while this window is borrowed.
    fn run_after_event(&self, code: u16, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor().clone();
        window.defer(cx, move |window, app| {
            editor.update(app, |view, cx| view.run_function(code, &[], window, cx));
        });
    }

    /// The tabs the bar shows, or `None` when the bar is hidden.
    fn bar_entries(&self, window: &Window, cx: &mut Context<Self>) -> Option<Vec<Entry>> {
        let tab_settings = settings::current(cx);
        if !tab_settings.show_bar {
            return None;
        }
        let own = window.window_handle();
        let entry = |handle, view: Entity<EditorView>, active, tab, cx: &gpui::App| Entry {
            caption: Self::caption_of(view.read(cx), cx),
            handle,
            view,
            active,
            tab,
        };
        if tab_settings.grouped {
            let current = self.active_tab();
            return Some(
                self.tab_editors()
                    .into_iter()
                    .enumerate()
                    .map(|(i, view)| entry(own, view, i == current, Some(i), cx))
                    .collect(),
            );
        }
        // Not grouped: every window lists all the documents. This workspace is being drawn, so it
        // is not read through the entity.
        let me = cx.entity_id();
        let mut entries = Vec::new();
        for (handle, workspace) in listed_workspaces(cx) {
            let views = if workspace.entity_id() == me {
                self.tab_editors()
            } else {
                workspace.read(cx).tab_editors()
            };
            for view in views {
                let mine = handle == own;
                entries.push(entry(handle, view, mine, None, cx));
            }
        }
        Some(entries)
    }

    /// The tab bar, if it is shown.
    pub(crate) fn render_tab_bar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let entries = self.bar_entries(window, cx)?;
        let own = window.window_handle();
        let mut bar = div()
            .id("tab-bar")
            .flex()
            .flex_row()
            .w_full()
            .bg(rgb(0xe4e4e4))
            .border_b_1()
            .border_color(rgb(0xc0c0c0))
            .text_size(px(12.))
            .text_color(rgb(0x1e1e1e));
        for (i, entry) in entries.iter().cloned().enumerate() {
            bar = bar.child(self.render_tab(i, entry, own, cx));
        }
        let list_button = div()
            .id("tab-list")
            .px_2()
            .py_0p5()
            .cursor_pointer()
            .hover(|s| s.bg(rgb(0xd0d0d0)))
            .child("▼")
            .on_click(cx.listener(|this, _, window, cx| {
                this.run_after_event(function_code("F_WINLIST"), window, cx);
            }));
        Some(
            bar.child(
                div()
                    .flex_1()
                    // `OnLButtonDblClk`: a double click beside the tabs makes a new document.
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                            if event.click_count >= 2 {
                                this.run_after_event(function_code("F_FILENEW"), window, cx);
                            }
                        }),
                    ),
            )
            .child(list_button)
            .into_any_element(),
        )
    }

    fn render_tab(
        &self,
        index: usize,
        entry: Entry,
        own: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Entry {
            handle,
            view,
            caption,
            active,
            tab,
        } = entry;
        let select = {
            let view = view.clone();
            cx.listener(
                move |this, _: &gpui::MouseDownEvent, window, cx| match tab {
                    Some(tab) => this.activate_tab(tab, window, cx),
                    None if handle != own => flows::reveal(cx, handle, view.clone()),
                    None => {}
                },
            )
        };
        let close = {
            let view = view.clone();
            move |window: &mut Window, cx: &mut gpui::App| {
                let view = view.clone();
                window
                    .spawn(cx, async move |cx| {
                        close_document(cx, handle, &view).await;
                    })
                    .detach();
            }
        };
        let close_for_button = close.clone();
        let mut element = div()
            .id(("tab", index))
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_2()
            .py_0p5()
            .max_w(px(220.))
            .border_r_1()
            .border_color(rgb(0xc0c0c0))
            .bg(if active { rgb(0xffffff) } else { rgb(0xd8d8d8) })
            .when(active, |s| s.font_weight(gpui::FontWeight::SEMIBOLD))
            .on_mouse_down(MouseButton::Left, select)
            // `OnTabMButtonUp`: the middle button closes the tab.
            .on_mouse_up(MouseButton::Middle, move |_, window, cx| close(window, cx))
            .child(
                div()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(caption.clone()),
            )
            .child(
                div()
                    .id(("tab-close", index))
                    .px_1()
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0xc8c8c8)))
                    .child("×")
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        close_for_button(window, cx)
                    }),
            );
        if let Some(from) = tab {
            element = element
                .on_drag(
                    DraggedTab {
                        from,
                        caption: caption.clone(),
                    },
                    |dragged, _, _, cx| cx.new(|_| dragged.clone()),
                )
                .on_drop(cx.listener(move |this, dragged: &DraggedTab, _, cx| {
                    this.move_tab(dragged.from, from, cx);
                }));
        }
        element.into_any_element()
    }
}
