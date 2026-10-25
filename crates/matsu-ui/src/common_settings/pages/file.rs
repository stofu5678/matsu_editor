//! 「ファイル」: `IDD_PROP_FILE` (`sakura_core/sakura_rc.rc`), `CPropFile`
//! (`sakura_core/prop/CPropComFile.cpp`).
//!
//! Live: saving an unmodified document, the three options of the history (the caret, the
//! bookmarks and the question about the encoding of last time), the drop options, which the
//! drop of files on a window reads from the settings, the watching of changes, the files that
//! cannot be written, the automatic save and the two warnings before a file is opened
//! (`crate::file_watch`), the file types of the save dialog (`crate::file_dialog::save_types`), the
//! decoding of MIME headers of ISO-2022-JP files and the lock of the open files
//! (`crate::file_ops::lock`).

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

/// The question about hot exit is asked only while the copies are kept (ADR-0022).
fn unsaved_is_kept(values: &Values) -> bool {
    values.bool("keep_unsaved")
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
                    .into(),
                    Item::Row(vec![
                        Control::check(
                            "check_file_time_stamp",
                            "更新の監視(&W)",
                            "bCheckFileTimeStamp",
                            true,
                        )
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
                        .when(watching)
                        .into(),
                    ]),
                    Control::check(
                        "uneditable_if_unwritable",
                        "上書き禁止検出時は編集禁止にする(&N)",
                        "bUneditableIfUnwritable",
                        true,
                    )
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
                        .when(auto_save_is_on)
                        .into(),
                        Item::Label("分ごと"),
                    ]),
                    // Not Sakura's (ADR-0022): the section [MATSU] is MATSU Editor's own.
                    Control::check(
                        "keep_unsaved",
                        "保存していない文書を残し、次の起動で戻す(&K)",
                        crate::unsaved::prefs::KEY_KEEP,
                        true,
                    )
                    .in_section(crate::unsaved::prefs::SECTION)
                    .into(),
                    Control::check(
                        "keep_unsaved_no_ask",
                        "終了のとき保存の確認をしない(&Z)",
                        crate::unsaved::prefs::KEY_NO_ASK,
                        false,
                    )
                    .in_section(crate::unsaved::prefs::SECTION)
                    .when(unsaved_is_kept)
                    .into(),
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
                .into(),
                Control::check(
                    "no_filter_save_file",
                    "新規以外から保存時は全ファイル表示(&F)",
                    "bNoFilterSaveFile",
                    true,
                )
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
                .into(),
                Item::Row(vec![
                    Control::check(
                        "alert_if_large_file",
                        "開こうとしたファイルが大きい場合に警告する(&T)",
                        "bAlertIfLargeFile",
                        false,
                    )
                    .into(),
                    Control::number("alert_file_size", "", "nAlertFileSize", 10, 1, 2048)
                        .named("警告するファイルの大きさ（MB）")
                        .when(large_file_is_warned)
                        .into(),
                    Item::Label("MBから"),
                ]),
                // Not Sakura's (ADR-0020, chapter 7): the name of the key is a proposal.
                Item::Row(vec![
                    Control::number(
                        "huge_file_size_mb",
                        "",
                        "nHugeFileSizeMB",
                        crate::file_ops::prefs::HUGE_FILE_SIZE_MB_DEFAULT,
                        1,
                        1_000_000,
                    )
                    .named("範囲ごとに読むファイルの大きさ（MB）")
                    .into(),
                    Item::Label("MB以上のファイルは、範囲ごとに読んで開く(&H)"),
                ]),
            ],
        ),
    ]
}

/// What OK does to the windows: the history's options, and the save of an unmodified document.
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, before: Option<&Values>) {
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
        file_share_mode: after.int("file_share_mode"),
        check_file_time_stamp: after.bool("check_file_time_stamp"),
        autoload_delay: after.int("autoload_delay"),
        uneditable_if_unwritable: after.bool("uneditable_if_unwritable"),
        alert_if_file_not_exist: after.bool("alert_if_file_not_exist"),
        alert_if_large_file: after.bool("alert_if_large_file"),
        alert_file_size: after.int("alert_file_size"),
        auto_save: after.bool("auto_save"),
        auto_save_interval: after.int("auto_save_interval"),
        no_filter_save_new: after.bool("no_filter_save_new"),
        no_filter_save_file: after.bool("no_filter_save_file"),
        auto_mime_decode: after.bool("auto_mime_decode"),
        huge_file_size_mb: after.int("huge_file_size_mb"),
        huge_scan: crate::file_ops::prefs::prefs(env.cx).huge_scan,
    });
    // The copies of unsaved documents (ADR-0022). The size limit is not in the dialog: it stays as it was.
    let old = crate::unsaved::prefs::prefs(env.cx);
    env.cx.set_global(crate::unsaved::prefs::KeepPrefs {
        keep: after.bool("keep_unsaved"),
        no_ask: after.bool("keep_unsaved_no_ask"),
        max_mb: old.max_mb,
    });
    if before.is_some_and(|b| b.bool("keep_unsaved")) && !after.bool("keep_unsaved") {
        crate::unsaved::ask_to_clear(env.cx);
    }
    // The open documents take the new mode (`CEditDoc::OnChangeSetting`).
    crate::file_ops::lock::apply_setting(env.cx, after.int("file_share_mode"));
}
