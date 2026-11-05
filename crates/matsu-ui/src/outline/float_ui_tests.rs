//! Tests in GPUI's headless test windows of what is left of the outline pane in the window: the
//! position and the size of the floating pane (`bRememberOutlineWindowPos`), docking by dragging
//! the title (`CDlgFuncList::Track`), and the settings of the type that follow the tab
//! (`ChangeLayout` for a changed file).

use gpui::{AnyWindowHandle, Bounds, Modifiers, MouseButton, Pixels, TestAppContext, point, px};

use super::dock::{DockSide, FloatRect, OutlineDock};
use super::layout::fit_float;
use super::layout_tests::{
    TempIni, bar_gap, current_place, dock_now, near, open, pane_command, rects, start, visual,
};
use super::menu::MenuCommand;
use super::tests::{CPP, active_view, pane, run, window, window_with};
use matsu_config::Profile;

/// A floating pane that remembers: 320 x 260 at (40, 30).
const REMEMBER: &str = "[Common]\r\neOutlineDockSide=0\r\nbRememberOutlineWindowPos=1\r\n\
     widthOutlineWindow=320\r\nheightOutlineWindow=260\r\nxOutlineWindowPos=40\r\n\
     yOutlineWindowPos=30\r\n";

fn float_bounds(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
) -> (Bounds<Pixels>, Bounds<Pixels>) {
    cx.run_until_parked();
    let vcx = visual(cx, handle);
    (
        vcx.debug_bounds("outline-text").expect("the text"),
        vcx.debug_bounds("outline-float")
            .expect("the floating pane"),
    )
}

fn open_float(cx: &mut TestAppContext, ini: &str) -> AnyWindowHandle {
    let handle = open(cx, ini);
    run(cx, handle, "F_OUTLINE");
    handle
}

/// Takes hold of the title, moves the pointer by `by` and lets go with `modifiers`.
fn drag_title(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    to: impl FnOnce(gpui::Point<Pixels>) -> gpui::Point<Pixels>,
    release: bool,
    modifiers: Modifiers,
) {
    let _ = rects(cx, handle);
    let vcx = visual(cx, handle);
    let header = vcx.debug_bounds("outline-header").expect("the title");
    let from = point(header.left() + px(6.), header.center().y);
    let to = to(from);
    vcx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
    vcx.simulate_mouse_move(to, MouseButton::Left, modifiers);
    if release {
        vcx.simulate_mouse_up(to, MouseButton::Left, modifiers);
    }
    cx.run_until_parked();
}

fn by(dx: f32, dy: f32) -> impl FnOnce(gpui::Point<Pixels>) -> gpui::Point<Pixels> {
    move |p| point(p.x + px(dx), p.y + px(dy))
}

#[test]
fn the_floating_pane_is_inside_the_area_and_keeps_what_it_can_of_its_size() {
    // no size: at the right with the width given, 70% of the height
    let r = fit_float(None, 1000.0, 600.0, 300.0);
    assert_eq!(
        r,
        FloatRect {
            x: 695,
            y: 0,
            w: 305,
            h: 420
        }
    );
    // a size that is set is kept, and pulled inside
    let set = FloatRect {
        x: 900,
        y: 500,
        w: 320,
        h: 260,
    };
    let r = fit_float(Some(set), 1000.0, 600.0, 300.0);
    assert_eq!(
        r,
        FloatRect {
            x: 680,
            y: 340,
            w: 320,
            h: 260
        }
    );
    // a position left of the area (a negative number of the file) is pulled in
    let set = FloatRect {
        x: -50,
        y: -5,
        w: 320,
        h: 260,
    };
    let r = fit_float(Some(set), 1000.0, 600.0, 300.0);
    assert_eq!((r.x, r.y), (0, 0));
    // too large for a small window, too small for the least
    let r = fit_float(Some(set), 200.0, 100.0, 300.0);
    assert_eq!((r.w, r.h), (200, 100));
    let tiny = FloatRect {
        x: 0,
        y: 0,
        w: 10,
        h: 10,
    };
    let r = fit_float(Some(tiny), 1000.0, 600.0, 300.0);
    assert_eq!((r.w, r.h), (65, 65));
    // a width or a height of 0 is "not set"
    let r = fit_float(
        Some(FloatRect {
            x: 5,
            y: 5,
            w: 0,
            h: 0,
        }),
        1000.0,
        600.0,
        300.0,
    );
    assert_eq!((r.x, r.y), (695, 0));
}

#[gpui::test]
fn the_remembered_position_and_size_place_the_floating_pane(cx: &mut TestAppContext) {
    let handle = open_float(cx, REMEMBER);
    let (area, float) = float_bounds(cx, handle);
    assert!(near(float.left(), area.left() + px(40.)), "{float:?}");
    assert!(near(float.top(), area.top() + px(30.)), "{float:?}");
    assert!(near(float.size.width, px(320.)), "{float:?}");
    assert!(near(float.size.height, px(260.)), "{float:?}");
}

#[gpui::test]
fn without_the_box_the_keys_of_the_file_are_not_used(cx: &mut TestAppContext) {
    let text = REMEMBER.replace("bRememberOutlineWindowPos=1", "bRememberOutlineWindowPos=0");
    let handle = open_float(cx, &text);
    let (area, float) = float_bounds(cx, handle);
    // the default place: at the right edge and the top, a third of the area wide
    assert!(near(float.right(), area.right()), "{float:?}");
    assert!(near(float.top(), area.top()), "{float:?}");
    assert!(
        near(float.size.width, area.size.width / 3.0 + bar_gap()),
        "{float:?}"
    );
}

#[gpui::test]
fn moving_the_floating_pane_by_its_title_keeps_the_place_when_the_box_is_on(
    cx: &mut TestAppContext,
) {
    let ini = TempIni::new(REMEMBER);
    let user = start(cx, &ini);
    let handle = window_with(cx, CPP, "a.cpp", user);
    super::tests::run(cx, handle, "F_OUTLINE");
    let (area, before) = float_bounds(cx, handle);
    drag_title(cx, handle, by(-20.0, 60.0), false, Modifiers::default());
    // while the title is held the frame of the place is drawn, and the file is not written
    assert!(
        visual(cx, handle)
            .debug_bounds("outline-drop-frame")
            .is_some()
    );
    assert!(ini.text().contains("xOutlineWindowPos=40"));
    let vcx = visual(cx, handle);
    let header = vcx.debug_bounds("outline-header").unwrap();
    let to = point(header.left() + px(6. - 20.), header.center().y + px(60.));
    vcx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    assert!(
        visual(cx, handle)
            .debug_bounds("outline-drop-frame")
            .is_none()
    );
    let (_, after) = float_bounds(cx, handle);
    assert!(near(after.left(), before.left() - px(20.)), "{after:?}");
    assert!(near(after.top(), before.top() + px(60.)), "{after:?}");
    assert!(near(after.size.width, before.size.width));
    assert_eq!(current_place(cx, handle).side, DockSide::Float);
    // the place went to the file, relative to the area
    let dock = dock_now(cx);
    assert!(dock.remember);
    assert_eq!(
        dock.float,
        FloatRect {
            x: 20,
            y: 90,
            w: 320,
            h: 260
        }
    );
    let p = ini.profile();
    assert_eq!(p.get("Common", "xOutlineWindowPos"), Some("20"));
    assert_eq!(p.get("Common", "yOutlineWindowPos"), Some("90"));
    assert_eq!(p.get("Common", "widthOutlineWindow"), Some("320"));
    assert_eq!(p.get("Common", "heightOutlineWindow"), Some("260"));
    // and that is what the next start gives
    assert_eq!(OutlineDock::from_profile(&p).float, dock.float);
    let _ = area;
}

#[gpui::test]
fn without_the_box_a_move_is_not_written(cx: &mut TestAppContext) {
    let ini = TempIni::new("[Common]\r\neOutlineDockSide=0\r\n");
    let user = start(cx, &ini);
    let handle = window_with(cx, CPP, "a.cpp", user);
    run(cx, handle, "F_OUTLINE");
    let (_, before) = float_bounds(cx, handle);
    drag_title(cx, handle, by(-100.0, 80.0), true, Modifiers::default());
    let (_, after) = float_bounds(cx, handle);
    // it moved
    assert!(near(after.left(), before.left() - px(100.)), "{after:?}");
    let text = ini.text();
    assert!(!text.contains("xOutlineWindowPos"), "{text}");
    assert!(!text.contains("widthOutlineWindow"), "{text}");
    assert!(!dock_now(cx).remember);
}

#[gpui::test]
fn the_bars_of_the_floating_pane_size_it_and_the_size_is_kept(cx: &mut TestAppContext) {
    let ini = TempIni::new(REMEMBER);
    let user = start(cx, &ini);
    let handle = window_with(cx, CPP, "a.cpp", user);
    run(cx, handle, "F_OUTLINE");
    let _ = float_bounds(cx, handle);
    let vcx = visual(cx, handle);
    // the left bar: the right edge stays
    let bar = vcx.debug_bounds("outline-splitter").expect("the left bar");
    let from = bar.center();
    let to = point(from.x - px(25.), from.y);
    vcx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
    vcx.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
    vcx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    // the bottom bar
    let vcx = visual(cx, handle);
    let bottom = vcx
        .debug_bounds("outline-float-bottom")
        .expect("the bottom bar");
    let from = bottom.center();
    let to = point(from.x, from.y + px(40.));
    vcx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
    vcx.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
    vcx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    let dock = dock_now(cx);
    assert_eq!((dock.float.w, dock.float.h), (345, 300));
    assert_eq!(
        dock.float.x + dock.float.w,
        40 + 320,
        "the right edge stays"
    );
    // the docked sizes are not the size of the floating pane
    assert_eq!(dock.common.right, 0);
    let p = ini.profile();
    assert_eq!(p.get("Common", "widthOutlineWindow"), Some("345"));
    assert_eq!(p.get("Common", "heightOutlineWindow"), Some("300"));
}

#[gpui::test]
fn the_button_switches_the_box_and_a_floating_pane_only_has_it(cx: &mut TestAppContext) {
    let ini = TempIni::new("[Common]\r\neOutlineDockSide=0\r\n");
    let user = start(cx, &ini);
    let handle = window_with(cx, CPP, "a.cpp", user);
    run(cx, handle, "F_OUTLINE");
    let _ = float_bounds(cx, handle);
    let vcx = visual(cx, handle);
    let button = vcx.debug_bounds("outline-remember");
    let at = button.expect("the button").center();
    vcx.simulate_click(at, Modifiers::default());
    cx.run_until_parked();
    assert!(dock_now(cx).remember);
    let p = ini.profile();
    assert_eq!(p.get("Common", "bRememberOutlineWindowPos"), Some("1"));
    // what the pane has now is kept with it
    assert!(p.get("Common", "widthOutlineWindow").is_some());
    let vcx = visual(cx, handle);
    let at = vcx.debug_bounds("outline-remember").unwrap().center();
    vcx.simulate_click(at, Modifiers::default());
    cx.run_until_parked();
    assert!(!dock_now(cx).remember);
    let p = ini.profile();
    assert_eq!(p.get("Common", "bRememberOutlineWindowPos"), Some("0"));
    // docked, there is no button
    pane_command(cx, handle, MenuCommand::Side(DockSide::Left));
    let _ = rects(cx, handle);
    assert!(
        visual(cx, handle)
            .debug_bounds("outline-remember")
            .is_none()
    );
}

#[gpui::test]
fn a_floating_pane_that_docks_or_closes_keeps_its_place_when_the_box_is_on(
    cx: &mut TestAppContext,
) {
    let ini = TempIni::new(REMEMBER);
    let user = start(cx, &ini);
    let handle = window_with(cx, CPP, "a.cpp", user);
    run(cx, handle, "F_OUTLINE");
    let _ = float_bounds(cx, handle);
    // what the file has is not the place of another size
    cx.update(|cx| {
        crate::outline::dock::set_memory(cx, |d| d.float = FloatRect::default());
    });
    pane_command(cx, handle, MenuCommand::Side(DockSide::Left));
    assert_eq!(
        dock_now(cx).float,
        FloatRect {
            x: 40,
            y: 30,
            w: 320,
            h: 260
        }
    );
    // floating again: the remembered place
    pane_command(cx, handle, MenuCommand::Side(DockSide::Float));
    let (area, float) = float_bounds(cx, handle);
    assert!(near(float.left(), area.left() + px(40.)), "{float:?}");
    assert!(near(float.size.height, px(260.)), "{float:?}");
}

#[gpui::test]
fn the_title_dragged_to_the_left_edge_docks_the_pane_there(cx: &mut TestAppContext) {
    let ini = TempIni::new("[Common]\r\neOutlineDockSide=0\r\n");
    let user = start(cx, &ini);
    let handle = window_with(cx, CPP, "a.cpp", user);
    run(cx, handle, "F_OUTLINE");
    let (area, _) = float_bounds(cx, handle);
    let target = point(area.left() + px(10.), area.center().y);
    drag_title(cx, handle, move |_| target, false, Modifiers::default());
    // the frame is the place it takes: the full height, a third of the area wide
    let frame = visual(cx, handle)
        .debug_bounds("outline-drop-frame")
        .expect("the frame");
    assert!(near(frame.size.height, area.size.height), "{frame:?}");
    assert!(
        near(frame.size.width, (area.size.width / 3.0).floor()),
        "{frame:?}"
    );
    assert_eq!(current_place(cx, handle).side, DockSide::Float);
    visual(cx, handle).simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    let place = current_place(cx, handle);
    assert_eq!(place.side, DockSide::Left);
    assert!(place.disp);
    assert_eq!(
        place.left,
        (f32::from(area.size.width) / 3.0).floor() as i32
    );
    let p = ini.profile();
    assert_eq!(p.get("Common", "eOutlineDockSide"), Some("1"));
    // the pane is a part of the window now: the text has the rest
    let r = rects(cx, handle);
    assert!(near(r.pane.left(), area.left()));
    assert!(r.text.size.width < area.size.width);
}

#[gpui::test]
fn the_title_dragged_to_each_edge_docks_there(cx: &mut TestAppContext) {
    for side in [DockSide::Right, DockSide::Top, DockSide::Bottom] {
        let handle = open_float(cx, "[Common]\r\neOutlineDockSide=0\r\n");
        let (area, _) = float_bounds(cx, handle);
        // 10 pixels from the edge (the edge is 32)
        let target = match side {
            DockSide::Right => point(area.right() - px(10.), area.center().y),
            DockSide::Top => point(area.center().x, area.top() + px(10.)),
            _ => point(area.center().x, area.bottom() - px(10.)),
        };
        drag_title(cx, handle, move |_| target, true, Modifiers::default());
        assert_eq!(current_place(cx, handle).side, side, "{side:?}");
        let size = dock_now(cx).common.size_of(side);
        assert!(size > 0, "{side:?}");
        // the sizes of the other sides are not touched
        let other = if side == DockSide::Right {
            dock_now(cx).common.left
        } else {
            dock_now(cx).common.right
        };
        assert_eq!(other, 0);
    }
}

#[gpui::test]
fn ctrl_and_escape_and_the_right_button_keep_the_pane_where_it_is(cx: &mut TestAppContext) {
    let handle = open_float(cx, "[Common]\r\neOutlineDockSide=0\r\n");
    let (area, _) = float_bounds(cx, handle);
    let target = point(area.left() + px(10.), area.center().y);
    // Ctrl: it stays a floating pane, at the place of the pointer
    drag_title(
        cx,
        handle,
        move |_| target,
        true,
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
    );
    assert_eq!(current_place(cx, handle).side, DockSide::Float);
    // Esc gives the drag up (the pane is at the left now: the title is taken at its new place)
    let far = point(area.center().x, area.top() + px(10.));
    drag_title(cx, handle, move |_| far, false, Modifiers::default());
    assert!(
        visual(cx, handle)
            .debug_bounds("outline-drop-frame")
            .is_some()
    );
    visual(cx, handle).simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(
        visual(cx, handle)
            .debug_bounds("outline-drop-frame")
            .is_none()
    );
    visual(cx, handle).simulate_mouse_up(far, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(current_place(cx, handle).side, DockSide::Float);
    // the right button
    drag_title(cx, handle, move |_| far, false, Modifiers::default());
    visual(cx, handle).simulate_mouse_down(far, MouseButton::Right, Modifiers::default());
    cx.run_until_parked();
    assert!(
        visual(cx, handle)
            .debug_bounds("outline-drop-frame")
            .is_none()
    );
    visual(cx, handle).simulate_mouse_up(far, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(current_place(cx, handle).side, DockSide::Float);
    // and no menu opened with it
    let p = pane(cx, handle).unwrap();
    assert!(!p.read_with(cx, |p, _| p.menu_is_open()));
}

#[gpui::test]
fn a_click_on_the_title_does_not_move_the_pane(cx: &mut TestAppContext) {
    let handle = open_float(cx, "[Common]\r\neOutlineDockSide=3\r\n");
    let _ = rects(cx, handle);
    drag_title(cx, handle, by(2.0, 1.0), true, Modifiers::default());
    assert_eq!(current_place(cx, handle).side, DockSide::Right);
}

#[gpui::test]
fn a_docked_pane_dragged_to_the_middle_floats_at_the_pointer(cx: &mut TestAppContext) {
    let handle = open_float(
        cx,
        "[Common]\r\neOutlineDockSide=3\r\nxyOutlineDock=0,0,300,0\r\n",
    );
    let (area, _) = {
        let _ = rects(cx, handle);
        let vcx = visual(cx, handle);
        (
            vcx.debug_bounds("outline-text").unwrap(),
            vcx.debug_bounds("outline-pane-box").unwrap(),
        )
    };
    let target = point(area.left() + px(300.), area.top() + px(200.));
    drag_title(cx, handle, move |_| target, true, Modifiers::default());
    assert_eq!(current_place(cx, handle).side, DockSide::Float);
    assert!(current_place(cx, handle).disp);
    let (_, float) = float_bounds(cx, handle);
    // the pointer holds the title: two captions in from the left, half a caption down
    assert!(near(float.left(), target.x - px(48.)), "{float:?}");
    assert!(near(float.top(), target.y - px(12.)), "{float:?}");
}

#[gpui::test]
fn a_pane_that_cannot_dock_is_moved_and_not_docked(cx: &mut TestAppContext) {
    let handle = open_float(cx, "[Common]\r\neOutlineDockSide=-1\r\n");
    let (area, before) = float_bounds(cx, handle);
    let target = point(area.left() + px(10.), area.center().y);
    drag_title(cx, handle, move |_| target, true, Modifiers::default());
    assert_eq!(current_place(cx, handle).side, DockSide::Undockable);
    let (_, after) = float_bounds(cx, handle);
    assert!(after.left() < before.left());
}

/// Two tabs: the first has the file `a.cpp`, the second an empty document of the basic type, and the
/// settings of the types are `text` with the types of the two put in ({a} and {b}). The first tab is
/// active.
fn two_tabs(cx: &mut TestAppContext, text: &str) -> (AnyWindowHandle, usize, usize) {
    let handle = window(cx, CPP, "a.cpp");
    let a = active_view(cx, handle).read_with(cx, |v, _| v.view_settings().type_index);
    run(cx, handle, "F_FILENEW");
    let b = active_view(cx, handle).read_with(cx, |v, _| v.view_settings().type_index);
    run(cx, handle, "F_PREVWINDOW");
    assert_ne!(a, b);
    let text = text
        .replace("{a}", &a.to_string())
        .replace("{b}", &b.to_string());
    cx.update(|cx| {
        cx.set_global(OutlineDock::from_profile(&Profile::from_bytes(
            text.as_bytes(),
        )))
    });
    (handle, a, b)
}

fn switch(cx: &mut TestAppContext, handle: AnyWindowHandle, function: &str) {
    run(cx, handle, function);
    cx.run_until_parked();
    // the window is drawn again after the tab changes: the layout is applied then
    cx.run_until_parked();
}

#[gpui::test]
fn a_tab_whose_type_has_the_pane_closed_closes_it_and_the_tab_back_opens_it(
    cx: &mut TestAppContext,
) {
    let (handle, a, _) = two_tabs(
        cx,
        "[Common]\r\nnOutlineDockSet=1\r\n[Types({a})]\r\nbOutlineDockDisp=1\r\neOutlineDockSide=3\r\n\
         [Types({b})]\r\nbOutlineDockDisp=0\r\neOutlineDockSide=3\r\n",
    );
    run(cx, handle, "F_OUTLINE");
    assert!(pane(cx, handle).is_some());
    switch(cx, handle, "F_NEXTWINDOW");
    assert!(
        pane(cx, handle).is_none(),
        "the type of the tab has it closed"
    );
    // it is not forgotten for the type it was open for
    assert!(dock_now(cx).type_outline(a).place.disp);
    switch(cx, handle, "F_PREVWINDOW");
    assert!(
        pane(cx, handle).is_some(),
        "the type of the tab has it open"
    );
    assert_eq!(current_place(cx, handle).side, DockSide::Right);
}

#[gpui::test]
fn the_places_of_the_types_move_the_pane_between_the_sides_and_a_floating_one_is_not_opened(
    cx: &mut TestAppContext,
) {
    let (handle, _, _) = two_tabs(
        cx,
        "[Common]\r\nnOutlineDockSet=1\r\n[Types({a})]\r\nbOutlineDockDisp=1\r\neOutlineDockSide=1\r\n\
         [Types({b})]\r\nbOutlineDockDisp=1\r\neOutlineDockSide=3\r\n",
    );
    run(cx, handle, "F_OUTLINE");
    assert_eq!(current_place(cx, handle).side, DockSide::Left);
    switch(cx, handle, "F_NEXTWINDOW");
    assert_eq!(current_place(cx, handle).side, DockSide::Right);
    switch(cx, handle, "F_PREVWINDOW");
    assert_eq!(current_place(cx, handle).side, DockSide::Left);
}

#[gpui::test]
fn a_docked_pane_closes_for_a_type_that_floats_and_a_floating_one_stays_for_one(
    cx: &mut TestAppContext,
) {
    let (handle, a, b) = two_tabs(
        cx,
        "[Common]\r\nnOutlineDockSet=1\r\n[Types({a})]\r\nbOutlineDockDisp=1\r\neOutlineDockSide=3\r\n\
         [Types({b})]\r\nbOutlineDockDisp=1\r\neOutlineDockSide=0\r\n",
    );
    run(cx, handle, "F_OUTLINE");
    assert_eq!(current_place(cx, handle).side, DockSide::Right);
    // docked to floating: closed, and a floating pane is not opened for another file
    switch(cx, handle, "F_NEXTWINDOW");
    assert!(pane(cx, handle).is_none());
    // and a floating pane is not opened by the tab back either: it is docked there, so it is
    switch(cx, handle, "F_PREVWINDOW");
    assert!(pane(cx, handle).is_some());
    // floating for floating is the same pane
    let text = format!(
        "[Common]\r\nnOutlineDockSet=1\r\n[Types({a})]\r\nbOutlineDockDisp=1\r\neOutlineDockSide=0\r\n\
         [Types({b})]\r\nbOutlineDockDisp=0\r\neOutlineDockSide=0\r\n"
    );
    cx.update(|cx| {
        cx.set_global(OutlineDock::from_profile(&Profile::from_bytes(
            text.as_bytes(),
        )))
    });
    pane_command(cx, handle, MenuCommand::Side(DockSide::Float));
    let before = pane(cx, handle).unwrap().entity_id();
    switch(cx, handle, "F_NEXTWINDOW");
    assert_eq!(pane(cx, handle).map(|p| p.entity_id()), Some(before));
}
