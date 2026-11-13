//! The menus of the AI log tab (`MenuKind::AiLog`): the menus the window shows while the log is the active tab.
//! They are small and have no Sakura function: an item runs one of the actions of [`super`].

use gpui::{Menu, MenuItem};

use super::{Close, Copy, SelectAll};

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
    vec![Menu::new("AI ログ(L)").items([
        MenuItem::action(format!("コピー(C){}", key("Ctrl+C")), Copy),
        MenuItem::action(format!("すべて選択(A){}", key("Ctrl+A")), SelectAll),
        MenuItem::separator(),
        MenuItem::action(format!("AI ログを閉じる(X){}", key("Ctrl+W")), Close),
    ])]
}
