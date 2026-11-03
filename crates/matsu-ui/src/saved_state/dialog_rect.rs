//! The position and the size that a dialog opens with.
//!
//! Sakura: `CommonSetting_Others::m_rcOpenDialog`, `m_rcCompareDialog`, `m_rcDiffDialog`,
//! `m_rcFavoriteDialog`, `m_rcTagJumpDialog` and `m_rcWindowListDialog` (`sakura_core/env/CommonSetting.h`)
//! with the keys `rcOpenDialog`, `rcCompareDialog`, `rcDiffDialog`, `rcFavoriteDialog`,
//! `rcTagJumpDialog` and `rcWindowListDialog` of `[Common]`, written as `left,top,right,bottom`
//! (`CDataProfile::IOProfileData<RECT>`: `%d,%d,%d,%d`; a value without four numbers is not read).
//! The default is `0,0,0,0`.
//!
//! A dialog takes the rectangle in `OnInitDialog` when its left or its bottom is not 0, and
//! writes it in `OnDestroy`, whichever way it is closed (`CDlgFavorite.cpp`, `CDlgDiff.cpp`,
//! `CDlgCompare.cpp`, `CDlgTagJumpList.cpp`, `CDlgWindowList.cpp`). The dialog is then put
//! inside the work area of the monitor (`CDialog::SetDialogPosSize`), with one dot to spare on
//! every side.
//!
//! Only 「履歴とお気に入りの管理」 is a window of the OS here (the others are panels in the
//! editor window and the open dialog is the OS's own), so it is the only one that keeps a
//! rectangle; the other keys are left as the file has them. The numbers are GPUI's logical
//! pixels, where Sakura's are device pixels.

use gpui::{App, Bounds, Global, Pixels, point, px, size};
use matsu_config::Profile;

const SECTION: &str = "Common";
const KEY_FAVORITE: &str = "rcFavoriteDialog";

/// A `RECT`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub(crate) fn width(self) -> i32 {
        self.right - self.left
    }

    pub(crate) fn height(self) -> i32 {
        self.bottom - self.top
    }

    /// What the file holds: `left,top,right,bottom`.
    pub(crate) fn text(self) -> String {
        format!("{},{},{},{}", self.left, self.top, self.right, self.bottom)
    }

    /// The reading of `swscanf_s(L"%d,%d,%d,%d")`: four numbers; white space may come before a
    /// number, and what follows the fourth is ignored. Anything else is not a rectangle.
    pub(crate) fn parse(text: &str) -> Option<Rect> {
        fn number(text: &str) -> Option<(i32, &str)> {
            let text = text.trim_start();
            let digits_at = usize::from(text.starts_with(['-', '+']));
            let end = digits_at
                + text[digits_at..]
                    .find(|c: char| !c.is_ascii_digit())
                    .unwrap_or(text.len() - digits_at);
            if end == digits_at {
                return None;
            }
            Some((text[..end].parse().ok()?, &text[end..]))
        }
        let (left, rest) = number(text)?;
        let (top, rest) = number(rest.strip_prefix(',')?)?;
        let (right, rest) = number(rest.strip_prefix(',')?)?;
        let (bottom, _) = number(rest.strip_prefix(',')?)?;
        Some(Rect {
            left,
            top,
            right,
            bottom,
        })
    }

    /// Whether a dialog takes it (`rcDialog.left != 0 || rcDialog.bottom != 0`).
    pub(crate) fn is_set(self) -> bool {
        self.left != 0 || self.bottom != 0
    }

    pub(crate) fn of_bounds(bounds: Bounds<Pixels>) -> Rect {
        let left = f32::from(bounds.origin.x).round() as i32;
        let top = f32::from(bounds.origin.y).round() as i32;
        Rect {
            left,
            top,
            right: left + f32::from(bounds.size.width).round() as i32,
            bottom: top + f32::from(bounds.size.height).round() as i32,
        }
    }

    pub(crate) fn bounds(self) -> Bounds<Pixels> {
        Bounds {
            origin: point(px(self.left as f32), px(self.top as f32)),
            size: size(px(self.width() as f32), px(self.height() as f32)),
        }
    }

    /// The area of the overlap with `other`.
    fn overlap(self, other: Rect) -> i64 {
        let w = i64::from(self.right.min(other.right)) - i64::from(self.left.max(other.left));
        let h = i64::from(self.bottom.min(other.bottom)) - i64::from(self.top.max(other.top));
        if w > 0 && h > 0 { w * h } else { 0 }
    }

    /// The square of the distance between the centers.
    fn distance(self, other: Rect) -> i64 {
        let dx = i64::from(self.left) + i64::from(self.right)
            - i64::from(other.left)
            - i64::from(other.right);
        let dy = i64::from(self.top) + i64::from(self.bottom)
            - i64::from(other.top)
            - i64::from(other.bottom);
        dx * dx + dy * dy
    }

    /// The rectangle moved into the work area of the monitor it is on (the one it overlaps the
    /// most, else the nearest), with one dot to spare (`CDialog::SetDialogPosSize`). A rectangle
    /// that is larger than the area starts at the top or the left of the area; its size is kept.
    /// Without a monitor it is left as it is.
    pub(crate) fn fitted(self, work_areas: &[Rect]) -> Rect {
        let Some(area) = work_areas.iter().copied().max_by_key(|area| {
            (
                self.overlap(*area),
                // the nearer, the better
                -self.distance(*area),
            )
        }) else {
            return self;
        };
        let area = Rect {
            left: area.left + 1,
            top: area.top + 1,
            right: area.right - 1,
            bottom: area.bottom - 1,
        };
        let (width, height) = (self.width(), self.height());
        let Rect {
            mut left, mut top, ..
        } = self;
        let mut right = left + width;
        let mut bottom = top + height;
        if bottom > area.bottom {
            if height > area.height() {
                top = area.top;
            } else {
                top = area.bottom - height;
            }
            bottom = top + height;
        }
        if right > area.right {
            if width > area.width() {
                // Sakura adds the height here, which is a typo: the width is kept.
                right = left + width;
            } else {
                left = area.right - width;
                right = area.right;
            }
        }
        if top < area.top {
            top = area.top;
            bottom = top + height;
        }
        if left < area.left {
            left = area.left;
            right = left + width;
        }
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }
}

/// What the dialogs open with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DialogRects {
    /// 「履歴とお気に入りの管理」 (`m_rcFavoriteDialog`).
    pub favorite: Rect,
}

impl Global for DialogRects {}

impl DialogRects {
    pub(crate) fn from_profile(profile: &Profile) -> Self {
        Self {
            favorite: profile
                .get(SECTION, KEY_FAVORITE)
                .and_then(Rect::parse)
                .unwrap_or_default(),
        }
    }
}

/// The rectangle of 「履歴とお気に入りの管理」 now.
pub(crate) fn favorite(cx: &App) -> Rect {
    cx.try_global::<DialogRects>()
        .copied()
        .unwrap_or_default()
        .favorite
}

/// The work areas of the monitors.
pub(crate) fn work_areas(cx: &App) -> Vec<Rect> {
    cx.displays()
        .iter()
        .map(|display| Rect::of_bounds(display.visible_bounds()))
        .collect()
}

/// 「履歴とお気に入りの管理」 was closed (`CDlgFavorite::OnDestroy`): its rectangle is kept for the
/// next one, in every window, and in the file.
pub(crate) fn remember_favorite(cx: &mut App, rect: Rect) {
    cx.set_global(DialogRects { favorite: rect });
    crate::history::write_settings(cx, |profile| {
        let text = rect.text();
        // A rectangle the file already has stays as written.
        if profile.get(SECTION, KEY_FAVORITE).and_then(Rect::parse) == Some(rect) {
            return Ok(false);
        }
        profile.set(SECTION, KEY_FAVORITE, &text)?;
        Ok(true)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn the_text_is_four_numbers_with_commas() {
        assert_eq!(rect(10, 20, 410, 320).text(), "10,20,410,320");
        assert_eq!(rect(-5, 0, 0, 0).text(), "-5,0,0,0");
        assert_eq!(Rect::default().text(), "0,0,0,0");
    }

    #[test]
    fn four_numbers_are_a_rectangle_and_nothing_else_is() {
        assert_eq!(Rect::parse("10,20,410,320"), Some(rect(10, 20, 410, 320)));
        assert_eq!(Rect::parse("-5, 6,7, 8"), Some(rect(-5, 6, 7, 8)));
        // only a number may have white space before it; the comma follows it at once
        assert_eq!(Rect::parse("-5, 6 ,7,8"), None);
        assert_eq!(Rect::parse("1,2,3,4,5"), Some(rect(1, 2, 3, 4)));
        assert_eq!(Rect::parse("1,2,3,4x"), Some(rect(1, 2, 3, 4)));
        assert_eq!(Rect::parse("1,2,3"), None);
        assert_eq!(Rect::parse("1,2,3,"), None);
        assert_eq!(Rect::parse("a,2,3,4"), None);
        assert_eq!(Rect::parse(""), None);
        assert_eq!(Rect::parse("99999999999,2,3,4"), None);
    }

    #[test]
    fn a_dialog_takes_the_rectangle_when_its_left_or_bottom_is_not_zero() {
        assert!(!Rect::default().is_set());
        assert!(rect(0, 0, 0, 5).is_set());
        assert!(rect(3, 0, 0, 0).is_set());
        assert!(!rect(0, 7, 9, 0).is_set());
    }

    const SCREEN: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1080,
    };

    #[test]
    fn a_rectangle_inside_the_work_area_stays() {
        let r = rect(100, 100, 500, 400);
        assert_eq!(r.fitted(&[SCREEN]), r);
    }

    #[test]
    fn a_rectangle_off_the_screen_is_brought_back_with_its_size() {
        // far to the right and below
        assert_eq!(
            rect(5000, 3000, 5400, 3300).fitted(&[SCREEN]),
            rect(1519, 779, 1919, 1079)
        );
        // far to the left and above
        assert_eq!(
            rect(-900, -700, -500, -400).fitted(&[SCREEN]),
            rect(1, 1, 401, 301)
        );
    }

    #[test]
    fn a_rectangle_larger_than_the_area_starts_at_its_top_left() {
        let r = rect(-50, -50, 2500, 1500).fitted(&[SCREEN]);
        assert_eq!((r.left, r.top), (1, 1));
        assert_eq!((r.width(), r.height()), (2550, 1550));
    }

    #[test]
    fn the_monitor_it_overlaps_the_most_decides() {
        let second = rect(1920, 0, 3840, 1080);
        let r = rect(3600, 100, 4100, 400);
        // mostly on the second monitor: moved back into it, not onto the first
        assert_eq!(r.fitted(&[SCREEN, second]), rect(3339, 100, 3839, 400));
        // without any monitor nothing can be done
        assert_eq!(r.fitted(&[]), r);
        // off every monitor: the nearest one
        let far = rect(9000, 100, 9400, 300);
        assert_eq!(far.fitted(&[SCREEN, second]), rect(3439, 100, 3839, 300));
    }

    #[test]
    fn the_keys_of_the_file_are_read() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"[Common]\r\nrcFavoriteDialog=10,20,410,320\r\n");
        let rects = DialogRects::from_profile(&Profile::from_bytes(&bytes));
        assert_eq!(rects.favorite, rect(10, 20, 410, 320));
        assert_eq!(
            DialogRects::from_profile(&Profile::new()).favorite,
            Rect::default()
        );
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"[Common]\r\nrcFavoriteDialog=10,20\r\n");
        assert_eq!(
            DialogRects::from_profile(&Profile::from_bytes(&bytes)).favorite,
            Rect::default()
        );
    }
}
