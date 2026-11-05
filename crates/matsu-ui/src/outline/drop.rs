//! Docking the pane by dragging its title: where the pane goes when it is let go.
//!
//! Sakura: `CDlgFuncList::GetDropRect` (the place and the rectangle for a drop), `Track` (the loop
//! that draws the rectangle while the title is dragged: a frame of 8 pixels for a floating place,
//! 4 for a docked one; Ctrl forces the floating place; Esc and the right button cancel) and
//! `OnNcLButtonDown` (a floating pane that cannot dock, `DOCKSIDE_UNDOCKABLE`, is not tracked).
//!
//! The functions work on plain numbers in the coordinates of the window, so the calculation is the
//! one of `GetDropRect` and nothing of GPUI.

use super::dock::{DOCK_MIN_SIZE, DockPlace, DockSide};

/// `SM_CXCURSOR`: how near to an edge of the area the pointer has to be to dock there.
pub(crate) const DOCK_EDGE: f32 = 32.0;
/// `SM_CXDRAG`: how far the pointer has to move before the drag begins.
pub(crate) const DRAG_START: f32 = 4.0;
/// The frame of a floating place (`sizeFull`).
pub(crate) const FRAME_FLOAT: f32 = 8.0;
/// The frame of a docked place (`sizeHalf`).
pub(crate) const FRAME_DOCK: f32 = 4.0;
/// `SM_CYCAPTION`: the pane that comes down from a docked place is held at its title.
pub(crate) const CAPTION: f32 = 24.0;
/// `SM_CXMIN` and `SM_CYMIN`: the least size of a floating pane that is brought back.
pub(crate) const FLOAT_MIN: (f32, f32) = (136.0, 39.0);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Point {
    pub x: f32,
    pub y: f32,
}

/// A rectangle in the coordinates of the window.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Rect {
    pub(crate) fn new(left: f32, top: f32, width: f32, height: f32) -> Self {
        Self {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }
    }

    pub(crate) fn width(&self) -> f32 {
        self.right - self.left
    }

    pub(crate) fn height(&self) -> f32 {
        self.bottom - self.top
    }

    /// `PtInRect`: the right and the bottom edge are not inside.
    pub(crate) fn contains(&self, p: Point) -> bool {
        self.left <= p.x && p.x < self.right && self.top <= p.y && p.y < self.bottom
    }

    fn offset(self, dx: f32, dy: f32) -> Self {
        Self {
            left: self.left + dx,
            top: self.top + dy,
            right: self.right + dx,
            bottom: self.bottom + dy,
        }
    }
}

/// What `GetDropRect` knows about the pane and the window.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DropContext {
    /// `GetDockSpaceRect`: the part of the window that the pane and the text share.
    pub area: Rect,
    /// Where the pane is now.
    pub pane: Rect,
    /// The side the pane is drawn at (`GetDockSide`).
    pub side: DockSide,
    /// The sizes kept for the four sides (`ProfDockLeft` and the others).
    pub kept: DockPlace,
    /// The size a pane that comes down from a docked place gets: the remembered one, or the
    /// default (`bRememberOutlineWindowPos` and `m_widthOutlineWindow` of the shared data).
    pub float_size: (f32, f32),
    /// Whether the size above is the remembered one, which is not smaller than [`FLOAT_MIN`].
    pub float_size_remembered: bool,
}

/// `CDockStretch::GetIdealStretch`: the size kept (0: a third of the area), kept inside the area
/// with [`DOCK_MIN_SIZE`] of the text left.
pub(crate) fn ideal_stretch(kept: i32, max: f32) -> f32 {
    let size = if kept <= 0 {
        (max / 3.0).trunc()
    } else {
        kept as f32
    };
    size.min(max - DOCK_MIN_SIZE).max(DOCK_MIN_SIZE)
}

/// `GetDropRect`: the place (and the rectangle) the pane takes when the pointer, which took hold
/// of the title at `grab`, is let go at `drop`. `force_float` is the Ctrl key.
pub(crate) fn drop_rect(
    ctx: &DropContext,
    grab: Point,
    drop: Point,
    force_float: bool,
) -> (DockSide, Rect) {
    let docked = ctx.side.is_docked();
    // the place it has: no move
    if docked && !force_float && ctx.pane.contains(drop) {
        return (ctx.side, ctx.pane);
    }

    // a place at an edge of the area
    let mut dock = ctx.area;
    if !force_float && dock.contains(drop) {
        let (w, h) = (dock.width(), dock.height());
        let left = ideal_stretch(ctx.kept.left, w);
        let top = ideal_stretch(ctx.kept.top, h);
        let right = ideal_stretch(ctx.kept.right, w);
        let bottom = ideal_stretch(ctx.kept.bottom, h);
        let side = if drop.x - dock.left < DOCK_EDGE {
            dock.right = dock.left + left;
            Some(DockSide::Left)
        } else if dock.right - drop.x < DOCK_EDGE {
            dock.left = dock.right - right;
            Some(DockSide::Right)
        } else if drop.y - dock.top < DOCK_EDGE {
            dock.bottom = dock.top + top;
            Some(DockSide::Top)
        } else if dock.bottom - drop.y < DOCK_EDGE {
            dock.top = dock.bottom - bottom;
            Some(DockSide::Bottom)
        } else {
            None
        };
        if let Some(side) = side {
            return (side, dock);
        }
    }

    // a floating place
    if !docked {
        // it is moved by what the pointer moved
        (
            DockSide::Float,
            ctx.pane.offset(drop.x - grab.x, drop.y - grab.y),
        )
    } else {
        // it comes down from a docked place, and the pointer holds it at the title
        let (mut w, mut h) = ctx.float_size;
        if ctx.float_size_remembered {
            w = w.max(FLOAT_MIN.0);
            h = h.max(FLOAT_MIN.1);
        }
        (
            DockSide::Float,
            Rect::new(drop.x - CAPTION * 2.0, drop.y - CAPTION / 2.0, w, h),
        )
    }
}
