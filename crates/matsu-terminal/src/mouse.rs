//! Mouse encoding: the bytes a terminal sends to the program for a mouse event, when the program has turned
//! mouse reporting on (ADR-0028; xterm's `ctlseqs`, "Mouse Tracking").
//!
//! The types are UI independent; the UI maps its own events to [`MouseAction`] and a cell. Positions are
//! cells of the view, counted from 0. Which events are reported follows the modes the program set
//! ([`Modes`]): a click mode (DECSET 1000) reports presses, releases and the wheel, the drag mode (1002) adds
//! motion while a button is down, and the motion mode (1003) adds all motion. The encoding is SGR (1006) when
//! the program asked for it, else the old one, in UTF-8 form (1005) when asked. X10 mode (DECSET 9) is not
//! reported: `alacritty_terminal` 0.26.0 does not know it.
//!
//! The convention of the UI is xterm's and Windows Terminal's: Shift held with the mouse is not reported and
//! selects as usual; this module still encodes a Shift that it is given (bit 4 of the button), as xterm does.

use crate::keys::Modifiers;
use crate::screen::Modes;

/// A mouse button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

impl MouseButton {
    fn code(self) -> u32 {
        match self {
            Self::Left => 0,
            Self::Middle => 1,
            Self::Right => 2,
        }
    }
}

/// The direction the wheel turned in. `Up` is the wheel turned away from the user, which shows earlier lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WheelDirection {
    Up,
    Down,
    Left,
    Right,
}

impl WheelDirection {
    fn code(self) -> u32 {
        match self {
            Self::Up => 64,
            Self::Down => 65,
            Self::Left => 66,
            Self::Right => 67,
        }
    }
}

/// What the mouse did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseAction {
    Press(MouseButton),
    Release(MouseButton),
    /// The pointer moved to another cell, with the button that is down, if one is.
    Move(Option<MouseButton>),
    /// One step of the wheel.
    Wheel(WheelDirection),
}

/// The bytes to write to the program for a mouse event at the cell (`row`, `column`) of the view, or `None`
/// when the program does not want it: mouse reporting is off, the mode does not report this kind of event
/// (motion in the click mode, motion with no button in the drag mode), or the cell is out of what the old
/// encoding can say (column or row past 222 without the UTF-8 form, 2014 with it).
pub fn encode_mouse(
    action: MouseAction,
    mods: Modifiers,
    row: usize,
    column: usize,
    modes: &Modes,
) -> Option<Vec<u8>> {
    if !modes.mouse_reporting() {
        return None;
    }
    let mut release = false;
    let mut code = match action {
        MouseAction::Press(button) => button.code(),
        MouseAction::Release(button) => {
            release = true;
            // The old encoding does not say which button went up.
            if modes.sgr_mouse { button.code() } else { 3 }
        }
        MouseAction::Move(button) => {
            let reported = modes.mouse_motion || (modes.mouse_drag && button.is_some());
            if !reported {
                return None;
            }
            32 + button.map_or(3, MouseButton::code)
        }
        MouseAction::Wheel(direction) => direction.code(),
    };
    code += 4 * u32::from(mods.shift) + 8 * u32::from(mods.alt) + 16 * u32::from(mods.ctrl);

    if modes.sgr_mouse {
        let last = if release { 'm' } else { 'M' };
        return Some(format!("\x1b[<{code};{};{}{last}", column + 1, row + 1).into_bytes());
    }
    // Each value is 32 and its number, from 1 for the first cell. One byte holds up to 255; the UTF-8 form
    // (1005) writes the values from 128 on as two bytes, up to 2047.
    let push = |out: &mut Vec<u8>, value: usize| -> Option<()> {
        match value {
            0..=0x7f => out.push(value as u8),
            0x80..=0xff if !modes.utf8_mouse => out.push(value as u8),
            0x80..=0x7ff if modes.utf8_mouse => {
                out.push(0xc0 | (value >> 6) as u8);
                out.push(0x80 | (value & 0x3f) as u8);
            }
            _ => return None,
        }
        Some(())
    };
    let mut out = vec![0x1b, b'[', b'M'];
    push(&mut out, 32 + code as usize)?;
    push(&mut out, 32 + column + 1)?;
    push(&mut out, 32 + row + 1)?;
    Some(out)
}

/// The arrow keys a wheel turn is sent as on the alternate screen, when the program asked for that (alternate
/// scroll, DECSET 1007) and not for mouse reports: `lines` presses, Up for a positive number (the wheel
/// turned away from the user), Down for a negative one. They are `SS3` sequences in application cursor mode.
/// (`xterm` and `alacritty` do the same.)
pub fn encode_wheel_as_arrows(lines: i32, modes: &Modes) -> Vec<u8> {
    let final_byte = if lines > 0 { b'A' } else { b'B' };
    let introducer: &[u8] = if modes.app_cursor { b"\x1bO" } else { b"\x1b[" };
    let mut out = Vec::new();
    for _ in 0..lines.unsigned_abs() {
        out.extend_from_slice(introducer);
        out.push(final_byte);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Modifiers = Modifiers::NONE;

    fn modes(click: bool, drag: bool, motion: bool, sgr: bool) -> Modes {
        Modes {
            mouse_click: click,
            mouse_drag: drag,
            mouse_motion: motion,
            sgr_mouse: sgr,
            ..Modes::DEFAULT
        }
    }

    fn click_mode() -> Modes {
        modes(true, false, false, false)
    }

    fn encode(action: MouseAction, row: usize, column: usize, modes: &Modes) -> Option<Vec<u8>> {
        encode_mouse(action, NONE, row, column, modes)
    }

    #[test]
    fn nothing_is_reported_while_reporting_is_off() {
        let off = Modes::DEFAULT;
        assert_eq!(
            encode(MouseAction::Press(MouseButton::Left), 0, 0, &off),
            None
        );
        assert_eq!(
            encode(MouseAction::Wheel(WheelDirection::Up), 0, 0, &off),
            None
        );
    }

    #[test]
    fn the_old_encoding_is_32_plus_the_numbers_from_1() {
        let m = click_mode();
        // button 0 (32 = ' '), column 1 (33 = '!'), row 1 (33 = '!')
        assert_eq!(
            encode(MouseAction::Press(MouseButton::Left), 0, 0, &m),
            Some(b"\x1b[M !!".to_vec())
        );
        // middle is 1, right is 2; column 10, row 5
        assert_eq!(
            encode(MouseAction::Press(MouseButton::Middle), 4, 9, &m),
            Some(vec![0x1b, b'[', b'M', 33, 32 + 10, 32 + 5])
        );
        assert_eq!(
            encode(MouseAction::Press(MouseButton::Right), 0, 0, &m),
            Some(vec![0x1b, b'[', b'M', 34, 33, 33])
        );
    }

    #[test]
    fn the_old_encoding_does_not_say_which_button_went_up() {
        let m = click_mode();
        for button in [MouseButton::Left, MouseButton::Middle, MouseButton::Right] {
            assert_eq!(
                encode(MouseAction::Release(button), 1, 2, &m),
                Some(vec![0x1b, b'[', b'M', 35, 35, 34])
            );
        }
    }

    #[test]
    fn the_sgr_encoding_has_the_numbers_from_1_and_ends_with_m_for_a_release() {
        let m = modes(true, false, false, true);
        assert_eq!(
            encode(MouseAction::Press(MouseButton::Left), 4, 9, &m),
            Some(b"\x1b[<0;10;5M".to_vec())
        );
        assert_eq!(
            encode(MouseAction::Release(MouseButton::Left), 4, 9, &m),
            Some(b"\x1b[<0;10;5m".to_vec())
        );
        // the release says which button
        assert_eq!(
            encode(MouseAction::Release(MouseButton::Right), 0, 0, &m),
            Some(b"\x1b[<2;1;1m".to_vec())
        );
    }

    #[test]
    fn the_wheel_is_button_64_to_67() {
        let sgr = modes(true, false, false, true);
        let old = click_mode();
        assert_eq!(
            encode(MouseAction::Wheel(WheelDirection::Up), 2, 3, &sgr),
            Some(b"\x1b[<64;4;3M".to_vec())
        );
        assert_eq!(
            encode(MouseAction::Wheel(WheelDirection::Down), 2, 3, &sgr),
            Some(b"\x1b[<65;4;3M".to_vec())
        );
        assert_eq!(
            encode(MouseAction::Wheel(WheelDirection::Left), 0, 0, &sgr),
            Some(b"\x1b[<66;1;1M".to_vec())
        );
        assert_eq!(
            encode(MouseAction::Wheel(WheelDirection::Right), 0, 0, &sgr),
            Some(b"\x1b[<67;1;1M".to_vec())
        );
        assert_eq!(
            encode(MouseAction::Wheel(WheelDirection::Up), 0, 0, &old),
            Some(vec![0x1b, b'[', b'M', 32 + 64, 33, 33])
        );
    }

    #[test]
    fn the_modes_choose_which_motion_is_reported() {
        let held = MouseAction::Move(Some(MouseButton::Left));
        let free = MouseAction::Move(None);
        let click = modes(true, false, false, true);
        let drag = modes(false, true, false, true);
        let motion = modes(false, false, true, true);
        // the click mode reports no motion
        assert_eq!(encode(held, 0, 0, &click), None);
        assert_eq!(encode(free, 0, 0, &click), None);
        // the drag mode reports motion with a button down (32 + the button)
        assert_eq!(encode(held, 0, 0, &drag), Some(b"\x1b[<32;1;1M".to_vec()));
        assert_eq!(encode(free, 0, 0, &drag), None);
        // the motion mode reports all of it; without a button the code is 32 + 3
        assert_eq!(encode(held, 0, 0, &motion), Some(b"\x1b[<32;1;1M".to_vec()));
        assert_eq!(encode(free, 0, 0, &motion), Some(b"\x1b[<35;1;1M".to_vec()));
        assert_eq!(
            encode(MouseAction::Move(Some(MouseButton::Right)), 0, 0, &motion),
            Some(b"\x1b[<34;1;1M".to_vec())
        );
        // presses are reported in every mode
        assert!(encode(MouseAction::Press(MouseButton::Left), 0, 0, &drag).is_some());
        assert!(encode(MouseAction::Press(MouseButton::Left), 0, 0, &motion).is_some());
    }

    #[test]
    fn the_modifiers_add_4_8_and_16_to_the_button() {
        let sgr = modes(true, false, false, true);
        let with = |shift, alt, ctrl| {
            encode_mouse(
                MouseAction::Press(MouseButton::Left),
                Modifiers { shift, ctrl, alt },
                0,
                0,
                &sgr,
            )
            .unwrap()
        };
        assert_eq!(with(true, false, false), b"\x1b[<4;1;1M");
        assert_eq!(with(false, true, false), b"\x1b[<8;1;1M");
        assert_eq!(with(false, false, true), b"\x1b[<16;1;1M");
        assert_eq!(with(true, true, true), b"\x1b[<28;1;1M");
        // in the old encoding the release is 3 and the modifiers still add
        let old = click_mode();
        assert_eq!(
            encode_mouse(
                MouseAction::Release(MouseButton::Left),
                Modifiers { ctrl: true, ..NONE },
                0,
                0,
                &old
            ),
            Some(vec![0x1b, b'[', b'M', 32 + 3 + 16, 33, 33])
        );
    }

    #[test]
    fn the_old_encoding_stops_at_column_223_and_the_sgr_one_does_not() {
        let old = click_mode();
        let press = MouseAction::Press(MouseButton::Left);
        // the last cell one byte can say: 32 + 223 = 255
        assert_eq!(
            encode(press, 0, 222, &old),
            Some(vec![0x1b, b'[', b'M', 32, 255, 33])
        );
        assert_eq!(encode(press, 0, 223, &old), None);
        assert_eq!(
            encode(press, 222, 0, &old),
            Some(vec![0x1b, b'[', b'M', 32, 33, 255])
        );
        assert_eq!(encode(press, 223, 0, &old), None);
        let sgr = modes(true, false, false, true);
        assert_eq!(
            encode(press, 300, 400, &sgr),
            Some(b"\x1b[<0;401;301M".to_vec())
        );
    }

    #[test]
    fn the_utf8_form_goes_up_to_2014_in_two_bytes() {
        let utf8 = Modes {
            utf8_mouse: true,
            ..click_mode()
        };
        let press = MouseAction::Press(MouseButton::Left);
        // column 223: 32 + 224 = 256 = U+0100, bytes C4 80
        assert_eq!(
            encode(press, 0, 223, &utf8),
            Some(vec![0x1b, b'[', b'M', 32, 0xc4, 0x80, 33])
        );
        // the small values are the single bytes of the old encoding
        assert_eq!(
            encode(press, 0, 0, &utf8),
            Some(vec![0x1b, b'[', b'M', 32, 33, 33])
        );
        // the last cell: 32 + 2015 = 2047 = U+07FF
        assert_eq!(
            encode(press, 0, 2014, &utf8),
            Some(vec![0x1b, b'[', b'M', 32, 0xdf, 0xbf, 33])
        );
        assert_eq!(encode(press, 0, 2015, &utf8), None);
    }

    #[test]
    fn the_wheel_as_arrows_is_one_key_per_line() {
        let plain = Modes::DEFAULT;
        assert_eq!(encode_wheel_as_arrows(1, &plain), b"\x1b[A");
        assert_eq!(encode_wheel_as_arrows(-2, &plain), b"\x1b[B\x1b[B");
        assert_eq!(encode_wheel_as_arrows(0, &plain), b"");
        let app = Modes {
            app_cursor: true,
            ..plain
        };
        assert_eq!(encode_wheel_as_arrows(3, &app), b"\x1bOA\x1bOA\x1bOA");
        assert_eq!(encode_wheel_as_arrows(-1, &app), b"\x1bOB");
    }
}
