//! 「キーワードヘルプ」: `IDD_PROP_KEYHELP` (`sakura_core/sakura_rc.rc`), `CPropTypesKeyHelp`
//! (`sakura_core/typeprop/CPropTypesKeyHelp.cpp`).
//!
//! Live: the list of the dictionaries (`KDct[NN]=use,about,path`) with its edit boxes and the
//! buttons 先頭, 上へ, 下へ, 最終, 挿入, 更新, 削除 (the editable list of the settings,
//! [`crate::common_settings::table`]). 挿入 and 更新 check the dictionary as Sakura does: it must
//! not be there already and the file must open; the description is made from the first line of
//! the file (commas become dots, `strcnv`) and the dictionary is used (`ListView_SetCheckState`).
//!
//! The list shows the use of a dictionary as a column 使用 with ○ or ×, which can be typed in the
//! edit box; Sakura has a check box in each row of the list.
//!
//! Live, all of it: the check box that turns the feature on (everything else on the page is grey
//! while it is off, `IDC_CHECK_KEYHELP`), ヒットした次の辞書も検索, キーワードも表示する, 選択範囲で前方
//! 一致検索, the choice for the right-click menu, and インポート and エクスポート of the list
//! (`CImpExpKeyHelp`, [`super::keyhelp_io`]). The window looks the word up as these say
//! ([`crate::key_help`]).

use std::path::{Path, PathBuf};

use gpui::PromptLevel;

use matsu_config::Profile;
use matsu_config::settings::KeyHelpDict;
use matsu_config::settings::type_rules::{DICT_ABOUT_LEN, MAX_KEYHELP_FILES};
use matsu_core::encoding::Encoding;

use super::super::ids::*;
use super::keyhelp_io;
use crate::common_settings::CommonSettingsDialog;
use crate::common_settings::model::{Control, Item, Values};
use crate::common_settings::table::{Column, Op, Row, Storage, TableSpec};
use crate::file_dialog::{self, Ask};

/// `STR_KEYHELP_RMENU_NONE`, `_TOP` and `_BOTTOM` (`EKeyHelpRMenuType`): whether the right-click
/// menu shows the meaning of the word, and where.
pub(in crate::type_settings) const RMENU_TYPES: &[(&str, i32)] =
    &[("表示しない", 0), ("先頭に表示", 1), ("末尾に表示", 2)];

/// `KEYHELP_RMENU_TOP`: the default (`CType.cpp`).
pub(in crate::type_settings) const RMENU_DEFAULT: i32 = 1;

/// How the list shows a dictionary that is used and one that is not.
pub(in crate::type_settings) const USED: &str = "○";
pub(in crate::type_settings) const NOT_USED: &str = "×";

/// The list. The type sheet keeps the rows in the type (`values.rs`), not by `storage`.
pub(in crate::type_settings) static LIST: TableSpec = TableSpec {
    columns: &[
        Column {
            title: "辞書ファイル",
            edit_label: "辞書ファイル",
            key: "",
            width: 45,
            max_chars: 259,
        },
        Column {
            title: "辞書の説明",
            edit_label: "辞書の説明",
            key: "",
            width: 40,
            max_chars: DICT_ABOUT_LEN - 1,
        },
        Column {
            title: "使用",
            edit_label: "使用(○/×)",
            key: "",
            width: 10,
            max_chars: 1,
        },
    ],
    max_rows: MAX_KEYHELP_FILES,
    ops: &[
        Op::Top,
        Op::Up,
        Op::Down,
        Op::Last,
        Op::Insert,
        Op::Update,
        Op::Delete,
    ],
    defaults: &[],
    storage: Storage::Custom {
        load: |_: &Profile| None,
        store: |_, _| Ok(false),
    },
    // `STR_PROPTYPKEYHELP_ERR_REG1`
    full_message: "これ以上登録できません。",
    prepare: Some(prepare),
};

/// The row of a dictionary.
pub(in crate::type_settings) fn row_of(dict: &KeyHelpDict) -> Row {
    vec![
        dict.path.clone(),
        dict.about.clone(),
        (if dict.used { USED } else { NOT_USED }).to_owned(),
    ]
}

/// The dictionary of a row. A row whose use is not × is used.
pub(in crate::type_settings) fn dict_of(row: &Row) -> KeyHelpDict {
    KeyHelpDict {
        used: row.get(2).is_none_or(|t| t != NOT_USED),
        about: row.get(1).cloned().unwrap_or_default(),
        path: row[0].clone(),
    }
}

/// The first line of a dictionary file, as `CTextInputStream::ReadLineW` reads it: UTF-8 with the
/// BOM, Shift_JIS without.
fn first_line(bytes: &[u8]) -> String {
    let (bytes, encoding) = match bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        Some(rest) => (rest, Encoding::Utf8),
        None => (bytes, Encoding::ShiftJis),
    };
    let end = bytes
        .iter()
        .position(|b| matches!(b, b'\r' | b'\n'))
        .unwrap_or(bytes.len());
    let mut line = String::new();
    encoding.codec().decode(&bytes[..end], &mut line);
    line
}

/// `strcnv` and the buffer of the description: no line ending, commas are dots, and at most
/// `DICT_ABOUT_LEN - 1` characters.
fn about_of(line: &str) -> String {
    let line = line.split(['\r', '\n']).next().unwrap_or_default();
    line.chars()
        .map(|c| if c == ',' { '.' } else { c })
        .take(DICT_ABOUT_LEN - 1)
        .collect()
}

/// `CPropTypesKeyHelp::DispatchEvent`, 挿入 and 更新: the dictionary is not in the list already
/// (the row that is updated may keep its path), the file opens, and the description comes from its
/// first line. A relative path starts at the folder of the settings file
/// (`CTextInputStream_AbsIni`).
fn prepare(
    row: &mut Row,
    rows: &[Row],
    updating: Option<usize>,
    base: Option<&Path>,
) -> Result<(), String> {
    let path = row[0].clone();
    let repeated = rows
        .iter()
        .enumerate()
        .any(|(i, other)| Some(i) != updating && other[0] == path);
    if repeated {
        // `STR_PROPTYPKEYHELP_ERR_REG2`
        return Err("既に登録済みの辞書です。".to_owned());
    }
    let typed = Path::new(&path);
    let full = match base {
        Some(dir) if typed.is_relative() => dir.join(typed),
        _ => typed.to_owned(),
    };
    // Only the start of the file is needed; a dictionary can be large.
    let bytes = read_start(&full)
        // `STR_PROPTYPKEYHELP_ERR_OPEN`
        .map_err(|_| format!("ファイルを開けませんでした。\n\n{path}"))?;
    row[1] = about_of(&first_line(&bytes));
    // The check box of the new row is on; a × typed in the box keeps it off.
    if row[2] != NOT_USED {
        row[2] = USED.to_owned();
    }
    Ok(())
}

/// The first bytes of the file: the first line is in them unless it is very long.
fn read_start(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::Read as _;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(4096)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The page is grey while the feature is off (`IDC_CHECK_KEYHELP`, `BN_CLICKED`).
fn feature_on(values: &Values) -> bool {
    values.bool(USE_KEYHELP)
}

fn say(message: &str, window: &mut gpui::Window, cx: &mut gpui::Context<CommonSettingsDialog>) {
    // Nobody waits for this answer.
    drop(window.prompt(PromptLevel::Info, message, None, &["OK"], cx));
}

/// The folder of the settings file, where a relative path starts.
fn ini_dir(cx: &gpui::App) -> Option<PathBuf> {
    crate::doc_types::with_user(cx, |user| {
        crate::settings::history_file(user).and_then(|path| path.parent().map(Path::to_owned))
    })
}

/// `IDC_BUTTON_KEYHELP_IMPORT` (`CImpExpManager::ImportUI`): reads a file over the list. A file
/// that cannot be opened is said so and changes nothing; lines that cannot be read are counted
/// in a message after the import.
fn import_list(
    _: &mut CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    let receiver = file_dialog::ask(
        window,
        cx,
        Ask::open(false).titled("キーワードヘルプのインポート", "インポート"),
    );
    cx.spawn_in(window, async move |this, cx| {
        let chosen = receiver.await.ok().flatten();
        if let Some(path) = chosen.and_then(|paths| paths.into_iter().next()) {
            this.update_in(cx, |this, window, cx| import_file(this, &path, window, cx))
                .ok();
        }
    })
    .detach();
}

/// Reads the file `path` into the list.
pub(in crate::type_settings) fn import_file(
    dialog: &mut CommonSettingsDialog,
    path: &Path,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    let Ok(bytes) = std::fs::read(path) else {
        // `STR_IMPEXP_ERR_FILEOPEN`
        say(
            &format!("ファイルを開けませんでした。\n\n{}", path.display()),
            window,
            cx,
        );
        return;
    };
    let base = ini_dir(cx);
    let imported = keyhelp_io::import(&keyhelp_io::lines_of(&bytes), |p| {
        keyhelp_io::opens(p, base.as_deref())
    });
    let rows = imported.dicts.iter().map(row_of).collect();
    dialog.replace_rows(KEYHELP_LIST, rows, window, cx);
    if !imported.message.is_empty() {
        say(&imported.message, window, cx);
    }
}

/// `IDC_BUTTON_KEYHELP_EXPORT` (`CImpExpManager::ExportUI`): writes the list to a file.
fn export_list(
    _: &mut CommonSettingsDialog,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    let receiver = file_dialog::ask(window, cx, Ask::save().name(Some("keyhelp.txt".to_owned())));
    cx.spawn_in(window, async move |this, cx| {
        let chosen = receiver
            .await
            .ok()
            .flatten()
            .and_then(|paths| paths.into_iter().next());
        if let Some(path) = chosen {
            this.update_in(cx, |this, window, cx| export_file(this, &path, window, cx))
                .ok();
        }
    })
    .detach();
}

/// Writes the list to the file `path`.
pub(in crate::type_settings) fn export_file(
    dialog: &mut CommonSettingsDialog,
    path: &Path,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<CommonSettingsDialog>,
) {
    // `GetData`: a row without a file is not a dictionary.
    let dicts: Vec<KeyHelpDict> = dialog
        .values()
        .rows(KEYHELP_LIST)
        .iter()
        .filter(|row| !row[0].is_empty())
        .map(dict_of)
        .collect();
    let written = std::fs::write(path, keyhelp_io::export(&dicts)).is_ok();
    say(
        &if written {
            // `STR_IMPEXP_OK_EXPORT`
            format!("ファイルをエクスポートしました。\n\n{}", path.display())
        } else {
            // `STR_IMPEXP_ERR_EXPORT`
            format!("エクスポート出来ませんでした。\n\n{}", path.display())
        },
        window,
        cx,
    );
}

fn check(id: &'static str, label: &'static str) -> Item {
    Control::check(id, label, "", false).when(feature_on).into()
}

pub(in crate::type_settings) fn items() -> Vec<Item> {
    vec![
        Control::check(USE_KEYHELP, "キーワードヘルプ機能を使う(&K)", "", false).into(),
        Control::table(KEYHELP_LIST, "辞書ファイル一覧(&L)", &LIST)
            .when(feature_on)
            .into(),
        Item::Row(vec![
            check(KEYHELP_ALL_SEARCH, "ヒットした次の辞書も検索(&A)"),
            Control::button(KEYHELP_IMPORT, "インポート(&I)...", import_list)
                .when(feature_on)
                .into(),
            Control::button(KEYHELP_EXPORT, "エクスポート(&X)...", export_list)
                .when(feature_on)
                .into(),
        ]),
        check(KEYHELP_KEY_DISP, "キーワードも表示する(&W)"),
        Item::Row(vec![
            check(KEYHELP_PREFIX, "選択範囲で前方一致検索(&P)"),
            Control::choice(
                KEYHELP_RMENU,
                "右クリックメニュー(&R)",
                "",
                RMENU_DEFAULT,
                RMENU_TYPES,
            )
            .when(feature_on)
            .into(),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_description_is_the_first_line_without_commas_and_cut_to_the_buffer() {
        assert_eq!(about_of("辞書, 1行目"), "辞書. 1行目");
        assert_eq!(about_of("a\r\nb"), "a");
        assert_eq!(
            about_of(&"あ".repeat(80)).chars().count(),
            DICT_ABOUT_LEN - 1
        );
    }

    #[test]
    fn the_first_line_is_read_as_utf8_with_a_bom_and_as_shift_jis_without() {
        let mut utf8 = vec![0xEF, 0xBB, 0xBF];
        utf8.extend_from_slice("題名\r\nline2".as_bytes());
        assert_eq!(first_line(&utf8), "題名");
        assert_eq!(first_line(b"\x82\xA0\n2"), "あ");
        assert_eq!(first_line(b""), "");
    }
}
