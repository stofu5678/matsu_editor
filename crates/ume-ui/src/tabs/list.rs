//! The tab list: Sakura's `CTabWnd::TabListMenu` (`sakura_core/window/CTabWnd.cpp`), the menu of
//! the ▼ button at the right end of the tab bar and of `F_WINLIST` while the bar is shown
//! (`CEditWnd::PopupWinList`).
//!
//! The menu lists the tabs of the group: the text of each tab (`GetTabName`: the caption of the
//! tab, or the path as 「ファイル名表示」 shows it when the paths are listed), sorted by that text
//! if `bSortTabList` is on, with a check mark on the tab of this window. Choosing an item makes
//! that tab the active one. The group is the tabs this window's bar shows; the tabs of the other
//! windows are the other groups: the list from the button leaves them out behind an item
//! 「すべて表示」, the list of `F_WINLIST` shows them after a separator. Besides them, the list of
//! `F_WINLIST` has an item that switches between the captions and the paths
//! (「パス名一覧に切替える」, 「タブ名一覧に切替える」) and keeps the choice in `bTab_ListFull`.
//!
//! The menu is drawn over the editor window, like the window list (`crate::window_ops::list`): the
//! OS has no popup menu that GPUI could show at a place of our choosing. It is at the top right
//! of the view, under the ▼ button. Keys: Up, Down, Home, End, Enter, Esc, and `W` and `A` for
//! the two special items (their access keys).

use crate::dialog_focus::KeepTabInside as _;
use gpui::{
    AnyWindowHandle, App, AppContext as _, ClickEvent, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseDownEvent, ParentElement, Render,
    ScrollHandle, StatefulInteractiveElement, Styled, WeakEntity, Window, div, px, rgb,
};

use super::settings;
use crate::app::Workspace;
use crate::file_ops::windows::{listed_workspaces, workspace_of};
use crate::view::EditorView;

/// `STR_TABWND_SHOWALL`; the access key is shown in parentheses, as the menus of this program do.
const SHOW_ALL: &str = "すべて表示(A)";
/// `STR_TABWND_SHOWTABNAME`, `STR_TABWND_SHOWPATHNAME`.
const SHOW_NAMES: &str = "タブ名一覧に切替える(W)";
const SHOW_PATHS: &str = "パス名一覧に切替える(W)";

/// A tab in the list.
pub(crate) struct Item<T> {
    pub text: String,
    /// The tab of this window (the radio mark).
    pub current: bool,
    pub target: T,
}

/// A row of the menu.
pub(crate) enum Row<T> {
    /// The item that switches between the captions and the paths.
    Switch {
        to_names: bool,
    },
    Separator,
    Tab(Item<T>),
    /// The item that shows the tabs of the other groups.
    ShowAll,
}

/// How the menu is made (the arguments of `TabListMenu`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Layout {
    /// `m_bSortTabList`.
    pub sort: bool,
    /// `bFull`: the paths, not the captions.
    pub full: bool,
    /// `bSel`: the item that switches the display.
    pub with_switch: bool,
    /// `bOtherGroup`: the tabs of the other groups are listed.
    pub all_groups: bool,
}

/// `compTABMENU_DATA`: the words are sorted as `lstrcmp` does, "ABC" next to "abc" (the lower case
/// first) and "XYZ" after them; equal texts keep their order.
///
/// Sakura's `std::sort` calls `compTABMENU_DATA` and uses its int as "less", and ties are broken by
/// `iItem`, so any two items are "less" both ways: not a valid order, and real Sakura's sorted
/// list is whatever the sort makes of it. This follows what the comparator means, on purpose.
fn compare<T>(a: &(usize, Item<T>), b: &(usize, Item<T>)) -> std::cmp::Ordering {
    let fold = |s: &str| s.chars().flat_map(char::to_lowercase).collect::<String>();
    fold(&a.1.text)
        .cmp(&fold(&b.1.text))
        .then_with(|| b.1.text.cmp(&a.1.text))
        .then_with(|| a.0.cmp(&b.0))
}

fn sorted<T>(items: Vec<Item<T>>, sort: bool) -> Vec<Item<T>> {
    let mut numbered: Vec<_> = items.into_iter().enumerate().collect();
    if sort {
        numbered.sort_by(compare);
    }
    numbered.into_iter().map(|(_, item)| item).collect()
}

/// The rows of the menu (`TabListMenu`): the switch and a separator, the tabs of the group, then
/// if the other groups have tabs a separator and these tabs or the item that shows them.
pub(crate) fn arrange<T>(own: Vec<Item<T>>, others: Vec<Item<T>>, layout: Layout) -> Vec<Row<T>> {
    let mut rows = Vec::new();
    if layout.with_switch {
        rows.push(Row::Switch {
            to_names: layout.full,
        });
        rows.push(Row::Separator);
    }
    rows.extend(sorted(own, layout.sort).into_iter().map(Row::Tab));
    if !others.is_empty() {
        rows.push(Row::Separator);
        if layout.all_groups {
            rows.extend(sorted(others, layout.sort).into_iter().map(Row::Tab));
        } else {
            rows.push(Row::ShowAll);
        }
    }
    rows
}

/// What a tab of the list leads to.
#[derive(Clone)]
pub(crate) struct Target {
    handle: AnyWindowHandle,
    view: Entity<EditorView>,
}

/// The text of a tab (`GetTabName`): the caption, or with `full` the path of the file as
/// 「ファイル名表示」 shows it, not cut to a width.
pub(crate) fn text_of(view: &EditorView, full: bool, cx: &App) -> String {
    let path = view.document().read(cx).path();
    match (full, path) {
        (true, Some(_)) => crate::file_names::current(cx).name_of(path, None, false),
        _ => Workspace::caption_of(view, cx),
    }
}

/// The tabs of the group of the window `owner` and those of the other groups. With the windows
/// grouped the group is the tabs of the window; else every window has one document and the bar
/// lists the documents of all windows, so the group is all of them.
fn collect(
    cx: &mut App,
    owner: AnyWindowHandle,
    full: bool,
) -> (Vec<Item<Target>>, Vec<Item<Target>>) {
    let grouped = settings::current(cx).grouped;
    let shown = workspace_of(cx, owner).map(|ws| ws.read(cx).editor().clone());
    let (mut own, mut others) = (Vec::new(), Vec::new());
    for (handle, workspace) in listed_workspaces(cx) {
        let in_group = !grouped || handle == owner;
        for view in workspace.read(cx).tab_editors() {
            let item = Item {
                text: text_of(view.read(cx), full, cx),
                current: in_group && handle == owner && shown.as_ref() == Some(&view),
                target: Target { handle, view },
            };
            if in_group {
                own.push(item);
            } else {
                others.push(item);
            }
        }
    }
    (own, others)
}

/// How a list is opened: the arguments of `TabListMenu`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Start {
    /// The paths, not the captions; none for the choice kept in `bTab_ListFull`.
    pub full: Option<bool>,
    pub with_switch: bool,
    pub all_groups: bool,
}

impl Start {
    /// The ▼ button (`OnLButtonDown`): the captions, the group only.
    pub(crate) const BUTTON: Self = Self {
        full: Some(false),
        with_switch: false,
        all_groups: false,
    };
    /// A right click on the ▼ button (`OnRButtonDown`): the paths, the group only.
    pub(crate) const BUTTON_PATHS: Self = Self {
        full: Some(true),
        with_switch: false,
        all_groups: false,
    };
    /// `F_WINLIST` (`PopupWinList`): the choice that is kept, every group, the switch.
    pub(crate) const FUNCTION: Self = Self {
        full: None,
        with_switch: true,
        all_groups: true,
    };
}

pub(crate) struct TabList {
    /// The window that shows the list.
    owner: AnyWindowHandle,
    view: WeakEntity<EditorView>,
    full: bool,
    with_switch: bool,
    all_groups: bool,
    /// The row the list is on.
    selected: usize,
    /// The list has moved to the tab of this window once, when it was first drawn.
    placed: bool,
    focus: FocusHandle,
    scroll: ScrollHandle,
}

impl TabList {
    fn layout(&self, cx: &App) -> Layout {
        Layout {
            sort: settings::options(cx).sort_list,
            full: self.full,
            with_switch: self.with_switch,
            all_groups: self.all_groups,
        }
    }

    fn rows(&self, cx: &mut App) -> Vec<Row<Target>> {
        let (own, others) = collect(cx, self.owner, self.full);
        arrange(own, others, self.layout(cx))
    }

    /// The rows as text, for the tests: the text of a tab (`*` first for the tab of this window),
    /// `-` for a separator, `[switch:names]`, `[switch:paths]` and `[all]` for the items.
    #[cfg(test)]
    pub(crate) fn shape(&self, cx: &mut App) -> Vec<String> {
        self.rows(cx)
            .iter()
            .map(|row| match row {
                Row::Switch { to_names: true } => "[switch:names]".to_owned(),
                Row::Switch { to_names: false } => "[switch:paths]".to_owned(),
                Row::Separator => "-".to_owned(),
                Row::Tab(item) if item.current => format!("*{}", item.text),
                Row::Tab(item) => item.text.clone(),
                Row::ShowAll => "[all]".to_owned(),
            })
            .collect()
    }

    /// The row the list is on, for the tests.
    #[cfg(test)]
    pub(crate) fn selected(&self) -> usize {
        self.selected
    }

    /// Closes the list and gives the focus back to the editor.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.tab_list = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    /// Does what the row says: makes the tab the active one, or changes the list.
    fn choose(&mut self, row: &Row<Target>, window: &mut Window, cx: &mut Context<Self>) {
        match row {
            Row::Tab(item) => {
                let Target { handle, view } = item.target.clone();
                self.close(window, cx);
                // The window and the tab.
                super::flows::reveal(cx, handle, view);
            }
            Row::Switch { .. } => {
                // `bFull = !bFull`: the menu is made again, and the choice is kept.
                self.full = !self.full;
                settings::set_list_full(cx, self.full);
                cx.notify();
            }
            Row::ShowAll => {
                self.all_groups = true;
                cx.notify();
            }
            Row::Separator => {}
        }
    }

    fn move_selection(&mut self, rows: &[Row<Target>], forward: bool) {
        let selectable = |i: usize| !matches!(rows[i], Row::Separator);
        let next = if forward {
            (self.selected + 1..rows.len()).find(|i| selectable(*i))
        } else {
            (0..self.selected).rev().find(|i| selectable(*i))
        };
        if let Some(next) = next {
            self.selected = next;
        }
    }

    /// The row that is chosen with the access key `key` (`W` and `A`).
    fn row_of_key(rows: &[Row<Target>], key: &str) -> Option<usize> {
        rows.iter().position(|row| match row {
            Row::Switch { .. } => key.eq_ignore_ascii_case("w"),
            Row::ShowAll => key.eq_ignore_ascii_case("a"),
            _ => false,
        })
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.rows(cx);
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        match key {
            "escape" => self.close(window, cx),
            "up" => self.move_selection(&rows, false),
            "down" => self.move_selection(&rows, true),
            "home" => {
                self.selected = rows
                    .iter()
                    .position(|r| !matches!(r, Row::Separator))
                    .unwrap_or(0)
            }
            "end" => {
                self.selected = rows
                    .iter()
                    .rposition(|r| !matches!(r, Row::Separator))
                    .unwrap_or(0);
            }
            "enter" => {
                if let Some(row) = rows.get(self.selected) {
                    self.choose(row, window, cx);
                }
            }
            _ if !modifiers.control && !modifiers.alt && !modifiers.platform => {
                match Self::row_of_key(&rows, key) {
                    Some(i) => {
                        self.selected = i;
                        self.choose(&rows[i], window, cx);
                    }
                    None => return,
                }
            }
            _ => return,
        }
        self.scroll.scroll_to_item(self.selected);
        cx.stop_propagation();
        cx.notify();
    }
}

impl Focusable for TabList {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

const TEXT: u32 = 0x222222;
const SELECTED_BG: u32 = 0x2f6fd0;

impl Render for TabList {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.rows(cx);
        if !self.placed {
            self.placed = true;
            self.selected = rows
                .iter()
                .position(|row| matches!(row, Row::Tab(item) if item.current))
                .or_else(|| rows.iter().position(|r| !matches!(r, Row::Separator)))
                .unwrap_or(0);
            // A long list opens scrolled to the tab of this window.
            self.scroll.scroll_to_item(self.selected);
        }
        self.selected = self.selected.min(rows.len().saturating_sub(1));
        let items: Vec<_> = rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                if matches!(row, Row::Separator) {
                    return div()
                        .id(("tab-list-separator", i))
                        .role(gpui::Role::Splitter)
                        .my_1()
                        .h(px(1.))
                        .bg(rgb(0xaaaaaa))
                        .into_any_element();
                }
                let (text, mark) = match row {
                    Row::Switch { to_names } => (
                        if *to_names { SHOW_NAMES } else { SHOW_PATHS }.to_owned(),
                        false,
                    ),
                    Row::ShowAll => (SHOW_ALL.to_owned(), false),
                    Row::Tab(item) => (item.text.clone(), item.current),
                    Row::Separator => unreachable!("handled above"),
                };
                let mut item = crate::a11y::list_row(
                    div().id(("tab-list-item", i)),
                    &text,
                    i == self.selected,
                )
                .px_2()
                .h(px(24.))
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer();
                if i == self.selected {
                    item = item.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff));
                }
                if mark {
                    item = item.aria_description("現在のタブ");
                }
                item.child(div().w(px(14.)).child(if mark { "✓" } else { "" }))
                    .child(text)
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        let rows = this.rows(cx);
                        if let Some(row) = rows.get(i) {
                            this.selected = i;
                            this.choose(row, window, cx);
                        }
                    }))
                    .into_any_element()
            })
            .collect();

        let max_rows = (window.viewport_size().height - px(24.)).max(px(48.));
        let panel = crate::a11y::group(div().id("tab-list"), "タブの一覧")
            .key_context("TabList")
            .track_focus(&self.focus)
            .keep_tab_inside(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .min_w(px(200.))
            .max_w(px(640.))
            .p_1()
            .flex()
            .flex_col()
            .bg(rgb(0xf0f0f0))
            .border_1()
            .border_color(rgb(0x888888))
            .text_color(rgb(TEXT))
            .text_size(px(14.))
            .child(
                div()
                    .id("tab-list-rows")
                    .role(gpui::Role::ListBox)
                    .aria_label("タブ")
                    .max_h(max_rows)
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .children(items),
            )
            // A click outside the menu closes it.
            .on_mouse_down_out(cx.listener(|this, _: &MouseDownEvent, window, cx| {
                this.close(window, cx);
            }));
        div()
            .id("tab-list-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .flex()
            .items_start()
            .justify_end()
            .child(panel)
    }
}

/// Shows the list over `view`. A list that is open is closed instead of opening a second one.
pub(crate) fn open(
    view: &mut EditorView,
    start: Start,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) {
    if view.tab_list.take().is_some() {
        window.focus(&view.focus_handle, cx);
        cx.notify();
        return;
    }
    let owner = window.window_handle();
    let weak = cx.weak_entity();
    let full = start
        .full
        .unwrap_or_else(|| settings::options(cx).list_full);
    let list = cx.new(|cx| {
        // Not `collect`: the view of the owner is being updated, and its text cannot be read. The
        // list starts on the first row; the first render moves it to the tab of this window.
        TabList {
            owner,
            view: weak,
            full,
            with_switch: start.with_switch,
            all_groups: start.all_groups,
            selected: 0,
            placed: false,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
        }
    });
    let focus = list.read(cx).focus.clone();
    window.focus(&focus, cx);
    view.tab_list = Some(list);
    cx.notify();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(texts: &[&str]) -> Vec<Item<usize>> {
        texts
            .iter()
            .enumerate()
            .map(|(i, text)| Item {
                text: (*text).to_owned(),
                current: i == 0,
                target: i,
            })
            .collect()
    }

    fn layout() -> Layout {
        Layout {
            sort: false,
            full: false,
            with_switch: false,
            all_groups: false,
        }
    }

    /// The rows as short text: the tabs by their text, `-` for a separator, `W` and `A` for the
    /// two items.
    fn shape(rows: &[Row<usize>]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Switch { to_names: true } => "W:names".to_owned(),
                Row::Switch { to_names: false } => "W:paths".to_owned(),
                Row::Separator => "-".to_owned(),
                Row::Tab(item) => item.text.clone(),
                Row::ShowAll => "A".to_owned(),
            })
            .collect()
    }

    #[test]
    fn the_tabs_keep_the_order_of_the_bar_unless_the_setting_sorts_them() {
        let rows = arrange(items(&["b", "c", "a"]), vec![], layout());
        assert_eq!(shape(&rows), ["b", "c", "a"]);
        let sort = Layout {
            sort: true,
            ..layout()
        };
        assert_eq!(
            shape(&arrange(items(&["b", "c", "a"]), vec![], sort)),
            ["a", "b", "c"]
        );
    }

    #[test]
    fn the_sort_puts_upper_and_lower_case_side_by_side_like_lstrcmp() {
        let sort = Layout {
            sort: true,
            ..layout()
        };
        let rows = arrange(items(&["XYZ", "abc", "ABC", "Abe", "あ"]), vec![], sort);
        assert_eq!(shape(&rows), ["abc", "ABC", "Abe", "XYZ", "あ"]);
    }

    #[test]
    fn equal_texts_keep_their_order() {
        let sort = Layout {
            sort: true,
            ..layout()
        };
        let rows = arrange(items(&["(無題)", "a", "(無題)"]), vec![], sort);
        let targets: Vec<usize> = rows
            .iter()
            .filter_map(|row| match row {
                Row::Tab(item) => Some(item.target),
                _ => None,
            })
            .collect();
        assert_eq!(targets, [0, 2, 1]);
    }

    #[test]
    fn the_other_groups_are_behind_an_item_or_after_a_separator() {
        let hidden = arrange(items(&["a"]), items(&["z", "y"]), layout());
        assert_eq!(shape(&hidden), ["a", "-", "A"]);
        let all = Layout {
            all_groups: true,
            ..layout()
        };
        assert_eq!(
            shape(&arrange(items(&["a"]), items(&["z", "y"]), all)),
            ["a", "-", "z", "y"]
        );
        // Each group is sorted by itself.
        let both = Layout {
            all_groups: true,
            sort: true,
            ..layout()
        };
        assert_eq!(
            shape(&arrange(items(&["b", "a"]), items(&["z", "y"]), both)),
            ["a", "b", "-", "y", "z"]
        );
        // No other group, no separator.
        assert_eq!(shape(&arrange(items(&["a"]), vec![], all)), ["a"]);
    }

    #[test]
    fn the_switch_names_the_display_it_changes_to() {
        let with = Layout {
            with_switch: true,
            ..layout()
        };
        assert_eq!(
            shape(&arrange(items(&["a"]), vec![], with)),
            ["W:paths", "-", "a"]
        );
        let full = Layout { full: true, ..with };
        assert_eq!(
            shape(&arrange(items(&["a"]), vec![], full)),
            ["W:names", "-", "a"]
        );
    }
}
