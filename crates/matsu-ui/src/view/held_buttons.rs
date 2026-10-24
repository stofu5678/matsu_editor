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

    /// Marks the button of the setting `option` (a key is not a button: nothing is marked).
    pub(crate) fn mark_option(&mut self, option: i32) {
        match option {
            WHEEL_MIDDLE_BUTTON => self.middle = true,
            WHEEL_SIDE_BUTTON_1 => self.side_1 = true,
            WHEEL_SIDE_BUTTON_2 => self.side_2 = true,
            _ => {}
        }
    }

    /// Whether `button` is marked, and clears the mark.
    pub(crate) fn take(&mut self, button: MouseButton) -> bool {
        let mark = match button {
            MouseButton::Middle => &mut self.middle,
            MouseButton::Navigate(NavigationDirection::Back) => &mut self.side_1,
            MouseButton::Navigate(NavigationDirection::Forward) => &mut self.side_2,
            _ => return false,
        };
        std::mem::take(mark)
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
    // A release in another window never arrives: a move with no button down says that the middle
    // button is up. (The side buttons are not in the button mask of every backend, X11's among
    // them, so a move says nothing about them.)
    let mut element =
        element.on_mouse_move(cx.listener(|view, event: &gpui::MouseMoveEvent, _, _| {
            if event.pressed_button.is_none() {
                view.held_buttons.middle = false;
            }
        }));
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
                    view.wheel_used_buttons.take(button);
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
