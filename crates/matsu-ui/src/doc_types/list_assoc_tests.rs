//! The association check boxes of the type list, with a fake in place of the registry (the real
//! one is Windows only). Nothing here touches the registry or the file system's paths.

use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use gpui::{Entity, TestAppContext, VisualTestContext};
use matsu_platform::file_association::{Change, TypeState};

use super::list::TypeList;
use super::list_assoc::Backend;
use super::list_tests::{open_list, select, setup};
use super::tests::view_of;

/// What the fake registry holds, and what was written to it.
#[derive(Default)]
struct Registry {
    menu: bool,
    default: bool,
    writes: Vec<(String, Change)>,
    fail_read: bool,
}

struct Fake(Rc<RefCell<Registry>>);

impl Backend for Fake {
    fn supported(&self) -> bool {
        true
    }

    fn read(&self, exts: &str) -> io::Result<TypeState> {
        let r = self.0.borrow();
        if r.fail_read {
            return Err(io::Error::other("no program path"));
        }
        Ok(TypeState {
            available: !exts.is_empty(),
            menu: r.menu,
            default: r.default,
            overridden: Vec::new(),
        })
    }

    fn write(&self, exts: &str, change: Change) -> io::Result<()> {
        let mut r = self.0.borrow_mut();
        r.writes.push((exts.to_owned(), change));
        match change {
            Change::Menu(on) => {
                r.menu = on;
                r.default = on;
            }
            Change::Default(on) => r.default = on,
        }
        Ok(())
    }
}

/// A list with the fake registry, the type `index` selected.
fn list_with_fake(
    cx: &mut VisualTestContext,
    index: usize,
) -> (Entity<TypeList>, Rc<RefCell<Registry>>) {
    let view = view_of(cx);
    let list = open_list(&view, cx);
    let registry = Rc::new(RefCell::new(Registry::default()));
    select(&list, cx, index);
    list.update(cx, |l, lcx| {
        l.backend = Box::new(Fake(registry.clone()));
        l.read_association(lcx);
        lcx.notify();
    });
    cx.run_until_parked();
    (list, registry)
}

fn focus_menu(list: &Entity<TypeList>, cx: &mut VisualTestContext) {
    list.update_in(cx, |l, window, lcx| window.focus(&l.menu_focus, lcx));
}

fn focus_default(list: &Entity<TypeList>, cx: &mut VisualTestContext) {
    list.update_in(cx, |l, window, lcx| window.focus(&l.default_focus, lcx));
}

#[gpui::test]
fn space_on_the_check_boxes_changes_the_association_and_asks_once(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    // テキスト (type 1) has extensions.
    let (list, registry) = list_with_fake(cx, 1);
    focus_menu(&list, cx);
    cx.simulate_keystrokes("space");
    cx.run_until_parked();
    // The warning comes first, and nothing is written before the answer.
    assert!(cx.has_pending_prompt());
    assert!(registry.borrow().writes.is_empty());
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    let exts = "txt,log,1st,err,ps";
    assert_eq!(
        registry.borrow().writes,
        [(exts.to_owned(), Change::Menu(true))]
    );
    // The double click is now a tab stop; Space turns it off, without asking again.
    focus_default(&list, cx);
    cx.simulate_keystrokes("space");
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
    assert_eq!(
        registry.borrow().writes[1],
        (exts.to_owned(), Change::Default(false))
    );
    assert!(registry.borrow().menu);
}

#[gpui::test]
fn a_click_reads_the_registry_again_instead_of_trusting_what_was_drawn(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let (list, registry) = list_with_fake(cx, 1);
    // Drawn as "not associated" ...
    focus_menu(&list, cx);
    // ... but another type (or program) associated the extensions meanwhile.
    {
        let mut r = registry.borrow_mut();
        r.menu = true;
        r.default = true;
    }
    cx.simulate_keystrokes("space");
    cx.run_until_parked();
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    // The click removes it; it does not "add" a second time.
    assert_eq!(
        registry.borrow().writes.last().unwrap().1,
        Change::Menu(false)
    );
    assert!(!registry.borrow().menu);
}

#[gpui::test]
fn a_type_without_extensions_has_grey_check_boxes_that_do_nothing(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    // 基本 (type 0) has none.
    let (list, registry) = list_with_fake(cx, 0);
    focus_menu(&list, cx);
    cx.simulate_keystrokes("space");
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
    assert!(registry.borrow().writes.is_empty());
}

#[gpui::test]
fn the_double_click_is_grey_until_the_menu_item_is_on(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let (list, registry) = list_with_fake(cx, 1);
    focus_default(&list, cx);
    cx.simulate_keystrokes("space");
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
    assert!(registry.borrow().writes.is_empty());
}

#[gpui::test]
fn a_failure_to_read_is_not_taken_for_not_associated(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let (list, registry) = list_with_fake(cx, 1);
    registry.borrow_mut().fail_read = true;
    list.update(cx, |l, lcx| {
        l.read_association(lcx);
        lcx.notify();
    });
    cx.run_until_parked();
    focus_menu(&list, cx);
    cx.simulate_keystrokes("space");
    cx.run_until_parked();
    // Nothing is written; the user is told (a box), not asked.
    assert!(registry.borrow().writes.is_empty());
}
