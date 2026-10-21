//! Auto scroll (オートスクロール, `F_AUTOSCROLL`): press the middle button, and the text scrolls by
//! itself, faster the farther the pointer is from where it started.
//!
//! Sakura: `CEditView::OnMBUTTONDOWN`, `OnMBUTTONUP`, `AutoScrollEnter`, `AutoScrollExit` and
//! `AutoScrollOnTimer` (`sakura_core/view/CEditView_Mouse.cpp`) and
//! `CViewCommander::Command_AUTOSCROLL` (`sakura_core/cmd/CViewCommander_Cursor.cpp`).
//!
//! * The middle button runs the function it is bound to (`MOUSEFUNCTION_CENTER`, `F_AUTOSCROLL` by
//!   default). Pressing waits (mode 1); releasing starts the scroll (mode 2) at the pointer.
//!   Pressing it again while it scrolls ends it.
//! * `F_AUTOSCROLL` from a key or a menu starts it at the pointer, or ends it when it runs.
//! * It scrolls only along the axes that have something to scroll: down when the text is longer
//!   than the view (`m_nViewRowNum < line count + 2`), sideways when it is wider. If neither, it
//!   does not start.
//! * Every 200 ms the pointer's distance from the start, divided by 8, less one toward zero, is
//!   how many rows and columns the view moves.
//! * Any other function (`HandleCommand`), a click of the left or right button, a key and the loss
//!   of the focus end it.
//!
//! Not done: the small window with the arrows that marks the start (`CAutoScrollWnd`) and the
//! pointer shapes that show the direction (`IDC_CURSOR_AUTOSCROLL_*`).

use std::time::Duration;

use gpui::{Context, MouseButton, MouseDownEvent, MouseUpEvent, Pixels, Point, Task, Window};
use ume_commands::keymap::{Chord, Key, Modifiers, MouseAction, Resolved};

use super::EditorView;

/// `F_AUTOSCROLL` (`Funccode_enum.h`).
pub(crate) const F_AUTOSCROLL: u16 = 30384;

/// `SetTimer(GetHwnd(), 2, 200, ...)`.
const INTERVAL: Duration = Duration::from_millis(200);

/// `relPos / 8` of `AutoScrollOnTimer`.
const SPEED_DIVISOR: i32 = 8;

/// `m_nAutoScrollMode`.
#[derive(Default)]
pub(crate) enum AutoScroll {
    /// 0.
    #[default]
    Off,
    /// 1: the middle button is down.
    Pending { origin: Point<Pixels> },
    /// 2: scrolling.
    Active {
        origin: Point<Pixels>,
        vertical: bool,
        horizontal: bool,
        /// The timer; dropping it stops it.
        _timer: Task<()>,
    },
}

impl AutoScroll {
    pub(crate) fn is_on(&self) -> bool {
        !matches!(self, Self::Off)
    }

    pub(crate) fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }
}

/// How far the view moves in one tick for a pointer `distance` pixels from the start: the
/// distance divided by 8 (toward zero), less one toward zero (`AutoScrollOnTimer`).
pub(crate) fn step(distance: i32) -> i32 {
    let n = distance / SPEED_DIVISOR;
    n - n.signum()
}

impl EditorView {
    /// Whether the text scrolls by itself now.
    pub fn is_autoscrolling(&self) -> bool {
        self.autoscroll.is_active()
    }

    /// Ends the auto scroll if it is on (`AutoScrollExit`).
    pub(crate) fn autoscroll_exit(&mut self) {
        self.autoscroll = AutoScroll::Off;
    }

    /// `F_AUTOSCROLL` (`Command_AUTOSCROLL`): starts at the pointer, or ends.
    pub(crate) fn toggle_autoscroll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.autoscroll.is_on() {
            self.autoscroll_exit();
        } else {
            self.autoscroll_enter(window.mouse_position(), window, cx);
        }
        cx.notify();
    }

    /// `AutoScrollEnter`: scrolls from `origin`, along the axes that have room.
    fn autoscroll_enter(
        &mut self,
        origin: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let vertical = self.metrics.visible_rows() < self.layout.row_count() + 2;
        let columns = (self.metrics.view_width / self.metrics.cell_width).floor();
        let horizontal = columns < self.widest_cols as f32;
        if !vertical && !horizontal {
            self.autoscroll = AutoScroll::Off;
            return;
        }
        let timer = cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(INTERVAL).await;
                let alive = this
                    .update_in(cx, |this, window, cx| this.autoscroll_tick(window, cx))
                    .unwrap_or(false);
                if !alive {
                    break;
                }
            }
        });
        self.autoscroll = AutoScroll::Active {
            origin,
            vertical,
            horizontal,
            _timer: timer,
        };
    }

    /// `AutoScrollOnTimer`. False when the auto scroll is over.
    pub(crate) fn autoscroll_tick(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let AutoScroll::Active {
            origin,
            vertical,
            horizontal,
            ..
        } = &self.autoscroll
        else {
            return false;
        };
        let (origin, vertical, horizontal) = (*origin, *vertical, *horizontal);
        // Another tab or pane took the focus: the text must not go on scrolling behind it.
        if !self.focus_handle.is_focused(window) {
            self.autoscroll_exit();
            return false;
        }
        let pointer = window.mouse_position();
        let dx = f32::from(pointer.x - origin.x) as i32;
        let dy = f32::from(pointer.y - origin.y) as i32;
        if horizontal {
            let columns = step(dx);
            if columns != 0 {
                self.scroll_by(-(columns as f32) * self.metrics.cell_width, 0.0);
            }
        }
        if vertical {
            let rows = step(dy);
            if rows != 0 {
                let top = self.top_row() as isize + rows as isize;
                self.scroll_top_to(top, cx);
            }
        }
        cx.notify();
        true
    }

    /// The function the middle button is bound to with the modifiers of `event`.
    fn middle_button_function(&self, m: gpui::Modifiers) -> Option<u16> {
        let mods = Modifiers {
            shift: m.shift,
            ctrl: m.control,
            alt: m.alt,
            meta: m.platform,
        };
        match self
            .keymap
            .resolve(Chord::new(Key::Mouse(MouseAction::MiddleClick), mods))
        {
            Resolved::Code(code) => Some(code),
            _ => None,
        }
    }

    /// `OnMBUTTONDOWN`.
    pub(crate) fn on_middle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.middle_button_function(event.modifiers) != Some(F_AUTOSCROLL) {
            return;
        }
        if self.autoscroll.is_on() {
            self.autoscroll_exit();
        } else {
            self.autoscroll = AutoScroll::Pending {
                origin: event.position,
            };
        }
        cx.notify();
    }

    /// `OnMBUTTONUP`: starts the scroll that the press waited for, or runs the function the
    /// button is bound to.
    pub(crate) fn on_middle_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.middle_button_function(event.modifiers) {
            Some(F_AUTOSCROLL) => {
                if let AutoScroll::Pending { origin } = self.autoscroll {
                    self.autoscroll_enter(origin, window, cx);
                    cx.notify();
                }
            }
            Some(code) if code != 0 => {
                self.autoscroll_exit();
                self.run_function(code, &[], window, cx);
            }
            _ => self.autoscroll_exit(),
        }
    }

    /// The buttons other than the middle one end the auto scroll (`OnLBUTTONDOWN`,
    /// `OnRBUTTONDOWN`).
    pub(crate) fn autoscroll_button_pressed(&mut self, button: MouseButton) {
        if button != MouseButton::Middle {
            self.autoscroll_exit();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::step;

    #[test]
    fn the_speed_is_the_distance_over_eight_less_one() {
        // `scrollPos = relPos / 8`, then one toward zero.
        assert_eq!(step(0), 0);
        assert_eq!(step(15), 0);
        assert_eq!(step(16), 1);
        assert_eq!(step(23), 1);
        assert_eq!(step(24), 2);
        assert_eq!(step(80), 9);
        assert_eq!(step(-15), 0);
        assert_eq!(step(-16), -1);
        assert_eq!(step(-80), -9);
    }
}
