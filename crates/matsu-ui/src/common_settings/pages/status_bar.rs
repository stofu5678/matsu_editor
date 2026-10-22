//! 「ステータスバー」: `IDD_PROP_STATUSBAR` (`sakura_core/sakura_rc.rc`), `CPropStatusbar`
//! (`sakura_core/prop/CPropComStatusbar.cpp`). The keys are in `[Statusbar]`
//! (`CShareData_IO::ShareData_IO_Statusbar`).
//!
//! Live, all of it: OK makes every window's status bar show the character code, the selection and
//! the column as the boxes say (`crate::status_options`).

use matsu_core::encoding::CharCodeOptions;

use super::super::Env;
use super::super::model::{Control, Item, Values};
use crate::status_options::StatusbarOptions;

/// `[Statusbar]` of the settings file.
const SECTION: &str = "Statusbar";

fn check(id: &'static str, label: &'static str, key: &'static str, default: bool) -> Item {
    Control::check(id, label, key, default)
        .in_section(SECTION)
        .into()
}

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![
        Item::Group(
            "表示文字コードの指定",
            vec![
                check(
                    "disp_unicode_in_sjis",
                    "SJIS, Latin1等で文字コード値をUnicodeで表示する(&S)",
                    "DispUnicodeInSjis",
                    false,
                ),
                check(
                    "disp_unicode_in_jis",
                    "JISで文字コード値をUnicodeで表示する(&J)",
                    "DispUnicodeInJis",
                    false,
                ),
                check(
                    "disp_unicode_in_euc",
                    "EUCで文字コード値をUnicodeで表示する(&E)",
                    "DispUnicodeInEuc",
                    false,
                ),
                check(
                    "disp_utf8_codepoint",
                    "UTF-8およびCESU-8をコードポイントで表示する(&8)",
                    "DispUtf8Codepoint",
                    true,
                ),
                check(
                    "disp_sp_codepoint",
                    "サロゲートペアをコードポイントで表示する(&P)",
                    "DispSurrogatePairCodepoint",
                    true,
                ),
            ],
        ),
        Item::Group(
            "表示文字数の指定",
            vec![
                check(
                    "disp_sel_count_by_byte",
                    "選択文字数を文字単位ではなくバイト単位で表示する(&B)　※高負荷のため非推奨",
                    "DispSelectCountByByte",
                    false,
                ),
                check(
                    "disp_col_by_char",
                    "現在桁をルーラー単位ではなく文字単位で表示する(&C)",
                    "DispColByChar",
                    false,
                ),
            ],
        ),
    ]
}

/// What the windows' status bars use now.
fn options(values: &Values) -> StatusbarOptions {
    StatusbarOptions {
        char_code: CharCodeOptions {
            unicode_in_sjis: values.bool("disp_unicode_in_sjis"),
            unicode_in_jis: values.bool("disp_unicode_in_jis"),
            unicode_in_euc: values.bool("disp_unicode_in_euc"),
            utf8_codepoint: values.bool("disp_utf8_codepoint"),
            surrogate_codepoint: values.bool("disp_sp_codepoint"),
        },
        select_count_by_byte: values.bool("disp_sel_count_by_byte"),
        column_by_char: values.bool("disp_col_by_char"),
    }
}

/// What OK does to the windows: the options are taken and every window draws its status bar again.
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, _before: Option<&Values>) {
    env.cx.set_global(options(after));
    for (_, workspace) in crate::file_ops::windows::workspaces(env.cx) {
        workspace.update(env.cx, |_, cx| cx.notify());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_settings::model::{Values, controls};

    fn values(profile: &matsu_config::Profile) -> Values {
        Values::load(&controls(&items()), profile)
    }

    #[test]
    fn the_defaults_are_sakuras() {
        let v = values(&matsu_config::Profile::default());
        assert_eq!(options(&v), StatusbarOptions::default());
    }

    #[test]
    fn every_box_reaches_the_options() {
        let mut profile = matsu_config::Profile::default();
        for (key, on) in [
            ("DispUnicodeInSjis", "1"),
            ("DispUnicodeInJis", "1"),
            ("DispUnicodeInEuc", "1"),
            ("DispUtf8Codepoint", "0"),
            ("DispSurrogatePairCodepoint", "0"),
            ("DispSelectCountByByte", "1"),
            ("DispColByChar", "1"),
        ] {
            profile.set(SECTION, key, on).unwrap();
        }
        let o = options(&values(&profile));
        assert!(o.char_code.unicode_in_sjis);
        assert!(o.char_code.unicode_in_jis);
        assert!(o.char_code.unicode_in_euc);
        assert!(!o.char_code.utf8_codepoint);
        assert!(!o.char_code.surrogate_codepoint);
        assert!(o.select_count_by_byte);
        assert!(o.column_by_char);
    }
}
