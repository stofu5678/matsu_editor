//! Tab and Shift+Tab stay among the controls of a modal dialog.
//!
//! A modal dialog of Windows runs its own message loop with `IsDialogMessage`: Tab and Shift+Tab
//! move among the controls of the dialog, from the last to the first and the other way round,
//! and the window behind the dialog gets nothing. Our dialogs are drawn inside the main window,
//! where GPUI's own Tab handling walks all the tab stops of the window, the text behind the
//! dialog and its menu bar too. [`keep_tab_inside`] puts a dialog's panel in charge of the
//! key: the order of the tab stops stays GPUI's (the order the controls are drawn in, which is
//! Sakura's tab order), and only the end of the dialog wraps round.
//!
//! A dialog that is its own OS window (the common settings, the type settings, the font
//! dialog) cannot leak and does not use this.

use gpui::{App, FocusHandle, InteractiveElement, Window};

/// The most tab stops [`tab_inside`] walks to find the last one of a dialog.
const MAX_STOPS: usize = 512;

/// `.keep_tab_inside(&focus)` on the panel of a dialog (after `.track_focus(&focus)`): Tab and
/// Shift+Tab move the focus among the tab stops inside `scope` only. Ctrl+Tab and Alt+Tab are
/// left to the dialog.
pub(crate) trait KeepTabInside: InteractiveElement + Sized {
    fn keep_tab_inside(self, scope: &FocusHandle) -> Self {
        let scope = scope.clone();
        self.capture_key_down(move |event, window, cx| {
            let modifiers = event.keystroke.modifiers;
            if event.keystroke.key == "tab"
                && !modifiers.control
                && !modifiers.alt
                && !modifiers.platform
            {
                tab_inside(&scope, modifiers.shift, window, cx);
                cx.stop_propagation();
            }
        })
    }
}

impl<E: InteractiveElement> KeepTabInside for E {}

/// Tab (`back` false) or Shift+Tab: the next (previous) tab stop; round the end of `scope` to its
/// first (last) one.
pub(crate) fn tab_inside(scope: &FocusHandle, back: bool, window: &mut Window, cx: &mut App) {
    if back {
        window.focus_prev(cx);
    } else {
        window.focus_next(cx);
    }
    if scope.contains_focused(window, cx) {
        return;
    }
    // Left the dialog: the first stop is the one after the panel itself, which comes before its
    // children in the order.
    scope.focus(window, cx);
    window.focus_next(cx);
    if !scope.contains_focused(window, cx) {
        // nothing in the dialog takes the focus (a button that is disabled meanwhile)
        scope.focus(window, cx);
        return;
    }
    if !back {
        return;
    }
    // The last stop: walk forward until the next one is outside.
    let mut last = window.focused(cx);
    for _ in 0..MAX_STOPS {
        window.focus_next(cx);
        if !scope.contains_focused(window, cx) {
            break;
        }
        let at = window.focused(cx);
        if at == last {
            break;
        }
        last = at;
    }
    match last {
        Some(last) => last.focus(window, cx),
        None => scope.focus(window, cx),
    }
}

/// What the tests of the dialogs share: Tab and Shift+Tab, many times, never leave `scope`.
#[cfg(test)]
pub(crate) fn assert_tab_stays_inside(keys: &mut gpui::VisualTestContext, scope: &FocusHandle) {
    let inside = |keys: &mut gpui::VisualTestContext| {
        keys.update(|window, cx| scope.contains_focused(window, cx))
    };
    // the dialog has the focus when it opens
    if !inside(keys) {
        keys.update(|window, cx| scope.focus(window, cx));
    }
    for n in 0..40 {
        keys.simulate_keystrokes("tab");
        assert!(inside(keys), "Tab left the dialog (press {n})");
    }
    for n in 0..40 {
        keys.simulate_keystrokes("shift-tab");
        assert!(inside(keys), "Shift+Tab left the dialog (press {n})");
    }
}

#[cfg(test)]
mod tests {
    use gpui::{
        Context, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext, Window, div,
    };

    use super::*;

    /// A tab stop before the dialog, three in it, and one after it.
    struct Toy {
        trapped: bool,
        before: FocusHandle,
        panel: FocusHandle,
        inside: [FocusHandle; 3],
        after: FocusHandle,
    }

    impl Toy {
        fn new(trapped: bool, inside_stops: bool, cx: &mut Context<Self>) -> Self {
            let stop = |cx: &mut Context<Self>| cx.focus_handle().tab_stop(true);
            let inside = |cx: &mut Context<Self>| cx.focus_handle().tab_stop(inside_stops);
            Self {
                trapped,
                before: stop(cx),
                panel: cx.focus_handle(),
                inside: [inside(cx), inside(cx), inside(cx)],
                after: stop(cx),
            }
        }
    }

    impl Render for Toy {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let mut panel = div().id("panel").track_focus(&self.panel);
            if self.trapped {
                panel = panel.keep_tab_inside(&self.panel);
            }
            div()
                .size_full()
                .child(div().id("before").track_focus(&self.before).size_4())
                .child(
                    panel.children(
                        self.inside
                            .iter()
                            .enumerate()
                            .map(|(i, h)| div().id(("inside", i)).track_focus(h).size_4()),
                    ),
                )
                .child(div().id("after").track_focus(&self.after).size_4())
        }
    }

    fn toy(
        cx: &mut TestAppContext,
        trapped: bool,
        inside_stops: bool,
    ) -> (gpui::VisualTestContext, gpui::Entity<Toy>) {
        let (toy, keys) = cx.add_window_view(move |_, cx| Toy::new(trapped, inside_stops, cx));
        (keys.clone(), toy)
    }

    fn focused_index(keys: &mut gpui::VisualTestContext, toy: &gpui::Entity<Toy>) -> usize {
        let all = toy.read_with(keys, |t, _| {
            [
                t.before.clone(),
                t.inside[0].clone(),
                t.inside[1].clone(),
                t.inside[2].clone(),
                t.after.clone(),
            ]
        });
        keys.update(|window, _| all.iter().position(|h| h.is_focused(window)).unwrap())
    }

    #[gpui::test]
    fn tab_wraps_round_the_ends_of_the_dialog(cx: &mut TestAppContext) {
        let (mut keys, toy) = toy(cx, true, true);
        let first = toy.read_with(&keys, |t, _| t.inside[0].clone());
        keys.update(|window, cx| first.focus(window, cx));
        assert_eq!(focused_index(&mut keys, &toy), 1);
        let mut seen = Vec::new();
        for _ in 0..6 {
            keys.simulate_keystrokes("tab");
            seen.push(focused_index(&mut keys, &toy));
        }
        assert_eq!(seen, [2, 3, 1, 2, 3, 1]);
        let mut seen = Vec::new();
        for _ in 0..5 {
            keys.simulate_keystrokes("shift-tab");
            seen.push(focused_index(&mut keys, &toy));
        }
        assert_eq!(seen, [3, 2, 1, 3, 2]);
    }

    #[gpui::test]
    fn gpuis_own_focus_next_leaves_the_dialog(cx: &mut TestAppContext) {
        // what the root of the window does for Tab (`Root::on_action_tab`): the test above would
        // pass in a window that has nothing outside; this shows there is something outside
        let (mut keys, toy) = toy(cx, false, true);
        let last = toy.read_with(&keys, |t, _| t.inside[2].clone());
        keys.update(|window, cx| {
            last.focus(window, cx);
        });
        keys.run_until_parked();
        keys.update(|window, cx| window.focus_next(cx));
        assert_eq!(focused_index(&mut keys, &toy), 4);
    }

    #[gpui::test]
    fn a_dialog_without_a_tab_stop_keeps_the_focus_on_itself(cx: &mut TestAppContext) {
        let (mut keys, toy) = toy(cx, true, false);
        let panel = toy.read_with(&keys, |t, _| t.panel.clone());
        keys.update(|window, cx| panel.focus(window, cx));
        for key in ["tab", "shift-tab"] {
            keys.simulate_keystrokes(key);
            assert!(keys.update(|window, cx| panel.contains_focused(window, cx)));
        }
    }
}
