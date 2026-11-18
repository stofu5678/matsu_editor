//! The spin button (Win32's up-down control, `msctls_updown32`) beside the edit box of a number.
//!
//! Sakura gives a number box an up-down control with `UDS_ALIGNRIGHT | UDS_ARROWKEYS |
//! UDS_AUTOBUDDY | UDS_SETBUDDYINT` (`sakura_core/sakura_rc.rc`) and sets its range with
//! `apiwrap::SetUpDownRange` (`sakura_core/util/window.h`: `UDM_SETRANGE` / `UDM_SETRANGE32`). Nothing
//! sets the step or the acceleration, so a click or an Up or Down key moves the number by 1 and
//! stops at the ends of the range (no `UDS_WRAP`). The up-down control reads the number from the
//! box (`UDS_SETBUDDYINT`), so what the user typed is the number it starts from, and it writes
//! the result back to the box.
//!
//! This is the one place that does it for every dialog of MATSU: [`spin_box`] puts the two
//! arrows at the right of the box and the Up and Down keys on the box ([`bind_keys`]), and
//! [`stepped`] is the rule. The arrows are two small buttons (▲ and ▼) in a column, as Win32 draws them, with a
//! name for a screen reader. Differences from Win32: holding an arrow does not repeat (Win32
//! repeats, and goes faster after a while).

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, Div, ElementId, Entity, InteractiveElement as _, IntoElement, KeyBinding,
    ParentElement as _, SharedString, Stateful, StatefulInteractiveElement as _, Styled as _,
    Window, actions, div, px, rgb,
};
use gpui_component::input::InputState;

actions!(
    matsu_spin,
    [
        /// The Up key in the box of a spin button.
        SpinUp,
        /// The Down key in the box of a spin button.
        SpinDown
    ]
);

/// The key context of the element that holds a spin button and its box.
const CONTEXT: &str = "SpinBox";

/// Whether the keys are bound (once per application).
struct KeysBound;

impl gpui::Global for KeysBound {}

/// Binds Up and Down in the box of a spin button (once per application; `input_field` calls it
/// for every field of a dialog). A single-line box takes the arrow keys as its own actions
/// (`MoveUp`, `MoveDown`) and does nothing with them, and a key listener of an element around it is
/// never called for a key that an action took. So the keys are bound in the box itself, on the
/// context `SpinBox > Input`: it is as deep as the box's own and bound later, so it comes first.
pub(crate) fn bind_keys(cx: &mut App) {
    if cx.has_global::<KeysBound>() {
        return;
    }
    cx.set_global(KeysBound);
    cx.bind_keys([
        KeyBinding::new("up", SpinUp, Some("SpinBox > Input")),
        KeyBinding::new("down", SpinDown, Some("SpinBox > Input")),
    ]);
}

const BORDER: u32 = 0xaaaaaa;
const FACE: u32 = 0xf0f0f0;
const FACE_HOVER: u32 = 0xe0e8f4;
const TEXT: u32 = 0x222222;
const DISABLED_TEXT: u32 = 0xaaaaaa;

/// The width of the column of arrows.
const ARROW_WIDTH: f32 = 18.;

/// The number after one step of `delta` (1 or -1) from the number in `text`, kept in `min..=max`
/// (the range of `UDM_SETRANGE`; the control has no `UDS_WRAP`). The text is read as a number
/// (`GetDlgItemInt`: a sign only where the range goes below 0); text that is not a number counts
/// as 0.
pub(crate) fn stepped(text: &str, delta: i32, min: i32, max: i32) -> i32 {
    let text = text.trim();
    let digits = text.strip_prefix('-').filter(|_| min < 0).unwrap_or(text);
    let parsed = if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        text.parse::<i64>().unwrap_or(if text.starts_with('-') {
            i64::MIN
        } else {
            i64::MAX
        })
    } else {
        0
    };
    let next = parsed
        .saturating_add(i64::from(delta))
        .clamp(i64::from(min), i64::from(max));
    i32::try_from(next).unwrap_or(max)
}

/// One step of `delta` on the box `input`: the number in it moves, the box shows it and has the
/// focus (so that the next Up or Down key goes on).
pub(crate) fn step(
    input: &Entity<InputState>,
    delta: i32,
    min: i32,
    max: i32,
    window: &mut Window,
    cx: &mut App,
) {
    let text = input.read(cx).value().to_string();
    let next = stepped(&text, delta, min, max).to_string();
    input.update(cx, |state, cx| {
        state.set_value(next, window, cx);
        state.focus(window, cx);
    });
}

/// `field` (the box of `input`) with the arrows at its right, and the Up and Down keys of the
/// box. The arrows are named `name` + 「を増やす」 and 「を減らす」 for a screen reader. `id`
/// must be unique in the window. With `enabled` off the arrows are grey and do nothing.
pub(crate) fn spin_box(
    id: impl Into<SharedString>,
    input: &Entity<InputState>,
    name: &str,
    (min, max): (i32, i32),
    enabled: bool,
    field: impl IntoElement,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let arrow = |part: &'static str, glyph: &'static str, label: String, delta: i32| {
        let input = input.clone();
        let text = if enabled { TEXT } else { DISABLED_TEXT };
        let arrow_id = format!("{id}-{part}");
        crate::a11y::button(
            div().id(ElementId::from(SharedString::from(arrow_id.clone()))),
            &label,
        )
        .debug_selector(move || arrow_id)
        .w(px(ARROW_WIDTH))
        .flex_1()
        .min_h_0()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(8.))
        .text_color(rgb(text))
        .bg(rgb(FACE))
        .border_1()
        .border_color(rgb(BORDER))
        .when(enabled, |d| {
            d.cursor_pointer()
                .hover(|d| d.bg(rgb(FACE_HOVER)))
                .on_click(move |_, window, cx| step(&input, delta, min, max, window, cx))
        })
        .child(glyph)
    };
    let up = arrow("up", "▲", format!("{name}を増やす"), 1);
    let down = arrow("down", "▼", format!("{name}を減らす"), -1);
    let up_input = input.clone();
    let down_input = input.clone();
    div()
        .id(ElementId::from(id.clone()))
        .key_context(CONTEXT)
        .flex()
        .items_stretch()
        .child(field)
        .child(div().flex().flex_col().child(up).child(down))
        // `UDS_ARROWKEYS`
        .on_action(move |_: &SpinUp, window, cx| {
            if enabled {
                step(&up_input, 1, min, max, window, cx);
            }
        })
        .on_action(move |_: &SpinDown, window, cx| {
            if enabled {
                step(&down_input, -1, min, max, window, cx);
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_moves_by_one_and_stops_at_the_ends() {
        assert_eq!(stepped("5", 1, 1, 10), 6);
        assert_eq!(stepped("5", -1, 1, 10), 4);
        assert_eq!(stepped("10", 1, 1, 10), 10);
        assert_eq!(stepped("1", -1, 1, 10), 1);
        // no wrapping around (no `UDS_WRAP`)
        assert_eq!(stepped("0", -1, 0, 32), 0);
        assert_eq!(stepped("32", 1, 0, 32), 32);
    }

    #[test]
    fn the_number_the_user_typed_is_the_one_it_starts_from() {
        assert_eq!(stepped("  7 ", 1, 0, 99), 8);
        // out of the range: the step ends in the range
        assert_eq!(stepped("500", 1, 0, 32), 32);
        assert_eq!(stepped("500", -1, 0, 32), 32);
        assert_eq!(stepped("0", 1, 5, 32), 5);
        // not a number: 0, then the range
        assert_eq!(stepped("", 1, 0, 32), 1);
        assert_eq!(stepped("", 1, 2, 32), 2);
        assert_eq!(stepped("abc", -1, 0, 32), 0);
        assert_eq!(stepped("99999999999999999999", -1, 0, 32), 32);
    }

    #[test]
    fn a_sign_counts_only_where_the_range_goes_below_zero() {
        assert_eq!(stepped("-5", 1, -128, 128), -4);
        assert_eq!(stepped("-128", -1, -128, 128), -128);
        assert_eq!(stepped("-5", 1, 0, 128), 1);
        assert_eq!(stepped("-", 1, -128, 128), 1);
        assert_eq!(stepped("-99999999999999999999", 1, -30000, 30000), -30000);
    }
}
