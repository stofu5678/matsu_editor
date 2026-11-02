//! The horizontal scroll bar (水平スクロールバー, `bScrollBarHorz`).
//!
//! Sakura: `CEditView::CreateScrollBar`, `AdjustScrollBars`, `OnHScroll`, `ScrollAtH` and
//! `GetRightEdgeForScrollBar` (`sakura_core/view/CEditView_Scroll.cpp`). A window re-creates the
//! bars of its views when the setting changes (`CEditWnd::OnChangeSetting`,
//! `sakura_core/window/CEditWnd.cpp`); the minimap has none (`!m_bMiniMap`).
//!
//! * The range is the right edge of the text in columns: the wrap column and the room for the
//!   wrap mark and the hanging punctuation (`GetWrapOverhang`); with no wrap, the widest line, the
//!   selection and the caret of the free cursor mode, and three columns, but not more than the
//!   largest wrap column.
//! * The thumb is as wide as the columns in view are of the range, and the bar is disabled (drawn
//!   without a thumb) when everything is in view, which also scrolls the view back to the left.
//! * The arrows move 4 columns, a click on the track moves a page (the columns in view), and the
//!   thumb sets the left column, in whole columns.
//! * The view can scroll `overhang` columns past what the bar can show (`ScrollAtH`).
//!
//! The bar is painted by the element of the view like the vertical one, with the parts in
//! `crate::geometry`; this file is the setting and the events.

use std::time::Duration;

use gpui::{App, Context, Global};
use matsu_layout::LayoutConfig;

use super::{Drag, EditorView};
use crate::geometry::{HSCROLL_ARROW, HSCROLL_LINE_COLUMNS, HScrollbar, max_left_col};
use crate::settings::view_settings::WrapMethod;

/// `m_bScrollBarHorz`: whether the views have a horizontal scroll bar. Sakura's default is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShowHScrollbar(pub bool);

impl Global for ShowHScrollbar {}

/// Whether the views have a horizontal scroll bar now.
pub(crate) fn shown(cx: &App) -> bool {
    cx.try_global::<ShowHScrollbar>().is_none_or(|s| s.0)
}

/// Takes the setting; views repaint with or without the bar.
pub(crate) fn set_shown(cx: &mut App, on: bool) {
    cx.set_global(ShowHScrollbar(on));
}

/// The room right of the text that the free cursor and the selection get in the range when the
/// text does not wrap (`layout3Keta`).
const NO_WRAP_MARGIN: u32 = 3;

/// How long a frame may spend on the width of the text.
const WIDTH_BUDGET: Duration = Duration::from_millis(3);

/// `GetRightEdgeForScrollBar`: the right end of what the bar covers, in columns.
///
/// `max_columns` is the wrap column (the largest one when the text does not wrap), `overhang` the
/// room right of it. `unwrapped_edge` is the right end of the text, the selection and the free
/// caret, and only the text that does not wrap has one.
pub(crate) fn right_edge(max_columns: u32, overhang: u32, unwrapped_edge: Option<u32>) -> u32 {
    let width = max_columns + overhang;
    match unwrapped_edge {
        Some(edge) => (edge + NO_WRAP_MARGIN).min(width),
        None => width,
    }
}

/// The widest line of the document in columns, found a piece per frame (`m_nTextWidth`).
///
/// An edit or a new layout setting starts the count again. Until it ends, the width is the larger
/// of the last complete count and what the new count has found, so that the bar does not jump back
/// and forth while a large document is gone through.
#[derive(Default)]
pub(crate) struct TextWidth {
    /// The document version and the layout settings the count belongs to.
    key: Option<(u64, LayoutConfig)>,
    /// The next line to look at.
    next: usize,
    /// The widest so far in this count.
    running: u32,
    /// The result of the last complete count.
    settled: u32,
    done: bool,
}

impl TextWidth {
    /// The widest end column known now.
    pub(crate) fn value(&self) -> u32 {
        self.settled.max(self.running)
    }

    /// The count has gone through the whole document.
    pub(crate) fn is_complete(&self) -> bool {
        self.done
    }

    /// Goes on with the count for about `budget`, after starting it again if `version` or `cfg`
    /// differ from the last call. Returns whether it is complete.
    fn advance(
        &mut self,
        layout: &matsu_layout::DocumentLayout,
        doc: &matsu_core::buffer::Document,
        version: u64,
        budget: Duration,
    ) -> bool {
        let cfg = layout.config();
        if self
            .key
            .as_ref()
            .is_none_or(|(v, c)| *v != version || c != cfg)
        {
            self.key = Some((version, cfg.clone()));
            self.next = 0;
            self.running = 0;
            self.done = false;
        }
        if self.done {
            return true;
        }
        match layout.widest_columns(doc, self.next, budget) {
            Ok((widest, next)) => {
                self.running = self.running.max(widest);
                self.next = next;
                if next >= doc.line_count() {
                    self.settled = self.running;
                    self.done = true;
                }
            }
            // A document that cannot be read has no width to find.
            Err(_) => self.done = true,
        }
        self.done
    }
}

/// Where the last paint put the bar, in window coordinates.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BarPaint {
    pub left: f32,
    pub top: f32,
    /// The width of the whole bar, the arrows included.
    pub width: f32,
}

impl EditorView {
    /// The columns in view (`m_nViewColNum`).
    pub(crate) fn view_columns(&self) -> u32 {
        (self.metrics.view_width / self.metrics.cell_width)
            .floor()
            .max(0.0) as u32
    }

    /// `GetRightEdgeForScrollBar` for this view, as of the last paint.
    pub(crate) fn h_right_edge(&self) -> u32 {
        let wrapped = self.prefs.wrap_method != WrapMethod::None;
        let unwrapped = (!wrapped).then(|| {
            self.text_width
                .value()
                .max(self.widest_cols)
                .max(self.h_selection_edge)
        });
        right_edge(
            self.layout.config().max_columns(),
            self.prefs.settings.wrap_overhang,
            unwrapped,
        )
    }

    /// The farthest left column (`ScrollAtH`), in columns.
    pub(crate) fn h_max_left_col(&self) -> f32 {
        max_left_col(
            self.h_right_edge(),
            self.prefs.settings.wrap_overhang,
            self.view_columns(),
        )
    }

    /// The bar as it is now, if the last paint drew one.
    pub(crate) fn h_scrollbar(&self) -> Option<(HScrollbar, BarPaint)> {
        let paint = self.paint?.hbar?;
        let left_col = self.scroll_x / self.metrics.cell_width;
        Some((
            HScrollbar::new(
                self.h_right_edge(),
                self.view_columns(),
                left_col,
                paint.width,
            ),
            paint,
        ))
    }

    /// `ScrollAtH`: scrolls to the left column `col`, kept between the first and the farthest.
    pub(crate) fn scroll_to_col(&mut self, col: f32) {
        let col = col.clamp(0.0, self.h_max_left_col());
        self.scroll_x = col * self.metrics.cell_width;
    }

    /// Goes on with the width of the text, if the text does not wrap. Called before each frame;
    /// another frame is asked for while the count is not complete.
    pub(crate) fn advance_text_width(&mut self, window: &mut gpui::Window, cx: &App) {
        if self.prefs.wrap_method != WrapMethod::None {
            return;
        }
        let shared = self.document.read(cx);
        let version = shared.version();
        if !self
            .text_width
            .advance(&self.layout, shared.doc(), version, WIDTH_BUDGET)
        {
            window.request_animation_frame();
        }
    }

    /// `AdjustScrollBars`: when everything is in view the bar is disabled and the view is back at
    /// the left. Called by the paint; it waits for the width of the text to be complete, so that
    /// a document still being gone through does not lose its place.
    pub(crate) fn h_scroll_settle(&mut self) {
        let complete = self.prefs.wrap_method != WrapMethod::None || self.text_width.is_complete();
        if complete && self.view_columns() >= self.h_right_edge() {
            self.scroll_x = 0.0;
        }
    }

    /// A press at `window_x` on the bar.
    pub(crate) fn h_scroll_press(&mut self, window_x: f32, cx: &mut Context<Self>) {
        let Some((bar, paint)) = self.h_scrollbar() else {
            return;
        };
        let x = window_x - paint.left;
        let left_col = (self.scroll_x / self.metrics.cell_width).round();
        // `OnHScroll`: SB_LINELEFT / SB_LINERIGHT / SB_PAGELEFT / SB_PAGERIGHT.
        if x < HSCROLL_ARROW {
            self.scroll_to_col(left_col - HSCROLL_LINE_COLUMNS);
        } else if x >= paint.width - HSCROLL_ARROW {
            self.scroll_to_col(left_col + HSCROLL_LINE_COLUMNS);
        } else if !bar.enabled() {
            // Nothing to scroll.
        } else {
            let on_track = x - HSCROLL_ARROW;
            if on_track >= bar.thumb_left && on_track <= bar.thumb_left + bar.thumb_width {
                self.drag = Drag::HThumb {
                    start_x: window_x,
                    start_col: bar.pos,
                };
            } else if on_track < bar.thumb_left {
                self.scroll_to_col(left_col - bar.page as f32);
            } else {
                self.scroll_to_col(left_col + bar.page as f32);
            }
        }
        cx.notify();
    }

    /// The pointer moves while the thumb is held (`SB_THUMBTRACK`).
    pub(crate) fn h_scroll_drag(
        &mut self,
        x: f32,
        start_x: f32,
        start_col: f32,
        cx: &mut Context<Self>,
    ) {
        let Some((bar, _)) = self.h_scrollbar() else {
            return;
        };
        self.scroll_to_col(bar.col_for_drag(start_col, x - start_x));
        cx.notify();
    }
}
