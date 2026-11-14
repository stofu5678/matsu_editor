//! The name of a function (`CFuncLookup::Funccode2Name`, `sakura_core/func/CFuncLookup.cpp`), the one
//! lookup that the key assignments page, the custom menus (the page and the popup), the toolbar (the
//! page and the tooltips) and the function key bar share.
//!
//! * a registered macro (`F_USERMACRO_0` ...) is named by its entry on the 「マクロ」 page, and
//!   `マクロ n (未登録)` when it has no file (`STR_ERR_DLGFUNCLKUP03`);
//! * a registered one-liner (`F_ONELINER_0` ..., ADR-0029) is named by its entry of `[OneLiner]`, and
//!   `ワンライナー n (未登録)` when it has no command (MATSU's own wording, as the macros');
//! * the right-click menu and the custom menus are named by their menus (`Custmenu2Name`);
//! * any other function is the label of the string table (`LS( funccode )`), and a code with none is
//!   `-- 不明 --` (`F_DISABLE`'s string).
//!
//! * a plugin command that is loaded is named by its label (`CJackManager::GetCommandName`); one that is not is
//!   `-- 不明 --`.

use std::cell::OnceCell;

use gpui::App;
use matsu_commands::sakura::FUNCTIONS;
use matsu_config::settings::CustomMenus;
use matsu_config::settings::macros::{RegisteredMacros, index_of_function};
use matsu_config::settings::oneliner::{self, OneLiners};

use crate::custom_menu::model::menu_index;
use crate::settings::UserSettings;

/// `F_DISABLE` in the string table: what a code with no name is.
pub(crate) const UNKNOWN_NAME: &str = "-- 不明 --";

/// `Funccode2Name` for the given custom menus and registered macros. A one-liner is named as not registered
/// here: the pages that call this have no one-liners yet (the settings page of PR 3 will pass them, with
/// [`funccode_to_name_with`]).
pub(crate) fn funccode_to_name(
    code: u16,
    menus: &CustomMenus,
    macros: &RegisteredMacros,
) -> String {
    funccode_to_name_with(code, menus, macros, &OneLiners::default())
}

/// `Funccode2Name` for the given custom menus, registered macros and registered one-liners.
pub(crate) fn funccode_to_name_with(
    code: u16,
    menus: &CustomMenus,
    macros: &RegisteredMacros,
    oneliners: &OneLiners,
) -> String {
    resolve(code, &|| menus, &|| macros, &|| oneliners)
}

/// The name of the function `code` if it is a registered one-liner: the name, or the command when there is
/// no name; `ワンライナー n (未登録)` when there is no command.
pub(crate) fn oneliner_title(table: &OneLiners, code: u16) -> Option<String> {
    let index = oneliner::index_of_function(code)?;
    let one = table.get(index)?;
    Some(if one.is_registered() {
        one.title().to_owned()
    } else {
        format!("ワンライナー {index} (未登録)")
    })
}

fn resolve<'a>(
    code: u16,
    menus: &dyn Fn() -> &'a CustomMenus,
    macros: &dyn Fn() -> &'a RegisteredMacros,
    oneliners: &dyn Fn() -> &'a OneLiners,
) -> String {
    if oneliner::index_of_function(code).is_some()
        && let Some(title) = oneliner_title(oneliners(), code)
    {
        return title;
    }
    if index_of_function(code).is_some() {
        if let Some(title) = crate::macros::title_of(macros(), code) {
            return title;
        }
    } else if let Some(index) = menu_index(code) {
        return menus()
            .name(index)
            .unwrap_or_else(|| UNKNOWN_NAME.to_owned());
    }
    // A plugin command is named by its label (`CJackManager::GetCommandName`).
    if let Some(label) = matsu_commands::plugin_commands::label(code) {
        return label;
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
    oneliners: OnceCell<OneLiners>,
}

impl FuncNames {
    pub(crate) fn new() -> Self {
        Self {
            menus: OnceCell::new(),
            macros: OnceCell::new(),
            oneliners: OnceCell::new(),
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
            &|| {
                self.oneliners.get_or_init(|| {
                    cx.try_global::<UserSettings>()
                        .map(UserSettings::oneliners)
                        .unwrap_or_default()
                })
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
    fn a_plugin_command_is_named_by_its_label_while_it_is_loaded() {
        use matsu_commands::plugin_commands::{self as pc, CommandInfo};
        let (menus, macros) = (CustomMenus::default(), RegisteredMacros::default());
        let code = pc::function_code(39, 49).unwrap();
        assert_eq!(funccode_to_name(code, &menus, &macros), UNKNOWN_NAME);
        pc::add_command(CommandInfo {
            code,
            slot: 39,
            plug: 49,
            plugin_id: "func.name".to_owned(),
            plugin_name: "Name".to_owned(),
            label: "Do it".to_owned(),
            handler: "do.js".to_owned(),
            icon: String::new(),
            added: false,
        });
        assert_eq!(funccode_to_name(code, &menus, &macros), "Do it");
        pc::set_commands(Vec::new());
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
    fn a_oneliner_is_named_by_its_entry_or_is_not_registered() {
        use matsu_config::settings::oneliner::{OneLiner, OutputTo};
        let (menus, macros) = (CustomMenus::default(), RegisteredMacros::default());
        let mut ones = OneLiners::default();
        let name = |ones: &OneLiners, code| funccode_to_name_with(code, &menus, &macros, ones);
        assert_eq!(name(&ones, 32711), "ワンライナー 0 (未登録)");
        assert_eq!(name(&ones, 32760), "ワンライナー 49 (未登録)");
        assert_eq!(
            funccode_to_name(32711, &menus, &macros),
            "ワンライナー 0 (未登録)"
        );
        ones.slots[0] = OneLiner {
            name: "並べ替え".to_owned(),
            command: "sort".to_owned(),
            output: OutputTo::Replace,
            ..OneLiner::default()
        };
        ones.slots[1].command = "jq .".to_owned();
        assert_eq!(name(&ones, 32711), "並べ替え");
        // No name: the command.
        assert_eq!(name(&ones, 32712), "jq .");
        // The three others are named by the labels of the table.
        assert_eq!(name(&ones, 32761), "登録済みワンライナーリスト");
        assert_eq!(name(&ones, 32762), "ワンライナーを入力して実行...");
        assert_eq!(name(&ones, 32763), "ワンライナーの設定...");
    }

    #[test]
    fn the_codes_of_the_oneliners_are_the_ones_of_the_table() {
        use matsu_config::settings::oneliner as o;
        let code = |name: &str| FUNCTIONS.iter().find(|f| f.name == name).unwrap().code;
        for n in 0..o::MAX_ONELINER {
            assert_eq!(
                code(&format!("F_ONELINER_{n}")),
                o::function_of_index(n).unwrap()
            );
        }
        assert_eq!(code("F_ONELINER_LIST"), o::F_ONELINER_LIST);
        assert_eq!(code("F_ONELINER_DIALOG"), o::F_ONELINER_DIALOG);
        assert_eq!(code("F_ONELINER_SETTINGS"), o::F_ONELINER_SETTINGS);
        assert_eq!(code("F_AI_LOG_SHOW"), o::F_AI_LOG_SHOW);
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
