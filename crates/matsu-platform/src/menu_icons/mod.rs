//! Pictures in the menu bar of macOS (ADR-0024, method B).
//!
//! GPUI builds the native menu bar from `MenuItem`s, which have no picture, so the menus of macOS
//! had no icons while the menus MATSU Editor draws itself (Windows and Linux) have. After
//! `cx.set_menus` has made the `NSMenu`, [`apply`] walks `NSApp.mainMenu` and gives
//! `NSMenuItem::setImage` to the items that have an icon.
//!
//! The items are found through the shape of the menu, not through GPUI's `tag` (a number that
//! grows with every rebuild of the menu): the top-level items of `mainMenu` are the menus in the
//! order they were given to `set_menus`, and the items of a submenu are the `MenuItem`s in order,
//! separators and submenus included. The OS may add items to the end of a menu (Edit gets
//! "Start Dictation..."); a position past the tree is left alone. An item whose title is not the
//! one expected is left alone too, so a menu that does not look as it was built gets no picture
//! rather than a wrong one.
//!
//! Only macOS has something to do; elsewhere [`apply`] returns 0. The calls into AppKit are not
//! run by the tests of CI (no desktop); `scripts/mac-checks/menu-icons.sh` dumps the items that
//! have a picture.

#[cfg(target_os = "macos")]
mod mac;

use std::sync::Arc;

/// One item of a menu, as it was given to GPUI.
#[derive(Clone, Debug)]
pub enum Node {
    /// A separator, or an item without a picture.
    Plain,
    /// An item that gets a picture. `title` is the title GPUI gave the item.
    Icon { title: String, png: Arc<[u8]> },
    /// A submenu: its title and its items.
    Submenu { title: String, items: Vec<Node> },
}

/// A top-level menu: its title and its items.
#[derive(Clone, Debug)]
pub struct Menu {
    pub title: String,
    pub items: Vec<Node>,
}

/// Gives the pictures to the items of the menu bar that is set now. `size` is the side of a
/// picture in points (the PNG files have more pixels for a Retina screen). Returns how many items got
/// a picture. Call it on the main thread, after `cx.set_menus`; elsewhere it does nothing.
pub fn apply(menus: &[Menu], size: f64) -> usize {
    #[cfg(target_os = "macos")]
    {
        mac::apply(menus, size)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (menus, size);
        0
    }
}

/// The menu bar as it is now, one line per item (`path<TAB>size of the picture or -`); empty
/// elsewhere than macOS. For `scripts/mac-checks/menu-icons.sh` (`MATSU_DUMP_MENU_ICONS`).
pub fn describe() -> String {
    #[cfg(target_os = "macos")]
    {
        mac::describe()
    }
    #[cfg(not(target_os = "macos"))]
    {
        String::new()
    }
}
