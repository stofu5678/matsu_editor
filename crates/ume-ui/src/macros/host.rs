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

use gpui::{Context, PathPromptOptions, PromptLevel, Window};
use ume_plugin::Value;
use ume_plugin::functions::window::{self, Deferred, Icon, Pending};
use ume_plugin::host::{HostEnv, ViewInfo};

use super::{EditorView, dialog_error};

/// The caption of the message boxes (Sakura shows "sakura macro").
const CAPTION: &str = "UME Editor マクロ";

impl EditorView {
    /// The environment for one call: what the window reports. `message` is the line of the call,
    /// which tells whether the call needs the types of the settings.
    pub(super) fn host_env(&mut self, message: &str, cx: &Context<Self>) -> HostEnv {
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
        env.recording_key_macro = self.macros.recorder.is_recording();
        env.macro_path = self
            .macros
            .running
            .as_ref()
            .map(|running| running.path.to_string_lossy().into_owned());
        env.current_type = self.prefs.settings.type_index;
        // The types of the settings are only copied for the two functions that read them.
        if message.contains("TypeExt")
            && let Some(user) = cx.try_global::<crate::settings::UserSettings>().cloned()
        {
            env.type_of_ext = Some(Box::new(move |ext| {
                user.view_settings(Some(ext)).type_index
            }));
        }
        env
    }

    /// Keeps what the call left in the environment, and does what it asked for.
    pub(super) fn finish_host_env(&mut self, env: HostEnv, cx: &mut Context<Self>) {
        self.macros.host_state = env.state;
        if let Some(width) = env.requests.tab_width {
            self.set_tab_width(width, cx);
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
            Deferred::FileOpen { .. } => {
                let answer = cx.prompt_for_paths(PathPromptOptions {
                    files: true,
                    directories: false,
                    multiple: false,
                    prompt: Some("開く".into()),
                });
                self.wait_for(serial, pending, window, cx, path_answer(answer));
            }
            Deferred::Folder { message, .. } => {
                let answer = cx.prompt_for_paths(PathPromptOptions {
                    files: false,
                    directories: true,
                    multiple: false,
                    prompt: (!message.is_empty()).then(|| message.into()),
                });
                self.wait_for(serial, pending, window, cx, path_answer(answer));
            }
            Deferred::FileSave { default, .. } => {
                // The folder of the default name if it names one, else that of the document.
                let given = Path::new(&default);
                let dir = given
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty() && p.is_dir())
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| self.dialog_directory(cx));
                let name = given
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .filter(|n| !n.is_empty());
                let answer = cx.prompt_for_new_path(&dir, name.as_deref());
                self.wait_for(serial, pending, window, cx, async move {
                    match answer.await {
                        Ok(Ok(Some(path))) => Ok(path_value(&path)),
                        Ok(Err(error)) => Err(dialog_error(&error)),
                        _ => Ok(Value::from("")),
                    }
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
        cx.spawn_in(window, async move |this, cx| {
            let result = answer.await;
            this.update_in(cx, |this, window, cx| {
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
            })
            .ok();
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
async fn path_answer<E: std::fmt::Display, C>(
    answer: impl Future<Output = Result<Result<Option<Vec<PathBuf>>, E>, C>>,
) -> Result<Value, String> {
    match answer.await {
        Ok(Ok(Some(mut paths))) => Ok(paths
            .pop()
            .map_or_else(|| Value::from(""), |p| path_value(&p))),
        Ok(Err(error)) => Err(dialog_error(&error)),
        _ => Ok(Value::from("")),
    }
}

fn path_value(path: &Path) -> Value {
    Value::from(path.to_string_lossy().into_owned())
}
