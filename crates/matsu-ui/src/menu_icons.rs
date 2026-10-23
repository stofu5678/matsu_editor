//! The icons in the menus (Sakura's アイコン付きメニュー, `bMenuIcon`).
//!
//! Sakura: `CMenuDrawer::MyAppendMenu` (`sakura_core/uiparts/CMenuDrawer.cpp`) gives an item of
//! a function the bitmap of its icon (`GetIconIdByFuncId`: the picture of the first toolbar
//! button of the function) while `m_Common.m_sWindow.m_bMenuIcon` is on, and the menu keeps a
//! column for the pictures. An item with a check mark and an icon shows the icon in a frame
//! (the check mark of Windows' menus); an item with a check mark and no icon shows the check
//! mark in the column.
//!
//! gpui-component's `PopupMenu` draws an icon from SVG only, so while the setting is on an item
//! is drawn by [`item_element`]: a column, then the label (which already ends with the shortcut).
//! macOS's menu bar is the OS menu, which GPUI builds from `MenuItem`s that have no icon: it
//! stays without icons. The right-click and custom menus are GPUI popups on every OS.

use std::sync::Arc;

use gpui::{
    AnyElement, App, Global, Image, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, Styled as _, div, img, px, rgb,
};
use gpui_component::{Icon, IconName, Sizable as _, h_flex};

use crate::bars::icons;

/// `m_bMenuIcon`: whether the menus show the icons. Sakura's default is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MenuIcons(pub bool);

impl Global for MenuIcons {}

/// Whether the menus show icons now.
pub(crate) fn enabled(cx: &App) -> bool {
    cx.try_global::<MenuIcons>().is_none_or(|m| m.0)
}

/// Takes the setting; menus that open afterwards follow it.
pub(crate) fn set(cx: &mut App, on: bool) {
    cx.set_global(MenuIcons(on));
}

/// The picture of the icon of function `code` for a screen with `scale_factor`, if it has one.
pub(crate) fn picture(code: u16, scale_factor: f32) -> Option<Arc<Image>> {
    let index = crate::bars::icon_of_function(code)?;
    icons::icon(index, icons::pixel_factor(scale_factor))
}

/// The width of the column of the icons, in logical pixels.
pub(crate) fn column_width(scale_factor: f32) -> f32 {
    icons::logical_size(scale_factor).max(icons::ICON_SIZE as f32)
}

const GAP: f32 = 4.0;

/// The space a submenu's label needs in front of it, to start where the labels of the items
/// start (the popup keeps its own column for the item, the item adds [`column_width`] and the
/// gap after it).
pub(crate) fn submenu_padding(scale_factor: f32) -> f32 {
    column_width(scale_factor) + GAP
}

/// One item of a menu with icons: the column (the icon, a frame round it when checked, a check
/// mark when checked and there is no icon, or nothing), then the label. A gray item is drawn
/// pale.
pub(crate) fn item_element(
    label: SharedString,
    picture: Option<Arc<Image>>,
    checked: bool,
    disabled: bool,
    scale_factor: f32,
) -> AnyElement {
    content(label, picture, checked, disabled, scale_factor).into_any_element()
}

/// [`item_element`] before it is made an `AnyElement`.
fn content(
    label: SharedString,
    picture: Option<Arc<Image>>,
    checked: bool,
    disabled: bool,
    scale_factor: f32,
) -> crate::a11y::Flagged<gpui::Stateful<gpui::Div>> {
    let width = column_width(scale_factor);
    let size = icons::logical_size(scale_factor);
    let column = div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .w(px(width))
        .h(px(width));
    let column = match picture {
        Some(picture) => {
            let mut icon = img(picture).w(px(size)).h(px(size));
            if disabled {
                icon = icon.opacity(0.4);
            }
            if checked {
                column
                    .border_1()
                    .border_color(rgb(0x7fa3d1))
                    .bg(rgb(0xc4d8f0))
                    .child(icon)
            } else {
                column.child(icon)
            }
        }
        None if checked => column.child(Icon::new(IconName::Check).xsmall()),
        None => column,
    };
    // An element item has no accessible name of its own (gpui-component's `a11y_label`), so the
    // content is a node with the name, the shortcut, the check mark and the gray state.
    let content =
        crate::a11y::popup_item_content(h_flex().id("menu-item-label"), &label, checked, disabled)
            .gap(px(GAP))
            .items_center()
            .child(column)
            .child(label);
    crate::a11y::disabled(content, disabled)
}

#[cfg(test)]
mod tests {
    use gpui::Role;

    use super::*;
    use crate::a11y::node_of;

    #[test]
    fn an_item_with_an_icon_has_its_name_check_and_gray_state() {
        let item = content("折り返し(W)    Ctrl+W".into(), None, true, true, 1.0);
        let node = node_of(&item).unwrap();
        assert_eq!(node.role(), Role::Label);
        assert_eq!(
            node.value(),
            Some("折り返し、Ctrl+W、チェック済み、使用不可")
        );
        assert_eq!(node.label(), Some("折り返し"));
        assert_eq!(node.keyboard_shortcut(), Some("Ctrl+W"));
        assert_eq!(node.toggled(), Some(gpui::Toggled::True));
        assert!(node.is_disabled());

        let plain = node_of(&content("貼り付け(P)".into(), None, false, false, 1.0)).unwrap();
        assert_eq!(plain.label(), Some("貼り付け"));
        assert_eq!(plain.toggled(), None);
        assert!(!plain.is_disabled());
    }
}
