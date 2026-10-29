//! Macros in the window: recording and playing key macros, and running macro files.
//!
//! The pieces that do not need a window are in `matsu-commands` (`key_macro`: the recorder, the
//! `.mac` format) and `matsu-plugin` (the host that runs a JavaScript, Python, Ruby or PHP macro as a child
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
//! * The file name that `F_EXECKEYMACRO` plays (`m_szKeyMacroFileName`) is kept in memory and
//!   shared by the windows, as in Sakura; it is not written to the settings file, so a new run
//!   starts without it (`RecKey.mac` stays in the macro folder; `F_LOADKEYMACRO` picks it).
//! * Stopping a recording writes `RecKey.mac` into the macro folder. Sakura leaves the name bare
//!   when the macro folder is empty, which means the current directory; here it is the folder of
//!   the settings file. Without a settings file the recording stays in the window's memory.
//! * Text from the IME is recorded as one `Char` per character, like typed characters.
//! * The user can still type while a script runs; Sakura blocks the window. Both write to the
//!   same document, one call at a time.
//! * A call that opens a dialog or a panel, or works in the background, is waited for: a script
//!   gets its answer, and a key macro goes on, when it is over (`wait.rs`). That includes a file
//!   in a new window, which Sakura (it starts another process) does not wait for.
//! * A script is aborted from the panel's button or by [`EditorView::cancel_macro`]; Sakura
//!   has no way to stop a macro.

mod host;
mod menu_call;
mod output;
pub(crate) mod plugin_call;
mod registered;
mod statements;
mod ui_calls;
pub(crate) mod wait;

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::{AnyElement, App, Context, Entity, Global, Window};
use matsu_commands::command::{Arg, CommandError};
use matsu_commands::key_macro::{KeyMacro, KeyMacroRecorder, Step};
use matsu_commands::macro_names;
use matsu_commands::registry::Registry;
use matsu_commands::sakura::FUNCTIONS;
use matsu_plugin::bridge;
use matsu_plugin::session::{
    self, EndReason, HostEvent, Language, MacroConfig, Session, Sink, Stream,
};

use crate::file_dialog::{self, Ask};
use crate::view::EditorView;
use output::PanelActions;

pub(crate) use output::{JumpContext, Kind, Output, Place};
pub(crate) use registered::table as registered_table;
pub(crate) use registered::{
    Auto, MacroList, run_auto_in, run_before_save, run_opened, run_opened_in_new, title_of,
};
pub(crate) use wait::{FlowGuard, follow, keep_box, track};

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

/// The file that `F_EXECKEYMACRO` plays, shared by all windows (Sakura: `m_szKeyMacroFileName` in
/// the shared data). Stopping a recording and `F_LOADKEYMACRO` set it. It is not saved in the
/// settings file, as in Sakura, so a new run of the editor starts without it.
#[derive(Default)]
struct SharedKeyMacro(Option<PathBuf>);

impl Global for SharedKeyMacro {}

fn shared_key_macro(cx: &App) -> Option<PathBuf> {
    cx.try_global::<SharedKeyMacro>()?.0.clone()
}

fn shared_key_macro_is_set(cx: &App) -> bool {
    cx.try_global::<SharedKeyMacro>()
        .is_some_and(|s| s.0.is_some())
}

fn set_shared_key_macro(cx: &mut App, path: Option<PathBuf>) {
    cx.set_global(SharedKeyMacro(path));
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

/// The file that holds the code of a script given as text. It lives in a directory of its own that
/// only this user can use, made new for it (`matsu_plugin::boot::private_dir`: another user cannot
/// plant a directory or swap the file), and both are removed when this is dropped: the script
/// ended, or its window was closed.
struct TempScript {
    _dir: matsu_plugin::boot::BootDir,
    path: PathBuf,
}

impl TempScript {
    fn create(ext: &str, code: &str) -> Result<Self, String> {
        let dir = matsu_plugin::boot::private_dir(None)
            .map_err(|e| format!("一時フォルダーを作れません: {e}"))?;
        let path = dir.path().join(format!("macro.{ext}"));
        std::fs::write(&path, code).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self { _dir: dir, path })
    }
}

/// What the queue of a playing key macro holds.
enum Item {
    Call(Step),
    /// The macro file that began before this ends here.
    Leave,
}

/// The deepest nesting of macro files (a `.mac` that names a `.mac`). Sakura has no limit.
const MAX_NESTING: usize = 16;

/// How many events from the host wait for the window at most.
const EVENT_BACKLOG: usize = 256;

/// Returns to the executor once, so that other tasks (painting, input) run in between.
async fn yield_now() {
    let mut yielded = false;
    std::future::poll_fn(|task| {
        if yielded {
            std::task::Poll::Ready(())
        } else {
            yielded = true;
            task.waker().wake_by_ref();
            std::task::Poll::Pending
        }
    })
    .await;
}

struct Running {
    session: Session,
    name: String,
    /// The macro file (for `$M` of `ExpandParameter`).
    path: PathBuf,
    timeout_text: String,
}

/// What an `EditorView` keeps about macros.
#[derive(Default)]
pub(crate) struct MacroState {
    recorder: KeyMacroRecorder,
    current: Option<CurrentMacro>,
    /// The calls of a key macro that have not run yet. A call that starts a script waits for it.
    queue: VecDeque<Item>,
    /// The macro files that are being played, outermost first (for the cycle check).
    active: Vec<PathBuf>,
    /// The depth of the open undo block when each of those files began (the reference count
    /// `CMacroBeforeAfter::ExecKeyMacroBefore` remembers); the file puts it back when it ends.
    undo_depths: Vec<Option<usize>>,
    /// `drain_key_macro` is on the stack: a macro file named by a call joins the queue and the
    /// loop that is running goes on with it, so playback never recurses.
    draining: bool,
    /// How many calls of the queue have run.
    played: usize,
    /// A macro is running: the commands it runs are not recorded.
    suspended: bool,
    running: Option<Running>,
    /// A plugin is in a call on this window (`plugin_call`): the calls of plugins go one at a time.
    plugin_running: bool,
    /// The file that holds the code of a script given as text (`F_EXECEXTMACRO` with a type); it is
    /// removed when the script ends.
    temp_script: Option<TempScript>,
    /// Counts the scripts that were started, so that the answer of a dialog that was open when
    /// its macro ended is not sent to the next one.
    serial: u64,
    /// The wait for a call that finishes later (`wait.rs`): the number of the wait that is on,
    /// if any. A key macro does not go on, and a script gets no answer, until it ends.
    waiting: Option<u64>,
    /// Counts the waits that began, so that a wait that was stopped can tell.
    epoch: u64,
    /// What the macro functions keep between two calls (cookies, the draw switch).
    host_state: matsu_plugin::host::HostState,
    /// The registered macros (登録マクロ): what was read, and the path of the save that is running one.
    registered: registered::State,
    pub(crate) output: Output,
    /// A longer (or shorter) timeout for the next scripts, for the tests that run on a busy machine.
    #[cfg(test)]
    pub(crate) script_timeout: Option<std::time::Duration>,
}

impl MacroState {
    pub(crate) fn is_recording(&self) -> bool {
        self.recorder.is_recording()
    }

    /// `CSMacroMgr::IsSaveOk`: the macro that `F_EXECKEYMACRO` plays can be saved, which only a key
    /// macro can (a recording or a key macro file; a script cannot).
    pub(crate) fn can_save_key_macro(&self, cx: &App) -> bool {
        // By reference: the menus ask this often, and a recording can be long.
        if matches!(self.current, Some(CurrentMacro::Recorded(_))) {
            return true;
        }
        let shared = cx.try_global::<SharedKeyMacro>().and_then(|s| s.0.as_ref());
        match (shared, &self.current) {
            (Some(path), _) | (None, Some(CurrentMacro::File(path))) => {
                file_kind(path) == FileKind::KeyMacro
            }
            _ => false,
        }
    }

    /// The macro that `F_EXECKEYMACRO` plays: the file that any window made the key macro, else
    /// what this window recorded without a folder to write it to.
    fn current_macro(&self, cx: &App) -> Option<CurrentMacro> {
        shared_key_macro(cx)
            .map(CurrentMacro::File)
            .or_else(|| self.current.clone())
    }

    /// The macro that `F_SAVEKEYMACRO` saves: what this window just recorded (Sakura keeps it in
    /// memory, even when writing `RecKey.mac` failed), else [`Self::current_macro`].
    fn saved_macro(&self, cx: &App) -> Option<CurrentMacro> {
        match &self.current {
            Some(recorded @ CurrentMacro::Recorded(_)) => Some(recorded.clone()),
            _ => self.current_macro(cx),
        }
    }

    /// There is a macro for `F_EXECKEYMACRO` to play (Sakura: `m_szKeyMacroFileName` is set).
    pub(crate) fn has_key_macro(&self, cx: &App) -> bool {
        shared_key_macro_is_set(cx) || self.current.is_some()
    }

    /// A macro is playing or running: the commands it calls are not recorded.
    pub(crate) fn is_suspended(&self) -> bool {
        self.suspended
    }

    /// The file a save will write to while the macro that runs before it is running
    /// (`CDocFile::GetSaveFilePath`); `None` otherwise.
    pub(crate) fn save_path(&self) -> Option<&Path> {
        self.suspended
            .then(|| self.registered.save_path())
            .flatten()
    }

    fn busy(&self) -> bool {
        self.running.is_some()
            || self.plugin_running
            || !self.queue.is_empty()
            || self.waiting.is_some()
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
        // Known difference: the replay types the break as Enter, which copies the indentation
        // of the line, while the text was inserted as it is. The exact form is `InsText`, which
        // has no command yet (it runs through the bridge's fallback); switch to it then.
        // A line break is the Enter key (`'\r'`), and CR LF is one break.
        let text = text.replace("\r\n", "\n").replace('\n', "\r");
        for c in text.chars() {
            self.record_code(F_WCHAR, &[Arg::Char(c)]);
        }
    }

    /// An IME composition ends with its text kept in the document (the IME unmarks it, or a key
    /// or a click interrupts it): record that text as typed. Call before `marked` is cleared.
    pub(crate) fn record_composition(&mut self, cx: &gpui::App) {
        if !self.macros.recorder.is_recording() {
            return;
        }
        let Some(marked) = self.marked.clone() else {
            return;
        };
        let text = self
            .document
            .read(cx)
            .doc()
            .line(marked.line)
            .ok()
            .flatten()
            .and_then(|line| line.text.get(marked.range.clone()).map(str::to_owned));
        if let Some(text) = text {
            self.record_typed_text(&text);
        }
    }

    /// `F_RECKEYMACRO`: starts recording, or stops it and keeps what was recorded.
    ///
    /// Stopping writes `RecKey.mac`; the message says why that failed, if it did.
    pub(crate) fn toggle_key_macro_recording(&mut self, cx: &mut Context<Self>) -> Option<String> {
        let failure = if self.macros.recorder.is_recording() {
            self.stop_key_macro_recording(cx)
        } else {
            self.macros.recorder.start();
            None
        };
        cx.notify();
        failure
    }

    /// Ends a recording like `Command_RECKEYMACRO` does: writes `RecKey.mac` in the macro folder
    /// and makes it the file that `F_EXECKEYMACRO` plays in every window. The file name is set
    /// before the write, as in Sakura, so a write that failed leaves the name of a file that may
    /// not be there (or an older recording); playing then says so. Without a folder (no settings
    /// file) the recording stays in this window's memory. Returns why the write failed.
    fn stop_key_macro_recording(&mut self, cx: &mut Context<Self>) -> Option<String> {
        if !self.macros.recorder.is_recording() {
            return None;
        }
        let recorded = self.macros.recorder.stop();
        let file = file_dialog::key_macro_file(cx);
        set_shared_key_macro(cx, file.clone());
        let failure = file.and_then(|path| {
            recorded.save(&path).err().map(|error| {
                format!(
                    "マクロファイルを作成できませんでした。{}: {error}",
                    path.display()
                )
            })
        });
        self.macros.current = Some(CurrentMacro::Recorded(recorded));
        failure
    }

    /// A macro file begins: returns the depth of the undo block, to put back when the file ends
    /// (the reference count `CMacroBeforeAfter::ExecKeyMacroBefore` remembers). The command that
    /// runs the file has left the block already (`F_EXECEXTMACRO` stores it first, "再帰処理対策"
    /// in `CViewCommander::HandleCommand`). `None`: a composition of the user's is open and its
    /// block is not the macro's.
    fn enter_undo_block(&mut self, cx: &mut Context<Self>) -> Option<usize> {
        (!self.composing).then(|| self.document.read(cx).model().history().depth())
    }

    /// A macro file ends: puts the undo block back to the depth it had when the file began. The
    /// blocks that it entered with `AddRefUndoBuffer()` and did not leave are left, and the ones
    /// that it left too often are entered again (Sakura: `CMacroBeforeAfter::ExecKeyMacroAfter`
    /// calls `SetRefCount` with the count of `ExecKeyMacroBefore`, or stores the block when that
    /// was 0). The outer macro goes on in its own block.
    fn leave_undo_block(&mut self, depth: Option<usize>, cx: &mut Context<Self>) {
        let Some(depth) = depth else { return };
        let cursor = &self.cursor;
        self.document.update(cx, |shared, _| {
            shared.parts_mut().0.restore_block_depth(depth, cursor);
        });
    }

    /// Leaves the undo blocks that a macro entered with `AddRefUndoBuffer()` and did not leave
    /// (Sakura: `CMacroBeforeAfter::ExecKeyMacroAfter` puts the reference count back to what it
    /// was before the macro). The edits in them become an undo step. Not a command, so the view
    /// stays where the macro put it.
    fn close_undo_blocks(&mut self, cx: &mut Context<Self>) {
        // The block of an IME composition that the user started is not the macro's: it closes
        // when the text is committed.
        if self.composing {
            return;
        }
        let cursor = &self.cursor;
        self.document.update(cx, |shared, _| {
            shared.parts_mut().0.close_blocks(cursor);
        });
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
        if let Some(message) = self.stop_key_macro_recording(cx) {
            self.macro_failed(message, window, cx);
        }
        // Sakura starts in the macro folder (`Command_SAVEKEYMACRO`).
        let ask = Ask::save()
            .start(file_dialog::macro_folder(cx))
            .name(Some("RecKey.mac".to_owned()))
            .titled("キーマクロの保存", "保存");
        let answer = file_dialog::ask(window, cx, ask);
        let tracked = track(cx);
        let fallback = window.window_handle();
        cx.spawn(async move |this, cx| {
            let _tracked = tracked;
            let answer = answer.await;
            follow(&this, fallback, cx, |this, window, cx| match answer {
                Ok(Some(paths)) => {
                    if let Some(path) = paths.last()
                        && let Err(message) = this.save_key_macro_to(path, cx)
                    {
                        this.macro_failed(message, window, cx);
                    }
                }
                Err(error) => this.macro_failed(dialog_error(&error), window, cx),
                Ok(None) => {}
            });
        })
        .detach();
    }

    /// Writes the key macro to `path`. Only a key macro can be saved (Sakura:
    /// `STR_ERR_CEDITVIEW_CMD26`).
    pub(crate) fn save_key_macro_to(&self, path: &Path, cx: &App) -> Result<(), String> {
        let key_macro = match &self.macros.saved_macro(cx) {
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
        if let Some(message) = self.stop_key_macro_recording(cx) {
            self.macro_failed(message, window, cx);
        }
        // Sakura starts in the macro folder (`Command_LOADKEYMACRO`).
        let ask = Ask::open(false)
            .start(file_dialog::macro_folder(cx))
            .titled("キーマクロの読み込み", "読み込み");
        let answer = file_dialog::ask(window, cx, ask);
        let tracked = track(cx);
        let fallback = window.window_handle();
        cx.spawn(async move |this, cx| {
            let _tracked = tracked;
            let answer = answer.await;
            follow(&this, fallback, cx, |this, window, cx| match answer {
                Ok(Some(mut paths)) => {
                    if let Some(path) = paths.pop() {
                        set_shared_key_macro(cx, Some(path.clone()));
                        this.macros.current = Some(CurrentMacro::File(path));
                    }
                }
                Err(error) => this.macro_failed(dialog_error(&error), window, cx),
                Ok(None) => {}
            });
        })
        .detach();
    }

    /// `F_EXECKEYMACRO`: stops a recording, then plays the key macro.
    pub(crate) fn exec_key_macro(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), CommandError> {
        // Sakura goes on to play the file even when it could not write it.
        if let Some(message) = self.stop_key_macro_recording(cx) {
            self.macro_failed(message, window, cx);
        }
        if self.macros.busy() {
            return Err(self.macro_error("マクロを実行中です".into(), window, cx));
        }
        let result = match self.macros.current_macro(cx) {
            // Nothing to play; Sakura greys the item out.
            None => Ok(()),
            Some(CurrentMacro::Recorded(key_macro)) => {
                self.start_key_macro(key_macro.steps().to_vec(), None, window, cx)
            }
            Some(CurrentMacro::File(path)) => self.run_macro_file(&path, window, cx),
        };
        result.map_err(|message| self.macro_error(message, window, cx))
    }

    // ---- macro files ----

    /// `F_EXECEXTMACRO` without a file: asks for one.
    pub(crate) fn ask_macro_to_run(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Sakura starts in the macro folder (`Command_EXECEXTMACRO`).
        let ask = Ask::open(false)
            .start(file_dialog::macro_folder(cx))
            .titled("マクロの実行", "実行");
        let answer = file_dialog::ask(window, cx, ask);
        let tracked = track(cx);
        let fallback = window.window_handle();
        cx.spawn(async move |this, cx| {
            let _tracked = tracked;
            let answer = answer.await;
            follow(&this, fallback, cx, |this, window, cx| match answer {
                Ok(Some(mut paths)) => {
                    if let Some(path) = paths.pop() {
                        // The error is reported by `exec_ext_macro`.
                        this.exec_ext_macro(&path, window, cx).ok();
                    }
                }
                Err(error) => this.macro_failed(dialog_error(&error), window, cx),
                Ok(None) => {}
            });
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
        // A key macro that is playing may name a macro file: its calls go on in the middle. A call
        // of a plugin has the document and the undo block until it ends (`plugin_call`).
        if self.macros.running.is_some() || self.macros.plugin_running {
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

    /// `F_EXECEXTMACRO` with a second argument (`Command_EXECEXTMACRO` with `pszType`): `code` is
    /// the text of a macro in the language that the extension `ext` names (`CSMacroMgr::Load` with
    /// `LoadKeyMacroStr`), not a file. Records the call if a key macro is being recorded, with both
    /// arguments so that the recording plays back (Sakura records only the first, which then
    /// plays as a file name). Errors go to the panel.
    pub(crate) fn exec_macro_text(
        &mut self,
        code: &str,
        ext: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), CommandError> {
        if self.macros.running.is_some() || self.macros.plugin_running {
            return Err(self.macro_error("マクロを実行中です".into(), window, cx));
        }
        if self.macros.recorder.is_recording() && !self.macros.suspended {
            self.macros.recorder.record(
                F_EXECEXTMACRO,
                &[Arg::Text(code.to_owned()), Arg::Text(ext.to_owned())],
            );
        }
        self.run_macro_text(code, ext, window, cx)
            .map_err(|message| self.macro_error(message, window, cx))
    }

    /// Runs the text of a macro by the extension that names its language: `mac` through the key
    /// macro player; the others through the host, from a file of a temporary name (the host runs
    /// files). The `Err` says what to show.
    fn run_macro_text(
        &mut self,
        code: &str,
        ext: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let ext = ext.trim().trim_start_matches('.');
        if ext.eq_ignore_ascii_case("mac") {
            let key_macro =
                KeyMacro::from_text(code).map_err(|e| format!("マクロの文字列: {e}"))?;
            return self.start_key_macro(key_macro.steps().to_vec(), None, window, cx);
        }
        let named = PathBuf::from(format!("macro.{ext}"));
        if !matches!(file_kind(&named), FileKind::Script(_)) {
            return Err(format!(
                "マクロの種類 {ext:?} は使えません。mac、mjs、js、cjs、ts、py、rb、php、ps1、pl、lua、mruby、upy のどれかにしてください"
            ));
        }
        let temp = TempScript::create(ext, code)?;
        self.start_script(&temp.path, window, cx)?;
        self.macros.temp_script = Some(temp);
        Ok(())
    }

    /// Runs a macro file by its extension: `.mac` through the key macro player, the others
    /// (`.mjs`, `.js`, `.cjs`, `.ts`, `.py`, `.rb`, `.php`, `.ps1`, `.pl`, `.el`, `.applescript`, `.lua`, `.mruby`, `.upy`) through the host.
    /// The `Err` says what to show.
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
                self.start_key_macro(key_macro.steps().to_vec(), Some(path), window, cx)
            }
            FileKind::Script(_) => self.start_script(path, window, cx),
            FileKind::Unknown => Err(format!(
                "{}: マクロの拡張子は .mac、.mjs、.js、.cjs、.ts、.py、.rb、.php、.ps1、.pl、.el、.applescript、.lua、.mruby、.upy のどれかにしてください",
                path.display()
            )),
        }
    }

    // ---- playing a key macro ----

    /// Plays `steps`. `file` is the macro file they came from, if any: a file that is already
    /// being played, or nesting deeper than [`MAX_NESTING`], is refused (Sakura would recurse
    /// until the stack ends).
    fn start_key_macro(
        &mut self,
        steps: Vec<Step>,
        file: Option<&Path>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let mut items: Vec<Item> = Vec::with_capacity(steps.len() + 1);
        if let Some(file) = file {
            let id = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
            if self.macros.active.contains(&id) {
                return Err(format!(
                    "{}: マクロが自分自身を呼んでいます",
                    file.display()
                ));
            }
            if self.macros.active.len() >= MAX_NESTING {
                return Err(format!(
                    "{}: マクロの入れ子が深すぎます (最大 {MAX_NESTING})",
                    file.display()
                ));
            }
            self.macros.active.push(id);
            let depth = self.enter_undo_block(cx);
            self.macros.undo_depths.push(depth);
            items.extend(steps.into_iter().map(Item::Call));
            items.push(Item::Leave);
        } else {
            items.extend(steps.into_iter().map(Item::Call));
        }
        // A macro file named by a key macro that is playing runs in its place, and the rest of
        // the outer macro follows.
        if self.macros.queue.is_empty() {
            self.macros.played = 0;
        }
        for item in items.into_iter().rev() {
            self.macros.queue.push_front(item);
        }
        self.macros.suspended = true;
        if !self.macros.draining {
            self.drain_key_macro(window, cx);
        }
        Ok(())
    }

    /// Drops what is left of the key macro that is playing.
    fn clear_key_macro(&mut self) {
        self.macros.queue.clear();
        self.macros.active.clear();
        self.macros.undo_depths.clear();
    }

    /// Runs the calls of the key macro one after the other, as the keys would. It stops at the
    /// first call that fails (`CKeyMacroMgr::ExecKeyMacro`), and waits while a script runs.
    fn drain_key_macro(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.macros.draining = true;
        while self.macros.running.is_none() && self.macros.waiting.is_none() {
            let step = match self.macros.queue.pop_front() {
                None => break,
                Some(Item::Leave) => {
                    self.macros.active.pop();
                    if let Some(depth) = self.macros.undo_depths.pop() {
                        self.leave_undo_block(depth, cx);
                    }
                    continue;
                }
                Some(Item::Call(step)) => step,
            };
            self.macros.played += 1;
            // Only the declared arguments are passed (as `KeyMacro::play` does).
            let declared = macro_names::by_code(step.code).map_or(usize::MAX, |f| f.args.len());
            let args = &step.args[..step.args.len().min(declared)];
            self.notice = None;
            // The result says whether the call failed; a notice may also come with a success.
            let (ok, wait) = self.run_macro_function(step.code, args, window, cx);
            if !ok {
                let notice = self.notice.clone().unwrap_or_default();
                self.clear_key_macro();
                let message = format!(
                    "キーマクロの {} 番目の呼び出し ({}) で止まりました: {notice}",
                    self.macros.played,
                    function_name(step.code)
                );
                self.macros.output.show();
                self.macros.output.push(Kind::Note, message);
                break;
            }
            // A dialog, a panel or a file that is read: the macro goes on when it is over.
            if let Some(wait) = wait {
                // The panel has the cancel button, which a key macro that waits needs.
                self.macros.output.show();
                self.begin_wait(wait, window, cx, |this, window, cx| {
                    this.drain_key_macro(window, cx);
                });
            }
        }
        self.macros.draining = false;
        if self.macros.running.is_none()
            && self.macros.waiting.is_none()
            && self.macros.queue.is_empty()
        {
            self.macros.suspended = false;
            self.macros.registered.clear_save_path();
            self.close_undo_blocks(cx);
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
        #[allow(unused_mut)]
        let mut config = MacroConfig::for_script(path, document_path).map_err(|e| e.to_string())?;
        #[cfg(test)]
        if let Some(timeout) = self.macros.script_timeout {
            config.timeout = timeout;
        }
        let timeout_text = format!("{} 秒", config.timeout.as_secs_f64());
        // Bounded: a script that prints without end waits here (on the host's reader thread)
        // for the window to catch up, instead of piling up in memory.
        let (tx, rx) = async_channel::bounded::<HostEvent>(EVENT_BACKLOG);
        let sink: Sink = Arc::new(move |event| {
            // Fails only when the window is gone.
            let _ = tx.send_blocking(event);
        });
        let session = session::start(config, sink).map_err(|e| e.to_string())?;
        self.macros.output.begin();
        self.macros
            .output
            .push(Kind::Note, format!("マクロ {name} を開始します"));
        self.macros.serial += 1;
        self.macros.running = Some(Running {
            session,
            name,
            path: path.to_path_buf(),
            timeout_text,
        });
        self.macros.suspended = true;
        // The host's threads send events; each one is handled here, on the UI thread, in order.
        // The script belongs to the tab it started in, not to the window: the events go to the
        // window that shows the tab now.
        let fallback = window.window_handle();
        cx.spawn(async move |this, cx| {
            while let Ok(event) = rx.recv().await {
                let last = matches!(event, HostEvent::Finished(_));
                if follow(&this, fallback, cx, |this, window, cx| {
                    this.on_host_event(event, window, cx)
                })
                .is_none()
                    || last
                {
                    break;
                }
                // Let the window paint and take clicks (the cancel button) between events.
                yield_now().await;
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
                // An IME composition that the user started ends first, as for a key.
                self.interrupt_composition(cx);
                // A message box, a file dialog or `Sleep` is answered when it is over.
                if !self.start_waiting_call(&text, window, cx)
                    && !self.run_ui_call(&text, window, cx)
                {
                    let mut env = self.host_env(&text, cx);
                    let reply = self.run_with(cx, |command_cx| {
                        bridge::handle_message_with(Registry::global(), command_cx, &mut env, &text)
                    });
                    self.finish_host_env(env, cx);
                    if let (Some(reply), Some(running)) = (reply, &self.macros.running) {
                        running.session.send(reply);
                    }
                }
            }
            HostEvent::Finished(finished) => {
                let running = self.macros.running.take();
                self.macros.temp_script = None;
                // A call of the script that was waiting gets no answer: the script is gone.
                self.macros.waiting = None;
                let (name, timeout_text) =
                    running.map_or_else(Default::default, |r| (r.name, r.timeout_text));
                let mut message = matsu_plugin::describe(&finished, &timeout_text);
                // On Windows, `python` can be the Microsoft Store stub, which exits with 9009.
                if finished.exit_code == Some(9009) {
                    message.push_str("。Python が見つかりません。インストールを確かめてください");
                }
                self.macros.output.push(Kind::Note, message.clone());
                self.notice = Some(format!("マクロ {name}: {message}"));
                // A key macro that started this script goes on if the script succeeded, and
                // stops there if it failed, timed out or was cancelled (like a call that fails).
                let succeeded =
                    finished.reason == EndReason::Exited && finished.exit_code == Some(0);
                if !succeeded && !self.macros.queue.is_empty() {
                    let played = self.macros.played;
                    self.clear_key_macro();
                    self.macros.output.push(
                        Kind::Note,
                        format!("キーマクロの {played} 番目の呼び出し ({name}) で止まりました"),
                    );
                }
                self.drain_key_macro(window, cx);
            }
        }
        cx.notify();
    }

    /// Stops the macro that is running, and drops the rest of a key macro that waits for it.
    pub fn cancel_macro(&mut self, cx: &mut Context<Self>) {
        self.clear_key_macro();
        // The wait for a dialog ends; the dialog itself stays for the user to close.
        self.macros.waiting = None;
        match &self.macros.running {
            // `Finished` follows with `Cancelled`.
            Some(running) => running.session.cancel(),
            None => {
                self.macros.suspended = false;
                self.macros.registered.clear_save_path();
                self.close_undo_blocks(cx);
            }
        }
        cx.notify();
    }

    // ---- the panel ----

    /// The output panel for the window to show under the text, while it is open.
    pub fn macro_panel(&self, view: Entity<EditorView>) -> Option<AnyElement> {
        let cancel = view.clone();
        let jump = view.clone();
        self.macros.output.panel(PanelActions {
            running: self.macros.running.is_some() || self.macros.waiting.is_some(),
            on_cancel: Box::new(move |_, _, cx| {
                cancel.update(cx, |view, cx| view.cancel_macro(cx));
            }),
            on_close: Box::new(move |_, _, cx| {
                view.update(cx, |view, cx| {
                    view.macros.output.close();
                    cx.notify();
                });
            }),
            on_jump: std::rc::Rc::new(move |place, window, cx| {
                jump.update(cx, |view, cx| view.jump_to_place(place, window, cx));
            }),
        })
    }

    /// Where a file dialog starts: the folder of the document, else what 「ファイルダイアログの初期位置」
    /// says (`GetDlgInitialDir`).
    fn dialog_start(&self, cx: &Context<Self>) -> Option<PathBuf> {
        file_dialog::start_folder(cx, self.document.read(cx).path())
    }

    /// The start folder and the name for a dialog that is given a default: the folder of the
    /// default if it names one that exists, else [`Self::dialog_start`]; the name is its file name.
    fn dialog_default(
        &self,
        default: &str,
        cx: &Context<Self>,
    ) -> (Option<PathBuf>, Option<String>) {
        // a default that ends with a separator is a folder with no name
        let (folder, name) =
            matsu_config::dialog_dir::split_default_name(default, &|dir| dir.is_dir());
        let folder = folder.or_else(|| self.dialog_start(cx));
        (folder, name)
    }
}

/// On Linux a dialog needs xdg-desktop-portal, which WSLg does not have.
fn dialog_error(error: &impl std::fmt::Display) -> String {
    format!("ファイルを選べません: {error}")
}

#[cfg(test)]
mod tests;
