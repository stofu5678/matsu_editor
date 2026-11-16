//! The list of the page 「生成 AI」: 50 rows of 番号, 名前, コマンド, 結果の置き場所 and 送る前の確認, the
//! templates, and what 設定 does to them (ADR-0035, decisions 2, 5 and 7). It is made like the list of the page
//! 「ワンライナー」 (`pages/oneliner/rows.rs`), whose limits and refusals it uses.

use matsu_config::settings::ai::{AiDestination, AiDestinations, AiOutput, MAX_AI};
use matsu_config::settings::oneliner::{COMMAND_MAX_UNITS, OneLinerError};
use matsu_config::{Profile, ProfileError};

use super::super::super::table::{Column, Row, Storage, TableSpec};
use super::super::oneliner::{Problem, problem};

/// The texts of the three places for the result, as the list and the choice show them (ADR-0035, decision 2).
pub(super) const OUTPUT_OPTIONS: [(&str, i32); 3] = [
    ("置き換える", 0),
    ("次の行に入れる", 1),
    ("ログのタブだけ", 2),
];

/// The list: the widths are percents of the width of the list.
static LIST: TableSpec = TableSpec {
    columns: &[
        column("番号", 7),
        column("名前", 22),
        column("コマンド", 40),
        column("結果の置き場所", 17),
        column("送る前の確認", 14),
    ],
    max_rows: MAX_AI,
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
const ASK: usize = 4;

const ASK_YES: &str = "する";
const ASK_NO: &str = "しない";

/// What the list shows for a place of the result.
fn output_label(output: AiOutput) -> &'static str {
    OUTPUT_OPTIONS
        .iter()
        .find(|(_, n)| *n == output.number())
        .map_or(OUTPUT_OPTIONS[1].0, |(label, _)| label)
}

/// The place that a text of the list stands for; one that is none of them is 次の行に入れる, as the file reads.
fn output_of_label(label: &str) -> AiOutput {
    let number = OUTPUT_OPTIONS
        .iter()
        .find(|(text, _)| *text == label)
        .map_or(1, |(_, n)| *n);
    AiOutput::from_number(number)
}

/// The texts of slot `index` as the list shows them.
fn row_of(index: usize, one: &AiDestination) -> Row {
    vec![
        index.to_string(),
        one.name.clone(),
        one.command.clone(),
        output_label(one.output).to_owned(),
        if one.ask_before_send { ASK_YES } else { ASK_NO }.to_owned(),
    ]
}

/// The 50 rows for the destinations.
pub(super) fn rows_of(table: &AiDestinations) -> Vec<Row> {
    let empty = AiDestination::default();
    (0..MAX_AI)
        .map(|i| row_of(i, table.get(i).unwrap_or(&empty)))
        .collect()
}

/// The destinations that the rows stand for.
pub(super) fn table_of(rows: &[Row]) -> AiDestinations {
    let mut table = AiDestinations::default();
    for (slot, row) in table.slots.iter_mut().zip(rows) {
        let text = |column: usize| row.get(column).cloned().unwrap_or_default();
        *slot = AiDestination {
            name: text(NAME),
            command: text(COMMAND),
            output: output_of_label(&text(OUTPUT)),
            ask_before_send: text(ASK) != ASK_NO,
        };
    }
    table
}

fn load(profile: &Profile) -> Option<Vec<Row>> {
    Some(rows_of(&AiDestinations::from_profile(profile)))
}

/// Writes `[AI]`: the slots from the rows; the keys this program does not know stay as the file has them.
/// The rows come from [`register`], which refuses what the file would refuse.
fn store(rows: &[Row], profile: &mut Profile) -> Result<bool, ProfileError> {
    table_of(rows).store(profile).map_err(|error| match error {
        OneLinerError::Profile(e) => e,
        other => ProfileError::InvalidValue(other.to_string()),
    })
}

/// The fields of the page about one row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Fields {
    pub name: String,
    pub command: String,
    pub output: AiOutput,
    /// 送る前に確認する.
    pub ask: bool,
}

impl Default for Fields {
    fn default() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            output: AiOutput::default(),
            ask: true,
        }
    }
}

pub(super) fn fields_of(row: &Row) -> Fields {
    let text = |column: usize| row.get(column).map(String::as_str).unwrap_or_default();
    Fields {
        name: text(NAME).to_owned(),
        command: text(COMMAND).to_owned(),
        output: output_of_label(text(OUTPUT)),
        ask: text(ASK) != ASK_NO,
    }
}

/// 設定: puts the fields into row `index`. A command that is empty or only spaces, with no name, empties the
/// slot. A row past the list changes nothing. A value that the file cannot hold is refused.
pub(super) fn register(rows: &mut [Row], index: usize, fields: &Fields) -> Result<(), Problem> {
    if let Some(problem) = problem(&fields.name, &fields.command) {
        return Err(problem);
    }
    let Some(row) = rows.get_mut(index) else {
        return Ok(());
    };
    let one = if fields.command.trim().is_empty() {
        AiDestination::default()
    } else {
        AiDestination {
            name: fields.name.trim().to_owned(),
            command: fields.command.clone(),
            output: fields.output,
            ask_before_send: fields.ask,
        }
    };
    *row = row_of(index, &one);
    Ok(())
}

/// A template for the command of a destination (ADR-0035, decision 7; ADR-0030, decisions 3, 4, 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Template {
    /// The name in the choice.
    pub label: &'static str,
    /// The command line.
    pub command: &'static str,
}

/// The choice of 「ひな形」: 0 is "no template", then the templates (their numbers are 1 and up).
pub(super) const TEMPLATE_OPTIONS: [(&str, i32); 3] = [
    ("(選んでください)", 0),
    ("Claude Code", 1),
    ("Codex CLI", 2),
];

/// The templates. They carry no instruction: the text that is sent is the whole prompt, and it goes in through
/// the standard input, never into an argument (ADR-0035). The flags turn the tools off and keep the tool from
/// writing a history. They were checked against `--help` of claude 2.1.323 and codex-cli 0.160.0
/// (`docs/poc/ai-send.md`, 2026-11-13) and against the documents of the tools.
///
/// * Claude Code: `--tools ""` turns every tool off, `--permission-mode dontAsk` denies what would ask and
///   `--no-session-persistence` is for `--print` only. With `-p` and no prompt argument, the prompt is the
///   standard input (the headless document; not tried here, the tool is not run).
/// * Codex CLI: `-` as the prompt reads it from the standard input (`codex exec`). `--sandbox read-only` is the
///   default, written out. The working folder is an empty folder that is not a Git repository, hence
///   `--skip-git-repo-check`.
/// * Ollama has no template: its documents do not show `ollama run MODEL` reading a prompt from the standard
///   input for text generation (only for an embedding model), so it cannot be promised to work here.
pub(super) const TEMPLATES: [Template; 2] = [
    Template {
        label: "Claude Code",
        command: "claude -p --no-session-persistence --tools \"\" --permission-mode dontAsk",
    },
    Template {
        label: "Codex CLI",
        command: "codex exec --sandbox read-only --ephemeral --skip-git-repo-check -",
    },
];

/// The template that the choice value stands for.
pub(super) fn template_of(number: i32) -> Option<&'static Template> {
    TEMPLATES.get(usize::try_from(number.checked_sub(1)?).ok()?)
}

/// 「ひな形」: the fields with the template put in. The command is the template's and the place of the result
/// is 次の行に入れる (ADR-0030, decision 5). The name stays unless there is none; the question before sending
/// is not touched.
pub(super) fn apply_template(fields: &Fields, template: &Template) -> Fields {
    Fields {
        name: if fields.name.trim().is_empty() {
            template.label.to_owned()
        } else {
            fields.name.clone()
        },
        command: template.command.to_owned(),
        output: AiOutput::NextLine,
        ask: fields.ask,
    }
}
