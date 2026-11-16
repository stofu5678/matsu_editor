//! The width of the popup menus.
//!
//! Sakura's menus are Win32 menus: as wide as their longest item. MATSU's are drawn inside the
//! window by gpui-component's `PopupMenu`, which stops at `max_w` (500 px unless told) and does
//! not cut an item, so a long item (a recent file with a long path) was drawn outside the box.
//! Here a menu may be as wide as its longest item, up to the width of the window, and an item
//! that is still wider is cut with `…`: a path in the middle (the start and the file name stay,
//! as Sakura's short form keeps both ends, `GetShortViewPath` in `sakura_core/util/file.cpp`),
//! any other label at the end. The command of the item still has the whole text.
//!
//! gpui-component chooses the side a submenu opens on from `max_w` of its parent
//! (`update_submenu_menu_anchor`: left when `max_w` and the parent's left edge pass the window's
//! right edge). [`opening`] chooses the side itself (right when the submenu fits there or the
//! room on the right is the larger, as Win32's do), sets the parent's `max_w` so that
//! gpui-component takes that side, and tells the submenu how much room it has from where it opens
//! to the window's edge: its labels are cut to that room, not to the width of the window.

use std::borrow::Cow;

use gpui::{Pixels, Window, px};

/// What gpui-component allows a menu when nothing is set; a menu never gets less.
const DEFAULT_MAX_WIDTH: f32 = 500.0;
/// The room kept to the window's edges: the popups snap to the window with this margin
/// (`snap_to_window_with_margin(Edges::all(EDGE_PADDING))` in gpui-component).
const MARGIN: f32 = 4.0;
/// What gpui-component puts around the rows of a menu (`popup_menu.rs`, `render`/`render_item`):
/// `p_1` on the list (4 px on both sides), `INNER_PADDING` 8 px on both sides of an item, and the
/// 1 px border of the popup.
const CHROME: f32 = 8.0 + 16.0 + 2.0;
/// The icon of a row (`Icon::xsmall`, `size_3`) and the gap after it (`gap_x_1`).
const ICON: f32 = 12.0;
const GAP: f32 = 4.0;
/// The gap before the arrow of a submenu (`gap_2`) and the arrow (`IconName::ChevronRight`,
/// `xsmall`).
const ARROW: f32 = 8.0 + 12.0;
/// The least width of a list (`min_w(rems(8.))`), in rems.
const MIN_REMS: f32 = 8.0;
/// A submenu that opens on the left ends this far left of its parent's left edge
/// (gpui-component: `left: -px(16.)` with the right corner as the anchor).
const LEFT_GAP: f32 = 16.0;
/// The margin of the `anchored()` that moves a popup into the window where it opens (the menu
/// bar, the right-click menus and the macro menus: `snap_to_window_with_margin(px(8.))`).
const SNAP_MARGIN: f32 = 8.0;
/// A path keeps its start only while this much of it fits besides the file name.
const MIN_HEAD: f32 = 16.0;
/// Added to the `max_w` of a menu whose submenu opens on the left, to stay clear of the edge.
const LEFT_SAFETY: f32 = 40.0;
/// How far a submenu overlaps its parent (gpui-component: `bounds.size.width - px(8.)`).
const OVERLAP: f32 = 8.0;
/// The size of the text of an item (`text_sm`: 0.875 rem).
const TEXT_REM: f32 = 0.875;

const ELLIPSIS: char = '…';

/// The kind of a row: what stands after the label differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    Action,
    Submenu,
}

/// How the rows of a menu are laid out, as far as the width goes (`popup_menu.rs`
/// `render_item`).
///
/// A row has a column before the label when any item has an icon or a check mark
/// (`has_left_icon`): [`ICON`] and [`GAP`]. With the menu icons on (`menu_icons`) an action is an
/// element with its own column of `icons` pixels and a gap, and a submenu has an invisible icon
/// as wide as that column.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Layout {
    /// The width of the column of the pictures of `menu_icons`, when they are on.
    pub icons: Option<Pixels>,
    /// Whether an item has a check mark (the column before the label).
    pub checks: bool,
    /// Whether the menu has a submenu (with the icons on, it gives every row the column).
    pub submenus: bool,
}

impl Layout {
    /// What a row of kind `row` takes besides its label, the padding of the list included.
    fn extra(self, row: Row) -> Pixels {
        let column = px(ICON + GAP);
        let left = self.icons.is_some() && self.submenus || self.checks;
        let lead = match (row, self.icons) {
            (Row::Action, Some(icons)) => {
                (if left { column } else { Pixels::ZERO }) + icons + px(GAP)
            }
            (Row::Submenu, Some(icons)) => column + icons + px(GAP),
            (_, None) => {
                if left {
                    column
                } else {
                    Pixels::ZERO
                }
            }
        };
        let trail = match row {
            Row::Action => Pixels::ZERO,
            Row::Submenu => px(ARROW),
        };
        lead + trail + px(CHROME)
    }
}

/// Where a menu is and how much room it has: its left edge in the window and the widest it may be.
/// A popup that would pass the window's edge is moved in by an `anchored()` that keeps `margin`
/// to the edge, so the left edge here is where it was asked to open, and [`opening`] moves it
/// the same way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Place {
    pub left: Pixels,
    pub room: Pixels,
    pub margin: Pixels,
}

impl Place {
    /// A menu that opens with its left edge at `left` (the menus of the bar, the right-click
    /// menus and the macro menus: moved into the window with a margin of `SNAP_MARGIN`).
    pub(crate) fn at(window: &Window, left: Pixels) -> Self {
        Self {
            left,
            room: window.viewport_size().width - px(SNAP_MARGIN * 2.0),
            margin: px(SNAP_MARGIN),
        }
    }
}

/// The `max_w` of a menu and the place of its submenus.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Opening {
    pub max_width: Pixels,
    /// The place of the widest submenu.
    pub child: Place,
    /// Where the submenus are aligned: on the left side a submenu ends 16 px before its parent,
    /// so a narrower one starts farther right than the widest.
    left_edge: Option<Pixels>,
    /// The least width gpui-component draws a list at (`min_w(rems(8.))`).
    min_width: Pixels,
}

impl Opening {
    /// The place of the submenu that would like `width`: on the right side every submenu starts
    /// at the same place; on the left side it ends at the same edge, so it starts at its own.
    pub(crate) fn child_for(&self, width: Pixels) -> Place {
        match self.left_edge {
            None => self.child,
            Some(edge) => Place {
                left: (edge - width.max(self.min_width).min(self.child.room))
                    .max(self.child.margin),
                ..self.child
            },
        }
    }
}

/// How a menu at `place`, whose widest item needs `own` and whose widest submenu would like
/// `child` (zero without one), opens its submenus in a window `window_width` wide.
///
/// The parent is first moved into the window as its `anchored()` does. The submenu starts
/// `OVERLAP` before the right edge of its parent and, on the left side, ends `LEFT_GAP` before
/// the parent's left edge. It goes right when it fits there or the room on the right is not
/// smaller than on the left.
fn opening(
    window_width: Pixels,
    place: Place,
    own: Pixels,
    child: Pixels,
    min_width: Pixels,
) -> Opening {
    let default = px(DEFAULT_MAX_WIDTH);
    // A list is never drawn narrower than `min_width`, whatever its labels need.
    let own = own.max(min_width);
    let child = if child > Pixels::ZERO {
        child.max(min_width)
    } else {
        child
    };
    // Where the menu is drawn: moved left when it would pass the right edge, then kept off the
    // left edge (`snap_to_window_with_margin`).
    let width = own.min(place.room);
    let left = place
        .left
        .min(window_width - place.margin - width)
        .max(place.margin);
    let place = Place { left, ..place };
    if child <= Pixels::ZERO {
        return Opening {
            max_width: default.max(own),
            child: place,
            left_edge: None,
            min_width,
        };
    }
    let margin = px(MARGIN);
    let right_left = place.left + width - px(OVERLAP);
    let right_room = window_width - margin - right_left;
    let left_room = place.left - px(LEFT_GAP) - margin;
    if child <= right_room || right_room >= left_room {
        // gpui-component goes right while `max_w` and the left edge stay inside the window.
        let max_width = own.max(default.min(window_width - place.left));
        return Opening {
            max_width,
            child: Place {
                left: right_left,
                room: right_room,
                margin,
            },
            left_edge: None,
            min_width,
        };
    }
    let width = child.min(left_room);
    Opening {
        max_width: own.max(window_width - place.left + px(LEFT_SAFETY)),
        child: Place {
            left: (place.left - px(LEFT_GAP) - width).max(margin),
            room: left_room,
            margin,
        },
        left_edge: Some(place.left - px(LEFT_GAP)),
        min_width,
    }
}

/// The measure of the text of the menus, and the room a menu has.
#[derive(Clone, Copy)]
pub(crate) struct Fit<'a> {
    window: &'a Window,
    place: Place,
    layout: Layout,
}

impl<'a> Fit<'a> {
    pub(crate) fn new(window: &'a Window, place: Place, layout: Layout) -> Self {
        // A list is drawn at least `min_w(rems(8.))` wide whatever room it has: that is the floor
        // of the room, so a window too small for it and a submenu with less room than it both
        // get the width gpui-component will draw; any room above it is used as it is.
        let room = place.room.max(window.rem_size() * MIN_REMS);
        Self {
            window,
            place: Place { room, ..place },
            layout,
        }
    }

    /// The same, for the rows of another menu (a submenu measured by its parent).
    pub(crate) fn with(&self, layout: Layout) -> Fit<'a> {
        Fit { layout, ..*self }
    }

    fn measure(&self, text: &str) -> Pixels {
        let style = self.window.text_style();
        let run = style.to_run(text.len());
        self.window
            .text_system()
            .layout_line(text, self.window.rem_size() * TEXT_REM, &[run], None)
            .width
    }

    /// `label`, cut with `…` if it needs more than the menu's room; borrowed when it fits.
    pub(crate) fn label<'l>(&self, label: &'l str, row: Row) -> Cow<'l, str> {
        shorten(label, self.place.room - self.layout.extra(row), &|text| {
            self.measure(text)
        })
    }

    /// [`label`](Self::label) for a text the caller owns: kept as it is when it fits, so that
    /// nothing is copied.
    pub(crate) fn owned(&self, label: String, row: Row) -> String {
        match self.label(&label, row) {
            Cow::Owned(cut) => cut,
            Cow::Borrowed(_) => label,
        }
    }

    /// The width a menu needs for the (already shortened) label `label`.
    pub(crate) fn width_of(&self, label: &str, row: Row) -> Pixels {
        self.measure(label) + self.layout.extra(row)
    }

    /// The width a submenu would like for `label` if it had all the room of the window.
    pub(crate) fn natural(&self, label: &str, row: Row) -> Pixels {
        let most = self.window.viewport_size().width - px(MARGIN * 2.0);
        self.width_of(label, row)
            .max(self.window.rem_size() * MIN_REMS)
            .min(most)
    }

    /// How the menu opens its submenus; `own` is the width its widest item needs and `child` the
    /// width its widest submenu would like (zero without one).
    pub(crate) fn opening(&self, own: Pixels, child: Pixels) -> Opening {
        let min_width = self.window.rem_size() * MIN_REMS;
        opening(
            self.window.viewport_size().width,
            self.place,
            own,
            child,
            min_width,
        )
    }
}

/// `label` cut to `room`: itself when it fits. A path (two separators or more) keeps its start
/// and its last part, `1 \\server\share\…\file.md  [UTF-8]`; anything else keeps its start.
fn shorten<'a>(label: &'a str, room: Pixels, measure: &dyn Fn(&str) -> Pixels) -> Cow<'a, str> {
    if measure(label) <= room {
        return Cow::Borrowed(label);
    }
    let chars: Vec<char> = label.chars().collect();
    let is_separator = |c: &char| matches!(c, '\\' | '/');
    let separators = chars.iter().filter(|c| is_separator(c)).count();
    // The tail of a path: its last part, from the last separator.
    let tail_at = chars.iter().rposition(is_separator).unwrap_or(chars.len());
    let tail: String = chars[tail_at..].iter().collect();
    let ellipsis = measure("…");
    // The start keeps some room: a tail that takes it all is cut at the end instead.
    let head_room = room - measure(&tail) - ellipsis;
    if separators >= 2 && head_room >= px(MIN_HEAD) {
        let head = longest(&chars[..tail_at], head_room, measure);
        return Cow::Owned(format!("{head}{ELLIPSIS}{tail}"));
    }
    let head = longest(&chars, room - ellipsis, measure);
    Cow::Owned(format!("{head}{ELLIPSIS}"))
}

/// The longest start of `chars` that fits `room`.
fn longest(chars: &[char], room: Pixels, measure: &dyn Fn(&str) -> Pixels) -> String {
    let (mut fits, mut over) = (0, chars.len() + 1);
    // Invariant: `chars[..fits]` fits; `chars[..over]` does not (or is past the end).
    while over - fits > 1 {
        let mid = fits + (over - fits) / 2;
        let text: String = chars[..mid].iter().collect();
        if measure(&text) <= room {
            fits = mid;
        } else {
            over = mid;
        }
    }
    chars[..fits].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ten pixels for a character.
    fn tens(text: &str) -> Pixels {
        px(10.0 * text.chars().count() as f32)
    }

    #[test]
    fn a_label_that_fits_is_left_alone() {
        assert_eq!(shorten("ファイル(F)", px(200.0), &tens), "ファイル(F)");
        // Exactly as wide as the room.
        assert_eq!(shorten("abcde", px(50.0), &tens), "abcde");
    }

    #[test]
    fn a_long_label_is_cut_at_the_end() {
        let shown = shorten("a very long label that goes on", px(100.0), &tens);
        assert_eq!(shown, "a very lo…");
        assert!(tens(&shown) <= px(100.0));
    }

    #[test]
    fn a_path_keeps_its_start_and_its_file_name() {
        let label =
            r"1 \\wsl.localhost\Ubuntu\home\naomasa\project\tmp\EDGE_SUBMISSION.md  [UTF-8]";
        let shown = shorten(label, px(500.0), &tens);
        assert!(shown.starts_with(r"1 \\wsl.localhost"), "{shown}");
        assert!(shown.ends_with(r"\EDGE_SUBMISSION.md  [UTF-8]"), "{shown}");
        assert!(shown.contains('…'), "{shown}");
        // As much of the start as fits: the result is as wide as the room allows.
        assert!(tens(&shown) <= px(500.0), "{shown}");
        assert!(tens(&shown) > px(490.0), "{shown}");
    }

    #[test]
    fn a_path_whose_file_name_takes_all_the_room_is_cut_at_the_end() {
        let shown = shorten(
            r"C:\dir\a-file-name-that-is-very-long.txt",
            px(100.0),
            &tens,
        );
        assert_eq!(shown, r"C:\dir\a-…");
    }

    #[test]
    fn a_label_with_one_separator_is_not_taken_for_a_path() {
        let shown = shorten("Cut/Copy and many other long words", px(100.0), &tens);
        assert_eq!(shown, "Cut/Copy …");
    }

    fn place(left: f32) -> Place {
        Place {
            left: px(left),
            room: px(584.0),
            margin: px(8.0),
        }
    }

    #[test]
    fn a_menu_without_a_submenu_keeps_the_default_width() {
        let w = px(600.0);
        let max = |own: f32| opening(w, place(0.0), px(own), px(0.0), px(128.0)).max_width;
        assert_eq!(max(250.0), px(500.0));
        assert_eq!(max(700.0), px(700.0));
    }

    #[test]
    fn a_submenu_that_does_not_fit_opens_on_the_right_with_the_room_there() {
        // A menu at the left edge (moved in to 8), 200 wide: the submenu starts at 200 and has
        // the rest of the window less the margin; the left side has no room.
        let open = opening(px(600.0), place(0.0), px(200.0), px(584.0), px(128.0));
        assert_eq!(open.child.left, px(200.0));
        assert_eq!(open.child.room, px(396.0));
        // gpui-component takes the right: its left edge and `max_w` stay inside the window.
        assert!(open.max_width <= px(600.0));
    }

    #[test]
    fn a_menu_near_the_right_edge_opens_its_submenu_on_the_left_with_the_room_there() {
        // The menu is moved in from 400 to 392 (600 - 8 - 200).
        let open = opening(px(600.0), place(400.0), px(200.0), px(584.0), px(128.0));
        // The room on the left: from the margin to 16 px before the parent's left edge.
        assert_eq!(open.child.room, px(372.0));
        assert_eq!(open.child.left, px(4.0));
        assert_eq!(open.child.left + open.child.room, px(392.0 - 16.0));
        // gpui-component takes the left: `max_w` and the left edge pass the window's edge.
        assert!(open.max_width + px(392.0) > px(600.0));
    }

    #[test]
    fn a_menu_opened_past_the_right_edge_is_measured_where_it_is_moved_to() {
        // Asked to open at 580, drawn at 392: the room on its left is that of 392, not of 580.
        let moved = opening(px(600.0), place(580.0), px(200.0), px(584.0), px(128.0));
        let there = opening(px(600.0), place(392.0), px(200.0), px(584.0), px(128.0));
        assert_eq!(moved, there);
        assert_eq!(moved.child.room, px(372.0));
    }

    #[test]
    fn a_submenu_that_fits_on_the_right_goes_there() {
        let open = opening(px(1000.0), place(300.0), px(200.0), px(300.0), px(128.0));
        assert_eq!(open.child.left, px(492.0));
        assert!(open.max_width + px(300.0) <= px(1000.0));
    }

    #[test]
    fn a_narrower_sibling_on_the_left_starts_farther_right() {
        let open = opening(px(600.0), place(400.0), px(200.0), px(584.0), px(128.0));
        // Left side: every submenu ends 16 px before the parent (392).
        let wide = open.child_for(px(584.0));
        let narrow = open.child_for(px(200.0));
        assert_eq!(wide, open.child);
        assert_eq!(narrow.left, px(392.0 - 16.0 - 200.0));
        assert_eq!(narrow.left + px(200.0), wide.left + wide.room);
        // On the right side they all start at the same place.
        let right = opening(px(1000.0), place(300.0), px(200.0), px(300.0), px(128.0));
        assert_eq!(right.child_for(px(100.0)), right.child);
    }

    #[test]
    fn a_submenu_with_short_labels_is_placed_as_wide_as_the_least_gpui_component_draws() {
        let open = opening(px(600.0), place(400.0), px(200.0), px(584.0), px(128.0));
        // 40 px of labels are drawn 128 px wide: the same place as a width of 128.
        assert_eq!(open.child_for(px(40.0)), open.child_for(px(128.0)));
        assert_eq!(open.child_for(px(40.0)).left, px(392.0 - 16.0 - 128.0));
        // The parent and the widest submenu are clamped too: a short parent (50 px of labels)
        // opens its submenu from where a 128 px list would, and goes the same side.
        let short = opening(px(600.0), place(400.0), px(50.0), px(584.0), px(128.0));
        let full = opening(px(600.0), place(400.0), px(128.0), px(584.0), px(128.0));
        assert_eq!(short, full);
    }
}
