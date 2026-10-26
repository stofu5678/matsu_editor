//! Tests of the look of the tab bar: the rows, the widths, the cut titles, the place and the icons
//! (`bTabMultiLine`, `nTabMaxWidth` and the others, `eTabPosition`, `bDispTabIcon`), and a tab
//! dragged out of the window (`CTabWnd::SeparateGroup`).

use gpui::{
    AnyWindowHandle, Bounds, Modifiers, MouseButton, Pixels, TestAppContext, point, px, size,
};

use super::icon::TabIcon;
use super::settings::{TabOptions, TabPosition};
use super::tests::{active_view, tab_count, tab_mode, tabs, visual, window, windows, workspace};

const WIDTH: f32 = 900.;
const HEIGHT: f32 = 600.;

fn options(cx: &mut TestAppContext, options: TabOptions) {
    cx.update(|app| app.set_global(options));
}

/// Draws the window at 900 x 600 and returns the boxes of the tabs.
fn draw(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Vec<Bounds<Pixels>> {
    let vcx = visual(cx, handle);
    vcx.simulate_resize(size(px(WIDTH), px(HEIGHT)));
    vcx.run_until_parked();
    vcx.simulate_resize(size(px(WIDTH), px(HEIGHT)));
    vcx.run_until_parked();
    let count = tab_count(cx, handle);
    let vcx = visual(cx, handle);
    (0..count)
        .map(|i| {
            let selector: &'static str = Box::leak(format!("tab-{i}").into_boxed_str());
            vcx.debug_bounds(selector)
                .unwrap_or_else(|| panic!("tab {i} is not drawn"))
        })
        .collect()
}

fn strip(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Bounds<Pixels> {
    visual(cx, handle)
        .debug_bounds("tab-strip")
        .expect("the tab strip is drawn")
}

/// The distinct tops of the tabs: one per row.
fn rows(boxes: &[Bounds<Pixels>]) -> Vec<f32> {
    let mut tops: Vec<f32> = Vec::new();
    for b in boxes {
        let top: f32 = b.origin.y.into();
        if !tops.iter().any(|t| (t - top).abs() < 1.) {
            tops.push(top);
        }
    }
    tops
}

fn width(b: &Bounds<Pixels>) -> f32 {
    b.size.width.into()
}

#[gpui::test]
fn the_tabs_are_in_one_row_as_wide_as_their_text_by_default(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    let boxes = draw(cx, handle);
    assert_eq!(rows(&boxes).len(), 1);
    // nTabMinWidth 60 .. nTabMaxWidth 200 (the defaults of Sakura).
    for b in &boxes {
        assert!((60. ..=200.).contains(&width(b)), "{}", width(b));
    }
}

#[gpui::test]
fn tabs_that_do_not_fit_shrink_to_the_minimum_and_stay_in_one_row(cx: &mut TestAppContext) {
    // CTabWnd::Refresh: the width is (bar - 8) / count, but not below nTabMinWidth.
    let handle = tabs(cx, 30);
    options(
        cx,
        TabOptions {
            min_width: 50,
            ..Default::default()
        },
    );
    let boxes = draw(cx, handle);
    assert_eq!(rows(&boxes).len(), 1, "a single row scrolls");
    for b in &boxes {
        assert!((width(b) - 50.).abs() < 1., "{}", width(b));
    }
    // The strip keeps the width of the bar; the tabs overflow it.
    assert!(width(&strip(cx, handle)) <= WIDTH);
}

#[gpui::test]
fn several_rows_wrap_the_tabs_and_each_row_fills_the_width(cx: &mut TestAppContext) {
    // TCS_MULTILINE with nTabMinWidthOnMulti 100: 30 tabs make 4 rows of at most 8 or 9.
    let handle = tabs(cx, 30);
    options(
        cx,
        TabOptions {
            multi_line: true,
            ..Default::default()
        },
    );
    let boxes = draw(cx, handle);
    let tops = rows(&boxes);
    assert!(tops.len() >= 3, "{tops:?}");
    let area = strip(cx, handle);
    let (left, right): (f32, f32) = (area.left().into(), area.right().into());
    for b in &boxes {
        assert!(width(b) >= 99., "{}", width(b));
        assert!(f32::from(b.right()) <= right + 1.);
        assert!(f32::from(b.left()) >= left - 1.);
    }
    // The tabs of a full row reach both sides (the tab control stretches them).
    let first_row: Vec<_> = boxes
        .iter()
        .filter(|b| (f32::from(b.origin.y) - tops[0]).abs() < 1.)
        .collect();
    let end = first_row
        .iter()
        .map(|b| f32::from(b.right()))
        .fold(0., f32::max);
    assert!((end - right).abs() < 2., "{end} {right}");
    // The bar is taller than with one row.
    assert!(f32::from(area.size.height) > 2. * (f32::from(boxes[0].size.height)));
}

#[gpui::test]
fn few_tabs_stay_in_one_row_with_several_rows_allowed(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    options(
        cx,
        TabOptions {
            multi_line: true,
            ..Default::default()
        },
    );
    let boxes = draw(cx, handle);
    assert_eq!(rows(&boxes).len(), 1);
    // They are stretched to fill the row, up to the maximum.
    for b in &boxes {
        assert!((width(b) - 200.).abs() < 1., "{}", width(b));
    }
}

#[gpui::test]
fn equal_width_tabs_share_the_row_between_the_limits(cx: &mut TestAppContext) {
    // bSameTabWidth: (900 - 8) / 3 is above nTabMaxWidth, so 200.
    let handle = tabs(cx, 3);
    options(
        cx,
        TabOptions {
            same_width: true,
            ..Default::default()
        },
    );
    let boxes = draw(cx, handle);
    for b in &boxes {
        assert!((width(b) - 200.).abs() < 1., "{}", width(b));
    }
    // A narrower maximum, then a wider one.
    options(
        cx,
        TabOptions {
            same_width: true,
            max_width: 120,
            ..Default::default()
        },
    );
    let boxes = draw(cx, handle);
    for b in &boxes {
        assert!((width(b) - 120.).abs() < 1., "{}", width(b));
    }
    // With 8 tabs of a 900 pixel bar the share is 111 pixels.
    let handle = tabs(cx, 8);
    options(
        cx,
        TabOptions {
            same_width: true,
            ..Default::default()
        },
    );
    let boxes = draw(cx, handle);
    let first = width(&boxes[0]);
    assert!((100. ..=200.).contains(&first), "{first}");
    for b in &boxes {
        assert!((width(b) - first).abs() < 1., "equal widths");
    }
}

#[gpui::test]
fn a_title_longer_than_the_maximum_is_cut_to_the_tab(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    options(
        cx,
        TabOptions {
            max_width: 40,
            min_width: 10,
            ..Default::default()
        },
    );
    let boxes = draw(cx, handle);
    for b in &boxes {
        assert!((width(b) - 40.).abs() < 1., "{}", width(b));
    }
    // The text stays inside its tab (it ends with an ellipsis there).
    let vcx = visual(cx, handle);
    let caption = vcx.debug_bounds("tab-caption-0").expect("caption");
    assert!(caption.right() <= boxes[0].right());
    assert!(caption.left() >= boxes[0].left());
}

#[gpui::test]
fn the_bar_is_at_the_top_or_at_the_bottom(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    draw(cx, handle);
    let top: f32 = strip(cx, handle).origin.y.into();
    assert!(top < 100., "{top}");
    options(
        cx,
        TabOptions {
            position: TabPosition::Bottom,
            ..Default::default()
        },
    );
    draw(cx, handle);
    let bottom: f32 = strip(cx, handle).origin.y.into();
    assert!(bottom > HEIGHT / 2., "{bottom}");
    // The bar is above the status bar: it ends before the window does.
    assert!(bottom < HEIGHT);
}

#[gpui::test]
fn the_icon_makes_the_tab_wider(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    let plain = width(&draw(cx, handle)[0]);
    options(
        cx,
        TabOptions {
            disp_icon: true,
            ..Default::default()
        },
    );
    let with_icon = width(&draw(cx, handle)[0]);
    assert!(with_icon >= plain + 10., "{plain} -> {with_icon}");
}

#[gpui::test]
fn an_untitled_document_has_the_icon_of_the_application(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    let icon =
        workspace(cx, handle).read_with(cx, |ws, cx| ws.tab_panel(0).map(|panel| panel.icon(cx)));
    assert_eq!(icon, Some(TabIcon::App));
}

// ---- dragging a tab out of the window ----

/// Drags tab `from` of the window to `to` (in window coordinates) and lets go there.
fn drag(cx: &mut TestAppContext, handle: AnyWindowHandle, from: usize, to: gpui::Point<Pixels>) {
    let boxes = draw(cx, handle);
    let start = boxes[from].center();
    let vcx = visual(cx, handle);
    let none = Modifiers::default();
    vcx.simulate_mouse_down(start, MouseButton::Left, none);
    vcx.simulate_mouse_move(point(start.x + px(10.), start.y), MouseButton::Left, none);
    vcx.simulate_mouse_move(to, MouseButton::Left, none);
    vcx.simulate_mouse_up(to, MouseButton::Left, none);
    cx.run_until_parked();
}

fn outside() -> gpui::Point<Pixels> {
    point(px(WIDTH + 400.), px(HEIGHT + 400.))
}

#[gpui::test]
fn a_tab_dragged_outside_the_window_makes_a_window(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    let moved = workspace(cx, handle).read_with(cx, |ws, _| ws.tab_editor(1).unwrap());
    drag(cx, handle, 1, outside());
    assert_eq!(windows(cx), 2);
    assert_eq!(tab_count(cx, handle), 2);
    let new_handle = cx
        .windows()
        .into_iter()
        .find(|h| *h != handle)
        .expect("a new window");
    assert_eq!(tab_count(cx, new_handle), 1);
    assert_eq!(
        active_view(cx, new_handle),
        moved,
        "the tab moved, not a copy"
    );
}

#[gpui::test]
fn the_new_window_is_where_the_tab_was_dropped(cx: &mut TestAppContext) {
    let handle = tabs(cx, 2);
    let boxes = draw(cx, handle);
    let source = handle.update(cx, |_, window, _| window.bounds()).unwrap();
    let start = boxes[1].center();
    let to = point(px(WIDTH + 300.), px(HEIGHT + 120.));
    drag(cx, handle, 1, to);
    let new_handle = cx.windows().into_iter().find(|h| *h != handle).unwrap();
    let placed = new_handle
        .update(cx, |_, window, _| window.bounds())
        .unwrap();
    // The pointer moved by (to - start) while the drag began 10 pixels from the press.
    let expect_x = f32::from(source.origin.x) + f32::from(to.x) - f32::from(start.x) - 10.;
    let x: f32 = placed.origin.x.into();
    assert!((x - expect_x).abs() < 2., "{x} {expect_x}");
}

#[gpui::test]
fn a_tab_dropped_inside_the_window_stays(cx: &mut TestAppContext) {
    let handle = tabs(cx, 3);
    drag(cx, handle, 1, point(px(300.), px(300.)));
    assert_eq!(windows(cx), 1);
    assert_eq!(tab_count(cx, handle), 3);
}

#[gpui::test]
fn the_only_tab_does_not_leave_its_window(cx: &mut TestAppContext) {
    let handle = window(cx, true);
    drag(cx, handle, 0, outside());
    assert_eq!(windows(cx), 1);
    assert_eq!(tab_count(cx, handle), 1);
}

#[gpui::test]
fn a_tab_is_not_dragged_out_without_the_tab_mode(cx: &mut TestAppContext) {
    // Sakura separates only with the bar shown and the windows grouped.
    let handle = tabs(cx, 3);
    tab_mode(cx, true, false);
    drag(cx, handle, 1, outside());
    assert_eq!(windows(cx), 1);
    assert_eq!(tab_count(cx, handle), 3);
}

#[gpui::test]
fn a_tab_dropped_over_another_window_joins_its_tabs(cx: &mut TestAppContext) {
    use super::tests::run;
    let first = tabs(cx, 2);
    run(cx, first, "F_FILENEW_NEWWINDOW");
    let second = cx.windows().into_iter().find(|h| *h != first).unwrap();
    // A point inside the second window's area and outside the first window.
    let a = first.update(cx, |_, w, _| w.bounds()).unwrap();
    let b = second.update(cx, |_, w, _| w.bounds()).unwrap();
    let screen = point(b.right() - px(5.), b.bottom() - px(5.));
    assert!(b.contains(&screen) && !a.contains(&screen), "{a:?} {b:?}");
    let to = screen - a.origin;
    let moved = workspace(cx, first).read_with(cx, |ws, _| ws.tab_editor(1).unwrap());
    drag(cx, first, 1, point(to.x, to.y));
    assert_eq!(windows(cx), 2, "no third window");
    assert_eq!(tab_count(cx, first), 1);
    assert_eq!(tab_count(cx, second), 2);
    assert_eq!(active_view(cx, second), moved);
}
