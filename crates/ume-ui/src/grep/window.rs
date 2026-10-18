//! Grep in a window: where the result goes, the search that runs off the UI thread, and what makes
//! the window a Grep window.
//!
//! Sakura: `CViewCommander::Command_GREP` (`sakura_core/cmd/CViewCommander_Grep.cpp`) writes the
//! result into this window when it is a Grep window that is not searching, or an untitled window
//! without changes; else it opens a new window for it. `CGrepAgent::DoGrep` shows the cancel
//! dialog while the search runs, writes the heading and the hits at the end of the text, moves the
//! caret back to where the result begins and marks the text as not changed. A Grep window has the
//! title `【Grep】key` (`${w?【Grep】$h$:...}`, the key cut at 32 characters,
//! `CSakuraEnvironment::ExpandParameter`), is not a window for loading files
//! (`CEditDoc::IsAcceptLoad`), cannot be closed while it searches (`CGrepAgent::OnBeforeClose`)
//! and does not ask to save when it is closed (`m_bGrepExitConfirm` is off by default,
//! `CEditDoc::OnFileClose`).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use gpui::{
    App, Context, Entity, Global, InteractiveElement, IntoElement, ParentElement, PromptLevel,
    Styled, Window, div, px, rgb,
};
use gpui_component::button::Button;
use ume_commands::clipboard::{ClipKind, Clipboard};
use ume_commands::command::CommandError;
use ume_core::buffer::{Eol, Pos};

use super::engine::{self, GrepError, GrepInfo, Outcome, Sink};
use super::panel::GrepPanel;
use super::settings::{DialogError, GrepSettings};
use crate::clipboard::GpuiClipboard;
use crate::document::SharedDocument;
use crate::file_ops::windows::{editors, open_editor_window};
use crate::view::EditorView;

/// The title of the message boxes of Grep.
const BOX_TITLE: &str = "UME Editor";

/// How many pieces of the result wait for the window at most (the engine sends 32 KB at most in
/// one).
const QUEUE_LEN: usize = 64;

/// The progress is sent at most this often.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);

/// What the dialog remembers from one Grep to the next, for every window
/// (`DllShareData::m_Common.m_sSearch` and `m_sSearchKeywords`). It lives as long as the program;
/// it is not saved in a file yet.
#[derive(Default)]
struct GrepMemory(GrepSettings);

impl Global for GrepMemory {}

/// The values the dialog starts with.
pub(crate) fn remembered(cx: &App) -> GrepSettings {
    cx.try_global::<GrepMemory>()
        .map(|memory| memory.0.clone())
        .unwrap_or_default()
}

pub(crate) fn remember(cx: &mut App, settings: GrepSettings) {
    cx.set_global(GrepMemory(settings));
}

/// The text of the clipboard for クリップボードの内容で置換 (`MyGetClipboardData` in
/// `CGrepAgent::DoGrep`), or `None` when there is none. A copy of a row ends with a line break
/// (the line ending of the result window, CR LF, is added if it has none).
pub(crate) fn clipboard_replacement(cx: &mut App) -> Option<String> {
    let data = GpuiClipboard(cx).get()?;
    if data.text.is_empty() {
        return None;
    }
    let mut text = data.text;
    if data.kind == ClipKind::Line && !text.ends_with(['\r', '\n']) {
        text.push_str("\r\n");
    }
    Some(text)
}

/// What a window knows about Grep.
#[derive(Default)]
pub struct GrepWindow {
    /// The key of the Grep whose result the window shows (`m_bGrepMode` and `m_szGrepKey`); a
    /// window that has one is a Grep window.
    key: Option<String>,
    /// The flag that stops the search that is running.
    running: Option<Arc<AtomicBool>>,
    /// The file being searched and the hits so far, for the bar that stops the search.
    progress: Option<(String, usize)>,
    /// The line where the result of the search that runs (or ran last) begins.
    start_line: usize,
    /// The dialog, while it is open.
    pub(crate) panel: Option<Entity<GrepPanel>>,
}

impl GrepWindow {
    /// The document was replaced (the file was closed): it is not a Grep result any more
    /// (`m_bGrepMode` is cleared). A search that is running keeps the window, which cannot be
    /// closed meanwhile.
    pub(crate) fn forget_result(&mut self) {
        if self.running.is_none() {
            self.key = None;
        }
    }

    /// Whether the window shows the result of a Grep.
    pub fn is_grep(&self) -> bool {
        self.key.is_some()
    }

    /// Whether a search is running in the window.
    pub fn is_running(&self) -> bool {
        self.running.is_some()
    }
}

/// What the search tells the window.
enum Event {
    /// A piece of the result.
    Text(String),
    /// The file being searched and the hits so far.
    Progress(String, usize),
    /// The end.
    Done(Result<Outcome, GrepError>),
}

/// The sink that sends to the window. A closed window stops the search.
struct ChannelSink {
    sender: async_channel::Sender<Event>,
    cancel: Arc<AtomicBool>,
    last_progress: Instant,
}

impl ChannelSink {
    fn send(&self, event: Event) {
        if self.sender.send_blocking(event).is_err() {
            self.cancel.store(true, Ordering::Relaxed);
        }
    }
}

impl Sink for ChannelSink {
    fn write(&mut self, text: &str) {
        self.send(Event::Text(text.to_owned()));
    }

    fn progress(&mut self, file: &str, hits: usize) {
        if self.last_progress.elapsed() >= PROGRESS_INTERVAL {
            self.last_progress = Instant::now();
            self.send(Event::Progress(file.to_owned(), hits));
        }
    }
}

/// The key as the title shows it: at most 32 UTF-16 code units, then `...`
/// (`CSakuraEnvironment::ExpandParameter`, `$h`).
pub fn title_key(key: &str) -> String {
    let cut = super::format::truncate_utf16(key, 32);
    if cut.len() < key.len() {
        format!("{cut}...")
    } else {
        cut.to_owned()
    }
}

impl EditorView {
    /// Whether the window shows the result of a Grep.
    pub fn is_grep_window(&self) -> bool {
        self.grep.is_grep()
    }

    /// Whether a Grep is running in the window.
    pub fn is_grep_running(&self) -> bool {
        self.grep.is_running()
    }

    /// The key the window searched for, or none for a window that is not a Grep window.
    pub(crate) fn grep_key(&self) -> Option<String> {
        self.grep.key.clone()
    }

    /// The text of a Grep window's tab, `【Grep】key`.
    pub(crate) fn grep_title(&self) -> Option<String> {
        self.grep
            .key
            .as_deref()
            .map(|key| format!("【Grep】{}", title_key(key)))
    }

    /// For tests of what a running search changes in the window: a search that never ends.
    #[cfg(test)]
    pub(crate) fn pretend_grep_running(&mut self, cx: &mut Context<Self>) {
        self.grep.key = Some(String::new());
        self.grep.running = Some(Arc::new(AtomicBool::new(false)));
        cx.notify();
    }

    #[cfg(test)]
    pub(crate) fn stop_pretending_grep(&mut self, cx: &mut Context<Self>) {
        self.grep.running = None;
        cx.notify();
    }

    /// Stops the Grep that is running (the cancel button).
    pub fn stop_grep(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = &self.grep.running {
            cancel.store(true, Ordering::Relaxed);
            cx.notify();
        }
    }

    /// Whether `Command_GREP` writes the result here: this is a Grep window that is not
    /// searching, or an untitled window without changes.
    pub(crate) fn grep_output_here(&self, cx: &App) -> bool {
        if self.grep.is_running() {
            return false;
        }
        let shared = self.document.read(cx);
        // A window in view mode refuses the text of the result.
        !shared.view_mode()
            && (self.grep.is_grep() || (shared.path().is_none() && !shared.model().is_modified()))
    }

    /// Runs a Grep. The result goes to this window when Sakura would write it here (unless
    /// `new_window`), else to a new window. A wrong condition is told in a box, in this window,
    /// and nothing starts. Returns whether the search started.
    pub fn start_grep(
        &mut self,
        info: GrepInfo,
        new_window: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Err(error) = engine::check(&info) {
            self.grep_error_box(&error, window, cx);
            return false;
        }
        if !new_window && self.grep_output_here(cx) {
            self.begin_grep(info, cx);
            return true;
        }
        // The result goes to a new tab of this window in tab mode, else to a new window
        // (`OpenNewEditor`). The tab changes the panes of this window, which hold this view, so it
        // is made after the command ends.
        let from = window.window_handle();
        if crate::tabs::settings::current(cx).bound() && !new_window {
            // A macro that called this waits for the tab to be made and the search to begin.
            let tracked = crate::macros::track(cx);
            window.defer(cx, move |window, app| {
                let _tracked = tracked;
                let opened = crate::tabs::flows::open_document(
                    app,
                    Some(window),
                    from,
                    false,
                    SharedDocument::empty(),
                    None,
                );
                if let Ok((_, target)) = opened {
                    target.update(app, |view, cx| view.begin_grep(info, cx));
                }
            });
            return true;
        }
        match open_editor_window(cx, SharedDocument::empty(), None) {
            Ok(handle) => {
                let target = editors(cx)
                    .into_iter()
                    .find(|(h, _)| *h == handle)
                    .map(|(_, view)| view);
                if let Some(target) = target {
                    target.update(cx, |view, cx| view.begin_grep(info, cx));
                    true
                } else {
                    false
                }
            }
            Err(error) => {
                self.notice = Some(format!("Grep の結果の窓を開けなかった: {error}"));
                cx.notify();
                false
            }
        }
    }

    /// The box for a mistake in the dialog (`WarningMessage`, `CheckRegexpSyntax`).
    pub(crate) fn grep_dialog_error_box(
        &self,
        error: &DialogError,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = if error.regex {
            crate::search::state::REGEX_ERROR_TITLE
        } else {
            BOX_TITLE
        };
        let answer = window.prompt(
            PromptLevel::Warning,
            title,
            Some(&error.message),
            &["OK"],
            cx,
        );
        crate::macros::keep_box(answer, window, cx);
    }

    /// The settings of a Grep 置換 as the `GrepInfo` of one. With `check` they are checked as the
    /// dialog does (`GetData`); the macro form does not (`CMacro::MakeGrepInfo`), `start_grep`
    /// still refuses a condition that is wrong. A clipboard that has no text is told in a box.
    pub(crate) fn grep_replace_info(
        &self,
        settings: &mut GrepSettings,
        check: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<GrepInfo, CommandError> {
        let clipboard = if settings.paste {
            let Some(text) = clipboard_replacement(cx) else {
                // `ErrorMessage( STR_DLGREPLC_CLIPBOARD )`
                let message = super::panel::NO_CLIPBOARD;
                let answer =
                    window.prompt(PromptLevel::Critical, BOX_TITLE, Some(message), &["OK"], cx);
                crate::macros::keep_box(answer, window, cx);
                return Err(CommandError::BadArgs(message.to_owned()));
            };
            Some(text)
        } else {
            None
        };
        if !check {
            return Ok(settings.replace_info(clipboard));
        }
        settings.check_replace(clipboard).map_err(|error| {
            self.grep_dialog_error_box(&error, window, cx);
            CommandError::BadArgs(error.message)
        })
    }

    /// The box for a condition that is wrong (`ErrorMessage` and `CheckRegexpSyntax`).
    pub(crate) fn grep_error_box(
        &self,
        error: &GrepError,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = match error {
            GrepError::Pattern(_) => crate::search::state::REGEX_ERROR_TITLE,
            _ => BOX_TITLE,
        };
        let answer = window.prompt(
            PromptLevel::Warning,
            title,
            Some(&error.message()),
            &["OK"],
            cx,
        );
        crate::macros::keep_box(answer, window, cx);
    }

    /// Starts the search with the result in this window. The search runs on a background task;
    /// the window gets the pieces of the result as they come.
    pub(crate) fn begin_grep(&mut self, info: GrepInfo, cx: &mut Context<Self>) {
        // The key is marked in the result (`m_strCurSearchKey`, `m_bCurSrchKeyMark`).
        self.search.set_options(info.options);
        self.search.set_pattern(&info.key);
        self.search.set_mark(true);
        // The key goes in the history (`CDlgGrep::GetData`, `CNormalProcess` for the command line).
        self.remember_search_key(cx);
        // The type of a Grep window (`grepout`) writes CR LF, as the result does.
        let empty = self.document.read(cx).doc().line_count() == 0;
        if empty {
            let mut defaults = crate::settings::new_document_defaults(cx, None);
            defaults.eol = Eol::CrLf;
            self.document
                .update(cx, |shared, _| shared.set_new_document_defaults(&defaults));
        }
        self.grep.start_line = self
            .document
            .read(cx)
            .doc()
            .end_pos()
            .map_or(0, |pos| pos.line);
        let cancel = Arc::new(AtomicBool::new(false));
        self.grep.key = Some(info.key.clone());
        self.grep.running = Some(cancel.clone());
        self.grep.progress = None;
        cx.notify();

        // Bounded: a search that finds faster than the window writes waits for it, so that the
        // pieces do not pile up in memory and a stop is not behind a long queue.
        let (sender, receiver) = async_channel::bounded::<Event>(QUEUE_LEN);
        let flag = cancel.clone();
        let task = cx.background_executor().spawn(async move {
            let mut sink = ChannelSink {
                sender: sender.clone(),
                cancel: flag.clone(),
                last_progress: Instant::now() - PROGRESS_INTERVAL,
            };
            let result = engine::run(&info, &flag, &mut sink);
            let _ = sender.send_blocking(Event::Done(result));
        });
        cx.spawn(async move |this, cx| {
            while let Ok(event) = receiver.recv().await {
                let finished = this.update(cx, |view, cx| view.grep_event(event, cx));
                match finished {
                    Ok(false) => {}
                    Ok(true) => break,
                    Err(_) => {
                        // the window is gone
                        cancel.store(true, Ordering::Relaxed);
                        break;
                    }
                }
            }
            task.await;
        })
        .detach();
    }

    /// Takes what the search sends. Returns whether it has ended.
    fn grep_event(&mut self, event: Event, cx: &mut Context<Self>) -> bool {
        match event {
            Event::Text(text) => {
                self.grep_append(&text, cx);
                false
            }
            Event::Progress(file, hits) => {
                self.grep.progress = Some((file, hits));
                cx.notify();
                false
            }
            Event::Done(result) => {
                self.grep.running = None;
                self.grep.progress = None;
                match result {
                    Ok(_) => self.finish_grep(cx),
                    Err(error) => {
                        self.notice = Some(error.message());
                    }
                }
                cx.notify();
                true
            }
        }
    }

    /// `AddTail`: adds the text at the end of the document. The caret stays where it was.
    fn grep_append(&mut self, text: &str, cx: &mut Context<Self>) {
        for attempt in 0..2 {
            let result = self.run_with(cx, |c| -> Result<(), CommandError> {
                let end = c.doc().end_pos()?;
                c.begin_block();
                let result = c.replace(end, end, text);
                c.end_block();
                result.map(|_| ())
            });
            if result.is_ok() {
                return;
            }
            if attempt == 0 {
                // A box selection refuses to be edited around: end it, and write again.
                let caret = self.cursor.caret();
                self.run_with(cx, |c| c.move_caret(caret, false)).ok();
            }
        }
        // View mode, turned on while the search runs.
        self.notice = Some("Grep の結果を書き込めなかった".to_owned());
        cx.notify();
    }

    /// The end of a search: the caret goes back to where the result begins and the text counts as
    /// not changed (`DoGrep`).
    fn finish_grep(&mut self, cx: &mut Context<Self>) {
        let start = Pos::new(self.grep.start_line, 0);
        self.run_with(cx, |c| c.move_caret(start, false)).ok();
        let cursor = self.cursor.clone();
        self.document.update(cx, |shared, _| {
            shared.parts_mut().0.mark_saved(&cursor);
        });
        self.ensure_caret_visible(cx);
    }

    /// The bar over the window that shows the search and stops it (the cancel dialog of Sakura:
    /// the file, the hits and the button).
    pub(crate) fn grep_progress_bar(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if !self.grep.is_running() {
            return None;
        }
        let (file, hits) = self.grep.progress.clone().unwrap_or_default();
        Some(
            div()
                .id("grep-progress")
                .absolute()
                .occlude()
                .bottom(px(8.))
                .right(px(24.))
                .p_2()
                .gap_2()
                .flex()
                .items_center()
                .bg(rgb(0xf6f6f6))
                .text_color(rgb(0x1e1e1e))
                .text_size(px(13.))
                .border_1()
                .border_color(rgb(0xb0b0b0))
                .rounded_md()
                .shadow_md()
                .child(format!("Grep 中  {hits} 個  {file}"))
                .child(
                    Button::new("grep-stop")
                        .label("中断")
                        .on_click(cx.listener(|view, _, _, cx| view.stop_grep(cx))),
                ),
        )
    }
}
