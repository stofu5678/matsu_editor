//! The menus of the shell tab (`MenuKind::Terminal`): the menus the window shows while a terminal is the
//! active tab. They are small and have no editor command: closing the tab, and the operations of the window
//! that make sense without an editor on screen. The keys of the window's functions stay with the window while
//! the terminal has the focus (`view::is_window_function`), so the labels carry no key of their own.

use gpui::{Menu, MenuItem, OwnedMenuItem};
use matsu_commands::sakura::FUNCTIONS;

use super::{Close, Copy, OpenSettings, Paste};
use crate::app::Workspace;
use crate::tabs::content::TabContent;
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
        // Gray while nothing is selected (`mark_items`; the native menu of macOS grays an action that
        // nobody handles, and the view handles `Copy` only while something is selected).
        MenuItem::action("コピー(O)", Copy),
        MenuItem::action("貼り付け(V)", Paste),
        MenuItem::separator(),
        // Alt+letter goes to the program in a shell tab, so the menu is the way to the settings from here.
        MenuItem::action("シェルの設定(T)...", OpenSettings),
        MenuItem::action("シェルのタブを開く(S)", function("F_SHELL_TAB")),
        MenuItem::action("新規作成(N)", function("F_FILENEW")),
        MenuItem::separator(),
        MenuItem::action("次のウィンドウ(W)", function("F_NEXTWINDOW")),
        MenuItem::action("前のウィンドウ(P)", function("F_PREVWINDOW")),
    ])]
}

/// Grays the copy item of the menus while the active shell has nothing selected (as Sakura's `InitMenu` grays
/// the items each time a menu opens). `items` are the items of a menu that is about to open.
pub(crate) fn mark_items(items: &mut [OwnedMenuItem], workspace: &Workspace, cx: &gpui::App) {
    let selected = workspace
        .tab_content(workspace.active_tab())
        .and_then(TabContent::terminal)
        .is_some_and(|tab| tab.view().read(cx).has_selection());
    gray_copy(items, selected);
}

fn gray_copy(items: &mut [OwnedMenuItem], selected: bool) {
    for item in items {
        match item {
            OwnedMenuItem::Action {
                action, disabled, ..
            } if action.as_any().downcast_ref::<Copy>().is_some() => *disabled |= !selected,
            OwnedMenuItem::Submenu(menu) => gray_copy(&mut menu.items, selected),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<OwnedMenuItem> {
        menus().into_iter().next().expect("a menu").owned().items
    }

    fn copy_is_disabled(items: &[OwnedMenuItem]) -> bool {
        items
            .iter()
            .find_map(|item| match item {
                OwnedMenuItem::Action {
                    action, disabled, ..
                } if action.as_any().downcast_ref::<Copy>().is_some() => Some(*disabled),
                _ => None,
            })
            .expect("the copy item")
    }

    #[test]
    fn the_copy_item_is_gray_without_a_selection_and_paste_never_is() {
        let mut gray = items();
        gray_copy(&mut gray, false);
        assert!(copy_is_disabled(&gray));
        let mut lit = items();
        gray_copy(&mut lit, true);
        assert!(!copy_is_disabled(&lit));
        let paste_disabled = gray.iter().any(|item| {
            matches!(
                item,
                OwnedMenuItem::Action { action, disabled: true, .. }
                    if action.as_any().downcast_ref::<Paste>().is_some()
            )
        });
        assert!(!paste_disabled);
    }

    #[test]
    fn the_menu_has_the_settings_item_and_no_access_key_twice() {
        let names: Vec<String> = items()
            .iter()
            .filter_map(|item| match item {
                OwnedMenuItem::Action { name, action, .. } => {
                    action.as_any().downcast_ref::<OpenSettings>()?;
                    Some(name.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(names, ["シェルの設定(T)..."]);
        let mut keys: Vec<char> = items()
            .iter()
            .filter_map(|item| match item {
                OwnedMenuItem::Action { name, .. } => name.rsplit_once('(')?.1.chars().next(),
                _ => None,
            })
            .collect();
        let all = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), all, "an access key is used twice: {keys:?}");
    }

    #[test]
    fn the_items_run_functions_that_exist() {
        for name in ["F_SHELL_TAB", "F_FILENEW", "F_NEXTWINDOW", "F_PREVWINDOW"] {
            assert_ne!(function(name).code, 0, "{name}");
        }
    }
}
