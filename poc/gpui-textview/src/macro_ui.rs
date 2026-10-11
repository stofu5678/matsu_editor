//! Macros in the editor window: the Tools menu commands, the bridge from a running macro to the
//! `Editor` (it answers the macro's calls on the UI thread), and the output panel.
//!
//! The process and its I/O are in `macro_host` and run on threads of their own. Their events
//! come to the UI through a channel, and each call of the macro is run here, one at a time, on
//! the text that the window shows.

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    AnyElement, Context, PathPromptOptions, ScrollHandle, Window, div, prelude::*, px, rgb,
};

use crate::macro_host::api::{self, MacroDocument};
use crate::macro_host::session::{self, HostEvent, MacroConfig, Session, Sink, Stream};
use crate::{CancelMacro, Editor, FONTS, RunMacro};

/// The most lines that the panel keeps (older ones go).
const MAX_LINES: usize = 5000;
/// The most lines that the panel draws (the last ones).
const SHOWN_LINES: usize = 300;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Out,
    Err,
    /// What the editor says: start, end, errors of its own.
    Note,
}

struct OutputLine {
    kind: Kind,
    text: String,
}

struct Running {
    session: Session,
    name: String,
    timeout_text: String,
}

#[derive(Default)]
pub struct MacroState {
    running: Option<Running>,
    lines: Vec<OutputLine>,
    /// The panel is shown.
    open: bool,
    scroll: ScrollHandle,
}

impl MacroState {
    fn push(&mut self, kind: Kind, text: String) {
        self.lines.push(OutputLine { kind, text });
        if self.lines.len() > MAX_LINES {
            self.lines.drain(..self.lines.len() - MAX_LINES);
        }
        self.scroll.scroll_to_bottom();
    }

    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }

    /// Everything in the panel, one line each (for the self-check).
    #[cfg(feature = "self-check")]
    pub fn output_text(&self) -> String {
        let lines: Vec<&str> = self.lines.iter().map(|l| l.text.as_str()).collect();
        lines.join("\n")
    }
}

/// The editor's text as the document of the macro API. Edits go through the editor's own
/// methods, so they move the cursor, keep it in view and redraw as the keys do.
struct EditorDoc<'a, 'b> {
    editor: &'a mut Editor,
    cx: &'a mut Context<'b, Editor>,
}

impl MacroDocument for EditorDoc<'_, '_> {
    fn line_count(&self) -> usize {
        self.editor.lines.len()
    }

    fn line_len(&self, row: usize) -> usize {
        self.editor.lines[row].len()
    }

    fn line_text(&self, row: usize) -> String {
        self.editor.lines[row].clone()
    }

    fn cursor(&self) -> (usize, usize) {
        self.editor.cursor
    }

    fn has_selection(&self) -> bool {
        self.editor.selection().is_some()
    }

    fn selected_text(&self) -> String {
        self.editor
            .selection()
            .map(|(from, to)| self.editor.text_in(from, to))
            .unwrap_or_default()
    }

    fn move_cursor(&mut self, pos: (usize, usize), extend: bool) {
        self.editor.move_to(pos, extend, self.cx);
    }

    fn move_vertical(&mut self, delta: isize) {
        self.editor.vertical(delta, false, self.cx);
    }

    fn select_all(&mut self) {
        self.editor.anchor = Some((0, 0));
        let end = self.editor.end_of_doc();
        self.editor.move_to(end, true, self.cx);
    }

    fn insert(&mut self, text: &str) {
        self.editor.insert(text, self.cx);
    }

    fn path(&self) -> Option<String> {
        self.editor.path.as_ref().map(|p| p.display().to_string())
    }
}

impl Editor {
    /// Tools > 名前を指定してマクロ実行...: asks for a file and runs it.
    pub fn run_macro_dialog(&mut self, _: &RunMacro, window: &mut Window, cx: &mut Context<Self>) {
        if self.macros.is_running() {
            self.notice = Some("マクロを実行中です".into());
            cx.notify();
            return;
        }
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("実行".into()),
        });
        cx.spawn_in(window, async move |this, cx| match paths.await {
            Ok(Ok(Some(mut paths))) => {
                if let Some(path) = paths.pop() {
                    this.update(cx, |this, cx| this.start_macro(path, cx)).ok();
                }
            }
            // On Linux the dialog needs xdg-desktop-portal, which WSLg does not have.
            Ok(Err(err)) => {
                this.update(cx, |this, cx| {
                    this.macros.open = true;
                    this.macros
                        .push(Kind::Note, format!("ファイルを選べません: {err}"));
                    cx.notify();
                })
                .ok();
            }
            _ => {}
        })
        .detach();
    }

    /// Tools > マクロの中断: kills the macro that is running.
    pub fn cancel_macro(&mut self, _: &CancelMacro, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(running) = &self.macros.running {
            running.session.cancel();
        }
        cx.notify();
    }

    /// Starts the macro file. The macro runs in another process, and its calls come back to
    /// `on_macro_event` through the channel.
    pub fn start_macro(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.macros.is_running() {
            return;
        }
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        self.macros.lines.clear();
        self.macros.open = true;
        self.macros
            .push(Kind::Note, format!("マクロ {name} を開始します"));
        let document_path = self.path.as_ref().map(|p| p.display().to_string());
        let config = match MacroConfig::for_script(&path, document_path) {
            Ok(config) => config,
            Err(err) => {
                self.macros.push(Kind::Note, err.to_string());
                cx.notify();
                return;
            }
        };
        let timeout_text = format!("{} 秒", config.timeout.as_secs_f64());
        let (tx, rx) = async_channel::unbounded::<HostEvent>();
        let sink: Sink = Arc::new(move |event| {
            let _ = tx.try_send(event);
        });
        match session::start(config, sink) {
            Ok(session) => {
                self.macros.running = Some(Running {
                    session,
                    name,
                    timeout_text,
                });
                self.notice = Some("マクロを実行中".into());
                cx.spawn(async move |this, cx| {
                    while let Ok(event) = rx.recv().await {
                        let last = matches!(event, HostEvent::Finished(_));
                        if this
                            .update(cx, |editor, cx| editor.on_macro_event(event, cx))
                            .is_err()
                        {
                            break;
                        }
                        if last {
                            break;
                        }
                    }
                })
                .detach();
            }
            Err(err) => self.macros.push(Kind::Note, err.to_string()),
        }
        cx.notify();
    }

    fn on_macro_event(&mut self, event: HostEvent, cx: &mut Context<Self>) {
        match event {
            HostEvent::Output { stream, text } => {
                let kind = match stream {
                    Stream::Stdout => Kind::Out,
                    Stream::Stderr => Kind::Err,
                };
                self.macros.push(kind, text);
            }
            HostEvent::Message(text) => {
                // One call at a time, on the UI thread, against the text on the screen.
                let reply = api::handle_message(&mut EditorDoc { editor: self, cx }, &text);
                if let (Some(reply), Some(running)) = (reply, &self.macros.running) {
                    running.session.send(reply);
                }
            }
            HostEvent::Finished(finished) => {
                let running = self.macros.running.take();
                let timeout_text = running
                    .as_ref()
                    .map_or_else(String::new, |r| r.timeout_text.clone());
                let mut message = crate::macro_host::describe(&finished, &timeout_text);
                // On Windows, `python` can be the Microsoft Store stub, which exits with 9009.
                if finished.exit_code == Some(9009) {
                    message
                        .push_str("。Python が見つかりません。環境変数 POC_PYTHON で指定できます");
                }
                self.notice = Some(format!(
                    "マクロ {}: {message}",
                    running.map_or_else(String::new, |r| r.name)
                ));
                self.macros.push(Kind::Note, message);
            }
        }
        cx.notify();
    }

    /// The output panel, under the text, while it is open.
    pub fn render_macro_panel(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.macros.open {
            return None;
        }
        let skip = self.macros.lines.len().saturating_sub(SHOWN_LINES);
        let lines = self.macros.lines[skip..].iter().map(|line| {
            let color = match line.kind {
                Kind::Out => rgb(0x1e1e1e),
                Kind::Err => rgb(0xb00020),
                Kind::Note => rgb(0x0b5cad),
            };
            // An empty line still needs its height.
            let text = if line.text.is_empty() {
                "\u{a0}".to_string()
            } else {
                line.text.clone()
            };
            div().text_color(color).child(text)
        });
        let title = if self.macros.is_running() {
            "マクロの出力 (実行中)"
        } else {
            "マクロの出力"
        };
        Some(
            div()
                .flex()
                .flex_col()
                .h(px(170.))
                .border_t_1()
                .border_color(rgb(0xd0d0d0))
                .bg(rgb(0xf7f7f7))
                .text_size(px(12.))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .justify_between()
                        .px_2()
                        .py_0p5()
                        .bg(rgb(0xe8e8e8))
                        .child(title)
                        .child(
                            div()
                                .id("macro-panel-close")
                                .cursor_pointer()
                                .child("閉じる")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.macros.open = false;
                                    cx.notify();
                                })),
                        ),
                )
                .child(
                    div()
                        .id("macro-output")
                        .flex_1()
                        .overflow_y_scroll()
                        .track_scroll(&self.macros.scroll)
                        .px_2()
                        .font_family(FONTS[self.font_index])
                        .children(lines),
                )
                .into_any_element(),
        )
    }
}
