//! Paste encoding (ADR-0028 point 4): what is written to the terminal for pasted text.

use crate::screen::Modes;

const BRACKET_START: &[u8] = b"\x1b[200~";
const BRACKET_END: &[u8] = b"\x1b[201~";

/// The result of [`encode_paste`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasteEncoding {
    /// The bytes to write to the terminal (empty when nothing is left to paste).
    pub bytes: Vec<u8>,
    /// The text has more than one line and the program has not asked for bracketed paste: the line
    /// breaks would run as commands, so the UI asks the user before it writes `bytes`.
    pub needs_confirmation: bool,
}

/// Encodes pasted text.
///
/// - Every control character except line breaks and tabs is removed (C0, DEL and C1). ESC in particular is
///   removed, so the text cannot contain the end marker of a bracketed paste (`ESC [ 201 ~`) and escape from
///   it, nor any other escape sequence.
/// - CRLF, CR and LF are one line break. Without bracketed paste a line break is sent as CR, as xterm does:
///   it must act like pressing Enter, which a terminal sends as CR. In bracketed paste mode the program
///   receives the text between the markers as data, so a line break is sent as LF.
/// - With bracketed paste mode on, the text is wrapped in `ESC [ 200 ~` and `ESC [ 201 ~`.
/// - [`PasteEncoding::needs_confirmation`] is set when the text has more than one line (one trailing line
///   break does not count as another line) and bracketed paste is off.
pub fn encode_paste(text: &str, modes: &Modes) -> PasteEncoding {
    let bracketed = modes.bracketed_paste;
    let line_break = if bracketed { '\n' } else { '\r' };

    let mut cleaned = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut breaks = 0usize;
    let mut ends_with_break = false;
    while let Some(c) = chars.next() {
        ends_with_break = false;
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                cleaned.push(line_break);
                breaks += 1;
                ends_with_break = true;
            }
            '\n' => {
                cleaned.push(line_break);
                breaks += 1;
                ends_with_break = true;
            }
            '\t' => cleaned.push('\t'),
            c if c.is_control() => {}
            c => cleaned.push(c),
        }
    }

    if cleaned.is_empty() {
        return PasteEncoding {
            bytes: Vec::new(),
            needs_confirmation: false,
        };
    }

    let lines = breaks + 1 - usize::from(ends_with_break);
    let mut bytes = Vec::with_capacity(cleaned.len() + BRACKET_START.len() + BRACKET_END.len());
    if bracketed {
        bytes.extend_from_slice(BRACKET_START);
    }
    bytes.extend_from_slice(cleaned.as_bytes());
    if bracketed {
        bytes.extend_from_slice(BRACKET_END);
    }
    PasteEncoding {
        bytes,
        needs_confirmation: !bracketed && lines > 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFF: Modes = Modes::DEFAULT;
    const ON: Modes = Modes {
        bracketed_paste: true,
        ..Modes::DEFAULT
    };

    #[test]
    fn single_line_without_brackets() {
        let p = encode_paste("echo hi", &OFF);
        assert_eq!(p.bytes, b"echo hi");
        assert!(!p.needs_confirmation);
    }

    #[test]
    fn bracketed_paste_wraps_the_text() {
        let p = encode_paste("日本語", &ON);
        assert_eq!(p.bytes, "\x1b[200~日本語\x1b[201~".as_bytes());
        assert!(!p.needs_confirmation);
    }

    #[test]
    fn escape_cannot_end_the_bracket() {
        let p = encode_paste("a\x1b[201~rm -rf /\n", &ON);
        assert_eq!(p.bytes, b"\x1b[200~a[201~rm -rf /\n\x1b[201~");
    }

    #[test]
    fn controls_are_removed_but_tabs_and_line_breaks_stay() {
        let p = encode_paste("a\x00b\x07c\x7fd\u{9b}e\tf", &OFF);
        assert_eq!(p.bytes, b"abcde\tf");
        let p = encode_paste("x\x08y\x0cz", &ON);
        assert_eq!(p.bytes, b"\x1b[200~xyz\x1b[201~");
    }

    #[test]
    fn line_breaks_are_cr_without_brackets() {
        let p = encode_paste("a\r\nb\nc\rd", &OFF);
        assert_eq!(p.bytes, b"a\rb\rc\rd");
        assert!(p.needs_confirmation);
    }

    #[test]
    fn line_breaks_are_lf_in_brackets() {
        let p = encode_paste("a\r\nb\nc\rd", &ON);
        assert_eq!(p.bytes, b"\x1b[200~a\nb\nc\nd\x1b[201~");
        assert!(!p.needs_confirmation);
    }

    #[test]
    fn one_trailing_line_break_is_one_line() {
        let p = encode_paste("ls\n", &OFF);
        assert_eq!(p.bytes, b"ls\r");
        assert!(!p.needs_confirmation);
        let p = encode_paste("ls\r\n", &OFF);
        assert_eq!(p.bytes, b"ls\r");
        assert!(!p.needs_confirmation);
        let p = encode_paste("ls\n\n", &OFF);
        assert!(p.needs_confirmation);
    }

    #[test]
    fn nothing_left_is_nothing_to_write() {
        for modes in [OFF, ON] {
            let p = encode_paste("", &modes);
            assert!(p.bytes.is_empty());
            let p = encode_paste("\x1b\x07", &modes);
            assert!(p.bytes.is_empty());
            assert!(!p.needs_confirmation);
        }
    }

    #[test]
    fn a_text_of_line_breaks_only_is_multi_line() {
        let p = encode_paste("\n\n\n", &OFF);
        assert_eq!(p.bytes, b"\r\r\r");
        assert!(p.needs_confirmation);
    }
}
