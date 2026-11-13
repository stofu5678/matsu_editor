//! What running a one-liner does in the window (ADR-0029). Sakura has nothing like it; the nearest thing is 外部コマンド実行
//! (`crate::exec::flow`), whose box, whose guard against two runs and whose way of reading the selection are used
//! here.
//!
//! # The input
//!
//! The selection, or the whole document when nothing is selected, goes to the standard input with the line breaks made
//! LF (`matsu_core::oneliner::feed_lf`); the document is read by lines, never made into one string.
//!
//! A box selection is handled as 外部コマンド実行 handles it: the text of the box (one line for each row) is the input,
//! and when the result replaces it the box is deleted and the result is inserted at the caret once, not into every
//! row. For 次の行に入れる the result goes after the last row of the box.
//!
//! # While it runs
//!
//! The box 「コマンド実行中・・・」 of 外部コマンド実行 covers the view and has the keys, so nothing is typed into the text
//! meanwhile. キャンセル (or Escape) stops the process and the processes it started, and nothing changes. What the
//! window, the document and its version are is looked at again after every wait: a window that is gone ends the
//! run, and a document that changed in spite of the box, or was replaced, gets nothing and the output window says so.
//!
//! # The end
//!
//! | what happened | the document | where it is said |
//! |---|---|---|
//! | exit code 0, 置き換える / 次の行に入れる | the result, one undo step, the new text selected | stderr, if any: the output window |
//! | exit code 0, アウトプットに出す | as it was | the result and stderr: the output window |
//! | exit code not 0 | as it was | the code and stderr: the output window |
//! | キャンセル | as it was | the status bar |
//! | cannot start, cannot be written for `cmd`, too large, cannot be read | as it was | a message box, as 外部コマンド実行 tells a command that cannot start |
//!
//! The line breaks of the result are made the ones of the document.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use gpui::{App, AsyncWindowContext, Context, PromptLevel, WeakEntity, Window};
use matsu_commands::command::{CommandError, Result as CommandResult};
use matsu_commands::context::CommandContext;
use matsu_commands::convert::datetime::{
    Clock, DEFAULT_DATE_PICTURE, DEFAULT_TIME_PICTURE, SystemClock, format_date, format_time,
};
use matsu_config::settings::oneliner::OutputTo;
use matsu_core::buffer::{Document, Pos};
use matsu_core::encoding::Eol;
use matsu_core::exec::{exit_code_text, header_text};
use matsu_core::oneliner::{OUTPUT_CAP_BYTES, feed_lf};
use matsu_oneliner::{Event, Finished, Input, OutputKind, Request, RunError, StartError, start};
use matsu_platform::exec::Stopper;

use crate::file_ops::ask::tell;
use crate::file_ops::start as spawn_flow;
use crate::view::EditorView;

/// How often the end of the process is looked for when it has not ended (as 外部コマンド実行 does).
const POLL: Duration = Duration::from_millis(15);

/// What to run: one line and where its result goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Job {
    pub line: String,
    pub output: OutputTo,
}

/// What is read from the window before the process starts.
struct Prepared {
    source: Source,
    editable: bool,
    doc: Document,
    /// The document that was read and its version, to see that nothing changed it while the process ran.
    generation: u64,
    version: u64,
    folder: Option<PathBuf>,
    eol: Eol,
    /// `[Terminal]` `Shell=` as it is now.
    shell: String,
}

/// The text that goes to the standard input.
enum Source {
    /// Nothing is selected: the whole document.
    Whole,
    Range(Pos, Pos),
    /// A box: its text, and the corners of the selection (the first one is the smaller).
    Box {
        text: String,
        selection: (Pos, Pos),
    },
}

/// Whether the text of the document of `view` can be changed: not in view mode, not a file that cannot be written
/// (`uneditable_if_unwritable`), not the scanned start of a big file. The same as what the commands refuse
/// (`EditorView::run_with`).
pub(crate) fn can_edit(view: &EditorView, cx: &App) -> bool {
    let shared = view.document().read(cx);
    let uneditable = crate::file_ops::prefs::prefs(cx).uneditable_if_unwritable;
    !(shared.view_mode() || shared.locked_by_unwritable(uneditable) || shared.is_partial())
}

/// `F_ONELINER_0 + index`: runs one-liner `index` of the settings. A slot with no command does nothing but say so
/// (a key or a macro can name any slot), and so does a one-liner that needs a document that can be edited when it
/// cannot be: nothing is run and the macro goes on, as for the other commands that are refused.
pub(crate) fn run_slot(
    view: &mut EditorView,
    index: usize,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) -> CommandResult {
    let table = super::table(cx);
    let Some(one) = table.get(index).filter(|one| one.is_registered()) else {
        view.notice = Some(format!("ワンライナー {index} は登録されていません。"));
        cx.notify();
        return Ok(());
    };
    run_job(
        view,
        Job {
            line: one.command.clone(),
            output: one.output,
        },
        window,
        cx,
    )
}

/// Starts the flow of `job`; a macro that called it waits until the flow has ended.
pub(crate) fn run_job(
    view: &mut EditorView,
    job: Job,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) -> CommandResult {
    if job.output != OutputTo::OutputWindow && !can_edit(view, cx) {
        view.notice =
            Some("このウィンドウは編集できないので、ワンライナーを実行しません。".to_owned());
        cx.notify();
        return Ok(());
    }
    spawn_flow(window, cx, async move |view, cx| {
        run(&view, cx, job).await;
    });
    Ok(())
}

/// Reads what the process needs from the window.
fn prepare(view: &mut EditorView, cx: &mut Context<EditorView>) -> Prepared {
    let editable = can_edit(view, cx);
    let is_box = view.cursor().is_box() && view.cursor().has_selection();
    let selection = view.cursor().selection();
    let source = match (is_box, selection) {
        (true, Some(selection)) => {
            let text = view
                .run_with(cx, |c| c.box_text_with_eol("\n"))
                .ok()
                .flatten()
                .unwrap_or_default();
            Source::Box { text, selection }
        }
        (false, Some((from, to))) => Source::Range(from, to),
        _ => Source::Whole,
    };
    let shared = view.document().read(cx);
    let folder = shared
        .path()
        .and_then(std::path::Path::parent)
        .filter(|dir| dir.is_dir())
        .map(PathBuf::from)
        .or_else(crate::terminal::home_dir);
    Prepared {
        source,
        editable,
        doc: shared.doc().clone(),
        generation: shared.generation(),
        version: shared.version(),
        folder,
        eol: shared.new_line(),
        shell: crate::terminal::settings::current(cx).shell,
    }
}

/// The text for the standard input, read from the snapshot that [`prepare`] took.
fn input_of(prepared: &Prepared) -> Input {
    match &prepared.source {
        Source::Box { text, .. } => {
            let text = text.clone();
            Arc::new(move |sink| {
                if !text.is_empty() {
                    sink(&text);
                }
                Ok(())
            })
        }
        Source::Range(from, to) => feed_input(prepared.doc.clone(), Some((*from, *to))),
        Source::Whole => feed_input(prepared.doc.clone(), None),
    }
}

fn feed_input(doc: Document, range: Option<(Pos, Pos)>) -> Input {
    Arc::new(move |sink| feed_lf(&doc, range, |piece| sink(piece)).map_err(|e| e.to_string()))
}

/// The date and time as the output window's header shows them.
fn now_texts() -> (String, String) {
    let now = SystemClock.now();
    (
        format_date(&now, DEFAULT_DATE_PICTURE),
        format_time(&now, DEFAULT_TIME_PICTURE),
    )
}

/// Runs the one-liner and waits for it, with the window usable but for the text, which the box covers.
pub(crate) async fn run(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext, job: Job) {
    // The `PATH` of the login shell (macOS started from Finder or the Dock), waited for off the thread of the window
    // and before the text is read, as 外部コマンド実行 does: then the text that is sent and the place the result goes
    // into are from the same moment.
    let login_path = if matsu_platform::login_path::is_pending() {
        cx.background_executor()
            .spawn(async { matsu_platform::login_path::wait() })
            .await
    } else {
        matsu_platform::login_path::get()
    };
    let Ok(prepared) = view.update_in(cx, |view, _window, cx| prepare(view, cx)) else {
        return;
    };
    if job.output != OutputTo::OutputWindow && !prepared.editable {
        say(
            view,
            cx,
            "このウィンドウは編集できないので、ワンライナーを実行しません。",
        );
        return;
    }
    if busy(view, cx) {
        tell_busy(cx).await;
        return;
    }

    let request = Request {
        shell: prepared.shell.clone(),
        line: job.line.clone(),
        folder: prepared.folder.clone(),
        input: input_of(&prepared),
        output: match job.output {
            OutputTo::Replace => OutputKind::Replace,
            OutputTo::NextLine => OutputKind::NextLine,
            OutputTo::OutputWindow => OutputKind::Window,
        },
        path: login_path,
    };
    // `start` reads the whole text once for `cmd` (to see that the code page can write it): not on the window's
    // thread.
    let started = cx
        .background_executor()
        .spawn(async move { start(request) })
        .await;
    let mut run = match started {
        Ok(run) => run,
        Err(error) => {
            tell(
                cx,
                PromptLevel::Warning,
                &failure_message(&job.line, &start_error_text(&error)),
                None,
            )
            .await;
            return;
        }
    };
    let stopper = run.stopper();

    // Nothing awaits between the check and the box: the box is the guard against two runs at once, shared with
    // 外部コマンド実行.
    let line = job.line.clone();
    let opened = view.update_in(cx, {
        let stopper = stopper.clone();
        move |view, window, cx| {
            if view.support.exec_running.is_some() {
                return false;
            }
            view.open_exec_running(&line, stopper, true, window, cx);
            true
        }
    });
    match opened {
        Ok(true) => {}
        Ok(false) => {
            stop_in_background(cx, stopper);
            tell_busy(cx).await;
            return;
        }
        Err(_) => {
            stop_in_background(cx, stopper);
            return;
        }
    }

    let event = loop {
        if let Some(event) = run.poll(Duration::ZERO) {
            break event;
        }
        if view.read_with(cx, |_, _| ()).is_err() {
            // the window is gone
            stop_in_background(cx, stopper);
            return;
        }
        cx.background_executor().timer(POLL).await;
    };

    // The state is looked at again, in the same step that puts the result in.
    let message = view
        .update_in(cx, |view, window, cx| {
            finish(view, &prepared, &job, event, window, cx)
        })
        .ok()
        .flatten();
    if let Some(message) = message {
        tell(cx, PromptLevel::Warning, &message, None).await;
    }
}

/// Another command has the box.
fn busy(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) -> bool {
    view.read_with(cx, |view, _| view.support.exec_running.is_some())
        .unwrap_or(false)
}

async fn tell_busy(cx: &mut AsyncWindowContext) {
    tell(
        cx,
        PromptLevel::Info,
        "別のコマンドを実行中です。終了してから実行してください。",
        None,
    )
    .await;
}

/// A notice in the status bar.
fn say(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext, text: &str) {
    let text = text.to_owned();
    view.update(cx, |view, cx| {
        view.notice = Some(text);
        cx.notify();
    })
    .ok();
}

/// Ends the process without blocking the window (stopping can take a moment).
fn stop_in_background(cx: &mut AsyncWindowContext, stopper: Stopper) {
    cx.background_executor()
        .spawn(async move { stopper.stop() })
        .detach();
}

/// 外部コマンド実行's `STR_EDITVIEW_EXECCMD_ERR` in this program's words: the one-liner and why it did not run.
fn failure_message(line: &str, why: &str) -> String {
    format!("ワンライナーの実行に失敗しました。\n\n{line}\n\n{why}")
}

fn start_error_text(error: &StartError) -> String {
    match error {
        StartError::EmptyLine => "ワンライナーが空です。".to_owned(),
        StartError::Shell(e) => format!("シェルの設定を使えません。\n{e}"),
        StartError::CannotEncode => {
            "入力に、シェル (cmd) の文字コードで表せない文字 (絵文字など) があります。実行しませんでした。"
                .to_owned()
        }
        StartError::Input(message) => format!("入力を読めませんでした。\n{message}"),
        StartError::Process { program, message } => {
            format!("シェル {program} を起動できませんでした。\n{message}")
        }
    }
}

fn run_error_text(error: &RunError) -> String {
    match error {
        RunError::OutputTooLarge => format!(
            "出力が {} MiB を超えたので、止めました。文書は変えていません。",
            OUTPUT_CAP_BYTES / (1024 * 1024)
        ),
        RunError::Input(message) => {
            format!("入力を読めませんでした。文書は変えていません。\n{message}")
        }
    }
}

/// Takes the box away and puts the result where it goes. Returns the text of a message box to show, for a run that
/// has no result to show elsewhere.
fn finish(
    view: &mut EditorView,
    prepared: &Prepared,
    job: &Job,
    event: Event,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) -> Option<String> {
    view.support.exec_running = None;
    window.focus(&view.focus_handle, cx);
    cx.notify();
    let finished = match event {
        Event::Failed(error) => return Some(failure_message(&job.line, &run_error_text(&error))),
        Event::Finished(finished) => finished,
    };
    // キャンセル: nothing changes
    if finished.exit.stopped {
        view.notice = Some("ワンライナーを中断しました。".to_owned());
        return None;
    }
    let code = finished.exit.code.unwrap_or(-1);
    if code != 0 {
        let body = error_section(&finished);
        write_panel(
            view,
            cx,
            &job.line,
            &format!("{body}{}", exit_code_text(code, "\n")),
        );
        return None;
    }
    // The document is the one that was read? It cannot be changed while the box covers it, but a document that
    // was replaced or edited anyway gets nothing.
    let shared = view.document().read(cx);
    if shared.generation() != prepared.generation || shared.version() != prepared.version {
        write_panel(
            view,
            cx,
            &job.line,
            &format!(
                "\n文書が変わったので、結果を入れませんでした。\n{}",
                error_section(&finished)
            ),
        );
        return None;
    }
    match job.output {
        OutputTo::OutputWindow => {
            let mut body = finished.output.concat();
            if !body.is_empty() && !body.ends_with('\n') {
                body.push('\n');
            }
            body.push_str(&error_section(&finished));
            write_panel(
                view,
                cx,
                &job.line,
                &format!("{body}{}", exit_code_text(0, "\n")),
            );
        }
        OutputTo::Replace | OutputTo::NextLine => {
            let result = apply(view, prepared, &finished.output, job.output, cx);
            if let Err(why) = result {
                return Some(failure_message(
                    &job.line,
                    &format!("結果を文書に入れられませんでした。文書は変えていません。\n{why}"),
                ));
            }
            if !finished.error.is_empty() {
                let body = error_section(&finished);
                write_panel(
                    view,
                    cx,
                    &job.line,
                    &format!("{body}{}", exit_code_text(0, "\n")),
                );
            }
        }
    }
    None
}

/// The standard error for the output window, under a line that says what it is. Empty when there is none.
fn error_section(finished: &Finished) -> String {
    if finished.error.is_empty() {
        return String::new();
    }
    let mut text = String::from("#標準エラー出力\n");
    text.push_str(&finished.error);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    if finished.error_truncated {
        text.push_str("(以降は省略しました)\n");
    }
    text
}

/// Shows a run in the output window: the header of 外部コマンド実行 with the one-liner, then `body`.
fn write_panel(view: &mut EditorView, cx: &mut Context<EditorView>, line: &str, body: &str) {
    let (date, time) = now_texts();
    let text = format!("{}{body}", header_text(&date, &time, line, "\n"));
    view.macros.output.trace_jump(&text, None);
    cx.notify();
}

/// Puts the result into the document as one undo step with the new text selected.
fn apply(
    view: &mut EditorView,
    prepared: &Prepared,
    output: &[String],
    to: OutputTo,
    cx: &mut Context<EditorView>,
) -> Result<(), String> {
    let eol = prepared.eol.as_str();
    let pieces: Vec<String> = output
        .iter()
        .map(|piece| {
            if eol == "\n" {
                piece.clone()
            } else {
                piece.replace('\n', eol)
            }
        })
        .collect();
    view.run_with(cx, |c| {
        let depth = c.undo_depth();
        c.begin_block();
        let result = apply_in(c, prepared, &pieces, to);
        c.end_block();
        if let Err(e) = result {
            // what was done is taken back; nothing is when the first edit was refused
            if c.undo_depth() > depth {
                c.undo();
            }
            return Err(e.to_string());
        }
        Ok(())
    })
}

fn apply_in(
    c: &mut CommandContext<'_>,
    prepared: &Prepared,
    pieces: &[String],
    to: OutputTo,
) -> CommandResult {
    let from = match to {
        OutputTo::NextLine => {
            let (at, prefix) = after_input(c, prepared)?;
            c.move_caret(at, false)?;
            let mut from = at;
            if !prefix.is_empty() {
                c.insert_text(prefix)?;
                from = c.caret();
            }
            from
        }
        _ => match &prepared.source {
            Source::Box { selection, .. } => {
                if c.cursor().selection() != Some(*selection) {
                    return Err(CommandError::Refused("the box selection changed"));
                }
                c.delete_selection()?;
                c.caret()
            }
            Source::Range(from, to) => {
                c.select_range(*from, *to)?;
                *from
            }
            Source::Whole => {
                let end = c.doc().end_pos()?;
                c.select_range(Pos::new(0, 0), end)?;
                Pos::new(0, 0)
            }
        },
    };
    let mut inserted = false;
    for piece in pieces.iter().filter(|piece| !piece.is_empty()) {
        c.insert_text(piece)?;
        inserted = true;
    }
    // no result: the input is replaced by nothing
    if !inserted && to == OutputTo::Replace {
        c.delete_selection()?;
    }
    let end = c.caret();
    if end != from {
        c.select_range(from, end)?;
    }
    Ok(())
}

/// Where 次の行に入れる puts the result: after the line where the input ends (for the whole document, its last line),
/// and the line break that line lacks when it is the last one of the document and has text.
fn after_input(c: &CommandContext<'_>, prepared: &Prepared) -> CommandResult<(Pos, &'static str)> {
    let doc = c.doc();
    let target = match &prepared.source {
        Source::Whole => doc.line_count().saturating_sub(1),
        // a selection that ends at the start of a line ends with the line break before it
        Source::Range(from, to) if to.col == 0 && to.line > from.line => to.line - 1,
        Source::Range(_, to) => to.line,
        Source::Box { selection, .. } => selection.1.line,
    };
    let has_break = doc.line_eol(target)?.is_some();
    let len = doc.line_len(target)?.unwrap_or(0);
    let eol = prepared.eol.as_str();
    Ok(if has_break {
        (Pos::new(target + 1, 0), "")
    } else if len > 0 {
        (Pos::new(target, len), eol)
    } else {
        (Pos::new(target, 0), "")
    })
}
