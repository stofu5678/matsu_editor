//! The name of a function (`CFuncLookup::Funccode2Name`, `sakura_core/func/CFuncLookup.cpp`), the one
//! lookup that the key assignments page, the custom menus (the page and the popup), the toolbar (the
//! page and the tooltips) and the function key bar share.
//!
//! * a registered macro (`F_USERMACRO_0` ...) is named by its entry on the 「マクロ」 page, and
//!   `マクロ n (未登録)` when it has no file (`STR_ERR_DLGFUNCLKUP03`);
//! * the right-click menu and the custom menus are named by their menus (`Custmenu2Name`);
//! * any other function is the label of the string table (`LS( funccode )`), and a code with none is
//!   `-- 不明 --` (`F_DISABLE`'s string).
//!
//! Plugin commands do not exist here.

use std::cell::OnceCell;

use gpui::App;
use matsu_commands::sakura::FUNCTIONS;
use matsu_config::settings::CustomMenus;
use matsu_config::settings::macros::{RegisteredMacros, index_of_function};

use crate::custom_menu::model::menu_index;
use crate::settings::UserSettings;

/// `F_DISABLE` in the string table: what a code with no name is.
pub(crate) const UNKNOWN_NAME: &str = "-- 不明 --";

/// `Funccode2Name` for the given custom menus and registered macros.
pub(crate) fn funccode_to_name(
    code: u16,
    menus: &CustomMenus,
    macros: &RegisteredMacros,
) -> String {
    resolve(code, &|| menus, &|| macros)
}

fn resolve<'a>(
    code: u16,
    menus: &dyn Fn() -> &'a CustomMenus,
    macros: &dyn Fn() -> &'a RegisteredMacros,
) -> String {
    if index_of_function(code).is_some() {
        if let Some(title) = crate::macros::title_of(macros(), code) {
            return title;
        }
    } else if let Some(index) = menu_index(code) {
        return menus()
            .name(index)
            .unwrap_or_else(|| UNKNOWN_NAME.to_owned());
    }
    // `&&` in the string table is a literal `&`.
    FUNCTIONS
        .iter()
        .find(|f| f.code == code)
        .map(|f| f.label.filter(|label| !label.is_empty()).unwrap_or(f.name))
        .map_or_else(
            || UNKNOWN_NAME.to_owned(),
            |label| crate::menu::menu_label(label, None),
        )
}

/// Gives the names of functions from the settings now. The custom menus and the registered macros are
/// read only when a name needs them, and once.
pub(crate) struct FuncNames {
    menus: OnceCell<CustomMenus>,
    macros: OnceCell<RegisteredMacros>,
}

impl FuncNames {
    pub(crate) fn new() -> Self {
        Self {
            menus: OnceCell::new(),
            macros: OnceCell::new(),
        }
    }

    /// The name of function `code`.
    pub(crate) fn name(&self, cx: &App, code: u16) -> String {
        resolve(
            code,
            &|| {
                self.menus.get_or_init(|| {
                    cx.try_global::<UserSettings>()
                        .map(UserSettings::custom_menus)
                        .unwrap_or_default()
                })
            },
            &|| {
                self.macros
                    .get_or_init(|| crate::macros::registered_table(cx))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_are_the_labels_of_the_table() {
        let (menus, macros) = (CustomMenus::default(), RegisteredMacros::default());
        // F_FILE_SAVE, then a code that nothing has
        assert_eq!(funccode_to_name(30103, &menus, &macros), "上書き保存");
        assert_eq!(funccode_to_name(64999, &menus, &macros), UNKNOWN_NAME);
    }

    #[test]
    fn a_macro_without_a_file_is_not_registered() {
        let (menus, macros) = (CustomMenus::default(), RegisteredMacros::default());
        assert_eq!(
            funccode_to_name(31600, &menus, &macros),
            "マクロ 0 (未登録)"
        );
        assert_eq!(
            funccode_to_name(31649, &menus, &macros),
            "マクロ 49 (未登録)"
        );
    }

    #[test]
    fn the_custom_menus_are_named_by_their_menus() {
        let (menus, macros) = (CustomMenus::default(), RegisteredMacros::default());
        assert_eq!(
            funccode_to_name(31580, &menus, &macros),
            "右クリックメニュー"
        );
        assert_eq!(funccode_to_name(31524, &menus, &macros), "タブメニュー");
        assert_eq!(funccode_to_name(31504, &menus, &macros), "メニュー4");
    }
}
