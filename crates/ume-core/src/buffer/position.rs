//! Positions in a document and column conversions.

/// A position in a document.
///
/// `line` is 0-based. `col` is a byte offset into the decoded UTF-8 text of the line, without its
/// line ending, and is on a char boundary. Positions that do not satisfy this are clamped by every
/// method that takes one; they never cause a panic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

impl Pos {
    pub const fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

/// The number of UTF-16 code units of `text`.
pub fn utf16_len(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

/// Converts a byte column of `line` to UTF-16 code units. A column past the end counts the whole
/// line. A column inside a char counts up to the start of that char.
///
/// Sakura's macro API counts columns in UTF-16 units (`sakura_core/macro/CMacro.cpp`).
pub fn col_to_utf16(line: &str, col: usize) -> usize {
    let mut units = 0;
    for (i, ch) in line.char_indices() {
        if i + ch.len_utf8() > col {
            break;
        }
        units += ch.len_utf16();
    }
    units
}

/// Converts UTF-16 code units to a byte column of `line`. Units past the end give the line length.
/// Units that point into the middle of a surrogate pair give the start of that char.
pub fn col_from_utf16(line: &str, units: usize) -> usize {
    let mut seen = 0;
    for (i, ch) in line.char_indices() {
        let next = seen + ch.len_utf16();
        if next > units {
            return i;
        }
        seen = next;
    }
    line.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_conversions() {
        let s = "a日😀b";
        assert_eq!(utf16_len(s), 5);
        assert_eq!(col_to_utf16(s, 0), 0);
        assert_eq!(col_to_utf16(s, 1), 1);
        assert_eq!(col_to_utf16(s, 4), 2);
        assert_eq!(col_to_utf16(s, 8), 4);
        assert_eq!(col_to_utf16(s, 9), 5);
        assert_eq!(col_to_utf16(s, 100), 5);
        // inside a char: rounds down to the start of the char
        assert_eq!(col_to_utf16(s, 2), 1);
        assert_eq!(col_from_utf16(s, 0), 0);
        assert_eq!(col_from_utf16(s, 2), 4);
        assert_eq!(col_from_utf16(s, 3), 4); // middle of the surrogate pair
        assert_eq!(col_from_utf16(s, 4), 8);
        assert_eq!(col_from_utf16(s, 5), 9);
        assert_eq!(col_from_utf16(s, 99), 9);
    }

    #[test]
    fn pos_orders_by_line_first() {
        assert!(Pos::new(0, 9) < Pos::new(1, 0));
    }
}
