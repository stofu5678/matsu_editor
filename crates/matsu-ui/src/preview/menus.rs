//! The menus of the preview tab (`MenuKind::Preview`): the menus the window shows while a preview is
//! the active tab. They are small and have no Sakura function: an item runs one of the actions of
//! [`super`], which the preview handles.

use gpui::{Menu, MenuItem};

use super::{AsHtml, AsMarkdown, Close, Refresh, ToSource};

/// The menus. `with_keys` writes the keys in the labels, as the menus of Windows and Linux do; the
/// native menu of macOS shows them itself from the key bindings.
pub(crate) fn menus(with_keys: bool) -> Vec<Menu> {
    let key = |text: &str| {
        if with_keys {
            format!("    {text}")
        } else {
            String::new()
        }
    };
    vec![Menu::new("プレビュー(P)").items([
        MenuItem::action(format!("更新(R){}", key("F5")), Refresh),
        MenuItem::action(format!("ソースに戻る(S){}", key("Esc")), ToSource),
        MenuItem::separator(),
        MenuItem::action("Markdown として表示(M)", AsMarkdown),
        MenuItem::action("HTML として表示(H)", AsHtml),
        MenuItem::separator(),
        MenuItem::action(format!("プレビューを閉じる(C){}", key("Ctrl+W")), Close),
    ])]
}
