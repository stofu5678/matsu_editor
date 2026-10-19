//! The parts of the page that the dialog does not draw itself: the drop-down list of kinds and the
//! check box, the list of functions, the buttons between the lists, and the tree.

use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _, px,
    rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_component::{Disableable as _, h_flex, v_flex};
use ume_config::settings::RowKind;

use super::super::super::dialog::CommonSettingsDialog;
use super::super::super::model::{Kind, parse_label};
use super::{KIND, PAGE_KINDS, function_names, functions_of_kind, row_text, state_of, tree_of};

const SELECTED_BG: u32 = 0x2f6fd0;
const LIST_BORDER: u32 = 0xaaaaaa;
const ROW_HEIGHT: f32 = 22.0;
/// The space each level of the tree is moved in.
const INDENT: f32 = 16.0;

/// 種別 and the check box.
pub(super) fn choosers(
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    let values = dialog.values();
    let kind = values.int(KIND);
    let kind_name = PAGE_KINDS
        .iter()
        .find(|(_, v)| *v == kind)
        .map_or("", |(name, _)| *name);
    let weak = cx.entity().downgrade();
    let kind_button = Button::new("mainmenu-kind").label(kind_name).dropdown_menu(
        move |menu: PopupMenu, _, _| {
            PAGE_KINDS.iter().fold(menu, |menu, &(name, value)| {
                let weak = weak.clone();
                menu.item(PopupMenuItem::new(name).on_click(move |_, _, cx| {
                    weak.update(cx, |this, cx| this.set_int(KIND, value, cx))
                        .ok();
                }))
            })
        },
    );
    let parentheses = Checkbox::new("mainmenu-parentheses")
        .label("アクセスキーを必ず( )付で表示(P)")
        .checked(state_of(values).key_parentheses)
        .on_click(cx.listener(|this, on: &bool, _, cx| this.main_menu_set_parentheses(*on, cx)));
    h_flex()
        .gap_2()
        .items_center()
        .justify_between()
        .child(
            h_flex()
                .gap_1()
                .items_center()
                .child("種別(K)")
                .child(kind_button),
        )
        .child(parentheses)
        .into_any_element()
}

fn row_element(
    id: (&'static str, usize),
    text: String,
    selected: bool,
    cx: &mut Context<CommonSettingsDialog>,
    on_click: impl Fn(&mut CommonSettingsDialog, &mut Window, &mut Context<CommonSettingsDialog>)
    + 'static,
) -> AnyElement {
    h_flex()
        .id(id)
        .h(px(ROW_HEIGHT))
        .px_1()
        .items_center()
        .when(selected, |this| {
            this.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
        })
        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| on_click(this, window, cx)))
        .child(text)
        .into_any_element()
}

fn list_box(id: &'static str, rows: Vec<AnyElement>) -> AnyElement {
    v_flex()
        .id(id)
        .debug_selector(|| id.into())
        .w(px(250.))
        .h(px(300.))
        .border_1()
        .border_color(rgb(LIST_BORDER))
        .overflow_y_scroll()
        .children(rows)
        .into_any_element()
}

/// A push button of the page that is a control the dialog does not draw (so that its Alt key
/// works).
fn control_button(
    dialog: &CommonSettingsDialog,
    id: &'static str,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    let controls = dialog.page_controls();
    let Some(control) = controls.iter().find(|c| c.id == id).copied() else {
        return div().into_any_element();
    };
    let (label, _) = parse_label(control.label);
    let usable = dialog.is_usable(&control);
    let Kind::Button { run } = control.kind else {
        return div().into_any_element();
    };
    Button::new(id)
        .label(label)
        .disabled(!usable)
        .on_click(cx.listener(move |this, _, window: &mut Window, cx| run(this, window, cx)))
        .into_any_element()
}

/// 機能, the buttons, and メニュー.
pub(super) fn lists(
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    let values = dialog.values();
    let tree = tree_of(values);
    let functions = functions_of_kind(values.int(KIND));
    // The names first: the rows below borrow `cx` for their listeners.
    let (function_labels, row_texts) = {
        let name_of = function_names(cx);
        let labels: Vec<String> = functions.iter().map(|&code| name_of(code)).collect();
        let texts: Vec<String> = tree
            .rows
            .iter()
            .map(|row| row_text(row, &name_of))
            .collect();
        (labels, texts)
    };

    let function_rows = function_labels
        .into_iter()
        .enumerate()
        .map(|(i, label)| {
            row_element(
                ("mainmenu-func", i),
                label,
                tree.func_sel == Some(i),
                cx,
                move |this, _, cx| this.set_int(super::FUNC_SEL, i as i32, cx),
            )
        })
        .collect();

    let tree_rows = tree
        .visible()
        .into_iter()
        .map(|i| {
            let page_row = &tree.rows[i];
            let indent = px(INDENT * page_row.row.level as f32);
            let is_node = page_row.row.kind == RowKind::Node;
            let glyph = match (is_node, page_row.collapsed) {
                (false, _) => "",
                (true, true) => "▸",
                (true, false) => "▾",
            };
            let selected = tree.sel == Some(i);
            h_flex()
                .h(px(ROW_HEIGHT))
                .items_center()
                .pl(indent)
                .child(
                    div()
                        .id(("mainmenu-toggle", i))
                        .w(px(INDENT))
                        .text_center()
                        .child(glyph)
                        .when(is_node, |this| {
                            this.on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                                this.main_menu_toggle(i, window, cx)
                            }))
                        }),
                )
                .child(
                    div()
                        .id(("mainmenu-res", i))
                        .flex_1()
                        .h_full()
                        .px_1()
                        .flex()
                        .items_center()
                        .when(selected, |this| {
                            this.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
                        })
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.main_menu_select(i, window, cx)
                        }))
                        .child(row_texts[i].clone()),
                )
                .into_any_element()
        })
        .collect();

    let buttons = v_flex()
        .gap_1()
        .pt(px(30.))
        .child(control_button(dialog, "mainmenu_delete", cx))
        .child(control_button(dialog, "mainmenu_node", cx))
        .child(control_button(dialog, "mainmenu_separator", cx))
        .child(control_button(dialog, "mainmenu_above", cx))
        .child(control_button(dialog, "mainmenu_below", cx))
        .child(control_button(dialog, "mainmenu_add", cx))
        .child(control_button(dialog, "mainmenu_up", cx))
        .child(control_button(dialog, "mainmenu_down", cx))
        .child(control_button(dialog, "mainmenu_right", cx))
        .child(control_button(dialog, "mainmenu_left", cx))
        .child(control_button(dialog, "mainmenu_expand", cx))
        .child(control_button(dialog, "mainmenu_collapse", cx));

    h_flex()
        .gap_2()
        .items_start()
        .child(
            v_flex()
                .gap_1()
                .child("機能(F)")
                .child(list_box("mainmenu-functions", function_rows)),
        )
        .child(buttons)
        .child(
            v_flex()
                .gap_1()
                .child("メニュー(M)")
                .child(list_box("mainmenu-tree", tree_rows)),
        )
        .into_any_element()
}
