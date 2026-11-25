//! A closed drop-down list (`CBS_DROPDOWNLIST`) drawn inside our own dialogs: the kind of window
//! of the window position dialog (`IDC_COMBO_WINTYPE`) and 色選択 of the question of the type
//! import (`IDC_COMBO_COLORS`) use it.
//!
//! One Tab stop (the button inside the box). With the list closed, Up, Down, Home, End, PageUp and
//! PageDown choose another entry at once, without going round; F4, Alt+Down and a click open the
//! list, and so does Space when the box itself has the focus (after a click opened the list the
//! focus is on the box, and the button inside it would take Space as a click). In the open list the
//! arrows, Home, End, PageUp and PageDown move, Enter and Space take the row, Escape, F4 and
//! Alt+Down or Alt+Up close it, and any other key that leaves the box (Tab) closes it and goes on.
//! The list is drawn over what is under the box (`deferred`), so opening it moves nothing, and it
//! scrolls when it has many rows.
//!
//! The dialog owns the choice and writes it in one place; this part owns only whether the list is
//! open, the row the keys are on in the open list, the scroll of the list and the focus of the box.
//! It tells the dialog what a key did ([`Keyed`]) and the dialog writes the choice. The list is
//! closed in one place, [`DropDown::close`]: a row was taken, Escape, F4, a key that leaves the
//! box, the focus left the box ([`DropDown::settle`], called from `render`), or the box was
//! disabled. The keys do nothing while the box is disabled (`usable` is false).

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, Context, ElementId, FocusHandle, InteractiveElement as _, IntoElement, KeyDownEvent,
    ParentElement as _, Pixels, ScrollHandle, StatefulInteractiveElement as _, Styled as _, Window,
    deferred, div, px, rgb,
};
use gpui_component::Disableable as _;
use gpui_component::button::Button;

/// The tallest the open list gets before it scrolls.
const LIST_HEIGHT: f32 = 176.;
/// The height of a row.
const ROW_HEIGHT: f32 = 22.;

/// What a key did to the drop-down, for the dialog that owns the choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Keyed {
    /// Not a key of the drop-down: nothing changed, and the key goes on.
    Ignored,
    /// The drop-down changed (it closed) and the key goes on: draw again.
    Redraw,
    /// The key was used: draw again and stop the key.
    Handled,
    /// The key chose this row (and closed the list): the dialog writes the choice, draws again and
    /// stops the key.
    Choose(usize),
}

/// The focus, the open state, the highlighted row and the scroll of one drop-down list.
pub(crate) struct DropDown {
    /// The focus of the box: it holds the focus of the button inside.
    focus: FocusHandle,
    open: bool,
    /// The row the keys are on in the open list.
    cursor: usize,
    scroll: ScrollHandle,
}

impl DropDown {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            focus: cx.focus_handle(),
            open: false,
            cursor: 0,
            scroll: ScrollHandle::new(),
        }
    }

    /// The focus of the box (the button inside it is the Tab stop).
    #[cfg(test)]
    pub(crate) fn focus(&self) -> &FocusHandle {
        &self.focus
    }

    #[cfg(test)]
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Whether the focus is in the box.
    pub(crate) fn has_focus(&self, window: &Window, cx: &App) -> bool {
        self.focus.contains_focused(window, cx)
    }

    /// Moves the focus to the box (an access key on its label): to the button inside it, the
    /// first Tab stop after the box.
    pub(crate) fn focus_box(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
        window.focus_next(cx);
    }

    /// Closes the list that is open: the one place that does it.
    pub(crate) fn close(&mut self) {
        self.open = false;
    }

    /// Closes the list when the box cannot be used any more or the focus has left it (Shift+Tab
    /// and a click elsewhere come here from `render`), as a combo box does. Returns whether it
    /// closed.
    pub(crate) fn settle(&mut self, window: &Window, cx: &App, usable: bool) -> bool {
        let closes = self.open && (!usable || !self.has_focus(window, cx));
        if closes {
            self.close();
        }
        closes
    }

    /// Opens the list on the row `selected`, or closes it. The focus goes into the box first: a
    /// click on the button does not move it, and a list opened while the focus is elsewhere would
    /// be closed again by `settle` as a list that lost the focus.
    pub(crate) fn toggle(&mut self, selected: usize, window: &mut Window, cx: &mut App) {
        if self.open {
            self.close();
            return;
        }
        self.open = true;
        // a focus that is on the button inside stays there: moved to the box, the next Tab would
        // go from the box to the button, which is still the same stop
        if !self.has_focus(window, cx) {
            window.focus(&self.focus, cx);
        }
        self.cursor = selected;
        self.scroll.scroll_to_item(selected);
    }

    /// The row that a key goes to from `from`, without going round.
    fn target(key: &str, from: usize, rows: usize) -> Option<usize> {
        let last = rows.saturating_sub(1);
        Some(match key {
            "up" => from.saturating_sub(1).min(last),
            "down" => (from + 1).min(last),
            "home" | "pageup" => 0,
            "end" | "pagedown" => last,
            _ => return None,
        })
    }

    /// A key on the box while its list is closed. `selected` is the row that is chosen.
    pub(crate) fn on_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut App,
        selected: usize,
        rows: usize,
        usable: bool,
    ) -> Keyed {
        if self.open || !usable || rows == 0 {
            return Keyed::Ignored;
        }
        let modifiers = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str();
        let own_focus = self.focus.is_focused(window);
        if key == "f4" && !modifiers.modified()
            || key == "down" && modifiers.alt
            || key == "space" && own_focus && !modifiers.modified()
        {
            self.toggle(selected, window, cx);
            return Keyed::Handled;
        }
        if modifiers.modified() {
            return Keyed::Ignored;
        }
        match Self::target(key, selected, rows) {
            Some(to) if to != selected => Keyed::Choose(to),
            Some(_) => Keyed::Handled,
            None => Keyed::Ignored,
        }
    }

    /// A key while the list is open, to be seen before the control that has the focus does (Enter
    /// would press a button, Escape would close the dialog): put it on the capture phase of the
    /// dialog's panel, before the handler of Tab.
    pub(crate) fn on_capture_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut App,
        rows: usize,
        usable: bool,
    ) -> Keyed {
        if !self.open {
            return Keyed::Ignored;
        }
        if !usable || !self.has_focus(window, cx) {
            self.close();
            return Keyed::Redraw;
        }
        let modifiers = event.keystroke.modifiers;
        let key = event.keystroke.key.as_str();
        // Alt+Down and Alt+Up close the open list, as F4 does
        if modifiers.alt && matches!(key, "down" | "up") {
            self.close();
            return Keyed::Handled;
        }
        if modifiers.modified() {
            return Keyed::Ignored;
        }
        match key {
            // Space takes the row as Enter does, whichever of the box and its button has the
            // focus (the button would take it as a click and close the list without choosing)
            "enter" | "space" => {
                self.close();
                Keyed::Choose(self.cursor.min(rows.saturating_sub(1)))
            }
            "escape" | "f4" => {
                self.close();
                Keyed::Handled
            }
            _ => match Self::target(key, self.cursor, rows) {
                Some(to) => {
                    self.cursor = to;
                    self.scroll.scroll_to_item(to);
                    Keyed::Handled
                }
                // Tab and the like leave the box: the list is closed and the key goes on
                None => {
                    self.close();
                    Keyed::Redraw
                }
            },
        }
    }
}

/// How one drop-down looks: `id` names the button, `{id}-box`, `{id}-list` and `{id}-row-{n}`.
pub(crate) struct Look {
    pub(crate) id: &'static str,
    /// The name of the list for a screen reader.
    pub(crate) name: &'static str,
    pub(crate) width: Pixels,
}

/// What the dialog does when the box is used (functions of the dialog, which owns the choice).
pub(crate) struct Hooks<T> {
    /// The button was clicked: open or close the list ([`DropDown::toggle`]).
    pub(crate) toggle: fn(&mut T, &mut Window, &mut Context<T>),
    /// A row of the open list was clicked: choose it and close the list.
    pub(crate) pick: fn(&mut T, usize, &mut Context<T>),
    /// A key on the box ([`DropDown::on_key`]).
    pub(crate) key: fn(&mut T, &KeyDownEvent, &mut Window, &mut Context<T>),
}

impl DropDown {
    /// The box: a button with the entry that is chosen, and under it the list while it is open.
    /// The list is inside the box that tracks the focus, so a click on a row keeps the focus in it.
    pub(crate) fn view<'a, T: 'static>(
        &self,
        look: &Look,
        selected: usize,
        rows: impl IntoIterator<Item = &'a str>,
        usable: bool,
        hooks: &Hooks<T>,
        cx: &mut Context<T>,
    ) -> gpui::AnyElement {
        let Hooks { toggle, pick, key } = *hooks;
        let id = look.id;
        let rows: Vec<&str> = rows.into_iter().collect();
        let text = rows.get(selected).copied().unwrap_or_default();
        div()
            .id(ElementId::from(format!("{id}-box")))
            .debug_selector(move || format!("{id}-box"))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(key))
            .relative()
            .w(look.width)
            .child(
                Button::new(id)
                    .label(format!("{text}  v"))
                    .w_full()
                    .disabled(!usable)
                    .on_click(cx.listener(move |this, _, window, cx| toggle(this, window, cx))),
            )
            .when(self.open, |combo| {
                // drawn over what is under the box (deferred, so that nothing after it covers
                // it), so that opening the list moves nothing
                combo.child(deferred(self.list(look, &rows, pick, cx)).with_priority(1))
            })
            .into_any_element()
    }

    /// The list under the box while it is open, anchored to the bottom of the box.
    fn list<T: 'static>(
        &self,
        look: &Look,
        rows: &[&str],
        pick: fn(&mut T, usize, &mut Context<T>),
        cx: &mut Context<T>,
    ) -> impl IntoElement {
        let id = look.id;
        let mut list = div()
            .id(ElementId::from(format!("{id}-list")))
            .debug_selector(move || format!("{id}-list"))
            .role(gpui::Role::ListBox)
            .aria_label(look.name)
            .absolute()
            .top_full()
            .left_0()
            .w(look.width)
            .max_h(px(LIST_HEIGHT))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .occlude()
            .border_1()
            .border_color(rgb(0xaaaaaa))
            .bg(rgb(0xffffff));
        for (row, text) in rows.iter().enumerate() {
            let at = row == self.cursor;
            list = list.child(
                crate::a11y::list_row(
                    div().id(ElementId::NamedInteger(
                        format!("{id}-row").into(),
                        row as u64,
                    )),
                    text,
                    at,
                )
                .debug_selector(move || format!("{id}-row-{row}"))
                .px_1()
                .h(px(ROW_HEIGHT))
                .flex()
                .items_center()
                .cursor_pointer()
                .when(at, |item| item.bg(rgb(0xc8dcf5)))
                .hover(|style| style.bg(rgb(0xdde8f5)))
                .on_click(cx.listener(move |this, _, _, cx| pick(this, row, cx)))
                .child((*text).to_owned()),
            );
        }
        list
    }
}

impl<T> Clone for Hooks<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Hooks<T> {}
