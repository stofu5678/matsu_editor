//! Where the layout reads the document from.

/// The lines of a document, read one at a time and by number. The caller adapts its document
/// to this (the crate does not know the buffer), so a document of any size can be laid out
/// without a second copy of it.
///
/// A line is as Sakura keeps it (`CDocLine`): its text without the line ending, and whether it
/// has one. A text that ends with a line ending has no empty line after it.
pub trait LineSource {
    /// The number of lines.
    fn line_count(&self) -> usize;

    /// Replaces `buf` with the text of line `index` (from 0, less than [`line_count`]) without
    /// its line ending, and tells whether the line has one.
    ///
    /// The layout calls it again for the lines of a page, so it should be quick.
    ///
    /// [`line_count`]: LineSource::line_count
    fn read_line(&self, index: usize, buf: &mut String) -> bool;
}

/// A text split at its line endings. An empty last element is the end of a text that ends with
/// a line ending, which is not a line (Sakura does not count it), and every line but the last
/// one has a line ending.
impl<S: AsRef<str>> LineSource for [S] {
    fn line_count(&self) -> usize {
        match self.last() {
            Some(last) if last.as_ref().is_empty() => self.len() - 1,
            _ => self.len(),
        }
    }

    fn read_line(&self, index: usize, buf: &mut String) -> bool {
        buf.clear();
        buf.push_str(self[index].as_ref());
        index + 1 < self.len()
    }
}

impl<S: AsRef<str>> LineSource for Vec<S> {
    fn line_count(&self) -> usize {
        self.as_slice().line_count()
    }

    fn read_line(&self, index: usize, buf: &mut String) -> bool {
        self.as_slice().read_line(index, buf)
    }
}

impl<S: AsRef<str>, const N: usize> LineSource for [S; N] {
    fn line_count(&self) -> usize {
        self.as_slice().line_count()
    }

    fn read_line(&self, index: usize, buf: &mut String) -> bool {
        self.as_slice().read_line(index, buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(src: &(impl LineSource + ?Sized), index: usize) -> (String, bool) {
        let mut buf = String::from("old");
        let eol = src.read_line(index, &mut buf);
        (buf, eol)
    }

    #[test]
    fn a_split_text_is_a_source() {
        let lines = ["a", "b", ""];
        assert_eq!(lines.line_count(), 2);
        assert_eq!(read(&lines, 0), ("a".to_string(), true));
        assert_eq!(read(&lines, 1), ("b".to_string(), true));
        // No final line ending: the last line has none.
        let lines = vec!["a".to_string(), "b".to_string()];
        assert_eq!(lines.line_count(), 2);
        assert_eq!(read(&lines, 1), ("b".to_string(), false));
        // An empty text has no line.
        assert_eq!([""].line_count(), 0);
        assert_eq!(Vec::<String>::new().line_count(), 0);
        // A text of one line ending is one empty line.
        assert_eq!(["", ""].line_count(), 1);
        assert_eq!(read(&["", ""], 0), (String::new(), true));
    }
}
