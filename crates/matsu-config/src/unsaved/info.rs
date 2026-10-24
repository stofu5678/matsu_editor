//! The information kept with an unsaved document (ADR-0022, section 3).

use crate::profile::{Profile, ProfileError};

const SECTION: &str = "Doc";

/// What is needed to bring a document back and to save it later in the form it had. It is written as an
/// INI section (the same format as `matsu.ini`).
///
/// The encoding is kept as the text the editor uses for it (for example `UTF-8`, `Shift_JIS`), so that this crate
/// does not depend on the core's list of encodings. The line ending is `CRLF`, `LF` or `CR`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocInfo {
    /// The number of a document that was never saved (無題 N).
    pub untitled_number: Option<u32>,
    /// The path of the file the document came from, when it has one.
    pub path: Option<String>,
    /// The time stamp of that file when the document was opened: seconds since the Unix epoch.
    pub file_modified_secs: Option<i64>,
    /// The sub-second part of that time stamp, in nanoseconds.
    pub file_modified_nanos: Option<u32>,
    /// The size of that file in bytes when the document was opened.
    pub file_size: Option<u64>,
    /// The encoding the document is saved in.
    pub encoding: String,
    /// Whether the saved file has a BOM.
    pub bom: bool,
    /// The line ending of new lines: `CRLF`, `LF` or `CR`.
    pub line_ending: String,
    /// The id of the document type (the number of `Types(N)`), when the user locked one.
    pub doc_type: Option<i32>,
    /// The caret: line (0 based) and column (in UTF-16 units, as Sakura's `nX`).
    pub caret_line: u64,
    pub caret_column: u64,
    /// The line at the top of the view.
    pub top_line: u64,
    /// The bookmarked lines (0 based), in order.
    pub bookmarks: Vec<u64>,
}

impl DocInfo {
    pub(crate) fn to_bytes(&self) -> Result<Vec<u8>, ProfileError> {
        let mut p = Profile::new();
        let mut set = |key: &str, value: String| p.set(SECTION, key, &value);
        if let Some(n) = self.untitled_number {
            set("UntitledNumber", n.to_string())?;
        }
        if let Some(path) = &self.path {
            set("Path", path.clone())?;
        }
        if let Some(v) = self.file_modified_secs {
            set("FileModifiedSecs", v.to_string())?;
        }
        if let Some(v) = self.file_modified_nanos {
            set("FileModifiedNanos", v.to_string())?;
        }
        if let Some(v) = self.file_size {
            set("FileSize", v.to_string())?;
        }
        set("Encoding", self.encoding.clone())?;
        set("Bom", u8::from(self.bom).to_string())?;
        set("LineEnding", self.line_ending.clone())?;
        if let Some(v) = self.doc_type {
            set("DocType", v.to_string())?;
        }
        set("CaretLine", self.caret_line.to_string())?;
        set("CaretColumn", self.caret_column.to_string())?;
        set("TopLine", self.top_line.to_string())?;
        let marks: Vec<String> = self.bookmarks.iter().map(u64::to_string).collect();
        set("Bookmarks", marks.join(","))?;
        Ok(p.to_bytes())
    }

    /// Reads the information. A key that is missing or unreadable gets its default, so a document is never
    /// lost because of its information; only the text matters.
    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        let p = Profile::from_bytes(bytes);
        let get = |key: &str| p.get(SECTION, key);
        let num = |key: &str| get(key).and_then(|v| v.trim().parse::<u64>().ok());
        Self {
            untitled_number: get("UntitledNumber").and_then(|v| v.trim().parse().ok()),
            path: get("Path").map(str::to_owned).filter(|s| !s.is_empty()),
            file_modified_secs: get("FileModifiedSecs").and_then(|v| v.trim().parse().ok()),
            file_modified_nanos: get("FileModifiedNanos").and_then(|v| v.trim().parse().ok()),
            file_size: num("FileSize"),
            encoding: get("Encoding").unwrap_or("UTF-8").to_owned(),
            bom: get("Bom").is_some_and(|v| v.trim() == "1"),
            line_ending: get("LineEnding").unwrap_or("LF").to_owned(),
            doc_type: get("DocType").and_then(|v| v.trim().parse().ok()),
            caret_line: num("CaretLine").unwrap_or(0),
            caret_column: num("CaretColumn").unwrap_or(0),
            top_line: num("TopLine").unwrap_or(0),
            bookmarks: get("Bookmarks")
                .map(|v| v.split(',').filter_map(|s| s.trim().parse().ok()).collect())
                .unwrap_or_default(),
        }
    }
}
