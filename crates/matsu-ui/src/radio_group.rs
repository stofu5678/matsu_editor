//! Radio buttons that act as one group, like the automatic radio buttons of a Windows dialog.
//!
//! In Sakura's dialogs the buttons of a group are `BS_AUTORADIOBUTTON` controls, and the first of
//! the group has `WS_GROUP | WS_TABSTOP` (`sakura_rc.rc`, for example `IDD_JUMP` lines 93-94).
//! Windows' dialog manager (`IsDialogMessage`, "Dialog Box Keyboard Interface") then does this:
//!
//! - Tab and Shift+Tab stop on the group once, at the checked button (at the first one while none
//!   is checked), and go on to the next tab stop outside the group.
//! - The arrow keys (Up and Left: the previous button, Down and Right: the next one) move the
//!   focus inside the group and check the button they move to. The move wraps round: the next one
//!   after the last button is the first (`GetNextDlgGroupItem`: "If hCtl is the last control in
//!   the group and bPrevious is FALSE, ... returns the first control in the group"). Buttons that
//!   cannot be used are skipped.
//!
//! gpui-component's `Radio` is a tab stop each and ignores the arrow keys, and its `RadioGroup`
//! only lays the buttons out. [`Group`] adds the two rules to the plain `Radio`:
//!
//! ```ignore
//! let group = Group::new(&self.ring, 3, Some(selected), select);
//! group.radio(0, "id-0", true, |radio| radio.labelled("..."))
//! ```
//!
//! Who owns what: the dialog owns which button is checked (its own state, as before, which also
//! tells the group through `selected`), GPUI owns which button has the focus, and the [`Ring`]
//! only keeps the focus handles of the group's buttons so that an arrow key can move the focus
//! to a button other than the one that got the key.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    App, ElementId, FocusHandle, InteractiveElement as _, IntoElement, KeyDownEvent, MouseButton,
    RenderOnce, Window,
};
use gpui_component::radio::Radio;

use crate::a11y::AriaDisabled as _;

/// What a group does to choose the button with this index: the arrow keys and a click on a button
/// both call it. Made with `cx.listener` of the dialog; it is the one place that writes the choice.
pub(crate) type Select = Rc<dyn Fn(&usize, &mut Window, &mut App)>;

/// The focus handles of the buttons of one group, by position.
///
/// A `Rc<RefCell<..>>` because the key handler of a button (a `'static` closure of GPUI) reads
/// the handles of the other buttons, and no `Entity` is at hand there. Every button writes its
/// own slot each time it is drawn, so a handle is as old as the last drawing of its button; the
/// handle itself stays the same while the button exists (it is GPUI's keyed state of the
/// button's id). A button that is gone is not read: `step` is bounded by the group's size, and
/// the slots beyond the size are dropped whenever a button is drawn (`register`).
#[derive(Clone, Default)]
pub(crate) struct Ring(Rc<RefCell<Vec<Slot>>>);

#[derive(Clone, Default)]
struct Slot {
    focus: Option<FocusHandle>,
    enabled: bool,
}

impl Ring {
    fn register(&self, index: usize, size: usize, focus: FocusHandle, enabled: bool) {
        let mut slots = self.0.borrow_mut();
        // the buttons that the group no longer has (a page that shows fewer) are forgotten
        slots.truncate(size);
        if slots.len() <= index {
            slots.resize(index + 1, Slot::default());
        }
        slots[index] = Slot {
            focus: Some(focus),
            enabled,
        };
    }

    /// The button the arrow key `forward` or back moves to from `from`, wrapping round, among the
    /// `size` buttons; the buttons that cannot be used (or that have not been drawn) are skipped.
    /// The button itself when it is the only one.
    fn step(&self, from: usize, size: usize, forward: bool) -> usize {
        let slots = self.0.borrow();
        let usable = |index: usize| {
            slots
                .get(index)
                .is_some_and(|slot| slot.enabled && slot.focus.is_some())
        };
        (1..size)
            .map(|distance| {
                if forward {
                    (from + distance) % size
                } else {
                    (from + size - distance) % size
                }
            })
            .find(|&index| usable(index))
            .unwrap_or(from)
    }

    /// Moves the keyboard focus to the button at `index`, as Windows does when an access key
    /// chooses a radio button (also one that is already checked). The caller chooses the button
    /// first, through the group's one writer (`select`); this only moves the focus. An access key
    /// is used only for a button that can be used, so a button that cannot be used, or that
    /// is not drawn, is a mistake of the caller: a debug build stops, a release build does not
    /// move the focus.
    pub(crate) fn focus(&self, index: usize, window: &mut Window, cx: &mut App) {
        let focus = self.usable_focus(index);
        debug_assert!(
            focus.is_some(),
            "radio_group: the access key chose button {index}, which cannot take the focus"
        );
        if let Some(focus) = focus {
            window.focus(&focus, cx);
        }
    }

    /// The same for a mouse down, which also reaches a button that cannot be used: it does
    /// nothing there.
    fn focus_if_usable(&self, index: usize, window: &mut Window, cx: &mut App) {
        if let Some(focus) = self.usable_focus(index) {
            window.focus(&focus, cx);
        }
    }

    /// The focus handle of the button at `index`, when the group drew it as it is now (the slots
    /// beyond the group's size are dropped whenever a button is drawn) and it can be used.
    fn usable_focus(&self, index: usize) -> Option<FocusHandle> {
        let slots = self.0.borrow();
        let slot = slots.get(index)?;
        slot.enabled.then(|| slot.focus.clone()).flatten()
    }

    fn focus_of(&self, index: usize) -> Option<FocusHandle> {
        self.0
            .borrow()
            .get(index)
            .and_then(|slot| slot.focus.clone())
    }

    /// Which button of the group has the keyboard focus (what the dialogs' tests look at).
    #[cfg(test)]
    pub(crate) fn focused(&self, window: &Window) -> Option<usize> {
        self.0.borrow().iter().position(|slot| {
            slot.focus
                .as_ref()
                .is_some_and(|focus| focus.is_focused(window))
        })
    }
}

/// One group of radio buttons, as the dialog describes it each time it draws.
#[derive(Clone)]
pub(crate) struct Group {
    ring: Ring,
    size: usize,
    selected: Option<usize>,
    select: Select,
}

impl Group {
    /// A group of `size` buttons of which the one at `selected` is checked. `select` chooses
    /// the button at an index.
    pub(crate) fn new(ring: &Ring, size: usize, selected: Option<usize>, select: Select) -> Self {
        Self {
            ring: ring.clone(),
            size,
            selected,
            select,
        }
    }

    /// The button at `index` (from 0, in the order of the group) with the id `id`. `build` makes
    /// the plain `Radio` of it (label, name, checked state); it must not change the button's
    /// `disabled` state (`enabled` does), its tab stop or its click (the group's `select` is it).
    pub(crate) fn radio(
        &self,
        index: usize,
        id: impl Into<ElementId>,
        enabled: bool,
        build: impl FnOnce(Radio) -> Radio,
    ) -> GroupRadio {
        let id = id.into();
        GroupRadio {
            radio: build(Radio::new(id.clone())),
            id,
            group: self.clone(),
            index,
            enabled,
        }
    }
}

/// A radio button of a [`Group`].
#[derive(IntoElement)]
pub(crate) struct GroupRadio {
    radio: Radio,
    id: ElementId,
    group: Group,
    index: usize,
    enabled: bool,
}

impl RenderOnce for GroupRadio {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // The focus handle that `Radio` keeps for itself. GPUI keys such state by the ids of
        // the elements above it, and a component adds its type name to them while it is
        // rendered (`request_layout_component`): `Radio` asks for it from inside its own name,
        // with the id of the button as the key. Asking from inside that name with the same key
        // gives the same handle (this button is drawn where the `Radio` would be). A test
        // (`arrow_keys_move_the_focus_and_check_the_button`) fails when gpui-component or GPUI
        // stops doing it.
        let focus = window.with_id(std::any::type_name::<Radio>(), |window| {
            window
                .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
                .read(cx)
                .clone()
        });
        let Group {
            ring,
            size,
            selected,
            select,
        } = self.group;
        ring.register(self.index, size, focus, self.enabled);
        // Tab enters the group at the checked button, or at the first one when none is checked.
        let tab_stop = match selected {
            Some(selected) => selected == self.index,
            None => self.index == 0,
        };
        let index = self.index;
        let clicked = select.clone();
        let pressed_ring = ring.clone();
        self.radio
            .disabled(!self.enabled)
            // UI Automation says it is disabled too (as the gray check boxes, #528)
            .aria_disabled(!self.enabled)
            .tab_stop(tab_stop)
            // A click, Space or Enter chooses the button by the same function as the arrow keys,
            // so that no dialog can write the two out of step.
            .on_click(move |_, window, cx| clicked(&index, window, cx))
            // The button takes the focus when the mouse goes down on it, also when it is already
            // checked (Windows focuses a radio button that is clicked; gpui-component's `Radio`
            // calls `prevent_default` on a mouse down, which stops GPUI's own focusing).
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                pressed_ring.focus_if_usable(index, window, cx);
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                let Some(forward) = direction(event) else {
                    return;
                };
                cx.stop_propagation();
                // This handler runs because this button has the focus. When the handle in the
                // ring is not the one the `Radio` tracks (gpui-component or GPUI changed how it
                // keeps it), the arrow keys would silently stop working: say so in a debug build.
                debug_assert!(
                    ring.focus_of(index)
                        .is_some_and(|focus| focus.is_focused(window)),
                    "radio_group: the focus handle of the ring is not the one of the Radio"
                );
                let to = ring.step(index, size, forward);
                if to == index {
                    return;
                }
                select(&to, window, cx);
                if let Some(focus) = ring.focus_of(to) {
                    window.focus(&focus, cx);
                }
            })
    }
}

/// Down and Right are forward, Up and Left backward; a key with a modifier is not an arrow key
/// of the dialog manager.
fn direction(event: &KeyDownEvent) -> Option<bool> {
    if event.keystroke.modifiers.modified() {
        return None;
    }
    match event.keystroke.key.as_str() {
        "down" | "right" => Some(true),
        "up" | "left" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests;

/// What the tests of the dialogs share.
#[cfg(test)]
pub(crate) mod testing {
    use gpui::VisualTestContext;

    use super::Ring;

    /// Where the focus is among the buttons of `rings`: the group and the button, or `None` while
    /// it is somewhere else.
    pub(crate) fn at(keys: &mut VisualTestContext, rings: &[&Ring]) -> Option<(usize, usize)> {
        keys.update(|window, _| {
            rings
                .iter()
                .enumerate()
                .find_map(|(group, ring)| ring.focused(window).map(|button| (group, button)))
        })
    }

    /// Presses `key` (Tab or Shift+Tab) `presses` times and says where the focus is after each
    /// press ([`at`]).
    pub(crate) fn trail(
        keys: &mut VisualTestContext,
        rings: &[&Ring],
        key: &str,
        presses: usize,
    ) -> Vec<Option<(usize, usize)>> {
        (0..presses)
            .map(|_| {
                keys.simulate_keystrokes(key);
                at(keys, rings)
            })
            .collect()
    }

    /// Presses Tab (at most `limit` times, so that a failure ends the test) until the focus is on
    /// a button of the group `group` of `rings`, and says which button it is on.
    pub(crate) fn tab_to(
        keys: &mut VisualTestContext,
        rings: &[&Ring],
        group: usize,
        limit: usize,
    ) -> usize {
        for _ in 0..limit {
            if let Some((found, button)) = at(keys, rings)
                && found == group
            {
                return button;
            }
            keys.simulate_keystrokes("tab");
        }
        panic!("Tab did not reach group {group} in {limit} presses");
    }

    /// Tab stops on every group of `checked` (the checked button of each, in the order of the
    /// rings), and only at the checked button: no stop of the trail is on another button.
    pub(crate) fn assert_stops_at_checked(trail: &[Option<(usize, usize)>], checked: &[usize]) {
        for stop in trail.iter().flatten() {
            assert_eq!(
                checked.get(stop.0),
                Some(&stop.1),
                "Tab stopped on button {} of group {}, which is not the checked one: {trail:?}",
                stop.1,
                stop.0
            );
        }
        for group in 0..checked.len() {
            assert!(
                trail.iter().flatten().any(|stop| stop.0 == group),
                "Tab never reached group {group}: {trail:?}"
            );
        }
    }
}
