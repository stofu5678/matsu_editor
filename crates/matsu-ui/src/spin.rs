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
//! name for a screen reader.
//!
//! Holding an arrow repeats and goes faster after a while ([`Hold`]). The Up and Down keys repeat with
//! the operating system's key repeat (the box gets the key again and again), one step each, so the
//! acceleration is only for the arrows.
//!
//! The delays and steps are those of the system's up-down control (`msctls_updown32`), which Sakura
//! does not change (no `UDM_SETACCEL` in `sakura_core`). They were measured on Windows 11 (arm64,
//! 2026-11-21, in Windows Sandbox) with a native control made with `UDS_SETBUDDYINT`,
//! `UDS_ALIGNRIGHT`, `UDS_AUTOBUDDY` and `UDS_ARROWKEYS`, range 0 to 100000: the up arrow held with
//! the mouse for 8 seconds, the position read every 5 ms. The control steps by 1 at the press
//! (40 ms), makes the first repeat at 554 ms, and then one every 111 to 112 ms (554, 666, 778, 889,
//! 1017, 1129, 1240 ms and so on). The steps are 1 until 2 seconds, 5 from 2 seconds (the position
//! was 12 at 1.686 s, 45 at 2.691 s, 95 at 3.825 s, 145 at 4.924 s) and 20 from 5 seconds (340 at
//! 6.039 s, 540 at 7.184 s, then +20 every 111 ms). So: a wait of [`FIRST_DELAY`] (500 ms), then a
//! step every [`INTERVAL`] (100 ms: the system's timer of 100 ms is rounded up to its 15.6 ms
//! tick, which is why it shows as about 110 ms there), and a step of 1, 5 after 2 seconds, 20 after
//! 5 seconds.

use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, Context, Div, ElementId, Entity, Focusable as _, InteractiveElement as _, IntoElement,
    KeyBinding, MouseButton, ParentElement as _, SharedString, Stateful,
    StatefulInteractiveElement as _, Styled as _, Subscription, Task, Window, actions, div, px,
    rgb,
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

/// The wait before the first repeat of a held arrow (measured: 554 ms on Windows, see the top of this file).
pub(crate) const FIRST_DELAY: Duration = Duration::from_millis(500);
/// The wait between the repeats after that (measured: about 110 ms on Windows, a timer of 100 ms).
pub(crate) const INTERVAL: Duration = Duration::from_millis(100);

/// The wait before the `n`th repeat (0 for the first), after the press, which stepped by itself.
pub(crate) fn delay_before(n: u32) -> Duration {
    if n == 0 { FIRST_DELAY } else { INTERVAL }
}

/// How many steps one repeat takes after the arrow has been held for `held`: the table measured on
/// Windows (1 until 2 seconds, 5 until 5 seconds, then 20).
pub(crate) fn increment_after(held: Duration) -> i32 {
    match held.as_secs() {
        0..=1 => 1,
        2..=4 => 5,
        _ => 20,
    }
}

/// How long the arrow has been held at the `n`th repeat (1 for the first): the first wait and the
/// `n - 1` waits after it. Counted from the waits, not read from a clock, so that it is the same
/// whatever the speed of the machine is.
fn held_at(n: u32) -> Duration {
    FIRST_DELAY + INTERVAL * n.saturating_sub(1)
}

/// The press of an arrow that repeats: the owner of the task that does the repeats. It lives as long
/// as the spin box is drawn (`Window::use_keyed_state`), and dropping it drops the task. The task is
/// dropped when the button is released, when the focus moves away and when the window is
/// deactivated (the release in another program never arrives; Win32's control ends the press when
/// it loses the capture, `WM_CAPTURECHANGED`).
pub(crate) struct Hold {
    task: Option<Task<()>>,
    /// The box's loss of the focus, while a press is on.
    focus_out: Option<Subscription>,
    _activation: Subscription,
}

impl Hold {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _activation = cx.observe_window_activation(window, |this, window, _| {
            if !window.is_window_active() {
                this.end();
            }
        });
        Self {
            task: None,
            focus_out: None,
            _activation,
        }
    }

    /// The release: the repeat ends.
    pub(crate) fn end(&mut self) {
        self.task = None;
        self.focus_out = None;
    }

    /// The press has stepped once; now it waits and repeats `delta` on `input` until it ends.
    fn start(
        &mut self,
        input: Entity<InputState>,
        delta: i32,
        (min, max): (i32, i32),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The box has the focus after the step; when it loses it (Tab, a click elsewhere) the press
        // is over, so that a repeat does not take the focus back.
        self.focus_out = Some(
            cx.on_focus_out(&input.focus_handle(cx), window, |this, _, _, _| {
                this.end();
            }),
        );
        self.task = Some(cx.spawn_in(window, async move |this, cx| {
            let mut n = 0;
            loop {
                cx.background_executor().timer(delay_before(n)).await;
                n += 1;
                let step_by = delta * increment_after(held_at(n));
                // `Err`: the box is gone
                if this
                    .update_in(cx, |_, window, cx| {
                        step(&input, step_by, min, max, window, cx)
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }
}

/// The number after one step of `delta` (a multiple of 1 or -1) from the number in `text`, kept in `min..=max`
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
    let next = stepped(&input.read(cx).value(), delta, min, max).to_string();
    input.update(cx, |state, cx| {
        state.set_value(next, window, cx);
        state.focus(window, cx);
    });
}

/// One arrow: a button with its name, grey, disabled for assistive technology and without a click
/// while `enabled` is off. `on_press` runs when the button goes down (Win32 steps then, not at the
/// release) and `on_release` when it goes up, on the arrow or off it.
fn arrow_button(
    arrow_id: String,
    glyph: &'static str,
    label: &str,
    enabled: bool,
    on_press: impl Fn(&mut Window, &mut App) + 'static,
    on_release: impl Fn(&mut Window, &mut App) + Clone + 'static,
) -> Stateful<Div> {
    use crate::a11y::AriaDisabled as _;

    let text = if enabled { TEXT } else { DISABLED_TEXT };
    crate::a11y::button(
        div().id(ElementId::from(SharedString::from(arrow_id.clone()))),
        label,
    )
    .aria_disabled(!enabled)
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
            // A mouse click has stepped at the press already. The handler stays so that the node
            // says it can be clicked (a screen reader's click is a press and a release, which reach
            // the handlers below).
            .on_click(|_, _, _| {})
            .on_mouse_down(MouseButton::Left, move |_, window, cx| on_press(window, cx))
            .on_mouse_up(MouseButton::Left, {
                let on_release = on_release.clone();
                move |_, window, cx| on_release(window, cx)
            })
            .on_mouse_up_out(MouseButton::Left, move |_, window, cx| {
                on_release(window, cx);
            })
    })
    .child(glyph)
}

/// `field` (the box of `input`) with the arrows at its right, and the Up and Down keys of the
/// box. The arrows are named `name` + 「を増やす」 and 「を減らす」 for a screen reader. `id`
/// must be unique in the window. With `enabled` off the arrows are grey and do nothing.
// The window and the app come last, as in every `Render` helper; the rest is what the caller states.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spin_box(
    id: impl Into<SharedString>,
    input: &Entity<InputState>,
    name: &str,
    (min, max): (i32, i32),
    enabled: bool,
    field: impl IntoElement,
    window: &mut Window,
    cx: &mut App,
) -> Stateful<Div> {
    let id: SharedString = id.into();
    let hold = window.use_keyed_state(ElementId::from(id.clone()), cx, |window, cx| {
        Hold::new(window, cx)
    });
    // A press that was on when the arrows went grey has no release handler any more: the owner
    // that sees the state ends it.
    if !enabled {
        hold.update(cx, |hold, _| hold.end());
    }
    let arrow = |part: &'static str, glyph: &'static str, label: String, delta: i32| {
        let input = input.clone();
        let press_hold = hold.clone();
        let release_hold = hold.clone();
        arrow_button(
            format!("{id}-{part}"),
            glyph,
            &label,
            enabled,
            move |window, cx| {
                // A focusable element round the box (the dialog) takes the focus at the press, in
                // its own handler after this one, unless it is told not to: the focus stays where it was.
                window.prevent_default();
                step(&input, delta, min, max, window, cx);
                press_hold.update(cx, |hold, cx| {
                    hold.start(input.clone(), delta, (min, max), window, cx);
                });
            },
            move |_, cx| release_hold.update(cx, |hold, _| hold.end()),
        )
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
    fn an_arrow_is_a_named_button_that_can_be_clicked_only_while_enabled() {
        use gpui::accesskit::Action;

        let node = |enabled: bool| {
            crate::a11y::node_of(&arrow_button(
                "a".to_owned(),
                "▲",
                "行数を増やす",
                enabled,
                |_, _| {},
                |_, _| {},
            ))
            .expect("an id and a role")
        };
        let on = node(true);
        assert_eq!(on.role(), gpui::Role::Button);
        assert_eq!(on.label(), Some("行数を増やす"));
        assert!(on.supports_action(Action::Click));
        // disabled: the click is gone and the node says it is disabled (`aria_disabled`)
        let off = node(false);
        assert_eq!(off.label(), Some("行数を増やす"));
        assert!(!off.supports_action(Action::Click));
    }

    #[test]
    fn it_waits_longer_before_the_first_repeat_than_between_the_others() {
        assert_eq!(delay_before(0), Duration::from_millis(500));
        assert_eq!(delay_before(1), Duration::from_millis(100));
        assert_eq!(delay_before(100), Duration::from_millis(100));
    }

    #[test]
    fn the_step_grows_with_the_time_the_arrow_is_held() {
        let ms = Duration::from_millis;
        assert_eq!(increment_after(ms(0)), 1);
        assert_eq!(increment_after(ms(1999)), 1);
        assert_eq!(increment_after(ms(2000)), 5);
        assert_eq!(increment_after(ms(4999)), 5);
        assert_eq!(increment_after(ms(5000)), 20);
        assert_eq!(increment_after(Duration::from_secs(60)), 20);
    }

    #[test]
    fn the_time_held_is_counted_from_the_waits() {
        // the first repeat comes after the first delay, the others one interval apart
        assert_eq!(held_at(1), Duration::from_millis(500));
        assert_eq!(held_at(2), Duration::from_millis(600));
        assert_eq!(held_at(16), Duration::from_millis(2000));
        assert_eq!(increment_after(held_at(15)), 1);
        assert_eq!(increment_after(held_at(16)), 5);
        assert_eq!(increment_after(held_at(45)), 5);
        assert_eq!(increment_after(held_at(46)), 20);
    }

    #[test]
    fn a_step_of_several_still_stops_at_the_ends() {
        assert_eq!(stepped("90", 20, 1, 99), 99);
        assert_eq!(stepped("10", -20, 1, 99), 1);
        assert_eq!(stepped("-10", -20, -30000, 30000), -30);
    }

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
