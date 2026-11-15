//! The input handler of the terminal: GPUI's `ElementInputHandler` for the view, with one difference.
//!
//! macOS opens the accent picker when a letter key is held, unless the handler asks for the raw key repeats
//! (`InputHandler::apple_press_and_hold_enabled`). A terminal repeats the key, as iTerm2 does with the setting
//! off. `ElementInputHandler` always answers yes, so this wraps it and answers no; every other call goes to it.
//! The IME path (marked text, commit) is the same.

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, ElementInputHandler, InputHandler, Pixels, Point,
    TextInputConfiguration, UTF16Selection, Window,
};

use super::view::TerminalView;

pub(super) struct TerminalInputHandler(pub ElementInputHandler<TerminalView>);

impl InputHandler for TerminalInputHandler {
    fn selected_text_range(
        &mut self,
        ignore_disabled_input: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<UTF16Selection> {
        self.0
            .selected_text_range(ignore_disabled_input, window, cx)
    }

    fn marked_text_range(&mut self, window: &mut Window, cx: &mut App) -> Option<Range<usize>> {
        self.0.marked_text_range(window, cx)
    }

    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<String> {
        self.0
            .text_for_range(range_utf16, adjusted_range, window, cx)
    }

    fn replace_text_in_range(
        &mut self,
        replacement_range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.0
            .replace_text_in_range(replacement_range, text, window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.0.replace_and_mark_text_in_range(
            range_utf16,
            new_text,
            new_selected_range,
            window,
            cx,
        );
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut App) {
        self.0.unmark_text(window, cx);
    }

    fn paste(&mut self, item: ClipboardItem, window: &mut Window, cx: &mut App) {
        self.0.paste(item, window, cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Bounds<Pixels>> {
        self.0.bounds_for_range(range_utf16, window, cx)
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<usize> {
        self.0.character_index_for_point(point, window, cx)
    }

    fn set_selected_text_range(
        &mut self,
        range_utf16: Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.0.set_selected_text_range(range_utf16, window, cx);
    }

    fn element_bounds(&mut self, window: &mut Window, cx: &mut App) -> Option<Bounds<Pixels>> {
        self.0.element_bounds(window, cx)
    }

    fn text_length_utf16(&mut self, window: &mut Window, cx: &mut App) -> Option<usize> {
        self.0.text_length_utf16(window, cx)
    }

    /// Holding a key repeats it (a terminal's way), not the accent picker's.
    fn apple_press_and_hold_enabled(&mut self) -> bool {
        false
    }

    fn accepts_text_input(&mut self, window: &mut Window, cx: &mut App) -> bool {
        self.0.accepts_text_input(window, cx)
    }

    /// With a Japanese (or other) input source on macOS, GPUI offers a printable key to the IME before the view
    /// sees it. Option+key is Meta when the setting says so, and that is the view's (`translate_with`): the
    /// IME would commit the character Option makes instead of ESC and the key. (A dead key such as Option+E has
    /// no character and always goes to the IME first, so it is not Meta.)
    fn prefers_ime_for_printable_keys(&mut self, window: &mut Window, cx: &mut App) -> bool {
        let option_is_meta = cfg!(target_os = "macos")
            && super::settings::current(cx).option_as_meta
            && window.modifiers().alt;
        !option_is_meta && self.0.prefers_ime_for_printable_keys(window, cx)
    }

    fn text_input_configuration(
        &mut self,
        window: &mut Window,
        cx: &mut App,
    ) -> TextInputConfiguration {
        self.0.text_input_configuration(window, cx)
    }

    fn text_input_editable_range(
        &mut self,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Range<usize>> {
        self.0.text_input_editable_range(window, cx)
    }
}
