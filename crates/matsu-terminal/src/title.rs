//! Cleaning of window titles set by the program in the terminal (OSC 0 and 2; ADR-0028 point 7).

/// The longest title kept, in characters.
pub const MAX_TITLE_CHARS: usize = 128;

/// Removes control characters and cuts the title to [`MAX_TITLE_CHARS`] characters.
///
/// Besides the C0, DEL and C1 controls, the bidirectional controls (U+202A..U+202E, U+2066..U+2069, U+200E,
/// U+200F, U+061C) and the line and paragraph separators are removed too: a program must not be able to
/// reorder or break the tab's label. Cutting happens after the removal, on a character boundary.
pub fn clean_title(raw: &str) -> String {
    raw.chars()
        .filter(|c| !is_unwanted(*c))
        .take(MAX_TITLE_CHARS)
        .collect()
}

/// Whether a character is taken out of what a program writes to be shown: the controls, the ones that reorder
/// text, and the ones that show nothing (zero-width), which make two different strings look alike.
pub(crate) fn is_unwanted(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{061C}'
                | '\u{200B}'..='\u{200F}'
                | '\u{2060}'
                | '\u{FEFF}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_zero_width_characters() {
        assert_eq!(
            clean_title("a\u{200B}b\u{200D}c\u{2060}d\u{FEFF}e"),
            "abcde"
        );
    }

    #[test]
    fn keeps_plain_text() {
        assert_eq!(clean_title("vim - 日本語.txt"), "vim - 日本語.txt");
        assert_eq!(clean_title(""), "");
    }

    #[test]
    fn removes_control_characters() {
        assert_eq!(clean_title("a\x1b[31mb\x07c\r\nd\te\x7f"), "a[31mbcde");
        assert_eq!(clean_title("a\u{9b}b\u{85}c"), "abc");
    }

    #[test]
    fn removes_bidi_and_separators() {
        assert_eq!(
            clean_title("a\u{202E}b\u{2066}c\u{200F}d\u{2028}e"),
            "abcde"
        );
    }

    #[test]
    fn cuts_to_the_limit_by_characters() {
        let long = "あ".repeat(MAX_TITLE_CHARS + 50);
        let cut = clean_title(&long);
        assert_eq!(cut.chars().count(), MAX_TITLE_CHARS);
        assert!(cut.chars().all(|c| c == 'あ'));
    }

    #[test]
    fn removal_happens_before_cutting() {
        let raw = format!("{}{}", "\x07".repeat(500), "x".repeat(MAX_TITLE_CHARS));
        assert_eq!(clean_title(&raw), "x".repeat(MAX_TITLE_CHARS));
    }
}
