//! 「バックアップ」: `IDD_PROP_BACKUP` (`sakura_core/sakura_rc.rc`), `CPropBackup`
//! (`sakura_core/prop/CPropComBackup.cpp`).
//!
//! The backup is made by `crate::file_ops::backup` (`CBackupAgent`). Sakura packs the options
//! into the numbers `bBackUpType2_Opt1` (the bits of the date and time, and the bit of the
//! automatic save), `bBackUpType2_Opt2` (the number of generations in the low 16 bits and the
//! first letter of the extension in the next 8) and `bBackUpType2_Opt3` and `_Opt4`
//! (`CommonSetting_Backup`, `sakura_core/env/CommonSetting.h`): the controls hold their bits
//! (`Control::packed`).
//!
//! The file name type 5 and 6 of Sakura (the original extension is kept) are the radio buttons 1
//! and 3 with the check 「元の拡張子を保存」, which has no key of its own: [`after_load`] and
//! [`before_store`] turn one into the other (`SetData` and `GetData`).
//!
//! Not done: the box 「詳細設定」 does not follow the other controls while the dialog is open
//! (`UpdateBackupFile`); it is made when the dialog opens and when OK writes the file. The
//! automatic save of Sakura (`BKUP_AUTO`, `bBackUpType2_Opt3`) is not on the page.

use gpui::{Context, Window};

use super::super::Env;
use super::super::dialog::CommonSettingsDialog;
use super::super::model::{Control, Item, Value, Values};
use crate::file_ops::backup::{BackupPrefs, DAY, HOUR, MIN, MONTH, SEC, YEAR};

/// The id of the check box 「元の拡張子を保存」.
pub(in crate::common_settings) const RETAIN_EXT: &str = "backup_retain_ext";

/// `bBackUpType2_Opt1` as `CShareData::InitShareData` has it: `BKUP_YEAR | BKUP_MONTH | BKUP_DAY`.
const OPT1_DEFAULT: i32 = YEAR | MONTH | DAY;
/// `bBackUpType2_Opt2`: `('b' << 16) + 10`.
const OPT2_DEFAULT: i32 = ((b'b' as i32) << 16) + 10;

const OPT1: &str = "bBackUpType2_Opt1";
const OPT2: &str = "bBackUpType2_Opt2";

/// `EnableBackupInput`: the controls of the page can be used only with the backup on.
fn on(values: &Values) -> bool {
    values.bool("backup")
}

/// The controls of the simple names are used while the detailed setting is off.
fn simple(values: &Values) -> bool {
    on(values) && !values.bool("backup_advanced")
}

fn type_is(values: &Values, kinds: &[i32]) -> bool {
    simple(values) && kinds.contains(&values.int("backup_type"))
}

fn generations_enabled(values: &Values) -> bool {
    type_is(values, &[3])
}

fn retain_enabled(values: &Values) -> bool {
    type_is(values, &[1, 3])
}

fn date_enabled(values: &Values) -> bool {
    type_is(values, &[2, 4])
}

fn advanced_enabled(values: &Values) -> bool {
    on(values) && values.bool("backup_advanced")
}

fn folder_enabled(values: &Values) -> bool {
    on(values) && values.bool("backup_folder_on")
}

/// `IDC_CHECK_BACKUP_YEAR` and the like.
fn date_flag(id: &'static str, label: &'static str, mask: i32) -> Item {
    Control::flag(id, label, OPT1, mask, OPT1_DEFAULT)
        .when(date_enabled)
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
            .into(),
            Control::check(
                "backup_advanced",
                "詳細設定する(&A)",
                "bBackUpPathAdvanced",
                false,
            )
            .when(on)
            .into(),
            Control::text("backup_file", "", "szBackUpPathAdvanced", "")
                .named("詳細設定の書式")
                .when(advanced_enabled)
                .into(),
            Control::radio(
                "backup_type",
                "拡張子を .bakに変更したもの(1世代 &B)",
                "nBackUpType",
                1,
                2,
            )
            .when(simple)
            .into(),
            Control::radio(
                "backup_type",
                "拡張子を連番(.b00～b98)に変更したもの(&G)",
                "nBackUpType",
                3,
                2,
            )
            .when(simple)
            .into(),
            Control::check(RETAIN_EXT, "元の拡張子を保存(&R)", "", false)
                .when(retain_enabled)
                .into(),
            Control::number(
                "backup_generations",
                "保存する世代数(1-99 &N)",
                OPT2,
                10,
                1,
                99,
            )
            .packed(0xffff, OPT2_DEFAULT)
            .when(generations_enabled)
            .into(),
            Control::radio(
                "backup_type",
                "ファイル名の後ろに保存時の日付・時刻を付加したもの(&E)",
                "nBackUpType",
                2,
                2,
            )
            .when(simple)
            .into(),
            Control::radio(
                "backup_type",
                "ファイル名の後ろに前回の保存時の日付・時刻を付加したもの(&U)",
                "nBackUpType",
                4,
                2,
            )
            .when(simple)
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
            .when(advanced_enabled)
            .into(),
            Control::radio(
                "backup_advanced_type",
                "前回のファイル更新時の日付・時刻を使用",
                "bBackUpType2_Opt4",
                4,
                0,
            )
            .when(advanced_enabled)
            .into(),
            Item::Row(vec![
                Control::check(
                    "backup_folder_on",
                    "指定フォルダーに作成する(&P)",
                    "bBackUpFolder",
                    false,
                )
                .when(on)
                .into(),
                Control::check(
                    "backup_folder_removable",
                    "リムーバブルメディアのみ(&L)",
                    "bBackUpFolderRM",
                    false,
                )
                .when(folder_enabled)
                .into(),
            ]),
            Item::Row(vec![
                Control::text("backup_folder", "フォルダー名(&F)", "szBackUpFolder", "")
                    .when(folder_enabled)
                    .into(),
                Control::button("backup_folder_browse", "&...", browse)
                    .named("バックアップフォルダーを参照")
                    .when(folder_enabled)
                    .into(),
            ]),
            Control::check(
                "backup_dust_box",
                "バックアップファイルをごみ箱に放り込む(&X)",
                "bBackUpDustBox",
                false,
            )
            .when(on)
            .into(),
            Control::check(
                "backup_dialog",
                "作成前に確認する(&V)",
                "bBackUpDialog",
                true,
            )
            .when(on)
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

/// `nBackUpType` as Sakura stores it: 5 and 6 are 1 and 3 with the extension kept.
fn stored_kind(values: &Values) -> i32 {
    match (values.int("backup_type"), values.bool(RETAIN_EXT)) {
        (1, true) => 5,
        (3, true) => 6,
        (kind, _) => kind,
    }
}

/// `UpdateBackupFile`: the name that the settings make, in the notation of the detailed setting.
fn preview(values: &Values) -> String {
    if !values.bool("backup") {
        return String::new();
    }
    let mut text = if values.bool("backup_folder_on") {
        String::new()
    } else if values.bool("backup_dust_box") {
        // `STR_PROPCOMBK_DUSTBOX`
        "(ゴミ箱)\\".to_owned()
    } else {
        ".\\".to_owned()
    };
    match stored_kind(values) {
        1 => text.push_str("$0.bak"),
        5 => text.push_str("$0.*.bak"),
        3 => text.push_str("$0.b??"),
        6 => text.push_str("$0.*.b??"),
        2 | 4 => {
            text.push_str("$0_");
            for (id, spec) in [
                ("backup_year", "%Y"),
                ("backup_month", "%m"),
                ("backup_day", "%d"),
                ("backup_hour", "%H"),
                ("backup_min", "%M"),
                ("backup_sec", "%S"),
            ] {
                if values.bool(id) {
                    text.push_str(spec);
                }
            }
            text.push_str(".*");
        }
        _ => {}
    }
    text
}

/// `SetData`: the kinds 5 and 6 become the radio button and the check box, a kind or a time that
/// no button has is the first one, and the box of the detailed setting shows the name that the
/// other settings make (while 「詳細設定する」 is off).
pub(in crate::common_settings) fn after_load(values: &mut Values) {
    let (kind, retain) = match values.int("backup_type") {
        5 => (1, true),
        6 => (3, true),
        kind @ 1..=4 => (kind, false),
        _ => (1, false),
    };
    values.set("backup_type", Value::Int(kind));
    values.set(RETAIN_EXT, Value::Bool(retain));
    if values.int("backup_advanced_type") != 4 {
        values.set("backup_advanced_type", Value::Int(2));
    }
    if !values.bool("backup_advanced") {
        values.set("backup_file", Value::Text(preview(values)));
    }
}

/// `GetData`: the radio button and the check box become the kind 5 or 6; the box of the detailed
/// setting is made from the other settings while 「詳細設定する」 is off.
pub(in crate::common_settings) fn before_store(values: &mut Values) {
    if !values.bool("backup_advanced") {
        values.set("backup_file", Value::Text(preview(values)));
    }
    let kind = stored_kind(values);
    values.set("backup_type", Value::Int(kind));
}

/// What OK does to the windows: the backup of the next save.
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, _before: Option<&Values>) {
    // The first letter of the extension is in the number of the generations, which the page only
    // shows the low bits of: read it from the file. At startup the file is not a global yet;
    // `install` then sets the letter ([`set_ext_char`]).
    let ext_char = env
        .cx
        .try_global::<crate::settings::UserSettings>()
        .and_then(|user| user.profile())
        .map_or('b', ext_char_of);
    let mut date_bits = 0;
    for (id, flag) in [
        ("backup_year", YEAR),
        ("backup_month", MONTH),
        ("backup_day", DAY),
        ("backup_hour", HOUR),
        ("backup_min", MIN),
        ("backup_sec", SEC),
    ] {
        if after.bool(id) {
            date_bits |= flag;
        }
    }
    env.cx.set_global(BackupPrefs {
        enabled: after.bool("backup"),
        confirm: after.bool("backup_dialog"),
        use_folder: after.bool("backup_folder_on"),
        folder_removable_only: after.bool("backup_folder_removable"),
        folder: after.text("backup_folder").to_owned(),
        kind: stored_kind(after),
        date_bits,
        generations: after.int("backup_generations").clamp(1, 99),
        ext_char,
        advanced_time: after.int("backup_advanced_type"),
        dust_box: after.bool("backup_dust_box"),
        advanced: after.bool("backup_advanced"),
        advanced_format: after.text("backup_file").to_owned(),
    });
}

/// `GetBackupExtChar` of the file's `bBackUpType2_Opt2`.
fn ext_char_of(profile: &ume_config::Profile) -> char {
    let opt2 = profile
        .get_i32(super::super::model::SECTION, OPT2)
        .unwrap_or(OPT2_DEFAULT);
    char::from_u32(((opt2 >> 16) & 0xff) as u32)
        .filter(|c| *c != '\0')
        .unwrap_or('b')
}

/// Takes the first letter of the extension from the file that the application started with.
pub(in crate::common_settings) fn set_ext_char(cx: &mut gpui::App, profile: &ume_config::Profile) {
    let mut prefs = crate::file_ops::backup::prefs(cx);
    prefs.ext_char = ext_char_of(profile);
    cx.set_global(prefs);
}
