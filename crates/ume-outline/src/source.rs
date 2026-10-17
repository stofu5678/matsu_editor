//! The lines of a document, as the analyzers read them.

use ume_core::buffer::Document;

/// A document that is read one line at a time.
///
/// Sakura's analyzers walk `CDocLineMgr` and ask for `GetDocLineStrWithEOL`: the text of the line
/// with its line ending. [`line`](Self::line) gives the same, as characters, so that the analyzers
/// index a line as Sakura does without caring for the UTF-8 byte offsets of the buffer.
pub trait LineSource {
    /// The number of lines (`CDocLineMgr::GetLineCount`). A document that ends with a line
    /// ending has no empty last line here, as in Sakura.
    fn line_count(&self) -> usize;

    /// The characters of line `index` (0 origin), followed by its line ending, or `None` past the
    /// last line (or if the line cannot be read).
    fn line(&self, index: usize) -> Option<Vec<char>>;

    /// The length of the line ending the document gets for new line breaks
    /// (`CDocEditor::m_cNewLineCode.GetLen()`), 1 or 2. Used by the bookmark list only.
    fn new_line_len(&self) -> usize {
        2
    }
}

impl LineSource for Document {
    fn line_count(&self) -> usize {
        Document::line_count(self)
    }

    fn line(&self, index: usize) -> Option<Vec<char>> {
        let line = Document::line(self, index).ok().flatten()?;
        let mut chars: Vec<char> = line.text.chars().collect();
        if let Some(eol) = line.eol {
            chars.extend(eol.as_str().chars());
        }
        Some(chars)
    }
}

/// Lines held in memory: for tests, and for text that is not a document.
#[derive(Clone, Debug, Default)]
pub struct VecSource {
    lines: Vec<Vec<char>>,
    new_line_len: usize,
}

impl VecSource {
    /// The lines of `text`; every line ending (`\r\n`, `\n` or `\r`) stays at the end of its line,
    /// and the text after the last line ending is a last line.
    pub fn new(text: &str) -> Self {
        let mut lines = Vec::new();
        let mut current = Vec::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            current.push(c);
            let ends = match c {
                '\n' => true,
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        current.push('\n');
                        chars.next();
                    }
                    true
                }
                _ => false,
            };
            if ends {
                lines.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
        Self {
            lines,
            new_line_len: 2,
        }
    }

    /// The same lines with another length of the line ending for new lines.
    pub fn with_new_line_len(mut self, len: usize) -> Self {
        self.new_line_len = len;
        self
    }
}

impl LineSource for VecSource {
    fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn line(&self, index: usize) -> Option<Vec<char>> {
        self.lines.get(index).cloned()
    }

    fn new_line_len(&self) -> usize {
        self.new_line_len
    }
}
