//! The group rules (Tab stops once at the checked button, the arrow keys move and check, wrap
//! round and skip the buttons that cannot be used) in a toy window: a tab stop, a group of four
//! radio buttons (the third cannot be used) and another tab stop.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Context, FocusHandle, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    Styled as _, TestAppContext, VisualTestContext, Window, div,
};

use super::{Group, Ring};
use crate::dialog_focus::KeepTabInside as _;

struct Toy {
    selected: Rc<Cell<Option<usize>>>,
    ring: Ring,
    panel: FocusHandle,
    before: FocusHandle,
    after: FocusHandle,
}

impl Render for Toy {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let chosen = self.selected.clone();
        let select = Rc::new(cx.listener(move |_, index: &usize, _, cx| {
            chosen.set(Some(*index));
            cx.notify();
        }));
        let group = Group::new(&self.ring, 4, self.selected.get(), select);
        div()
            .id("panel")
            .track_focus(&self.panel)
            .keep_tab_inside(&self.panel)
            .child(div().id("before").track_focus(&self.before).size_4())
            .child(div().flex().flex_col().children((0..4).map(|index| {
                let on = self.selected.clone();
                group.radio(index, ("radio", index), index != 2, move |radio| {
                    radio.label("radio").checked(on.get() == Some(index))
                })
            })))
            .child(div().id("after").track_focus(&self.after).size_4())
    }
}

fn toy(
    cx: &mut TestAppContext,
    selected: Option<usize>,
) -> (gpui::Entity<Toy>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    let (toy, keys) = cx.add_window_view(move |_, cx| Toy {
        selected: Rc::new(Cell::new(selected)),
        ring: Ring::default(),
        panel: cx.focus_handle(),
        before: cx.focus_handle().tab_stop(true),
        after: cx.focus_handle().tab_stop(true),
    });
    keys.update(|window, cx| window.draw(cx).clear(cx));
    (toy, keys)
}

/// Where the focus is: 0 for the first tab stop, 1 to 4 for the buttons, 5 for the last tab stop.
fn at(toy: &gpui::Entity<Toy>, keys: &mut VisualTestContext) -> Option<usize> {
    let (before, ring, after) = toy.read_with(keys, |t, _| {
        (t.before.clone(), t.ring.clone(), t.after.clone())
    });
    keys.update(|window, _| {
        if before.is_focused(window) {
            return Some(0);
        }
        if after.is_focused(window) {
            return Some(5);
        }
        (0..4).find_map(|i| {
            ring.focus_of(i)
                .filter(|focus| focus.is_focused(window))
                .map(|_| i + 1)
        })
    })
}

fn focus_before(toy: &gpui::Entity<Toy>, keys: &mut VisualTestContext) {
    let before = toy.read_with(keys, |t, _| t.before.clone());
    keys.update(|window, cx| before.focus(window, cx));
}

fn selected(toy: &gpui::Entity<Toy>, keys: &mut VisualTestContext) -> Option<usize> {
    toy.read_with(keys, |t, _| t.selected.get())
}

#[gpui::test]
fn tab_stops_on_the_group_once_at_the_checked_button(cx: &mut TestAppContext) {
    let (toy, keys) = toy(cx, Some(1));
    focus_before(&toy, keys);
    keys.simulate_keystrokes("tab");
    assert_eq!(at(&toy, keys), Some(2), "the checked button, the second");
    keys.simulate_keystrokes("tab");
    assert_eq!(
        at(&toy, keys),
        Some(5),
        "the next tab stop, outside the group"
    );
    keys.simulate_keystrokes("shift-tab");
    assert_eq!(
        at(&toy, keys),
        Some(2),
        "Shift+Tab comes back to the checked one too"
    );
    keys.simulate_keystrokes("shift-tab");
    assert_eq!(at(&toy, keys), Some(0));
    assert_eq!(selected(&toy, keys), Some(1), "Tab chooses nothing");
}

#[gpui::test]
fn tab_enters_at_the_first_button_while_none_is_checked(cx: &mut TestAppContext) {
    let (toy, keys) = toy(cx, None);
    focus_before(&toy, keys);
    keys.simulate_keystrokes("tab");
    assert_eq!(at(&toy, keys), Some(1));
    keys.simulate_keystrokes("tab");
    assert_eq!(at(&toy, keys), Some(5));
}

#[gpui::test]
fn arrow_keys_move_the_focus_and_check_the_button(cx: &mut TestAppContext) {
    let (toy, keys) = toy(cx, Some(0));
    focus_before(&toy, keys);
    keys.simulate_keystrokes("tab");
    assert_eq!(at(&toy, keys), Some(1));
    keys.simulate_keystrokes("down");
    assert_eq!((at(&toy, keys), selected(&toy, keys)), (Some(2), Some(1)));
    // the third one cannot be used: skipped
    keys.simulate_keystrokes("right");
    assert_eq!((at(&toy, keys), selected(&toy, keys)), (Some(4), Some(3)));
    // wraps round from the last to the first
    keys.simulate_keystrokes("down");
    assert_eq!((at(&toy, keys), selected(&toy, keys)), (Some(1), Some(0)));
    // and from the first to the last
    keys.simulate_keystrokes("up");
    assert_eq!((at(&toy, keys), selected(&toy, keys)), (Some(4), Some(3)));
    keys.simulate_keystrokes("left");
    assert_eq!((at(&toy, keys), selected(&toy, keys)), (Some(2), Some(1)));
    // Tab now leaves from the button that was chosen
    keys.simulate_keystrokes("tab");
    assert_eq!(at(&toy, keys), Some(5));
    keys.simulate_keystrokes("shift-tab");
    assert_eq!(at(&toy, keys), Some(2));
}

#[gpui::test]
fn an_arrow_key_with_a_modifier_does_nothing(cx: &mut TestAppContext) {
    let (toy, keys) = toy(cx, Some(0));
    focus_before(&toy, keys);
    keys.simulate_keystrokes("tab");
    for key in ["ctrl-down", "alt-down", "shift-down"] {
        keys.simulate_keystrokes(key);
        assert_eq!(
            (at(&toy, keys), selected(&toy, keys)),
            (Some(1), Some(0)),
            "{key}"
        );
    }
}
