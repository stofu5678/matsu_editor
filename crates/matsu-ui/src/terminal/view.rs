//! The view of a shell tab: the screen of one terminal, its keys, and the events of the shell.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, Bounds, Context, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    KeyDownEvent, Keystroke, Modifiers, MouseButton, ParentElement as _, Pixels, Render,
    Styled as _, Task, WeakEntity, Window, div, rgb,
};
use matsu_commands::keymap::Lookup;
use matsu_commands::sakura::{FUNCTIONS, FunctionKind};
use matsu_terminal::{EventReceiver, Snapshot, Terminal, TerminalEvent, TerminalSize};

use super::element::{CellSize, TerminalElement};
use super::input::{ScrollKey, Translated, translate_with};
use super::palette::Palette;
use super::{Close, Copy, Paste};
use crate::app::Workspace;
use crate::status::StatusInfo;
use crate::view::RunFunction;
use crate::view::text_font::TextFont;

mod clipboard;
mod ime;
mod mouse;
mod program;

pub(super) use ime::{preedit_cells, rows_up};

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
    /// The name of the shell's program: the caption of the tab until the program sets a title.
    program: String,
    /// What the tooltip says: the shell and its working directory.
    tooltip: String,
    size: TerminalSize,
    /// The size changed and the window has not been told (its status bar shows the size).
    size_told: bool,
    ended: Option<Ended>,
    /// A `Redraw` is on its way to this view (see [`should_forward`]).
    redraw_waiting: Arc<AtomicBool>,
    /// The text the IME is composing, drawn at the cursor and not yet written (`view::ime`).
    marked: Option<ime::Marked>,
    /// Where the screen was drawn last, for the mouse and the IME's windows.
    layout: Option<Layout>,
    /// The left button is down after a press on the screen: the selection follows the pointer.
    dragging: bool,
    /// A question about a paste of several lines is open.
    paste_asking: bool,
    /// The title the program set (OSC 0 and 2), `None` for the shell's own name (`view::program`).
    title: Option<String>,
    bell: program::BellGate,
    /// A bell rang while the tab or the window was in the background and the tab has not been looked at since.
    bell_marked: bool,
    /// The screen flashes: a bell rang while the terminal was looked at.
    bell_flash: bool,
    /// How many bells got through the gate.
    #[cfg(all(test, unix))]
    bells_rung: usize,
    /// The button the program was told is down (a press that was reported): its drag and release are the
    /// program's too (`view::mouse`).
    reported: Option<matsu_terminal::MouseButton>,
    /// The cell of the last motion that was reported, so that a pointer inside a cell reports once.
    last_cell: Option<(usize, usize)>,
    /// Lines of the wheel not yet turned into a whole line (a trackpad sends pixels).
    wheel_rest: f32,
    wheel_rest_x: f32,
    wheel_dest: mouse::WheelDest,
    /// The pointer and the modifiers as of the last mouse or modifier event, for the link under it.
    pointer: Option<gpui::Point<Pixels>>,
    last_modifiers: Modifiers,
    /// The link under the pointer while the link key is held (it is underlined).
    link: Option<matsu_terminal::Link>,
    /// A question about a link is open.
    link_asking: bool,
    _events: Option<Task<()>>,
}

/// Where the screen of a terminal was drawn: its place in the window and the size of its cells.
#[derive(Clone, Copy, Debug)]
struct Layout {
    bounds: Bounds<Pixels>,
    cell: CellSize,
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
            marked: None,
            layout: None,
            dragging: false,
            paste_asking: false,
            title: None,
            bell: program::BellGate::default(),
            bell_marked: false,
            bell_flash: false,
            #[cfg(all(test, unix))]
            bells_rung: 0,
            reported: None,
            last_cell: None,
            wheel_rest: 0.0,
            wheel_rest_x: 0.0,
            wheel_dest: mouse::WheelDest::Scroll,
            pointer: None,
            last_modifiers: Modifiers::default(),
            link: None,
            link_asking: false,
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
        match event {
            TerminalEvent::Redraw => {
                // Cleared before the screen is read, so a change after the read sends another one.
                self.redraw_waiting.store(false, Ordering::Release);
                cx.notify();
                if self.link.is_some() || mouse::link_modifier(self.last_modifiers) {
                    // The lines moved under the pointer: is it on a link now, or still?
                    self.refresh_link(cx);
                }
                if std::mem::take(&mut self.size_told) {
                    self.tell_window(cx);
                }
            }
            TerminalEvent::ChildExit { code } => {
                let code = *code;
                self.ended = Some(Ended { code });
                // Nothing can be written any more: the text being composed has no place to go.
                self.end_composition();
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
            TerminalEvent::Title(title) => self.set_title(title, cx),
            TerminalEvent::ResetTitle => self.reset_title(cx),
            TerminalEvent::Bell => self.on_bell(window, cx),
            TerminalEvent::ClipboardStore { kind, text } => {
                Self::store_clipboard(*kind, text.clone(), cx);
            }
        }
    }

    fn on_mouse_up_event(
        &mut self,
        event: &gpui::MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.on_mouse_up(event, cx);
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

    /// The screen was laid out in `bounds` with cells of `cell`: the mouse and the IME's windows go by it.
    pub(super) fn set_layout(&mut self, bounds: Bounds<Pixels>, cell: CellSize) {
        self.layout = Some(Layout { bounds, cell });
    }

    /// Whether the shell is still running: closing the tab would end it.
    pub(crate) fn is_running(&self) -> bool {
        self.ended.is_none() && self.terminal.is_running()
    }

    pub(crate) fn ended(&self) -> Option<Ended> {
        self.ended
    }

    /// The tooltip of the tab: the shell and its working directory, led by the title the program set, if it
    /// has (the caption shows the title instead of the shell's name, so the tooltip is where the shell is named).
    pub(crate) fn tooltip(&self) -> String {
        match &self.title {
            Some(title) => format!("{title}\n{}", self.tooltip),
            None => self.tooltip.clone(),
        }
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

    /// How many lines the scrollback holds now (for the tests).
    #[cfg(test)]
    pub(crate) fn history_size(&self) -> usize {
        self.terminal.snapshot().info.history_size
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
        let mac = cfg!(target_os = "macos");
        // Read at every key: a change of the setting reaches the tabs that are open.
        let option_as_meta = super::settings::current(cx).option_as_meta;
        // The window's keys stay with the window (also while the IME composes: one that comes here is one
        // the IME let go).
        if let Some(code) = window_function_of(&event.keystroke, event.prefer_character_input, cx) {
            cx.stop_propagation();
            self.in_workspace(window, cx, move |ws, window, cx| {
                ws.run_after_event(code, window, cx);
            });
            return;
        }
        if self.is_composing() {
            // The IME has the keys while it composes: Enter commits, Backspace and the arrows edit the
            // text. The platform gives the IME the keys first, so one that arrives here is one it let go;
            // it is not the program's. A key with text goes on to the input handler (the second key of a
            // dead key, a character the IME declined).
            // (The keys of the menus and the window that `translate` passes stay free.)
            if !matches!(
                translate_with(
                    &event.keystroke,
                    event.prefer_character_input,
                    mac,
                    option_as_meta,
                ),
                Translated::Text(_) | Translated::Pass
            ) {
                cx.stop_propagation();
            }
            return;
        }
        match translate_with(
            &event.keystroke,
            event.prefer_character_input,
            mac,
            option_as_meta,
        ) {
            // A character typed with Ctrl and Alt both down (AltGr on Windows, a layout's level on others):
            // the Linux backends of GPUI hand a key's text to the input handler only when no modifier but
            // Shift is down, so it is written here. (No `WM_CHAR` follows a handled key on Windows.)
            Translated::Text(text)
                if event.keystroke.modifiers.control && event.keystroke.modifiers.alt =>
            {
                cx.stop_propagation();
                self.send_text(&text);
            }
            // Other text is the input handler's (`view::ime`), whatever the platform does with the key.
            Translated::Pass | Translated::Text(_) => {}
            Translated::Ignore => cx.stop_propagation(),
            Translated::Key(key, mods) => {
                cx.stop_propagation();
                self.send_key(key, mods);
            }
            Translated::CopyOrKey(key, mods) => {
                cx.stop_propagation();
                if !self.copy_selection(cx) {
                    self.send_key(key, mods);
                }
                cx.notify();
            }
            Translated::Copy => {
                cx.stop_propagation();
                self.copy_selection(cx);
                cx.notify();
            }
            Translated::Paste => {
                cx.stop_propagation();
                self.paste(window, cx);
            }
            Translated::Scroll(key) => {
                cx.stop_propagation();
                self.scroll_by_key(key);
                cx.notify();
            }
        }
    }

    /// Scrolls the view through the scrollback for one of the scroll keys. A page is the screen less a line,
    /// so the line at the edge stays in sight.
    fn scroll_by_key(&mut self, key: ScrollKey) {
        let page = i32::from(self.size.rows).saturating_sub(1).max(1);
        match key {
            ScrollKey::LineUp => self.terminal.scroll_display(1),
            ScrollKey::LineDown => self.terminal.scroll_display(-1),
            ScrollKey::PageUp => self.terminal.scroll_display(page),
            ScrollKey::PageDown => self.terminal.scroll_display(-page),
            ScrollKey::Top => self.terminal.scroll_to_top(),
            ScrollKey::Bottom => self.terminal.scroll_to_bottom(),
        }
    }

    /// Writes a key to the program. A key that is sent clears the selection, as typing does.
    fn send_key(&mut self, key: matsu_terminal::Key, mods: matsu_terminal::Modifiers) {
        if let Some(bytes) = self.terminal.encode_key(key, mods) {
            self.send_typed(&bytes);
        }
    }

    /// Writes what the user typed, pasted or composed. Typing clears the selection (as in Windows Terminal)
    /// and brings the view back to the live screen when it was scrolled back into the scrollback.
    fn send_typed(&mut self, bytes: &[u8]) {
        self.clear_selection();
        if self.terminal.info().display_offset > 0 {
            self.terminal.scroll_to_bottom();
        }
        self.send(bytes);
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
        let name = format!("シェル: {}", self.title_text());
        // The copy item of the menu is gray while nothing is selected: the native menu of macOS grays an
        // item whose action nobody handles.
        let selected = self.has_selection();
        crate::a11y::terminal_region(div().id("terminal"), &name)
            .key_context("Terminal")
            .track_focus(&self.focus)
            .capture_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::on_mouse_down))
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up_event))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_mouse_up_event))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::on_mouse_up_event))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up_event))
            .on_mouse_up_out(MouseButton::Middle, cx.listener(Self::on_mouse_up_event))
            .on_mouse_up_out(MouseButton::Right, cx.listener(Self::on_mouse_up_event))
            .on_scroll_wheel(cx.listener(|this, event, _, cx| this.on_wheel(event, cx)))
            .on_modifiers_changed(cx.listener(|this, event, _, cx| {
                this.on_modifiers_changed(event, cx);
            }))
            .when(selected, |el| {
                el.on_action(cx.listener(|this, _: &Copy, _, cx| {
                    this.copy_selection(cx);
                    cx.notify();
                }))
            })
            .on_action(cx.listener(|this, _: &Paste, window, cx| this.paste(window, cx)))
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
