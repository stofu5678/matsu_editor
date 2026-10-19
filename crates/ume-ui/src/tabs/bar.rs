//! The tab bar (`CTabWnd`, `sakura_core/window/CTabWnd.cpp`; the help is
//! `help/sakura/res/HLP000150.html`).
//!
//! * Each tab shows the caption of its document (`GetTabName`, [`Workspace::caption_of`]).
//! * A click makes the tab active (`OnTabLButtonUp`); the close button and a click with the
//!   middle button close the document with the question about saving (`F_WINCLOSE`,
//!   `OnTabLButtonUp`, `OnTabMButtonUp`); a double click on the bar beside the tabs makes a new
//!   document (`OnLButtonDblClk`: `F_FILENEW`); a right click on a tab opens custom menu 24, the
//!   tab menu, for that tab (`OnTabRButtonUp`, `ExecTabCommand`).
//! * The two buttons at the right end: ▼ lists the tabs by their captions on a left click and by
//!   their paths on a right click ([`super::list`], `OnLButtonDown`, `OnRButtonDown`), and × closes
//!   what the settings say (`OnLButtonUp`: [`close_function`]).
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
    InteractiveElement, IntoElement, MouseButton, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px, rgb,
};
use gpui_component::tooltip::Tooltip;
use ume_commands::sakura::FUNCTIONS;

use super::list::{self, Start};
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
    /// The tooltip: the path as 「ファイル名表示」 shows it (`GetTabName` with the full name).
    tooltip: String,
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

/// What the close button at the right end of the bar closes (`CTabWnd::OnLButtonUp`): in the tab
/// mode the current tab, or the whole group with `bTab_CloseOneWin`; else every editor (the close
/// button of the window closes one).
pub(crate) fn close_function(
    tab: settings::TabSettings,
    options: settings::TabOptions,
) -> &'static str {
    match (tab.bound(), options.close_one_win) {
        (true, false) => "F_WINCLOSE",
        (true, true) => "F_GROUPCLOSE",
        (false, _) => "F_EXITALLEDITORS",
    }
}

/// The tooltip of the close button (`OnMouseMove`: `STR_TABWND_CLOSETAB`, the name of the function).
fn close_tooltip(function: &str) -> &'static str {
    match function {
        "F_WINCLOSE" => "タブを閉じる",
        "F_GROUPCLOSE" => "グループを閉じる",
        _ => "編集の全終了",
    }
}

/// `STR_TABWND_LR_INFO`: the tooltip of the ▼ button.
const LIST_TOOLTIP: &str = "左クリック: タブ名一覧\n右クリック: パス名一覧";

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
    pub(crate) fn run_after_event(&self, code: u16, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor().clone();
        window.defer(cx, move |window, app| {
            editor.update(app, |view, cx| view.run_function(code, &[], window, cx));
        });
    }

    /// Opens the tab list over the active view once this event is handled.
    fn open_tab_list(&self, start: Start, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.editor().clone();
        window.defer(cx, move |window, app| {
            editor.update(app, |view, cx| list::open(view, start, window, cx));
        });
    }

    /// A right click on a tab: makes that document the active one, then shows custom menu 24 (the
    /// tab menu) at the pointer, so its functions run on that document (`ExecTabCommand`:
    /// `ShowHideWindow( hWnd, TRUE )`, then the function is posted to the window).
    ///
    /// `tab` is the position of the tab in this window. Without it the tabs are not grouped: the
    /// document is the one of this window, or of another window, which is brought to the front and
    /// opens the menu where its own window has the pointer.
    fn open_tab_menu(
        &mut self,
        tab: Option<usize>,
        handle: AnyWindowHandle,
        view: &Entity<EditorView>,
        at: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let code = function_code("F_CUSTMENU_24");
        if let Some(tab) = tab {
            self.activate_tab(tab, window, cx);
        } else if handle != window.window_handle() {
            flows::reveal_and_run(cx, handle, view.clone(), code);
            return;
        }
        let editor = self.editor().clone();
        // The function may look at the tabs of every window.
        window.defer(cx, move |window, app| {
            editor.update(app, |view, cx| view.run_function_at(code, at, window, cx));
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
            tooltip: Self::tooltip_of(view.read(cx), cx),
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
        let options = settings::options(cx);
        // `m_sTabBar.m_lf`: the font of the text of the tabs (the text size is 12 pixels without one).
        let font = settings::tab_font(cx);
        let mut bar = div()
            .id("tab-bar")
            .flex()
            .flex_row()
            .w_full()
            .bg(rgb(0xe4e4e4))
            .border_b_1()
            .border_color(rgb(0xc0c0c0))
            .text_size(px(font.size_px().unwrap_or(12.)))
            .text_color(rgb(0x1e1e1e));
        if let Some(family) = font.family() {
            bar = bar.font_family(family);
        }
        if let Some(weight) = font.weight() {
            bar = bar.font_weight(gpui::FontWeight(weight));
        }
        if font.italic() {
            bar = bar.italic();
        }
        for (i, entry) in entries.iter().cloned().enumerate() {
            bar = bar.child(self.render_tab(i, entry, own, options, cx));
        }
        // `OnLButtonDown`: the list of the captions opens on the press, as a menu does.
        let list_button = div()
            .id("tab-list")
            .debug_selector(|| "tab-list-button".to_owned())
            .px_2()
            .py_0p5()
            .cursor_pointer()
            .hover(|s| s.bg(rgb(0xd0d0d0)))
            .tooltip(|window, cx| Tooltip::new(LIST_TOOLTIP).build(window, cx))
            .child("▼")
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &gpui::MouseDownEvent, window, cx| {
                    this.open_tab_list(Start::BUTTON, window, cx);
                    cx.stop_propagation();
                }),
            )
            // `OnRButtonDown`: the list of the paths.
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _: &gpui::MouseDownEvent, window, cx| {
                    this.open_tab_list(Start::BUTTON_PATHS, window, cx);
                    cx.stop_propagation();
                }),
            );
        let close_name = close_function(settings::current(cx), options);
        let close_button = div()
            .id("tab-bar-close")
            .debug_selector(|| "tab-bar-close".to_owned())
            .px_2()
            .py_0p5()
            .cursor_pointer()
            .hover(|s| s.bg(rgb(0xd0d0d0)))
            .tooltip(move |window, cx| Tooltip::new(close_tooltip(close_name)).build(window, cx))
            .child("×")
            .on_click(cx.listener(move |this, _, window, cx| {
                this.run_after_event(function_code(close_name), window, cx);
            }));
        // `m_bChgWndByWheel`: the wheel over the bar goes to the next or previous window
        // (`CEditWnd::DoMouseWheel`: a turn toward the user is the next one).
        if options.change_by_wheel {
            bar = bar.on_scroll_wheel(cx.listener(
                |this, event: &gpui::ScrollWheelEvent, window, cx| {
                    let y = event.delta.pixel_delta(px(16.)).y;
                    // A trackpad sends many small moves; only a real move counts.
                    let least = if event.delta.precise() {
                        px(50.)
                    } else {
                        px(1.)
                    };
                    if y.abs() < least {
                        return;
                    }
                    let code = function_code(if y < px(0.) {
                        "F_NEXTWINDOW"
                    } else {
                        "F_PREVWINDOW"
                    });
                    this.run_after_event(code, window, cx);
                    cx.stop_propagation();
                },
            ));
        }
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
            .child(close_button)
            .into_any_element(),
        )
    }

    fn render_tab(
        &self,
        index: usize,
        entry: Entry,
        own: AnyWindowHandle,
        options: settings::TabOptions,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Entry {
            handle,
            view,
            caption,
            tooltip,
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
        // `OnTabRButtonUp`: `ExecTabCommand( F_CUSTMENU_BASE + CUSTMENU_INDEX_FOR_TABWND )` activates
        // the tab, then sends the function to its window, which opens the menu at the pointer.
        let open_menu = {
            let view = view.clone();
            cx.listener(move |this, event: &gpui::MouseUpEvent, window, cx| {
                this.open_tab_menu(tab, handle, &view, event.position, window, cx);
                cx.stop_propagation();
            })
        };
        let group = SharedString::from(format!("tab-group-{index}"));
        let mut element = div()
            .id(("tab", index))
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_2()
            .py_0p5()
            .group(group.clone())
            // `m_bSameTabWidth`: every tab is as wide as the others.
            .when(!options.same_width, |tab| tab.max_w(px(220.)))
            .when(options.same_width, |tab| tab.w(px(160.)).flex_none())
            .border_r_1()
            .border_color(rgb(0xc0c0c0))
            .bg(if active { rgb(0xffffff) } else { rgb(0xd8d8d8) })
            .when(active, |s| s.font_weight(gpui::FontWeight::SEMIBOLD))
            .debug_selector(move || format!("tab-{index}"))
            .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            .on_mouse_down(MouseButton::Left, select)
            .on_mouse_up(MouseButton::Right, open_menu)
            // `OnTabMButtonUp`: the middle button closes the tab.
            .on_mouse_up(MouseButton::Middle, move |_, window, cx| close(window, cx))
            .child(
                div()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(caption.clone()),
            )
            // `m_bDispTabClose`: never, on every tab, or on the tab the pointer is over.
            .when(options.disp_close != settings::DispTabClose::No, |tab| {
                let auto = options.disp_close == settings::DispTabClose::Auto;
                let hover_group = group.clone();
                tab.child(
                    div()
                        .id(("tab-close", index))
                        .px_1()
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0xc8c8c8)))
                        .when(auto, |button| {
                            button
                                .invisible()
                                .group_hover(hover_group, |style| style.visible())
                        })
                        .child("×")
                        .on_click(move |_, window, cx| {
                            cx.stop_propagation();
                            close_for_button(window, cx)
                        }),
                )
            });
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
