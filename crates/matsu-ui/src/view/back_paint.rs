//! Painting the background image under the text: `CEditView::DrawBackImage`
//! (`sakura_core/view/CEditView_Paint.cpp`). The tiles come from `crate::back_image::placement`.
//!
//! What is over the image, as Sakura paints it (`bTransText` in `CEditView::OnPaint`,
//! `DrawLogicLine`, `CTextDrawer::DispLineNumber`, `CFigureStrategy::DrawImp_StyleSelect`): a
//! background that is the background of the text (`TXT`) is not painted, so the image shows
//! through the row, the line number area (when `GYOU` has the same background) and the pieces of
//! text; any other background (the line of the caret, the even lines, the selection, a keyword's
//! own background, the colors of a bookmark) is painted over the image.

use std::sync::Arc;

use gpui::{Bounds, Corners, Pixels, Window, point, px, size};

use crate::back_image::placement::{self, Layout, Rect};
use crate::back_image::{BackBitmap, BackImageSpec};

/// The image and how the type wants it placed.
#[derive(Clone)]
pub(super) struct BackPaint {
    pub bitmap: Arc<BackBitmap>,
    pub spec: BackImageSpec,
}

/// The view the image is placed in, in GPUI pixels.
#[derive(Clone, Copy, Debug)]
pub(super) struct Frame {
    /// The whole view.
    pub bounds: Bounds<Pixels>,
    /// The width of the line number area (the text area starts there).
    pub gutter_width: f32,
    /// The space above the first row: the ruler and the space under it.
    pub top_margin: f32,
    /// The height of the ruler: the image starts under it.
    pub ruler_px: f32,
    /// The vertical scroll bar at the right.
    pub right_bar: f32,
    /// The horizontal scroll bar at the bottom (0 when there is none).
    pub bottom_bar: f32,
    /// How far the text is scrolled, in pixels.
    pub scroll: (f32, f32),
    /// Device pixels in a GPUI pixel.
    pub scale: f32,
}

/// Where the tiles are (in device pixels from the top left of the view) for `clip`, a part of
/// the view relative to its top left.
pub(super) fn tiles(back: &BackPaint, frame: &Frame, clip: Bounds<Pixels>) -> Vec<Rect> {
    let dev = |v: f32| (v * frame.scale).round() as i64;
    let width = f32::from(frame.bounds.size.width);
    let height = f32::from(frame.bounds.size.height);
    let area = Rect::new(
        dev(frame.gutter_width),
        dev(frame.top_margin),
        dev(width - frame.right_bar),
        dev(height - frame.bottom_bar),
    );
    // `rc.top = max(rc.top, ruler height)`: the ruler is not painted over.
    let painted = Rect::new(0, dev(frame.ruler_px), area.right, area.bottom);
    let clip = Rect::new(
        dev(f32::from(clip.left())).max(painted.left),
        dev(f32::from(clip.top())).max(painted.top),
        dev(f32::from(clip.right())).min(painted.right),
        dev(f32::from(clip.bottom())).min(painted.bottom),
    );
    if clip.width() <= 0 || clip.height() <= 0 {
        return Vec::new();
    }
    let (w, h) = back.bitmap.size;
    placement::tiles(&Layout {
        area,
        clip,
        image: (i64::from(w), i64::from(h)),
        copies: (
            i64::from(back.bitmap.copies.0),
            i64::from(back.bitmap.copies.1),
        ),
        position: back.spec.position,
        offset: (i64::from(back.spec.offset.0), i64::from(back.spec.offset.1)),
        repeat: back.spec.repeat,
        scroll: back.spec.scroll,
        scrolled: (dev(frame.scroll.0), dev(frame.scroll.1)),
    })
}

/// Paints the image in the part `clip` of the view (relative to its top left). The background
/// color is painted by the caller, under this.
pub(super) fn paint(window: &mut Window, back: &BackPaint, frame: &Frame, clip: Bounds<Pixels>) {
    let tiles = tiles(back, frame, clip);
    let scale = frame.scale;
    let origin = frame.bounds.origin;
    // The visible part is the clip inside the area the image is painted in.
    let visible = Bounds::from_corners(
        point(
            origin.x + clip.left(),
            origin.y + clip.top().max(px(frame.ruler_px)),
        ),
        point(
            origin.x
                + clip
                    .right()
                    .min(frame.bounds.size.width - px(frame.right_bar)),
            origin.y
                + clip
                    .bottom()
                    .min(frame.bounds.size.height - px(frame.bottom_bar)),
        ),
    );
    for tile in tiles {
        let image_bounds = Bounds::new(
            point(
                origin.x + px(tile.left as f32 / scale),
                origin.y + px(tile.top as f32 / scale),
            ),
            size(
                px(tile.width() as f32 / scale),
                px(tile.height() as f32 / scale),
            ),
        );
        window
            .paint_image(
                visible,
                image_bounds,
                Corners::default(),
                back.bitmap.render.clone(),
                0,
                false,
            )
            .ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matsu_config::settings::types::back_img_pos;

    fn back(position: i32, repeat: (bool, bool), scroll: (bool, bool)) -> BackPaint {
        let decoded = crate::back_image::Decoded {
            width: 100,
            height: 50,
            bgra: vec![0; 100 * 50 * 4],
            copies: (1, 1),
        };
        BackPaint {
            bitmap: Arc::new(
                BackBitmap::from_decoded(decoded).expect("the picture has the size it says"),
            ),
            spec: BackImageSpec {
                path: std::path::PathBuf::new(),
                position,
                offset: (0, 0),
                repeat,
                scroll,
                opacity: 255,
            },
        }
    }

    fn frame(scale: f32) -> Frame {
        Frame {
            bounds: Bounds::new(point(px(10.), px(10.)), size(px(400.), px(300.))),
            gutter_width: 40.0,
            top_margin: 20.0,
            ruler_px: 20.0,
            right_bar: 17.0,
            bottom_bar: 0.0,
            scroll: (0.0, 0.0),
            scale,
        }
    }

    fn whole(frame: &Frame) -> Bounds<Pixels> {
        Bounds::new(point(px(0.), px(0.)), frame.bounds.size)
    }

    #[test]
    fn the_image_is_placed_in_the_text_area_of_the_view() {
        let f = frame(1.0);
        let top_left = tiles(
            &back(back_img_pos::TOP_LEFT, (false, false), (true, true)),
            &f,
            whole(&f),
        );
        assert_eq!(top_left, [Rect::new(40, 20, 140, 70)]);
        // the right edge of the area is the view without the scroll bar
        let bottom_right = tiles(
            &back(back_img_pos::BOTTOM_RIGHT, (false, false), (true, true)),
            &f,
            whole(&f),
        );
        assert_eq!(bottom_right, [Rect::new(283, 250, 383, 300)]);
    }

    #[test]
    fn scrolling_moves_the_tiles_of_a_scrolling_image_only() {
        let mut f = frame(1.0);
        f.scroll = (12.0, 30.0);
        let moving = tiles(
            &back(back_img_pos::TOP_LEFT, (false, false), (true, true)),
            &f,
            whole(&f),
        );
        assert_eq!(moving, [Rect::new(28, -10, 128, 40)]);
        let fixed = tiles(
            &back(back_img_pos::TOP_LEFT, (false, false), (false, false)),
            &f,
            whole(&f),
        );
        assert_eq!(fixed, [Rect::new(40, 20, 140, 70)]);
        let sideways = tiles(
            &back(back_img_pos::TOP_LEFT, (false, false), (true, false)),
            &f,
            whole(&f),
        );
        assert_eq!(sideways, [Rect::new(28, 20, 128, 70)]);
    }

    #[test]
    fn a_repeating_image_keeps_its_grid_while_the_text_scrolls() {
        let mut f = frame(1.0);
        let b = back(back_img_pos::TOP_LEFT, (true, true), (true, true));
        let before = tiles(&b, &f, whole(&f));
        f.scroll = (100.0, 50.0);
        assert_eq!(tiles(&b, &f, whole(&f)), before);
    }

    #[test]
    fn the_image_is_not_painted_over_the_ruler() {
        let f = frame(1.0);
        let b = back(back_img_pos::TOP_LEFT, (true, true), (true, true));
        // A part of the view that is all ruler has no tile.
        let ruler = Bounds::new(point(px(0.), px(0.)), size(px(400.), px(20.)));
        assert!(tiles(&b, &f, ruler).is_empty());
        // The part under it does.
        let under = Bounds::new(point(px(0.), px(20.)), size(px(50.), px(10.)));
        assert!(!tiles(&b, &f, under).is_empty());
    }

    #[test]
    fn the_size_of_the_picture_is_in_device_pixels() {
        // At 200%, the picture (100 x 50 pixels) is 50 x 25 GPUI pixels, and the area starts at
        // 80 device pixels.
        let f = frame(2.0);
        let t = tiles(
            &back(back_img_pos::TOP_LEFT, (false, false), (true, true)),
            &f,
            whole(&f),
        );
        assert_eq!(t, [Rect::new(80, 40, 180, 90)]);
    }
}
