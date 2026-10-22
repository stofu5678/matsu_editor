//! Line layout of MATSU Editor: it wraps one line of text into rows and maps byte offsets to
//! display columns the way Sakura Editor does ([ADR-0013], the part that the screen and the
//! printing share).
//!
//! It knows nothing of pixels, fonts, a UI, or an OS. Everything is counted in columns: a
//! half-width character takes 1 and a full-width one 2 (see [`WidthRule`]). The caller draws a
//! column as a cell of the font's half-width.
//!
//! ```
//! use matsu_layout::{layout_line, LayoutConfig, Wrap};
//!
//! let cfg = LayoutConfig { wrap: Wrap::Columns(10), ..LayoutConfig::default() };
//! let mut rows = Vec::new();
//! layout_line("日本語のテキスト", false, &cfg, &mut rows);
//! assert_eq!(rows.len(), 2);
//! assert_eq!(rows[0].text("日本語のテキスト"), "日本語のテ");
//! ```
//!
//! # What it does
//!
//! - [`layout_line`] wraps a line like `CLayoutMgr::_MakeOneLine`
//!   (`sakura_core/doc/layout/CLayoutMgr_DoLayout.cpp`): at the wrap column or not at all, with
//!   tab stops, 英文ワードラップ, 行頭禁則, 行末禁則, 句読点のぶら下げ, 改行文字のぶら下げ, and the
//!   indent of wrapped rows (折り返し行のインデント).
//! - [`offset_to_position`] and [`position_to_offset`] convert like `LogicToLayout` and
//!   `LayoutToLogicEx` (`sakura_core/doc/layout/CLayoutMgr.cpp`), for the caret and the mouse.
//! - [`DocumentLayout`] lays out a whole `matsu_core::buffer::Document`: the rows of every line, the
//!   mapping between rows and lines, the EOF row, positions on the screen, updates after edits,
//!   and a first layout in chunks.
//!
//! # The width rule and where it differs from Sakura
//!
//! Sakura asks the font whether a character is wider than a half-width letter
//! (`CCharWidthCache::CalcHankakuByFont`), so its layout changes with the font and the OS. This
//! crate decides from Unicode alone ([`WidthRule`]), to give the same rows on every OS. It
//! matches Sakura with a Japanese monospaced font (MS Gothic and the like). Known differences:
//!
//! - Characters that the font draws at another width: proportional fonts, fonts with half-width
//!   Greek or Cyrillic, symbols drawn half-width (`½`, `·`), and the like. Sakura follows the
//!   font; this crate follows East Asian Width (Ambiguous = 2 by default, set
//!   [`WidthRule::ambiguous_wide`] to change it; Latin-1 and Latin Extended letters are 1
//!   whatever it says, except the JIS X 0208 symbols `§ ¨ ° ± ´ ¶ × ÷`).
//! - Combining marks, variation selectors and zero-width characters take 1 column here, as
//!   Sakura draws each UTF-16 unit in its own cell. The width of the glyph in the font may differ.
//! - Emoji outside the BMP take 2, as `CNativeW::GetKetaOfChar` says (it ends up with the width
//!   of the pair in the font in `GetHabaOfChar`, which can be more). Emoji inside the BMP follow
//!   East Asian Width (`☺`, 1; `⌚`, 2).
//! - Sakura's lone surrogate stand-ins for invalid bytes (U+DC00..U+DCFF) are 1 column in
//!   `GetKetaOfChar`. Ours are the U+10FF00..U+10FFFF stand-ins, drawn as a box with two hex
//!   digits, so they take 2 ([`WidthRule::invalid_byte`] changes that).
//! - A tab takes `tab - x % tab` columns, always counted from the left edge of the screen, as
//!   Sakura (the indent is included in `x`).
//! - Control characters take 1 column. Sakura measures a proxy character for them and also gets
//!   about 1 column.
//!
//! # Not done here
//!
//! The scroll position, TSV/CSV mode (`TSV_MODE_*`), the character spacing (`m_nSpacing`) and the
//! colouring. A line ending is the flag `has_eol`; the text of a line never holds it.
//!
//! [ADR-0013]: https://github.com/stofu5678/ume_editor/blob/main/docs/adr/0013-module-structure.md

mod config;
pub mod document;
mod kinsoku;
mod map;
mod width;
mod wrap;

pub use config::{LayoutConfig, MAX_LINE_COLUMNS, MIN_LINE_COLUMNS, Wrap, WrapIndent};
pub use document::{DocumentLayout, LineLayout, Progress, RowInfo, RowPos, RowRef, RowTarget};
pub use kinsoku::{CharSet, DEFAULT_HEAD, DEFAULT_KUTO, DEFAULT_TAIL, Kinsoku};
pub use map::{Located, Position, offset_to_position, position_to_offset};
pub use width::{INVALID_BYTE_BASE, WidthRule};
pub use wrap::{Row, layout_line};

#[cfg(test)]
mod tests;
