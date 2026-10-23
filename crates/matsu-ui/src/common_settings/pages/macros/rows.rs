//! The list of the page 「マクロ」: 50 rows of 番号, マクロ名, ファイル名, 実行時に読み込み and 自動実行,
//! and what 設定 does to them.
//!
//! Sakura keeps the rows in the list view itself and reads them back when the page is left
//! (`CPropMacro::SetData`, `GetData`, `SetMacro2List_Macro` in `sakura_core/prop/CPropComMacro.cpp`):
//! the reload flag is the text `on` or `off`, and the automatic macros are the letters `O`
//! (オープン後), `T` (タイプ変更後) and `S` (保存前) in the last column. The dialog here keeps the
//! same texts, so that the rules are the same.

use matsu_config::Profile;
use matsu_config::ProfileError;
use matsu_config::settings::macros::{
    MACRONAME_MAX, MAX_CUSTMACRO, MAX_PATH, MacroRec, RegisteredMacros, macro_folder,
    store_macro_folder, with_last_separator,
};

use super::super::super::table::{Column, Row, Storage, TableSpec};

/// `STR_PROPCOMMACR_LIST1` to `LIST5` and the widths of `InitDialog` (out of 499 - 17).
static LIST: TableSpec = TableSpec {
    columns: &[
        column("番号", 8),
        column("マクロ名", 41),
        column("ファイル名", 31),
        column("実行時に読み込み", 8),
        column("自動実行", 8),
    ],
    max_rows: MAX_CUSTMACRO,
    // The list has no buttons of its own: 設定 is the page's.
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
        max_chars: MAX_PATH,
    }
}

pub(super) fn spec() -> &'static TableSpec {
    &LIST
}

const NAME: usize = 1;
const FILE: usize = 2;
const RELOAD: usize = 3;
const AUTO: usize = 4;

/// The letters of the column 自動実行, in Sakura's order.
const OPENED: char = 'O';
const TYPE_CHANGED: char = 'T';
const SAVE: char = 'S';

/// The texts of the macro `index` as the list shows them (`SetData`).
fn row_of(index: usize, table: &RegisteredMacros) -> Row {
    let rec = table.get(index).cloned().unwrap_or_default();
    let mut letters = String::new();
    for (letter, number) in [
        (OPENED, table.on_opened),
        (TYPE_CHANGED, table.on_type_changed),
        (SAVE, table.on_save),
    ] {
        if number == Some(index) {
            letters.push(letter);
        }
    }
    vec![
        index.to_string(),
        rec.name,
        rec.file,
        (if rec.reload { "on" } else { "off" }).to_owned(),
        letters,
    ]
}

/// The 50 rows for the registered macros.
pub(super) fn rows_of(table: &RegisteredMacros) -> Vec<Row> {
    (0..MAX_CUSTMACRO).map(|i| row_of(i, table)).collect()
}

/// What `GetData` makes of the rows: the macros, and the automatic macros (the last row with the
/// letter wins, as the loop of Sakura's goes on to the end). The timer is not in the list.
pub(super) fn table_of(rows: &[Row], timer: i32) -> RegisteredMacros {
    let mut table = RegisteredMacros {
        cancel_timer: timer,
        ..RegisteredMacros::default()
    };
    for (index, row) in rows.iter().enumerate().take(MAX_CUSTMACRO) {
        let text = |column: usize| row.get(column).cloned().unwrap_or_default();
        table.macros[index] = MacroRec {
            name: text(NAME),
            file: text(FILE),
            // `wcscmp(buf, L"on") == 0`
            reload: text(RELOAD) == "on",
        };
        for letter in text(AUTO).chars() {
            match letter {
                OPENED => table.on_opened = Some(index),
                TYPE_CHANGED => table.on_type_changed = Some(index),
                SAVE => table.on_save = Some(index),
                _ => {}
            }
        }
    }
    table
}

fn load(profile: &Profile) -> Option<Vec<Row>> {
    Some(rows_of(&RegisteredMacros::from_profile(profile)))
}

/// `ShareData_IO_Macro` (writing) and the end of `CPropMacro::GetData`: the folder of the macros
/// ends with a backslash.
fn store(rows: &[Row], profile: &mut Profile) -> Result<bool, ProfileError> {
    let timer = RegisteredMacros::from_profile(profile).cancel_timer;
    let mut changed = table_of(rows, timer).store(profile)?;
    if let Some(folder) = macro_folder(profile) {
        changed |= store_macro_folder(profile, &with_last_separator(&folder))?;
    }
    Ok(changed)
}

/// The fields of the page about one row (`CheckListPosition_Macro`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Fields {
    pub name: String,
    pub file: String,
    pub reload: bool,
    pub opened: bool,
    pub type_changed: bool,
    pub save: bool,
}

pub(super) fn fields_of(row: &Row) -> Fields {
    let text = |column: usize| row.get(column).map(String::as_str).unwrap_or_default();
    let letters = text(AUTO);
    Fields {
        name: text(NAME).to_owned(),
        file: text(FILE).to_owned(),
        reload: text(RELOAD) == "on",
        opened: letters.contains(OPENED),
        type_changed: letters.contains(TYPE_CHANGED),
        save: letters.contains(SAVE),
    }
}

/// 設定 (`SetMacro2List_Macro`): puts the fields into row `index`. The name takes at most 63
/// characters and the file 260 (`EditCtl_LimitText`). A check box of an automatic macro that is
/// on makes this row the automatic macro (another row loses it); one that is off frees the
/// macro only if this row was it.
pub(super) fn register(rows: &mut [Row], index: usize, fields: &Fields) {
    if index >= rows.len() {
        return;
    }
    let mut table = table_of(rows, 0);
    let wanted = |on: bool, now: Option<usize>| {
        if on {
            Some(index)
        } else {
            now.filter(|&n| n != index)
        }
    };
    table.on_opened = wanted(fields.opened, table.on_opened);
    table.on_type_changed = wanted(fields.type_changed, table.on_type_changed);
    table.on_save = wanted(fields.save, table.on_save);
    table.macros[index] = MacroRec {
        name: fields.name.chars().take(MACRONAME_MAX - 1).collect(),
        file: fields.file.chars().take(MAX_PATH).collect(),
        reload: fields.reload,
    };
    let fresh = rows_of(&table);
    for (row, new) in rows.iter_mut().zip(fresh) {
        *row = new;
    }
}

/// The number the page's Id box holds, in the range of the rows (the box is a list of 0 to 49).
pub(super) fn index_of(text: &str) -> usize {
    usize::try_from(super::super::super::dialog::parse_number(text))
        .unwrap_or(0)
        .min(MAX_CUSTMACRO - 1)
}
