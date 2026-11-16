//! Sending to a generative AI, mode a (ADR-0035, 3): runs destination `n` of `[AI]` on the target text, which goes as
//! the prompt on the standard input, nothing added.
//!
//! The machinery is the one-liners' (ADR-0030 option a; ADR-0035, 8): `crate::oneliner::flow` starts the process,
//! shows the box 「コマンド実行中・・・」, asks the confirmation (`crate::ai_confirm`), keeps the record in the log tab and
//! puts the result in. This module is the entry: it reads the destination and says what is different from a
//! one-liner.
//!
//! * The target is the selection, or the whole document when nothing is selected; a box selection is the text of
//!   the box (as for the one-liners, ADR-0029). The line breaks are LF. An empty target sends nothing and says so.
//! * Where the result goes is [`AiOutput`]: 置き換える, 次の行に入れる, or the log tab only.
//! * The confirmation window shows [`PLAIN_NOTICE`] at its top.
//!
//! Its tests are with the one-liners' (`crate::oneliner::tests::ai`), whose harness they share.
//!
//! The functions are `ui_commands/ai/`: `F_AI_PROMPT_0` to `F_AI_PROMPT_49`. Mode b, the modal and the menus are
//! other steps of ADR-0035.

use gpui::{Context, Window};
use matsu_commands::command::Result as CommandResult;
use matsu_config::settings::ai::{AiDestinations, AiOutput};
use matsu_config::settings::oneliner::OutputTo;

use crate::oneliner::flow::{AiSlot, Job, run_job};
use crate::settings::UserSettings;
use crate::view::EditorView;

pub(crate) mod store;

/// The line at the top of the confirmation window in mode a (ADR-0035, 3).
pub(crate) const PLAIN_NOTICE: &str = "選んだ文字列が、そのままプロンプトとして送られます";

/// The destinations of `[AI]` now.
pub(crate) fn table(cx: &gpui::App) -> AiDestinations {
    cx.try_global::<UserSettings>()
        .map(UserSettings::ai_destinations)
        .unwrap_or_default()
}

/// `F_AI_PROMPT_0 + index`: sends the target text to destination `index` of `[AI]` as the prompt. A destination
/// with no command does nothing but say so (a key or a macro can name any number), as for the one-liners.
pub(crate) fn run_prompt(
    view: &mut EditorView,
    index: usize,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) -> CommandResult {
    let table = table(cx);
    let Some(destination) = table.get(index).filter(|d| d.is_registered()) else {
        view.notice = Some(format!("生成 AI の送り先 {index} は登録されていません。"));
        cx.notify();
        return Ok(());
    };
    run_job(
        view,
        Job {
            line: destination.command.clone(),
            output: output_to(destination.output),
            ai: Some(AiSlot {
                index,
                name: destination.title().to_owned(),
                ask: destination.ask_before_send,
            }),
        },
        window,
        cx,
    )
}

/// The place of the result as the flow of the one-liners has it. The log tab only is the one that is not the
/// document: for an AI job the flow does not use the output window (see `crate::oneliner::flow`).
fn output_to(output: AiOutput) -> OutputTo {
    match output {
        AiOutput::Replace => OutputTo::Replace,
        AiOutput::NextLine => OutputTo::NextLine,
        AiOutput::LogOnly => OutputTo::OutputWindow,
    }
}
