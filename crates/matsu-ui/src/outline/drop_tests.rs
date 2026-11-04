//! Tests of the calculation of `CDlgFuncList::GetDropRect`: where the pane goes when the title is
//! let go.

use super::dock::{DockPlace, DockSide};
use super::drop::{CAPTION, DOCK_EDGE, DropContext, Point, Rect, drop_rect, ideal_stretch};

fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}

/// An area of 1000 x 600 at (100, 50), a pane at `side` with the sizes kept in `kept`.
fn ctx(side: DockSide, kept: DockPlace, pane: Rect) -> DropContext {
    DropContext {
        area: Rect::new(100.0, 50.0, 1000.0, 600.0),
        pane,
        side,
        kept,
        float_size: (400.0, 420.0),
        float_size_remembered: false,
    }
}

fn floating() -> DropContext {
    ctx(
        DockSide::Float,
        DockPlace::default(),
        Rect::new(500.0, 100.0, 300.0, 200.0),
    )
}

fn docked_right() -> DropContext {
    ctx(
        DockSide::Right,
        DockPlace {
            right: 250,
            ..DockPlace::default()
        },
        Rect::new(850.0, 50.0, 250.0, 600.0),
    )
}

#[test]
fn the_ideal_stretch_is_a_third_when_nothing_is_kept_and_keeps_the_least_of_the_text() {
    assert_eq!(ideal_stretch(0, 900.0), 300.0);
    assert_eq!(ideal_stretch(200, 900.0), 200.0);
    // DOCK_MIN_SIZE (60) on both sides
    assert_eq!(ideal_stretch(5, 900.0), 60.0);
    assert_eq!(ideal_stretch(5000, 900.0), 840.0);
    // the integer division of Sakura
    assert_eq!(ideal_stretch(0, 100.0), 60.0);
    assert_eq!(ideal_stretch(0, 1001.0), 333.0);
}

#[test]
fn near_an_edge_of_the_area_the_pane_docks_there_with_the_size_kept_for_that_side() {
    let c = ctx(
        DockSide::Float,
        DockPlace {
            left: 150,
            top: 90,
            right: 0,
            bottom: 120,
            ..DockPlace::default()
        },
        Rect::new(500.0, 100.0, 300.0, 200.0),
    );
    let grab = p(520.0, 110.0);
    // the left edge: the width kept, the whole height
    let (side, r) = drop_rect(&c, grab, p(110.0, 300.0), false);
    assert_eq!(side, DockSide::Left);
    assert_eq!(r, Rect::new(100.0, 50.0, 150.0, 600.0));
    // the right edge: nothing kept, a third of the area
    let (side, r) = drop_rect(&c, grab, p(1090.0, 300.0), false);
    assert_eq!(side, DockSide::Right);
    assert_eq!(r, Rect::new(100.0 + 1000.0 - 333.0, 50.0, 333.0, 600.0));
    // the top edge
    let (side, r) = drop_rect(&c, grab, p(600.0, 60.0), false);
    assert_eq!(side, DockSide::Top);
    assert_eq!(r, Rect::new(100.0, 50.0, 1000.0, 90.0));
    // the bottom edge
    let (side, r) = drop_rect(&c, grab, p(600.0, 640.0), false);
    assert_eq!(side, DockSide::Bottom);
    assert_eq!(r, Rect::new(100.0, 50.0 + 600.0 - 120.0, 1000.0, 120.0));
}

#[test]
fn the_edge_is_the_width_of_a_cursor_and_the_left_comes_before_the_top() {
    let c = floating();
    let grab = p(520.0, 110.0);
    // 31 pixels from the left edge docks, 32 does not (`< nDock`)
    assert_eq!(
        drop_rect(&c, grab, p(100.0 + DOCK_EDGE - 1.0, 300.0), false).0,
        DockSide::Left
    );
    assert_eq!(
        drop_rect(&c, grab, p(100.0 + DOCK_EDGE, 300.0), false).0,
        DockSide::Float
    );
    // the corner: the left wins over the top, and the right over the top and the bottom
    assert_eq!(drop_rect(&c, grab, p(101.0, 51.0), false).0, DockSide::Left);
    assert_eq!(
        drop_rect(&c, grab, p(1099.0, 649.0), false).0,
        DockSide::Right
    );
}

#[test]
fn outside_the_area_there_is_no_docking() {
    let c = floating();
    let grab = p(520.0, 110.0);
    // above the area (the menu bar), to the left of it
    assert_eq!(
        drop_rect(&c, grab, p(600.0, 20.0), false).0,
        DockSide::Float
    );
    assert_eq!(
        drop_rect(&c, grab, p(90.0, 300.0), false).0,
        DockSide::Float
    );
    // the right and the bottom edge of the rectangle are outside (`PtInRect`)
    assert_eq!(
        drop_rect(&c, grab, p(1100.0, 300.0), false).0,
        DockSide::Float
    );
}

#[test]
fn a_floating_pane_moves_by_what_the_pointer_moved() {
    let c = floating();
    let (side, r) = drop_rect(&c, p(520.0, 110.0), p(620.0, 160.0), false);
    assert_eq!(side, DockSide::Float);
    assert_eq!(r, Rect::new(600.0, 150.0, 300.0, 200.0));
}

#[test]
fn ctrl_keeps_the_pane_floating_also_at_an_edge() {
    let c = floating();
    let (side, r) = drop_rect(&c, p(520.0, 110.0), p(110.0, 300.0), true);
    assert_eq!(side, DockSide::Float);
    assert_eq!(r, Rect::new(90.0, 290.0, 300.0, 200.0));
}

#[test]
fn a_docked_pane_let_go_on_itself_does_not_move() {
    let c = docked_right();
    let (side, r) = drop_rect(&c, p(900.0, 60.0), p(950.0, 200.0), false);
    assert_eq!((side, r), (DockSide::Right, c.pane));
    // Ctrl takes it down from there
    let (side, _) = drop_rect(&c, p(900.0, 60.0), p(950.0, 200.0), true);
    assert_eq!(side, DockSide::Float);
}

#[test]
fn a_docked_pane_goes_to_another_edge() {
    let c = docked_right();
    let (side, r) = drop_rect(&c, p(900.0, 60.0), p(105.0, 300.0), false);
    assert_eq!(side, DockSide::Left);
    assert_eq!(r.width(), 333.0);
}

#[test]
fn a_docked_pane_comes_down_held_at_its_title_with_the_default_size() {
    let c = docked_right();
    let (side, r) = drop_rect(&c, p(900.0, 60.0), p(500.0, 300.0), false);
    assert_eq!(side, DockSide::Float);
    assert_eq!(
        r,
        Rect::new(500.0 - CAPTION * 2.0, 300.0 - CAPTION / 2.0, 400.0, 420.0)
    );
}

#[test]
fn a_remembered_size_is_not_below_the_least_of_a_window() {
    let mut c = docked_right();
    c.float_size = (50.0, 20.0);
    c.float_size_remembered = true;
    let (_, r) = drop_rect(&c, p(900.0, 60.0), p(500.0, 300.0), false);
    assert_eq!((r.width(), r.height()), (136.0, 39.0));
    // a default size is left as it is
    c.float_size_remembered = false;
    let (_, r) = drop_rect(&c, p(900.0, 60.0), p(500.0, 300.0), false);
    assert_eq!((r.width(), r.height()), (50.0, 20.0));
}
