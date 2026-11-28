//! The painting of the minimap. Only the rows in view are colored and shaped, so a large document
//! costs what the pane shows (a few hundred rows of a tiny font), not what the document has.
//!
//! The rows come from the pane the minimap follows (`EditorView::minimap_rows`): the same layout
//! and colors as the text, with the tab and space marks, the line numbers, the caret line, the odd
//! lines and the note lines left out as Sakura does for a minimap (`m_bMiniMap`).

use gpui::{
    App, Bounds, ContentMask, DispatchPhase, Entity, IntoElement, MouseMoveEvent, MouseUpEvent,
    Pixels, ShapedLine, SharedString, Styled, TextAlign, TextRun, Window, canvas, fill, point, px,
    rgb, size,
};
use matsu_layout::X;

use super::current;
use super::geometry::{self, LEFT_MARGIN};
use super::panel::{Drag, MiniMap};
use crate::geometry::{SCROLLBAR_WIDTH, XScale};
use crate::view::back_paint::{self, BackPaint, Frame};
use crate::view::minimap::MiniLook;

/// What the prepaint decided to draw.
pub(super) struct Painted {
    look: MiniLook,
    top: usize,
    row_height: f32,
    view_rows: usize,
    total_rows: usize,
    /// The rows the window shows.
    page: std::ops::Range<usize>,
    rows: Vec<PaintedRow>,
    /// The pixels of the layout's `X` in the minimap's font.
    scale: XScale,
    /// The background image of the type, drawn as the view draws it (not scaled).
    back: Option<BackPaint>,
}

struct PaintedRow {
    row: usize,
    pieces: Vec<(X, ShapedLine)>,
    backs: Vec<(X, X, u32)>,
}

/// The element: it fills its parent, and paints the minimap of `entity`.
pub(super) fn minimap_canvas(entity: Entity<MiniMap>) -> impl IntoElement {
    let for_paint = entity.clone();
    canvas(
        move |bounds, window, cx| prepaint(&entity, bounds, window, cx),
        move |bounds, painted, window, cx| paint(&for_paint, bounds, painted, window, cx),
    )
    .size_full()
}

fn run(font: &gpui::Font, len: usize, color: u32) -> TextRun {
    TextRun {
        len,
        font: font.clone(),
        color: rgb(color).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

fn prepaint(
    entity: &Entity<MiniMap>,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) -> Option<Painted> {
    let state = entity.read(cx);
    state.area.set(bounds);
    let source = state.source()?.clone();
    let settings = current(cx);
    let font_px = geometry::font_px(settings.font_size);
    let row_height = geometry::row_height(settings.font_size);
    let view_rows = geometry::view_rows(f32::from(bounds.size.height), row_height);
    let editor = source.read(cx);
    let total_rows = editor.minimap_total_rows();
    let top = state.top_row.min(geometry::max_top(total_rows, view_rows));

    let font = editor.text_font(cx).font();
    let shape = |window: &mut Window, text: &str, color: u32| -> ShapedLine {
        let text: SharedString = text.to_owned().into();
        let runs = [run(&font, text.len(), color)];
        window
            .text_system()
            .shape_line(text, px(font_px), &runs, None)
    };
    let look = editor.minimap_look();
    // One column is the advance of a half-width character, as the text of the window measures it
    // (the pieces are placed at whole columns, so they line up with the glyphs of the font). The
    // layout is the view's, in `X`: the minimap's font gives the pixels of an `X`.
    let cell = f32::from(shape(window, "0000000000", look.background).width()) / 10.0;
    let cell = if cell > 0.0 { cell } else { 1.0 };
    // Columns that can be seen; a piece is cut there, so that a very long line costs nothing.
    let text_width = f32::from(bounds.size.width) - LEFT_MARGIN - SCROLLBAR_WIDTH;
    let max_cols = (text_width / cell).ceil().max(1.0) as u32 + 1;

    let rows = editor
        .minimap_rows(top, view_rows + 1, max_cols, cx)
        .into_iter()
        .map(|row| PaintedRow {
            row: row.row,
            pieces: row
                .pieces
                .iter()
                .map(|piece| (piece.x, shape(window, &piece.text, piece.color)))
                .collect(),
            backs: row.backs,
        })
        .collect();
    Some(Painted {
        look,
        top,
        row_height,
        view_rows,
        total_rows,
        page: editor.minimap_page(),
        rows,
        scale: editor.x_scale_for(cell),
        back: editor.back_paint(),
    })
}

fn paint(
    entity: &Entity<MiniMap>,
    bounds: Bounds<Pixels>,
    painted: Option<Painted>,
    window: &mut Window,
    cx: &mut App,
) {
    window.paint_quad(fill(bounds, rgb(0xffffff)));
    let Some(p) = painted else {
        return;
    };
    // The top row may have been moved back into the range (the text got shorter).
    entity.update(cx, |minimap, _| minimap.top_row = p.top);

    window.paint_quad(fill(bounds, rgb(p.look.background)));
    // The background image, under the rows (`OnPaint`: `DrawBackImage` first). The minimap is a
    // `CEditView` with `m_bMiniMap` and nothing there leaves the image out: the picture keeps its
    // size in pixels, goes where the type says in the text area of the minimap (it starts after
    // `LEFT_MARGIN`, `CTextArea::DetectWidthOfLineNumberArea` gives 4; no ruler, no line numbers),
    // and scrolls with the rows of the minimap (`GetViewTopLine()` times the height of a row; the
    // minimap has no sideways scroll). The rows paint only what is not the background of the text,
    // so the image shows through them.
    back_paint::track(window, cx, p.back.as_ref());
    if let Some(back) = &p.back {
        let frame = back_frame(bounds, p.top, p.row_height, window.scale_factor());
        back_paint::paint(window, back, &frame, bounds);
    }
    let lh = px(p.row_height);
    let text_bounds = Bounds::new(
        bounds.origin,
        size(
            (bounds.size.width - px(SCROLLBAR_WIDTH)).max(px(0.)),
            bounds.size.height,
        ),
    );
    let x_of = |x: X| bounds.left() + px(LEFT_MARGIN + p.scale.px(x));
    window.with_content_mask(
        Some(ContentMask {
            bounds: text_bounds,
        }),
        |window| {
            for row in &p.rows {
                let y = bounds.top() + px((row.row - p.top) as f32 * p.row_height);
                // The rows the window shows have the color of `COLORIDX_PAGEVIEW` behind them.
                if let Some(back) = p.look.page_view.filter(|_| p.page.contains(&row.row)) {
                    window.paint_quad(fill(
                        Bounds::new(point(bounds.left(), y), size(text_bounds.size.width, lh)),
                        rgb(back),
                    ));
                }
                for &(from, to, color) in &row.backs {
                    window.paint_quad(fill(
                        Bounds::from_corners(point(x_of(from), y), point(x_of(to), y + lh)),
                        rgb(color),
                    ));
                }
                for (x, line) in &row.pieces {
                    line.paint(point(x_of(*x), y), lh, TextAlign::Left, None, window, cx)
                        .ok();
                }
            }
        },
    );

    // The scroll bar.
    let track = Bounds::new(
        point(bounds.right() - px(SCROLLBAR_WIDTH), bounds.top()),
        size(px(SCROLLBAR_WIDTH), bounds.size.height),
    );
    window.paint_quad(fill(track, rgb(0xf3f3f3)));
    let (thumb_top, thumb_height) = geometry::thumb(
        p.top,
        p.view_rows,
        p.total_rows,
        f32::from(bounds.size.height),
    );
    let dragging = matches!(entity.read(cx).drag, Drag::Thumb { .. });
    window.paint_quad(
        fill(
            Bounds::new(
                point(track.left() + px(3.), track.top() + px(thumb_top)),
                size(px(SCROLLBAR_WIDTH - 6.), px(thumb_height)),
            ),
            if dragging {
                rgb(0x8c8c8c)
            } else {
                rgb(0xc2c2c2)
            },
        )
        .corner_radii(px(4.)),
    );

    // The mouse is captured while the button is down: the window's events, not the pane's.
    let moved = entity.clone();
    // GPUI calls a window listener in the capture phase and again in the bubble phase: once.
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
        if phase != DispatchPhase::Capture {
            return;
        }
        moved.update(cx, |minimap, cx| {
            minimap.mouse_moved(
                event.position,
                event.pressed_button == Some(gpui::MouseButton::Left),
                event.modifiers.shift,
                cx,
            );
        });
    });
    let released = entity.clone();
    window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
        if phase != DispatchPhase::Capture {
            return;
        }
        released.update(cx, |minimap, cx| minimap.mouse_released(cx));
    });
}

/// Where the image of the type is placed in the minimap: see [`paint`].
fn back_frame(bounds: Bounds<Pixels>, top_row: usize, row_height: f32, scale: f32) -> Frame {
    Frame {
        bounds,
        gutter_width: LEFT_MARGIN,
        top_margin: 0.0,
        ruler_px: 0.0,
        right_bar: SCROLLBAR_WIDTH,
        bottom_bar: 0.0,
        scroll: (0.0, top_row as f64 * f64::from(row_height)),
        scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::back_image::placement::Rect;
    use crate::back_image::{BackBitmap, BackImageSpec, Decoded};
    use matsu_config::settings::types::back_img_pos;

    fn back(position: i32, scroll: (bool, bool)) -> BackPaint {
        let decoded = Decoded {
            width: 40,
            height: 30,
            bgra: vec![0; 40 * 30 * 4],
            copies: (1, 1),
        };
        BackPaint {
            bitmap: std::sync::Arc::new(BackBitmap::from_decoded(decoded).unwrap()),
            spec: BackImageSpec {
                path: std::path::PathBuf::new(),
                position,
                offset: (0, 0),
                repeat: (false, false),
                scroll,
                opacity: 255,
            },
        }
    }

    fn area() -> Bounds<Pixels> {
        Bounds::new(point(px(500.), px(20.)), size(px(150.), px(300.)))
    }

    fn whole() -> Bounds<Pixels> {
        Bounds::new(point(px(0.), px(0.)), area().size)
    }

    #[test]
    fn the_image_keeps_its_size_and_starts_after_the_left_margin() {
        // `CTextArea`: the left of the text area of a minimap is 4; no ruler above it.
        let frame = back_frame(area(), 0, 4.0, 1.0);
        let tiles = back_paint::tiles(&back(back_img_pos::TOP_LEFT, (true, true)), &frame, whole());
        assert_eq!(tiles, [Rect::new(4, 0, 44, 30)]);
        // bottom right: the right edge of the area is the pane without its scroll bar
        let tiles = back_paint::tiles(
            &back(back_img_pos::BOTTOM_RIGHT, (true, true)),
            &frame,
            whole(),
        );
        let right = 150 - SCROLLBAR_WIDTH as i64;
        assert_eq!(tiles, [Rect::new(right - 40, 270, right, 300)]);
    }

    #[test]
    fn the_image_scrolls_with_the_rows_of_the_minimap_only_where_the_type_says() {
        // `DrawBackImage`: `GetViewTopLine()` times the height of a row, and no sideways scroll.
        let frame = back_frame(area(), 3, 4.0, 1.0);
        let moving =
            back_paint::tiles(&back(back_img_pos::TOP_LEFT, (true, true)), &frame, whole());
        assert_eq!(moving, [Rect::new(4, -12, 44, 18)]);
        let fixed = back_paint::tiles(
            &back(back_img_pos::TOP_LEFT, (false, false)),
            &frame,
            whole(),
        );
        assert_eq!(fixed, [Rect::new(4, 0, 44, 30)]);
    }
}
