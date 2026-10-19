//! What the page draws: the lists, the check boxes and the buttons round them.

use gpui::{
    AnyElement, App, ClickEvent, Context, Global, InteractiveElement as _, IntoElement as _,
    ParentElement as _, ScrollHandle, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::{Disableable as _, h_flex, v_flex};

use super::super::super::dialog::CommonSettingsDialog;
use super::super::super::func_lookup::{KINDS, codes_of_kind};
use super::super::super::model::{Kind, parse_label};
use super::UNASSIGNED;
use super::bindings::{self, modifier_prefix};
use super::{
    ALT_ID, CTRL_ID, FUNC_SEL, KEY_SEL, KIND, SHIFT_ID, SHOWN_FUNC, bindings_of, mods_of, selected,
};
use crate::func_name::FuncNames;

const SELECTED_BG: u32 = 0x2f6fd0;
const LIST_BORDER: u32 = 0xaaaaaa;
const MUTED: u32 = 0x888888;
const ROW_HEIGHT: f32 = 20.0;
const LIST_WIDTH: f32 = 240.0;

/// The scroll position of the key list, kept so that a click in the list of assigned keys can
/// bring that key into view (the list box of Sakura does this by selecting it).
struct KeyListScroll(ScrollHandle);

impl Global for KeyListScroll {}

pub(super) fn key_list_scroll(cx: &mut App) -> ScrollHandle {
    if let Some(scroll) = cx.try_global::<KeyListScroll>() {
        return scroll.0.clone();
    }
    let handle = ScrollHandle::new();
    cx.set_global(KeyListScroll(handle.clone()));
    handle
}

/// One row of a list, highlighted when selected.
fn row(
    id: (&'static str, usize),
    text: String,
    selected: bool,
    cx: &mut Context<CommonSettingsDialog>,
    on_click: impl Fn(&mut CommonSettingsDialog, &mut Context<CommonSettingsDialog>) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .h(px(ROW_HEIGHT))
        .flex_shrink_0()
        .px_1()
        .flex()
        .items_center()
        .whitespace_nowrap()
        .when(selected, |this| {
            this.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
        })
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| on_click(this, cx)))
        .child(text)
        .into_any_element()
}

fn list_box(id: &'static str, height: f32, rows: Vec<AnyElement>) -> gpui::Stateful<gpui::Div> {
    v_flex()
        .id(id)
        .debug_selector(|| id.into())
        .w(px(LIST_WIDTH))
        .h(px(height))
        .border_1()
        .border_color(rgb(LIST_BORDER))
        .bg(rgb(0xffffff))
        .overflow_y_scroll()
        .children(rows)
}

pub(super) fn draw(
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    let values = dialog.values();
    let table = bindings_of(values);
    let mods = mods_of(values);
    let kind = values.int(KIND);
    let kind_name = KINDS
        .iter()
        .find(|(_, v)| *v == kind)
        .map_or("", |(name, _)| *name);
    let codes = codes_of_kind(kind);
    let func_sel = selected(values, FUNC_SEL, codes.len());
    let key_sel = selected(values, KEY_SEL, table.rows.len());
    let names = FuncNames::new();

    let function_rows = codes
        .iter()
        .enumerate()
        .map(|(i, &code)| {
            row(
                ("keybind-func", i),
                names.name(cx, code),
                func_sel == Some(i),
                cx,
                move |this, cx| this.select_function(i, cx),
            )
        })
        .collect();

    // The keys of the function that was selected last (`GetKeyStrList`).
    let shown = table.assigned_to(u16::try_from(values.int(SHOWN_FUNC)).unwrap_or(0));
    let shown_sel = selected(values, "keybind_shown_sel", shown.len());
    let shown_rows = shown
        .iter()
        .enumerate()
        .map(|(i, found)| {
            let found = found.clone();
            row(
                ("keybind-shown", i),
                found.text.clone(),
                shown_sel == Some(i),
                cx,
                move |this, cx| {
                    key_list_scroll(cx).scroll_to_item(found.key);
                    this.select_assigned(i, &found, cx);
                },
            )
        })
        .collect();

    // The key list shows the checked modifiers before every key (`ChangeKeyList`).
    let prefix = modifier_prefix(mods);
    let key_rows = table
        .rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            row(
                ("keybind-key", i),
                format!("{prefix}{}", r.key.name()),
                key_sel == Some(i),
                cx,
                move |this, cx| this.select_key(i, cx),
            )
        })
        .collect();
    let key_list = list_box("keybind-keys", 260., key_rows).track_scroll(&key_list_scroll(cx));

    // The function of the selected key.
    let key_function = key_sel.map_or_else(String::new, |key| match table.function(key, mods) {
        0 => UNASSIGNED.to_owned(),
        code => names.name(cx, code),
    });
    let unusable = key_sel
        .and_then(|key| table.rows.get(key))
        .and_then(|r| bindings::why_unusable(r.key));

    let weak = cx.entity().downgrade();
    let kind_button =
        Button::new("keybind-kind")
            .label(kind_name)
            .dropdown_menu(move |menu, _, _| {
                KINDS.iter().fold(menu, |menu, &(name, value)| {
                    let weak = weak.clone();
                    menu.item(PopupMenuItem::new(name).on_click(move |_, _, cx| {
                        weak.update(cx, |this, cx| this.set_int(KIND, value, cx))
                            .ok();
                    }))
                })
            });

    let controls = dialog.page_controls();
    let button = |id: &'static str, cx: &mut Context<CommonSettingsDialog>| -> AnyElement {
        let Some(control) = controls.iter().find(|c| c.id == id).copied() else {
            return div().into_any_element();
        };
        let (label, _) = parse_label(control.label);
        let Kind::Button { run } = control.kind else {
            return div().into_any_element();
        };
        let note = (!control.live).then(|| div().text_color(rgb(MUTED)).child("（未対応）"));
        h_flex()
            .gap_1()
            .items_center()
            .child(
                Button::new(id)
                    .label(label)
                    .disabled(!dialog.is_usable(&control))
                    .on_click(
                        cx.listener(move |this, _, window: &mut Window, cx| run(this, window, cx)),
                    ),
            )
            .children(note)
            .into_any_element()
    };
    let check = |id: &'static str, cx: &mut Context<CommonSettingsDialog>| -> AnyElement {
        let label = controls
            .iter()
            .find(|c| c.id == id)
            .map_or_else(String::new, |c| parse_label(c.label).0);
        Checkbox::new(id)
            .label(label)
            .checked(dialog.values().bool(id))
            .on_click(cx.listener(move |this, on: &bool, _, cx| this.set_bool(id, *on, cx)))
            .into_any_element()
    };

    let left = v_flex()
        .gap_1()
        .child(
            h_flex()
                .gap_1()
                .items_center()
                .child("種別(K)")
                .child(kind_button),
        )
        .child("機能(F)")
        .child(list_box("keybind-functions", 220., function_rows))
        .child("機能に割り当てられているキー(G)")
        .child(list_box("keybind-assigned", 96., shown_rows));
    let middle = v_flex()
        .gap_2()
        .pt(px(40.))
        .child(check(SHIFT_ID, cx))
        .child(check(CTRL_ID, cx))
        .child(check(ALT_ID, cx))
        .child(div().h(px(60.)))
        .child(button("keybind_assign", cx))
        .child(button("keybind_release", cx));
    let right = v_flex()
        .gap_1()
        .child(
            h_flex()
                .gap_2()
                .child(button("keybind_import", cx))
                .child(button("keybind_export", cx)),
        )
        .child("キー(Y)")
        .child(key_list)
        .child("キーに割り当てられている機能")
        .child(
            div()
                .id("keybind-key-function")
                .debug_selector(|| "keybind-key-function".into())
                .w(px(LIST_WIDTH))
                .h(px(24.))
                .px_1()
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(LIST_BORDER))
                .child(key_function),
        )
        .children(unusable.map(|why| {
            div()
                .id("keybind-unusable")
                .debug_selector(|| "keybind-unusable".into())
                .w(px(LIST_WIDTH))
                .text_size(px(11.))
                .text_color(rgb(MUTED))
                .child(why)
        }));

    h_flex()
        .gap_3()
        .items_start()
        .child(left)
        .child(middle)
        .child(right)
        .into_any_element()
}
