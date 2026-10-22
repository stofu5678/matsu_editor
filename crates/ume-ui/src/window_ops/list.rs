//! The window list: the popup of `F_WINLIST` (ウィンドウ一覧) and the dialog of `F_DLGWINLIST`
//! (ウィンドウ一覧表示).
//!
//! Both are drawn over the editor window, like the options of the Save As: the OS has no popup
//! menu that GPUI could show at a place of our choosing, and the dialog of Sakura (a list with
//! check boxes and three buttons) is a window of the tray process, which this program does not
//! have. The list shows the editor windows in the order they were opened.
//!
//! * The popup (`CEditWnd::PopupWinList`, `WinListMenu`): one item for each window, with an
//!   access key 1 to 9, A to Z (and again from 1 for the 36th), a check mark on the current one.
//!   Choosing one brings that window to the front. Keys: the access key, Up and Down, Enter,
//!   Esc.
//! * The dialog (`CDlgWindowList`): the same labels with a check box each, and the buttons
//!   上書き保存 (`F_FILESAVE` in the checked windows), ウィンドウを閉じる (close them, with the
//!   questions of the close) and OK (which only closes the dialog). The list follows the windows
//!   that come and go. Sakura's dialog does not bring a window to the front; here Enter and a
//!   double click do (the way to choose one that the task asks for).

use crate::dialog_focus::KeepTabInside as _;
use std::collections::HashSet;
use std::time::Duration;

use gpui::{
    AnyWindowHandle, App, AppContext as _, ClickEvent, Context, Entity, EntityId, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, MouseDownEvent, ParentElement,
    Render, ScrollHandle, StatefulInteractiveElement, Styled, WeakEntity, Window, div, px, rgb,
    rgba,
};
use ume_commands::sakura::FUNCTIONS;
use ume_core::encoding::Encoding;

use super::run_in_window;
use crate::file_ops::windows::{editors, workspace_of};
use crate::view::EditorView;

/// The access key of the window at `index` (0 origin): 1 to 9, A to Z, and again
/// (`CFileNameManager::GetAccessKeyByIndex`: the 36th is 0 in Sakura's arithmetic, so the key
/// after Z is 0, then 1 again).
pub(crate) fn access_key(index: usize) -> char {
    let key = (index + 1) % 36;
    // `key` is below 36.
    char::from_digit(key as u32, 36)
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('0')
}

/// The text of a window in the list (`CFileNameManager::GetMenuFullLabel`): the access key, the
/// file name or `(無題)`, ` *` for unsaved changes and the encoding as `  [UTF-8]` unless it is
/// Shift_JIS (`IsValidCodeTypeExceptSJIS`). `name` is the path as 「ファイル名表示」 shows it (`FileNames::name_of`), which is what the window
/// list and the DIFF dialogs use.
pub(crate) fn label(index: usize, name: &str, modified: bool, encoding: Encoding) -> String {
    let bracket = if encoding == Encoding::ShiftJis {
        String::new()
    } else {
        encoding.bracket_name()
    };
    format!(
        "{} {}{}{}",
        access_key(index),
        name,
        if modified { " *" } else { "" },
        bracket
    )
}

/// The index of the window that the access key `key` (any case) chooses.
pub(crate) fn index_of_key(key: &str, count: usize) -> Option<usize> {
    let mut chars = key.chars();
    let c = chars.next()?.to_ascii_uppercase();
    if chars.next().is_some() {
        return None;
    }
    (0..count).find(|i| access_key(*i) == c)
}

/// A window in the list.
#[derive(Clone)]
struct Row {
    handle: AnyWindowHandle,
    view: Entity<EditorView>,
    label: String,
    current: bool,
}

fn rows(cx: &mut App, owner: AnyWindowHandle) -> Vec<Row> {
    // The document the owner shows (its active tab) has the check mark.
    let shown = workspace_of(cx, owner).map(|ws| ws.read(cx).editor().clone());
    let names = crate::file_names::current(cx);
    editors(cx)
        .into_iter()
        .enumerate()
        .map(|(i, (handle, view))| {
            let shared = view.read(cx).document().read(cx);
            let label = label(
                i,
                &names.name_of(shared.path(), shared.untitled_number(), true),
                shared.model().is_modified(),
                shared.file().encoding,
            );
            let current = handle == owner && shown.as_ref() == Some(&view);
            Row {
                handle,
                view,
                label,
                current,
            }
        })
        .collect()
}

fn code_of(name: &str) -> u16 {
    FUNCTIONS
        .iter()
        .find(|f| f.name == name)
        .map_or(0, |f| f.code)
}

/// How often the dialog looks at the windows again (they close and change while it is open).
const REFRESH: Duration = Duration::from_millis(300);

pub(crate) struct WindowList {
    /// The dialog of `F_DLGWINLIST`; else the popup of `F_WINLIST`.
    dialog: bool,
    /// The window that shows the list.
    owner: AnyWindowHandle,
    view: WeakEntity<EditorView>,
    selected: usize,
    checked: HashSet<EntityId>,
    focus: FocusHandle,
    scroll: ScrollHandle,
}

impl WindowList {
    fn new(
        dialog: bool,
        owner: AnyWindowHandle,
        view: WeakEntity<EditorView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Not `rows`: the view of the owner is being updated while the list is made, and its
        // text cannot be read.
        let all = editors(cx);
        let selected = all
            .iter()
            .position(|(_, v)| v.entity_id() == view.entity_id())
            .or_else(|| all.iter().position(|(handle, _)| *handle == owner))
            .unwrap_or(0);
        if dialog {
            // The list follows the windows that come and go while it is open.
            cx.spawn_in(window, async move |this, cx| {
                loop {
                    cx.background_executor().timer(REFRESH).await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        Self {
            dialog,
            owner,
            view,
            selected,
            checked: HashSet::new(),
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
        }
    }

    /// Closes the list and gives the focus back to the editor.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.window_list = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    /// Brings the window to the front, and closes the list.
    fn choose(
        &mut self,
        handle: AnyWindowHandle,
        view: Entity<EditorView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close(window, cx);
        if handle == self.owner {
            window.activate_window();
        }
        // The window and, in tab mode, the tab.
        crate::tabs::flows::reveal(cx, handle, view);
    }

    /// The row the list is on.
    #[cfg(test)]
    pub(crate) fn selected(&self) -> usize {
        self.selected
    }

    fn move_selection(&mut self, delta: isize, count: usize) {
        if count > 0 {
            self.selected = self.selected.saturating_add_signed(delta).min(count - 1);
        }
    }

    fn toggle(&mut self, id: EntityId) {
        if !self.checked.remove(&id) {
            self.checked.insert(id);
        }
    }

    /// 上書き保存 and ウィンドウを閉じる: runs `function` in every checked window. The windows
    /// are reached from a task, not from inside an update of this one.
    pub(super) fn run_on_checked(
        &mut self,
        function: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let checked: Vec<_> = rows(cx, self.owner)
            .into_iter()
            .filter(|r| self.checked.contains(&r.view.entity_id()))
            .collect();
        let code = code_of(function);
        cx.spawn_in(window, async move |this, cx| {
            for row in checked {
                run_in_window(cx, row.handle, &row.view, code);
            }
            this.update(cx, |this, cx| {
                // The windows that are gone no longer count.
                let open: HashSet<_> = rows(cx, this.owner)
                    .iter()
                    .map(|r| r.view.entity_id())
                    .collect();
                this.checked.retain(|h| open.contains(h));
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let rows = rows(cx, self.owner);
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        match key {
            "escape" => self.close(window, cx),
            "up" => self.move_selection(-1, rows.len()),
            "down" => self.move_selection(1, rows.len()),
            "home" => self.selected = 0,
            "end" => self.selected = rows.len().saturating_sub(1),
            "enter" => {
                if let Some(row) = rows.get(self.selected) {
                    self.choose(row.handle, row.view.clone(), window, cx);
                }
            }
            "space" if self.dialog => {
                if let Some(row) = rows.get(self.selected) {
                    self.toggle(row.view.entity_id());
                }
            }
            _ if !self.dialog && !modifiers.control && !modifiers.alt && !modifiers.platform => {
                // The access key of an item.
                match index_of_key(key, rows.len()) {
                    Some(i) => self.choose(rows[i].handle, rows[i].view.clone(), window, cx),
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

impl Focusable for WindowList {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

const TEXT: u32 = 0x222222;
const SELECTED_BG: u32 = 0x2f6fd0;

impl Render for WindowList {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = rows(cx, self.owner);
        self.selected = self.selected.min(rows.len().saturating_sub(1));
        let items: Vec<_> = rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let handle = row.handle;
                let view = row.view.clone();
                let id = row.view.entity_id();
                let mut item =
                    crate::a11y::list_row(div().id(("window", i)), &row.label, i == self.selected)
                        .px_2()
                        .h(px(24.))
                        .flex()
                        .items_center()
                        .gap_2()
                        .cursor_pointer();
                if i == self.selected {
                    item = item.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff));
                }
                if self.dialog {
                    let checked = self.checked.contains(&id);
                    item = item.child(
                        div()
                            .id(("checkbox", i))
                            .role(gpui::Role::CheckBox)
                            .aria_label(format!("{}を選択", row.label))
                            .aria_toggled(if checked {
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
                            .child(if checked { "✓" } else { "" })
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.toggle(id);
                                this.selected = i;
                                cx.stop_propagation();
                                cx.notify();
                            })),
                    );
                } else {
                    // The check mark of the current window in the menu.
                    item = item.child(div().w(px(14.)).child(if row.current { "✓" } else { "" }));
                }
                item.child(row.label.clone()).on_click(cx.listener(
                    move |this, event: &ClickEvent, window, cx| {
                        this.selected = i;
                        if !this.dialog || event.click_count() >= 2 {
                            this.choose(handle, view.clone(), window, cx);
                        }
                        cx.notify();
                    },
                ))
            })
            .collect();

        // Room for the rows: the height of the window less the title, the buttons and a margin
        // (the popup has none of them). Longer lists scroll.
        let spare = if self.dialog { 150. } else { 24. };
        let max_rows = (window.viewport_size().height - px(spare)).max(px(48.));
        let rows_box = div()
            .id("window-list-rows")
            .role(gpui::Role::ListBox)
            .aria_label("ウィンドウ")
            .max_h(max_rows)
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .children(items);

        let button = |id: &'static str, text: &'static str| {
            crate::a11y::button(div().id(id), text)
                .px_4()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(0xaaaaaa))
                .bg(rgb(0xf4f4f4))
                .cursor_pointer()
                .child(text)
        };
        let mut panel = crate::a11y::dialog(
            div().id("window-list"),
            if self.dialog {
                "ウィンドウ一覧表示"
            } else {
                "ウィンドウ一覧"
            },
        )
        .key_context("WindowList")
        .track_focus(&self.focus)
        .keep_tab_inside(&self.focus)
        .on_key_down(cx.listener(Self::on_key_down))
        .min_w(px(300.))
        .max_w(px(640.))
        .p_1()
        .flex()
        .flex_col()
        .bg(rgb(0xf0f0f0))
        .border_1()
        .border_color(rgb(0x888888))
        .text_color(rgb(TEXT))
        .text_size(px(14.));
        if self.dialog {
            panel =
                panel
                    .p_4()
                    .child(div().mb_2().child("ウィンドウ一覧表示"))
                    .child(
                        div()
                            .bg(rgb(0xffffff))
                            .border_1()
                            .border_color(rgb(0xaaaaaa))
                            .min_h(px(120.))
                            .child(rows_box),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(button("window-list-save", "上書き保存(S)").on_click(
                                cx.listener(|this, _: &ClickEvent, window, cx| {
                                    this.run_on_checked("F_FILESAVE", window, cx);
                                }),
                            ))
                            .child(
                                button("window-list-close", "ウィンドウを閉じる(C)").on_click(
                                    cx.listener(|this, _: &ClickEvent, window, cx| {
                                        this.run_on_checked("F_WINCLOSE", window, cx);
                                    }),
                                ),
                            )
                            .child(button("window-list-ok", "OK").on_click(cx.listener(
                                |this, _: &ClickEvent, window, cx| this.close(window, cx),
                            ))),
                    );
        } else {
            panel = panel
                .child(rows_box)
                // A click outside the popup closes it, as outside a menu.
                .on_mouse_down_out(cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    this.close(window, cx);
                }));
        }
        let overlay = div()
            .id("window-list-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .flex();
        if self.dialog {
            overlay
                .bg(rgba(0x00000055))
                .items_center()
                .justify_center()
                .child(panel)
        } else {
            // The popup is at the top right of the view (`PopupWinList`, without the mouse).
            overlay.items_start().justify_end().child(panel)
        }
    }
}

/// Shows the list over `view`. A list that is open is closed instead of opening a second one.
pub(crate) fn open(
    view: &mut EditorView,
    dialog: bool,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) {
    if view.window_list.take().is_some() {
        window.focus(&view.focus_handle, cx);
        cx.notify();
        return;
    }
    let owner = window.window_handle();
    let weak = cx.weak_entity();
    let list = cx.new(|cx| WindowList::new(dialog, owner, weak, window, cx));
    let focus = list.read(cx).focus.clone();
    window.focus(&focus, cx);
    view.window_list = Some(list);
    cx.notify();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_access_keys_are_1_to_9_then_a_to_z() {
        let keys: String = (0..36).map(access_key).collect();
        assert_eq!(keys, "123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0");
        // The 37th starts again.
        assert_eq!(access_key(36), '1');
    }

    #[test]
    fn a_key_chooses_the_first_window_with_that_access_key() {
        assert_eq!(index_of_key("1", 3), Some(0));
        assert_eq!(index_of_key("3", 3), Some(2));
        assert_eq!(index_of_key("4", 3), None);
        assert_eq!(index_of_key("a", 12), Some(9));
        assert_eq!(index_of_key("A", 12), Some(9));
        assert_eq!(index_of_key("escape", 12), None);
        assert_eq!(index_of_key("", 12), None);
    }

    #[test]
    fn a_label_has_the_key_the_name_the_mark_and_the_encoding() {
        let name = "/home/a/memo.txt";
        assert_eq!(
            label(0, name, false, Encoding::ShiftJis),
            "1 /home/a/memo.txt"
        );
        assert_eq!(
            label(1, name, true, Encoding::Utf8),
            "2 /home/a/memo.txt *  [UTF-8]"
        );
        assert_eq!(label(9, "(無題)", false, Encoding::ShiftJis), "A (無題)");
        assert_eq!(
            label(2, "(無題)", true, Encoding::Utf16Le),
            "3 (無題) *  [UTF-16]"
        );
    }

    #[test]
    fn a_label_shows_the_path_as_the_file_name_display_makes_it() {
        use crate::file_names::{FileNames, NoFolders};

        let rows = [vec!["/home/a/".to_owned(), "家:".to_owned()]];
        let names = FileNames::new(true, 100, &rows, &NoFolders, false);
        let path = std::path::Path::new("/home/a/memo.txt");
        assert_eq!(
            label(
                0,
                &names.name_of(Some(path), None, true),
                true,
                Encoding::Utf8
            ),
            "1 家:memo.txt *  [UTF-8]"
        );
        // A document with no file is 無題, whatever the list says.
        assert_eq!(
            label(
                1,
                &names.name_of(None, None, true),
                false,
                Encoding::ShiftJis
            ),
            "2 (無題)"
        );
        // A long path is shortened to the width of the setting.
        let narrow = FileNames::new(true, 15, &[], &NoFolders, false);
        let long = std::path::Path::new("C:\\users\\a\\work\\memo.txt");
        assert_eq!(
            label(
                2,
                &narrow.name_of(Some(long), None, true),
                false,
                Encoding::ShiftJis
            ),
            "3 C:\\...\\memo.txt"
        );
    }
}
