//! Paging and scrolling the way Sakura does it: the view's top row, the page commands and the
//! wheel. The UI commands under `ui_commands/cursor/` are thin; the rules are here so that they
//! are in one place.
//!
//! Sakura (`sakura_core/`):
//! * `CEditView::ScrollAtV` (`view/CEditView_Scroll.cpp`) sets the top row and keeps it inside the
//!   scroll range ([`EditorView::scroll_top_to`]).
//! * `CViewCommander::MoveViewTopLine` (`cmd/CViewCommander_Cursor.cpp`) sets the top row with no
//!   upper limit, so the view can show the end of the document in its middle
//!   ([`EditorView::move_view_top_line`]).
//! * `Command_1PageUp/Down`, `Command_HalfPageUp/Down`, `Command_WndScrollUp/Down` in the same
//!   file, and `CEditView::OnMOUSEWHEEL2` (`view/CEditView_Mouse.cpp`) for the wheel.

use gpui::{Context, ScrollWheelEvent, Window};
use matsu_commands::command::{Arg, Result};
use matsu_commands::keymap::{Chord, Key, Modifiers, MouseAction, Resolved};

use super::EditorView;
use crate::geometry::clamp_scroll_y;

/// Rows (or columns) that one notch of the wheel scrolls: Windows' `WheelScrollLines`, which
/// Sakura reads from the registry (`OnMOUSEWHEEL2`: `nRollLineNum = 3` when there is none).
pub(crate) const WHEEL_ROWS_PER_NOTCH: f32 = 3.0;

/// More notches than this in one event cannot be a real wheel; see `handle_wheel_axis`.
const MAX_NOTCHES_PER_EVENT: f32 = 1000.0;

/// `F_WHEELUP` ..`F_WHEELPAGERIGHT` (`Funccode_x.hsrc`): the functions of the wheel that scroll.
const F_WHEEL_FIRST: u16 = 30385;
const F_WHEEL_LAST: u16 = 30392;
pub(crate) const F_WHEELUP: u16 = 30385;
pub(crate) const F_WHEELDOWN: u16 = 30386;
pub(crate) const F_WHEELLEFT: u16 = 30387;
pub(crate) const F_WHEELRIGHT: u16 = 30388;
pub(crate) const F_WHEELPAGEUP: u16 = 30389;
pub(crate) const F_WHEELPAGEDOWN: u16 = 30390;
pub(crate) const F_WHEELPAGELEFT: u16 = 30391;
pub(crate) const F_WHEELPAGERIGHT: u16 = 30392;

/// `_CARETMARGINRATE` (`view/CCaret.h`): the caret margin is 1/20 of the rows of the view.
const CARET_MARGIN_RATE: usize = 20;

impl EditorView {
    /// The row at the top of the view (Sakura's `GetViewTopLine`).
    pub fn top_row(&self) -> usize {
        (self.scroll_y / self.metrics.line_height).round().max(0.0) as usize
    }

    /// Sakura's `ScrollAtV`: the top row becomes `row`, kept between the first row and the end of
    /// the scroll range. Returns how many rows the view scrolled (positive: down).
    pub fn scroll_top_to(&mut self, row: isize, cx: &mut Context<Self>) -> isize {
        let before = self.top_row() as isize;
        let wanted = row.max(0) as f32 * self.metrics.line_height;
        self.scroll_past_end = false;
        self.scroll_y = clamp_scroll_y(&self.metrics, self.layout.row_count(), wanted);
        cx.notify();
        self.top_row() as isize - before
    }

    /// Sakura's `MoveViewTopLine`: the top row becomes `row`, not above the first row and with
    /// no limit below, so the scroll range does not stop it. Any other scrolling brings the top
    /// back into the range.
    pub fn move_view_top_line(&mut self, row: isize, cx: &mut Context<Self>) {
        self.scroll_y = row.max(0) as f32 * self.metrics.line_height;
        self.scroll_past_end = true;
        cx.notify();
    }

    /// `rows` rows down (negative: up) with the caret, to the goal column.
    pub(crate) fn move_caret_rows(
        &mut self,
        rows: isize,
        extend: bool,
        cx: &mut Context<Self>,
    ) -> Result {
        self.run_with(cx, |command_cx| command_cx.move_vertical(rows, extend))
    }

    /// As [`move_caret_rows`](Self::move_caret_rows) for the box moves: a box selection is begun
    /// if there is none (`Sub_BoxSelectLock`) and the box grows.
    fn move_caret_rows_box(&mut self, rows: isize, cx: &mut Context<Self>) -> Result {
        self.run_with(cx, |command_cx| {
            command_cx.ensure_box()?;
            command_cx.move_vertical(rows, true)
        })
    }

    /// `Command_1PageUp/Down`: the caret moves a page less one row and the view scrolls by as many
    /// rows, so the caret keeps its place on the screen. The view scrolls even when the caret
    /// cannot move (2014.01.10 Moca in Sakura's comment).
    pub(crate) fn page_by(
        &mut self,
        direction: isize,
        extend: bool,
        cx: &mut Context<Self>,
    ) -> Result {
        self.page_with(direction, cx, |view, rows, cx| {
            view.move_caret_rows(rows, extend, cx)
        })
    }

    /// `F_1PageUp_BOX` and `F_1PageDown_BOX`: [`page_by`](Self::page_by) with the box selection.
    pub(crate) fn page_by_box(&mut self, direction: isize, cx: &mut Context<Self>) -> Result {
        self.page_with(direction, cx, |view, rows, cx| {
            view.move_caret_rows_box(rows, cx)
        })
    }

    fn page_with(
        &mut self,
        direction: isize,
        cx: &mut Context<Self>,
        move_caret: impl FnOnce(&mut Self, isize, &mut Context<Self>) -> Result,
    ) -> Result {
        let rows = (self.metrics.visible_rows() as isize - 1).max(1);
        let top = self.top_row() as isize;
        let moved = move_caret(self, direction * rows, cx);
        self.scroll_top_to(top + direction * rows, cx);
        moved
    }

    /// `Command_HalfPageUp/Down`: the caret moves half the rows of the view; the view follows it
    /// only as far as needed.
    pub(crate) fn half_page_by(
        &mut self,
        direction: isize,
        extend: bool,
        cx: &mut Context<Self>,
    ) -> Result {
        let rows = (self.metrics.visible_rows() / 2) as isize;
        self.move_caret_rows(direction * rows, extend, cx)
    }

    /// `F_HalfPageUp_BOX` and `F_HalfPageDown_BOX`: [`half_page_by`](Self::half_page_by) with the
    /// box selection.
    pub(crate) fn half_page_by_box(&mut self, direction: isize, cx: &mut Context<Self>) -> Result {
        let rows = (self.metrics.visible_rows() / 2) as isize;
        self.move_caret_rows_box(direction * rows, cx)
    }

    /// `Command_WndScrollUp` (`direction` 1: the text moves up, the top row grows) and
    /// `Command_WndScrollDown` (`direction` -1): one row, and the caret is pushed along when it
    /// would leave the margin at the edge.
    pub(crate) fn scroll_window_by(&mut self, direction: isize, cx: &mut Context<Self>) -> Result {
        let visible = self.metrics.visible_rows();
        let total = self.layout.row_count() as isize;
        let mut margin = (visible / CARET_MARGIN_RATE).max(1) as isize;
        if direction < 0 {
            margin += 2;
        }
        let top = self.top_row() as isize;
        self.scroll_top_to(top + direction, cx);
        let new_top = self.top_row() as isize;
        if self.cursor.has_selection() {
            return Ok(());
        }
        let caret = self.caret_row(cx).row as isize;
        let visible = visible as isize;
        if direction > 0 {
            if caret < new_top + margin {
                // Sakura compares with `GetViewTopLine() == 1`, the row after the first.
                let rows = if new_top == 1 { margin + 1 } else { 1 };
                return self.move_caret_rows(rows, false, cx);
            }
        } else if caret > visible + new_top - margin {
            let rows = if caret > total - margin {
                (total - margin) - caret
            } else {
                -1
            };
            return self.move_caret_rows(rows, false, cx);
        }
        Ok(())
    }

    // ---- the keys ----

    /// How many times a key that is held runs its function. A held up or down key moves
    /// `nRepeatedScrollLineNum` rows (3 by default) a time, not one: `Command_UP` and
    /// `Command_DOWN` (`sakura_core/cmd/CViewCommander_Cursor.cpp`) take `bRepeat`, which
    /// `HandleCommand` sets when the same function runs again without a key up in between
    /// (`m_bPrevCommand`, reset by `WM_KEYUP`); a held key is the auto repeat of the OS. While a
    /// key macro is recorded `bRepeat` is false, so that the macro has the function once.
    pub(crate) fn key_repeat_count(&self, function: &str, held: bool, cx: &Context<Self>) -> usize {
        if !held
            || !matches!(
                function,
                "F_UP" | "F_DOWN" | "F_UP_SEL" | "F_DOWN_SEL" | "F_UP_BOX" | "F_DOWN_BOX"
            )
            || self.macros.is_recording()
        {
            return 1;
        }
        crate::common_settings::scroll_options(cx)
            .repeated_lines
            .max(1)
    }

    // ---- the wheel ----

    /// The wheel, through Sakura's key table (`OnMOUSEWHEEL2`): the function of the wheel
    /// direction and the modifiers is run. The wheel functions scroll, by `WHEEL_ROWS_PER_NOTCH`
    /// rows per notch or by a page; any other function (Ctrl+wheel is the font size) runs once per
    /// notch. A trackpad, which reports pixels, still scrolls smoothly when the function is the
    /// plain wheel.
    pub(crate) fn handle_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let delta = event.delta.pixel_delta(gpui::px(self.metrics.line_height));
        // Sakura gets the vertical and the horizontal wheel as separate messages; a trackpad
        // reports both at once, and each axis is handled on its own.
        let (mut dx, mut dy) = (f32::from(delta.x), f32::from(delta.y));
        // With Shift held GPUI's Windows and X11 backends turn a turn of the wheel into a
        // horizontal one (`handle_mouse_wheel_msg`); Sakura gets it as the vertical wheel, and
        // the key table and the combinations of the common settings decide what Shift does.
        if !cfg!(target_os = "macos")
            && event.modifiers.shift
            && !event.delta.precise()
            && dy == 0.0
            && dx != 0.0
        {
            (dx, dy) = (0.0, dx);
        }
        for (vertical, amount) in [(true, dy), (false, dx)] {
            if amount != 0.0 {
                self.handle_wheel_axis(event, vertical, amount, window, cx);
            }
        }
    }

    /// One axis of the wheel: `amount` pixels, positive up (or left).
    fn handle_wheel_axis(
        &mut self,
        event: &ScrollWheelEvent,
        vertical: bool,
        amount: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let line_height = self.metrics.line_height;
        // The text moves down (or right) when the wheel goes up (or left).
        let action = match (vertical, amount > 0.0) {
            (true, true) => MouseAction::WheelUp,
            (true, false) => MouseAction::WheelDown,
            (false, true) => MouseAction::WheelLeft,
            (false, false) => MouseAction::WheelRight,
        };
        let m = event.modifiers;
        let mods = Modifiers {
            shift: m.shift,
            ctrl: m.control,
            alt: m.alt,
            meta: m.platform,
        };
        let code = match self.keymap.resolve(Chord::new(Key::Mouse(action), mods)) {
            Resolved::Code(code) => code,
            _ => return,
        };
        // GPUI's Windows backend multiplies a notch by the system's lines per notch (3 by
        // default) and its Linux backends by 3 before they report lines; macOS reports lines as
        // they are.
        let lines_per_notch = if event.delta.precise() || cfg!(target_os = "macos") {
            1.0
        } else {
            WHEEL_ROWS_PER_NOTCH
        };
        let mut notches = amount.abs() / (line_height * lines_per_notch);
        // Windows' "one screen at a time" setting arrives as a huge number of lines (`u32::MAX`
        // per notch). Sakura reads it as -1: a page per notch (`OnMOUSEWHEEL2`).
        let page_setting = notches > MAX_NOTCHES_PER_EVENT;
        if page_setting {
            notches = 1.0;
        }
        if !(F_WHEEL_FIRST..=F_WHEEL_LAST).contains(&code) {
            // `bExecCmd`: one run of the function per notch.
            let count = self.take_whole(vertical, notches, 1.0, 1.0);
            for _ in 0..count {
                self.run_function(code, &[], window, cx);
            }
            return;
        }
        // 通常スクロールの時だけ: the keys or buttons that turn the wheel into a page or a
        // sideways scroll (`CEditView::OnMOUSEWHEEL2`, `IsSpecialScrollMode`).
        let options = crate::common_settings::scroll_options(cx);
        let sideways = self
            .held_buttons
            .special_mode(options.hscroll_by_wheel, event.modifiers);
        let by_page = self
            .held_buttons
            .special_mode(options.page_by_wheel, event.modifiers);
        // The button that turned the wheel must not also run its own function when it is let
        // go (`OnMBUTTONUP`, `OnXLBUTTONUP`, `OnXRBUTTONUP`: `IsPageScrollByWheel`,
        // `IsHScrollByWheel`).
        if sideways {
            self.wheel_used_buttons
                .mark_option(options.hscroll_by_wheel);
        }
        if by_page {
            self.wheel_used_buttons.mark_option(options.page_by_wheel);
        }
        let pixels = event.delta.precise().then_some(amount.abs());
        self.scroll_by_wheel_function(
            code,
            vertical,
            notches,
            page_setting || by_page,
            sideways,
            pixels,
            cx,
        );
    }

    /// A wheel function run as a command (a key, a menu or a macro): `args` is `zDelta`, the
    /// wheel rotation, 120 per notch, and the default 120 when it is missing or 0. Only its size
    /// counts, because the function decides the direction (`Command_WHEELUP` .. in
    /// `sakura_core/cmd/CViewCommander_Cursor.cpp`, which call `OnMOUSEWHEEL2` with the function).
    pub(crate) fn run_wheel_function(&mut self, code: u16, args: &[Arg], cx: &mut Context<Self>) {
        let z_delta = match args.first() {
            Some(Arg::Int(z)) if *z != 0 => z.unsigned_abs() as f32,
            _ => 120.0,
        };
        let horizontal = matches!(
            code,
            F_WHEELLEFT | F_WHEELRIGHT | F_WHEELPAGELEFT | F_WHEELPAGERIGHT
        );
        self.scroll_by_wheel_function(code, !horizontal, z_delta / 120.0, false, false, None, cx);
    }

    /// The scrolling functions of the wheel (`F_WHEELUP` .. `F_WHEELPAGERIGHT`): `notches` of the
    /// wheel scroll in the direction of the function, whichever way the wheel went. `vertical`
    /// is the axis whose leftover `notches` is added to. `pixels` is the distance of a trackpad,
    /// which scrolls smoothly unless the function is a page. `page_setting` is Windows' "one
    /// screen at a time", which makes any of them a page (so does the key of 組み合わせてホイール操作
    /// した時ページスクロールする). `sideways` turns the scroll sideways (the key of 横スクロールする);
    /// the direction stays the one of the function.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn scroll_by_wheel_function(
        &mut self,
        code: u16,
        vertical: bool,
        notches: f32,
        page_setting: bool,
        sideways: bool,
        pixels: Option<f32>,
        cx: &mut Context<Self>,
    ) {
        let (horizontal, page, forward) = match code {
            F_WHEELUP => (false, false, false),
            F_WHEELDOWN => (false, false, true),
            F_WHEELLEFT => (true, false, false),
            F_WHEELRIGHT => (true, false, true),
            F_WHEELPAGEUP => (false, true, false),
            F_WHEELPAGEDOWN => (false, true, true),
            F_WHEELPAGELEFT => (true, true, false),
            F_WHEELPAGERIGHT => (true, true, true),
            _ => return,
        };
        let page = page || page_setting;
        let horizontal = horizontal || sideways;
        let sign: isize = if forward { 1 } else { -1 };
        if let Some(pixels) = pixels
            && !page
        {
            // Smooth scrolling for a trackpad: the pixels as they come, in the function's direction.
            let amount = pixels * sign as f32;
            if horizontal {
                self.scroll_by(-amount, 0.0);
            } else {
                self.scroll_by(0.0, -amount);
            }
            cx.notify();
            return;
        }
        let per_notch = match (horizontal, page) {
            (false, false) | (true, false) => WHEEL_ROWS_PER_NOTCH,
            (false, true) => (self.metrics.visible_rows() as f32 - 1.0).max(1.0),
            (true, true) => {
                ((self.metrics.view_width / self.metrics.cell_width).floor() - 1.0).max(1.0)
            }
        };
        let count = self.take_whole(vertical, notches, per_notch, sign as f32) as isize;
        if count == 0 {
            return;
        }
        if horizontal {
            self.scroll_by(-(sign * count) as f32 * self.metrics.cell_width, 0.0);
        } else {
            let top = self.top_row() as isize;
            self.scroll_top_to(top + sign * count, cx);
        }
        cx.notify();
    }

    /// Adds `notches` of the wheel to what is left over from before and takes the whole rows
    /// (`per_notch` of them in a notch): Sakura's `m_nWheelDelta`, which keeps what does not make
    /// a row. A change of direction (`sign`) drops what was left.
    fn take_whole(&mut self, vertical: bool, notches: f32, per_notch: f32, sign: f32) -> usize {
        // Each axis keeps its own leftover: a diagonal move must not feed one into the other.
        let left = &mut self.wheel_remainder[usize::from(!vertical)];
        if *left * sign < 0.0 {
            *left = 0.0;
        }
        let total = left.abs() + notches;
        let whole = (total * per_notch).floor();
        *left = (total - whole / per_notch) * sign;
        whole as usize
    }
}
