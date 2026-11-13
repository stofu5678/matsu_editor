//! The view of a shell tab: the screen of one terminal, its keys, and the events of the shell.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui::{
    App, Context, FocusHandle, Focusable, InteractiveElement as _, IntoElement, KeyDownEvent,
    Keystroke, MouseButton, ParentElement as _, Render, Styled as _, Task, WeakEntity, Window, div,
    rgb,
};
use matsu_commands::keymap::Lookup;
use matsu_commands::sakura::{FUNCTIONS, FunctionKind};
use matsu_terminal::{EventReceiver, Snapshot, Terminal, TerminalEvent, TerminalSize};

use super::Close;
use super::element::{CellSize, TerminalElement};
use super::input::{Translated, translate};
use super::palette::Palette;
use crate::app::Workspace;
use crate::status::StatusInfo;
use crate::view::RunFunction;
use crate::view::text_font::TextFont;

/// The group of the functions of the window (`FUNCTION_GROUPS`: ウィンドウ系). The keys of its functions stay
/// with the window while a terminal has the focus (ADR-0028 point 9).
const WINDOW_GROUP: u8 = 12;

/// The functions of the window group that stay out of it: the ones that split the panes of a document (a
/// terminal has none), and `F_REDRAW`, whose key (F5) is a key of the programs in a terminal and which has
/// nothing to draw again (the terminal paints what it has).
const NOT_FOR_A_TERMINAL: &[&str] = &["F_SPLIT_V", "F_SPLIT_H", "F_SPLIT_VH", "F_REDRAW"];

/// Whether the function with `code` is one of the window's, whose keys the terminal leaves alone.
pub(crate) fn is_window_function(code: u16) -> bool {
    FUNCTIONS.iter().any(|f| {
        f.code == code
            && f.kind == FunctionKind::Command
            && f.groups.contains(&WINDOW_GROUP)
            && !NOT_FOR_A_TERMINAL.contains(&f.name)
    })
}

/// The window function that the user's key map binds to the key, if there is one.
fn window_function_of(
    keystroke: &Keystroke,
    prefer_character_input: bool,
    cx: &App,
) -> Option<u16> {
    // A character the layout wants typed (AltGr+E is a euro sign on some layouts, and GPUI reports it as
    // Ctrl+Alt+E) is not a key of the window.
    if prefer_character_input {
        return None;
    }
    let chord = crate::keys::chord_of(keystroke)?;
    let code = match crate::app::with_key_map(cx, |map| map.lookup(chord)) {
        Lookup::Command(reg) => FUNCTIONS
            .iter()
            .find(|f| f.name == reg.sakura)
            .map(|f| f.code),
        Lookup::NotImplemented {
            function: Some(f), ..
        } => Some(f.code),
        _ => None,
    }?;
    is_window_function(code).then_some(code)
}

/// What the screen is drawn with: the editor's text font and size, and the colors.
#[derive(Clone)]
pub(super) struct Look {
    pub font: TextFont,
    pub size: f32,
    pub palette: Palette,
}

/// How the shell ended, when it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Ended {
    /// The exit code, `None` when it is not known (on Unix also when a signal ended the shell).
    pub code: Option<i32>,
}

/// One terminal in a tab.
pub(crate) struct TerminalView {
    terminal: Terminal,
    focus: FocusHandle,
    workspace: WeakEntity<Workspace>,
    look: Look,
    /// The name of the shell's program, for the caption.
    program: String,
    /// What the tooltip says: the shell and its working directory.
    tooltip: String,
    size: TerminalSize,
    /// The size changed and the window has not been told (its status bar shows the size).
    size_told: bool,
    ended: Option<Ended>,
    /// A `Redraw` is on its way to this view (see [`should_forward`]).
    redraw_waiting: Arc<AtomicBool>,
    _events: Option<Task<()>>,
}

/// Whether the bridge hands `event` to the UI. The core reports one `Redraw` until the receiver has read it,
/// but the bridge reads at once, so on its own every redraw would pile up in the channel while the UI is busy.
/// A `Redraw` is therefore forwarded only when none is waiting (`waiting` is set here, cleared by the view when it
/// handles the event); every other event is forwarded.
fn should_forward(waiting: &AtomicBool, event: &TerminalEvent) -> bool {
    *event != TerminalEvent::Redraw || !waiting.swap(true, Ordering::AcqRel)
}

impl TerminalView {
    pub(super) fn new(
        terminal: Terminal,
        size: TerminalSize,
        look: Look,
        (program, tooltip): (String, String),
        workspace: WeakEntity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            terminal,
            focus: cx.focus_handle(),
            workspace,
            look,
            program,
            tooltip,
            size,
            size_told: true,
            ended: None,
            redraw_waiting: Arc::new(AtomicBool::new(false)),
            _events: None,
        }
    }

    /// Starts bringing the events of the terminal to this view. The receiver blocks, so a thread waits on it and
    /// hands the events to the UI thread through a channel; it ends when the terminal is gone.
    pub(super) fn start_events(
        &mut self,
        events: EventReceiver,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let (sender, receiver) = async_channel::unbounded();
        let waiting = Arc::clone(&self.redraw_waiting);
        let spawned = std::thread::Builder::new()
            .name("matsu-terminal-events".to_owned())
            .spawn(move || {
                while let Ok(event) = events.recv() {
                    if should_forward(&waiting, &event) && sender.send_blocking(event).is_err() {
                        break;
                    }
                }
            });
        if spawned.is_err() {
            return;
        }
        self._events = Some(cx.spawn_in(window, async move |this, cx| {
            while let Ok(event) = receiver.recv().await {
                let alive = this
                    .update_in(cx, |this, window, cx| this.handle_event(&event, window, cx))
                    .is_ok();
                if !alive {
                    break;
                }
            }
        }));
    }

    fn handle_event(&mut self, event: &TerminalEvent, window: &mut Window, cx: &mut Context<Self>) {
        match *event {
            TerminalEvent::Redraw => {
                // Cleared before the screen is read, so a change after the read sends another one.
                self.redraw_waiting.store(false, Ordering::Release);
                cx.notify();
                if std::mem::take(&mut self.size_told) {
                    self.tell_window(cx);
                }
            }
            TerminalEvent::ChildExit { code } => {
                self.ended = Some(Ended { code });
                cx.notify();
                if code == Some(0) {
                    // Like Windows Terminal's default: a shell that ends well takes its tab with it.
                    let host = self.workspace.clone();
                    let me = cx.entity();
                    window.defer(cx, move |window, cx| {
                        host.update(cx, |ws, cx| ws.close_shell_now(&me, window, cx))
                            .ok();
                    });
                } else {
                    self.tell_window(cx);
                }
            }
            // The program's title, the bell and the clipboard are the fourth PR (ADR-0028): the events are
            // taken off the channel and dropped.
            TerminalEvent::Title(_)
            | TerminalEvent::ResetTitle
            | TerminalEvent::Bell
            | TerminalEvent::ClipboardStore { .. } => {}
        }
    }

    /// The window shows things of this tab (the status bar, the caption): it is drawn again.
    fn tell_window(&self, cx: &mut Context<Self>) {
        self.workspace.update(cx, |_, cx| cx.notify()).ok();
    }

    /// The font, its size and the default colors (`0xRRGGBB`, foreground then background) this shell was
    /// opened with: another shell opened from this tab looks the same.
    pub(crate) fn text_look(&self) -> (TextFont, f32, u32, u32) {
        (
            self.look.font.clone(),
            self.look.size,
            self.look.palette.foreground,
            self.look.palette.background,
        )
    }

    /// Gives the shell another size and colors (for the tests).
    #[cfg(test)]
    pub(crate) fn set_text_look(&mut self, size: f32, foreground: u32, background: u32) {
        self.look.size = size;
        self.look.palette = Palette::new(foreground, background);
    }

    pub(super) fn look(&self) -> &Look {
        &self.look
    }

    pub(super) fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// A copy of the screen to draw.
    pub(super) fn screen(&self) -> Snapshot {
        self.terminal.snapshot()
    }

    /// The screen has room for `columns` x `rows` cells of `cell`: the terminal is resized when that is not its
    /// size already (at least 2 x 1), which tells the program.
    pub(super) fn fit(&mut self, columns: u16, rows: u16, cell: CellSize) {
        let size = TerminalSize {
            columns,
            rows,
            cell_width: cell.width.round() as u16,
            cell_height: cell.height.round() as u16,
        };
        if size == self.size {
            return;
        }
        self.size = size;
        self.size_told = true;
        // An error means the shell has ended: the screen is resized all the same.
        let _ = self.terminal.resize(size);
    }

    /// Whether the shell is still running: closing the tab would end it.
    pub(crate) fn is_running(&self) -> bool {
        self.ended.is_none() && self.terminal.is_running()
    }

    pub(crate) fn ended(&self) -> Option<Ended> {
        self.ended
    }

    /// The name of the shell's program (the caption of the tab).
    pub(crate) fn program(&self) -> &str {
        &self.program
    }

    pub(crate) fn tooltip(&self) -> &str {
        &self.tooltip
    }

    /// The size in cells, `80×24`.
    pub(crate) fn size_text(&self) -> String {
        format!("{}×{}", self.size.columns, self.size.rows)
    }

    #[cfg(test)]
    pub(crate) fn size(&self) -> TerminalSize {
        self.size
    }

    /// The size the terminal itself has, `(columns, rows)` (for the tests).
    #[cfg(test)]
    pub(crate) fn grid(&self) -> (usize, usize) {
        let info = self.terminal.info();
        (info.columns, info.rows)
    }

    /// What the status bar says: the size, and how the shell ended.
    pub(crate) fn status(&self) -> StatusInfo {
        let notice = match self.ended {
            None => self.size_text(),
            Some(Ended { code: Some(code) }) => format!(
                "シェルが終了しました（終了コード {code}）  {}",
                self.size_text()
            ),
            Some(Ended { code: None }) => {
                format!("シェルが終了しました  {}", self.size_text())
            }
        };
        StatusInfo {
            notice: Some(notice),
            message_only: true,
            ..StatusInfo::default()
        }
    }

    /// The text of the screen, a line for each row without its trailing blanks (for the tests).
    #[cfg(test)]
    pub(crate) fn screen_text(&self) -> String {
        let snapshot = self.terminal.snapshot();
        let mut lines: Vec<String> = snapshot
            .rows
            .iter()
            .map(matsu_terminal::Row::text)
            .collect();
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        lines.join("\n")
    }

    /// Ends the shell now (the end of the editor): the terminal closes its PTY on another thread.
    pub(crate) fn shutdown(&mut self) {
        self.terminal.shutdown();
    }

    fn send(&self, bytes: &[u8]) {
        if self.ended.is_none() {
            // A write to a shell that has ended fails; nothing is shown for it.
            let _ = self.terminal.write(bytes);
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // The window's keys stay with the window.
        if let Some(code) = window_function_of(&event.keystroke, event.prefer_character_input, cx) {
            cx.stop_propagation();
            self.in_workspace(window, cx, move |ws, window, cx| {
                ws.run_after_event(code, window, cx);
            });
            return;
        }
        match translate(
            &event.keystroke,
            event.prefer_character_input,
            cfg!(target_os = "macos"),
        ) {
            Translated::Pass => {}
            Translated::Ignore => cx.stop_propagation(),
            Translated::Key(key, mods) => {
                cx.stop_propagation();
                if let Some(bytes) = self.terminal.encode_key(key, mods) {
                    self.send(&bytes);
                }
            }
            Translated::Text(text) => {
                cx.stop_propagation();
                self.send(text.as_bytes());
            }
        }
    }

    /// Runs `f` on the window after the event that is running ends: the window changes the tabs, and this view
    /// is one of them.
    fn in_workspace(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) + 'static,
    ) {
        let host = self.workspace.clone();
        window.defer(cx, move |window, cx| {
            host.update(cx, |workspace, cx| f(workspace, window, cx))
                .ok();
        });
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for TerminalView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let name = format!("シェル: {}", self.program);
        crate::a11y::terminal_region(div().id("terminal"), &name)
            .key_context("Terminal")
            .track_focus(&self.focus)
            .capture_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| window.focus(&this.focus, cx)),
            )
            .on_action(cx.listener(|this, action: &RunFunction, window, cx| {
                let code = action.code;
                this.in_workspace(window, cx, move |ws, window, cx| {
                    ws.run_after_event(code, window, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &Close, window, cx| {
                this.in_workspace(window, cx, |ws, window, cx| {
                    ws.close_active_own_tab(window, cx);
                });
            }))
            .size_full()
            .overflow_hidden()
            .bg(rgb(self.look.palette.default_background()))
            .child(TerminalElement::new(cx.entity()))
    }
}

/// The folder the shell starts in, for the tooltip.
pub(super) fn tooltip_text(program: &str, directory: Option<&PathBuf>) -> String {
    match directory {
        Some(directory) => format!("シェル: {program}\n{}", directory.display()),
        None => format!("シェル: {program}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_redraw_waits_for_the_view_and_the_other_events_all_go() {
        let waiting = AtomicBool::new(false);
        let redraw = TerminalEvent::Redraw;
        assert!(should_forward(&waiting, &redraw));
        // the view has not handled it yet: the next ones are left out
        assert!(!should_forward(&waiting, &redraw));
        assert!(!should_forward(&waiting, &redraw));
        // the other events are never dropped
        assert!(should_forward(
            &waiting,
            &TerminalEvent::ChildExit { code: Some(3) }
        ));
        assert!(should_forward(&waiting, &TerminalEvent::Bell));
        assert!(should_forward(&waiting, &TerminalEvent::Title("t".into())));
        // the view handled it: the next redraw goes
        waiting.store(false, Ordering::Release);
        assert!(should_forward(&waiting, &redraw));
    }
}
