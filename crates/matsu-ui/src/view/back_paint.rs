// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/view/CEditView_Paint.cpp (CEditView::OnPaint, DrawBackImage)
//   sakura_core/view/CEditView_Paint_Bracket.cpp (CEditView::DrawBracketPair)
//   sakura_core/view/CTextDrawer.cpp (CTextDrawer::DispLineNumber)
//
// Original copyright notice of those files:
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2022, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2008, kobake; 2018-2022, Sakura Editor Organization
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! Painting the background image under the text: `CEditView::DrawBackImage`
//! (`sakura_core/view/CEditView_Paint.cpp`). The tiles come from `crate::back_image::placement`.
//!
//! What is over the image, as Sakura paints it (`bTransText` in `CEditView::OnPaint`,
//! `DrawLogicLine`, `CTextDrawer::DispLineNumber`, `CFigureStrategy::DrawImp_StyleSelect`): a
//! background that is the background of the text (`TXT`) is not painted, so the image shows
//! through the row, the line number area (when `GYOU` has the same background) and the pieces of
//! text; any other background (the line of the caret, the even lines, the selection, a keyword's
//! own background, the colors of a bookmark) is painted over the image.

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use gpui::{App, Bounds, Corners, Global, Pixels, RenderImage, Window, WindowId, point, px};

use crate::back_image::placement::{self, Layout, Rect};
use crate::back_image::{BackBitmap, BackImageSpec};

/// The image and how the type wants it placed.
#[derive(Clone)]
pub(super) struct BackPaint {
    pub bitmap: Arc<BackBitmap>,
    pub spec: BackImageSpec,
}

/// The pictures a window has drawn, so that the ones no view uses any more are taken out of the
/// window's sprite atlas. GPUI keeps a drawn picture in the atlas until `Window::drop_image`
/// (letting go of the `Arc` does not), so a picture that the settings have replaced would stay in
/// the graphics memory until the window closed.
#[derive(Default)]
struct Drawn(HashMap<WindowId, Vec<DrawnImage>>);

impl Global for Drawn {}

struct DrawnImage {
    /// Kept to be able to drop it from the atlas (the key is its id).
    render: Arc<RenderImage>,
    /// The views hold the picture; this does not, so it tells when no view does.
    bitmap: Weak<BackBitmap>,
}

/// Call it once for each paint of a view with the image it is going to draw (`None` if it draws
/// none): notes the picture for this window, and drops from the atlas the pictures of this window
/// that no view holds any more. The pictures of windows that are closed are forgotten.
pub(super) fn track(window: &mut Window, cx: &mut App, current: Option<&BackPaint>) {
    let id = window.window_handle().window_id();
    let open: Vec<WindowId> = cx.windows().iter().map(|w| w.window_id()).collect();
    let drawn = cx.default_global::<Drawn>();
    drawn
        .0
        .retain(|window_id, _| *window_id == id || open.contains(window_id));
    let mine = drawn.0.entry(id).or_default();
    if let Some(back) = current
        && !mine
            .iter()
            .any(|d| Arc::ptr_eq(&d.render, &back.bitmap.render))
    {
        mine.push(DrawnImage {
            render: back.bitmap.render.clone(),
            bitmap: Arc::downgrade(&back.bitmap),
        });
    }
    mine.retain(|d| {
        let used = d.bitmap.strong_count() > 0;
        if !used {
            window.drop_image(d.render.clone()).ok();
        }
        used
    });
}

/// `bounds` with each edge on a whole device pixel (rounded in the window, as `plan` and GPUI
/// round the edges of an image). A rectangle given such bounds is painted whole, without the
/// partly painted edge row that GPUI gives a rectangle whose edge is between two pixels.
pub(super) fn snapped(bounds: Bounds<Pixels>, scale: f32) -> Bounds<Pixels> {
    let round = |v: Pixels| px((f32::from(v) * scale).round() / scale);
    Bounds::from_corners(
        point(round(bounds.left()), round(bounds.top())),
        point(round(bounds.right()), round(bounds.bottom())),
    )
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
/// the window (in window coordinates, like the rectangles the caller paints); and the part of the
/// view that is painted, cut to whole device pixels (from the top left of the view).
///
/// Every number is a whole device pixel, whatever the clip is: a cell cut out of the view gets
/// the same picture as the same pixels of the whole view (the clip of a cell starts between two
/// pixels, but the picture must not move with it). The edges of the clip are rounded as GPUI
/// rounds the edges of a rectangle (each edge by itself, in the window), so that the clip is
/// the same device pixels as a rectangle painted over the same `Bounds`.
pub(super) fn plan(back: &BackPaint, frame: &Frame, clip: Bounds<Pixels>) -> (Vec<Rect>, Rect) {
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
    let origin = (
        dev(f32::from(frame.bounds.origin.x)),
        dev(f32::from(frame.bounds.origin.y)),
    );
    let clip = Rect::new(
        (dev(f32::from(clip.left())) - origin.0).max(painted.left),
        (dev(f32::from(clip.top())) - origin.1).max(painted.top),
        (dev(f32::from(clip.right())) - origin.0).min(painted.right),
        (dev(f32::from(clip.bottom())) - origin.1).min(painted.bottom),
    );
    if clip.width() <= 0 || clip.height() <= 0 {
        return (Vec::new(), clip);
    }
    let (w, h) = back.bitmap.size;
    let tiles = placement::tiles(&Layout {
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
    });
    (tiles, clip)
}

/// [`plan`]'s tiles, for a clip given from the top left of the view.
#[cfg(test)]
pub(super) fn tiles(back: &BackPaint, frame: &Frame, clip: Bounds<Pixels>) -> Vec<Rect> {
    plan(back, frame, shifted(frame, clip)).0
}

/// `clip`, from the top left of the view, in window coordinates.
#[cfg(test)]
fn shifted(frame: &Frame, clip: Bounds<Pixels>) -> Bounds<Pixels> {
    Bounds::new(
        point(
            frame.bounds.origin.x + clip.origin.x,
            frame.bounds.origin.y + clip.origin.y,
        ),
        clip.size,
    )
}

/// Paints the image in the part `clip` of the window (the `Bounds` of the rectangle the caller
/// paints the background in). The background color is painted by the caller, under this.
pub(super) fn paint(window: &mut Window, back: &BackPaint, frame: &Frame, clip: Bounds<Pixels>) {
    let (tiles, painted) = plan(back, frame, clip);
    let scale = frame.scale;
    // The view starts at a whole device pixel (as the pictures are placed from it).
    let origin = point(
        px((f32::from(frame.bounds.origin.x) * scale).round() / scale),
        px((f32::from(frame.bounds.origin.y) * scale).round() / scale),
    );
    let at = |x: i64, y: i64| {
        point(
            origin.x + px(x as f32 / scale),
            origin.y + px(y as f32 / scale),
        )
    };
    let visible = Bounds::from_corners(
        at(painted.left, painted.top),
        at(painted.right, painted.bottom),
    );
    for tile in tiles {
        let image_bounds =
            Bounds::from_corners(at(tile.left, tile.top), at(tile.right, tile.bottom));
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
    use gpui::size;
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

    /// Which pixel of the picture is at (x, y) of the view, by the tiles.
    fn source_pixel(tiles: &[Rect], x: i64, y: i64, size: (i64, i64)) -> Option<(i64, i64)> {
        tiles
            .iter()
            .find(|t| t.left <= x && x < t.right && t.top <= y && y < t.bottom)
            .map(|t| {
                (
                    (x - t.left).rem_euclid(size.0),
                    (y - t.top).rem_euclid(size.1),
                )
            })
    }

    #[test]
    fn a_cell_gets_the_same_picture_as_the_same_pixels_of_the_whole_view() {
        // The cell of a bracket starts between two pixels and the text is scrolled by a part of a
        // pixel: the picture in the cell must be where it is in the whole view.
        let picture = (100, 50);
        let positions = [
            back_img_pos::TOP_LEFT,
            back_img_pos::CENTER,
            back_img_pos::BOTTOM_RIGHT,
        ];
        let flags = [(false, false), (true, true), (true, false), (false, true)];
        let cells = [(87.4, 63.6), (130.2, 40.9), (41.5, 20.5), (300.7, 200.1)];
        for scale in [1.0, 1.25, 2.0] {
            for repeat in flags {
                for scroll in flags {
                    for position in positions {
                        for scrolled in [(0.0, 0.0), (37.3, 91.7), (250.5, 12.25), (0.5, 400.0)] {
                            let b = back(position, repeat, scroll);
                            let mut f = frame(scale);
                            f.scroll = scrolled;
                            let (all, _) = plan(&b, &f, shifted(&f, whole(&f)));
                            for (x, y) in cells {
                                let cell =
                                    Bounds::new(point(px(x), px(y)), size(px(8.4), px(18.3)));
                                let (cut, rect) = plan(&b, &f, shifted(&f, cell));
                                for py in rect.top..rect.bottom {
                                    for pxl in rect.left..rect.right {
                                        assert_eq!(
                                            source_pixel(&cut, pxl, py, picture),
                                            source_pixel(&all, pxl, py, picture),
                                            "scale {scale} repeat {repeat:?} scroll {scroll:?} \
                                             position {position} scrolled {scrolled:?} \
                                             cell ({x}, {y}) pixel ({pxl}, {py})"
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_rectangle_and_the_image_of_a_cell_have_the_same_whole_pixels() {
        // The rectangle that clears a cell and the clip of the image painted in it are given
        // `snapped` bounds: the same device pixels, with no edge between two pixels.
        let b = back(back_img_pos::TOP_LEFT, (true, true), (true, true));
        for scale in [1.0_f32, 1.25, 1.5, 2.0] {
            let mut f = frame(scale);
            f.bounds = Bounds::new(point(px(10.3), px(10.7)), f.bounds.size);
            for (x, y) in [(87.4_f32, 956.5_f32), (130.2, 63.6), (41.5, 50.5)] {
                let cell = Bounds::new(point(px(x), px(y)), size(px(8.4), px(18.3)));
                let snapped = snapped(cell, scale);
                for edge in [
                    snapped.left(),
                    snapped.top(),
                    snapped.right(),
                    snapped.bottom(),
                ] {
                    let device = f32::from(edge) * scale;
                    assert!(
                        (device - device.round()).abs() < 1e-3,
                        "{device} at {scale}"
                    );
                }
                // `plan` of the snapped cell is the pixels of the snapped rectangle.
                let (_, clip) = plan(&b, &f, snapped);
                let dev = |v: Pixels| (f32::from(v) * scale).round() as i64;
                let (ox, oy) = (
                    (f32::from(f.bounds.origin.x) * scale).round() as i64,
                    (f32::from(f.bounds.origin.y) * scale).round() as i64,
                );
                // (the area of the view cuts the clip: only compare a cell inside it)
                if clip.width() > 0 && clip.height() > 0 && y < 300.0 {
                    assert_eq!(
                        (
                            clip.left + ox,
                            clip.top + oy,
                            clip.right + ox,
                            clip.bottom + oy
                        ),
                        (
                            dev(snapped.left()),
                            dev(snapped.top()),
                            dev(snapped.right()),
                            dev(snapped.bottom())
                        ),
                        "scale {scale} cell ({x}, {y})"
                    );
                }
            }
        }
    }

    #[test]
    fn the_clip_is_the_device_pixels_of_the_rectangle_painted_under_it() {
        // The view does not start at a whole device pixel and neither does the cell: the clip of
        // the image has the pixels GPUI gives a rectangle of the same bounds (each edge rounded
        // in the window), not the sum of two roundings.
        let b = back(back_img_pos::TOP_LEFT, (true, true), (true, true));
        for scale in [1.25_f32, 1.5, 2.0] {
            for origin in [(10.0_f32, 10.0_f32), (10.3, 10.7), (0.4, 0.6)] {
                let mut f = frame(scale);
                f.bounds = Bounds::new(point(px(origin.0), px(origin.1)), f.bounds.size);
                for (rx, ry) in [
                    (63.6_f32, 63.6_f32),
                    (87.4, 40.9),
                    (130.2, 100.1),
                    (41.5, 20.5),
                ] {
                    let cell = Bounds::new(
                        point(px(origin.0 + rx), px(origin.1 + ry)),
                        size(px(8.4), px(18.3)),
                    );
                    let (_, clip) = plan(&b, &f, cell);
                    let device = |v: f32| (v * scale).round() as i64;
                    let view = (device(origin.0), device(origin.1));
                    assert_eq!(
                        (
                            view.0 + clip.left,
                            view.1 + clip.top,
                            view.0 + clip.right,
                            view.1 + clip.bottom
                        ),
                        (
                            device(origin.0 + rx),
                            device(origin.1 + ry),
                            device(origin.0 + rx + 8.4),
                            device(origin.1 + ry + 18.3)
                        ),
                        "scale {scale} origin {origin:?} cell ({rx}, {ry})"
                    );
                }
            }
        }
    }

    #[test]
    fn one_cell_of_the_view_gets_the_part_of_the_image_under_it() {
        // `DrawBracketPair` paints the image again in the cell of a bracket.
        let f = frame(1.0);
        let b = back(back_img_pos::TOP_LEFT, (false, false), (true, true));
        let cell = |x: f32, y: f32| Bounds::new(point(px(x), px(y)), size(px(9.), px(18.)));
        // inside the image (it covers x 40..140, y 20..70 of the view)
        assert_eq!(tiles(&b, &f, cell(60., 30.)), [Rect::new(40, 20, 140, 70)]);
        // a cell the image does not reach has no tile
        assert!(tiles(&b, &f, cell(200., 30.)).is_empty());
        assert!(tiles(&b, &f, cell(60., 120.)).is_empty());
        // and a repeating image is under every cell
        let r = back(back_img_pos::TOP_LEFT, (true, true), (true, true));
        assert_eq!(tiles(&r, &f, cell(200., 120.)).len(), 1);
    }

    fn write_png(path: &std::path::Path, side: u32, shade: u8) {
        let mut img = image::RgbaImage::new(side, side);
        for (x, y, p) in img.enumerate_pixels_mut() {
            *p = image::Rgba([shade, x as u8, y as u8, 255]);
        }
        img.save_with_format(path, image::ImageFormat::Png).unwrap();
    }

    fn paint_again(cx: &mut gpui::VisualTestContext) {
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
    }

    fn use_picture(
        view: &gpui::Entity<super::super::EditorView>,
        cx: &mut gpui::VisualTestContext,
        path: Option<&std::path::Path>,
    ) {
        view.update(cx, |v, cx| {
            let mut settings = v.view_settings().clone();
            settings.back_image = path.map(|path| BackImageSpec {
                path: path.to_owned(),
                position: 0,
                offset: (0, 0),
                repeat: (false, false),
                scroll: (true, true),
                opacity: 255,
            });
            v.apply_settings(settings, cx);
        });
    }

    #[gpui::test]
    fn a_picture_the_settings_replaced_leaves_the_atlas(cx: &mut gpui::TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a.png"), dir.path().join("b.png"));
        write_png(&a, 8, 10);
        write_png(&b, 9, 20);
        let (f, vcx) = super::super::tests::setup(cx, "text");
        use_picture(&f.view, vcx, Some(&a));
        paint_again(vcx);
        let first = f.view.read_with(vcx, |v, _| v.back_bitmap.clone().unwrap());
        let in_atlas = |vcx: &mut gpui::VisualTestContext, bitmap: &Arc<BackBitmap>| {
            vcx.update(|window, _| window.has_image_atlas_entry(&bitmap.render))
        };
        assert!(in_atlas(vcx, &first), "the picture was drawn");

        // Another view of the window holds the first picture (`first` is its hold): it stays in
        // the atlas when this view takes another picture.
        use_picture(&f.view, vcx, Some(&b));
        paint_again(vcx);
        let second = f.view.read_with(vcx, |v, _| v.back_bitmap.clone().unwrap());
        assert!(!Arc::ptr_eq(&first, &second));
        assert!(in_atlas(vcx, &first), "a view still uses the first picture");
        assert!(in_atlas(vcx, &second));

        // When no view uses it, the next paint takes it out.
        let first_render = first.render.clone();
        drop(first);
        paint_again(vcx);
        assert!(!vcx.update(|window, _| window.has_image_atlas_entry(&first_render)));
        assert!(in_atlas(vcx, &second));

        // A type without an image leaves nothing of the second either.
        let second_render = second.render.clone();
        drop(second);
        use_picture(&f.view, vcx, None);
        paint_again(vcx);
        assert!(!vcx.update(|window, _| window.has_image_atlas_entry(&second_render)));
    }

    #[gpui::test]
    fn the_same_picture_is_drawn_again_without_a_second_atlas_entry(cx: &mut gpui::TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        write_png(&a, 8, 10);
        let (f, vcx) = super::super::tests::setup(cx, "text");
        use_picture(&f.view, vcx, Some(&a));
        paint_again(vcx);
        let first = f.view.read_with(vcx, |v, _| v.back_bitmap.clone().unwrap());
        // The settings are applied again with the same file: the same picture is used.
        use_picture(&f.view, vcx, Some(&a));
        paint_again(vcx);
        let again = f.view.read_with(vcx, |v, _| v.back_bitmap.clone().unwrap());
        assert!(Arc::ptr_eq(&first, &again));
        assert!(vcx.update(|window, _| window.has_image_atlas_entry(&first.render)));
    }
}
