//! The mouse of the terminal (ADR-0028 points 5 and 9): reports to the program, the wheel, and links.
//!
//! Who has the mouse follows xterm and Windows Terminal. While the program has turned mouse reporting on,
//! presses, releases, drags and the wheel are encoded and written to it (`matsu_terminal::encode_mouse`), and
//! **Shift held with the mouse is the user's**: it selects, scrolls and opens links as it does without a
//! program that wants the mouse. Without reporting, the left button selects (`view::clipboard`), the wheel
//! scrolls the view back through the scrollback, and on the alternate screen it is sent as arrow keys when the
//! program asked for alternate scroll (xterm and alacritty do the same). Ctrl+click (Cmd+click on macOS)
//! opens the link under the pointer after asking; the link is underlined while the key is held over it.

use gpui::{
    Context, Modifiers, ModifiersChangedEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, PromptLevel, ScrollWheelEvent, Window, px,
};
use matsu_terminal::{
    Link, MouseAction, WheelDirection, display_uri, encode_mouse, encode_wheel_as_arrows,
    is_openable,
};

use super::TerminalView;
use crate::file_ops::ask::{confirm, tell};

/// Where a turn of the wheel goes (see `TerminalView::on_wheel`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WheelDest {
    /// Reports to the program that has the mouse.
    Report,
    /// Arrow keys on the alternate screen.
    Arrows,
    /// The scrollback.
    Scroll,
}

/// The cell of a button the program is told about, or `None` for the buttons that are not reported (back and
/// forward).
fn report_button(button: MouseButton) -> Option<matsu_terminal::MouseButton> {
    match button {
        MouseButton::Left => Some(matsu_terminal::MouseButton::Left),
        MouseButton::Middle => Some(matsu_terminal::MouseButton::Middle),
        MouseButton::Right => Some(matsu_terminal::MouseButton::Right),
        MouseButton::Navigate(_) => None,
    }
}

fn gpui_button(button: matsu_terminal::MouseButton) -> MouseButton {
    match button {
        matsu_terminal::MouseButton::Left => MouseButton::Left,
        matsu_terminal::MouseButton::Middle => MouseButton::Middle,
        matsu_terminal::MouseButton::Right => MouseButton::Right,
    }
}

fn terminal_modifiers(modifiers: Modifiers) -> matsu_terminal::Modifiers {
    matsu_terminal::Modifiers {
        shift: modifiers.shift,
        ctrl: modifiers.control,
        alt: modifiers.alt,
    }
}

/// Whether the modifiers are the ones that open links: Ctrl, or Cmd on macOS.
pub(super) fn link_modifier(modifiers: Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.platform
    } else {
        modifiers.control
    }
}

/// The question before a link is opened: the address is shown in full (cut when it is very long).
fn open_message(uri: &str) -> String {
    format!(
        "次のリンクをブラウザーで開きますか?\n{}",
        display_uri(uri, 300)
    )
}

/// What is said about a link that is not opened: only `http` and `https` are, and not with a user name in the
/// address (`is_openable`).
fn refuse_message(uri: &str) -> String {
    format!(
        "このリンクは開けません。開けるのは http と https のリンクだけです（ユーザー名つきのアドレスも開きません）。\n{}",
        display_uri(uri, 300)
    )
}

impl TerminalView {
    /// Whether the mouse is the program's now: it asked for reports, and Shift is not held.
    fn reporting(&self, shift: bool) -> bool {
        !shift && self.terminal.modes().mouse_reporting()
    }

    /// The cell under a point of the window as `(row, column)` of the view (a point outside is pulled to the
    /// nearest cell).
    fn cell(&self, position: Point<Pixels>) -> Option<(usize, usize)> {
        let (row, column, _) = self.cell_at(position)?;
        Some((usize::try_from(row).unwrap_or(0), column))
    }

    /// Whether a point of the window is on the screen of the terminal.
    fn on_screen(&self, position: Point<Pixels>) -> bool {
        self.layout
            .is_some_and(|layout| layout.bounds.contains(&position))
    }

    /// Writes the report of a mouse event. `false` when the program does not want it.
    fn report(
        &mut self,
        action: MouseAction,
        modifiers: Modifiers,
        row: usize,
        column: usize,
    ) -> bool {
        match self
            .terminal
            .encode_mouse(action, terminal_modifiers(modifiers), row, column)
        {
            Some(bytes) => {
                self.send(&bytes);
                true
            }
            None => false,
        }
    }

    pub(super) fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus, cx);
        self.pointer = Some(event.position);
        self.last_modifiers = event.modifiers;
        if self.reporting(event.modifiers.shift) {
            let Some(button) = report_button(event.button) else {
                return;
            };
            // A second button pressed while one is down is not reported: the release that the program
            // waits for is the first button's.
            if self.reported.is_some() {
                return;
            }
            if let Some((row, column)) = self.cell(event.position)
                && self.report(MouseAction::Press(button), event.modifiers, row, column)
            {
                self.reported = Some(button);
                self.last_cell = Some((row, column));
            }
            return;
        }
        if event.button == MouseButton::Left
            && link_modifier(event.modifiers)
            && let Some(link) = self.link_under(event.position)
        {
            self.open_link(link, window, cx);
            return;
        }
        self.select_press(event, cx);
    }

    pub(in crate::terminal) fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pointer = Some(event.position);
        self.last_modifiers = event.modifiers;
        if self.report_motion(event) {
            return;
        }
        self.select_move(event, cx);
        self.refresh_link(cx);
    }

    /// Reports the pointer moving to another cell, while the program has the mouse. `true` when the event was
    /// the program's (a button it was told about is down, or it wants all motion).
    fn report_motion(&mut self, event: &MouseMoveEvent) -> bool {
        if let Some(button) = self.reported {
            if event.pressed_button != Some(gpui_button(button)) {
                // The button went up where the window did not see it: the program is told now, at the cell of
                // the last report, so that it does not stay in the drag.
                self.reported = None;
                if let Some((row, column)) = self.last_cell.take() {
                    self.report(MouseAction::Release(button), event.modifiers, row, column);
                }
                return false;
            }
            if let Some((row, column)) = self.cell(event.position) {
                self.report_moved(MouseAction::Move(Some(button)), event, row, column);
            }
            return true;
        }
        if event.pressed_button.is_none()
            && self.on_screen(event.position)
            && self.reporting(event.modifiers.shift)
        {
            if let Some((row, column)) = self.cell(event.position) {
                self.report_moved(MouseAction::Move(None), event, row, column);
            }
            return true;
        }
        false
    }

    /// Writes a motion report when the pointer is in another cell than the last one reported.
    fn report_moved(
        &mut self,
        action: MouseAction,
        event: &MouseMoveEvent,
        row: usize,
        column: usize,
    ) {
        if self.last_cell != Some((row, column)) {
            self.last_cell = Some((row, column));
            self.report(action, event.modifiers, row, column);
        }
    }

    pub(super) fn on_mouse_up(&mut self, event: &MouseUpEvent, cx: &mut Context<Self>) {
        self.pointer = Some(event.position);
        self.last_modifiers = event.modifiers;
        if let Some(button) = self.reported
            && report_button(event.button) == Some(button)
        {
            self.reported = None;
            self.last_cell = None;
            // The program may have turned reporting off since the press: nothing is written then.
            if let Some((row, column)) = self.cell(event.position) {
                self.report(MouseAction::Release(button), event.modifiers, row, column);
            }
            return;
        }
        self.select_release(event, cx);
    }

    /// The wheel: reports to the program, arrow keys on the alternate screen, or the scrollback.
    pub(super) fn on_wheel(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let Some(layout) = self.layout else {
            return;
        };
        let line = layout.cell.height;
        let delta = event.delta.pixel_delta(px(line));
        let (dx, mut dy) = (f32::from(delta.x), f32::from(delta.y));
        // With Shift held GPUI's Windows and X11 backends turn a turn of the wheel into a horizontal one
        // (the editor undoes it the same way, `view/paging.rs`); here Shift is the user's wheel.
        if !cfg!(target_os = "macos")
            && event.modifiers.shift
            && !event.delta.precise()
            && dy == 0.0
            && dx != 0.0
        {
            dy = dx;
        }
        if dy == 0.0 {
            return;
        }
        cx.stop_propagation();
        // Positive is the wheel turned away from the user: earlier lines. Windows' "one screen at a time"
        // setting arrives as a huge number of lines: at most a screen per event. Pixels (a trackpad) are
        // gathered until they make a whole line.
        let modes = self.terminal.modes();
        let shift = event.modifiers.shift;
        let dest = if !shift && modes.mouse_reporting() {
            WheelDest::Report
        } else if !shift && modes.alt_screen && modes.alternate_scroll {
            WheelDest::Arrows
        } else {
            WheelDest::Scroll
        };
        if dest != std::mem::replace(&mut self.wheel_dest, dest) {
            // Part of a line that was meant for another place (Shift was let go, a program turned the mouse on)
            // is not carried over to this one.
            self.wheel_rest = 0.0;
        }
        let screen = f32::from(self.size.rows);
        self.wheel_rest += (dy / line).clamp(-screen, screen);
        let lines = self.wheel_rest.trunc();
        self.wheel_rest -= lines;
        let lines = lines as i32;
        if lines == 0 {
            return;
        }
        if dest == WheelDest::Report {
            let direction = if lines > 0 {
                WheelDirection::Up
            } else {
                WheelDirection::Down
            };
            if let Some((row, column)) = self.cell(event.position) {
                let mods = terminal_modifiers(event.modifiers);
                for _ in 0..lines.unsigned_abs() {
                    if let Some(bytes) =
                        encode_mouse(MouseAction::Wheel(direction), mods, row, column, &modes)
                    {
                        self.send(&bytes);
                    }
                }
            }
        } else if dest == WheelDest::Arrows {
            self.send(&encode_wheel_as_arrows(lines, &modes));
        } else {
            self.terminal.scroll_display(lines);
            self.refresh_link(cx);
            cx.notify();
        }
    }

    /// A modifier key went down or up: the link under a still pointer is underlined or not.
    pub(super) fn on_modifiers_changed(
        &mut self,
        event: &ModifiersChangedEvent,
        cx: &mut Context<Self>,
    ) {
        self.last_modifiers = event.modifiers;
        self.refresh_link(cx);
    }

    /// The link under a point of the window, if the point is on the screen and in a link.
    fn link_under(&self, position: Point<Pixels>) -> Option<Link> {
        if !self.on_screen(position) {
            return None;
        }
        let (row, column) = self.cell(position)?;
        self.terminal.link_at(row, column)
    }

    /// Works out the link to underline: the one under the pointer while the link key is held and the mouse is
    /// the user's (the pointer is not dragging a selection).
    pub(super) fn refresh_link(&mut self, cx: &mut Context<Self>) {
        let modifiers = self.last_modifiers;
        let wanted = link_modifier(modifiers)
            && !self.dragging
            && (modifiers.shift || !self.terminal.modes().mouse_reporting());
        let link = if wanted {
            self.pointer.and_then(|at| self.link_under(at))
        } else {
            None
        };
        if link != self.link {
            self.link = link;
            cx.notify();
        }
    }

    /// The link that is underlined (for the drawing).
    pub(in crate::terminal) fn hover_link(&self) -> Option<&Link> {
        self.link.as_ref()
    }

    /// Asks before a link is opened with the system's browser; only `http` and `https` are opened, the others
    /// are refused with a message. One question at a time.
    fn open_link(&mut self, link: Link, window: &mut Window, cx: &mut Context<Self>) {
        if std::mem::replace(&mut self.link_asking, true) {
            return;
        }
        let uri = link.uri;
        cx.spawn_in(window, async move |this, cx| {
            if is_openable(&uri) {
                let yes = confirm(cx, &open_message(&uri), None, ["はい(Y)", "いいえ(N)"]).await;
                // The tab may have been closed while the question was open: then nothing is opened. The
                // address is checked again for what it is about to open.
                this.update(cx, |this, cx| {
                    this.link_asking = false;
                    if yes && is_openable(&uri) && this.in_a_tab(cx) {
                        cx.open_url(&uri);
                    }
                })
                .ok();
            } else {
                tell(cx, PromptLevel::Warning, &refuse_message(&uri), None).await;
                this.update(cx, |this, _| this.link_asking = false).ok();
            }
        })
        .detach();
    }

    /// Whether this view is still the content of a tab of its window (the tab may be closed while a question
    /// is open).
    fn in_a_tab(&self, cx: &Context<Self>) -> bool {
        let me = cx.entity();
        self.workspace
            .upgrade()
            .is_some_and(|workspace| workspace.read(cx).tab_of_shell(&me).is_some())
    }

    /// A question about a link is open (for the tests).
    #[cfg(test)]
    pub(crate) fn asking_about_a_link(&self) -> bool {
        self.link_asking
    }

    /// The link that is underlined, as its address (for the tests).
    #[cfg(test)]
    pub(crate) fn hovered_uri(&self) -> Option<String> {
        self.link.as_ref().map(|link| link.uri.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_questions_show_the_address_without_controls_and_not_too_long() {
        assert!(open_message("https://a.example/x").contains("https://a.example/x"));
        assert!(!open_message("https://a.example/\u{1b}[2J").contains('\u{1b}'));
        let long = format!("https://a.example/{}", "x".repeat(1000));
        assert!(open_message(&long).chars().count() < 400);
        assert!(refuse_message("ftp://a.example/").contains("ftp://a.example/"));
    }

    #[test]
    fn the_link_key_is_ctrl_and_on_macos_cmd() {
        let ctrl = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        let cmd = Modifiers {
            platform: true,
            ..Modifiers::default()
        };
        assert_eq!(link_modifier(ctrl), !cfg!(target_os = "macos"));
        assert_eq!(link_modifier(cmd), cfg!(target_os = "macos"));
        assert!(!link_modifier(Modifiers::default()));
    }
}
