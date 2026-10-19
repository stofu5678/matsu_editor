//! The names of the functions the page shows (`CFuncLookup::Funccode2Name`).

use std::cell::OnceCell;

use gpui::App;
use ume_commands::sakura::FUNCTIONS;
use ume_config::settings::CustomMenus;

use crate::bars;
use crate::settings::UserSettings;

/// `F_MENU_RBUTTON`: the right-click menu.
const RIGHT_CLICK_MENU: u16 = 31580;
/// `F_CUSTMENU_BASE`: `F_CUSTMENU_1` to `F_CUSTMENU_24` follow it.
const CUSTOM_MENU_BASE: u16 = 31500;
const CUSTOM_MENU_LAST: u16 = 31524;
/// `F_USERMACRO_0`, the first of the registered macros (`MAX_CUSTMACRO`, 50, of them).
const USER_MACRO_BASE: u16 = 31600;
const USER_MACRO_LAST: u16 = 31649;

/// `STR_PROPCOMKEYBIND_UNASSIGN`: what the function of a key is when it has none.
pub(super) const UNASSIGNED: &str = "未割当";
/// `F_DISABLE` in the string table.
const UNKNOWN: &str = "-- 不明 --";

/// Gives the names of functions. The names of the custom menus are in the settings file, which
/// is read only when one is asked for.
pub(super) struct Names {
    menus: OnceCell<CustomMenus>,
}

impl Names {
    pub(super) fn new() -> Self {
        Self {
            menus: OnceCell::new(),
        }
    }

    /// The name of function `code`: the name set for a custom menu, the label of the string table
    /// for the others. A code with neither is `-- 不明 --`.
    pub(super) fn of(&self, code: u16, cx: &App) -> String {
        match code {
            RIGHT_CLICK_MENU => self.menu(0, cx),
            c if (CUSTOM_MENU_BASE + 1..=CUSTOM_MENU_LAST).contains(&c) => {
                self.menu(usize::from(c - CUSTOM_MENU_BASE), cx)
            }
            // `STR_ERR_DLGFUNCLKUP03`: no macro can be registered here.
            c if (USER_MACRO_BASE..=USER_MACRO_LAST).contains(&c) => {
                format!("マクロ {} (未登録)", c - USER_MACRO_BASE)
            }
            c => by_label(c),
        }
    }

    fn menu(&self, index: usize, cx: &App) -> String {
        let menus = self.menus.get_or_init(|| {
            cx.try_global::<UserSettings>()
                .map(UserSettings::custom_menus)
                .unwrap_or_default()
        });
        menus.name(index).unwrap_or_else(|| UNKNOWN.to_owned())
    }
}

fn by_label(code: u16) -> String {
    let name = bars::function_name(code);
    if !name.is_empty() {
        return name;
    }
    FUNCTIONS
        .iter()
        .find(|f| f.code == code)
        .map_or_else(|| UNKNOWN.to_owned(), |f| f.name.to_owned())
}
