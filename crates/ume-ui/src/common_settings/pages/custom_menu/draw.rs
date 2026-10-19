//! The parts of the page that the dialog does not draw itself: the two drop-down lists, the lists
//! of functions and of the items with the buttons between them, and the check box.

use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _, px,
    rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::{Disableable as _, h_flex, v_flex};
use ume_config::settings::custom_menu::MAX_CUSTOM_MENU;

use super::super::super::dialog::CommonSettingsDialog;
use super::super::super::model::{Kind, parse_label};
use super::{
    KIND, PAGE_KINDS, current_menu, edit_of, function_name, functions_of_kind, menus_of, row_text,
};

const SELECTED_BG: u32 = 0x2f6fd0;
const LIST_BORDER: u32 = 0xaaaaaa;
const ROW_HEIGHT: f32 = 22.0;

/// 種別 and 選択: the kind of the functions and the menu that is edited.
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
    let menus = menus_of(values);
    let menu_name = menus.name(current_menu(values)).unwrap_or_default();

    let weak = cx.entity().downgrade();
    let kind_button = {
        let weak = weak.clone();
        Button::new("custmenu-kind")
            .label(kind_name)
            .dropdown_menu(move |menu, _, _| {
                PAGE_KINDS.iter().fold(menu, |menu, &(name, value)| {
                    let weak = weak.clone();
                    menu.item(PopupMenuItem::new(name).on_click(move |_, _, cx| {
                        weak.update(cx, |this, cx| this.set_int(KIND, value, cx))
                            .ok();
                    }))
                })
            })
    };
    let names: Vec<String> = (0..MAX_CUSTOM_MENU)
        .map(|i| menus.name(i).unwrap_or_default())
        .collect();
    let menu_button =
        Button::new("custmenu-menu")
            .label(menu_name)
            .dropdown_menu(move |menu, _, _| {
                names.iter().enumerate().fold(menu, |menu, (index, name)| {
                    let weak = weak.clone();
                    menu.item(
                        PopupMenuItem::new(name.clone()).on_click(move |_, window, cx| {
                            weak.update(cx, |this, cx| this.select_menu(index, window, cx))
                                .ok();
                        }),
                    )
                })
            });
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
        .child(
            h_flex()
                .gap_1()
                .items_center()
                .child("選択(C)")
                .child(menu_button),
        )
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
    let edit = edit_of(values);
    let menus = menus_of(values);
    let macros = crate::macros::registered_table(cx);
    let functions = functions_of_kind(values.int(KIND));

    let function_rows = functions
        .iter()
        .enumerate()
        .map(|(i, &code)| {
            row_element(
                ("custmenu-func", i),
                function_name(code, &menus, &macros),
                edit.func_sel == Some(i),
                cx,
                move |this, _, cx| this.set_int(super::FUNC_SEL, i as i32, cx),
            )
        })
        .collect();
    let item_rows = edit
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            row_element(
                ("custmenu-res", i),
                row_text(item, &menus, &macros),
                edit.sel == Some(i),
                cx,
                move |this, window, cx| this.select_item(i, window, cx),
            )
        })
        .collect();

    let buttons = v_flex()
        .gap_2()
        .pt(px(30.))
        .child(control_button(dialog, "custmenu_delete", cx))
        .child(control_button(dialog, "custmenu_separator", cx))
        .child(control_button(dialog, "custmenu_insert", cx))
        .child(control_button(dialog, "custmenu_add", cx))
        .child(control_button(dialog, "custmenu_up", cx))
        .child(control_button(dialog, "custmenu_down", cx));

    h_flex()
        .gap_2()
        .items_start()
        .child(
            v_flex()
                .gap_1()
                .child("機能(F)")
                .child(list_box("custmenu-functions", function_rows)),
        )
        .child(buttons)
        .child(
            v_flex()
                .gap_1()
                .child("メニュー(M)")
                .child(list_box("custmenu-items", item_rows)),
        )
        .into_any_element()
}

/// 「サブメニューとして表示」: whether the menu is a submenu in the menus that hold it.
pub(super) fn submenu(
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    let values = dialog.values();
    let popup = menus_of(values)
        .get(current_menu(values))
        .is_some_and(|m| m.popup);
    Checkbox::new("custmenu-submenu")
        .label("サブメニューとして表示(S)")
        .checked(popup)
        .on_click(cx.listener(|this, on: &bool, _, cx| this.set_submenu(*on, cx)))
        .into_any_element()
}
