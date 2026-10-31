//! The access keys of the menu bar, in a whole window and in a bar of its own.

use super::*;
use gpui::{Menu, MenuItem, TestAppContext, actions};

/// The keys of the whole window: macOS has a native menu and none of this.
#[cfg(not(target_os = "macos"))]
mod window_keys {
    use gpui::{EntityInputHandler as _, Modifiers, MouseButton, TestAppContext, point};
    use gpui_component::Root;
    use matsu_commands::keymap::{Key, KeyMap, UserRow};
    use matsu_core::buffer::Document;
    use matsu_core::model::DocumentModel;

    use super::super::*;
    use crate::app::Workspace;
    use crate::document::SharedDocument;

    fn alt() -> Modifiers {
        Modifiers {
            alt: true,
            ..Modifiers::default()
        }
    }

    /// A window with Sakura's menus, the text `abc`, and `keymap` as the key map of the menus.
    pub(super) fn window(
        cx: &mut TestAppContext,
        keymap: KeyMap,
    ) -> (
        Entity<Workspace>,
        Entity<SharedDocument>,
        &mut gpui::VisualTestContext,
    ) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::app::set_key_map(cx, keymap);
            crate::app::refresh_menus(cx);
        });
        let doc = Document::from_utf8(b"abc".to_vec()).unwrap();
        let shared = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
        let made: std::rc::Rc<std::cell::RefCell<Option<Entity<Workspace>>>> = Default::default();
        let slot = made.clone();
        let document = shared.clone();
        let (_root, cx) = cx.add_window_view(move |window, cx| {
            let workspace = cx.new(|cx| Workspace::new(document, window, cx));
            *slot.borrow_mut() = Some(workspace.clone());
            Root::new(workspace, window, cx)
        });
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        let workspace = made.borrow().clone().unwrap();
        (workspace, shared, cx)
    }

    pub(super) fn bar_of(
        workspace: &Entity<Workspace>,
        cx: &gpui::VisualTestContext,
    ) -> Entity<MenuBar> {
        workspace.read_with(cx, |w, _| w.menu_bar.clone())
    }

    pub(super) fn open_menu_name(
        bar: &Entity<MenuBar>,
        cx: &gpui::VisualTestContext,
    ) -> Option<String> {
        bar.read_with(cx, |bar, cx| {
            let ix = bar.selected_index?;
            bar.menu_names(cx).get(ix).cloned()
        })
    }

    pub(super) fn armed(bar: &Entity<MenuBar>, cx: &gpui::VisualTestContext) -> Option<usize> {
        bar.read_with(cx, |bar, _| bar.armed())
    }

    pub(super) fn text(shared: &Entity<SharedDocument>, cx: &gpui::VisualTestContext) -> String {
        shared.read_with(cx, |s, _| s.doc().line(0).unwrap().unwrap().text)
    }

    pub(super) fn tap_alt(cx: &mut gpui::VisualTestContext) {
        cx.simulate_modifiers_change(alt());
        cx.simulate_modifiers_change(Modifiers::default());
    }

    #[gpui::test]
    fn alt_and_a_letter_open_the_menu_with_that_access_key(cx: &mut TestAppContext) {
        let (workspace, shared, cx) = window(cx, KeyMap::default());
        let bar = bar_of(&workspace, cx);
        let names = bar.read_with(cx, |bar, cx| bar.menu_names(cx));
        assert!(names.contains(&"ツール(T)".to_owned()), "{names:?}");
        cx.simulate_keystrokes("alt-t");
        assert_eq!(open_menu_name(&bar, cx).as_deref(), Some("ツール(T)"));
        // The letter did not go into the text.
        assert_eq!(text(&shared, cx), "abc");
        // Another menu opens in place of the open one.
        cx.simulate_keystrokes("alt-f");
        assert_eq!(open_menu_name(&bar, cx).as_deref(), Some("ファイル(F)"));
        cx.simulate_keystrokes("escape");
        assert_eq!(open_menu_name(&bar, cx), None);
    }

    #[gpui::test]
    pub(super) fn a_command_on_alt_and_a_letter_wins_over_the_menu(cx: &mut TestAppContext) {
        // Alt+T runs a function in this key map, as Alt+L does in Sakura's.
        let mut keymap = KeyMap::default();
        keymap.apply_user_rows(&[UserRow {
            key: Key::Letter('T'),
            functions: [0, 0, 0, 0, 30280, 0, 0, 0],
        }]);
        let (workspace, _shared, cx) = window(cx, keymap);
        let bar = bar_of(&workspace, cx);
        cx.simulate_keystrokes("alt-t");
        assert_eq!(open_menu_name(&bar, cx), None);
        // A letter that is free still opens its menu.
        cx.simulate_keystrokes("alt-e");
        assert_eq!(open_menu_name(&bar, cx).as_deref(), Some("編集(E)"));
    }

    #[gpui::test]
    pub(super) fn a_letter_in_the_open_menu_runs_the_item_with_that_access_key(
        cx: &mut TestAppContext,
    ) {
        let (workspace, shared, cx) = window(cx, KeyMap::default());
        let bar = bar_of(&workspace, cx);
        cx.simulate_input("x");
        assert_eq!(text(&shared, cx), "xabc");
        cx.simulate_keystrokes("alt-e");
        assert_eq!(open_menu_name(&bar, cx).as_deref(), Some("編集(E)"));
        // 元に戻す(U)
        cx.simulate_keystrokes("u");
        assert_eq!(open_menu_name(&bar, cx), None, "the menu closes");
        assert_eq!(text(&shared, cx), "abc");
        // The focus is back in the text.
        cx.simulate_input("y");
        assert_eq!(text(&shared, cx), "yabc");
        // Upper case works too. R is shared by the gray やり直し and another item: nothing runs and
        // the menu stays.
        cx.simulate_keystrokes("alt-e shift-r");
        assert_eq!(open_menu_name(&bar, cx).as_deref(), Some("編集(E)"));
        assert_eq!(text(&shared, cx), "yabc");
    }

    #[gpui::test]
    pub(super) fn a_letter_that_no_item_has_does_nothing(cx: &mut TestAppContext) {
        let (workspace, shared, cx) = window(cx, KeyMap::default());
        let bar = bar_of(&workspace, cx);
        cx.simulate_keystrokes("alt-e 9");
        assert_eq!(open_menu_name(&bar, cx).as_deref(), Some("編集(E)"));
        assert_eq!(text(&shared, cx), "abc");
    }

    #[gpui::test]
    pub(super) fn a_tap_of_alt_marks_the_bar_and_the_arrows_move_the_mark(cx: &mut TestAppContext) {
        let (workspace, shared, cx) = window(cx, KeyMap::default());
        let bar = bar_of(&workspace, cx);
        cx.simulate_modifiers_change(alt());
        assert_eq!(armed(&bar, cx), None, "not before Alt is released");
        cx.simulate_modifiers_change(Modifiers::default());
        assert_eq!(armed(&bar, cx), Some(0));
        assert_eq!(open_menu_name(&bar, cx), None, "marked, not open");
        cx.simulate_keystrokes("right");
        assert_eq!(armed(&bar, cx), Some(1));
        cx.simulate_keystrokes("left left");
        let count = bar.read_with(cx, |b, _| b.menus.len());
        assert_eq!(armed(&bar, cx), Some(count - 1));
        // The keys did not go to the text.
        assert_eq!(text(&shared, cx), "abc");
    }

    #[gpui::test]
    pub(super) fn down_opens_the_marked_menu_and_a_letter_opens_another(cx: &mut TestAppContext) {
        let (workspace, _shared, cx) = window(cx, KeyMap::default());
        let bar = bar_of(&workspace, cx);
        tap_alt(cx);
        cx.simulate_keystrokes("right down");
        assert_eq!(armed(&bar, cx), None);
        assert_eq!(open_menu_name(&bar, cx).as_deref(), Some("編集(E)"));
        cx.simulate_keystrokes("escape");
        assert_eq!(open_menu_name(&bar, cx), None);
        tap_alt(cx);
        cx.simulate_keystrokes("t");
        assert_eq!(open_menu_name(&bar, cx).as_deref(), Some("ツール(T)"));
    }

    #[gpui::test]
    pub(super) fn escape_another_alt_or_another_key_ends_the_mark(cx: &mut TestAppContext) {
        let (workspace, shared, cx) = window(cx, KeyMap::default());
        let bar = bar_of(&workspace, cx);
        tap_alt(cx);
        cx.simulate_keystrokes("escape");
        assert_eq!(armed(&bar, cx), None);
        tap_alt(cx);
        tap_alt(cx);
        assert_eq!(armed(&bar, cx), None, "Alt again");
        // Another key ends the mark and goes to the text.
        tap_alt(cx);
        cx.simulate_input("z");
        assert_eq!(armed(&bar, cx), None);
        assert_eq!(text(&shared, cx), "zabc");
    }

    #[gpui::test]
    pub(super) fn alt_used_with_a_key_or_the_mouse_does_not_mark_the_bar(cx: &mut TestAppContext) {
        let (workspace, _shared, cx) = window(cx, KeyMap::default());
        let bar = bar_of(&workspace, cx);
        // Alt+key (Alt+digits of a key map, Alt+Tab of the system).
        cx.simulate_modifiers_change(alt());
        cx.simulate_keystrokes("alt-5");
        cx.simulate_modifiers_change(Modifiers::default());
        assert_eq!(armed(&bar, cx), None);
        // Alt+drag.
        cx.simulate_modifiers_change(alt());
        cx.simulate_mouse_down(
            point(gpui::px(300.), gpui::px(300.)),
            MouseButton::Left,
            alt(),
        );
        cx.simulate_modifiers_change(Modifiers::default());
        assert_eq!(armed(&bar, cx), None);
        // Alt with Shift (the layout switch).
        cx.simulate_modifiers_change(alt());
        cx.simulate_modifiers_change(Modifiers {
            alt: true,
            shift: true,
            ..Modifiers::default()
        });
        cx.simulate_modifiers_change(Modifiers::default());
        assert_eq!(armed(&bar, cx), None);
    }

    #[gpui::test]
    pub(super) fn nothing_happens_while_the_ime_composes(cx: &mut TestAppContext) {
        let (workspace, _shared, cx) = window(cx, KeyMap::default());
        let bar = bar_of(&workspace, cx);
        let editor = workspace.read_with(cx, |w, _| w.editor().clone());
        editor.update_in(cx, |view, window, cx| {
            view.replace_and_mark_text_in_range(None, "か", None, window, cx);
        });
        cx.simulate_keystrokes("alt-t");
        assert_eq!(open_menu_name(&bar, cx), None);
        tap_alt(cx);
        assert_eq!(armed(&bar, cx), None);
    }
}

// ---- a bar of its own, with items that share an access key ----

actions!(access_key_tests, [First, Second, Third]);

struct Probe {
    menu_bar: Entity<MenuBar>,
    focus: FocusHandle,
    ran: Vec<&'static str>,
}

impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(
                div()
                    .id("first")
                    .track_focus(&self.focus)
                    .on_action(cx.listener(|this, _: &First, _, _| this.ran.push("first")))
                    .on_action(cx.listener(|this, _: &Second, _, _| this.ran.push("second")))
                    .on_action(cx.listener(|this, _: &Third, _, _| this.ran.push("third"))),
            )
            .child(div().h(px(30.)).child(self.menu_bar.clone()))
    }
}

/// A window whose bar has one open menu: A, a separator, B and another A.
fn probe(cx: &mut TestAppContext) -> (Entity<Probe>, &mut gpui::VisualTestContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        let menu = Menu::new("メニュー(M)").items([
            MenuItem::action("一つ目(A)", First),
            MenuItem::separator(),
            MenuItem::action("二つ目(B)", Second),
            MenuItem::action("三つ目(a)", Third),
        ]);
        GlobalState::global_mut(cx).set_app_menus(vec![menu.owned()]);
    });
    let (probe, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        Probe {
            menu_bar: MenuBar::new(cx),
            focus,
            ran: Vec::new(),
        }
    });
    let bar = probe.read_with(cx, |p, _| p.menu_bar.clone());
    bar.update_in(cx, |bar, window, cx| {
        bar.set_selected_index(Some(0), window, cx);
    });
    cx.run_until_parked();
    (probe, cx)
}

fn ran(probe: &Entity<Probe>, cx: &gpui::VisualTestContext) -> Vec<&'static str> {
    probe.read_with(cx, |p, _| p.ran.clone())
}

#[gpui::test]
fn a_key_that_one_item_has_runs_it(cx: &mut TestAppContext) {
    let (probe, cx) = probe(cx);
    cx.simulate_keystrokes("b");
    assert_eq!(ran(&probe, cx), ["second"]);
    let bar = probe.read_with(cx, |p, _| p.menu_bar.clone());
    assert_eq!(bar.read_with(cx, |b, _| b.selected_index), None);
}

#[gpui::test]
fn a_key_that_several_items_have_selects_the_first_and_does_not_run_it(cx: &mut TestAppContext) {
    let (probe, cx) = probe(cx);
    cx.simulate_keystrokes("a");
    assert!(ran(&probe, cx).is_empty());
    cx.simulate_keystrokes("enter");
    assert_eq!(ran(&probe, cx), ["first"]);
}

#[gpui::test]
fn the_same_key_again_selects_the_next_item_and_goes_round(cx: &mut TestAppContext) {
    let (probe, cx) = probe(cx);
    cx.simulate_keystrokes("a a");
    assert!(ran(&probe, cx).is_empty());
    cx.simulate_keystrokes("enter");
    assert_eq!(ran(&probe, cx), ["third"]);

    let (probe, cx) = self::probe(cx);
    cx.simulate_keystrokes("shift-a a shift-a enter");
    assert_eq!(ran(&probe, cx), ["first"]);
}

#[gpui::test]
fn the_keys_move_on_from_where_the_arrows_left_the_selection(cx: &mut TestAppContext) {
    let (probe, cx) = probe(cx);
    // Down selects the first item, which has the key already: the next one is the other A.
    cx.simulate_keystrokes("down a enter");
    assert_eq!(ran(&probe, cx), ["third"]);
}
