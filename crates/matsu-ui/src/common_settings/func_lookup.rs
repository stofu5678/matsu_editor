//! Sakura's lookup of the functions by kind (`CFuncLookup`, `sakura_core/func/CFuncLookup.cpp`): the
//! kinds (種別) that the pages offer in a combo box, and the functions of each kind in the order of
//! the list. The pages that pick a function use it: 「ツールバー」, 「キー割り当て」 and
//! 「カスタムメニュー」.

use matsu_commands::sakura::FUNCTION_LISTS;

/// `F_CUSTMENU_BASE`-based codes of the kind カスタムメニュー (`CFuncLookup::Pos2FuncCode`):
/// `F_MENU_RBUTTON`, then `F_CUSTMENU_1` to `F_CUSTMENU_24`.
const CUSTOM_MENU_CODES: std::ops::RangeInclusive<u16> = 31501..=31524;
const RIGHT_CLICK_MENU: u16 = 31580;
/// `F_USERMACRO_0` + n of the kind 外部マクロ (`CFuncLookup::Pos2FuncCode` with `bGetUnavailable`, which
/// the pages use: a macro that is not registered is listed too).
const USER_MACRO_CODES: std::ops::Range<u16> = {
    use matsu_config::settings::macros::{F_USERMACRO_0, MAX_CUSTMACRO};
    F_USERMACRO_0..F_USERMACRO_0 + MAX_CUSTMACRO as u16
};
/// `F_ONELINER_0` + n, then the input dialog and the settings, of the kind ワンライナー (MATSU's own kind,
/// ADR-0029). `F_ONELINER_LIST` is a special item of the menus, not listed here. Like the macros, one that
/// is not registered is listed too.
const ONELINER_CODES: [std::ops::Range<u16>; 2] = {
    use matsu_config::settings::oneliner::{F_ONELINER_0, F_ONELINER_DIALOG, MAX_ONELINER};
    [
        F_ONELINER_0..F_ONELINER_0 + MAX_ONELINER as u16,
        F_ONELINER_DIALOG..F_ONELINER_DIALOG + 3,
    ]
};

/// The kinds (`CFuncLookup::SetCategory2Combo`): the 15 groups of the command list, then the
/// ones Sakura adds (外部マクロ, カスタムメニュー, プラグイン) and MATSU's own (ワンライナー, last so that
/// the numbers of the others stay).
pub(in crate::common_settings) static KINDS: [(&str, i32); 19] = [
    ("ファイル操作系", 0),
    ("編集系", 1),
    ("カーソル移動系", 2),
    ("選択系", 3),
    ("矩形選択系", 4),
    ("クリップボード系", 5),
    ("挿入系", 6),
    ("変換系", 7),
    ("検索系", 8),
    ("モード切り替え系", 9),
    ("設定系", 10),
    ("マクロ系", 11),
    ("ウィンドウ系", 12),
    ("支援", 13),
    ("その他", 14),
    ("外部マクロ", 15),
    ("カスタムメニュー", 16),
    ("プラグイン", 17),
    // 18 is 特別機能 on the pages of the menus (`STR_SPECIAL_FUNC`), so the kind of the one-liners is 19.
    ("ワンライナー", 19),
];

/// The function codes of kind `kind` in the order of the list (`CFuncLookup::Pos2FuncCode`).
pub(in crate::common_settings) fn codes_of_kind(kind: i32) -> Vec<u16> {
    let Ok(kind) = usize::try_from(kind) else {
        return Vec::new();
    };
    if let Some(codes) = FUNCTION_LISTS.get(kind) {
        return codes.iter().copied().filter(|&c| c != 0).collect();
    }
    match kind - FUNCTION_LISTS.len() {
        0 => USER_MACRO_CODES.collect(),
        1 => std::iter::once(RIGHT_CLICK_MENU)
            .chain(CUSTOM_MENU_CODES)
            .collect(),
        // The commands of the plugins that are enabled (`CJackManager::GetCommandCode`).
        2 => matsu_commands::plugin_commands::commands()
            .iter()
            .map(|c| c.code)
            .collect(),
        // 3 is the kind 特別機能 of the menu pages, which they list themselves.
        4 => ONELINER_CODES.into_iter().flatten().collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matsu_commands::sakura::FUNCTION_GROUPS;

    #[test]
    fn the_kinds_are_the_groups_of_the_command_list_and_the_three_added_ones() {
        let names: Vec<&str> = KINDS.iter().map(|(n, _)| *n).collect();
        assert_eq!(names[..15], *FUNCTION_GROUPS);
        assert_eq!(
            names[15..],
            [
                "外部マクロ",
                "カスタムメニュー",
                "プラグイン",
                "ワンライナー"
            ]
        );
        // The values are the positions, but the one-liners' (19): 18 is 特別機能 on the menu pages.
        for (i, (_, v)) in KINDS[..18].iter().enumerate() {
            assert_eq!(*v, i as i32);
        }
        assert_eq!(KINDS[18].1, 19);
    }

    #[test]
    fn the_kind_custom_menu_lists_the_right_click_menu_then_the_24_menus() {
        let codes = codes_of_kind(16);
        assert_eq!(codes.len(), 25);
        assert_eq!(codes[0], RIGHT_CLICK_MENU);
        assert_eq!(codes[1..], CUSTOM_MENU_CODES.collect::<Vec<_>>()[..]);
        assert!(codes_of_kind(17).is_empty() || codes_of_kind(17).iter().all(|&c| c >= 20000));
        assert!(codes_of_kind(-1).is_empty() && codes_of_kind(20).is_empty());
    }

    #[test]
    fn the_kind_oneliner_lists_the_50_one_liners_the_dialog_and_the_settings() {
        use matsu_config::settings::oneliner::{
            F_AI_LOG_SHOW, F_ONELINER_DIALOG, F_ONELINER_SETTINGS,
        };
        let codes = codes_of_kind(19);
        assert_eq!(codes.len(), 53);
        assert!(codes_of_kind(18).is_empty());
        assert_eq!((codes[0], codes[49]), (32711, 32760));
        assert!(codes[..50].windows(2).all(|w| w[1] == w[0] + 1));
        assert_eq!(
            codes[50..],
            [F_ONELINER_DIALOG, F_ONELINER_SETTINGS, F_AI_LOG_SHOW]
        );
    }

    #[test]
    fn the_kind_external_macro_lists_the_50_registered_macros() {
        let codes = codes_of_kind(15);
        assert_eq!(codes.len(), 50);
        assert_eq!((codes[0], codes[49]), (31600, 31649));
        assert!(codes.windows(2).all(|w| w[1] == w[0] + 1));
    }
}
