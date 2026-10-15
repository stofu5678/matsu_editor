//! Text conversions as plain functions on strings, ported from `sakura_core/convert/`.
//!
//! The commands of the conversion group (`commands/convert/`) read the selection, call one of
//! these functions and write the result back (see
//! [`CommandContext::convert_selection`](crate::context::CommandContext::convert_selection)).
//! Nothing here knows the document or the cursor, so the functions are tested on strings.
//!
//! * [`case`]: lower and upper case by Sakura's ranges.
//! * [`kana`]: full and half width, hiragana and katakana, alphanumerics and symbols.
//! * [`tabs`]: tabs to spaces and spaces to tabs.
//! * [`datetime`]: the date and time pictures of `F_INS_DATE` and `F_INS_TIME`, and the clock.

pub mod case;
pub mod datetime;
pub mod kana;
pub mod tabs;
