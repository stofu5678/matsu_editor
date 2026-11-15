//! The list of the page 「プラグイン」: the 40 slots of the plugin table.
//!
//! Sakura fills the list from the table and the plugins it has read (`CPropPlugin::SetData_LIST`,
//! `sakura_core/prop/CPropComPlugin.cpp`): the number, the name of the plugin or `-`, the state, and
//! the folder. A row here has more texts than the list shows, so that the page needs nothing else:
//! the ID, the number of commands and the state of the slot (what OK writes), and the description,
//! the author, the version and the URL of the plugin (what the page shows under the list).
//!
//! A row is the table's row of texts, but the state of the plugin in it is not a text to compare:
//! [`Agreement`] is what the user enabled (kept in the column [`CONSENT`] only as the table's
//! storage), [`Standing`] is what the row is now (the agreement and the stop by failures, which
//! `plugin_call::is_stopped` owns and is read when it is needed), and the words of the column 状態
//! are made from them only when the list is drawn ([`shown_state`]). What the user agreed to on
//! this page and that `plugins.ini` does not have yet is [`Agreed`], a value of its own on the
//! page, not a column of the row.

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
    ],
    max_rows: MAX_PLUGIN,
    // The buttons of the page are its own.
    ops: &[],
    defaults: &[],
    storage: Storage::Custom { load, store },
    full_message: "",
    prepare: None,
    confirm: None,
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
/// The [`Agreement`] of the row, as [`Agreement::code`] writes it (read it with [`agreement`]).
const CONSENT: usize = 11;
/// Why the plugin cannot be enabled: [`NOT_FOUND_NOTE`], or the words of [`unsupported_text`]; empty
/// when it can.
pub(super) const NOTE: usize = 12;

/// How many options `plugin.def` defines (`m_options.size()`).
pub(super) const OPTIONS: usize = 13;

pub(super) const NOT_FOUND_NOTE: &str = "notfound";

/// What the user enabled (ADR-0026, chapter 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Agreement {
    Off,
    On,
    /// Enabled once, and a file of the plugin is not the one agreed to: stopped for the change.
    Changed,
}

impl Agreement {
    fn code(self) -> &'static str {
        match self {
            Agreement::Off => "off",
            Agreement::On => "on",
            Agreement::Changed => "changed",
        }
    }

    fn of_code(code: &str) -> Self {
        match code {
            "on" => Agreement::On,
            "changed" => Agreement::Changed,
            _ => Agreement::Off,
        }
    }
}

/// What the user enabled in the row.
pub(super) fn agreement(row: &Row) -> Agreement {
    Agreement::of_code(row.get(CONSENT).map_or("", String::as_str))
}

fn set_agreement(row: &mut Row, agreement: Agreement) {
    row[CONSENT] = agreement.code().to_owned();
}

/// What the plugin of the row is now: the [`Agreement`], and for an agreement that is on, whether
/// three failures in a row stopped it (ADR-0026, chapter 2). The stop is read from its owner each
/// time, so that the row cannot keep an old answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Standing {
    Off,
    On,
    Changed,
    Stopped,
}

pub(super) fn standing(row: &Row) -> Standing {
    match agreement(row) {
        Agreement::Off => Standing::Off,
        Agreement::Changed => Standing::Changed,
        Agreement::On if crate::macros::plugin_call::is_stopped(&row[ID]) => Standing::Stopped,
        Agreement::On => Standing::On,
    }
}

/// What the user agreed to on this page that `plugins.ini` does not have yet: the ID of a plugin
/// and the files it was enabled with. It is kept apart from the rows, as a hidden list of the page
/// ([`agreed_spec`]) whose rows are the ID and the files as `consent::manifest_text`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(in crate::common_settings) struct Agreed(Vec<(String, String)>);

impl Agreed {
    pub(in crate::common_settings) fn from_rows(rows: &[Row]) -> Self {
        Self(
            rows.iter()
                .filter_map(|row| Some((row.first()?.clone(), row.get(1)?.clone())))
                .collect(),
        )
    }

    pub(super) fn to_rows(&self) -> Vec<Row> {
        self.0
            .iter()
            .map(|(id, manifest)| vec![id.clone(), manifest.clone()])
            .collect()
    }

    /// The files of the agreement made on this page for the plugin `id`.
    pub(super) fn files_of(&self, id: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(agreed, _)| agreed == id)
            .map(|(_, manifest)| manifest.as_str())
    }

    /// Records the agreement of `id`; an empty `manifest` forgets it.
    fn set(&mut self, id: &str, manifest: &str) {
        self.0.retain(|(agreed, _)| agreed != id);
        if !id.is_empty() && !manifest.is_empty() {
            self.0.push((id.to_owned(), manifest.to_owned()));
        }
    }
}

/// The hidden list that holds [`Agreed`] among the values of the page. It has no place in the file.
static AGREED: TableSpec = TableSpec {
    columns: &[hidden(), hidden()],
    max_rows: MAX_PLUGIN,
    ops: &[],
    defaults: &[],
    storage: Storage::Custom {
        load: |_| Some(Vec::new()),
        store: |_, _| Ok(false),
    },
    full_message: "",
    prepare: None,
    confirm: None,
};

pub(in crate::common_settings) fn agreed_spec() -> &'static TableSpec {
    &AGREED
}

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

/// The text of the column 状態 from the state of the slot, the note and what the row is now.
/// A plugin that is in the table says 有効 or 無効 (ADR-0026, chapter 4); 変更あり is a plugin that
/// was enabled and whose files are not the ones the user agreed to.
fn state_text(state: PluginState, note: &str, standing: Standing) -> String {
    let with_note = |base: &str| {
        if !note.is_empty() && note != NOT_FOUND_NOTE {
            format!("{base}（{note}）")
        } else if standing == Standing::On {
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
        PluginState::Stopped | PluginState::Loaded => match (note, standing) {
            (NOT_FOUND_NOTE, _) => NOT_FOUND.to_owned(),
            ("", Standing::On) => ENABLED.to_owned(),
            ("", Standing::Changed) => CHANGED.to_owned(),
            ("", Standing::Stopped) => STOPPED.to_owned(),
            ("", Standing::Off) => DISABLED.to_owned(),
            (note, _) => note.to_owned(),
        },
    }
}

/// The words of the column 状態 for the row. Made when the list is drawn (and when a test asks):
/// the row does not hold them.
pub(super) fn shown_state(row: &Row) -> String {
    state_text(state_of(&row[CODE]), &row[NOTE], standing(row))
}

/// The user enabled (with the files they agreed to) or disabled the plugin of the row. `agreed`
/// is where the files go; disabling forgets them.
pub(super) fn set_consent(row: &mut Row, agreed: &mut Agreed, on: bool, manifest: &str) {
    set_agreement(row, if on { Agreement::On } else { Agreement::Off });
    agreed.set(&row[ID], if on { manifest } else { "" });
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
    set_agreement(&mut row, Agreement::Off);
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
        row[NOTE] = NOT_FOUND_NOTE.to_owned();
        return row;
    }
    row[FOLDER] = rec.name.clone();
    let def = dirs.and_then(|dirs| dirs.load_def(&rec.name, DEFAULT_LANGUAGE));
    // The note first: it borrows the definition, whose texts then move into the row.
    row[NOTE] = match &def {
        None => NOT_FOUND_NOTE.to_owned(),
        Some(def) => def
            .unsupported()
            .as_ref()
            .map(unsupported_text)
            .unwrap_or_default(),
    };
    match def {
        Some(def) => {
            row[OPTIONS] = def.options.len().to_string();
            row[NAME] = def.name;
            row[DESCRIPTION] = def.description;
            row[AUTHOR] = def.author;
            row[VERSION] = def.version;
            row[URL] = def.url;
        }
        None => row[NAME] = "-".to_owned(),
    }
    let plugin_dir = dirs.and_then(|dirs| dirs.plugin_dir(&rec.name));
    set_agreement(
        &mut row,
        match consent.consent_for_dir(&rec.id, plugin_dir.as_deref()) {
            Consent::Off => Agreement::Off,
            Consent::On => Agreement::On,
            Consent::Changed(_) => Agreement::Changed,
        },
    );
    row
}

/// Sets the agreement of the rows that are on or changed against the folders as they are now, for
/// a page that was loaded earlier: a plugin whose files changed since is changed (変更あり), and
/// one whose files are the agreed ones again is on. What was agreed is `agreed` (enabled in this
/// dialog) or the record in `plugins.ini`. The stop by failures is not set here: it stays with the
/// plugin whatever the files are ([`standing`]). The folders are read through the cached
/// fingerprint, so a folder that did not change is not hashed again. Returns whether a row changed.
pub(super) fn refresh_consent(
    rows: &mut [Row],
    agreed: &Agreed,
    dirs: Option<&SearchDirs>,
    saved: &ConsentFile,
) -> bool {
    let mut changed = false;
    for row in rows.iter_mut() {
        if !has_plugin(row) || agreement(row) == Agreement::Off {
            continue;
        }
        let files = match agreed.files_of(&row[ID]) {
            Some(manifest) => consent::parse_manifest(manifest),
            None => match saved.record(&row[ID]) {
                Some(record) => record.files,
                None => continue,
            },
        };
        let now = dirs
            .and_then(|dirs| dirs.plugin_dir(&row[FOLDER]))
            .and_then(|dir| consent::fingerprint_cached(&dir).ok())
            .unwrap_or_default();
        let after = if consent::compare(&files, &now).is_empty() {
            Agreement::On
        } else {
            Agreement::Changed
        };
        if agreement(row) != after {
            set_agreement(row, after);
            changed = true;
        }
    }
    changed
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
pub(super) fn uninstall(
    rows: &mut [Row],
    agreed: &mut Agreed,
    index: usize,
    dirs: Option<&SearchDirs>,
) {
    let mut settings = settings_of(rows, false);
    settings.uninstall(index);
    if let Some(row) = rows.get_mut(index) {
        // A removed plugin keeps no agreement made on this page.
        agreed.set(&row[ID], "");
        *row = row_of(index, &settings.table[index], dirs, &consent_of(dirs));
    }
}

/// What OK does to `plugins.ini` (ADR-0026, chapter 5): a plugin enabled in the dialog is recorded
/// with the files it was enabled with, one disabled or removed is forgotten, and the rest stays
/// (a plugin stopped for a change keeps its record until the user enables it again or disables it).
/// Returns whether the file changed.
pub(in crate::common_settings) fn write_consent(
    rows: &[Row],
    agreed: &Agreed,
    file: &mut ConsentFile,
) -> bool {
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
            _ => match (agreement(row), agreed.files_of(id)) {
                // An agreement made on this page is saved whatever the row shows (on or changed,
                // and stopped by failures: those are the state of the files and of the run); only
                // 無効にする and 削除 clear it. The run-time check refuses until the files match it.
                (Agreement::On | Agreement::Changed, Some(manifest)) => {
                    let files = consent::parse_manifest(manifest);
                    // An ID that the file cannot hold is not recorded (and does not run).
                    changed |= file
                        .enable(id, text(FOLDER), &files, &consent::today())
                        .is_ok();
                }
                (Agreement::Off, _) => changed |= file.disable(id),
                _ => {}
            },
        }
    }
    changed
}
