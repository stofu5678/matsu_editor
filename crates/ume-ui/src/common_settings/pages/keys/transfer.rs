//! インポート and エクスポート: `CPropKeybind::Import` and `Export`, `CImpExpKeybind`
//! (`sakura_core/typeprop/CImpExpManager.cpp`), the `.key` file.
//!
//! The file is a profile with the version and the number of keys in `[Info]` and the keys in
//! `[KeyBind]` (`ShareData_IO_KeyBind`), the functions as macro names where they have one. Importing
//! puts the rows of the file over the table of the page (a key the file has gets the file's
//! functions; the others stay); nothing is written to the settings file until OK.
//!
//! Not built: the old `.key` files of the version 2 (the text file that starts with
//! `// テキストエディタキー設定 Ver2`), and the mouse rows of the version 3 (they have no key code, only
//! a name). Such files are refused as `STR_IMPEXP_KEY_FORMAT` says.

use std::path::Path;

use gpui::{Context, Window};
use ume_commands::keymap::user::{format_entries_named, parse_entries};
use ume_config::Profile;

use super::super::super::dialog::CommonSettingsDialog;
use super::bindings::Bindings;
use super::{ROWS, bindings_of};
use crate::common_settings::transfer::{cannot_export, cannot_open, exported, say};
use crate::file_dialog::Ask;

/// `GetDefaultExtension` of `CImpExpKeybind`.
const WILDCARD: &str = "*.key";
const INFO: &str = "Info";
const SECTION: &str = "KeyBind";
/// `WSTR_KEYBIND_HEAD4` (多言語対応) and `WSTR_KEYBIND_HEAD3`.
const VERSION_4: &str = "SakuraKeyBind_Ver4";
const VERSION_3: &str = "SakuraKeyBind_Ver3";
/// The size of the table, `_countof( m_pKeyNameArr ) - 1`: more keys than this are refused.
const KEYNAME_SIZE: i32 = 0x100;

/// `STR_IMPEXP_KEY_FORMAT`.
fn wrong_format(path: &Path) -> String {
    format!("キー設定ファイルの形式が違います。\n\n{}", path.display())
}

/// `CImpExpKeybind::Import`: the table with the rows of `file` over `table`. `None` when the file is not
/// a key file of the version 3 or 4, or says it has more keys than the table can hold.
pub(super) fn read_key_file(file: &Profile, table: &Bindings) -> Option<Bindings> {
    let version = file.get(INFO, "KEYBIND_VERSION");
    if version != Some(VERSION_4) && version != Some(VERSION_3) {
        return None;
    }
    // A file without the count keeps the count the table has, which is in range.
    let count = file
        .get(INFO, "KEYBIND_COUNT")
        .and_then(|text| text.trim().parse::<i32>().ok());
    if count.is_some_and(|n| !(0..=KEYNAME_SIZE).contains(&n)) {
        return None;
    }
    // A function that is no name or number is "none" (`F_DEFAULT`); a key that is not in the table
    // is left out. Sakura says nothing of either.
    let (rows, _) = parse_entries(file.entries(SECTION));
    let mut table = table.clone();
    for row in rows {
        if let Some(own) = table.rows.iter_mut().find(|own| own.key == row.key) {
            own.functions = row.functions;
        }
    }
    Some(table)
}

/// `CImpExpKeybind::Export`: the contents of a `.key` file.
pub(super) fn key_file(table: &Bindings) -> Profile {
    let mut file = Profile::new();
    // The names are fixed strings and numbers: setting them cannot fail.
    let _ = file.set(INFO, "KEYBIND_VERSION", VERSION_4);
    let _ = file.set(INFO, "KEYBIND_COUNT", &table.rows.len().to_string());
    for (name, value) in format_entries_named(&table.rows) {
        let _ = file.set(SECTION, &name, &value);
    }
    file
}

impl CommonSettingsDialog {
    /// 「インポート」: asks for a file and reads it over the table.
    pub(super) fn import_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pick_settings_file(
            Ask::open(false).titled("キー割り当てのインポート", "インポート"),
            WILDCARD,
            window,
            cx,
            |this, path, window, cx| {
                this.import_key_file(path, window, cx);
            },
        );
    }

    /// `CImpExpKeybind::Import` of `path`. A file that cannot be read or is not a key file is said so
    /// and changes nothing. Returns whether the table was read.
    pub(super) fn import_key_file(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Ok(file) = Profile::load(path) else {
            say(&cannot_open(path), window, cx);
            return false;
        };
        let Some(table) = read_key_file(&file, &bindings_of(self.values())) else {
            say(&wrong_format(path), window, cx);
            return false;
        };
        self.set_numbers(ROWS, table.to_numbers(), cx);
        // The list of the keys of the function is made again (`LBN_SELCHANGE` of `IDC_LIST_FUNC`).
        self.show_keys_of_function(cx);
        true
    }

    /// 「エクスポート」: asks for a name and writes the table.
    pub(super) fn export_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pick_settings_file(
            Ask::save().titled("キー割り当てのエクスポート", "保存"),
            WILDCARD,
            window,
            cx,
            |this, path, window, cx| {
                this.export_key_file(path, window, cx);
            },
        );
    }

    /// `CImpExpKeybind::Export` to `path`. Returns whether the file was written.
    pub(super) fn export_key_file(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let written = key_file(&bindings_of(self.values()))
            .save_sakura_style(path, Some(VERSION_4))
            .is_ok();
        say(
            &if written {
                exported(path)
            } else {
                cannot_export(path)
            },
            window,
            cx,
        );
        written
    }
}
