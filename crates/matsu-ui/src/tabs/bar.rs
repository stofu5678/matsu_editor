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
//! * A tab can be dragged to another place of the bar (`ReorderTab`). A tab dropped outside the
//!   window goes to the window under the pointer, or makes a window of its own where it was
//!   dropped (`OnTabLButtonUp`: `SeparateGroup`; [`Workspace::finish_tab_drag`]). GPUI cannot drop
//!   a drag into another window, so the drop is found from the position of the pointer on the
//!   screen.
//! * The settings of the page タブバー: icons (`bDispTabIcon`, [`super::icon`]), several rows
//!   (`bTabMultiLine`), the place of the bar (`eTabPosition`) and the widths of the tabs
//!   (`nTabMaxWidth`, `nTabMinWidth`, `nTabMinWidthOnMulti`, [`settings::TabOptions::width_limits`]).
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
use matsu_commands::sakura::FUNCTIONS;

use std::rc::Rc;

use super::content::TabPanel;
use super::icon::TabIcon;
use super::list::{self, Start};
use super::{flows, semantics, settings};
use crate::app::Workspace;
use crate::file_ops::close::confirm_and_close_window;
use crate::file_ops::windows::listed_workspaces;
use crate::view::EditorView;

/// What closing a tab does.
type CloseTab = Rc<dyn Fn(&mut Window, &mut gpui::App)>;

/// A tab of the bar.
#[derive(Clone)]
struct Entry {
    handle: AnyWindowHandle,
    view: Entity<EditorView>,
    caption: String,
    /// The name for assistive technology (`TabPanel::accessible_name`).
    name: String,
    /// The tooltip: the path as 「ファイル名表示」 shows it (`GetTabName` with the full name).
    tooltip: String,
    active: bool,
    /// The position among the tabs of this window, when it is one of them.
    tab: Option<usize>,
    /// The tab is a preview (`crate::preview`): its `view` is the hidden view of its document, and
    /// closing it asks nothing.
    preview: bool,
    /// The icon the tab shows with `bDispTabIcon`.
    icon: TabIcon,
}

/// Where the drag of a tab began, kept from its start to the release of the button
/// (`m_nSrcTab`, `m_ptSrcCursor` of `CTabWnd`). GPUI tells the elements under the pointer about a
/// drop, and a drop outside the window has none, so the window finds out itself
/// ([`Workspace::finish_tab_drag`]).
#[derive(Clone, Copy)]
pub(crate) struct TabDragOrigin {
    pub window: AnyWindowHandle,
    pub index: usize,
    /// The pointer in the window when the drag began.
    pub start: gpui::Point<gpui::Pixels>,
}

/// The drag of a tab that is going on, if any.
#[derive(Default)]
pub(crate) struct TabDrag(pub Option<TabDragOrigin>);

impl gpui::Global for TabDrag {}

/// The drag of a tab ends: forgets where it began and returns that.
pub(crate) fn end_tab_drag(cx: &mut gpui::App) -> Option<TabDragOrigin> {
    cx.default_global::<TabDrag>().0.take()
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
pub(super) fn close_tooltip(function: &str) -> &'static str {
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
        // From the search box the focus goes to the view first (`CEditWnd::OnCommand`).
        self.leave_search_box(window, cx);
        // The view of a preview is not drawn, so closing "the document" there closes the preview.
        if self.content.preview().is_some() && code == function_code("F_WINCLOSE") {
            let workspace = cx.entity();
            window.defer(cx, move |window, app| {
                workspace.update(app, |ws, cx| ws.close_active_preview(window, cx));
            });
            return;
        }
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
        let entry = |handle,
                     view: Entity<EditorView>,
                     panel: &dyn TabPanel,
                     active,
                     tab,
                     preview,
                     cx: &gpui::App| Entry {
            icon: panel.icon(cx),
            caption: panel.caption(cx),
            name: panel.accessible_name(cx),
            tooltip: panel.tooltip(cx),
            handle,
            view,
            active,
            tab,
            preview,
        };
        if tab_settings.grouped {
            let current = self.active_tab();
            return Some(
                (0..self.tab_count())
                    .filter_map(|i| {
                        let (view, panel) = (self.tab_bar_view(i)?, self.tab_panel(i)?);
                        let preview = self.tab_content(i)?.preview().is_some();
                        Some(entry(own, view, panel, i == current, Some(i), preview, cx))
                    })
                    .collect(),
            );
        }
        // Not grouped: every window lists all the documents. This workspace is being drawn, so it
        // is not read through the entity.
        let me = cx.entity_id();
        let mut entries = Vec::new();
        for (handle, workspace) in listed_workspaces(cx) {
            let tabs = if workspace.entity_id() == me {
                self
            } else {
                workspace.read(cx)
            };
            // A window that has a preview has two tabs for one document; its own entries are tabs
            // of this window then, so that a click selects one of them. A preview of another
            // window is not listed (it cannot be revealed from here).
            let mine = handle == own;
            let own_tabs = mine && tabs.has_preview_tab();
            for i in 0..tabs.tab_count() {
                let (Some(view), Some(panel)) = (tabs.tab_bar_view(i), tabs.tab_panel(i)) else {
                    continue;
                };
                let preview = tabs.tab_content(i).is_some_and(|c| c.preview().is_some());
                if preview && !mine {
                    continue;
                }
                let (active, tab) = if own_tabs {
                    (i == tabs.active_tab(), Some(i))
                } else {
                    (mine, None)
                };
                entries.push(entry(handle, view, panel, active, tab, preview, cx));
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
        // A hidden close button paints no mouse listeners, so a screen reader's Click on it would
        // miss: while one is connected the button stays visible (it still shows only on hover
        // for everyone else).
        let keep_close_visible = window.is_a11y_active();
        // `m_sTabBar.m_lf`: the font of the text of the tabs (the text size is 12 pixels without one).
        let font = settings::tab_font(cx);
        // `m_eTabPosition`: the border is on the side that faces the editor.
        let bottom = options.position == settings::TabPosition::Bottom;
        let mut bar = semantics::bar(div().id("tab-bar"), entries.len())
            .flex()
            .flex_row()
            // With several rows the buttons keep the height of one (`DrawListBtn`: at the top).
            .items_start()
            .w_full()
            .bg(rgb(0xe4e4e4))
            .when(bottom, |bar| bar.border_t_1())
            .when(!bottom, |bar| bar.border_b_1())
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
        // The tabs: wrapped into rows with `bTabMultiLine` (`TCS_MULTILINE`), else in one row that
        // scrolls when they do not fit (the tab control shows scroll arrows).
        let mut strip = div()
            .id("tab-strip")
            .debug_selector(|| "tab-strip".to_owned())
            .flex()
            .flex_row()
            .flex_1()
            .min_w_0();
        strip = if options.multi_line {
            strip.flex_wrap()
        } else {
            strip.overflow_x_scroll()
        };
        for (i, entry) in entries.iter().cloned().enumerate() {
            strip = strip.child(self.render_tab(i, entry, own, options, keep_close_visible, cx));
        }
        // `OnLButtonDown`: the list of the captions opens on the press, as a menu does.
        let list_button = semantics::button(div().id("tab-list"), semantics::LIST_BUTTON_NAME)
            .debug_selector(|| "tab-list-button".to_owned())
            .px_2()
            .py_0p5()
            .cursor_pointer()
            .hover(|s| s.bg(rgb(0xd0d0d0)))
            .tooltip(|window, cx| Tooltip::new(LIST_TOOLTIP).build(window, cx))
            .child("▼")
            .on_a11y_action(gpui::AccessibleAction::Click, {
                let workspace = cx.entity();
                move |_, window, cx| {
                    workspace.update(cx, |this, cx| {
                        this.open_tab_list(Start::BUTTON, window, cx);
                    });
                }
            })
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
        let close_button = semantics::button(div().id("tab-bar-close"), close_tooltip(close_name))
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
        // `OnLButtonDblClk`: a double click beside the tabs makes a new document. Several rows fill
        // the width (the tab control stretches them), so only a single row has room beside them.
        if !options.multi_line {
            // Equal-width tabs take the room first.
            let room = div().flex_grow(0.001).on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                    if event.click_count >= 2 {
                        this.run_after_event(function_code("F_FILENEW"), window, cx);
                    }
                }),
            );
            strip = strip.child(room);
        }
        Some(
            bar.child(strip)
                .child(list_button)
                .child(close_button)
                .into_any_element(),
        )
    }

    /// The left button went up outside the elements of the window: if a tab was being dragged and the
    /// pointer is outside the window, the tab leaves it (`OnTabLButtonUp`, `DRAG_DRAG` with the
    /// drop outside the tab bar and outside this window).
    ///
    /// Where the button went up decides (`SeparateGroup`): over another window of the editor the
    /// tab joins its tabs, anywhere else it makes a window of its own where it was dropped. Only in
    /// the tab mode (`bDispTabWnd && !bDispTabWndMultiWin`); a window with one tab has nothing to
    /// separate from, but the tab can still join another window.
    pub(crate) fn finish_tab_drag(
        &mut self,
        position: gpui::Point<gpui::Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let origin = end_tab_drag(cx);
        let Some(origin) = origin.filter(|o| o.window == window.window_handle()) else {
            return;
        };
        let inside = gpui::Bounds::new(gpui::Point::default(), window.viewport_size());
        if inside.contains(&position) || !cx.has_active_drag() {
            return;
        }
        window.defer(cx, move |window, app| {
            flows::drag_tab_out(app, window, origin, position);
        });
    }

    /// Whether the tabs have a tooltip: not while a popup menu (the tab menu) is open.
    pub(crate) fn tab_tooltips_shown(&self, cx: &gpui::App) -> bool {
        !self.editor().read(cx).custom_menu.is_open()
    }

    fn render_tab(
        &self,
        index: usize,
        entry: Entry,
        own: AnyWindowHandle,
        options: settings::TabOptions,
        keep_close_visible: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Entry {
            handle,
            view,
            caption,
            name,
            tooltip,
            active,
            tab,
            preview,
            icon,
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
        // The same for a screen reader's Click action (the press above is a mouse event, which
        // does not advertise one).
        let select_by_a11y = {
            let workspace = cx.entity();
            let view = view.clone();
            move |_: Option<&gpui::accesskit::ActionData>,
                  window: &mut Window,
                  cx: &mut gpui::App| {
                let view = view.clone();
                workspace.update(cx, |this, cx| match tab {
                    Some(tab) => this.activate_tab(tab, window, cx),
                    None if handle != own => flows::reveal(cx, handle, view),
                    None => {}
                });
            }
        };
        // A preview has no document to ask about: its tab just goes.
        let close: CloseTab = match tab {
            Some(index) if preview => {
                let workspace = cx.entity();
                Rc::new(move |window: &mut Window, cx: &mut gpui::App| {
                    let workspace = workspace.clone();
                    window.defer(cx, move |window, cx| {
                        workspace.update(cx, |ws, cx| ws.close_tab_or_window(index, window, cx));
                    });
                })
            }
            _ => {
                let view = view.clone();
                Rc::new(move |window: &mut Window, cx: &mut gpui::App| {
                    let view = view.clone();
                    window
                        .spawn(cx, async move |cx| {
                            close_document(cx, handle, &view).await;
                        })
                        .detach();
                })
            }
        };
        let close_for_button = close.clone();
        // `OnTabRButtonUp`: `ExecTabCommand( F_CUSTMENU_BASE + CUSTMENU_INDEX_FOR_TABWND )` activates
        // the tab, then sends the function to its window, which opens the menu at the pointer.
        let open_menu = {
            let view = view.clone();
            cx.listener(move |this, event: &gpui::MouseUpEvent, window, cx| {
                // The tab menu is the editor's (custom menu 24): a preview has none.
                if !preview {
                    this.open_tab_menu(tab, handle, &view, event.position, window, cx);
                }
                cx.stop_propagation();
            })
        };
        let (min_width, max_width) = options.width_limits();
        let group = SharedString::from(format!("tab-group-{index}"));
        let mut element = semantics::tab(div().id(("tab", index)), &name, active, index)
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_2()
            .py_0p5()
            .group(group.clone())
            // The widths (`CTabWnd::Refresh`, `TabCtrl_SetItemSize`): between `nTabMinWidth` (or
            // `nTabMinWidthOnMulti` in several rows) and `nTabMaxWidth`. `m_bSameTabWidth`: every tab
            // gets the same share of the row, `(width - 8) / count` before the limits (`flex: 1 1 0`).
            // Else a tab is as wide as its text, shrinks when the row is full and, in several rows,
            // grows to fill its row.
            .min_w(px(min_width as f32))
            .max_w(px(max_width as f32))
            .when(options.same_width, |tab| tab.flex_1())
            .when(!options.same_width, |tab| tab.flex_shrink_1())
            .when(!options.same_width && options.multi_line, |tab| {
                tab.flex_grow(1.)
            })
            .border_r_1()
            .border_color(rgb(0xc0c0c0))
            .bg(if active { rgb(0xffffff) } else { rgb(0xd8d8d8) })
            .when(active, |s| s.font_weight(gpui::FontWeight::SEMIBOLD))
            .debug_selector(move || format!("tab-{index}"))
            // A popup menu is modal in Sakura, so no tooltip is shown over it. GPUI keeps a shown
            // tooltip until the pointer leaves the element, and paints it above the menu that
            // opens at the pointer, so the tabs have none while a menu is open.
            .when(self.tab_tooltips_shown(cx), move |tab| {
                tab.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            })
            .on_mouse_down(MouseButton::Left, select)
            .on_a11y_action(gpui::AccessibleAction::Click, select_by_a11y)
            .on_mouse_up(MouseButton::Right, open_menu)
            // `OnTabMButtonUp`: the middle button closes the tab.
            .on_mouse_up(MouseButton::Middle, move |_, window, cx| close(window, cx))
            // `m_bDispTabIcon`: the icon is left of the text.
            .when(options.disp_icon, |tab| tab.child(icon.element()))
            .child(
                div()
                    .debug_selector(move || format!("tab-caption-{index}"))
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(caption.clone()),
            )
            // `m_bDispTabClose`: never, on every tab, or on the tab the pointer is over.
            .when(options.disp_close != settings::DispTabClose::No, |tab| {
                let auto =
                    options.disp_close == settings::DispTabClose::Auto && !keep_close_visible;
                let hover_group = group.clone();
                tab.child(
                    semantics::tab_close_button(div().id(("tab-close", index)), &name)
                        .px_1()
                        .flex_none()
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
                    |dragged, _, window, cx| {
                        // `m_ptSrcCursor`: where the drag began, for the window a drop outside
                        // the window makes.
                        cx.set_global(TabDrag(Some(TabDragOrigin {
                            window: window.window_handle(),
                            index: dragged.from,
                            start: window.mouse_position(),
                        })));
                        cx.new(|_| dragged.clone())
                    },
                )
                .on_drop(cx.listener(move |this, dragged: &DraggedTab, _, cx| {
                    this.move_tab(dragged.from, from, cx);
                }));
        }
        element.into_any_element()
    }
}
