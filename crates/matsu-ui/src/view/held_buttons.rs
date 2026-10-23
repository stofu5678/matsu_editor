//! The mouse buttons that are down while the wheel turns, and the wheel combinations of the
//! common settings 「全般」 (`m_nPageScrollByWheel`, `m_nHorizontalScrollByWheel`).
//!
//! Sakura: `CEditView::IsSpecialScrollMode` (`sakura_core/view/CEditView_Mouse.cpp`) asks the OS
//! whether the chosen key or button is down (`GetAsyncKeyState`). GPUI does not say which buttons
//! are down when the wheel turns, so the view keeps the middle button and the two side buttons
//! (back and forward) itself.

use gpui::{Div, InteractiveElement, Modifiers, MouseButton, NavigationDirection, Stateful};

use super::EditorView;
use crate::common_settings::{
    WHEEL_CONTROL_KEY, WHEEL_MIDDLE_BUTTON, WHEEL_SHIFT_KEY, WHEEL_SIDE_BUTTON_1,
    WHEEL_SIDE_BUTTON_2,
};

/// The buttons that count for the wheel combinations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct HeldButtons {
    pub middle: bool,
    /// `VK_XBUTTON1`: GPUI's back button.
    pub side_1: bool,
    /// `VK_XBUTTON2`: GPUI's forward button.
    pub side_2: bool,
}

impl HeldButtons {
    fn set(&mut self, button: MouseButton, down: bool) {
        match button {
            MouseButton::Middle => self.middle = down,
            MouseButton::Navigate(NavigationDirection::Back) => self.side_1 = down,
            MouseButton::Navigate(NavigationDirection::Forward) => self.side_2 = down,
            _ => {}
        }
    }

    /// `IsSpecialScrollMode`: whether the key or button of the setting `option` is down.
    pub(crate) fn special_mode(self, option: i32, modifiers: Modifiers) -> bool {
        match option {
            WHEEL_MIDDLE_BUTTON => self.middle,
            WHEEL_SIDE_BUTTON_1 => self.side_1,
            WHEEL_SIDE_BUTTON_2 => self.side_2,
            WHEEL_CONTROL_KEY => modifiers.control,
            WHEEL_SHIFT_KEY => modifiers.shift,
            _ => false,
        }
    }
}

/// Makes the view keep the state of the buttons that the wheel combinations use.
pub(crate) fn track(element: Stateful<Div>, cx: &mut gpui::Context<EditorView>) -> Stateful<Div> {
    let mut element = element;
    for button in [
        MouseButton::Middle,
        MouseButton::Navigate(NavigationDirection::Back),
        MouseButton::Navigate(NavigationDirection::Forward),
    ] {
        element = element
            .on_mouse_down(
                button,
                cx.listener(move |view, _, _, _| {
                    view.held_buttons.set(button, true);
                    // A new press starts over: the last release may have been outside the view.
                    view.wheel_used_middle = false;
                }),
            )
            .on_mouse_up(
                button,
                cx.listener(move |view, _, _, _| view.held_buttons.set(button, false)),
            )
            .on_mouse_up_out(
                button,
                cx.listener(move |view, _, _, _| view.held_buttons.set(button, false)),
            );
    }
    element
}
