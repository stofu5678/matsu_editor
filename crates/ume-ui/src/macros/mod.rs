//! Macros in the window: recording and playing key macros, and running macro files.
//!
//! The pieces that do not need a window are in `ume-commands` (`key_macro`: the recorder, the
//! `.mac` format) and `ume-plugin` (the host that runs a JavaScript or Python macro as a child
//! process, and the bridge that answers its calls with the commands). This module is the glue
//! that needs the window: it hooks the recorder where commands run, asks for files with the
//! native dialogs, and runs a macro's calls on the UI thread, one at a time.
//!
//! # Sakura's behavior (`sakura_core/cmd/CViewCommander_Macro.cpp`, `CViewCommander.cpp`)
//!
//! * `F_RECKEYMACRO` toggles recording. Starting clears the key macro buffer. Every command that
//!   `CSMacroMgr::CanFuncIsKeyMacro` allows is appended in `HandleCommand`, before it runs.
//!   `F_EXECEXTMACRO` is appended by itself after the file is chosen, and it pauses the
//!   recording while the macro runs.
//! * `F_SAVEKEYMACRO` and `F_LOADKEYMACRO` stop the recording first. Loading only remembers
//!   the file name; the file is read when `F_EXECKEYMACRO` runs it (any macro file may be
//!   named, not only `.mac`). Only a key macro can be saved.
//! * `F_EXECKEYMACRO` stops a recording and then plays. Each call is its own undo step
//!   (`SetUndoBuffer(true)` before the macro closes the block that `HandleCommand` opened).
//!
//! # Differences from Sakura
//!
//! * Stopping a recording does not write `RecKey.mac` into the macro folder (Sakura does and
//!   remembers that file). The recorded macro stays in memory until the editor closes; the
//!   macro folder is a setting that comes with the settings PR. `F_SAVEKEYMACRO` saves it.
//! * Text from the IME is recorded as one `Char` per character, like typed characters.
//! * The user can still type while a script runs; Sakura blocks the window. Both write to the
//!   same document, one call at a time.
//! * A script is aborted from the panel's button or by [`EditorView::cancel_macro`]; Sakura
//!   has no way to stop a macro.

mod output;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::{AnyElement, Context, Entity, PathPromptOptions, Window};
use ume_commands::command::{Arg, CommandError};
use ume_commands::key_macro::{KeyMacro, KeyMacroRecorder, Step};
use ume_commands::macro_names;
use ume_commands::registry::Registry;
use ume_commands::sakura::FUNCTIONS;
use ume_plugin::bridge;
use ume_plugin::session::{self, HostEvent, Language, MacroConfig, Session, Sink, Stream};

use crate::view::EditorView;
use output::PanelActions;

pub(crate) use output::{Kind, Output};

/// `F_WCHAR`, the function of a typed character (`Char` in a macro).
pub(crate) const F_WCHAR: u16 = 30200;
/// `F_EXECEXTMACRO`.
pub(crate) const F_EXECEXTMACRO: u16 = 31254;

/// The key macro that `F_EXECKEYMACRO` plays.
#[derive(Clone, Debug)]
pub(crate) enum CurrentMacro {
    /// What the last recording produced.
    Recorded(KeyMacro),
    /// A file that `F_LOADKEYMACRO` named; it is read when it runs.
    File(PathBuf),
}

/// What kind of macro a file is, by its extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileKind {
    /// A key macro (Sakura's `.mac` text).
    KeyMacro,
    Script(Language),
    Unknown,
}

pub(crate) fn file_kind(path: &Path) -> FileKind {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if ext.as_deref() == Some("mac") {
        FileKind::KeyMacro
    } else if let Some(language) = Language::of_file(path) {
        FileKind::Script(language)
    } else {
        FileKind::Unknown
    }
}

struct Running {
    session: Session,
    name: String,
    timeout_text: String,
}

/// What an `EditorView` keeps about macros.
#[derive(Default)]
pub(crate) struct MacroState {
    recorder: KeyMacroRecorder,
    current: Option<CurrentMacro>,
    /// The calls of a key macro that have not run yet. A call that starts a script waits for it.
    queue: VecDeque<Step>,
    /// How many calls of the queue have run.
    played: usize,
    /// A macro is running: the commands it runs are not recorded.
    suspended: bool,
    running: Option<Running>,
    pub(crate) output: Output,
}

impl MacroState {
    pub(crate) fn is_recording(&self) -> bool {
        self.recorder.is_recording()
    }

    fn busy(&self) -> bool {
        self.running.is_some() || !self.queue.is_empty()
    }
}

fn function_code(name: &str) -> Option<u16> {
    FUNCTIONS.iter().find(|f| f.name == name).map(|f| f.code)
}

fn function_name(code: u16) -> String {
    FUNCTIONS
        .iter()
        .find(|f| f.code == code)
        .map_or_else(|| code.to_string(), |f| f.name.to_owned())
}

impl EditorView {
    pub fn is_recording_key_macro(&self) -> bool {
        self.macros.is_recording()
    }

    /// A key macro or a script is running.
    pub fn is_macro_running(&self) -> bool {
        self.macros.busy()
    }

    /// What the output panel shows, one line each.
    pub fn macro_output_text(&self) -> String {
        self.macros.output.text()
    }

    // ---- recording ----

    /// Called where a command is run (a key, the menu, typed text), before it runs, like
    /// `HandleCommand` in Sakura. `name` is the command's Sakura function.
    pub(crate) fn record_call(&mut self, name: &str, args: &[Arg]) {
        if !self.macros.recorder.is_recording() || self.macros.suspended {
            return;
        }
        if let Some(code) = function_code(name) {
            self.record_code(code, args);
        }
    }

    fn record_code(&mut self, code: u16, args: &[Arg]) {
        // `F_EXECEXTMACRO` is recorded by itself, once the file is known.
        if self.macros.recorder.is_recording() && !self.macros.suspended && code != F_EXECEXTMACRO {
            self.macros.recorder.record(code, args);
        }
    }

    /// Text that came in without a command (the IME's result): recorded as typed characters.
    pub(crate) fn record_typed_text(&mut self, text: &str) {
        if !self.macros.recorder.is_recording() {
            return;
        }
        for c in text.chars().filter(|c| !matches!(c, '\r' | '\n')) {
            self.record_code(F_WCHAR, &[Arg::Char(c)]);
        }
    }

    /// `F_RECKEYMACRO`: starts recording, or stops it and keeps what was recorded.
    pub(crate) fn toggle_key_macro_recording(&mut self, cx: &mut Context<Self>) {
        if self.macros.recorder.is_recording() {
            self.stop_key_macro_recording();
        } else {
            self.macros.recorder.start();
        }
        cx.notify();
    }

    fn stop_key_macro_recording(&mut self) {
        if self.macros.recorder.is_recording() {
            let recorded = self.macros.recorder.stop();
            self.macros.current = Some(CurrentMacro::Recorded(recorded));
        }
    }

    // ---- messages ----

    /// For a UI command that fails at once: reports it like [`Self::macro_failed`] and gives the
    /// error to return, so that `run_function` says `false` and a playing key macro stops.
    pub(crate) fn macro_error(
        &mut self,
        message: String,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> CommandError {
        self.macro_failed(message.clone(), window, cx);
        CommandError::BadArgs(message)
    }

    /// Tells the user about a failure: in the panel, and in the status bar.
    pub(crate) fn macro_failed(
        &mut self,
        message: String,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        self.macros.output.show();
        self.macros.output.push(Kind::Note, message.clone());
        // The command that reported it overwrites the notice when it returns, so this waits.
        cx.defer_in(window, move |this, _, cx| {
            this.notice = message.lines().next().map(str::to_owned);
            cx.notify();
        });
        cx.notify();
    }

    // ---- key macro files ----

    /// `F_SAVEKEYMACRO`: asks for a file and saves the key macro in it.
    pub(crate) fn ask_key_macro_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_key_macro_recording();
        let dir = self.dialog_directory(cx);
        let answer = cx.prompt_for_new_path(&dir, Some("RecKey.mac"));
        cx.spawn_in(window, async move |this, cx| {
            let answer = answer.await;
            this.update_in(cx, |this, window, cx| match answer {
                Ok(Ok(Some(path))) => {
                    if let Err(message) = this.save_key_macro_to(&path) {
                        this.macro_failed(message, window, cx);
                    }
                }
                Ok(Err(error)) => this.macro_failed(dialog_error(&error), window, cx),
                _ => {}
            })
            .ok();
        })
        .detach();
    }

    /// Writes the key macro to `path`. Only a key macro can be saved (Sakura:
    /// `STR_ERR_CEDITVIEW_CMD26`).
    pub(crate) fn save_key_macro_to(&self, path: &Path) -> Result<(), String> {
        let key_macro = match &self.macros.current {
            Some(CurrentMacro::Recorded(key_macro)) => key_macro.clone(),
            None => KeyMacro::new(),
            Some(CurrentMacro::File(file)) if file_kind(file) == FileKind::KeyMacro => {
                KeyMacro::load(file).map_err(|e| format!("{}: {e}", file.display()))?
            }
            Some(CurrentMacro::File(_)) => {
                return Err(
                    "保存可能なマクロがありません。キーボードマクロ以外は保存できません。".into(),
                );
            }
        };
        key_macro.save(path).map_err(|e| {
            format!(
                "マクロファイルを作成できませんでした。{}: {e}",
                path.display()
            )
        })
    }

    /// `F_LOADKEYMACRO`: asks for a file and makes it the key macro. Like Sakura, it is read
    /// when it runs.
    pub(crate) fn ask_key_macro_load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_key_macro_recording();
        let answer = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("読み込み".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let answer = answer.await;
            this.update_in(cx, |this, window, cx| match answer {
                Ok(Ok(Some(mut paths))) => {
                    if let Some(path) = paths.pop() {
                        this.macros.current = Some(CurrentMacro::File(path));
                    }
                }
                Ok(Err(error)) => this.macro_failed(dialog_error(&error), window, cx),
                _ => {}
            })
            .ok();
        })
        .detach();
    }

    /// `F_EXECKEYMACRO`: stops a recording, then plays the key macro.
    pub(crate) fn exec_key_macro(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), CommandError> {
        self.stop_key_macro_recording();
        if self.macros.busy() {
            return Err(self.macro_error("マクロを実行中です".into(), window, cx));
        }
        let result = match self.macros.current.clone() {
            // Nothing to play; Sakura greys the item out.
            None => Ok(()),
            Some(CurrentMacro::Recorded(key_macro)) => {
                self.start_key_macro(key_macro.steps().to_vec(), window, cx);
                Ok(())
            }
            Some(CurrentMacro::File(path)) => self.run_macro_file(&path, window, cx),
        };
        result.map_err(|message| self.macro_error(message, window, cx))
    }

    // ---- macro files ----

    /// `F_EXECEXTMACRO` without a file: asks for one.
    pub(crate) fn ask_macro_to_run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let answer = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("実行".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let answer = answer.await;
            this.update_in(cx, |this, window, cx| match answer {
                Ok(Ok(Some(mut paths))) => {
                    if let Some(path) = paths.pop() {
                        // The error is reported by `exec_ext_macro`.
                        this.exec_ext_macro(&path, window, cx).ok();
                    }
                }
                Ok(Err(error)) => this.macro_failed(dialog_error(&error), window, cx),
                _ => {}
            })
            .ok();
        })
        .detach();
    }

    /// `F_EXECEXTMACRO` with a file: records the call if a key macro is being recorded (and
    /// pauses the recording while the macro runs), then runs the file. Errors go to the panel.
    pub(crate) fn exec_ext_macro(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), CommandError> {
        // A key macro that is playing may name a macro file: its calls go on in the middle.
        if self.macros.running.is_some() {
            return Err(self.macro_error("マクロを実行中です".into(), window, cx));
        }
        if self.macros.recorder.is_recording() && !self.macros.suspended {
            self.macros.recorder.record(
                F_EXECEXTMACRO,
                &[Arg::Text(path.to_string_lossy().into_owned())],
            );
        }
        self.run_macro_file(path, window, cx)
            .map_err(|message| self.macro_error(message, window, cx))
    }

    /// Runs a macro file by its extension: `.mac` through the key macro player, `.js` and
    /// `.py` through the host. The `Err` says what to show.
    fn run_macro_file(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        match file_kind(path) {
            FileKind::KeyMacro => {
                let key_macro =
                    KeyMacro::load(path).map_err(|e| format!("{}: {e}", path.display()))?;
                self.start_key_macro(key_macro.steps().to_vec(), window, cx);
                Ok(())
            }
            FileKind::Script(_) => self.start_script(path, window, cx),
            FileKind::Unknown => Err(format!(
                "{}: マクロの拡張子は .mac、.js、.py のどれかにしてください",
                path.display()
            )),
        }
    }

    // ---- playing a key macro ----

    fn start_key_macro(&mut self, steps: Vec<Step>, window: &mut Window, cx: &mut Context<Self>) {
        // A macro file named by a key macro that is playing runs in its place, and the rest of
        // the outer macro follows.
        if self.macros.queue.is_empty() {
            self.macros.played = 0;
        }
        for step in steps.into_iter().rev() {
            self.macros.queue.push_front(step);
        }
        self.macros.suspended = true;
        self.drain_key_macro(window, cx);
    }

    /// Runs the calls of the key macro one after the other, as the keys would. It stops at the
    /// first call that fails (`CKeyMacroMgr::ExecKeyMacro`), and waits while a script runs.
    fn drain_key_macro(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        while self.macros.running.is_none() {
            let Some(step) = self.macros.queue.pop_front() else {
                break;
            };
            self.macros.played += 1;
            // Only the declared arguments are passed (as `KeyMacro::play` does).
            let declared = macro_names::by_code(step.code).map_or(usize::MAX, |f| f.args.len());
            let args = &step.args[..step.args.len().min(declared)];
            self.notice = None;
            // The result says whether the call failed; a notice may also come with a success.
            if !self.run_function(step.code, args, window, cx) {
                let notice = self.notice.clone().unwrap_or_default();
                self.macros.queue.clear();
                let message = format!(
                    "キーマクロの {} 番目の呼び出し ({}) で止まりました: {notice}",
                    self.macros.played,
                    function_name(step.code)
                );
                self.macros.output.show();
                self.macros.output.push(Kind::Note, message);
                break;
            }
        }
        if self.macros.running.is_none() && self.macros.queue.is_empty() {
            self.macros.suspended = false;
        }
        cx.notify();
    }

    // ---- running a script ----

    fn start_script(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let document_path = self
            .document
            .read(cx)
            .path()
            .map(|p| p.display().to_string());
        let config = MacroConfig::for_script(path, document_path).map_err(|e| e.to_string())?;
        let timeout_text = format!("{} 秒", config.timeout.as_secs_f64());
        let (tx, rx) = async_channel::unbounded::<HostEvent>();
        let sink: Sink = Arc::new(move |event| {
            let _ = tx.try_send(event);
        });
        let session = session::start(config, sink).map_err(|e| e.to_string())?;
        self.macros.output.begin();
        self.macros
            .output
            .push(Kind::Note, format!("マクロ {name} を開始します"));
        self.macros.running = Some(Running {
            session,
            name,
            timeout_text,
        });
        self.macros.suspended = true;
        // The host's threads send events; each one is handled here, on the UI thread, in order.
        cx.spawn_in(window, async move |this, cx| {
            while let Ok(event) = rx.recv().await {
                let last = matches!(event, HostEvent::Finished(_));
                if this
                    .update_in(cx, |this, window, cx| this.on_host_event(event, window, cx))
                    .is_err()
                    || last
                {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
        Ok(())
    }

    fn on_host_event(&mut self, event: HostEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            HostEvent::Output { stream, text } => {
                let kind = match stream {
                    Stream::Stdout => Kind::Out,
                    Stream::Stderr => Kind::Err,
                };
                self.macros.output.push(kind, text);
            }
            HostEvent::Message(text) => {
                // One call at a time, on the UI thread, against the document on the screen.
                let reply = self.run_with(cx, |command_cx| {
                    bridge::handle_message(Registry::global(), command_cx, &text)
                });
                if let (Some(reply), Some(running)) = (reply, &self.macros.running) {
                    running.session.send(reply);
                }
            }
            HostEvent::Finished(finished) => {
                let running = self.macros.running.take();
                let (name, timeout_text) =
                    running.map_or_else(Default::default, |r| (r.name, r.timeout_text));
                let mut message = ume_plugin::describe(&finished, &timeout_text);
                // On Windows, `python` can be the Microsoft Store stub, which exits with 9009.
                if finished.exit_code == Some(9009) {
                    message.push_str("。Python が見つかりません。インストールを確かめてください");
                }
                self.macros.output.push(Kind::Note, message.clone());
                self.notice = Some(format!("マクロ {name}: {message}"));
                // A key macro that started this script goes on.
                self.drain_key_macro(window, cx);
            }
        }
        cx.notify();
    }

    /// Stops the macro that is running, and drops the rest of a key macro that waits for it.
    pub fn cancel_macro(&mut self, cx: &mut Context<Self>) {
        self.macros.queue.clear();
        match &self.macros.running {
            // `Finished` follows with `Cancelled`.
            Some(running) => running.session.cancel(),
            None => self.macros.suspended = false,
        }
        cx.notify();
    }

    // ---- the panel ----

    /// The output panel for the window to show under the text, while it is open.
    pub fn macro_panel(&self, view: Entity<EditorView>) -> Option<AnyElement> {
        let cancel = view.clone();
        self.macros.output.panel(PanelActions {
            running: self.macros.running.is_some(),
            on_cancel: Box::new(move |_, _, cx| {
                cancel.update(cx, |view, cx| view.cancel_macro(cx));
            }),
            on_close: Box::new(move |_, _, cx| {
                view.update(cx, |view, cx| {
                    view.macros.output.close();
                    cx.notify();
                });
            }),
        })
    }

    /// Where a file dialog starts: the folder of the document, else the current folder.
    fn dialog_directory(&self, cx: &Context<Self>) -> PathBuf {
        self.document
            .read(cx)
            .path()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .filter(|p| !p.as_os_str().is_empty())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

/// On Linux a dialog needs xdg-desktop-portal, which WSLg does not have.
fn dialog_error(error: &impl std::fmt::Display) -> String {
    format!("ファイルを選べません: {error}")
}

#[cfg(test)]
mod tests;
