//! Tests of where the outline pane sits in the window, the bar that sizes it, the right-click
//! menu that moves it, and what is kept in the settings file, in GPUI's headless test windows.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{
    AnyWindowHandle, Bounds, Modifiers, MouseButton, Pixels, TestAppContext, VisualTestContext,
    point, px, size,
};
use matsu_config::Profile;
use matsu_outline::{ListColumn, SortType};

use super::dock::{self, DockSide, OutlineDock};
use super::layout::{ideal_size, size_for_pointer};
use super::menu::MenuCommand;
use super::tests::{CPP, active_view, pane, run, window, window_with, workspace};
use crate::settings::UserSettings;

/// The settings file of a test, in a folder that goes at the end of the test.
struct TempIni(PathBuf);

impl TempIni {
    fn new(text: &str) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "matsu-outline-dock-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("matsu.ini");
        std::fs::write(&path, text).unwrap();
        Self(path)
    }

    fn text(&self) -> String {
        String::from_utf8(std::fs::read(&self.0).unwrap()).unwrap()
    }

    fn profile(&self) -> Profile {
        Profile::load(&self.0).unwrap()
    }
}

impl Drop for TempIni {
    fn drop(&mut self) {
        if let Some(dir) = self.0.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// Starts the program the way `app::run` does with the file `ini`: the history store and the
/// states are installed, and the settings are the global.
fn start(cx: &mut TestAppContext, ini: &TempIni) -> UserSettings {
    let user = UserSettings::load_from(std::slice::from_ref(&ini.0));
    cx.update(|cx| {
        crate::history::install(cx, &user);
        crate::saved_state::install(cx, &user);
    });
    user
}

/// A window with the settings of `ini` text (no file on disk).
fn open(cx: &mut TestAppContext, ini: &str) -> AnyWindowHandle {
    cx.update(|cx| {
        cx.set_global(OutlineDock::from_profile(&Profile::from_bytes(
            ini.as_bytes(),
        )))
    });
    window(cx, CPP, "a.cpp")
}

fn visual(cx: &mut TestAppContext, handle: AnyWindowHandle) -> &mut VisualTestContext {
    VisualTestContext::from_window(handle, cx).into_mut()
}

fn near(a: Pixels, b: Pixels) -> bool {
    f32::from(a - b).abs() < 0.6
}

struct Rects {
    pane: Bounds<Pixels>,
    text: Bounds<Pixels>,
}

fn rects(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Rects {
    cx.run_until_parked();
    let vcx = visual(cx, handle);
    Rects {
        pane: vcx
            .debug_bounds("outline-pane-box")
            .expect("the pane is drawn"),
        text: vcx.debug_bounds("outline-text").expect("the text is drawn"),
    }
}

fn current_place(cx: &mut TestAppContext, handle: AnyWindowHandle) -> dock::DockPlace {
    pane(cx, handle)
        .expect("the pane is open")
        .read_with(cx, |p, cx| p.place(cx))
}

fn dock_now(cx: &mut TestAppContext) -> OutlineDock {
    cx.update(|cx| dock::current(cx))
}

fn bar_gap() -> Pixels {
    px(5.)
}

#[test]
fn the_ideal_size_is_a_third_and_keeps_the_least_of_both_parts() {
    // GetIdealStretch: 0 is a third of the area
    assert_eq!(ideal_size(0, 900.0), 300.0);
    assert_eq!(ideal_size(200, 900.0), 200.0);
    // too small and too large are brought in (DOCK_MIN_SIZE is 60)
    assert_eq!(ideal_size(10, 900.0), 60.0);
    assert_eq!(ideal_size(5000, 900.0), 835.0);
}

#[test]
fn the_bar_follows_the_pointer_on_each_side() {
    let area = Bounds::new(point(px(100.), px(50.)), size(px(800.), px(600.)));
    let at = |x: f32, y: f32| point(px(x), px(y));
    // the middle of the bar is under the pointer: 2.5 pixels of it are on the other side
    assert_eq!(size_for_pointer(DockSide::Left, area, at(400., 0.)), 297.5);
    assert_eq!(size_for_pointer(DockSide::Right, area, at(700., 0.)), 197.5);
    assert_eq!(size_for_pointer(DockSide::Top, area, at(0., 250.)), 197.5);
    assert_eq!(
        size_for_pointer(DockSide::Bottom, area, at(0., 450.)),
        197.5
    );
    // it cannot take the text away, or shrink to nothing
    assert_eq!(size_for_pointer(DockSide::Left, area, at(2000., 0.)), 735.0);
    assert_eq!(size_for_pointer(DockSide::Right, area, at(2000., 0.)), 60.0);
    // a floating pane is sized like one at the right
    assert_eq!(size_for_pointer(DockSide::Float, area, at(700., 0.)), 197.5);
}

#[gpui::test]
fn without_settings_the_pane_is_docked_at_the_right(cx: &mut TestAppContext) {
    let handle = window(cx, CPP, "a.cpp");
    run(cx, handle, "F_OUTLINE");
    let r = rects(cx, handle);
    assert_eq!(current_place(cx, handle).side, DockSide::Right);
    // the text is at the left of the pane, with the bar between, and both have the full height
    assert!(near(r.text.right() + bar_gap(), r.pane.left()));
    assert!(near(r.text.top(), r.pane.top()) && near(r.text.bottom(), r.pane.bottom()));
    // a third of the window: Sakura's first size
    let total = r.text.size.width + bar_gap() + r.pane.size.width;
    assert!(
        near(r.pane.size.width, total / 3.0),
        "{r:?}",
        r = (r.pane, r.text)
    );
}

#[gpui::test]
fn a_pane_at_the_left_has_the_width_the_file_says(cx: &mut TestAppContext) {
    let handle = open(
        cx,
        "[Common]\r\neOutlineDockSide=1\r\nxyOutlineDock=200,0,300,0\r\n",
    );
    run(cx, handle, "F_OUTLINE");
    let r = rects(cx, handle);
    assert!(near(r.pane.size.width, px(200.)), "{:?}", r.pane);
    assert!(near(r.pane.right() + bar_gap(), r.text.left()));
    assert!(near(r.text.top(), r.pane.top()) && near(r.text.bottom(), r.pane.bottom()));
}

#[gpui::test]
fn a_pane_at_the_right_has_the_width_the_file_says(cx: &mut TestAppContext) {
    let handle = open(
        cx,
        "[Common]\r\neOutlineDockSide=3\r\nxyOutlineDock=200,0,300,0\r\n",
    );
    run(cx, handle, "F_OUTLINE");
    let r = rects(cx, handle);
    assert!(near(r.pane.size.width, px(300.)), "{:?}", r.pane);
    assert!(near(r.text.right() + bar_gap(), r.pane.left()));
    assert!(near(r.text.top(), r.pane.top()) && near(r.text.bottom(), r.pane.bottom()));
}

#[gpui::test]
fn a_pane_at_the_top_has_the_height_the_file_says(cx: &mut TestAppContext) {
    let handle = open(
        cx,
        "[Common]\r\neOutlineDockSide=2\r\nxyOutlineDock=200,120,300,140\r\n",
    );
    run(cx, handle, "F_OUTLINE");
    let r = rects(cx, handle);
    assert!(near(r.pane.size.height, px(120.)), "{:?}", r.pane);
    assert!(near(r.pane.bottom() + bar_gap(), r.text.top()));
    // and the full width
    assert!(near(r.text.left(), r.pane.left()) && near(r.text.right(), r.pane.right()));
}

#[gpui::test]
fn a_pane_at_the_bottom_has_the_height_the_file_says(cx: &mut TestAppContext) {
    let handle = open(
        cx,
        "[Common]\r\neOutlineDockSide=4\r\nxyOutlineDock=200,120,300,140\r\n",
    );
    run(cx, handle, "F_OUTLINE");
    let r = rects(cx, handle);
    assert!(near(r.pane.size.height, px(140.)), "{:?}", r.pane);
    assert!(near(r.text.bottom() + bar_gap(), r.pane.top()));
    assert!(near(r.text.left(), r.pane.left()) && near(r.text.right(), r.pane.right()));
}

#[gpui::test]
fn a_floating_pane_covers_the_text_and_takes_none_of_it(cx: &mut TestAppContext) {
    for side in [0, -1] {
        let handle = open(
            cx,
            &format!("[Common]\r\neOutlineDockSide={side}\r\nxyOutlineDock=0,0,250,0\r\n"),
        );
        run(cx, handle, "F_OUTLINE");
        let r = rects(cx, handle);
        assert!(near(r.pane.size.width, px(250.)), "{:?}", r.pane);
        // inside the text, at its right edge
        assert!(r.pane.right() <= r.text.right() + px(0.6));
        assert!(r.pane.left() >= r.text.left());
        assert!(r.pane.size.height < r.text.size.height);
    }
}

#[gpui::test]
fn the_pane_of_the_type_is_used_when_the_file_says_so(cx: &mut TestAppContext) {
    let handle = window(cx, CPP, "a.cpp");
    let type_index = active_view(cx, handle).read_with(cx, |v, _| v.view_settings().type_index);
    let ini = format!(
        "[Common]\r\nnOutlineDockSet=1\r\neOutlineDockSide=3\r\n[Types({type_index})]\r\neOutlineDockSide=4\r\nxyOutlineDock=0,0,0,100\r\n"
    );
    cx.update(|cx| {
        cx.set_global(OutlineDock::from_profile(&Profile::from_bytes(
            ini.as_bytes(),
        )))
    });
    run(cx, handle, "F_OUTLINE");
    let r = rects(cx, handle);
    assert_eq!(current_place(cx, handle).side, DockSide::Bottom);
    assert!(near(r.pane.size.height, px(100.)), "{:?}", r.pane);
}

fn pane_command(cx: &mut TestAppContext, handle: AnyWindowHandle, command: MenuCommand) {
    let pane = pane(cx, handle).expect("the pane is open");
    handle
        .update(cx, |_, window, app| {
            pane.update(app, |p, cx| p.run_menu_command(command, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

#[gpui::test]
fn the_menu_moves_the_pane_to_each_side(cx: &mut TestAppContext) {
    let handle = window(cx, CPP, "a.cpp");
    run(cx, handle, "F_OUTLINE");
    for side in [
        DockSide::Left,
        DockSide::Top,
        DockSide::Bottom,
        DockSide::Right,
    ] {
        pane_command(cx, handle, MenuCommand::Side(side));
        let r = rects(cx, handle);
        assert_eq!(current_place(cx, handle).side, side);
        match side {
            DockSide::Left => assert!(near(r.pane.right() + bar_gap(), r.text.left())),
            DockSide::Right => assert!(near(r.text.right() + bar_gap(), r.pane.left())),
            DockSide::Top => assert!(near(r.pane.bottom() + bar_gap(), r.text.top())),
            _ => assert!(near(r.text.bottom() + bar_gap(), r.pane.top())),
        }
    }
    // floating: the text takes the whole area again
    pane_command(cx, handle, MenuCommand::Side(DockSide::Float));
    let r = rects(cx, handle);
    assert!(r.pane.left() >= r.text.left() && r.pane.right() <= r.text.right() + px(0.6));
}

#[gpui::test]
fn the_pane_still_jumps_to_the_text_at_every_side(cx: &mut TestAppContext) {
    let handle = window(cx, CPP, "a.cpp");
    run(cx, handle, "F_OUTLINE");
    for side in [
        DockSide::Left,
        DockSide::Top,
        DockSide::Bottom,
        DockSide::Float,
        DockSide::Right,
    ] {
        pane_command(cx, handle, MenuCommand::Side(side));
        // the row of "main" is the last
        let rows = super::tests::rows(cx, handle);
        let last = rows.len() - 1;
        let pane = pane(cx, handle).unwrap();
        handle
            .update(cx, |_, window, app| {
                pane.update(app, |p, cx| p.jump(last, false, window, cx))
            })
            .unwrap();
        cx.run_until_parked();
        let caret = active_view(cx, handle).read_with(cx, |v, _| v.cursor().caret());
        assert_eq!(caret.line, 7, "{side:?}");
    }
}

#[gpui::test]
fn dragging_the_bar_sizes_the_pane_and_keeps_the_size(cx: &mut TestAppContext) {
    let ini = TempIni::new("[Common]\r\neOutlineDockSide=3\r\nxyOutlineDock=0,0,300,0\r\n");
    let user = start(cx, &ini);
    let handle = window_with(cx, CPP, "a.cpp", user);
    run(cx, handle, "F_OUTLINE");
    let _ = rects(cx, handle);
    let vcx = visual(cx, handle);
    let bar = vcx.debug_bounds("outline-splitter").expect("the bar");
    let from = bar.center();
    let to = point(from.x - px(100.), from.y);
    vcx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
    vcx.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
    // while the bar is held the file is not written
    assert!(ini.text().contains("xyOutlineDock=0,0,300,0"));
    vcx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
    let r = rects(cx, handle);
    assert!(near(r.pane.size.width, px(400.)), "{:?}", r.pane);
    assert_eq!(dock_now(cx).common.right, 400);
    assert!(
        ini.text().contains("xyOutlineDock=0,0,400,0"),
        "{}",
        ini.text()
    );
    // the other sides keep their own sizes
    assert_eq!(ini.profile().get("Common", "eOutlineDockSide"), Some("3"));
}

#[gpui::test]
fn the_bar_cannot_take_all_of_the_text(cx: &mut TestAppContext) {
    let handle = open(
        cx,
        "[Common]\r\neOutlineDockSide=3\r\nxyOutlineDock=0,0,300,0\r\n",
    );
    run(cx, handle, "F_OUTLINE");
    let _ = rects(cx, handle);
    let vcx = visual(cx, handle);
    let bar = vcx.debug_bounds("outline-splitter").expect("the bar");
    vcx.simulate_mouse_down(bar.center(), MouseButton::Left, Modifiers::default());
    let far = point(px(-500.), bar.center().y);
    vcx.simulate_mouse_move(far, MouseButton::Left, Modifiers::default());
    vcx.simulate_mouse_up(far, MouseButton::Left, Modifiers::default());
    let r = rects(cx, handle);
    // DOCK_MIN_SIZE is what the text keeps
    assert!(r.text.size.width >= px(59.4), "{:?}", r.text);
}

#[gpui::test]
fn a_move_without_the_button_does_not_drag(cx: &mut TestAppContext) {
    let handle = open(
        cx,
        "[Common]\r\neOutlineDockSide=3\r\nxyOutlineDock=0,0,300,0\r\n",
    );
    run(cx, handle, "F_OUTLINE");
    let _ = rects(cx, handle);
    let vcx = visual(cx, handle);
    let bar = vcx.debug_bounds("outline-splitter").expect("the bar");
    vcx.simulate_mouse_down(bar.center(), MouseButton::Left, Modifiers::default());
    // the button was let go where the window did not see it
    vcx.simulate_mouse_move(
        point(bar.center().x - px(80.), bar.center().y),
        None,
        Modifiers::default(),
    );
    let r = rects(cx, handle);
    assert!(near(r.pane.size.width, px(300.)), "{:?}", r.pane);
}

#[gpui::test]
fn the_right_click_opens_the_menu(cx: &mut TestAppContext) {
    let handle = window(cx, CPP, "a.cpp");
    run(cx, handle, "F_OUTLINE");
    let r = rects(cx, handle);
    let pane = pane(cx, handle).unwrap();
    assert!(!pane.read_with(cx, |p, _| p.menu_is_open()));
    let vcx = visual(cx, handle);
    let at = r.pane.center();
    vcx.simulate_mouse_down(at, MouseButton::Right, Modifiers::default());
    vcx.simulate_mouse_up(at, MouseButton::Right, Modifiers::default());
    cx.run_until_parked();
    assert!(pane.read_with(cx, |p, _| p.menu_is_open()));
}

#[gpui::test]
fn the_sort_is_kept_for_the_type_and_comes_back(cx: &mut TestAppContext) {
    let ini = TempIni::new("[Common]\r\nnOutlineDockSet=0\r\n");
    let user = start(cx, &ini);
    let handle = window_with(cx, CPP, "a.cpp", user);
    let type_index = active_view(cx, handle).read_with(cx, |v, _| v.view_settings().type_index);
    run(cx, handle, "F_OUTLINE");
    let p = pane(cx, handle).unwrap();
    p.update(cx, |p, cx| p.set_sort_type(SortType::ZtoA, cx));
    p.update(cx, |p, cx| p.click_column(ListColumn::Name, cx));
    let section = format!("Types({type_index})");
    let kept = ini.profile();
    assert_eq!(kept.get(&section, "nOutlineSortType"), Some("3"));
    // the click on a column keeps the column and the direction as Sakura does
    assert_eq!(kept.get(&section, "nOutlineSortCol"), Some("2"));
    assert_eq!(kept.get(&section, "bOutlineSortDesc"), Some("0"));

    // the next start: a new pane sorts the same way
    let handle = window_with(cx, CPP, "b.cpp", UserSettings::none());
    cx.update(|cx| cx.set_global(OutlineDock::from_profile(&ini.profile())));
    run(cx, handle, "F_OUTLINE");
    let p = pane(cx, handle).unwrap();
    let (list, tree) = p.read_with(cx, |p, _| p.sorts());
    assert_eq!(tree, SortType::ZtoA);
    assert_eq!(list.column, ListColumn::Name);
    assert!(!list.descending);
}

#[gpui::test]
fn the_place_is_kept_and_the_next_start_opens_the_pane_there(cx: &mut TestAppContext) {
    let ini = TempIni::new("[Common]\r\nnTabs=1\r\n");
    let user = start(cx, &ini);
    let handle = window_with(cx, CPP, "a.cpp", user);
    run(cx, handle, "F_OUTLINE");
    // opening is kept
    assert_eq!(ini.profile().get("Common", "bOutlineDockDisp"), Some("1"));
    pane_command(cx, handle, MenuCommand::Side(DockSide::Top));
    assert_eq!(ini.profile().get("Common", "eOutlineDockSide"), Some("2"));
    // the other keys of the file are left as they were
    assert_eq!(ini.profile().get("Common", "nTabs"), Some("1"));

    // the next start reads the file: a window opens with the pane at the top
    let restarted = OutlineDock::from_profile(&ini.profile());
    cx.update(|cx| cx.set_global(restarted));
    let second = window(cx, CPP, "b.cpp");
    cx.run_until_parked();
    assert!(pane(cx, second).is_some(), "the pane was open");
    let r = rects(cx, second);
    assert!(near(r.pane.bottom() + bar_gap(), r.text.top()));

    // closing it is kept as well, and a window that was not given the pane does not open one
    let p = pane(cx, second).unwrap();
    handle
        .update(cx, |_, window, app| {
            p.update(app, |p, cx| {
                p.run_menu_command(MenuCommand::Close, window, cx)
            })
        })
        .unwrap();
    cx.run_until_parked();
    assert!(pane(cx, second).is_none());
    assert_eq!(ini.profile().get("Common", "bOutlineDockDisp"), Some("0"));
}

#[gpui::test]
fn a_pane_that_floated_is_not_opened_by_the_next_start(cx: &mut TestAppContext) {
    // ChangeLayout does not open a floating dialog for a new window
    let handle = open(
        cx,
        "[Common]\r\nbOutlineDockDisp=1\r\neOutlineDockSide=0\r\n",
    );
    cx.run_until_parked();
    assert!(pane(cx, handle).is_none());
    // a docked one is
    let handle = open(
        cx,
        "[Common]\r\nbOutlineDockDisp=1\r\neOutlineDockSide=1\r\n",
    );
    cx.run_until_parked();
    assert!(pane(cx, handle).is_some());
    // and a closed one is not
    let handle = open(
        cx,
        "[Common]\r\nbOutlineDockDisp=0\r\neOutlineDockSide=1\r\n",
    );
    cx.run_until_parked();
    assert!(pane(cx, handle).is_none());
}

#[gpui::test]
fn a_restored_pane_leaves_the_focus_in_the_text(cx: &mut TestAppContext) {
    let handle = open(
        cx,
        "[Common]\r\nbOutlineDockDisp=1\r\neOutlineDockSide=3\r\n",
    );
    cx.run_until_parked();
    assert!(pane(cx, handle).is_some());
    let view = active_view(cx, handle);
    let focused = handle
        .update(cx, |_, window, app| {
            view.read(app).focus_handle.is_focused(window)
        })
        .unwrap();
    assert!(focused);
}

#[gpui::test]
fn with_the_windows_in_sync_a_change_reaches_the_other_window(cx: &mut TestAppContext) {
    let first = window(cx, CPP, "a.cpp");
    let second = window(cx, CPP, "b.cpp");
    run(cx, first, "F_OUTLINE");
    run(cx, second, "F_OUTLINE");
    pane_command(cx, first, MenuCommand::Side(DockSide::Left));
    assert_eq!(current_place(cx, second).side, DockSide::Left);
}

#[gpui::test]
fn without_the_sync_a_change_stays_in_the_window(cx: &mut TestAppContext) {
    let first = window(cx, CPP, "a.cpp");
    let second = window(cx, CPP, "b.cpp");
    run(cx, first, "F_OUTLINE");
    run(cx, second, "F_OUTLINE");
    pane_command(cx, first, MenuCommand::Sync);
    assert!(!dock_now(cx).sync);
    pane_command(cx, first, MenuCommand::Side(DockSide::Bottom));
    assert_eq!(current_place(cx, first).side, DockSide::Bottom);
    assert_eq!(current_place(cx, second).side, DockSide::Right);
    // turning the sync on again makes the window follow the settings
    pane_command(cx, first, MenuCommand::Sync);
    assert_eq!(current_place(cx, first).side, DockSide::Bottom);
    assert_eq!(current_place(cx, second).side, DockSide::Bottom);
}

#[gpui::test]
fn choosing_the_inheritance_changes_where_the_place_comes_from(cx: &mut TestAppContext) {
    let handle = window(cx, CPP, "a.cpp");
    run(cx, handle, "F_OUTLINE");
    pane_command(cx, handle, MenuCommand::Side(DockSide::Left));
    // 共通継承 -> タイプ別継承: the type has nothing of its own yet, so it starts from the default
    pane_command(cx, handle, MenuCommand::Inherit(1));
    assert_eq!(dock_now(cx).dock_set, 1);
    assert_eq!(current_place(cx, handle).side, DockSide::Right);
    pane_command(cx, handle, MenuCommand::Side(DockSide::Bottom));
    pane_command(cx, handle, MenuCommand::Inherit(0));
    assert_eq!(current_place(cx, handle).side, DockSide::Left);
}

#[gpui::test]
fn expand_all_and_collapse_all_open_and_close_the_tree(cx: &mut TestAppContext) {
    let handle = window(cx, CPP, "a.cpp");
    run(cx, handle, "F_OUTLINE");
    pane_command(cx, handle, MenuCommand::CollapseAll);
    let collapsed = super::tests::rows(cx, handle);
    assert!(collapsed.iter().all(|r| r.depth == 0 && !r.expanded));
    pane_command(cx, handle, MenuCommand::ExpandAll);
    let expanded = super::tests::rows(cx, handle);
    assert!(expanded.len() > collapsed.len());
    assert!(expanded.iter().any(|r| r.depth > 0));
}

#[gpui::test]
fn the_bookmark_items_of_the_menu_clear_the_bookmarks(cx: &mut TestAppContext) {
    let handle = window(cx, "a\nb\nc\nd\n", "a.txt");
    let view = active_view(cx, handle);
    for line in [1, 3] {
        view.update(cx, |v, cx| {
            v.run_with(cx, |c| c.set_bookmark_line(line, true));
        });
    }
    run(cx, handle, "F_BOOKMARK_VIEW");
    let rows = super::tests::rows(cx, handle);
    assert_eq!(rows.len(), 2);
    // 'ブックマーク削除' works on the selected item
    let p = pane(cx, handle).unwrap();
    handle
        .update(cx, |_, window, app| {
            p.update(app, |p, cx| p.jump(0, false, window, cx))
        })
        .unwrap();
    pane_command(cx, handle, MenuCommand::BookmarkDelete);
    assert_eq!(super::tests::rows(cx, handle).len(), 1);
    let marks = |cx: &mut TestAppContext| {
        workspace(cx, handle).read_with(cx, |ws, cx| {
            ws.editor()
                .read(cx)
                .document()
                .read(cx)
                .model()
                .bookmarks()
                .iter()
                .collect::<Vec<_>>()
        })
    };
    assert_eq!(marks(cx), [3]);
    pane_command(cx, handle, MenuCommand::BookmarkDeleteAll);
    assert!(marks(cx).is_empty());
    assert!(super::tests::rows(cx, handle).is_empty());
}

#[gpui::test]
fn a_click_on_the_floating_pane_does_not_reach_the_text_under_it(cx: &mut TestAppContext) {
    let handle = open(
        cx,
        "[Common]\r\neOutlineDockSide=0\r\nxyOutlineDock=0,0,300,0\r\n",
    );
    run(cx, handle, "F_OUTLINE");
    let r = rects(cx, handle);
    // the text has its caret at the start, and the click is on the pane over the text
    let before = active_view(cx, handle).read_with(cx, |v, _| v.cursor().caret());
    let vcx = visual(cx, handle);
    let at = point(r.pane.center().x, r.pane.top() + px(8.));
    vcx.simulate_mouse_down(at, MouseButton::Left, Modifiers::default());
    vcx.simulate_mouse_up(at, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    let after = active_view(cx, handle).read_with(cx, |v, _| v.cursor().caret());
    assert_eq!(before, after);
    // and the focus is not taken by the text
    let view = active_view(cx, handle);
    let focused = handle
        .update(cx, |_, window, app| {
            view.read(app).focus_handle.is_focused(window)
        })
        .unwrap();
    assert!(!focused);
}

#[gpui::test]
fn unifying_reaches_the_windows_that_are_not_in_sync(cx: &mut TestAppContext) {
    let first = window(cx, CPP, "a.cpp");
    let second = window(cx, CPP, "b.cpp");
    run(cx, first, "F_OUTLINE");
    run(cx, second, "F_OUTLINE");
    pane_command(cx, second, MenuCommand::Sync);
    pane_command(cx, first, MenuCommand::Side(DockSide::Left));
    // the second window does not follow, it is not in sync
    assert_eq!(current_place(cx, second).side, DockSide::Right);
    let p = pane(cx, first).unwrap();
    p.update(cx, |p, cx| p.unify(cx));
    cx.run_until_parked();
    assert_eq!(current_place(cx, second).side, DockSide::Left);
}

#[gpui::test]
fn a_window_that_loads_a_file_restores_the_pane_of_the_type_of_that_file(cx: &mut TestAppContext) {
    use crate::document::SharedDocument;
    use crate::file_ops::windows::{LoadRequest, open_editor_window};

    let ini = TempIni::new("[Common]\r\n");
    let file = ini.0.with_file_name("a.cpp");
    std::fs::write(&file, "int main() {}\n").unwrap();
    let user = UserSettings::load_from(std::slice::from_ref(&ini.0));
    let type_index = user.type_index_for(Some("a.cpp"));
    // only the type of the file has the pane open (nOutlineDockSet=1)
    let text = format!(
        "[Common]\r\nnOutlineDockSet=1\r\n[Types({type_index})]\r\nbOutlineDockDisp=1\r\neOutlineDockSide=3\r\n"
    );
    cx.update(|cx| {
        cx.set_global(OutlineDock::from_profile(&Profile::from_bytes(
            text.as_bytes(),
        )));
        cx.set_global(user);
    });
    cx.update(gpui_component::init);
    let handle = cx
        .update(|cx| {
            open_editor_window(
                cx,
                SharedDocument::empty(),
                Some(LoadRequest {
                    path: file.clone(),
                    encoding: None,
                }),
            )
        })
        .unwrap();
    cx.run_until_parked();
    assert!(pane(cx, handle).is_some(), "the pane of the type is open");
    let _ = std::fs::remove_file(&file);
}
