//! `NSMenuItem::setImage` on the items of `NSApp.mainMenu`. See the parent module.

use objc2::rc::Retained;
use objc2::{AnyThread as _, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSImage, NSMenu, NSMenuItem};
use objc2_foundation::{NSData, NSSize};

use super::{Menu, Node};

pub(super) fn apply(menus: &[Menu], size: f64) -> usize {
    let Some(mtm) = MainThreadMarker::new() else {
        return 0;
    };
    let Some(bar) = NSApplication::sharedApplication(mtm).mainMenu() else {
        return 0;
    };
    let mut count = 0;
    for (index, menu) in menus.iter().enumerate() {
        let Some(top) = bar.itemAtIndex(index as isize) else {
            break;
        };
        if top.title().to_string() != menu.title {
            continue;
        }
        if let Some(submenu) = top.submenu() {
            count += apply_items(&submenu, &menu.items, size);
        }
    }
    count
}

fn apply_items(menu: &NSMenu, nodes: &[Node], size: f64) -> usize {
    let mut count = 0;
    for (index, node) in nodes.iter().enumerate() {
        let Some(item) = menu.itemAtIndex(index as isize) else {
            break;
        };
        match node {
            Node::Plain => {}
            Node::Icon { title, png } => {
                if item.title().to_string() == *title && set_image(&item, png, size) {
                    count += 1;
                }
            }
            Node::Submenu { title, items } => {
                if item.title().to_string() == *title
                    && let Some(submenu) = item.submenu()
                {
                    count += apply_items(&submenu, items, size);
                }
            }
        }
    }
    count
}

fn set_image(item: &Retained<NSMenuItem>, png: &[u8], size: f64) -> bool {
    let data = NSData::with_bytes(png);
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
        return false;
    };
    // The picture is a colored bitmap (Sakura's), not a template image: it is not tinted. Its size
    // in points is the logical size; the pixels beyond that are for a Retina screen.
    image.setSize(NSSize::new(size, size));
    item.setImage(Some(&image));
    true
}

/// One line per item of the menu bar as it is now: the path of titles, a tab, and the size of the
/// picture in points or `-`. For `scripts/mac-checks/menu-icons.sh`.
pub(super) fn describe() -> String {
    let Some(mtm) = MainThreadMarker::new() else {
        return String::new();
    };
    let Some(bar) = NSApplication::sharedApplication(mtm).mainMenu() else {
        return String::new();
    };
    let mut out = String::new();
    describe_menu(&bar, "", &mut out);
    out
}

fn describe_menu(menu: &NSMenu, path: &str, out: &mut String) {
    for index in 0..menu.numberOfItems() {
        let Some(item) = menu.itemAtIndex(index) else {
            continue;
        };
        let title = item.title().to_string();
        let here = if path.is_empty() {
            title.clone()
        } else {
            format!("{path} > {title}")
        };
        if let Some(submenu) = item.submenu() {
            describe_menu(&submenu, &here, out);
        } else if !item.isSeparatorItem() {
            let picture = item.image().map_or_else(
                || "-".to_owned(),
                |image| format!("{}x{}", image.size().width, image.size().height),
            );
            out.push_str(&format!("{here}\t{picture}\n"));
        }
    }
}
