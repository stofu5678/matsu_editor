//! The list of the page 「プラグイン」: the 40 slots of the plugin table.
//!
//! Sakura fills the list from the table and the plugins it has read (`CPropPlugin::SetData_LIST`,
//! `sakura_core/prop/CPropComPlugin.cpp`): the number, the name of the plugin or `-`, the state, and
//! the folder. A row here has more texts than the list shows, so that the page needs nothing else:
//! the ID, the number of commands and the state of the slot (what OK writes), and the description,
//! the author, the version and the URL of the plugin (what the page shows under the list).

use matsu_config::plugins::consent::{self, Consent, ConsentFile};
use matsu_config::plugins::{
    DEFAULT_LANGUAGE, MAX_PLUGIN, PluginRec, PluginSettings, PluginState, SearchDirs, Unsupported,
    search_dirs,
};
use matsu_config::{Profile, ProfileError};

pub(super) use super::super::super::table::Row;
use super::super::super::table::{Column, Storage, TableSpec};

/// `STR_PROPCOMPLG_LIST1` to `LIST5` (the ID is MATSU's, ADR-0026) and the widths in percent.
static LIST: TableSpec = TableSpec {
    columns: &[
        column("番号", 7),
        column("プラグイン名", 29),
        column("状態", 29),
        column("ID", 20),
        column("フォルダー", 15),
        // The texts that a row holds besides what the list shows (see the places below). A column
        // with no width is not drawn, but `TableSpec` keeps every column of a row.
        hidden(),
        hidden(),
        hidden(),
        hidden(),
        hidden(),
        hidden(),
        hidden(),
        hidden(),
        hidden(),
        hidden(),
    ],
    max_rows: MAX_PLUGIN,
    // The buttons of the page are its own.
    ops: &[],
    defaults: &[],
    storage: Storage::Custom { load, store },
    full_message: "",
    prepare: None,
};

const fn column(title: &'static str, width: u32) -> Column {
    Column {
        title,
        edit_label: "",
        key: "",
        width,
        max_chars: 260,
    }
}

const fn hidden() -> Column {
    column("", 0)
}

pub(in crate::common_settings) fn spec() -> &'static TableSpec {
    &LIST
}

/// The places in a row.
pub(super) const NUMBER: usize = 0;
pub(super) const NAME: usize = 1;
pub(super) const STATE: usize = 2;
pub(super) const ID: usize = 3;
pub(super) const FOLDER: usize = 4;
/// The number of commands (`m_nCmdNum`).
pub(super) const COMMANDS: usize = 5;
/// The state of the slot (`m_state`), as [`code_of`] writes it.
pub(super) const CODE: usize = 6;
pub(super) const DESCRIPTION: usize = 7;
pub(super) const AUTHOR: usize = 8;
pub(super) const VERSION: usize = 9;
pub(super) const URL: usize = 10;
/// Whether the user enabled the plugin: `on`, `off`, or `changed` (enabled once, and a file of the
/// plugin is not the one agreed to; ADR-0026, chapter 4).
pub(super) const CONSENT: usize = 11;
/// The files (as `consent::manifest_text`) of a plugin that the user enabled in this dialog and
/// that `plugins.ini` does not have yet. Empty when there is nothing to write.
pub(super) const MANIFEST: usize = 12;
/// Why the plugin cannot be enabled: [`NOT_FOUND_NOTE`], or the words of [`unsupported_text`]; empty
/// when it can.
pub(super) const NOTE: usize = 13;

/// How many options `plugin.def` defines (`m_options.size()`).
pub(super) const OPTIONS: usize = 14;

pub(super) const NOT_FOUND_NOTE: &str = "notfound";

/// The words for the state of a slot. `STR_PROPCOMPLG_STATE1`, `STATE2` and `STATE5` for the
/// changes made in the dialog; 「登録済み」 for a plugin that is in the table (Sakura says 停止 until
/// the plugin is read, and 稼働 when it is; nothing is read or run here).
const ADDED: &str = "追加";
const UPDATED: &str = "更新";
const DELETED: &str = "削除";
const ENABLED: &str = "有効";
const DISABLED: &str = "無効";
const CHANGED: &str = "変更あり（停止）";
const STOPPED: &str = "停止（失敗が続きました）";
const NOT_FOUND: &str = "見つかりません";

fn code_of(state: PluginState) -> &'static str {
    match state {
        PluginState::None => "none",
        PluginState::Installed => "added",
        PluginState::Updated => "updated",
        PluginState::Stopped | PluginState::Loaded => "registered",
        PluginState::Deleted => "deleted",
    }
}

fn state_of(code: &str) -> PluginState {
    match code {
        "added" => PluginState::Installed,
        "updated" => PluginState::Updated,
        "registered" => PluginState::Stopped,
        "deleted" => PluginState::Deleted,
        _ => PluginState::None,
    }
}

/// What MATSU cannot run, in words (ADR-0026, chapter 1).
pub(in crate::common_settings) fn unsupported_text(why: &Unsupported) -> String {
    match why {
        Unsupported::Dll => "DLL プラグインは使えません".to_owned(),
        Unsupported::VbScript => "VBScript は使えません".to_owned(),
        Unsupported::UnknownType(kind) if kind.is_empty() => "種別がありません".to_owned(),
        Unsupported::UnknownType(kind) => format!("種別「{kind}」は使えません"),
    }
}

/// The text of the column 状態 from the state of the slot, the note and what the user enabled.
/// A plugin that is in the table says 有効 or 無効 (ADR-0026, chapter 4); 変更あり is a plugin that
/// was enabled and whose files are not the ones the user agreed to.
fn state_text(state: PluginState, note: &str, consent: &str) -> String {
    let with_note = |base: &str| {
        if !note.is_empty() && note != NOT_FOUND_NOTE {
            format!("{base}（{note}）")
        } else if consent == "on" {
            format!("{base}・{ENABLED}")
        } else {
            base.to_owned()
        }
    };
    match state {
        PluginState::None => String::new(),
        PluginState::Installed => with_note(ADDED),
        PluginState::Updated => with_note(UPDATED),
        PluginState::Deleted => DELETED.to_owned(),
        PluginState::Stopped | PluginState::Loaded => match (note, consent) {
            (NOT_FOUND_NOTE, _) => NOT_FOUND.to_owned(),
            ("", "on") => ENABLED.to_owned(),
            ("", "changed") => CHANGED.to_owned(),
            ("", "stopped") => STOPPED.to_owned(),
            ("", _) => DISABLED.to_owned(),
            (note, _) => note.to_owned(),
        },
    }
}

/// Makes the column 状態 from the other columns of the row.
pub(super) fn refresh_state(row: &mut Row) {
    let state = state_of(&row[CODE]);
    row[STATE] = state_text(state, &row[NOTE], &row[CONSENT]);
}

/// The user enabled (`on`, with the files they agreed to) or disabled (`off`) the plugin of the row.
pub(super) fn set_consent(row: &mut Row, on: bool, manifest: &str) {
    row[CONSENT] = (if on { "on" } else { "off" }).to_owned();
    row[MANIFEST] = manifest.to_owned();
    refresh_state(row);
}

/// Whether the plugin of the row is in the table and can be enabled: it is found and MATSU can run it.
pub(super) fn can_enable(row: &Row) -> bool {
    has_plugin(row) && row.get(NOTE).is_some_and(String::is_empty)
}

/// The row of slot `index`: what the table has and what the folder of the plugin says. `dirs` is
/// where the folders are; with `None` no plugin is found.
pub(super) fn row_of(
    index: usize,
    rec: &PluginRec,
    dirs: Option<&SearchDirs>,
    consent: &ConsentFile,
) -> Row {
    let mut row = vec![String::new(); OPTIONS + 1];
    row[CONSENT] = "off".to_owned();
    row[NUMBER] = index.to_string();
    row[COMMANDS] = rec.cmd_num.to_string();
    row[CODE] = code_of(rec.state).to_owned();
    row[ID] = rec.id.clone();
    if rec.state == PluginState::None {
        return row;
    }
    if rec.state == PluginState::Deleted {
        // `SetData_LIST`: no plugin, no folder.
        row[NAME] = "-".to_owned();
        row[STATE] = DELETED.to_owned();
        row[NOTE] = NOT_FOUND_NOTE.to_owned();
        return row;
    }
    row[FOLDER] = rec.name.clone();
    let def = dirs.and_then(|dirs| dirs.load_def(&rec.name, DEFAULT_LANGUAGE));
    match &def {
        Some(def) => {
            row[NAME] = def.name.clone();
            row[DESCRIPTION] = def.description.clone();
            row[AUTHOR] = def.author.clone();
            row[VERSION] = def.version.clone();
            row[URL] = def.url.clone();
            row[OPTIONS] = def.options.len().to_string();
        }
        None => row[NAME] = "-".to_owned(),
    }
    row[NOTE] = match &def {
        None => NOT_FOUND_NOTE.to_owned(),
        Some(def) => def
            .unsupported()
            .as_ref()
            .map(unsupported_text)
            .unwrap_or_default(),
    };
    let plugin_dir = dirs.and_then(|dirs| dirs.plugin_dir(&rec.name));
    row[CONSENT] = match consent.consent_for_dir(&rec.id, plugin_dir.as_deref()) {
        Consent::Off => "off",
        // Three failures in a row stopped it (ADR-0026, chapter 2).
        Consent::On if crate::macros::plugin_call::is_stopped(&rec.id) => "stopped",
        Consent::On => "on",
        Consent::Changed(_) => "changed",
    }
    .to_owned();
    refresh_state(&mut row);
    row
}

/// The 40 rows for the table.
pub(in crate::common_settings) fn rows_of(
    settings: &PluginSettings,
    dirs: Option<&SearchDirs>,
) -> Vec<Row> {
    let consent = consent_of(dirs);
    settings
        .table
        .iter()
        .enumerate()
        .map(|(index, rec)| row_of(index, rec, dirs, &consent))
        .collect()
}

/// `plugins.ini`, next to the settings file; empty without a place for the plugins.
pub(super) fn consent_of(dirs: Option<&SearchDirs>) -> ConsentFile {
    dirs.map(|dirs| ConsentFile::load(&dirs.consent_file()))
        .unwrap_or_default()
}

/// The table the rows stand for (`GetData`). `enabled` is the check box, which the rows do not hold.
pub(in crate::common_settings) fn settings_of(rows: &[Row], enabled: bool) -> PluginSettings {
    let mut settings = PluginSettings {
        enabled,
        ..PluginSettings::default()
    };
    for (rec, row) in settings.table.iter_mut().zip(rows) {
        let text = |column: usize| row.get(column).map(String::as_str).unwrap_or_default();
        rec.state = state_of(text(CODE));
        if rec.state == PluginState::None {
            continue;
        }
        rec.id = text(ID).to_owned();
        rec.name = text(FOLDER).to_owned();
        rec.cmd_num = text(COMMANDS).parse().unwrap_or(0);
    }
    settings
}

fn load(profile: &Profile) -> Option<Vec<Row>> {
    Some(rows_of(
        &PluginSettings::from_profile(profile),
        search_dirs().as_ref(),
    ))
}

/// `ShareData_IO_Plugin` (writing).
fn store(rows: &[Row], profile: &mut Profile) -> Result<bool, ProfileError> {
    let enabled = PluginSettings::from_profile(profile).enabled;
    settings_of(rows, enabled).store(profile)
}

/// Whether the slot of the row has a plugin that can be removed.
pub(super) fn has_plugin(row: &Row) -> bool {
    matches!(
        row.get(CODE).map(|code| state_of(code)),
        Some(PluginState::Installed | PluginState::Updated | PluginState::Stopped)
    )
}

/// `UninstallPlugin` on the rows: the slot keeps its ID and loses its folder.
pub(super) fn uninstall(rows: &mut [Row], index: usize, dirs: Option<&SearchDirs>) {
    let mut settings = settings_of(rows, false);
    settings.uninstall(index);
    if let Some(row) = rows.get_mut(index) {
        *row = row_of(index, &settings.table[index], dirs, &consent_of(dirs));
    }
}

/// What OK does to `plugins.ini` (ADR-0026, chapter 5): a plugin enabled in the dialog is recorded
/// with the files it was enabled with, one disabled or removed is forgotten, and the rest stays
/// (a plugin stopped for a change keeps its record until the user enables it again or disables it).
/// Returns whether the file changed.
pub(in crate::common_settings) fn write_consent(rows: &[Row], file: &mut ConsentFile) -> bool {
    let mut changed = false;
    for row in rows {
        let text = |column: usize| row.get(column).map(String::as_str).unwrap_or_default();
        let id = text(ID);
        if id.is_empty() {
            continue;
        }
        match state_of(text(CODE)) {
            PluginState::None => {}
            PluginState::Deleted => changed |= file.disable(id),
            _ => match text(CONSENT) {
                "on" if !text(MANIFEST).is_empty() => {
                    let files = consent::parse_manifest(text(MANIFEST));
                    // An ID that the file cannot hold is not recorded (and does not run).
                    changed |= file
                        .enable(id, text(FOLDER), &files, &consent::today())
                        .is_ok();
                }
                "off" => changed |= file.disable(id),
                _ => {}
            },
        }
    }
    changed
}
