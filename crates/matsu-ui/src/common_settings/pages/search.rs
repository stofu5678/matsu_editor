//! 「検索」: `IDD_PROP_GREP` (`sakura_core/sakura_rc.rc`), `CPropGrep`
//! (`sakura_core/prop/CPropComGrep.cpp`).
//!
//! Live: the word at the caret as the first search string, the question about saving when a Grep
//! window is closed, and the two modes of the tags file search (`crate::tags::TagJumpPrefs`). Not
//! available yet: taking the search condition of another view, the choice of the regular
//! expression library (MATSU Editor has its own, `matsu-regex`), the display of the result while
//! Grep runs, and the tag jump by the Enter key and the double click in the result of Grep.

use super::super::Env;
use super::super::model::{Control, Item, Values};
use crate::search::prefs::SearchPrefs;

/// `STR_TAGJUMP_0` .. `STR_TAGJUMP_3` (`nTagJumpMode`, `nTagJumpModeKeyword`). The first box has
/// no 2, the second has all four (`CPropGrep::SetData`).
const TAG_MODES: &[(&str, i32)] = &[
    ("次のファイルを検索しない", 0),
    ("ヒットしたら次のファイルを検索しない", 1),
    ("次のファイルを検索する", 3),
];
const TAG_MODES_KEYWORD: &[(&str, i32)] = &[
    ("次のファイルを検索しない", 0),
    ("ヒットしたら次のファイルを検索しない", 1),
    ("完全一致でヒットしたら次のファイルを検索しない", 2),
    ("次のファイルを検索する", 3),
];

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![
        Item::Group(
            "検索／置換／Grep",
            vec![
                Control::check(
                    "caret_text_for_search",
                    "カーソル位置の文字列をデフォルトの検索文字列にする(&C)",
                    "bCaretTextForSearch",
                    true,
                )
                .into(),
                Control::check(
                    "inherit_key_other_view",
                    "次・前検索で他のビューでの検索条件変更を引き継ぐ(&I)",
                    "m_bInheritKeyOtherView",
                    true,
                )
                .unavailable()
                .into(),
                Control::text(
                    "regexp_lib",
                    "正規表現ライブラリ指定(&L)",
                    "szRegexpLib",
                    "",
                )
                .unavailable()
                .into(),
            ],
        ),
        Item::Group(
            "Grep",
            vec![
                Control::check(
                    "grep_exit_confirm",
                    "Grepモードで保存確認する(&V)",
                    "bGrepExitConfirm",
                    false,
                )
                .into(),
                Control::check(
                    "grep_real_time",
                    "リアルタイムで表示する(&R)",
                    "bGrepRealTime",
                    false,
                )
                .unavailable()
                .into(),
            ],
        ),
        Item::Group(
            "Grep結果からのタグジャンプ",
            vec![
                Control::check(
                    "gtjw_return",
                    "エンターキーでタグジャンプ(&E)",
                    "bGTJW_RETURN",
                    true,
                )
                .unavailable()
                .into(),
                Control::check(
                    "gtjw_ldblclk",
                    "ダブルクリックでタグジャンプ(&W)",
                    "bGTJW_LDBLCLK",
                    true,
                )
                .unavailable()
                .into(),
            ],
        ),
        Item::Group(
            "タグジャンプ",
            vec![
                Control::choice(
                    "tag_jump_mode",
                    "タグファイルの検索(&T):",
                    "nTagJumpMode",
                    1,
                    TAG_MODES,
                )
                .into(),
                Control::choice(
                    "tag_jump_mode_keyword",
                    "キーワード指定のタグファイル検索(&K):",
                    "nTagJumpModeKeyword",
                    3,
                    TAG_MODES_KEYWORD,
                )
                .into(),
            ],
        ),
    ]
}

/// What OK does to the windows: the options of the search dialogs and of the Grep window, and the
/// modes of the tags file search.
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, _before: Option<&Values>) {
    env.cx.set_global(SearchPrefs {
        caret_text_for_search: after.bool("caret_text_for_search"),
        grep_exit_confirm: after.bool("grep_exit_confirm"),
    });
    let mode = |id: &str| u8::try_from(after.int(id)).unwrap_or(0);
    let (direct, keyword) = (mode("tag_jump_mode"), mode("tag_jump_mode_keyword"));
    crate::tags::remember_prefs(env.cx, |prefs| {
        prefs.direct_mode = direct;
        prefs.keyword_mode = keyword;
    });
}
