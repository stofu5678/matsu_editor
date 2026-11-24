//! Which row of a combo box's history list the arrow keys go to.
//!
//! The find, replace, Grep, external command and tag jump boxes are Windows combo boxes
//! (`CBS_DROPDOWN`, `sakura_rc.rc`). Measured on Windows 11 with the system's drop-down combo box:
//! Up and Down move the highlighted row, in an open list or a closed one, and put the row's text
//! in the box at once; with a text that is not a row nothing is highlighted, and either key takes
//! the first row; Down stops at the last row and Up at the first.

/// The row that holds `text`, if there is one: the row that an opened list highlights.
pub(crate) fn row_of(rows: &[String], text: &str) -> Option<usize> {
    rows.iter().position(|row| row == text)
}

/// The row that Up or Down goes to from `highlight` in a list of `len` rows (more than none).
pub(crate) fn arrow_row(highlight: Option<usize>, down: bool, len: usize) -> usize {
    match (highlight, down) {
        (Some(row), true) => (row + 1).min(len - 1),
        (Some(row), false) => row.saturating_sub(1),
        (None, _) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_row_takes_the_first_with_either_key() {
        assert_eq!(arrow_row(None, true, 3), 0);
        assert_eq!(arrow_row(None, false, 3), 0);
    }

    #[test]
    fn the_ends_stay() {
        assert_eq!(arrow_row(Some(2), true, 3), 2);
        assert_eq!(arrow_row(Some(0), false, 3), 0);
        assert_eq!(arrow_row(Some(0), true, 3), 1);
        assert_eq!(arrow_row(Some(2), false, 3), 1);
    }

    #[test]
    fn the_row_of_a_text() {
        let rows = vec!["a".to_owned(), "b".to_owned()];
        assert_eq!(row_of(&rows, "b"), Some(1));
        assert_eq!(row_of(&rows, "typed"), None);
    }
}
