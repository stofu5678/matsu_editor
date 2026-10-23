//! The pictures of the native menu bar of macOS (ADR-0024, method B).
//!
//! `app::set_menus` gives GPUI the menus, which cannot carry pictures. This module describes the
//! same menus as a tree of [`Node`]s with the picture of each item (the one `crate::menu_icons`
//! draws on Windows and Linux: Sakura's `GetIconIdByFuncId`), and `matsu_platform::menu_icons`
//! puts them on the `NSMenuItem`s that GPUI made. The tree must have the shape of what
//! `app::gpui_items` returns: same order, same titles, items that GPUI leaves out left out here.

use matsu_commands::keymap::DisplayStyle;
use matsu_platform::menu_icons::Node;

use crate::menu::{Entry, MenuAction, label_with_shortcut};

/// The screen scale the pictures are made for. A Retina screen is the common case on a Mac; a
/// 1x screen scales the 32x32 pictures down.
pub(crate) const SCALE: f32 = 2.0;

/// The tree of `entries`, in the shape of `app::gpui_items(entries, style)` on macOS.
pub(crate) fn tree(entries: &[Entry], style: DisplayStyle) -> Vec<Node> {
    entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Separator => Some(Node::Plain),
            Entry::Submenu { label, entries } => Some(Node::Submenu {
                title: label.clone(),
                items: tree(entries, style),
            }),
            Entry::Item(item) => match &item.action {
                // GPUI has no item for it on macOS (Quit is in the application menu).
                MenuAction::Exit if cfg!(target_os = "macos") => None,
                MenuAction::Exit => Some(Node::Plain),
                MenuAction::Function(code) => Some(
                    crate::menu_icons::picture(*code, SCALE)
                        .map(|image| Node::Icon {
                            title: label_with_shortcut(item, style),
                            png: image.bytes.as_slice().into(),
                        })
                        .unwrap_or(Node::Plain),
                ),
                MenuAction::RecentFile(_)
                | MenuAction::RecentFolder(_)
                | MenuAction::Placeholder => Some(Node::Plain),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use matsu_commands::keymap::KeyMap;
    use matsu_platform::menu_icons::Node;

    use super::*;
    use crate::menu::{NoLists, Registered, main_menu_for};

    /// The nodes and the GPUI items have the same shape: same count at every level, a submenu
    /// where GPUI has one, and the same titles for the items that have a picture.
    fn same_shape(nodes: &[Node], items: &[gpui::OwnedMenuItem]) {
        assert_eq!(nodes.len(), items.len());
        for (node, item) in nodes.iter().zip(items) {
            match (node, item) {
                (Node::Plain, _) => {}
                (Node::Icon { title, .. }, gpui::OwnedMenuItem::Action { name, .. }) => {
                    assert_eq!(title, name);
                }
                (
                    Node::Submenu {
                        title,
                        items: nodes,
                    },
                    gpui::OwnedMenuItem::Submenu(menu),
                ) => {
                    assert_eq!(title, &menu.name.to_string());
                    same_shape(nodes, &menu.items);
                }
                (node, _) => panic!("{node:?} does not match a GPUI item"),
            }
        }
    }

    #[test]
    fn the_tree_has_the_shape_of_the_gpui_items_and_some_pictures() {
        let keymap = KeyMap::default();
        let custom = Default::default();
        let tops = main_menu_for(
            crate::menu::default_main_menu(),
            &keymap,
            &Registered,
            &NoLists,
            &custom,
        );
        let mut icons = 0;
        for top in tops {
            let items = crate::app::gpui_items(top.entries.clone(), DisplayStyle::Mac);
            let owned = gpui::Menu::new(top.label.clone()).items(items).owned();
            let nodes = tree(&top.entries, DisplayStyle::Mac);
            same_shape(&nodes, &owned.items);
            icons += count_icons(&nodes);
        }
        assert!(icons > 20, "only {icons} items have a picture");
    }

    fn count_icons(nodes: &[Node]) -> usize {
        nodes
            .iter()
            .map(|n| match n {
                Node::Icon { .. } => 1,
                Node::Submenu { items, .. } => count_icons(items),
                Node::Plain => 0,
            })
            .sum()
    }
}
