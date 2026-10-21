//! 「ファイル名表示」: `IDD_PROP_FNAME` (`sakura_core/sakura_rc.rc`), `CPropFileName`
//! (`sakura_core/prop/CPropComFileName.cpp`).
//!
//! The check box and the number of the short form of a long path (`bShortPath`,
//! `nShortPathMaxWidth`) and the list of replacements (`From00`, `To00` ... and their count
//! `ArrNum`, at most 16) are in `[Nickname]` (`ShareData_IO_Nickname`). The list is an editable
//! list ([`crate::common_settings::table`]); OK writes it and the windows show their paths with it
//! ([`crate::file_names`]).

use super::super::Env;
use super::super::model::{Control, Item, Values};
use super::super::table::{Column, Op, Storage, TableSpec};
use crate::file_names::{FileNames, Folders};

/// `[Nickname]` of the settings file.
const SECTION: &str = "Nickname";

/// `MAX_TRANSFORM_FILENAME` (`sakura_core/config/maxdata.h`).
const MAX_ROWS: usize = 16;

/// The list: `STR_PROPCOMFNM_LIST1` and `LIST2` for the headings, the edit boxes
/// `置換前(&F)` and `置換後(&R)` of `IDD_PROP_FNAME`. A column takes 60 and 35 percent of the
/// list (`CPropFileName::DispatchEvent`, `WM_INITDIALOG`); the edit boxes take `_MAX_PATH - 1`.
static LIST: TableSpec = TableSpec {
    columns: &[
        Column {
            title: "置換前",
            edit_label: "置換前(&F)",
            key: "From",
            width: 60,
            max_chars: 259,
        },
        Column {
            title: "置換後",
            edit_label: "置換後(&R)",
            key: "To",
            width: 35,
            max_chars: 259,
        },
    ],
    max_rows: MAX_ROWS,
    ops: &Op::ALL,
    // `CShareData::InitShareData`, 「ファイル名表示」.
    defaults: &[
        &["%DeskTop%\\", "デスクトップ\\"],
        &["%Personal%\\", "マイドキュメント\\"],
        &["%Cache%\\Content.IE5\\", "IEキャッシュ\\"],
        &["%TEMP%\\", "TEMP\\"],
        &["%Common DeskTop%\\", "共有デスクトップ\\"],
        &["%Common Documents%\\", "共有ドキュメント\\"],
        &["%AppData%\\", "アプリデータ\\"],
    ],
    storage: Storage::Numbered {
        section: SECTION,
        count_key: "ArrNum",
        digits: 2,
    },
    // `STR_PROPCOMFNM_ERR_REG`
    full_message: "これ以上登録できません。",
    prepare: None,
};

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![
        Item::Row(vec![
            Control::check("short_path", "長いパスの省略表示(&L)", "bShortPath", true)
                .in_section(SECTION)
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
            .into(),
        ]),
        Item::Group(
            "ファイル名の簡易表示(&I)",
            vec![
                Control::table("file_name_list", "", &LIST)
                    .named("ファイル名の簡易表示")
                    .into(),
            ],
        ),
    ]
}

/// What OK does to the windows: they show their paths with the new list.
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, _before: Option<&Values>) {
    let names = FileNames::new(
        after.bool("short_path"),
        usize::try_from(after.int("short_path_max_width")).unwrap_or(0),
        after.rows("file_name_list"),
        &Folders::new(env.cx),
        !cfg!(windows),
    );
    env.cx.set_global(names);
}
