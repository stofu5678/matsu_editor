//! The shape of the caret (`CCaret::ShowEditCaret`, `sakura_core/view/CCaret.cpp`).
//!
//! The common setting "カーソル形状" (`nCaretType`, the low 4 bits) is the Windows style (0) or the
//! MS-DOS style (1); the typing mode (`IsInsMode`) is insert or overwrite:
//!
//! | style   | insert                          | overwrite                        |
//! |---------|---------------------------------|----------------------------------|
//! | Windows | a bar, 2 px or the OS width     | a block as wide as the char      |
//! | MS-DOS  | a block as wide as the char, half as high | a block as wide as the char |
//!
//! The OS width is `SPI_GETCARETWIDTH` ("システムの設定に従う(けど2px以上)", 2011.12.22): when
//! it is more than 2 px the bar is that wide, but not wider than a half-width column. Only Windows
//! has the setting (`matsu_platform::caret_width`); elsewhere the bar is 2 px.
//!
//! A block is as wide as the char under the caret, and one half-width column at the end of a line
//! or the document and over a tab; the block of the MS-DOS style is the same, so its width is
//! `overwrite_caret_width` of the element. The caret sits at the bottom of the row
//! (`CalcCaretDrawPos`: "下寄せ").

use matsu_layout::X;

/// The width of the bar of the Windows style, in pixels, when the OS has no other say.
pub(crate) const BAR_WIDTH: f32 = 2.0;

/// How the caret is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CaretShape {
    /// A bar, full height, this many pixels wide, in the caret color.
    Bar { width: f32 },
    /// A see-through block, `width` wide, at the bottom of the row; `half` makes it half as high.
    Block { width: X, half: bool },
}

/// The shape of the caret for the style (`dos`), the typing mode (`overwrite`), the width of the
/// char under it (`width`), the width of a column in pixels (`cell`) and the width the OS gives the
/// caret (`os_width`, in pixels).
pub(crate) fn caret_shape(
    dos: bool,
    overwrite: bool,
    width: X,
    cell: f32,
    os_width: Option<u32>,
) -> CaretShape {
    match (dos, overwrite) {
        (false, false) => {
            // `SystemParametersInfo(SPI_GETCARETWIDTH)`: only a width over 2 px counts, and it is
            // cut at one half-width column.
            let width = os_width
                .filter(|w| *w as f32 > BAR_WIDTH)
                .map_or(BAR_WIDTH, |w| (w as f32).min(cell));
            CaretShape::Bar { width }
        }
        (_, true) => CaretShape::Block { width, half: false },
        (true, false) => CaretShape::Block { width, half: true },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn x(columns: u32) -> X {
        X::from_raw(columns * 64)
    }

    #[test]
    fn the_windows_style_is_a_bar_in_insert_mode_and_a_block_in_overwrite_mode() {
        assert_eq!(
            caret_shape(false, false, x(2), 8.0, None),
            CaretShape::Bar { width: 2.0 }
        );
        assert_eq!(
            caret_shape(false, true, x(2), 8.0, None),
            CaretShape::Block {
                width: x(2),
                half: false
            }
        );
    }

    #[test]
    fn the_dos_style_is_a_block_half_as_high_in_insert_mode() {
        assert_eq!(
            caret_shape(true, false, x(1), 8.0, None),
            CaretShape::Block {
                width: x(1),
                half: true
            }
        );
        assert_eq!(
            caret_shape(true, true, x(2), 8.0, None),
            CaretShape::Block {
                width: x(2),
                half: false
            }
        );
        // the width of the OS does not touch the DOS style
        assert_eq!(
            caret_shape(true, false, x(1), 8.0, Some(6)),
            CaretShape::Block {
                width: x(1),
                half: true
            }
        );
    }

    #[test]
    fn the_os_width_counts_over_two_pixels_and_up_to_a_column() {
        let bar = |os| caret_shape(false, false, x(1), 8.0, os);
        assert_eq!(bar(Some(1)), CaretShape::Bar { width: 2.0 });
        assert_eq!(bar(Some(2)), CaretShape::Bar { width: 2.0 });
        assert_eq!(bar(Some(5)), CaretShape::Bar { width: 5.0 });
        assert_eq!(bar(Some(20)), CaretShape::Bar { width: 8.0 });
    }
}
