//! 「ステータスバー」: `IDD_PROP_STATUSBAR` (`sakura_core/sakura_rc.rc`), `CPropStatusbar`
//! (`sakura_core/prop/CPropComStatusbar.cpp`). The keys are in `[Statusbar]`
//! (`CShareData_IO::ShareData_IO_Statusbar`).

use super::super::model::{Control, Item};

/// `[Statusbar]` of the settings file.
const SECTION: &str = "Statusbar";

fn check(id: &'static str, label: &'static str, key: &'static str, default: bool) -> Item {
    Control::check(id, label, key, default)
        .in_section(SECTION)
        .unavailable()
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
