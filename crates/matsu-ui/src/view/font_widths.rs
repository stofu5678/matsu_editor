//! The widths of the characters of the editor's font, for the layout (ADR-0034, stage 4).
//!
//! A monospaced font is laid out on the grid of the width rule and is not measured (ADR-0034,
//! decided 3: nothing changes for it). Any other font gets a table of widths that is filled by
//! measuring one character at a time, as Sakura's `CCharWidthCache` does
//! (`sakura_core/charset/charcode.cpp`).
//!
//! The unit of the table is the layout's `X`: the advance of `0` is a half-width letter and is
//! [`X_PER_HALF_WIDTH`] (64) of them, and every other advance is its ratio to it, rounded.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use gpui::{Font, Pixels, SharedString, TextSystem};
use matsu_layout::Widths;

use crate::geometry::X_PER_HALF_WIDTH;

/// The characters whose advances tell whether a font is monospaced (ADR-0034, "3."). The last is
/// the half-width letter that the unit is made of.
pub(crate) const PROBES: [char; 6] = ['i', 'l', '.', 'M', 'W', '0'];

/// How much the advances of the probes may differ in a monospaced font, in pixels.
const MONOSPACE_TOLERANCE_PX: f32 = 0.01;

/// Whether `advances` (in pixels) are the same within [`MONOSPACE_TOLERANCE_PX`]. Fewer than two
/// cannot tell, so they count as the same.
pub(crate) fn advances_are_equal(advances: &[f32]) -> bool {
    let widest = advances.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let narrowest = advances.iter().copied().fold(f32::INFINITY, f32::min);
    advances.len() < 2 || widest - narrowest <= MONOSPACE_TOLERANCE_PX
}

/// The character whose advance stands for a control character: Sakura measures a control code
/// by a proxy, a space for NUL and `･` (U+FF65) for the others, and takes the larger of the two
/// widths (`CCharWidthCache::QueryPixelWidth`, "2014.12.21 コントロールコードの表示・NULが1px幅に
/// なるのをスペース幅にする"). `None` for a character that is measured as it is.
fn proxy_of(c: char) -> Option<char> {
    // A tab is placed by the tab stops and a line ending takes one column; neither is measured.
    if c.is_control() && !matches!(c, '\t' | '\n' | '\r') {
        Some(if c == '\0' { ' ' } else { '\u{FF65}' })
    } else {
        None
    }
}

/// `advance` in `X` for a half-width letter of `half` pixels, at least 0.
pub(crate) fn x_of_advance(advance: f32, half: f32) -> u32 {
    if half <= 0.0 || advance <= 0.0 {
        return 0;
    }
    (advance / half * X_PER_HALF_WIDTH as f32).round().max(0.0) as u32
}

/// The widths for a font whose advances (in pixels) are given by `advance`, or `None` when the font
/// is monospaced and the grid is used. `space_units` is the character spacing in `X`.
///
/// `advance` is called for the probes now and for each character the layout uses later, from any
/// thread.
pub(crate) fn widths_for(
    advance: impl Fn(char) -> f32 + Send + Sync + 'static,
    space_units: u32,
) -> Option<Widths> {
    let probes = PROBES.map(&advance);
    let half = advance('0');
    if half <= 0.0 || advances_are_equal(&probes) {
        return None;
    }
    Some(Widths::table(X_PER_HALF_WIDTH, space_units, move |c| {
        let own = advance(c);
        let measured = match proxy_of(c) {
            Some(proxy) => own.max(advance(proxy)),
            None => own,
        };
        x_of_advance(measured, half)
    }))
}

/// What decides a table: the font, its size and the spacing. A change of any of them is a new
/// table (a new generation); the same again is the same table.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FontKey {
    pub family: SharedString,
    pub weight: u32,
    pub italic: bool,
    /// The size in pixels, as the bits of the `f32`.
    pub size_bits: u32,
    /// The character spacing of the type, in pixels.
    pub column_space: u32,
}

/// The font of the view and its widths: whether it is monospaced, and the table when it is not.
#[derive(Clone, Debug, Default)]
pub(crate) struct FontWidths {
    key: Option<FontKey>,
    table: Option<Widths>,
}

impl FontWidths {
    /// Whether this is the widths for `key` (the font has not changed since).
    pub fn is_for(&self, key: &FontKey) -> bool {
        self.key.as_ref() == Some(key)
    }

    /// The table, or `None` for the grid.
    pub fn table(&self) -> Option<&Widths> {
        self.table.as_ref()
    }

    /// The widths for `key` with `font` at `size`: from the cache of the process when this font
    /// was measured before (a pane that is made from another has the same table, and so the same
    /// generation), else measured now.
    pub fn measure(key: FontKey, font: &Font, size: Pixels, system: &Arc<TextSystem>) -> Self {
        static CACHE: OnceLock<Mutex<HashMap<FontKey, Option<Widths>>>> = OnceLock::new();
        let cache = CACHE.get_or_init(Default::default);
        if let Some(known) = cache.lock().ok().and_then(|cache| cache.get(&key).cloned()) {
            return Self {
                key: Some(key),
                table: known,
            };
        }
        let id = system.resolve_font(font);
        let for_advance = {
            let system = Arc::clone(system);
            move |c: char| f32::from(system.layout_width(id, size, c))
        };
        // The spacing is in units of the half-width letter, which is the advance of `0`.
        let half = for_advance('0');
        let space_units =
            crate::geometry::per_column_for(half, key.column_space) - X_PER_HALF_WIDTH;
        let table = widths_for(for_advance, space_units);
        if let Ok(mut cache) = cache.lock() {
            // A few sizes of a few fonts: the zoom steps through many sizes, so the cache is
            // emptied rather than grown without end (a table is up to 128 KiB).
            if cache.len() >= 24 {
                cache.clear();
            }
            cache.insert(key.clone(), table.clone());
        }
        Self {
            key: Some(key),
            table,
        }
    }

    /// Widths that say "monospaced" for `key`, without measuring (for tests).
    #[cfg(test)]
    pub fn grid_for(key: FontKey) -> Self {
        Self {
            key: Some(key),
            table: None,
        }
    }

    /// Widths of `table` for `key` (for tests).
    #[cfg(test)]
    pub fn table_for(key: FontKey, table: Widths) -> Self {
        Self {
            key: Some(key),
            table: Some(table),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A proportional font at 7 pixels for a half-width `0`.
    fn proportional(c: char) -> f32 {
        match c {
            'i' | 'l' | '.' => 3.5,
            'm' | 'M' => 11.0,
            'W' => 11.5,
            'あ' => 12.5,
            '漢' => 13.0,
            '･' => 7.0,
            ' ' => 3.0,
            c if c.is_control() => 0.0,
            _ => 7.0,
        }
    }

    #[test]
    fn a_font_with_equal_advances_is_monospaced() {
        assert!(advances_are_equal(&[7.0, 7.0, 7.0, 7.0, 7.0, 7.0]));
        assert!(advances_are_equal(&[7.0, 7.004, 6.997, 7.0, 7.0, 7.0]));
        assert!(!advances_are_equal(&[3.5, 3.5, 3.5, 11.0, 11.5, 7.0]));
        // Hinting that rounds one advance to the next pixel is not monospaced.
        assert!(!advances_are_equal(&[7.0, 7.0, 7.0, 7.02, 7.0, 7.0]));
        assert!(advances_are_equal(&[]));
        assert!(advances_are_equal(&[5.0]));
    }

    #[test]
    fn a_monospaced_font_gets_no_table() {
        assert!(widths_for(|c| if c == '漢' { 14.0 } else { 7.0 }, 0).is_none());
        // Not measurable (no advance of `0`): the grid stays.
        assert!(widths_for(|_| 0.0, 0).is_none());
    }

    #[test]
    fn a_proportional_font_gets_the_widths_of_its_advances_in_x() {
        let widths = widths_for(proportional, 0).expect("a table");
        assert!(widths.is_table());
        assert_eq!(widths.per_column(), 64);
        // the advance of `0` is a half-width letter, 64
        assert_eq!(widths.width_of('0').raw(), 64);
        assert_eq!(widths.width_of('i').raw(), 32);
        assert_eq!(widths.width_of('m').raw(), 101);
        assert_eq!(widths.width_of('W').raw(), 105);
        assert_eq!(widths.width_of('あ').raw(), 114);
        assert_eq!(widths.width_of('漢').raw(), 119);
    }

    #[test]
    fn spacing_is_added_in_units_of_the_half_width() {
        let widths = widths_for(proportional, 16).expect("a table");
        assert_eq!(widths.per_column(), 80);
        assert_eq!(widths.width_of('0').raw(), 64 + 16);
        // a wide character takes two columns of spacing
        assert_eq!(widths.width_of('漢').raw(), 119 + 32);
    }

    #[test]
    fn a_control_character_takes_the_width_of_its_proxy() {
        let widths = widths_for(proportional, 0).expect("a table");
        // The font draws it 0 wide; Sakura measures `･` (a half-width letter here) instead, and
        // a space for NUL.
        assert_eq!(widths.width_of('\u{1}').raw(), 64);
        assert_eq!(widths.width_of('\u{7F}').raw(), 64);
        assert_eq!(widths.width_of('\0').raw(), 27);
        // A control character that is wider than its proxy keeps its own width.
        let wide =
            widths_for(|c| if c == '\u{2}' { 20.0 } else { proportional(c) }, 0).expect("a table");
        assert_eq!(wide.width_of('\u{2}').raw(), 183);
    }

    #[test]
    fn no_advance_is_less_than_one_x() {
        let widths = widths_for(|c| if c == 'z' { 0.0 } else { proportional(c) }, 0).unwrap();
        assert_eq!(widths.width_of('z').raw(), 1);
        assert_eq!(x_of_advance(0.0, 7.0), 0);
        assert_eq!(x_of_advance(7.0, 0.0), 0);
        assert_eq!(x_of_advance(3.5, 7.0), 32);
    }

    #[test]
    fn the_key_tells_fonts_sizes_and_spacings_apart() {
        let key = FontKey {
            family: "Yu Gothic UI".into(),
            weight: 400,
            italic: false,
            size_bits: 13.0f32.to_bits(),
            column_space: 0,
        };
        let state = FontWidths::grid_for(key.clone());
        assert!(state.is_for(&key));
        for other in [
            FontKey {
                family: "Noto Sans JP".into(),
                ..key.clone()
            },
            FontKey {
                size_bits: 14.0f32.to_bits(),
                ..key.clone()
            },
            FontKey {
                column_space: 1,
                ..key.clone()
            },
            FontKey {
                italic: true,
                ..key.clone()
            },
            FontKey {
                weight: 700,
                ..key.clone()
            },
        ] {
            assert!(!state.is_for(&other), "{other:?}");
        }
        assert!(!FontWidths::default().is_for(&key));
    }
}
