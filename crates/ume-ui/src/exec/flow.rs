//! What 外部コマンド実行 does once it has a command line and the choices: `CEditView::ExecCmd`
//! (`sakura_core/view/CEditView_ExecCmd.cpp`) and `CViewCommander::Command_EXECCOMMAND`
//! (`sakura_core/cmd/CViewCommander_Macro.cpp`).
//!
//! Differences from Sakura, all for a window that stays usable and a document that is read in
//! ranges:
//!
//! * The process runs in the background. A box with 中断 stands for Sakura's 「コマンド実行中・・・」
//!   dialog ([`super::running`]); it covers the view while the output goes into the document, and
//!   is a small box in the corner when the output goes to the output panel.
//! * The selection, or the whole document, is read by lines while the process runs and sent to its
//!   standard input in pieces; Sakura writes a temporary file first.
//! * The whole output is one undo step. Sakura inserts each piece by itself.
//! * The words that Sakura writes around the output use the line breaks of the document (Sakura
//!   writes CR LF).
//! * Without a folder the command runs in the folder of the file in the window, as in Sakura,
//!   where loading or saving a file moves the current folder to the file's (`CDocEditor`); a
//!   document without a file uses the folder of this program.
//! * A box selection is deleted when the output goes into the window, and the output is inserted
//!   at the caret once.
//! * stdout and stderr are one stream, as in Sakura.

use std::path::PathBuf;
use std::time::Duration;

use gpui::{AsyncWindowContext, Context, PromptLevel, WeakEntity};
use ume_commands::command::Arg;
use ume_commands::convert::datetime::{
    Clock, DEFAULT_DATE_PICTURE, DEFAULT_TIME_PICTURE, SystemClock, format_date, format_time,
};
use ume_core::buffer::{Document, Pos};
use ume_core::encoding::Eol;
use ume_core::exec::{
    ExecFlags, OutputDecoder, exit_code_text, feed_text, header_text, limit_command, stopped_text,
};
use ume_platform::exec::{Event, Exit, Spec, Stopper, start, stdin_channel};
use ume_plugin::Value;
use ume_plugin::functions;
use ume_plugin::host::Editor;

use crate::file_ops::ask::tell;
use crate::view::EditorView;

/// How often the output is looked at when none came (`MsgWaitForMultipleObjects( ..., 20, ...)`
/// in Sakura waits 20 ms).
const POLL: Duration = Duration::from_millis(15);

/// How much text is put in the window at a time.
const BATCH_BYTES: usize = 256 * 1024;

/// What to run.
#[derive(Clone, Debug)]
pub(crate) struct Request {
    pub command: String,
    pub flags: ExecFlags,
    /// The folder of the dialog or of the macro; used when `flags` has 0x200.
    pub folder: Option<String>,
}

/// What is read from the window before the process starts.
struct Prepared {
    command: String,
    editable: bool,
    selection: Option<(Pos, Pos)>,
    box_text: Option<String>,
    doc: Document,
    bom: bool,
    doc_dir: Option<PathBuf>,
    eol: Eol,
}

/// `CSakuraEnvironment::ExpandParameter` on `text` (`$F` is the file name, `$$` is `$`). A
/// command is kept as it is when the expansion is not available.
fn expand(view: &mut EditorView, text: &str, cx: &mut Context<EditorView>) -> String {
    let mut env = view.host_env("", cx);
    let answer = view.run_with(cx, |command_cx| {
        functions::call(
            &mut Editor {
                cx: command_cx,
                env: &mut env,
            },
            "ExpandParameter",
            &[Value::from(text)],
        )
    });
    view.finish_host_env(env, cx);
    match answer {
        Some(Ok(value)) => value.as_str().unwrap_or(text).to_owned(),
        _ => text.to_owned(),
    }
}

fn prepare(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    command: &str,
) -> Option<Prepared> {
    view.update_in(cx, |view, _window, cx| {
        // `Command_EXECCOMMAND`: ExpandParameter into a buffer of 1024 characters
        let expanded = expand(view, command, cx);
        let command = limit_command(&expanded).to_owned();
        let is_box = view.cursor().is_box() && view.cursor().has_selection();
        let selection = view.cursor().selection();
        let box_text = if is_box {
            view.run_with(cx, |c| c.box_text()).ok().flatten()
        } else {
            None
        };
        let shared = view.document().read(cx);
        Prepared {
            command,
            editable: !shared.view_mode(),
            selection: if is_box { None } else { selection },
            box_text,
            doc: shared.doc().clone(),
            bom: shared.file().bom,
            doc_dir: shared
                .path()
                .and_then(|p| p.parent())
                .filter(|p| p.is_dir())
                .map(PathBuf::from),
            eol: shared.new_line(),
        }
    })
    .ok()
}

/// The date and time as the extra information shows them.
fn now_texts() -> (String, String) {
    let now = SystemClock.now();
    (
        format_date(&now, DEFAULT_DATE_PICTURE),
        format_time(&now, DEFAULT_TIME_PICTURE),
    )
}

/// Where the output goes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    /// The document, at the caret.
    Window,
    /// The output panel of macros.
    Panel,
}

/// Puts `text` in the target. `false` when it cannot be (the window is gone, the document
/// refuses the edit).
fn put(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    target: Target,
    text: &str,
) -> bool {
    if text.is_empty() {
        return true;
    }
    view.update_in(cx, |view, _window, cx| match target {
        Target::Window => view
            .run_with(cx, |command_cx| command_cx.insert_text(text))
            .is_ok(),
        Target::Panel => {
            view.macros.output.trace(text, false);
            cx.notify();
            true
        }
    })
    .unwrap_or(false)
}

/// `Command_EXECCOMMAND` and `CEditView::ExecCmd`: runs the command and waits for it, with the
/// window usable. Mistakes are told to the user, as in Sakura.
pub(crate) async fn run_command(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    request: Request,
) {
    let Some(prepared) = prepare(view, cx, &request.command) else {
        return;
    };
    let flags = request.flags;
    let to_window = flags.to_window(prepared.editable);
    let capture = flags.get_stdout();
    let had_selection = prepared.selection.is_some() || prepared.box_text.is_some();

    // the folder: the one given when asked for, else the folder of the file
    let folder = if flags.uses_cur_dir() {
        request.folder.filter(|f| !f.is_empty()).map(PathBuf::from)
    } else {
        None
    }
    .or_else(|| prepared.doc_dir.clone());

    let mut spec = Spec::new(prepared.command.clone());
    spec.folder = folder;
    spec.capture = capture;
    spec.unicode_shell = flags.output_encoding() == ume_core::encoding::Encoding::Utf16Le;
    let sender = if flags.send_stdin() {
        let (tx, rx) = stdin_channel();
        spec.stdin = Some(rx);
        Some(tx)
    } else {
        None
    };

    let mut run = match start(spec) {
        Ok(run) => run,
        Err(error) => {
            // STR_EDITVIEW_EXECCMD_ERR, with the command line
            let message = format!(
                "コマンド実行は失敗しました。\n\n{}\n\n{error}",
                prepared.command
            );
            tell(cx, PromptLevel::Warning, &message, None).await;
            return;
        }
    };

    // the selection (else the document) goes to the standard input while the process runs
    if let Some(tx) = sender {
        let encoding = flags.input_encoding();
        let bom = prepared.bom;
        let (doc, range) = match &prepared.box_text {
            Some(text) => (
                Document::from_utf8(text.clone().into_bytes()).unwrap_or_default(),
                None,
            ),
            None => (prepared.doc.clone(), prepared.selection),
        };
        std::thread::spawn(move || {
            // an error of the document ends the input; the process then sees the end of it
            let _ = feed_text(&doc, range, encoding, bom, |piece| tx.send(piece));
        });
    }

    // Sakura does not wait for a command whose output is not taken
    if !capture {
        return;
    }

    let stopper = run.stopper();
    let command_line = prepared.command.clone();
    let opened = view.update_in(cx, {
        let stopper = stopper.clone();
        move |view, window, cx| {
            view.open_exec_running(&command_line, stopper, to_window, window, cx);
        }
    });
    if opened.is_err() {
        stop_in_background(cx, stopper);
        return;
    }

    let target = if to_window {
        Target::Window
    } else {
        Target::Panel
    };
    let eol = if to_window {
        prepared.eol.as_str()
    } else {
        "\n"
    };
    let mut keep_going = true;
    let mut select_from = None;

    if to_window {
        // One undo step for all the output. A selection is replaced by the first piece; a box is
        // deleted; with the whole document going through the command and coming back, the
        // document is replaced as a whole (`DeleteData` after `SetSelectArea`, 2007.05.20 maru).
        let filter_all = !had_selection && flags.send_stdin();
        select_from = prepared.selection.map(|(from, _)| from);
        let begun = view
            .update_in(cx, |view, _window, cx| {
                view.run_with(cx, |c| {
                    c.begin_block();
                    let prepared = (|| -> ume_commands::command::Result {
                        if filter_all {
                            c.select_all()?;
                            c.delete_selection()?;
                        } else if c.cursor().is_box() && c.cursor().has_selection() {
                            c.delete_selection()?;
                        }
                        Ok(())
                    })();
                    if prepared.is_err() {
                        c.end_block();
                    }
                    prepared.is_ok()
                })
            })
            .unwrap_or(false);
        if !begun {
            stop_in_background(cx, stopper);
            close(view, cx, to_window, false, None);
            return;
        }
    }

    let extra = flags.extra_info(to_window);
    if extra {
        let (date, time) = now_texts();
        keep_going = put(
            view,
            cx,
            target,
            &header_text(&date, &time, &prepared.command, eol),
        );
    }

    let mut decoder = OutputDecoder::new(flags.output_encoding());
    let mut exit: Option<Exit> = None;
    while keep_going && exit.is_none() {
        let mut batch = String::new();
        loop {
            match run.poll(Duration::ZERO) {
                Some(Event::Output(bytes)) => {
                    batch.push_str(&decoder.push(&bytes));
                    if batch.len() >= BATCH_BYTES {
                        break;
                    }
                }
                Some(Event::Exited(done)) => {
                    exit = Some(done);
                    break;
                }
                None => break,
            }
        }
        let idle = batch.is_empty() && exit.is_none();
        keep_going = put(view, cx, target, &batch);
        if idle {
            cx.background_executor().timer(POLL).await;
        }
    }

    let Some(exit) = exit else {
        // the window went away, or the document refused the text: end the process
        stop_in_background(cx, stopper);
        close(view, cx, to_window, to_window, select_from);
        return;
    };

    // 最後の文字の出力, the message of the 中断, the exit code
    let mut tail = decoder.finish();
    if extra {
        if exit.stopped {
            tail.push_str(&stopped_text(eol));
        }
        tail.push_str(&exit_code_text(exit.code.unwrap_or(0), eol));
    }
    put(view, cx, target, &tail);
    close(view, cx, to_window, to_window, select_from);
}

/// Ends the process without blocking the window (stopping can take a moment).
fn stop_in_background(cx: &mut AsyncWindowContext, stopper: Stopper) {
    cx.background_executor()
        .spawn(async move { stopper.stop() })
        .detach();
}

/// Takes the box away and ends the undo step; what was inserted is selected when something was
/// selected before (`GetSelectionInfo().SetSelectArea(ptFrom .. caret)`).
fn close(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    modal: bool,
    end_block: bool,
    select_from: Option<Pos>,
) {
    view.update_in(cx, |view, window, cx| {
        if end_block {
            view.run_with(cx, |c| {
                c.end_block();
                if let Some(from) = select_from {
                    let end = c.caret();
                    let _ = c.select_range(from, end);
                }
            });
        }
        view.support.exec_running = None;
        if modal {
            window.focus(&view.focus_handle, cx);
        }
        cx.notify();
    })
    .ok();
}

/// `Command_EXECCOMMAND_DIALOG`: asks, remembers the answer, and runs it as `F_EXECMD` with the
/// choices of the dialog (which a key macro records as that call).
pub(crate) async fn run_dialog(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    let Ok(editable) = view.read_with(cx, |view, app| !view.document().read(app).view_mode())
    else {
        return;
    };
    let Ok((history, flags)) = cx.update(|_, app| (super::history(app), super::flags(app))) else {
        return;
    };
    let (sender, receiver) = async_channel::bounded(1);
    let opened = view.update_in(cx, move |view, window, cx| {
        view.open_exec_dialog(history, flags, editable, sender, window, cx);
    });
    if opened.is_err() {
        return;
    }
    // cancelling is "done": nothing runs
    let Some(choice) = receiver.recv().await.ok().flatten() else {
        return;
    };
    {
        let (command, cur_dir, flags) =
            (choice.command.clone(), choice.cur_dir.clone(), choice.flags);
        cx.update(|_, app| super::remember(app, &command, &cur_dir, flags))
            .ok();
    }
    // `HandleCommand( F_EXECMD, true, cmd, flags, dir )`: the folder is left out when empty
    let folder = (!choice.cur_dir.is_empty()).then(|| choice.cur_dir.clone());
    let mut args = vec![
        Arg::Text(choice.command.clone()),
        Arg::Int(i64::from(choice.flags.0)),
    ];
    args.extend(folder.clone().map(Arg::Text));
    view.update(cx, |view, _| view.record_call("F_EXECMD", &args))
        .ok();
    run_command(
        view,
        cx,
        Request {
            command: choice.command,
            flags: choice.flags,
            folder,
        },
    )
    .await;
}
