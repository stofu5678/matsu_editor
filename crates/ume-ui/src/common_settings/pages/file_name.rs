//! 「ファイル名表示」: `IDD_PROP_FNAME` (`sakura_core/sakura_rc.rc`), `CPropFileName`
//! (`sakura_core/prop/CPropComFileName.cpp`).
//!
//! Not available yet, all of it: UME Editor shows the full path in the window title, the tabs and
//! the window list, and does not replace parts of it (`CFileNameManager::GetTransformFileNameFast`,
//! `sakura_core/env/CFileNameManager.cpp`). The long path is not shortened either. The page shows
//! the two settings of the file, which Sakura keeps in `[Nickname]` (`ShareData_IO_Nickname`). The
//! list of replacements (`From00`, `To00`, ...) needs a list control with its buttons, which the
//! dialog does not have yet; the file keeps it as it is.

use super::super::model::{Control, Item};

/// `[Nickname]` of the settings file.
const SECTION: &str = "Nickname";

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![
        Item::Row(vec![
            Control::check("short_path", "長いパスの省略表示(&L)", "bShortPath", true)
                .in_section(SECTION)
                .unavailable()
                .into(),
            Control::number(
                "short_path_max_width",
                "文字数(&N)",
                "nShortPathMaxWidth",
                100,
                0,
                9999,
            )
            .in_section(SECTION)
            .unavailable()
            .into(),
        ]),
        Item::Group(
            "ファイル名の簡易表示(&I)",
            vec![Item::Label(
                "置換前と置換後の一覧（先頭・上へ・下へ・最終・挿入・追加・更新・削除）は未対応です。\nファイルの [Nickname] の From00, To00 ... はそのまま残ります。",
            )],
        ),
    ]
}
