//! `F_FAVORITE`: the window that manages the history (履歴の管理).
//!
//! Sakura: `CDlgFavorite` (`sakura_core/dlg/CDlgFavorite.cpp`) is a dialog with a tab for each
//! history. Each has a list with a check box (お気に入り, favorite) for each item and the buttons
//! "すべて", "お気に入り以外", "存在しない項目", "選択項目" (delete) and "閉じる". An item beyond the
//! number that is shown in the menu is marked "(非表示)".
//!
//! Only the file and the folder history exist in this program (the search, replace, Grep,
//! command and current folder histories, and the editable list of excluded paths, are not
//! built), so there are two tabs. A change is written to the history at once; Sakura keeps the
//! check boxes in the dialog and applies them when it is closed or before a delete, which comes
//! to the same.
//!
//! The dialog is a window of its own, as in Sakura, with the window of the editor behind it.

use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, ClickEvent, Context, Entity, FocusHandle,
    Focusable, Global, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, PromptLevel,
    Render, ScrollHandle, StatefulInteractiveElement, Styled, Window, WindowBounds, WindowOptions,
    div, px, rgb, size,
};
use gpui_component::Root;
use matsu_config::history::{History, Recent};

use crate::file_ops::windows::editors;

/// The histories the dialog manages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tab {
    Files,
    Folders,
}

impl Tab {
    /// The caption of the tab (`STR_DLGFAV_FILE`, `STR_DLGFAV_FOLDER`).
    pub(crate) fn caption(self) -> &'static str {
        match self {
            Tab::Files => "ファイル",
            Tab::Folders => "フォルダー",
        }
    }
}

/// An item of a history as the list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Row {
    /// The access key and the "(非表示)" mark (`FormatFavoriteColumn`).
    pub key: String,
    pub favorite: bool,
    pub text: String,
}

/// The access key of the item `index` (0 to 9, A to Z, and again) and "(非表示)" when the menu does
/// not show it (`FormatFavoriteColumn`).
pub(crate) fn key_column(index: usize, shown: bool) -> String {
    let n = index % 36;
    let key = char::from_digit(n as u32, 36)
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('0');
    format!("{key} {}", if shown { "  " } else { "(非表示)" })
}

/// The rows of a tab.
pub(crate) fn rows(history: &History, tab: Tab) -> Vec<Row> {
    fn make<T: Recent>(list: &matsu_config::history::RecentList<T>) -> Vec<Row> {
        list.items()
            .iter()
            .enumerate()
            .map(|(i, item)| Row {
                key: key_column(i, i < list.view_count()),
                favorite: item.is_favorite(),
                text: item.text().to_owned(),
            })
            .collect()
    }
    match tab {
        Tab::Files => make(&history.files),
        Tab::Folders => make(&history.folders),
    }
}

fn paths_of(history: &History, tab: Tab) -> Vec<String> {
    rows(history, tab).into_iter().map(|r| r.text).collect()
}

/// Makes the item a favorite or not.
pub(crate) fn set_favorite(history: &mut History, tab: Tab, index: usize, favorite: bool) {
    match tab {
        Tab::Files => history.files.set_favorite(index, favorite),
        Tab::Folders => history.folders.set_favorite(index, favorite),
    };
    // A favorite comes into the part the menu shows (`UpdateView`).
    let (files, folders) = (history.files.view_count(), history.folders.view_count());
    history.files.change_view_count(files);
    history.folders.change_view_count(folders);
}

/// The index of the item with this path in the latest history: another window may have changed the
/// order since the dialog showed it, so rows are told by their path.
fn index_of(history: &History, tab: Tab, path: &str) -> Option<usize> {
    match tab {
        Tab::Files => history.files.find(path),
        Tab::Folders => history.folders.find(path),
    }
}

/// "すべて": removes every item, favorites too (`DeleteAllItem`).
pub(crate) fn delete_all(history: &mut History, tab: Tab) {
    match tab {
        Tab::Files => history.files.clear(),
        Tab::Folders => history.folders.clear(),
    }
}

/// "お気に入り以外" (`DeleteItemsNoFavorite`).
pub(crate) fn delete_non_favorites(history: &mut History, tab: Tab) {
    match tab {
        Tab::Files => history.files.delete_non_favorites(),
        Tab::Folders => history.folders.delete_non_favorites(),
    };
}

/// Whether Sakura would find the path (`CutLastYenFromDirectoryPath`, `IsFileExists(path, false)`:
/// a file or a folder).
pub(crate) fn path_exists(path: &str) -> bool {
    let trimmed = path
        .strip_suffix(['\\', '/'])
        .filter(|rest| !rest.is_empty() && !rest.ends_with(':'))
        .unwrap_or(path);
    Path::new(trimmed).exists()
}

/// "存在しない項目" (`IDC_BUTTON_DELETE_NOTFOUND`): removes the items whose path is gone.
pub(crate) fn delete_missing(history: &mut History, tab: Tab, exists: impl Fn(&str) -> bool) {
    match tab {
        Tab::Files => history.files.delete_where(|f| !exists(&f.path)),
        Tab::Folders => history.folders.delete_where(|f| !exists(&f.path)),
    };
}

/// "選択項目" (`DeleteSelected`): removes the items at `indices`, the last first so that the numbers
/// of the others stay right.
pub(crate) fn delete_selected(history: &mut History, tab: Tab, indices: &BTreeSet<usize>) {
    for &i in indices.iter().rev() {
        match tab {
            Tab::Files => history.files.delete(i),
            Tab::Folders => history.folders.delete(i),
        };
    }
}

/// The message before "すべて" (`STR_DLGFAV_CONF_DEL_FAV`).
fn confirm_all(tab: Tab) -> String {
    format!(
        "最近使った{}の履歴を削除します。\nよろしいですか？\n",
        tab.caption()
    )
}

/// The message before "お気に入り以外" (`STR_DLGFAV_CONF_DEL_NOTFAV`).
fn confirm_non_favorites(tab: Tab) -> String {
    format!(
        "最近使った{}の履歴のお気に入り以外を削除します。\nよろしいですか？",
        tab.caption()
    )
}

/// The message before "存在しない項目" (`STR_DLGFAV_CONF_DEL_PATH`).
fn confirm_missing(tab: Tab) -> String {
    format!(
        "最近使った{}の存在しないパスを削除します。\nよろしいですか？",
        tab.caption()
    )
}

/// The dialog that is open, so that a second `F_FAVORITE` brings it to the front.
#[derive(Default)]
struct OpenDialog(Option<AnyWindowHandle>);

impl Global for OpenDialog {}

/// How often the dialog looks at the history again (another window may change it) and whether the
/// editor windows are still there.
const REFRESH: Duration = Duration::from_millis(500);

const TEXT: u32 = 0x222222;
const SELECTED_BG: u32 = 0x2f6fd0;

pub(crate) struct FavoriteDialog {
    tab: Tab,
    history: History,
    selected: BTreeSet<usize>,
    /// The item the keys move from.
    focus_row: usize,
    /// The row a range selection (Shift) starts from.
    anchor: usize,
    focus: FocusHandle,
    scroll: ScrollHandle,
}

impl FavoriteDialog {
    pub(crate) fn new(history: History, window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(REFRESH).await;
                let alive = this
                    .update_in(cx, |this, window, cx| {
                        // The dialog belongs to the editor: it goes when the last window does.
                        if editors(cx).is_empty() {
                            window.remove_window();
                            return;
                        }
                        this.reload(cx);
                    })
                    .is_ok();
                if !alive {
                    break;
                }
            }
        })
        .detach();
        Self {
            tab: Tab::Files,
            history,
            selected: BTreeSet::new(),
            focus_row: 0,
            anchor: 0,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn tab(&self) -> Tab {
        self.tab
    }

    pub(crate) fn rows(&self) -> Vec<Row> {
        rows(&self.history, self.tab)
    }

    #[cfg(test)]
    pub(crate) fn selected(&self) -> &BTreeSet<usize> {
        &self.selected
    }

    /// Reads the history again (the file may have been changed by another window).
    pub(crate) fn reload(&mut self, cx: &mut Context<Self>) {
        if let Some(history) = super::current(cx)
            && history != self.history
        {
            let same_rows = self.paths() == paths_of(&history, self.tab);
            self.history = history;
            self.keep_selection(same_rows);
            cx.notify();
        }
    }

    /// Applies a change to the history, and shows the result.
    fn change(&mut self, cx: &mut Context<Self>, change: impl FnOnce(&mut History)) {
        let before = self.paths();
        if let Some(history) = super::update(cx, change) {
            self.history = history;
        }
        let same_rows = before == self.paths();
        self.keep_selection(same_rows);
        cx.notify();
    }

    fn paths(&self) -> Vec<String> {
        paths_of(&self.history, self.tab)
    }

    /// The selection is by row number, so it only stays when the rows are the same ones in the same
    /// order; else it would point at other files.
    fn keep_selection(&mut self, same_rows: bool) {
        if !same_rows {
            self.selected.clear();
        }
        let count = self.rows().len();
        self.selected.retain(|i| *i < count);
        self.focus_row = self.focus_row.min(count.saturating_sub(1));
    }

    pub(crate) fn select_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        if self.tab != tab {
            self.tab = tab;
            self.selected.clear();
            self.focus_row = 0;
            self.anchor = 0;
            cx.notify();
        }
    }

    /// Clicks on a row: alone it selects the row, with Control it adds or removes it, with Shift
    /// it selects from the row clicked before.
    pub(crate) fn click_row(
        &mut self,
        index: usize,
        control: bool,
        shift: bool,
        cx: &mut Context<Self>,
    ) {
        if shift {
            let (from, to) = (self.anchor.min(index), self.anchor.max(index));
            if !control {
                self.selected.clear();
            }
            self.selected.extend(from..=to);
        } else if control {
            if !self.selected.remove(&index) {
                self.selected.insert(index);
            }
            self.anchor = index;
        } else {
            self.selected = BTreeSet::from([index]);
            self.anchor = index;
        }
        self.focus_row = index;
        cx.notify();
    }

    /// The check box of a row.
    pub(crate) fn toggle_favorite(&mut self, index: usize, cx: &mut Context<Self>) {
        let tab = self.tab;
        let Some(row) = self.rows().into_iter().nth(index) else {
            return;
        };
        self.change(cx, |h| {
            if let Some(i) = index_of(h, tab, &row.text) {
                set_favorite(h, tab, i, !row.favorite);
            }
        });
    }

    /// Asks, and runs `change` on the history if the answer is yes.
    fn confirm_then(
        &mut self,
        message: String,
        window: &mut Window,
        cx: &mut Context<Self>,
        change: impl FnOnce(&mut History) + 'static,
    ) {
        let answer = window.prompt(
            PromptLevel::Warning,
            &message,
            None,
            &["はい(Y)", "いいえ(N)"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(0) = answer.await {
                this.update(cx, |this, cx| this.change(cx, change)).ok();
            }
        })
        .detach();
    }

    /// "すべて".
    pub(crate) fn ask_delete_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab = self.tab;
        if !self.rows().is_empty() {
            self.confirm_then(confirm_all(tab), window, cx, move |h| delete_all(h, tab));
        }
    }

    /// "お気に入り以外".
    pub(crate) fn ask_delete_non_favorites(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab = self.tab;
        if !self.rows().is_empty() {
            self.confirm_then(confirm_non_favorites(tab), window, cx, move |h| {
                delete_non_favorites(h, tab);
            });
        }
    }

    /// "存在しない項目".
    pub(crate) fn ask_delete_missing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tab = self.tab;
        if !self.rows().is_empty() {
            self.confirm_then(confirm_missing(tab), window, cx, move |h| {
                delete_missing(h, tab, path_exists);
            });
        }
    }

    /// "選択項目": no question (Sakura asks none).
    pub(crate) fn delete_selected_rows(&mut self, cx: &mut Context<Self>) {
        let (tab, selected) = (self.tab, self.selected.clone());
        if selected.is_empty() {
            return;
        }
        let paths: Vec<String> = self
            .rows()
            .into_iter()
            .enumerate()
            .filter(|(i, _)| selected.contains(i))
            .map(|(_, r)| r.text)
            .collect();
        let next = selected.iter().next_back().copied().unwrap_or(0) + 1 - selected.len();
        self.selected.clear();
        self.change(cx, |h| {
            let indices = paths.iter().filter_map(|p| index_of(h, tab, p)).collect();
            delete_selected(h, tab, &indices);
        });
        // The item after the last one deleted is selected, or the last item.
        let count = self.rows().len();
        if count > 0 {
            let at = next.min(count - 1);
            self.selected = BTreeSet::from([at]);
            self.focus_row = at;
            self.anchor = at;
        }
        cx.notify();
    }

    fn close(&mut self, window: &mut Window) {
        window.remove_window();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let count = self.rows().len();
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        let target = match key {
            "escape" | "enter" => {
                self.close(window);
                return;
            }
            "delete" => {
                self.delete_selected_rows(cx);
                cx.stop_propagation();
                return;
            }
            "space" => {
                if count > 0 {
                    self.toggle_favorite(self.focus_row, cx);
                }
                cx.stop_propagation();
                return;
            }
            "a" if modifiers.control || modifiers.platform => {
                self.selected = (0..count).collect();
                cx.notify();
                cx.stop_propagation();
                return;
            }
            "tab" => {
                let tab = if self.tab == Tab::Files {
                    Tab::Folders
                } else {
                    Tab::Files
                };
                self.select_tab(tab, cx);
                cx.stop_propagation();
                return;
            }
            "up" => self.focus_row.saturating_sub(1),
            "down" => (self.focus_row + 1).min(count.saturating_sub(1)),
            "home" => 0,
            "end" => count.saturating_sub(1),
            _ => return,
        };
        if count > 0 {
            self.click_row(target, false, modifiers.shift, cx);
            self.scroll.scroll_to_item(target);
        }
        cx.stop_propagation();
    }
}

impl Focusable for FavoriteDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for FavoriteDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.focus.is_focused(window) {
            window.focus(&self.focus, cx);
        }
        let ui_font = gpui_component::Theme::global(cx).font_family.clone();
        let rows = self.rows();
        let items: Vec<_> = rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let selected = self.selected.contains(&i);
                let mut item = crate::a11y::list_row(
                    div().id(("row", i)),
                    &format!("{} {}", row.key, row.text),
                    selected,
                )
                .px_2()
                .h(px(24.))
                .flex()
                .items_center()
                .gap_2()
                .cursor_pointer();
                if selected {
                    item = item.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff));
                }
                item.child(
                    div()
                        .id(("favorite", i))
                        .role(gpui::Role::CheckBox)
                        .aria_label(format!("お気に入り: {}", row.text))
                        .aria_toggled(if row.favorite {
                            gpui::Toggled::True
                        } else {
                            gpui::Toggled::False
                        })
                        .w(px(16.))
                        .h(px(16.))
                        .border_1()
                        .border_color(rgb(0x666666))
                        .bg(rgb(0xffffff))
                        .text_color(rgb(TEXT))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(if row.favorite { "✓" } else { "" })
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.toggle_favorite(i, cx);
                            cx.stop_propagation();
                        })),
                )
                .child(div().w(px(80.)).child(row.key.clone()))
                .child(div().flex_1().child(row.text.clone()))
                .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                    let modifiers = event.modifiers();
                    this.click_row(
                        i,
                        modifiers.control || modifiers.platform,
                        modifiers.shift,
                        cx,
                    );
                }))
            })
            .collect();

        let button = |id: &'static str, text: &'static str| {
            crate::a11y::button(div().id(id), text)
                .px_3()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(0xaaaaaa))
                .bg(rgb(0xf4f4f4))
                .cursor_pointer()
                .child(text)
        };
        let tab_button = |tab: Tab, active: bool| {
            let mut tab_div = div()
                .id(tab.caption())
                .role(gpui::Role::Tab)
                .aria_label(tab.caption())
                .aria_selected(active)
                .px_4()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(0xaaaaaa))
                .cursor_pointer()
                .child(tab.caption());
            tab_div = if active {
                tab_div.bg(rgb(0xffffff))
            } else {
                tab_div.bg(rgb(0xe4e4e4))
            };
            tab_div.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.select_tab(tab, cx);
            }))
        };

        crate::a11y::dialog(div().id("favorite-dialog"), "履歴とお気に入りの管理")
            .key_context("FavoriteDialog")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .size_full()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0xf0f0f0))
            .text_color(rgb(TEXT))
            .text_size(px(14.))
            .font_family(ui_font)
            .child(
                div()
                    .id("favorite-tabs")
                    .role(gpui::Role::TabList)
                    .aria_label("対象")
                    .flex()
                    .gap_1()
                    .child(tab_button(Tab::Files, self.tab == Tab::Files))
                    .child(tab_button(Tab::Folders, self.tab == Tab::Folders)),
            )
            .child(
                div()
                    .id("favorite-rows")
                    .role(gpui::Role::ListBox)
                    .aria_label("一覧")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(rgb(0xaaaaaa))
                    .children(items),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child("削除:")
                    .child(button("delete-all", "すべて(A)").on_click(cx.listener(
                        |this, _: &ClickEvent, window, cx| this.ask_delete_all(window, cx),
                    )))
                    .child(
                        button("delete-non-favorites", "お気に入り以外(N)").on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| {
                                this.ask_delete_non_favorites(window, cx);
                            },
                        )),
                    )
                    .child(
                        button("delete-missing", "存在しない項目(E)").on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| this.ask_delete_missing(window, cx),
                        )),
                    )
                    .child(button("delete-selected", "選択項目(S)").on_click(
                        cx.listener(|this, _: &ClickEvent, _, cx| this.delete_selected_rows(cx)),
                    ))
                    .child(div().flex_1())
                    .child(button("close", "閉じる").on_click(
                        cx.listener(|this, _: &ClickEvent, window, _| this.close(window)),
                    )),
            )
    }
}

/// Whether the dialog is open.
pub(crate) fn is_open(cx: &App) -> bool {
    cx.try_global::<OpenDialog>()
        .and_then(|d| d.0)
        .is_some_and(|handle| cx.windows().contains(&handle))
}

/// Shows the dialog; one that is open comes to the front instead of a second one.
pub(crate) fn open(cx: &mut App) {
    if let Some(handle) = cx.try_global::<OpenDialog>().and_then(|d| d.0)
        && cx.windows().contains(&handle)
    {
        handle
            .update(cx, |_, window, _| window.activate_window())
            .ok();
        return;
    }
    let history =
        super::current(cx).unwrap_or_else(|| History::new(15, 15, super::CASE_INSENSITIVE));
    let bounds = Bounds::centered(None, size(px(760.), px(460.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        icon: crate::app_icon::window_icon(),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("履歴とお気に入りの管理".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let opened = cx.open_window(options, move |window, cx| {
        let dialog: Entity<FavoriteDialog> = cx.new(|cx| FavoriteDialog::new(history, window, cx));
        cx.new(|cx| Root::new(dialog, window, cx))
    });
    if let Ok(handle) = opened {
        cx.set_global(OpenDialog(Some(handle.into())));
    }
}
