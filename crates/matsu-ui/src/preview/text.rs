//! The text the preview shows: which format it is, and how much of the document it reads.
//!
//! The preview never takes the whole document as one string (CLAUDE.md): it reads line by line
//! through `Document::line` and stops at [`MAX_BYTES`], so a huge file costs the same as a
//! big one. What was left out is told in the header of the preview.

use std::path::Path;

use matsu_core::buffer::{Chunk, Document, Pos};

/// The most text the preview reads: 256 KiB. The parser behind the view turns the whole text into a
/// tree (on a background thread) each time the text changes. In a debug build, in the tests, a
/// README-like Markdown took 0.5 s for 16 KiB, 2.2 s for 64 KiB and 18 s for 256 KiB (a release
/// build is much faster; it was not measured), so the cap is where a large README ends.
pub(crate) const MAX_BYTES: usize = 256 << 10;

/// The largest document (in bytes of the file, as it will be saved) that is previewed at all: 2 MiB.
/// A bigger one shows only a notice. Reading a column of a line goes through the whole original
/// piece it is in, so for a file that is one huge line (or any file in one piece) the cost is that
/// of the file; this bound keeps it small without reading the file's own index.
pub(crate) const MAX_SOURCE_BYTES: usize = 2 << 20;

/// What the text is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    Markdown,
    Html,
}

impl Format {
    /// The name in the header of the preview.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Markdown => "Markdown",
            Self::Html => "HTML",
        }
    }
}

/// The format a file is shown in first: HTML for `.html`, `.htm` and `.xhtml`, Markdown for
/// anything else (a Markdown file, a plain text, a file with no name yet).
pub(crate) fn format_of(path: Option<&Path>) -> Format {
    let extension = path
        .and_then(Path::extension)
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    match extension.as_deref() {
        Some("html" | "htm" | "xhtml") => Format::Html,
        _ => Format::Markdown,
    }
}

/// The text read from a document.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Extract {
    /// The lines with `\n` after each, whatever the document's line ending is.
    pub(crate) text: String,
    /// The document has more than `text` holds.
    pub(crate) truncated: bool,
    /// The document is bigger than [`MAX_SOURCE_BYTES`] and nothing was read.
    pub(crate) too_large: bool,
}

/// Whether the document is at most `limit` bytes. It walks the pieces in chunks and stops as soon
/// as the sum passes `limit`, so a huge file is never read beyond that.
pub(crate) fn within_source_limit(doc: &Document) -> bool {
    fits(doc, MAX_SOURCE_BYTES)
}

fn fits(doc: &Document, limit: usize) -> bool {
    let mut total = 0usize;
    for chunk in doc.chunks_of(64 << 10) {
        let Ok(chunk) = chunk else {
            return false;
        };
        total += match chunk {
            Chunk::Original(bytes) => bytes.len(),
            Chunk::Added(text) => text.len(),
        };
        if total > limit {
            return false;
        }
    }
    true
}

/// Reads the lines of `doc` from the first until the text would pass `max_bytes`. A line that does
/// not fit is cut at a character boundary.
pub(crate) fn extract(doc: &Document, max_bytes: usize) -> Extract {
    if !fits(doc, MAX_SOURCE_BYTES) {
        return Extract {
            text: String::new(),
            truncated: true,
            too_large: true,
        };
    }
    let mut text = String::new();
    for i in 0..doc.line_count() {
        let room = max_bytes.saturating_sub(text.len());
        // Only as much of the line as can fit is read, so that a file that is one huge line
        // costs no more than the cap. Four bytes more than the room: a character is up to four
        // bytes, so a piece that is shorter than `room` is the whole line.
        let Ok(piece) = doc.text_between(Pos::new(i, 0), Pos::new(i, room.saturating_add(4)))
        else {
            break;
        };
        if piece.len() + 1 > room {
            let mut cut = room.min(piece.len());
            while !piece.is_char_boundary(cut) {
                cut -= 1;
            }
            text.push_str(&piece[..cut]);
            return Extract {
                text,
                truncated: true,
                too_large: false,
            };
        }
        text.push_str(&piece);
        text.push('\n');
    }
    Extract {
        text,
        truncated: false,
        too_large: false,
    }
}

/// The file an image of the document is loaded from, or `None` when it must not be loaded: only a
/// path (absolute, or relative to the folder of the document) is. An address (`http://`,
/// `file://`), a `//host/...` and an embedded `data:` image are left out, so that a preview
/// cannot fetch anything from the network.
pub(crate) fn local_image(src: &str, base: Option<&Path>) -> Option<std::path::PathBuf> {
    let src = src.trim();
    let lower = src.to_ascii_lowercase();
    // Two separators of either kind in a row (`\\server`, `/\server`, `\/server`, `//server`)
    // name a file on another computer on Windows: opening it sends the user's credentials over
    // the network, which a preview must not do.
    let b = src.as_bytes();
    if b.len() >= 2 && matches!(b[0], b'/' | b'\\') && matches!(b[1], b'/' | b'\\') {
        return None;
    }
    if src.is_empty()
        || lower.contains("://")
        || lower.starts_with("data:")
        || lower.starts_with("//")
    {
        return None;
    }
    let path = Path::new(src);
    if path.is_absolute() {
        Some(path.to_path_buf())
    } else {
        base.map(|b| b.join(path))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn document(text: &str) -> Document {
        Document::from_utf8(text.as_bytes().to_vec()).unwrap()
    }

    #[test]
    fn html_files_are_html_and_the_rest_is_markdown() {
        assert_eq!(format_of(Some(Path::new("a/index.HTML"))), Format::Html);
        assert_eq!(format_of(Some(Path::new("page.htm"))), Format::Html);
        assert_eq!(format_of(Some(Path::new("README.md"))), Format::Markdown);
        assert_eq!(format_of(Some(Path::new("notes.txt"))), Format::Markdown);
        assert_eq!(format_of(Some(Path::new("Makefile"))), Format::Markdown);
        assert_eq!(format_of(None), Format::Markdown);
    }

    #[test]
    fn the_lines_are_joined_with_lf_whatever_the_line_ending_is() {
        let crlf = extract(&document("# a\r\n\r\nこんにちは\r\n"), MAX_BYTES);
        assert_eq!(crlf.text, "# a\n\nこんにちは\n");
        assert!(!crlf.truncated);
        let cr = extract(&document("a\rb"), MAX_BYTES);
        assert_eq!(cr.text, "a\nb\n");
        assert_eq!(extract(&document(""), MAX_BYTES).text, "");
    }

    #[test]
    fn a_big_document_is_read_up_to_the_cap_only() {
        let line = "x".repeat(99) + "\n";
        let doc = document(&line.repeat(1000));
        let got = extract(&doc, 1000);
        assert!(got.truncated);
        assert!(got.text.len() <= 1000, "{}", got.text.len());
        // Ten whole lines of 100 bytes fit; the cap is reached exactly.
        assert_eq!(got.text.lines().count(), 10);
        assert!(got.text.lines().all(|l| l.len() == 99));
    }

    #[test]
    fn a_line_longer_than_the_cap_is_cut_between_characters() {
        let doc = document(&"あ".repeat(100));
        let got = extract(&doc, 10);
        assert!(got.truncated);
        // Three characters of three bytes fit in ten bytes.
        assert_eq!(got.text, "あああ");
    }

    #[test]
    fn a_document_over_the_source_limit_is_not_read() {
        let doc = document(&"y".repeat(MAX_SOURCE_BYTES + 1));
        let got = extract(&doc, MAX_BYTES);
        assert!(got.too_large && got.truncated);
        assert_eq!(got.text, "");
    }

    #[test]
    fn a_document_at_the_source_limit_is_read_up_to_the_cap() {
        let doc = document(&"y".repeat(MAX_SOURCE_BYTES));
        let got = extract(&doc, 1000);
        assert!(got.truncated && !got.too_large);
        assert_eq!(got.text.len(), 1000);
    }

    #[test]
    fn only_files_are_loaded_as_images() {
        let base = Some(Path::new("/docs"));
        assert_eq!(
            local_image("img/a.png", base),
            Some(Path::new("/docs/img/a.png").to_path_buf())
        );
        assert_eq!(
            local_image("/abs/b.png", base),
            Some(Path::new("/abs/b.png").to_path_buf())
        );
        // No folder to look in: a relative path is not loaded.
        assert_eq!(local_image("a.png", None), None);
        for blocked in [
            "http://example.com/t.gif",
            "HTTPS://example.com/t.gif",
            "file:///etc/passwd",
            "//example.com/t.gif",
            r"\\server\share\t.png",
            r"/\server\share\t.png",
            r"\/server/share/t.png",
            r"\\?\UNC\server\share\t.png",
            "data:image/png;base64,AAAA",
            "",
        ] {
            assert_eq!(local_image(blocked, base), None, "{blocked}");
        }
    }

    #[test]
    fn a_document_that_just_fits_is_not_truncated() {
        let got = extract(&document("abc\ndef\n"), 8);
        assert_eq!(got.text, "abc\ndef\n");
        assert!(!got.truncated);
        let got = extract(&document("abc\ndef\n"), 7);
        assert!(got.truncated);
    }
}
