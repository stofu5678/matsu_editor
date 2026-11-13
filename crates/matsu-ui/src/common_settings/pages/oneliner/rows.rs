//! The list of the page 「ワンライナー」: 50 rows of 番号, 名前, コマンド and 結果の置き場所, and what 設定
//! does to them (ADR-0029, decision 7).
//!
//! Sakura has no such page, so this follows the page 「マクロ」 (`CPropMacro`,
//! `sakura_core/prop/CPropComMacro.cpp`): the rows are texts that the list shows, and the page reads them
//! back when OK is pressed. The history of the input dialog (`Recent`) is not on the page: OK writes it back
//! as the file has it.

use matsu_config::settings::oneliner::{
    COMMAND_MAX_UNITS, MAX_ONELINER, NAME_MAX_UNITS, OneLiner, OneLinerError, OneLiners, OutputTo,
};
use matsu_config::{Profile, ProfileError};

use super::super::super::table::{Column, Row, Storage, TableSpec};

/// The texts of the three places for the result, as the list and the choice show them.
pub(super) const OUTPUT_OPTIONS: [(&str, i32); 3] = [
    ("置き換える", 0),
    ("次の行に入れる", 1),
    ("アウトプットに出す", 2),
];

/// The list: the widths are percents of the width of the list.
static LIST: TableSpec = TableSpec {
    columns: &[
        column("番号", 8),
        column("名前", 26),
        column("コマンド", 44),
        column("結果の置き場所", 22),
    ],
    max_rows: MAX_ONELINER,
    // The list has no buttons of its own: 設定 is the page's.
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
        max_chars: COMMAND_MAX_UNITS,
    }
}

pub(super) fn spec() -> &'static TableSpec {
    &LIST
}

const NAME: usize = 1;
const COMMAND: usize = 2;
const OUTPUT: usize = 3;

/// What the list shows for a place of the result.
pub(super) fn output_label(output: OutputTo) -> &'static str {
    OUTPUT_OPTIONS
        .iter()
        .find(|(_, n)| *n == output.number())
        .map_or(OUTPUT_OPTIONS[0].0, |(label, _)| label)
}

/// The place that a text of the list stands for; one that is none of them is the first, as the file reads.
fn output_of_label(label: &str) -> OutputTo {
    let number = OUTPUT_OPTIONS
        .iter()
        .find(|(text, _)| *text == label)
        .map_or(0, |(_, n)| *n);
    OutputTo::from_number(number)
}

/// The texts of slot `index` as the list shows them.
fn row_of(index: usize, one: &OneLiner) -> Row {
    vec![
        index.to_string(),
        one.name.clone(),
        one.command.clone(),
        output_label(one.output).to_owned(),
    ]
}

/// The 50 rows for the registered one-liners.
pub(super) fn rows_of(table: &OneLiners) -> Vec<Row> {
    let empty = OneLiner::default();
    (0..MAX_ONELINER)
        .map(|i| row_of(i, table.get(i).unwrap_or(&empty)))
        .collect()
}

/// The one-liners that the rows stand for, with `recent` as the history.
pub(super) fn table_of(rows: &[Row], recent: Vec<String>) -> OneLiners {
    let mut table = OneLiners {
        recent,
        ..OneLiners::default()
    };
    for (slot, row) in table.slots.iter_mut().zip(rows) {
        let text = |column: usize| row.get(column).cloned().unwrap_or_default();
        *slot = OneLiner {
            name: text(NAME),
            command: text(COMMAND),
            output: output_of_label(&text(OUTPUT)),
        };
    }
    table
}

fn load(profile: &Profile) -> Option<Vec<Row>> {
    Some(rows_of(&OneLiners::from_profile(profile)))
}

/// Writes `[OneLiner]`: the slots from the rows, the history and the keys this program does not know as the
/// file has them. The rows come from [`register`], which refuses what the file would refuse.
fn store(rows: &[Row], profile: &mut Profile) -> Result<bool, ProfileError> {
    let recent = OneLiners::from_profile(profile).recent;
    table_of(rows, recent)
        .store(profile)
        .map_err(|error| match error {
            OneLinerError::Profile(e) => e,
            other => ProfileError::InvalidValue(other.to_string()),
        })
}

/// The fields of the page about one row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Fields {
    pub name: String,
    pub command: String,
    pub output: OutputTo,
}

pub(super) fn fields_of(row: &Row) -> Fields {
    let text = |column: usize| row.get(column).map(String::as_str).unwrap_or_default();
    Fields {
        name: text(NAME).to_owned(),
        command: text(COMMAND).to_owned(),
        output: output_of_label(text(OUTPUT)),
    }
}

/// Why a value of the fields is refused. The page says so and does not change the row; it never cuts a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Problem {
    NameBreak,
    CommandBreak,
    NameTooLong,
    CommandTooLong,
    /// A name with no command to run.
    NameWithoutCommand,
}

impl Problem {
    /// The words for the user (MATSU's own wording, ADR-0018 type F).
    pub(super) fn text(self) -> String {
        match self {
            Problem::NameBreak => "名前に改行は使えません。".to_owned(),
            Problem::CommandBreak => {
                "コマンドに改行は使えません。1 行で書いてください。".to_owned()
            }
            Problem::NameTooLong => format!(
                "名前が長すぎます。{} 文字までです(絵文字などは 2 文字に数えます)。",
                NAME_MAX_UNITS - 1
            ),
            Problem::CommandTooLong => format!(
                "コマンドが長すぎます。{} 文字までです(絵文字などは 2 文字に数えます)。",
                COMMAND_MAX_UNITS - 1
            ),
            Problem::NameWithoutCommand => {
                "コマンドがないので、登録できません。コマンドを書くか、名前を消してください。"
                    .to_owned()
            }
        }
    }
}

fn units(text: &str) -> usize {
    text.encode_utf16().count()
}

fn has_break(text: &str) -> bool {
    text.contains(['\r', '\n'])
}

/// What is wrong with a name and a command, if anything. The name is taken without the spaces around it (the
/// file reads it so); a command is kept as it is typed.
pub(super) fn problem(name: &str, command: &str) -> Option<Problem> {
    let name = name.trim();
    if has_break(name) {
        Some(Problem::NameBreak)
    } else if has_break(command) {
        Some(Problem::CommandBreak)
    } else if units(name) >= NAME_MAX_UNITS {
        Some(Problem::NameTooLong)
    } else if units(command) >= COMMAND_MAX_UNITS {
        Some(Problem::CommandTooLong)
    } else if command.trim().is_empty() && !name.is_empty() {
        Some(Problem::NameWithoutCommand)
    } else {
        None
    }
}

/// 設定: puts the fields into row `index`. A command that is empty or only spaces, with no name, empties the
/// slot (it is not registered, and the file drops it). A row past the list changes nothing.
pub(super) fn register(rows: &mut [Row], index: usize, fields: &Fields) -> Result<(), Problem> {
    if let Some(problem) = problem(&fields.name, &fields.command) {
        return Err(problem);
    }
    let Some(row) = rows.get_mut(index) else {
        return Ok(());
    };
    let one = if fields.command.trim().is_empty() {
        OneLiner::default()
    } else {
        OneLiner {
            name: fields.name.trim().to_owned(),
            command: fields.command.clone(),
            output: fields.output,
        }
    };
    *row = row_of(index, &one);
    Ok(())
}

/// The number the page's Id box holds, in the range of the rows (the box is a number from 0 to 49).
pub(super) fn index_of(text: &str) -> usize {
    usize::try_from(super::super::super::dialog::parse_number(text))
        .unwrap_or(0)
        .min(MAX_ONELINER - 1)
}
