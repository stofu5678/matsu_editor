//! Opening the menus from the keyboard on Windows and Linux, the way a Win32 menu bar does.
//!
//! * Alt+letter opens the menu with that access key, unless the key map gives the chord to a
//!   command: Sakura's accelerator table (`CKeyBind::CreateAccerelator`) is translated before the
//!   system sees the key, so a command on Alt+letter runs and only the other chords open a menu.
//! * A tap of Alt (press and release with nothing in between) marks the first menu without opening
//!   it. Then Left and Right move the mark, Down, Up or Enter open the menu, a letter opens the
//!   menu with that access key, and Esc, another Alt or any other key ends it.
//! * F10 is not used: Sakura binds it to a command (run in the database).
//!
//! The listeners are on the root of the window ([`Workspace`]); the letters in an open menu are
//! handled by `BarMenu`. Nothing here reacts while the IME composes.

use super::MenuBar;
use super::access::key_of;
use crate::app::{MenuKeyMap, Workspace};
use gpui::{Context, KeyDownEvent, Modifiers, ModifiersChangedEvent, MouseDownEvent, Window};
use matsu_commands::keymap::Lookup;

impl MenuBar {
    /// Opens the menu whose access key is `key`; with several, the next one after the menu that is
    /// open or marked. Returns whether a menu was opened.
    pub(crate) fn open_by_key(
        &mut self,
        key: char,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = key.to_ascii_uppercase();
        let matches: Vec<usize> = (0..self.menus.len())
            .filter(|&i| crate::a11y::key_hint(&self.menus[i].read(cx).name) == Some(key))
            .collect();
        let from = self.selected_index.or(self.armed);
        let Some(ix) = super::access::next_match(&matches, from) else {
            return false;
        };
        self.armed = None;
        self.set_selected_index(Some(ix), window, cx);
        true
    }

    /// Whether the bar is marked for the arrow keys (after a tap of Alt).
    #[cfg(test)]
    pub(crate) fn armed(&self) -> Option<usize> {
        self.armed
    }

    /// The modifier keys changed: a tap of Alt marks the first menu, or ends the mark, or closes
    /// the open menu (Windows does the same).
    pub(crate) fn modifiers_changed(
        &mut self,
        modifiers: Modifiers,
        applies: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A tap that ends where the keys do not apply (in a dialog) is not for the bar.
        if !self.alt.changed(modifiers) || !applies {
            return;
        }
        if self.armed.take().is_some() {
            cx.notify();
        } else if self.selected_index.is_some() {
            self.set_selected_index(None, window, cx);
        } else if !self.menus.is_empty() {
            self.armed = Some(0);
            cx.notify();
        }
    }

    /// A mouse button went down: Alt was used with the mouse, and a marked bar is let go.
    pub(crate) fn mouse_down(&mut self, cx: &mut Context<Self>) {
        self.alt.other_input();
        if self.armed.take().is_some() {
            cx.notify();
        }
    }

    /// A key went down, before the view under the focus sees it. While the bar is marked the arrow
    /// keys and the letters are for the bar. Returns whether the key was taken.
    pub(crate) fn capture_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.alt.other_input();
        let Some(armed) = self.armed else {
            return false;
        };
        let keystroke = &event.keystroke;
        let m = keystroke.modifiers;
        if event.prefer_character_input
            || keystroke.is_ime_in_progress()
            || m.control
            || m.alt
            || m.platform
            || m.function
        {
            self.armed = None;
            cx.notify();
            return false;
        }
        let count = self.menus.len();
        match keystroke.key.as_str() {
            "left" => {
                self.armed = Some((armed + count - 1) % count);
                cx.notify();
            }
            "right" => {
                self.armed = Some((armed + 1) % count);
                cx.notify();
            }
            "down" | "up" | "enter" => {
                self.armed = None;
                self.set_selected_index(Some(armed), window, cx);
            }
            "escape" => {
                self.armed = None;
                cx.notify();
            }
            key => {
                let Some(letter) = key_of(key) else {
                    self.armed = None;
                    cx.notify();
                    return false;
                };
                // A letter that no menu has is swallowed too (Windows beeps).
                if !self.open_by_key(letter, window, cx) {
                    self.armed = None;
                    cx.notify();
                }
            }
        }
        true
    }
}

impl Workspace {
    /// Whether the keys of the menu bar apply now: the focus is in the text view, not in a dialog
    /// or the menus' own popup.
    fn menu_keys_apply(&self, window: &Window, cx: &gpui::App) -> bool {
        if self.content.preview().is_some() {
            return true;
        }
        let editor = self.editor().read(cx);
        editor.marked.is_none() && editor.focus_handle.contains_focused(window, cx)
    }

    /// Before the view under the focus: the keys of a marked bar.
    pub(crate) fn menu_key_capture(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .menu_bar
            .update(cx, |bar, cx| bar.capture_key(event, window, cx))
        {
            cx.stop_propagation();
        }
    }

    /// After the view under the focus has not taken the key: Alt+letter opens a menu.
    pub(crate) fn menu_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        let m = keystroke.modifiers;
        if event.prefer_character_input
            || keystroke.is_ime_in_progress()
            || !m.alt
            || m.shift
            || m.control
            || m.platform
            || m.function
        {
            return;
        }
        let Some(key) = key_of(&keystroke.key) else {
            return;
        };
        // A chord of the key map wins: the command runs (or says that it is not there yet).
        if let Some(chord) = crate::keys::chord_of(keystroke)
            && cx.try_global::<MenuKeyMap>().is_some_and(|map| {
                matches!(
                    map.0.lookup(chord),
                    Lookup::Command(_) | Lookup::NotImplemented { .. }
                )
            })
        {
            return;
        }
        let in_menu = self.menu_bar.read(cx).selected_index.is_some();
        if !in_menu && !self.menu_keys_apply(window, cx) {
            return;
        }
        if self
            .menu_bar
            .update(cx, |bar, cx| bar.open_by_key(key, window, cx))
        {
            cx.stop_propagation();
        }
    }

    /// Alt went down or up.
    pub(crate) fn menu_modifiers(
        &mut self,
        event: &ModifiersChangedEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let applies =
            self.menu_keys_apply(window, cx) || self.menu_bar.read(cx).selected_index.is_some();
        self.menu_bar.update(cx, |bar, cx| {
            bar.modifiers_changed(event.modifiers, applies, window, cx);
        });
    }

    /// A mouse button went down anywhere in the window.
    pub(crate) fn menu_mouse_down(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_bar.update(cx, |bar, cx| bar.mouse_down(cx));
    }
}
