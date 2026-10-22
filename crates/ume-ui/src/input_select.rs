//! Text fields of the dialogs select their text when the keyboard moves into them.
//!
//! In a Windows dialog, moving into a single-line edit control with Tab, Shift+Tab or an access
//! key (a mnemonic) selects all of its text: the dialog manager moves the focus itself
//! (`WM_NEXTDLGCTL`, `IsDialogMessage`) and selects the text of an edit that answers
//! `DLGC_HASSETSEL` to `WM_GETDLGCODE` (Microsoft Learn, "Dialog Box Keyboard Interface" and
//! "WM_GETDLGCODE"). The first edit of a dialog gets the focus the same way when `WM_INITDIALOG`
//! returns TRUE, so a dialog opens with that text selected. A click only places the caret, and
//! so does a plain `SetFocus` call. macOS selects the contents of a text field that Tab moves
//! into as well. Sakura uses the stock dialogs, so every edit of its dialogs behaves so; the
//! coordinator saw it on Sakura (the path of the file tree dialog, 2026-10-22).
//!
//! One place does it for every dialog: [`input_field`] makes a field that selects all
//! when it gets the focus while the last input was a key (GPUI's input modality, set before the
//! handlers of the event run). Tab and the access keys are keys; a click is not. A dialog that
//! opens with its first field focused calls [`focus_selecting`], because the menu item or the
//! button that opened it may have been clicked.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{App, Context, Entity, Focusable as _, Window};
use gpui_component::input::InputState;

/// A single-line field that selects all of its text when the keyboard moves the focus into it.
/// Use it instead of `InputState::new` for every field of a dialog:
/// `cx.new(|cx| input_field(window, cx).placeholder("..."))`. The listener goes away with the
/// field.
pub(crate) fn input_field(window: &mut Window, cx: &mut Context<InputState>) -> InputState {
    let state = InputState::new(window, cx);
    let handle = state.focus_handle(cx);
    // The window gets back the focus it lost with its activation (Alt+Tab from another program):
    // that is not the keyboard moving into the field, and the text stays as it was.
    let lost_with_the_window = Rc::new(Cell::new(false));
    cx.on_blur(&handle, window, {
        let lost_with_the_window = lost_with_the_window.clone();
        move |_, window, _| lost_with_the_window.set(!window.is_window_active())
    })
    .detach();
    cx.on_focus(&handle, window, move |state, window, cx| {
        if lost_with_the_window.take() {
            return;
        }
        if window.last_input_was_keyboard() {
            state.select_all(window, cx);
        }
    })
    .detach();
    state
}

/// Focuses the field and selects all of its text, as a dialog does for its first edit when it
/// opens.
pub(crate) fn focus_selecting(input: &Entity<InputState>, window: &mut Window, cx: &mut App) {
    input.update(cx, |state, cx| {
        state.focus(window, cx);
        state.select_all(window, cx);
    });
}

#[cfg(test)]
mod tests {
    use gpui::{
        AppContext as _, Context, InteractiveElement as _, IntoElement, KeyDownEvent, Modifiers,
        ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext, div, px,
    };
    use gpui_component::input::Input;

    use super::*;

    /// Two fields below each other; Tab moves the focus as the window root does.
    struct Fields {
        first: Entity<InputState>,
        second: Entity<InputState>,
    }

    impl Render for Fields {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .capture_key_down(cx.listener(|_, event: &KeyDownEvent, window, cx| {
                    match (
                        event.keystroke.key.as_str(),
                        event.keystroke.modifiers.shift,
                    ) {
                        ("tab", false) => window.focus_next(cx),
                        ("tab", true) => window.focus_prev(cx),
                        _ => return,
                    }
                    cx.stop_propagation();
                }))
                .child(
                    div()
                        .debug_selector(|| "first".to_owned())
                        .h(px(40.))
                        .child(Input::new(&self.first)),
                )
                .child(
                    div()
                        .debug_selector(|| "second".to_owned())
                        .h(px(40.))
                        .child(Input::new(&self.second)),
                )
        }
    }

    fn fields(cx: &mut TestAppContext) -> (Entity<Fields>, &mut VisualTestContext) {
        cx.update(gpui_component::init);
        let (fields, keys) = cx.add_window_view(|window, cx| {
            let field = |text: &str, window: &mut Window, cx: &mut Context<Fields>| {
                let state = cx.new(|cx| input_field(window, cx));
                state.update(cx, |state, cx| state.set_value(text.to_owned(), window, cx));
                state
            };
            Fields {
                first: field("alpha", window, cx),
                second: field("日本語のテキスト", window, cx),
            }
        });
        keys.update(|window, _| window.activate_window());
        (fields, keys)
    }

    fn selected(fields: &Entity<Fields>, second: bool, keys: &mut VisualTestContext) -> String {
        let state = fields.read_with(keys, |f, _| {
            if second {
                f.second.clone()
            } else {
                f.first.clone()
            }
        });
        state.read_with(keys, |s, _| {
            let range = s.selected_range();
            s.value()[range].to_owned()
        })
    }

    /// Draws a frame: the focus listeners run when the window has drawn the new focus.
    fn draw(keys: &mut VisualTestContext) {
        keys.update(|window, _| window.refresh());
        keys.run_until_parked();
    }

    fn focus(fields: &Entity<Fields>, second: bool, keys: &mut VisualTestContext) {
        let state = fields.read_with(keys, |f, _| {
            if second {
                f.second.clone()
            } else {
                f.first.clone()
            }
        });
        keys.update(|window, cx| {
            state.update(cx, |state, cx| state.focus(window, cx));
        });
        keys.run_until_parked();
    }

    #[gpui::test]
    fn tab_and_shift_tab_select_the_text_of_the_field_they_move_into(cx: &mut TestAppContext) {
        let (fields, keys) = fields(cx);
        focus(&fields, false, keys);
        keys.simulate_keystrokes("tab");
        draw(keys);
        assert_eq!(selected(&fields, true, keys), "日本語のテキスト");
        keys.simulate_keystrokes("shift-tab");
        draw(keys);
        assert_eq!(selected(&fields, false, keys), "alpha");
    }

    #[gpui::test]
    fn a_click_only_places_the_caret(cx: &mut TestAppContext) {
        let (fields, keys) = fields(cx);
        focus(&fields, false, keys);
        let bounds = keys.debug_bounds("second").expect("the field is drawn");
        keys.simulate_click(bounds.center(), Modifiers::none());
        draw(keys);
        assert_eq!(selected(&fields, true, keys), "");
        // the click did move the focus into the field
        let second = fields.read_with(keys, |f, _| f.second.clone());
        let focus = second.read_with(keys, |s, cx| s.focus_handle(cx));
        assert!(keys.update(|window, _| focus.is_focused(window)));
    }

    #[gpui::test]
    fn focus_selecting_selects_the_text_whatever_the_last_input_was(cx: &mut TestAppContext) {
        let (fields, keys) = fields(cx);
        // a click, as when a dialog is opened with the mouse
        focus(&fields, false, keys);
        let bounds = keys.debug_bounds("first").expect("the field is drawn");
        keys.simulate_click(bounds.center(), Modifiers::none());
        let second = fields.read_with(keys, |f, _| f.second.clone());
        keys.update(|window, cx| focus_selecting(&second, window, cx));
        keys.run_until_parked();
        assert_eq!(selected(&fields, true, keys), "日本語のテキスト");
    }
}
