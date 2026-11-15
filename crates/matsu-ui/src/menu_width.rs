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
//! gpui-component also uses `max_w` of a menu to choose the side its submenu opens on
//! (`update_submenu_menu_anchor`: left when `max_w` and the menu's left edge pass the window's
//! right edge), so [`max_width`] adds the width of the widest submenu to it: the submenu opens
//! on the right when it fits there, as Win32's do.

use std::borrow::Cow;

use gpui::{Pixels, Window, px};

/// What gpui-component allows a menu when nothing is set; a menu never gets less.
const DEFAULT_MAX_WIDTH: f32 = 500.0;
/// What a label needs besides its text: the padding, the column of the check mark or the icon,
/// the key of the item and the arrow of a submenu. Too much does no harm (`max_w` only limits);
/// too little would wrap a label.
const SLACK: f32 = 120.0;
/// The room kept to the window's edges.
const MARGIN: f32 = 8.0;
/// How far a submenu overlaps its parent (gpui-component: `bounds.size.width - px(8.)`).
const OVERLAP: f32 = 8.0;
/// The size of the text of an item (`text_sm`: 0.875 rem).
const TEXT_REM: f32 = 0.875;

const ELLIPSIS: char = '…';

/// The `max_w` of a menu in a window that leaves `cap`, whose widest item needs `own` and whose
/// widest submenu needs `submenu` (zero without one).
fn max_width(cap: Pixels, own: Pixels, submenu: Pixels) -> Pixels {
    let anchor = if submenu > Pixels::ZERO {
        own + submenu - px(OVERLAP)
    } else {
        Pixels::ZERO
    };
    px(DEFAULT_MAX_WIDTH).max(own).max(anchor).min(cap)
}

/// What fits in the window, and the measure of the text of the menus.
pub(crate) struct Fit<'a> {
    window: &'a Window,
    cap: Pixels,
}

impl<'a> Fit<'a> {
    pub(crate) fn new(window: &'a Window) -> Self {
        let cap = window.viewport_size().width - px(MARGIN * 2.0);
        Self {
            window,
            cap: cap.max(px(SLACK * 2.0)),
        }
    }

    fn measure(&self, text: &str) -> Pixels {
        let style = self.window.text_style();
        let run = style.to_run(text.len());
        self.window
            .text_system()
            .layout_line(text, self.window.rem_size() * TEXT_REM, &[run], None)
            .width
    }

    /// `label`, cut with `…` if it needs more than the window has.
    pub(crate) fn label(&self, label: &str) -> String {
        shorten(label, self.cap - px(SLACK), &|text| self.measure(text)).into_owned()
    }

    /// The width a menu needs for the (already shortened) label `label`.
    pub(crate) fn width_of(&self, label: &str) -> Pixels {
        self.measure(label) + px(SLACK)
    }

    /// The `max_w` of a menu whose widest item needs `own` and whose widest submenu needs
    /// `submenu` (zero without one).
    pub(crate) fn max_width(&self, own: Pixels, submenu: Pixels) -> Pixels {
        max_width(self.cap, own, submenu)
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
    if separators >= 2 && head_room >= room * 0.2 {
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

    #[test]
    fn the_room_of_a_menu_follows_its_longest_item_and_its_submenu() {
        let cap = px(1000.0);
        // A short menu keeps gpui-component's width; a long item widens it.
        assert_eq!(max_width(cap, px(250.0), px(0.0)), px(500.0));
        assert_eq!(max_width(cap, px(700.0), px(0.0)), px(700.0));
        // A wide submenu is counted, so that it opens where it fits.
        assert_eq!(max_width(cap, px(250.0), px(600.0)), px(842.0));
        // Never more than the window has.
        assert_eq!(max_width(cap, px(900.0), px(900.0)), cap);
    }
}
