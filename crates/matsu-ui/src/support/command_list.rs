//! The command list (コマンド一覧, `F_MENU_ALLFUNC`): a menu of every function, by group.
//!
//! Sakura: `CViewCommander::Command_MENU_ALLFUNC` (`sakura_core/cmd/CViewCommander_Support.cpp`)
//! builds a popup menu with one submenu per group (`CFuncLookup::Category2Name`) and the functions
//! of the group in the order of `sakura_core/func/Funccode.cpp` (`CFuncLookup::Pos2FuncCode`,
//! [`FUNCTION_LISTS`]); it opens at the top of the edit window, 540 pixels from the left at most;
//! the chosen function is posted as a command (`WM_COMMAND`) and runs as if from a menu.
//!
//! Here the list is a modal pair of columns over the view: the groups on the left, the functions
//! of the chosen group on the right. Up and Down choose, Right or Enter goes into a group, Left goes
//! back, Esc closes, Enter on a function runs it; the mouse works too. A function that has no
//! command in this program yet is shown, but grayed out and not chosen (as the main menu does).
//!
//! Not done: the three groups that Sakura adds at run time (registered macros, custom menus,
//! plugin commands), which have no data here yet; the mnemonic letters of the items.

use crate::a11y::AriaDisabled as _;
use crate::dialog_focus::KeepTabInside as _;
use gpui::{
    AppContext as _, ClickEvent, Context, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Render, ScrollHandle,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, px, rgb,
};
use matsu_commands::keymap::{DisplayStyle, KeyMap};
use matsu_commands::sakura::{FUNCTION_GROUPS, FUNCTION_LISTS, FUNCTIONS};

use crate::menu::{Available, Registered, menu_label};
use crate::view::EditorView;

/// Sakura opens the list this far from the left edge of the edit window (`po.x = 540`).
const LEFT: f32 = 540.0;

/// The width of the two columns, to keep them inside the view.
const LIST_WIDTH: f32 = 400.0;

/// One function of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Row {
    pub code: u16,
    /// The label (`CFuncLookup::Funccode2Name`).
    pub label: String,
    /// The first key of the function, if it has one.
    pub shortcut: Option<String>,
    /// Whether the program has a command for it.
    pub enabled: bool,
}

/// One group, with its functions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group {
    pub name: &'static str,
    pub rows: Vec<Row>,
}

/// The label of a function: its name in Sakura's string table, else the function's name.
fn label_of(code: u16) -> String {
    FUNCTIONS.iter().find(|f| f.code == code).map_or_else(
        || format!("F_{code}"),
        |f| menu_label(f.label.unwrap_or(f.name), None),
    )
}

/// All groups with their functions, in Sakura's order. A function that does not exist
/// (`F_DISABLE`) is left out, as `Command_MENU_ALLFUNC` does (`code != 0`).
pub(crate) fn groups(
    keymap: &KeyMap,
    available: &dyn Available,
    style: DisplayStyle,
) -> Vec<Group> {
    FUNCTION_GROUPS
        .iter()
        .zip(FUNCTION_LISTS.iter())
        .map(|(&name, codes)| Group {
            name,
            rows: codes
                .iter()
                .copied()
                .filter(|&code| code != 0)
                .map(|code| Row {
                    code,
                    label: label_of(code),
                    shortcut: keymap.shortcut_for(code).map(|chord| chord.display(style)),
                    enabled: available.is_available(code),
                })
                .collect(),
        })
        .collect()
}

/// Which row is chosen, without the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Cursor {
    pub group: usize,
    pub row: usize,
    /// Whether the right column has the choice.
    pub in_rows: bool,
}

/// What a key does to the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Nothing,
    Close,
    /// Run the function with this code.
    Run(u16),
}

impl Cursor {
    /// Applies a key (GPUI's key name). `groups` is the list as it is shown.
    pub(crate) fn key(&mut self, key: &str, groups: &[Group]) -> Step {
        let rows = groups.get(self.group).map_or(0, |g| g.rows.len());
        match (key, self.in_rows) {
            ("up", false) => self.group = self.group.saturating_sub(1),
            ("down", false) => self.group = (self.group + 1).min(groups.len().saturating_sub(1)),
            ("home", false) => self.group = 0,
            ("end", false) => self.group = groups.len().saturating_sub(1),
            ("right" | "enter", false) => {
                if rows > 0 {
                    self.in_rows = true;
                    self.row = 0;
                }
            }
            ("escape", false) => return Step::Close,
            ("up", true) => self.row = self.row.saturating_sub(1),
            ("down", true) => self.row = (self.row + 1).min(rows.saturating_sub(1)),
            ("pageup", true) => self.row = self.row.saturating_sub(10),
            ("pagedown", true) => self.row = (self.row + 10).min(rows.saturating_sub(1)),
            ("home", true) => self.row = 0,
            ("end", true) => self.row = rows.saturating_sub(1),
            ("left" | "escape", true) => self.in_rows = false,
            ("enter", true) => {
                if let Some(row) = groups.get(self.group).and_then(|g| g.rows.get(self.row))
                    && row.enabled
                {
                    return Step::Run(row.code);
                }
            }
            _ => return Step::Nothing,
        }
        if !self.in_rows {
            self.row = 0;
        }
        Step::Nothing
    }
}

impl EditorView {
    /// `F_MENU_ALLFUNC`: opens the command list.
    pub(crate) fn open_command_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.support.command_list.is_some() {
            return;
        }
        // The shortcuts are written as the menus write them (`app.rs`).
        let style = if cfg!(target_os = "macos") {
            DisplayStyle::Mac
        } else {
            DisplayStyle::Sakura
        };
        let groups = groups(&self.keymap, &Registered, style);
        let size = (self.metrics.view_width, self.metrics.view_height);
        let view = cx.entity().downgrade();
        let list = cx.new(|cx| CommandList::new(view, groups, size, cx));
        let focus = list.read(cx).focus.clone();
        window.focus(&focus, cx);
        self.support.command_list = Some(list);
        cx.notify();
    }
}

/// The modal list over the view.
pub(crate) struct CommandList {
    view: WeakEntity<EditorView>,
    groups: Vec<Group>,
    cursor: Cursor,
    focus: FocusHandle,
    scroll: ScrollHandle,
    /// The width of the view, to keep the list inside it.
    view_width: f32,
    /// The height of the view, for the right column.
    view_height: f32,
}

impl CommandList {
    pub(crate) fn new(
        view: WeakEntity<EditorView>,
        groups: Vec<Group>,
        view_size: (f32, f32),
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            view,
            groups,
            cursor: Cursor::default(),
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            view_width: view_size.0,
            view_height: view_size.1,
        }
    }

    /// Closes the list and gives the focus back to the view; runs `code` if there is one
    /// (Sakura posts the command to the main window after the menu is gone).
    fn finish(&mut self, code: Option<u16>, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.support.command_list = None;
            window.focus(&view.focus_handle, cx);
            if let Some(code) = code {
                view.run_function(code, &[], window, cx);
            }
            cx.notify();
        })
        .ok();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let step = self.cursor.key(&event.keystroke.key, &self.groups);
        match step {
            Step::Close => self.finish(None, window, cx),
            Step::Run(code) => self.finish(Some(code), window, cx),
            Step::Nothing => {}
        }
        if self.cursor.in_rows {
            self.scroll.scroll_to_item(self.cursor.row);
        }
        // the list is modal: no key reaches the view
        cx.stop_propagation();
        cx.notify();
    }
}

impl Focusable for CommandList {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

const TEXT: u32 = 0x222222;
const DISABLED: u32 = 0x999999;
const CHOSEN: u32 = 0x2f6fd0;
const BACKGROUND: u32 = 0xf7f7f7;

impl Render for CommandList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cursor = self.cursor;
        let group_rows = self.groups.iter().enumerate().map(|(i, group)| {
            let chosen = i == cursor.group;
            let row = crate::a11y::list_row(div().id(("group", i)), group.name, chosen)
                .px_2()
                .h(px(24.))
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(group.name)
                .child(if group.rows.is_empty() { "" } else { "▶" })
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.cursor.group = i;
                    this.cursor.row = 0;
                    this.cursor.in_rows = !this.groups[i].rows.is_empty();
                    cx.notify();
                }));
            if chosen {
                row.bg(rgb(CHOSEN)).text_color(rgb(0xffffff))
            } else {
                row
            }
        });
        let function_rows = self
            .groups
            .get(cursor.group)
            .map(|g| g.rows.as_slice())
            .unwrap_or_default()
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let chosen = cursor.in_rows && i == cursor.row;
                let code = row.code;
                let enabled = row.enabled;
                let item = crate::a11y::list_row(
                    div().id(("function", i)),
                    &crate::a11y::function_row_name(&row.label, row.shortcut.as_deref()),
                    chosen,
                )
                .aria_disabled(!enabled)
                .px_2()
                .h(px(24.))
                .flex()
                .items_center()
                .justify_between()
                .gap_4()
                .whitespace_nowrap()
                .child(row.label.clone())
                .child(row.shortcut.clone().unwrap_or_default())
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    if enabled {
                        this.finish(Some(code), window, cx);
                    }
                }));
                if !enabled {
                    item.text_color(rgb(DISABLED))
                } else if chosen {
                    item.bg(rgb(CHOSEN)).text_color(rgb(0xffffff))
                } else {
                    item
                }
            });
        // 540 pixels from the left as in Sakura, but not so far right that the columns leave the view
        let left = LEFT.min((self.view_width - LIST_WIDTH).max(0.0));
        let column = || {
            div()
                .py_1()
                .bg(rgb(BACKGROUND))
                .border_1()
                .border_color(rgb(0x888888))
        };
        let functions = column()
            .id("function-list")
            .role(gpui::Role::ListBox)
            .aria_label("機能")
            .min_w(px(220.))
            .max_h(px(self.view_height))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .children(function_rows);
        div()
            .id("command-list-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| this.finish(None, window, cx)),
            )
            .child(
                crate::a11y::dialog(div().id("command-list"), "コマンド一覧")
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .track_focus(&self.focus)
                    .keep_tab_inside(&self.focus)
                    .key_context("CommandList")
                    .on_key_down(cx.listener(Self::on_key_down))
                    .absolute()
                    .top_0()
                    .left(px(left))
                    .flex()
                    .items_start()
                    .text_color(rgb(TEXT))
                    .text_size(px(14.))
                    .child(
                        column()
                            .id("group-list")
                            .role(gpui::Role::ListBox)
                            .aria_label("グループ")
                            .w(px(150.))
                            .children(group_rows),
                    )
                    .child(functions),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<Group> {
        groups(&KeyMap::default(), &Registered, DisplayStyle::Sakura)
    }

    fn group<'a>(groups: &'a [Group], name: &str) -> &'a Group {
        groups.iter().find(|g| g.name == name).unwrap()
    }

    #[test]
    fn the_groups_are_sakuras_in_its_order() {
        let groups = all();
        let names: Vec<_> = groups.iter().map(|g| g.name).collect();
        assert_eq!(names, FUNCTION_GROUPS);
        // the Others array of Funccode.cpp holds only F_DISABLE, which the list leaves out
        assert!(group(&groups, "その他").rows.is_empty());
    }

    #[test]
    fn the_functions_of_a_group_are_in_the_order_of_funccode_cpp() {
        let groups = all();
        let labels: Vec<_> = group(&groups, "ファイル操作系")
            .rows
            .iter()
            .take(3)
            .map(|r| r.label.as_str())
            .collect();
        assert_eq!(labels, ["新規作成", "新規ウインドウを開く", "開く..."]);
        let support: Vec<_> = group(&groups, "支援").rows.iter().map(|r| r.code).collect();
        assert_eq!(support.first(), Some(&31430));
        assert!(support.contains(&31445) && support.contains(&31455));
    }

    #[test]
    fn no_row_has_code_zero_and_every_row_has_a_label() {
        for g in all() {
            for r in &g.rows {
                assert_ne!(r.code, 0);
                assert!(!r.label.is_empty() && !r.label.contains("&&"), "{}", r.code);
            }
        }
    }

    #[test]
    fn a_function_with_a_command_is_enabled_and_shows_its_key() {
        let groups = groups(&KeyMap::default(), &Registered, DisplayStyle::Sakura);
        let support = group(&groups, "支援");
        let hokan = support.rows.iter().find(|r| r.code == 31430).unwrap();
        assert!(hokan.enabled);
        assert_eq!(hokan.shortcut.as_deref(), Some("Ctrl+Space"));
        let files = group(&groups, "ファイル操作系");
        assert!(files.rows.iter().all(|r| r.code != 0));
        // the function table is the same as the list's: every row is a known function
        for g in &groups {
            for r in &g.rows {
                assert!(FUNCTIONS.iter().any(|f| f.code == r.code), "{}", r.code);
            }
        }
    }

    #[test]
    fn a_function_without_a_command_is_disabled() {
        let none = |_: u16| false;
        let groups = groups(&KeyMap::default(), &none, DisplayStyle::Sakura);
        assert!(groups.iter().flat_map(|g| &g.rows).all(|r| !r.enabled));
    }

    #[test]
    fn keys_move_between_the_groups_and_the_functions() {
        let groups = all();
        let mut c = Cursor::default();
        assert_eq!(c.key("down", &groups), Step::Nothing);
        assert_eq!(c.group, 1);
        assert_eq!(c.key("up", &groups), Step::Nothing);
        assert_eq!(c.key("up", &groups), Step::Nothing);
        assert_eq!(c.group, 0);
        c.key("end", &groups);
        assert_eq!(c.group, groups.len() - 1);
        // an empty group (その他) cannot be entered
        assert_eq!(c.key("enter", &groups), Step::Nothing);
        assert!(!c.in_rows);
        c.key("home", &groups);
        c.key("right", &groups);
        assert!(c.in_rows);
        c.key("down", &groups);
        assert_eq!(c.row, 1);
        c.key("left", &groups);
        assert!(!c.in_rows);
        assert_eq!(c.row, 0);
    }

    #[test]
    fn enter_on_a_function_runs_it_and_escape_goes_back_then_closes() {
        let groups = all();
        let mut c = Cursor::default();
        c.key("enter", &groups);
        // the first function of the first group is F_FILENEW
        assert_eq!(c.key("enter", &groups), Step::Run(30101));
        assert_eq!(c.key("escape", &groups), Step::Nothing);
        assert!(!c.in_rows);
        assert_eq!(c.key("escape", &groups), Step::Close);
    }

    #[test]
    fn a_disabled_function_cannot_be_run() {
        let none = |_: u16| false;
        let groups = groups(&KeyMap::default(), &none, DisplayStyle::Sakura);
        let mut c = Cursor::default();
        c.key("enter", &groups);
        assert_eq!(c.key("enter", &groups), Step::Nothing);
    }
}
