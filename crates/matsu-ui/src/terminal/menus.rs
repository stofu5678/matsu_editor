//! The menus of the shell tab (`MenuKind::Terminal`): the menus the window shows while a terminal is the
//! active tab. They are small and have no editor command: closing the tab, and the operations of the window
//! that make sense without an editor on screen. The keys of the window's functions stay with the window while
//! the terminal has the focus (`view::is_window_function`), so the labels carry no key of their own.

use gpui::{Menu, MenuItem};
use matsu_commands::sakura::FUNCTIONS;

use super::Close;
use crate::view::RunFunction;

fn function(name: &str) -> RunFunction {
    RunFunction {
        code: FUNCTIONS
            .iter()
            .find(|f| f.name == name)
            .map_or(0, |f| f.code),
    }
}

/// The menus. The native menu of macOS shows the key of the close item from the key binding.
pub(crate) fn menus() -> Vec<Menu> {
    vec![Menu::new("シェル(S)").items([
        MenuItem::action("シェルのタブを閉じる(C)", Close),
        MenuItem::separator(),
        MenuItem::action("シェルのタブを開く(S)", function("F_SHELL_TAB")),
        MenuItem::action("新規作成(N)", function("F_FILENEW")),
        MenuItem::separator(),
        MenuItem::action("次のウィンドウ(W)", function("F_NEXTWINDOW")),
        MenuItem::action("前のウィンドウ(P)", function("F_PREVWINDOW")),
    ])]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_items_run_functions_that_exist() {
        for name in ["F_SHELL_TAB", "F_FILENEW", "F_NEXTWINDOW", "F_PREVWINDOW"] {
            assert_ne!(function(name).code, 0, "{name}");
        }
    }
}
