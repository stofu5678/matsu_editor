//! 「バックアップ」: `IDD_PROP_BACKUP` (`sakura_core/sakura_rc.rc`), `CPropBackup`
//! (`sakura_core/prop/CPropComBackup.cpp`).
//!
//! Not available yet, all of it: UME Editor makes no backup when it saves (`CBackupAgent`,
//! `sakura_core/doc/CBackupAgent.cpp`). The page shows what the file holds. Sakura packs the
//! options into the numbers `bBackUpType2_Opt1` (the bits of the date and time, and the bit of the
//! automatic save), `bBackUpType2_Opt2` (the number of generations in the low 16 bits and the
//! first letter of the extension in the next 8) and `bBackUpType2_Opt3` and `_Opt4`
//! (`CommonSetting_Backup`, `sakura_core/env/CommonSetting.h`): the controls hold their bits
//! (`Control::packed`). The file name type 5 and 6 of Sakura (the original extension is kept) are
//! shown as 「拡張子を .bakに変更したもの」 and 「拡張子を連番」 with the check 「元の拡張子を保存」
//! off, because that check has no key of its own.

use gpui::{Context, Window};

use super::super::dialog::CommonSettingsDialog;
use super::super::model::{Control, Item};

/// `BKUP_YEAR` .. `BKUP_SEC` (`EBackupOptionFlag`).
const YEAR: i32 = 32;
const MONTH: i32 = 16;
const DAY: i32 = 8;
const HOUR: i32 = 4;
const MIN: i32 = 2;
const SEC: i32 = 1;

/// `bBackUpType2_Opt1` as `CShareData::InitShareData` has it: `BKUP_YEAR | BKUP_MONTH | BKUP_DAY`.
const OPT1_DEFAULT: i32 = YEAR | MONTH | DAY;
/// `bBackUpType2_Opt2`: `('b' << 16) + 10`.
const OPT2_DEFAULT: i32 = ((b'b' as i32) << 16) + 10;

const OPT1: &str = "bBackUpType2_Opt1";

/// `IDC_CHECK_BACKUP_YEAR` and the like.
fn date_flag(id: &'static str, label: &'static str, mask: i32) -> Item {
    Control::flag(id, label, OPT1, mask, OPT1_DEFAULT)
        .unavailable()
        .into()
}

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![Item::Group(
        "バックアップの作成",
        vec![
            Control::check(
                "backup",
                "保存時にバックアップを作成する(&K)",
                "bBackUp",
                false,
            )
            .unavailable()
            .into(),
            Control::check(
                "backup_advanced",
                "詳細設定する(&A)",
                "bBackUpPathAdvanced",
                false,
            )
            .unavailable()
            .into(),
            Control::text("backup_file", "", "szBackUpPathAdvanced", "")
                .unavailable()
                .into(),
            Control::radio(
                "backup_type",
                "拡張子を .bakに変更したもの(1世代 &B)",
                "nBackUpType",
                1,
                2,
            )
            .unavailable()
            .into(),
            Control::radio(
                "backup_type",
                "拡張子を連番(.b00～b98)に変更したもの(&G)",
                "nBackUpType",
                3,
                2,
            )
            .unavailable()
            .into(),
            Control::check("backup_retain_ext", "元の拡張子を保存(&R)", "", false)
                .unavailable()
                .into(),
            Control::number(
                "backup_generations",
                "保存する世代数(1-99 &N)",
                "bBackUpType2_Opt2",
                10,
                1,
                99,
            )
            .packed(0xffff, OPT2_DEFAULT)
            .unavailable()
            .into(),
            Control::radio(
                "backup_type",
                "ファイル名の後ろに保存時の日付・時刻を付加したもの(&E)",
                "nBackUpType",
                2,
                2,
            )
            .unavailable()
            .into(),
            Control::radio(
                "backup_type",
                "ファイル名の後ろに前回の保存時の日付・時刻を付加したもの(&U)",
                "nBackUpType",
                4,
                2,
            )
            .unavailable()
            .into(),
            Item::Row(vec![
                date_flag("backup_year", "西暦(&Y)", YEAR),
                date_flag("backup_month", "月(&M)", MONTH),
                date_flag("backup_day", "日(&D)", DAY),
                date_flag("backup_hour", "時(&H)", HOUR),
                date_flag("backup_min", "分(&I)", MIN),
                date_flag("backup_sec", "秒(&S)", SEC),
            ]),
            Item::Label(
                "詳細設定\n$x : x個上のフォルダー名(0-9)\n* : 拡張子\n\n%Y/%m/%d : 年月日 %y西暦(2桁)\n%H:%M:%S: 時分秒\n%% : '%'記号",
            ),
            Control::radio(
                "backup_advanced_type",
                "保存時の日付・時刻を使用",
                "bBackUpType2_Opt4",
                2,
                0,
            )
            .unavailable()
            .into(),
            Control::radio(
                "backup_advanced_type",
                "前回のファイル更新時の日付・時刻を使用",
                "bBackUpType2_Opt4",
                4,
                0,
            )
            .unavailable()
            .into(),
            Item::Row(vec![
                Control::check(
                    "backup_folder_on",
                    "指定フォルダーに作成する(&P)",
                    "bBackUpFolder",
                    false,
                )
                .unavailable()
                .into(),
                Control::check(
                    "backup_folder_removable",
                    "リムーバブルメディアのみ(&L)",
                    "bBackUpFolderRM",
                    false,
                )
                .unavailable()
                .into(),
            ]),
            Item::Row(vec![
                Control::text("backup_folder", "フォルダー名(&F)", "szBackUpFolder", "")
                    .unavailable()
                    .into(),
                Control::button("backup_folder_browse", "&...", browse)
                    .unavailable()
                    .into(),
            ]),
            Control::check(
                "backup_dust_box",
                "バックアップファイルをごみ箱に放り込む(&X)",
                "bBackUpDustBox",
                false,
            )
            .unavailable()
            .into(),
            Control::check(
                "backup_dialog",
                "作成前に確認する(&V)",
                "bBackUpDialog",
                true,
            )
            .unavailable()
            .into(),
        ],
    )]
}

/// `IDC_BUTTON_BACKUP_FOLDER_REF`: chooses the folder of the backups.
fn browse(
    dialog: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    dialog.choose_folder(
        "backup_folder",
        "バックアップを作成するフォルダーの選択",
        window,
        cx,
    );
}
