//! What the macro functions that need the window get from it: the environment of one call
//! (`ume_plugin::host::HostEnv`), and the calls that wait for a dialog or for time.
//!
//! The bridge (`ume_plugin::bridge`) answers a call on the spot. A message box, a file dialog and
//! `Sleep` cannot be answered on the spot: the window has to go on while the macro waits. The
//! reply is sent when the user has answered or the time is over, and the session's clock stops
//! meanwhile (`Session::hold`). Sakura's modal dialogs block its window the same way.
//!
//! Sakura: `sakura_core/macro/CMacro.cpp`, `CMacro::HandleFunction`.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{Context, PromptLevel, Window};
use ume_plugin::Value;
use ume_plugin::functions;
use ume_plugin::functions::window::{self, Deferred, Icon, Pending};
use ume_plugin::host::{HostEnv, ViewInfo};

use super::{EditorView, dialog_error};
use crate::file_dialog::{self, Answer, Ask};
use crate::input_dialog::InputRequest;

/// The caption of the message boxes (Sakura shows "sakura macro").
const CAPTION: &str = "UME Editor マクロ";

impl EditorView {
    /// The environment for one call: what the window reports. `message` is the line of the call,
    /// which tells whether the call needs the types of the settings.
    pub(crate) fn host_env(&mut self, message: &str, cx: &Context<Self>) -> HostEnv {
        let metrics = self.metrics;
        // `CTextArea::UpdateViewColRowNums`: (size - 1) / pitch, at least 1.
        let cells = |size: f32, pitch: f32| -> i64 {
            if pitch > 0.0 {
                (((size - 1.0).max(0.0) / pitch).floor() as i64).max(1)
            } else {
                1
            }
        };
        let top_row = if metrics.line_height > 0.0 {
            (self.scroll_y / metrics.line_height).floor().max(0.0) as i64
        } else {
            0
        };
        // The functions that convert between layout and logical positions need exact row counts:
        // a document that is still being laid out in the background has estimates for the rest.
        // Only they pay for completing the layout.
        let needs_exact = ume_plugin::protocol::parse_request(message)
            .is_ok_and(|request| functions::needs_exact_layout(&request.method));
        if needs_exact && !self.layout.is_complete() {
            let _ = self.layout.layout_all(self.document.read(cx).doc());
        }
        let mut env = HostEnv::detached();
        env.state = std::mem::take(&mut self.macros.host_state);
        env.char_code = Some(i64::from(
            self.document.read(cx).file().encoding.sakura_code(),
        ));
        env.view = Some(ViewInfo {
            rows: cells(metrics.view_height, metrics.line_height),
            columns: cells(metrics.view_width, metrics.cell_width),
            top_row,
        });
        env.status_bar_shown = self.prefs.show_status_bar;
        env.wrap_columns = Some(self.layout.config().max_columns());
        env.ins_space = Some(self.prefs.settings.ins_space);
        env.recording_key_macro = self.macros.recorder.is_recording();
        env.macro_path = self
            .macros
            .running
            .as_ref()
            .map(|running| running.path.to_string_lossy().into_owned());
        env.current_type = self.prefs.settings.type_index;
        // The types of the settings are only copied for the two functions that read them.
        if message.contains("TypeExt") {
            // `GetDocumentTypeOfExt`; the built-in types when there is no settings file.
            let types = crate::doc_types::with_user(cx, |user| user.shared_types());
            env.type_of_ext = Some(Box::new(move |ext| types.type_index_for_ext(ext)));
        }
        env
    }

    /// Keeps what the call left in the environment, and does what it asked for.
    pub(crate) fn finish_host_env(&mut self, env: HostEnv, cx: &mut Context<Self>) {
        self.macros.host_state = env.state;
        if let Some(width) = env.requests.tab_width {
            self.set_tab_width(width, cx);
        }
        if let Some(ins_space) = env.requests.ins_space {
            // `ChangeInsertSpace`: only a flag of this view (the Tab key does not read it yet).
            self.prefs.settings.ins_space = ins_space;
        }
        if let Some(columns) = env.requests.wrap_columns {
            self.set_wrap_columns(columns, cx);
        }
    }

    /// If the message is a call that waits (see the module documentation), starts the wait and
    /// returns true; the answer is sent later.
    pub(super) fn start_waiting_call(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let pending = match window::parse(text) {
            None => return false,
            Some(Err(line)) => {
                if let Some(running) = &self.macros.running {
                    running.session.send(line);
                }
                return true;
            }
            Some(Ok(pending)) => pending,
        };
        let serial = self.macros.serial;
        if let Some(running) = &self.macros.running {
            running.session.hold();
        }
        match pending.what.clone() {
            Deferred::Message {
                text,
                buttons,
                icon,
            } => {
                let level = match icon {
                    Icon::Error => PromptLevel::Critical,
                    Icon::Warning => PromptLevel::Warning,
                    Icon::Question | Icon::Information | Icon::None => PromptLevel::Info,
                };
                let answer = window.prompt(level, CAPTION, Some(&text), buttons.labels(), cx);
                self.wait_for(serial, pending, window, cx, async move {
                    // A prompt that is dropped without an answer is a closed box.
                    Ok(Value::from(buttons.result(answer.await.ok())))
                });
            }
            Deferred::FileOpen { default, .. } => {
                // The folder of the default name if it names one, else that of the document.
                let (folder, name) = self.dialog_default(&default, cx);
                let ask = Ask::open(false)
                    .start(folder)
                    .name(name)
                    .titled("ファイルを開く", "開く");
                let answer = file_dialog::ask(window, cx, ask);
                self.wait_for(serial, pending, window, cx, path_answer(answer));
            }
            Deferred::Folder { message, default } => {
                let start = Some(PathBuf::from(&default))
                    .filter(|dir| !default.is_empty() && dir.is_dir())
                    .or_else(|| self.dialog_start(cx));
                let ask = Ask::folder().start(start);
                let ask = if message.is_empty() {
                    ask
                } else {
                    ask.titled(message.clone(), message)
                };
                let answer = file_dialog::ask(window, cx, ask);
                self.wait_for(serial, pending, window, cx, path_answer(answer));
            }
            Deferred::FileSave { default, .. } => {
                let (folder, name) = self.dialog_default(&default, cx);
                let ask = Ask::save().start(folder).name(name);
                let answer = file_dialog::ask(window, cx, ask);
                self.wait_for(serial, pending, window, cx, path_answer(answer));
            }
            Deferred::Input {
                message,
                default,
                max_len,
            } => {
                let request = InputRequest {
                    title: CAPTION.to_owned(),
                    message,
                    default,
                    max_len,
                };
                let answer = self.open_input_dialog(request, window, cx);
                // Cancel is an empty string (F_INPUTBOX).
                self.wait_for(serial, pending, window, cx, async move {
                    Ok(Value::from(answer.await.unwrap_or_default()))
                });
            }
            Deferred::Menu { at_caret, nodes } => {
                let answer = self.open_macro_menu(at_caret, &nodes, window, cx);
                self.wait_for(serial, pending, window, cx, async move {
                    Ok(Value::from(answer.await))
                });
            }
            Deferred::Sleep { millis } => {
                let timer = cx
                    .background_executor()
                    .timer(Duration::from_millis(millis));
                self.wait_for(serial, pending, window, cx, async move {
                    timer.await;
                    Ok(Value::from(0))
                });
            }
        }
        true
    }

    /// Waits for `answer` and then sends the result to the macro that asked (`serial` is its
    /// number: an answer for a macro that has gone is dropped), and runs its clock again. A
    /// failure is shown to the user and answers an empty string.
    fn wait_for(
        &mut self,
        serial: u64,
        pending: Pending,
        window: &mut Window,
        cx: &mut Context<Self>,
        answer: impl Future<Output = Result<Value, String>> + 'static,
    ) {
        // The macro belongs to its tab: the answer goes to the window that shows it now.
        let fallback = window.window_handle();
        cx.spawn(async move |this, cx| {
            let result = answer.await;
            super::follow(&this, fallback, cx, |this, window, cx| {
                let value = match result {
                    Ok(value) => value,
                    Err(message) => {
                        this.macro_failed(message, window, cx);
                        Value::from("")
                    }
                };
                if this.macros.serial == serial
                    && let Some(running) = &this.macros.running
                {
                    running.session.send(pending.reply(value));
                    running.session.release();
                }
            });
        })
        .detach();
    }

    /// Sets the tab width of this view only (`F_CHGTABWIDTH`: a temporary setting), and lays the
    /// text out again.
    pub fn set_tab_width(&mut self, width: u32, cx: &mut Context<Self>) {
        if self.prefs.settings.tab_width != width {
            self.prefs.settings.tab_width = width;
            self.relayout_in(cx);
            cx.notify();
        }
    }

    /// Wraps at `columns` for this view only (`F_CHGWRAPCOLUMN`: the method becomes 指定桁, the
    /// column is the one given).
    pub fn set_wrap_columns(&mut self, columns: u32, cx: &mut Context<Self>) {
        self.prefs.wrap_method = crate::settings::WrapMethod::SettingWidth;
        self.prefs.wrap_columns = columns;
        self.relayout_in(cx);
        cx.notify();
    }
}

/// The answer of a dialog that chooses one path: the path, or an empty string for a cancel.
async fn path_answer(answer: impl Future<Output = Answer>) -> Result<Value, String> {
    match answer.await {
        Ok(Some(mut paths)) => Ok(paths
            .pop()
            .map_or_else(|| Value::from(""), |p| path_value(&p))),
        Err(error) => Err(dialog_error(&error)),
        Ok(None) => Ok(Value::from("")),
    }
}

fn path_value(path: &Path) -> Value {
    Value::from(path.to_string_lossy().into_owned())
}
