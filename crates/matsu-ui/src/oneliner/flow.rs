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
//!
//! # A one-liner that sends to a generative AI (ADR-0030)
//!
//! A slot with `AI[nnn]=1` goes through the same flow with three more steps:
//!
//! 1. Before anything starts, when `AskBeforeSend[nnn]` is 1, the confirmation window (`ask_ai_confirm`) shows what
//!    would be sent: the same range as the input. 「送らない」, Escape or closing the window runs nothing and changes
//!    nothing (the status bar says so, and the log has a record that nothing was sent). 「送る」 with
//!    「次から聞かない」 writes `AskBeforeSend[nnn]=0`. The state is looked at again after the wait.
//! 2. The working folder is a new, empty folder for each run ([`workdir`]), removed when the run ends.
//! 3. The log tab (`crate::ai_log`) gets a record before the process starts, and the end of the run completes it,
//!    whatever the end is.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Local;
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

use crate::ai_confirm::Request as ConfirmRequest;
use crate::ai_log::{self, RecordHandle};
use crate::file_ops::ask::tell;
use crate::file_ops::start as spawn_flow;
use crate::view::EditorView;

use super::workdir::WorkDir;

/// How often the end of the process is looked for when it has not ended (as 外部コマンド実行 does).
const POLL: Duration = Duration::from_millis(15);

/// What to run: one line and where its result goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Job {
    pub line: String,
    pub output: OutputTo,
    /// The slot, when it sends to a generative AI (`AI[nnn]=1`). `None` for every other one-liner.
    pub ai: Option<AiSlot>,
}

/// What the flow needs of an AI one-liner's slot (ADR-0030).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AiSlot {
    /// The number of the slot, to write `AskBeforeSend` back.
    pub index: usize,
    /// The name shown in the confirmation window and the log.
    pub name: String,
    /// `AskBeforeSend[nnn]`: the confirmation window comes first.
    pub ask: bool,
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
            ai: one.ai.then(|| AiSlot {
                index,
                name: one.title().to_owned(),
                ask: one.ask_before_send,
            }),
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
    if !reserve(view, cx) {
        tell_busy(cx).await;
        return;
    }

    // An AI one-liner: the confirmation, the record in the log and its own working folder (ADR-0030).
    let began = Instant::now();
    let mut log = None;
    let mut workdir = None;
    if let Some(ai) = &job.ai {
        let Some(handle) = begin_ai(view, cx, &prepared, &job, ai, began).await else {
            return;
        };
        match WorkDir::create() {
            Ok(dir) => workdir = Some(dir),
            Err(error) => {
                let why = format!("作業用のフォルダを作れませんでした。\n{error}");
                end_log(cx, Some(&handle), failed_log(&why, began));
                release(view, cx);
                tell(
                    cx,
                    PromptLevel::Warning,
                    &failure_message(&job.line, &why),
                    None,
                )
                .await;
                return;
            }
        }
        log = Some(handle);
    }

    let request = Request {
        shell: prepared.shell.clone(),
        line: job.line.clone(),
        folder: match &workdir {
            Some(dir) => Some(dir.path().to_owned()),
            None => prepared.folder.clone(),
        },
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
            let why = start_error_text(&error);
            end_log(cx, log.as_ref(), failed_log(&why, began));
            release(view, cx);
            tell(
                cx,
                PromptLevel::Warning,
                &failure_message(&job.line, &why),
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
            view.support.oneliner_starting = false;
            view.open_exec_running(&line, stopper, true, window, cx);
            true
        }
    });
    match opened {
        Ok(true) => {}
        Ok(false) => {
            end_log(
                cx,
                log.as_ref(),
                failed_log(
                    "別のコマンドを実行中だったので、実行しませんでした。",
                    began,
                ),
            );
            release(view, cx);
            stop_in_background(cx, stopper);
            tell_busy(cx).await;
            return;
        }
        Err(_) => {
            stop_in_background(cx, stopper);
            end_log(
                cx,
                log.as_ref(),
                failed_log("文書が閉じられたので、実行しませんでした。", began),
            );
            return;
        }
    }

    let event = loop {
        if let Some(event) = run.poll(Duration::ZERO) {
            break event;
        }
        if view.read_with(cx, |_, _| ()).is_err() {
            // the editor is gone (the log may still be open): the run is ended and so is its record
            stop_in_background(cx, stopper);
            end_log(
                cx,
                log.as_ref(),
                failed_log("文書が閉じられたので、中断しました。", began),
            );
            return;
        }
        cx.background_executor().timer(POLL).await;
    };

    // The process is over: its working folder goes, and the log has the end, whatever it was.
    drop(workdir);
    if log.is_some() {
        end_log(cx, log.as_ref(), log_end(&event, began));
    }

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

/// The steps before the process of an AI one-liner (ADR-0030): asks when the slot says so, and puts the record in
/// the log. Returns the record, or `None` when nothing is to run: the reservation is released and the reason is said
/// (the status bar for a refusal, a message box for a failure).
async fn begin_ai(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    prepared: &Prepared,
    job: &Job,
    ai: &AiSlot,
    began: Instant,
) -> Option<RecordHandle> {
    let started_with = |input: String, input_size: Option<usize>| ai_log::Started {
        name: ai.name.clone(),
        command_line: job.line.clone(),
        input,
        input_size,
        started_at: Local::now(),
    };

    if ai.ask {
        let asked = view.update_in(cx, |view, window, cx| {
            confirm_request(prepared, job, ai)
                .map(|request| view.ask_ai_confirm(request, window, cx))
        });
        let answer = match asked {
            Ok(Ok(wait)) => wait.await,
            Ok(Err(why)) => {
                release(view, cx);
                tell(
                    cx,
                    PromptLevel::Warning,
                    &failure_message(&job.line, &why),
                    None,
                )
                .await;
                return None;
            }
            // the window is gone
            Err(_) => return None,
        };
        // After the wait the state is looked at again: the window, and the document that was read. 「次から聞かない」
        // is the user's own setting, so it is written even when the document changed meanwhile.
        let unchanged = view
            .update_in(cx, |view, _window, cx| {
                if answer.send && answer.dont_ask_again {
                    super::store::stop_asking(cx, ai.index, &job.line);
                }
                let shared = view.document().read(cx);
                shared.generation() == prepared.generation
                    && shared.version() == prepared.version
                    && (job.output == OutputTo::OutputWindow || can_edit(view, cx))
            })
            .ok()?;
        if !answer.send {
            // nothing runs; the log says that nothing was sent
            if let Some(handle) = begin_log(view, cx, started_with(String::new(), None)).await {
                end_log(
                    cx,
                    Some(&handle),
                    failed_log(
                        "送りませんでした。確認の窓で「送らない」が選ばれたか、窓が閉じられました。",
                        began,
                    ),
                );
            }
            release(view, cx);
            say(
                view,
                cx,
                "生成 AI に送らないので、ワンライナーを実行しませんでした。",
            );
            return None;
        }
        if !unchanged {
            release(view, cx);
            say(
                view,
                cx,
                "文書か設定が変わったので、ワンライナーを実行しませんでした。",
            );
            return None;
        }
    }

    // Approved: the text for the log is read now (not before the window, which must not wait for a large input),
    // from the snapshot that the input is read from.
    let input = input_of(prepared);
    let (text, size) = cx
        .background_executor()
        .spawn(async move { log_input(&input) })
        .await;
    let handle = begin_log(view, cx, started_with(text, size)).await;
    if handle.is_none() {
        release(view, cx);
        say(
            view,
            cx,
            "AI ログを開けないので、ワンライナーを実行しませんでした。",
        );
    }
    handle
}

/// What the confirmation window shows: the name, the command line and the same range that the input is read from
/// (a box: the text of the box).
fn confirm_request(prepared: &Prepared, job: &Job, ai: &AiSlot) -> Result<ConfirmRequest, String> {
    let boxed;
    let (doc, from, to) = match &prepared.source {
        Source::Whole => {
            let end = prepared.doc.end_pos().map_err(|e| e.to_string())?;
            (&prepared.doc, Pos::new(0, 0), end)
        }
        Source::Range(from, to) => (&prepared.doc, *from, *to),
        Source::Box { text, .. } => {
            boxed = Document::from_utf8(text.clone().into_bytes()).map_err(|e| e.to_string())?;
            let end = boxed.end_pos().map_err(|e| e.to_string())?;
            (&boxed, Pos::new(0, 0), end)
        }
    };
    ConfirmRequest::from_document(&ai.name, &job.line, doc, from, to).map_err(|e| e.to_string())
}

/// The head of the input for the log, up to the size the log keeps, and the size of the whole when it is more.
fn log_input(input: &Input) -> (String, Option<usize>) {
    let mut text = String::new();
    let mut total = 0usize;
    // an input that cannot be read ends here: the log has what was read
    let _ = input(&mut |piece| {
        total += piece.len();
        // only the part that still fits, cut on a character boundary; the size of the whole is counted anyway
        let room = ai_log::records::MAX_TEXT_BYTES.saturating_sub(text.len());
        if room > 0 {
            let mut end = piece.len().min(room);
            while !piece.is_char_boundary(end) {
                end -= 1;
            }
            text.push_str(&piece[..end]);
        }
        true
    });
    let size = (total > text.len()).then_some(total);
    (text, size)
}

/// Opens the log (behind the document) and adds the running record. `None` when the window has no workspace.
async fn begin_log(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    started: ai_log::Started,
) -> Option<RecordHandle> {
    let host = view
        .read_with(cx, |view, _| view.split_host.clone())
        .ok()??;
    cx.update(|window, app| {
        host.update(app, |workspace, cx| {
            ai_log::begin(workspace, started, window, cx)
        })
        .ok()
    })
    .ok()
    .flatten()
}

fn end_log(cx: &mut AsyncWindowContext, handle: Option<&RecordHandle>, finished: ai_log::Finished) {
    if let Some(handle) = handle {
        cx.update(|_, app| handle.finish(finished, app)).ok();
    }
}

/// A record of a run that did not happen or did not get a result: only what is known.
fn failed_log(why: &str, began: Instant) -> ai_log::Finished {
    ai_log::Finished {
        output: String::new(),
        stderr: why.to_owned(),
        exit_code: None,
        elapsed: began.elapsed(),
    }
}

/// What the log keeps of the end of a run: the answer, the standard error, the exit code. A run stopped by キャンセル
/// says so in the standard error's place, above what the process wrote.
fn log_end(event: &Event, began: Instant) -> ai_log::Finished {
    match event {
        Event::Failed(error) => failed_log(&run_error_text(error), began),
        Event::Finished(finished) => {
            let mut stderr = String::new();
            if finished.exit.stopped {
                stderr.push_str("キャンセルで中断しました。\n");
            }
            stderr.push_str(&finished.error);
            if finished.error_truncated {
                stderr.push_str("\n(以降は省略しました)\n");
            }
            ai_log::Finished {
                output: finished.output.concat(),
                stderr,
                exit_code: finished.exit.code,
                elapsed: began.elapsed(),
            }
        }
    }
}

/// Takes the place of the one command that takes the window, before the process is started (which waits): `false`
/// when a command has the box or another one-liner is starting. Released by [`release`], or by the box that takes
/// over.
fn reserve(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) -> bool {
    view.update(cx, |view, cx| {
        if view.support.exec_running.is_some() || view.support.oneliner_starting {
            return false;
        }
        view.support.oneliner_starting = true;
        cx.notify();
        true
    })
    .unwrap_or(false)
}

fn release(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    view.update(cx, |view, cx| {
        view.support.oneliner_starting = false;
        cx.notify();
    })
    .ok();
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
    // no result for the next line: not even the line break that the last line lacks
    if to == OutputTo::NextLine && pieces.iter().all(String::is_empty) {
        return Ok(());
    }
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
