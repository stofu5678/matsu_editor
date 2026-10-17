//! 「ファイル」: `IDD_PROP_FILE` (`sakura_core/sakura_rc.rc`), `CPropFile`
//! (`sakura_core/prop/CPropComFile.cpp`).
//!
//! Live: saving an unmodified document, the three options of the history (the caret, the
//! bookmarks and the question about the encoding of last time), and the drop options, which the
//! drop of files on a window reads from the settings. Not available yet: the lock of files and the
//! watching of changes, the automatic save, the warnings before a file is opened, the filter of the
//! save dialog and the decoding of MIME headers.

use super::super::Env;
use super::super::model::{Control, Item, Values};
use crate::file_ops::prefs::FilePrefs;
use crate::history::HistoryOptions;

/// `SHAREMODE_NOT_EXCLUSIVE`, `SHAREMODE_DENY_WRITE`, `SHAREMODE_DENY_READWRITE` (`nFileShareMode`),
/// with the names of `STR_EXCLU_*`.
const SHARE_MODES: &[(&str, i32)] = &[
    ("しない", 0),
    ("上書きを禁止する", 1),
    ("読み書きを禁止する", 2),
];

/// `BKUP_AUTO`: the bit of `bBackUpType2_Opt1` that turns the automatic save on
/// (`CommonSetting_Backup::IsAutoBackupEnabled`), and the default of the whole number
/// (`BKUP_YEAR | BKUP_MONTH | BKUP_DAY`).
const BKUP_AUTO: i32 = 64;
const BACKUP_OPT1_DEFAULT: i32 = 32 | 16 | 8;

/// `EnableFilePropInput`: the delay is edited only while the lock is off and the watching is on.
fn watching(values: &Values) -> bool {
    values.int("file_share_mode") == 0 && values.bool("check_file_time_stamp")
}

/// `EnableFilePropInput`: the watching is chosen only while the lock is off.
fn no_lock(values: &Values) -> bool {
    values.int("file_share_mode") == 0
}

/// `EnableFilePropInput`: the interval is edited only while the automatic save is on.
fn auto_save_is_on(values: &Values) -> bool {
    values.bool("auto_save")
}

/// `EnableFilePropInput`: the number of files is edited only while 「閉じて開く」 is off.
fn drop_opens_many(values: &Values) -> bool {
    !values.bool("drop_file_and_close")
}

/// `EnableFilePropInput`: the size is edited only while the warning is on.
fn large_file_is_warned(values: &Values) -> bool {
    values.bool("alert_if_large_file")
}

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![
        Item::Columns(vec![
            vec![Item::Group(
                "ファイルの排他制御",
                vec![
                    Control::choice(
                        "file_share_mode",
                        "排他制御(&H)",
                        "nFileShareMode",
                        0,
                        SHARE_MODES,
                    )
                    .unavailable()
                    .into(),
                    Item::Row(vec![
                        Control::check(
                            "check_file_time_stamp",
                            "更新の監視(&W)",
                            "bCheckFileTimeStamp",
                            true,
                        )
                        .unavailable()
                        .when(no_lock)
                        .into(),
                        Control::number(
                            "autoload_delay",
                            "自動読込時遅延(&A)",
                            "nAutoloadDelay",
                            0,
                            0,
                            i16::MAX as i32,
                        )
                        .unavailable()
                        .when(watching)
                        .into(),
                    ]),
                    Control::check(
                        "uneditable_if_unwritable",
                        "上書き禁止検出時は編集禁止にする(&N)",
                        "bUneditableIfUnwritable",
                        true,
                    )
                    .unavailable()
                    .into(),
                ],
            )],
            vec![Item::Group(
                "ファイルの保存",
                vec![
                    Control::check(
                        "enable_unmodified_overwrite",
                        "無変更でも上書きする(&S)",
                        "bEnableUnmodifiedOverwrite",
                        false,
                    )
                    .into(),
                    Control::flag(
                        "auto_save",
                        "自動的に保存する(&U)",
                        "bBackUpType2_Opt1",
                        BKUP_AUTO,
                        BACKUP_OPT1_DEFAULT,
                    )
                    .unavailable()
                    .into(),
                    Item::Row(vec![
                        Control::number(
                            "auto_save_interval",
                            "間隔は(&I)",
                            "bBackUpType2_Opt3",
                            5,
                            1,
                            35791,
                        )
                        .unavailable()
                        .when(auto_save_is_on)
                        .into(),
                        Item::Label("分ごと"),
                    ]),
                ],
            )],
        ]),
        Item::Group(
            "「名前を付けて保存」でファイルの種類が[ユーザー指定]のときのファイル一覧表示",
            vec![Item::Row(vec![
                Control::check(
                    "no_filter_save_new",
                    "新規から保存時は全ファイル表示(&O)",
                    "bNoFilterSaveNew",
                    true,
                )
                .unavailable()
                .into(),
                Control::check(
                    "no_filter_save_file",
                    "新規以外から保存時は全ファイル表示(&F)",
                    "bNoFilterSaveFile",
                    true,
                )
                .unavailable()
                .into(),
            ])],
        ),
        Item::Group(
            "ファイルオープン",
            vec![
                Item::Row(vec![
                    Control::check(
                        "drop_file_and_close",
                        "ファイルのドロップは「閉じて開く」にする(&L)",
                        "bDropFileAndClose",
                        false,
                    )
                    .into(),
                    Item::Label("←注：ドロップファイルは1つのみ有効"),
                ]),
                Item::Row(vec![
                    Control::number(
                        "drop_file_num_max",
                        "ファイルのドロップは一度に&MAX",
                        "nDropFileNumMax",
                        8,
                        1,
                        99,
                    )
                    .when(drop_opens_many)
                    .into(),
                    Item::Label("個までとする"),
                ]),
                Control::check(
                    "restore_cur_position",
                    "ファイルを開いたときにカーソル位置を復元する(&C)",
                    "m_bRestoreCurPosition",
                    true,
                )
                .into(),
                Control::check(
                    "restore_bookmarks",
                    "ファイルを開いたときにブックマークを復元する(&B)",
                    "m_bRestoreBookmarks",
                    true,
                )
                .into(),
                Control::check(
                    "auto_mime_decode",
                    "ファイルを開いたときにMIMEエンコードされたヘッダーをデコードする(&D)",
                    "bAutoMIMEdecode",
                    false,
                )
                .unavailable()
                .into(),
                Control::check(
                    "query_if_code_change",
                    "前回と異なる文字コードのとき問い合わせを行う(&Q)",
                    "bQueryIfCodeChange",
                    true,
                )
                .into(),
                Control::check(
                    "alert_if_file_not_exist",
                    "開こうとしたファイルが存在しないとき警告する(&E)",
                    "bAlertIfFileNotExist",
                    false,
                )
                .unavailable()
                .into(),
                Item::Row(vec![
                    Control::check(
                        "alert_if_large_file",
                        "開こうとしたファイルが大きい場合に警告する(&T)",
                        "bAlertIfLargeFile",
                        false,
                    )
                    .unavailable()
                    .into(),
                    Control::number("alert_file_size", "", "nAlertFileSize", 10, 1, 2048)
                        .unavailable()
                        .when(large_file_is_warned)
                        .into(),
                    Item::Label("MBから"),
                ]),
            ],
        ),
    ]
}

/// What OK does to the windows: the history's options, and the save of an unmodified document.
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, _before: Option<&Values>) {
    crate::history::set_options(
        env.cx,
        HistoryOptions {
            restore_cur_position: after.bool("restore_cur_position"),
            restore_bookmarks: after.bool("restore_bookmarks"),
            query_if_code_change: after.bool("query_if_code_change"),
        },
    );
    env.cx.set_global(FilePrefs {
        enable_unmodified_overwrite: after.bool("enable_unmodified_overwrite"),
    });
}
