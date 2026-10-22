//! The layout of the dialog: `IDD_FILETREE` of `sakura_core/sakura_rc.rc`.
//!
//! Sakura's window has the settings above, the item boxes on the left, a column of buttons in the
//! middle and the tree on the right, with the import and export buttons, OK, キャンセル and
//! ヘルプ at the bottom. The texts are Sakura's; an access key `(&X)` is drawn as `(X)` and left
//! out of the name that a screen reader says (`crate::a11y::label_name`).

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    ParentElement, Render, Role, SharedString, StatefulInteractiveElement, Styled, Window, div, px,
    rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputState};
use gpui_component::radio::Radio;
use gpui_component::{Disableable as _, h_flex, v_flex};

use super::actions::PATH_MENU;
use super::{FileTreeDialog, Kind};
use crate::a11y::{AriaDisabled as _, label_name};
use crate::common_settings::model::parse_label;
use crate::outline::file_tree::source::SettingFrom;

const TEXT: u32 = 0x222222;
const MUTED: u32 = 0x888888;
const BORDER: u32 = 0xaaaaaa;
const SELECTED_BG: u32 = 0x2f6fd0;
const FOCUS_RING: u32 = 0x2f6fd0;

/// The name of the dialog (`IDD_FILETREE`).
pub(crate) const TITLE: &str = "ファイルツリー設定";
/// The name of the tree for a screen reader.
pub(crate) const TREE_NAME: &str = "ファイルツリー";

impl FileTreeDialog {
    /// `IDC_STATIC_SETTFING_FROM` (`SetDataInit`): where the items come from.
    pub(crate) fn setting_text(&self) -> String {
        match (self.loaded_from(), &self.loaded_project_ini) {
            // `STR_FILETREE_FROM_FILE`
            (SettingFrom::File, Some(path)) => format!("設定: {}", path.display()),
            (from, ini) => {
                let mut text = match from {
                    // `STR_FILETREE_FROM_COMMON` and `STR_FILETREE_FROM_TYPE`
                    SettingFrom::Common | SettingFrom::File => "設定: 共通設定".to_owned(),
                    SettingFrom::Type => "設定: タイプ別設定".to_owned(),
                };
                if ini.is_some() {
                    // `LS(F_FILE_TOPMENU)`
                    text.push_str("+ファイル");
                }
                text
            }
        }
    }

    fn button(
        &self,
        id: &'static str,
        label: &'static str,
        enabled: bool,
        cx: &mut Context<Self>,
        run: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> Button {
        Button::new(id)
            .label(parse_label(label).0)
            .accessibility_label(label_name(label))
            .disabled(!enabled)
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| run(this, window, cx)))
    }

    fn labelled_input(
        label: &'static str,
        input: &gpui::Entity<InputState>,
        enabled: bool,
    ) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .child(
                div()
                    .w(px(90.))
                    .when(!enabled, |d| d.text_color(rgb(MUTED)))
                    .child(parse_label(label).0),
            )
            .child(
                div().flex_1().child(
                    Input::new(input)
                        .aria_label(label_name(label))
                        .disabled(!enabled),
                ),
            )
    }

    fn radio(
        &self,
        id: &'static str,
        label: &'static str,
        kind: Kind,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Radio::new(id)
            .label(parse_label(label).0)
            .accessibility_label(label_name(label))
            .checked(self.kind == kind)
            .on_click(cx.listener(move |this, _: &bool, _, cx| this.set_kind(kind, cx)))
    }

    fn check(
        &self,
        id: &'static str,
        label: &'static str,
        on: bool,
        enabled: bool,
        cx: &mut Context<Self>,
        set: impl Fn(&mut Self, bool) + 'static,
    ) -> impl IntoElement {
        Checkbox::new(id)
            .label(parse_label(label).0)
            .accessibility_label(label_name(label))
            .checked(on)
            .disabled(!enabled)
            .aria_disabled(!enabled)
            .on_click(cx.listener(move |this, value: &bool, _, cx| {
                set(this, *value);
                cx.notify();
            }))
    }

    fn group(title: &'static str) -> gpui::Stateful<gpui::Div> {
        v_flex()
            .id(SharedString::from(format!("ft-group-{title}")))
            .role(Role::Group)
            .aria_label(title)
            .gap_1()
            .p_2()
            .border_1()
            .border_color(rgb(BORDER))
            .child(div().text_color(rgb(MUTED)).child(title))
    }

    /// The left column: where the settings come from, the project file, the kind and the boxes of
    /// the item.
    fn left_column(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let path_on = self.path_enabled();
        let grep_on = self.grep_enabled();
        let can_load = self.loaded_from() != SettingFrom::File;
        let path_menu = self.path_menu.then(|| {
            let mut menu = v_flex()
                .id("ft-path-menu")
                .role(Role::Menu)
                .aria_label("パスに入れる")
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(0xffffff));
            for (index, (label, text)) in PATH_MENU.into_iter().enumerate() {
                menu = menu.child(
                    div()
                        .id(("ft-path-menu-item", index))
                        .role(Role::MenuItem)
                        .aria_label(label_name(label))
                        .px_2()
                        .cursor_pointer()
                        .hover(|style| style.bg(rgb(0xdde8f5)))
                        .when(index == self.path_menu_at, |d| d.bg(rgb(0xdde8f5)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.put_in_path(text, window, cx);
                        }))
                        .child(label),
                );
            }
            menu
        });
        v_flex()
            .gap_2()
            .w(px(330.))
            .child(div().child(self.setting_text()))
            .child(
                Checkbox::new("ft-load-ini")
                    .label(parse_label("各フォルダーの設定を読み込む(&I)").0)
                    .accessibility_label(label_name("各フォルダーの設定を読み込む(&I)"))
                    .checked(self.project)
                    .on_click(
                        cx.listener(|this, value: &bool, _, cx| this.set_project(*value, cx)),
                    ),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(parse_label("デフォルト設定ファイル名(&J)").0)
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                div().flex_1().child(
                                    Input::new(&self.default_ini)
                                        .aria_label(label_name("デフォルト設定ファイル名(&J)")),
                                ),
                            )
                            .child(self.button(
                                "ft-ref1",
                                "...(&1)",
                                true,
                                cx,
                                |this, window, cx| {
                                    this.browse_default_ini(window, cx);
                                },
                            )),
                    )
                    .child(h_flex().justify_end().child(self.button(
                        "ft-load",
                        "読み込み(&V)",
                        can_load,
                        cx,
                        |this, window, cx| this.load_default_ini(window, cx),
                    ))),
            )
            .child(
                Self::group("種類").child(
                    h_flex()
                        .gap_3()
                        .child(self.radio("ft-grep", "Grep(&G)", Kind::Grep, cx))
                        .child(self.radio("ft-file", "ファイル(&F)", Kind::File, cx))
                        .child(self.radio("ft-folder", "フォルダー(&2)", Kind::Folder, cx)),
                ),
            )
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(div().flex_1().child(Self::labelled_input(
                        "パス(&T)",
                        &self.path,
                        path_on,
                    )))
                    .child(
                        self.button("ft-ref2", "&...", path_on, cx, |this, window, cx| {
                            this.browse_path(window, cx);
                        }),
                    )
                    .child(self.button(
                        "ft-path-menu-button",
                        "▼(&3)",
                        path_on,
                        cx,
                        |this, _, cx| {
                            this.toggle_path_menu(cx);
                        },
                    )),
            )
            .children(path_menu)
            .child(Self::labelled_input("ラベル(&E)", &self.label, true))
            .child(Self::labelled_input("ファイル(&K)", &self.file, grep_on))
            .child(
                Self::group("非表示属性")
                    .child(self.check(
                        "ft-hidden",
                        "隠しファイル(&W)",
                        self.hidden,
                        grep_on,
                        cx,
                        |this, on| this.hidden = on,
                    ))
                    .child(self.check(
                        "ft-read-only",
                        "読み取り専用(&Z)",
                        self.read_only,
                        grep_on,
                        cx,
                        |this, on| this.read_only = on,
                    ))
                    .child(self.check(
                        "ft-system",
                        "システムファイル(&S)",
                        self.system,
                        grep_on,
                        cx,
                        |this, on| this.system = on,
                    )),
            )
            .into_any_element()
    }

    /// The column of buttons between the boxes and the tree.
    fn button_column(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let add = self.can_add();
        let update = self.can_update();
        v_flex()
            .gap_1()
            .w(px(150.))
            .child(
                self.button("ft-delete", "削除(&4)", true, cx, |this, window, cx| {
                    this.delete_item(window, cx);
                }),
            )
            .child(
                self.button("ft-insert", "上⇒(&Q)", add, cx, |this, window, cx| {
                    this.insert_above(window, cx);
                }),
            )
            .child(
                self.button("ft-insert-a", "下⇒(&A)", add, cx, |this, window, cx| {
                    this.insert_below(window, cx);
                }),
            )
            .child(
                self.button("ft-add", ">>(&B)", add, cx, |this, window, cx| {
                    this.add_item(window, cx);
                }),
            )
            .child(
                self.button("ft-update", "更新(&P)", update, cx, |this, _, cx| {
                    this.update_item(cx);
                }),
            )
            .child(self.button(
                "ft-file-add",
                "ファイル挿入(&C)...",
                true,
                cx,
                |this, window, cx| {
                    this.add_files(window, cx);
                },
            ))
            .child(self.button(
                "ft-replace",
                "パスの一括置換(&N)...",
                true,
                cx,
                |this, window, cx| {
                    this.replace_paths(window, cx);
                },
            ))
            .child(div().h(px(14.)))
            .child(self.button("ft-up", "↑(&U)", true, cx, |this, _, cx| this.move_up(cx)))
            .child(
                self.button("ft-down", "↓(&D)", true, cx, |this, window, cx| {
                    this.move_down(window, cx);
                }),
            )
            .child(
                self.button("ft-right", "→(&R)", true, cx, |this, window, cx| {
                    this.move_right(window, cx);
                }),
            )
            .child(
                self.button("ft-left", "←(&L)", true, cx, |this, window, cx| {
                    this.move_left(window, cx);
                }),
            )
            .into_any_element()
    }

    /// The tree (`IDC_TREE_FL`): a row for each shown node, indented by its depth.
    fn tree_column(&mut self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let rows = self.tree.rows();
        let selected = self.tree.selected().cloned();
        let focused = self.tree_focus.is_focused(window);
        // the selected row comes into view when the selection has moved (a key, an added item)
        if self.revealed != selected {
            if let Some(at) = rows
                .iter()
                .position(|row| Some(&row.path) == selected.as_ref())
            {
                self.tree_scroll.scroll_to_item(at);
            }
            self.revealed.clone_from(&selected);
        }
        let mut list = v_flex()
            .id("ft-tree")
            .role(Role::Tree)
            .aria_label(TREE_NAME)
            .track_focus(&self.tree_focus)
            .track_scroll(&self.tree_scroll)
            .w(px(260.))
            .h(px(380.))
            .overflow_y_scroll()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(if focused { FOCUS_RING } else { BORDER }));
        for (index, row) in rows.into_iter().enumerate() {
            let is_selected = selected.as_ref() == Some(&row.path);
            let path = row.path.clone();
            let toggle_path = row.path.clone();
            let mark = match (row.has_children, row.expanded) {
                (false, _) => "",
                (true, true) => "▾",
                (true, false) => "▸",
            };
            let mut node = h_flex()
                .id(("ft-row", index))
                .role(Role::TreeItem)
                .aria_label(row.label.clone())
                .aria_level(row.depth + 1)
                .aria_selected(is_selected)
                .pl(px(4. + 16. * row.depth as f32))
                .h(px(20.))
                .items_center()
                .cursor_pointer()
                .when(is_selected, |d| {
                    d.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
                })
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.tree_focus.focus(window, cx);
                    this.select_row(&path, window, cx);
                }))
                .child(
                    div()
                        .id(("ft-toggle", index))
                        .w(px(14.))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, window, cx| {
                                this.toggle_row(&toggle_path, window, cx);
                                cx.stop_propagation();
                            }),
                        )
                        .child(mark),
                )
                .child(row.label);
            if row.has_children {
                node = node.aria_expanded(row.expanded);
            }
            list = list.child(node);
        }
        list.into_any_element()
    }

    fn bottom_row(&mut self, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .justify_between()
            .gap_2()
            .child(
                h_flex()
                    .gap_1()
                    .child(self.button(
                        "ft-import",
                        "インポート(&M)",
                        true,
                        cx,
                        |this, window, cx| {
                            this.import(window, cx);
                        },
                    ))
                    .child(self.button(
                        "ft-export",
                        "エクスポート(&Y)",
                        true,
                        cx,
                        |this, window, cx| {
                            this.export(window, cx);
                        },
                    )),
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        // names the button in the tests (nothing in release builds)
                        div()
                            .debug_selector(|| "ft-ok".to_owned())
                            .child(self.button("ft-ok", "OK", true, cx, |this, window, cx| {
                                this.ok(window, cx)
                            })),
                    )
                    .child(self.button(
                        "ft-cancel",
                        "キャンセル(&X)",
                        true,
                        cx,
                        |this, window, cx| {
                            this.cancel(window, cx);
                        },
                    ))
                    // `IDC_BUTTON_HELP`: the help is not available yet
                    .child(self.button("ft-help", "ヘルプ(&H)", false, cx, |_, _, _| {})),
            )
            .into_any_element()
    }

    /// The keys that come before the focused control gets them (a capture): while the list of
    /// the path is open it takes the arrow keys, Enter and the letters of its entries, so Enter in
    /// the path box does not reach OK; Tab and Shift+Tab stay among the controls of the dialog
    /// (`IsDialogMessage`), and not on the text behind it or its menu bar.
    fn on_capture_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if key == "tab" && !modifiers.control && !modifiers.alt {
            self.move_focus(modifiers.shift, window, cx);
            cx.stop_propagation();
        } else if self.path_menu
            && key != "escape"
            && !modifiers.modified()
            && self.path_menu_key(key, window, cx)
        {
            cx.stop_propagation();
        }
    }

    /// Tab: the next control of the dialog, round the end to the first (the last, with Shift).
    pub(crate) fn move_focus(&mut self, back: bool, window: &mut Window, cx: &mut Context<Self>) {
        if back {
            window.focus_prev(cx);
        } else {
            window.focus_next(cx);
        }
        if !self.focus.contains_focused(window, cx) {
            if back {
                self.tree_focus.focus(window, cx);
            } else {
                self.default_ini
                    .update(cx, |state, cx| state.focus(window, cx));
            }
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if key == "escape" {
            if self.path_menu {
                self.path_menu = false;
                cx.notify();
            } else {
                self.cancel(window, cx);
            }
            cx.stop_propagation();
            return;
        }
        if modifiers.alt && !modifiers.control {
            let mut chars = key.chars();
            if let (Some(letter), None) = (chars.next(), chars.next())
                && self.use_access_key(letter, window, cx)
            {
                cx.stop_propagation();
            }
            return;
        }
        if self.tree_focus.is_focused(window) && !modifiers.modified() {
            // Enter is the default button (OK) from the tree too
            if key == "enter" {
                self.ok(window, cx);
                cx.stop_propagation();
            } else if self.tree_key(key, window, cx) {
                cx.stop_propagation();
            }
        }
    }
}

impl Render for FileTreeDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = h_flex()
            .gap_3()
            .items_start()
            .child(self.left_column(cx))
            .child(self.button_column(cx))
            .child(self.tree_column(window, cx));
        let panel = v_flex()
            .id("file-tree-dialog")
            .role(Role::Dialog)
            .aria_label(TITLE)
            .key_context("FileTreeDialog")
            .track_focus(&self.focus)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .capture_key_down(cx.listener(Self::on_capture_key))
            .on_key_down(cx.listener(Self::on_key_down))
            .max_w_full()
            .max_h_full()
            .overflow_y_scroll()
            .p_3()
            .gap_3()
            .bg(rgb(0xf0f0f0))
            .text_color(rgb(TEXT))
            .text_size(px(13.))
            .border_1()
            .border_color(rgb(0x888888))
            .rounded_md()
            .shadow_md()
            .child(div().font_weight(gpui::FontWeight::BOLD).child(TITLE))
            .child(body)
            .child(self.bottom_row(cx));
        div()
            .id("file-tree-dialog-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .child(panel)
    }
}
