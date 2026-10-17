// Ported in part from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/dlg/CDlgTagJumpList.cpp (find_key_core, ReadTagsParameter, parseTagsLine,
//     find_key_for_BinarySearch, find_key_for_LinearSearch, GetFullPathAndLine, SearchBestTag)
//   sakura_core/env/CSortedTagJumpList.cpp (AddParamA)
//
// sakura_core/dlg/CDlgTagJumpList.cpp:
//   Copyright (C) 2003, MIK
//   Copyright (C) 2005, MIK
//   Copyright (C) 2006, genta, ryoji
//   Copyright (C) 2010, Moca
//   Copyright (C) 2018-2026, Sakura Editor Organization
// sakura_core/env/CSortedTagJumpList.cpp:
//   Copyright (C) 2005, MIK, genta
//   Copyright (C) 2018-2022, Sakura Editor Organization
//
// These files are derived from the files above and were modified: the C++ code (`FILE*`, `fgets`,
// `sscanf`, UTF-16 buffers and Windows paths) was rewritten in Rust on `BufRead + Seek` and
// `Path`, the list is a `Vec` sorted at the end instead of a linked list sorted on every insert,
// the results that Sakura keeps between two searches to skip tags files (`STagFindState`) are not
// kept, and the tests are new.
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! Reading tags files (`ctags` output) for the direct tag jump and the tag jump list.
//!
//! Sakura: `CDlgTagJumpList::find_key_core` and the functions around it
//! (`sakura_core/dlg/CDlgTagJumpList.cpp`); the help is `help/sakura/res/HLP000065.html`.
//!
//! A file called `tags` is looked for in the folder of the document and then in each folder
//! above it, up to the root. The number of folders that were climbed is the "階層" (depth) of a
//! hit. A tags file is read as Sakura reads it:
//!
//! * the header lines `!_TAG_FILE_FORMAT` (1 or 2; 2 reads 1 too), `!_TAG_FILE_SORTED` (1 sorted,
//!   2 sorted ignoring case), and Sakura's own `!_TAG_S_SEARCH_NEXT` (what to do about the next
//!   tags file, and a folder to continue in) and `!_TAG_S_FILE_BASEDIR` (the folder that the file
//!   names are relative to);
//! * `keyword<TAB>file<TAB>line;"<TAB>kind<TAB>note` (the extended format with `--excmd=n`,
//!   whose line numbers Sakura needs), or `keyword<TAB>file<TAB>line`;
//! * a first line that starts with a form feed is an `etags` file, which is skipped;
//! * a line is read up to 1023 bytes and the rest of a longer line is thrown away.
//!
//! The file is never read as a whole: a sorted file is searched by bisection on byte offsets when
//! the search is an exact, case-sensitive one, else it is read line by line from the start, up to
//! the end of the page that was asked for (50 hits, [`PAGE_SIZE`]).
//!
//! Differences from Sakura: a line is decoded as UTF-8, or as Shift_JIS when it is not UTF-8
//! (Sakura uses the ANSI code page); the comparison of keywords ignores case for ASCII letters
//! only; the results of the previous search are not used to skip tags files (the hits are the
//! same); the bisection does not throw away the first line of the file when it starts reading
//! there, and reads the start of the file when every probe was past the keyword; "do not search the next file" (mode 0) applies after the first tags file has been
//! opened (Sakura's loop would then never open one).

use std::cmp::Ordering;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

mod kinds;
#[cfg(test)]
mod tests;

pub use kinds::kind_name;

/// `TAG_FILENAME_T`.
pub const TAG_FILE_NAME: &str = "tags";

/// How many hits one page of the list holds (`CSortedTagJumpList(50)`).
pub const PAGE_SIZE: usize = 50;

/// The longest line that is read (the buffer of `fgets` is 1024 bytes with the terminator).
const LINE_BYTES: usize = 1023;

/// `MAX_TAG_STRING_LENGTH` (`_MAX_PATH`): longer strings are cut.
const MAX_STRING: usize = 260;

/// `_MAX_PATH / 2`: the most folders that are climbed.
const MAX_DEPTH: usize = 130;

/// How far the bisection steps back to find the first of equal keywords (`1024 * 2`).
const SKIP_BACK_BYTES: u64 = 2048;

/// One hit of a tags file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagItem {
    /// The tag name.
    pub keyword: String,
    /// The file name as the tags file writes it (the "ファイル名" column).
    pub file: String,
    /// The line, from 1.
    pub line: usize,
    /// The kind of the tag, one letter of ctags (`f` function, `c` class, ...), if there is one.
    pub kind: Option<char>,
    /// The rest of the line (the "備考" column).
    pub note: String,
    /// The number of folders that were climbed from the document to find the tags file.
    pub depth: usize,
    /// Whether the tags file was found by `!_TAG_S_SEARCH_NEXT`, not by climbing (the depth is
    /// then shown in parentheses).
    pub via_next: bool,
    /// The file made absolute: by the base folder of the tags file if it has one, else by the
    /// folder of the tags file (`GetFullPathAndLine`).
    pub path: PathBuf,
}

/// What to look for (`find_key_core`).
#[derive(Clone, Debug)]
pub struct TagQuery<'a> {
    pub keyword: &'a str,
    /// 文字列の途中にマッチ.
    pub partial: bool,
    /// The whole name must match (the direct tag jump); else the name must start with the keyword.
    pub exact: bool,
    /// 大文字小文字の同一視.
    pub ignore_case: bool,
    /// Follow the tags file when it says its names are sorted ignoring case (the direct tag
    /// jump).
    pub ignore_case_by_tags: bool,
    /// What to do about the next tags file when the tags file does not say (the setting
    /// "タグジャンプモード": 0 do not search it, 1 not after a hit, 2 not after an exact hit, 3
    /// always).
    pub next_mode: u8,
    /// How many hits to skip (the pages before).
    pub top: usize,
}

impl<'a> TagQuery<'a> {
    /// The query of the direct tag jump (`FindDirectTagJump`): the whole name, case-sensitive.
    pub fn direct(keyword: &'a str, next_mode: u8) -> Self {
        Self {
            keyword,
            partial: false,
            exact: true,
            ignore_case: false,
            ignore_case_by_tags: true,
            next_mode,
            top: 0,
        }
    }
}

/// The hits of one search.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TagPage {
    /// The hits of this page, in the order of the keyword.
    pub items: Vec<TagItem>,
    /// How many hits there were up to the end of this page, those of the pages before included.
    /// More than 1 means the user has to choose (`m_nMatchAll`).
    pub matched: usize,
    /// Whether there is a next page.
    pub has_next: bool,
}

/// Where the search is in the folders (`STagFindState`).
struct Walk {
    dir: PathBuf,
    depth: usize,
    loop_max: usize,
    next_mode: u8,
    /// Following `!_TAG_S_SEARCH_NEXT` (`m_bJumpPath`).
    via_next: bool,
    searched: Vec<PathBuf>,
    opened_any: bool,
}

/// Searches the tags files of the folders from the one of `document` upwards.
///
/// `document` is the path of the file that is edited (for a new document, a made-up name in the
/// current folder, as `Sub_PreProcTagJumpByTagsFile` does).
pub fn find_tags(document: &Path, query: &TagQuery<'_>) -> TagPage {
    let mut search = Search::new(query);
    if query.keyword.is_empty() {
        return search.finish();
    }
    let mut state = Walk {
        dir: document.parent().unwrap_or(Path::new("")).to_path_buf(),
        depth: 0,
        loop_max: max_up(document),
        next_mode: query.next_mode,
        via_next: false,
        searched: Vec::new(),
        opened_any: false,
    };
    let mut ignore_case = query.ignore_case;
    while state.depth <= state.loop_max && state.depth < MAX_DEPTH {
        if state.opened_any && !state.goes_on(query, search.matched) {
            break;
        }
        if state.searched.contains(&state.dir) {
            break;
        }
        state.searched.push(state.dir.clone());

        let mut next_dir = None;
        if let Ok(file) = File::open(state.dir.join(TAG_FILE_NAME)) {
            state.opened_any = true;
            state.next_mode = query.next_mode;
            let mut reader = BufReader::new(file);
            // The rule keeps the setting from before the header, the choice of the method after it.
            let rule = Rule {
                partial: query.partial,
                exact: query.exact,
                ignore_case,
            };
            if let Some(header) = read_header(&mut reader, &state.dir, state.via_next) {
                if let Some(mode) = header.next_mode {
                    state.next_mode = mode;
                }
                next_dir = header.next_dir.clone();
                if query.ignore_case_by_tags
                    && let Some(fold) = header.fold_case_header
                {
                    ignore_case = fold;
                }
                let source = Source {
                    dir: &state.dir,
                    depth: state.depth,
                    via_next: state.via_next,
                    base_dir: header
                        .base_dir
                        .clone()
                        .or_else(|| state.via_next.then(|| state.dir.clone())),
                    format: header.format,
                };
                // An error in the file ends the search of this file, not the whole search.
                let bisectable = header.sorted
                    && !header.fold_case
                    && !ignore_case
                    && query.exact
                    && !query.partial;
                let after_header = reader.stream_position().unwrap_or(0);
                let before = search.mark();
                let done = bisectable
                    && matches!(search.bisect(&mut reader, query.keyword, &source), Ok(true));
                if !done {
                    // A file in Shift_JIS is not sorted by the text we read, so it is read through.
                    search.reset(before);
                    let _ = reader.seek(SeekFrom::Start(after_header));
                    let _ = search.scan(
                        &mut reader,
                        query.keyword,
                        &source,
                        &rule,
                        header.sorted,
                        header.fold_case,
                    );
                }
            }
        }
        match next_dir {
            Some(dir) => {
                state.via_next = true;
                state.loop_max = max_up(&dir.join("dummy"));
                state.dir = dir;
                // Sakura sets the depth to 0 and the loop then counts it up to 1.
                state.depth = 0;
            }
            None => {
                if let Some(parent) = state.dir.parent() {
                    state.dir = parent.to_path_buf();
                }
            }
        }
        if search.has_next {
            break;
        }
        state.depth += 1;
    }
    search.finish()
}

impl Walk {
    /// Whether the next tags file is searched: 0 never, 1 if there is no hit yet, 2 if there is
    /// no hit yet or the search is not an exact one, 3 always.
    fn goes_on(&self, query: &TagQuery<'_>, matched: usize) -> bool {
        match self.next_mode {
            0 => false,
            1 => matched == 0,
            2 => !(query.exact && matched > 0),
            _ => true,
        }
    }
}

/// `CalcMaxUpDirectory`: how many folders there are above the file at `path`.
fn max_up(path: &Path) -> usize {
    let names = path
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .count();
    names.saturating_sub(1).min(MAX_DEPTH)
}

/// The lines starting with `!_TAG_` and what they say (`ReadTagsParameter`).
struct Header {
    format: u8,
    sorted: bool,
    fold_case: bool,
    /// Whether the header has `!_TAG_FILE_SORTED`, and if so whether it says "ignoring case".
    fold_case_header: Option<bool>,
    next_mode: Option<u8>,
    next_dir: Option<PathBuf>,
    base_dir: Option<PathBuf>,
}

/// Reads the header of a tags file and leaves the reader at the first line that is not a header
/// line. `None` if the file is not one that is read (an `etags` file).
fn read_header<R: BufRead + Seek>(r: &mut R, dir: &Path, via_next: bool) -> Option<Header> {
    let mut header = Header {
        format: 2,
        sorted: true,
        fold_case: false,
        fold_case_header: None,
        next_mode: None,
        next_dir: None,
        base_dir: None,
    };
    let mut buf = Vec::new();
    let mut lines = 0;
    let mut rewind_to = 0;
    while read_line(r, &mut buf).ok()? {
        lines += 1;
        if lines == 1 && buf.first() == Some(&0x0c) {
            return None;
        }
        if buf.first() != Some(&b'!') {
            r.seek(SeekFrom::Start(rewind_to)).ok()?;
            return Some(header);
        }
        let line = decode_text(&buf);
        if let Some(rest) = line.strip_prefix("!_TAG_") {
            let fields = scan_fields(&line, 3);
            if fields.len() < 2 {
                // Sakura does not move the rewind position past a line it cannot read.
                continue;
            }
            let value = fields[1];
            let third = fields.get(2).copied().unwrap_or("");
            let n = atoi(value);
            if rest.starts_with("FILE_FORMAT") {
                if let Ok(format @ 1..=2) = u8::try_from(n) {
                    header.format = format;
                }
            } else if rest.starts_with("FILE_SORTED") {
                header.sorted = n == 1;
                header.fold_case = n == 2;
                header.fold_case_header = Some(n == 2);
            } else if rest.starts_with("S_SEARCH_NEXT") {
                if value.starts_with(['0', '1', '2', '3']) {
                    if let Ok(mode @ 0..=3) = u8::try_from(n) {
                        header.next_mode = Some(mode);
                    }
                    if n >= 1 && !third.is_empty() {
                        header.next_dir = Some(join_dir(dir, third));
                    }
                }
            } else if rest.starts_with("S_FILE_BASEDIR") {
                // While following a folder, a relative base folder is relative to that folder.
                header.base_dir = Some(if via_next {
                    join_dir(dir, value)
                } else {
                    PathBuf::from(normalize(value))
                });
            }
        }
        rewind_to = r.stream_position().ok()?;
    }
    Some(header)
}

/// `CopyDirDir`: `target` as a folder, relative to `base` if it is relative.
fn join_dir(base: &Path, target: &str) -> PathBuf {
    let target = PathBuf::from(normalize(target));
    if target.has_root() || target.is_absolute() {
        target
    } else {
        base.join(target)
    }
}

/// The file names of a tags file written on Windows use backslashes.
fn normalize(name: &str) -> String {
    if cfg!(windows) {
        name.to_owned()
    } else {
        name.replace('\\', "/")
    }
}

/// Reads one line like `fgets(buf, 1024)` followed by `SkipLine` for a longer line. The line
/// break is kept. `false` at the end of the file.
fn read_line<R: BufRead>(r: &mut R, buf: &mut Vec<u8>) -> io::Result<bool> {
    buf.clear();
    let n = r.by_ref().take(LINE_BYTES as u64).read_until(b'\n', buf)?;
    if n == 0 {
        return Ok(false);
    }
    if n == LINE_BYTES && buf.last() != Some(&b'\n') {
        // the rest of the line is thrown away
        loop {
            let chunk = r.fill_buf()?;
            if chunk.is_empty() {
                break;
            }
            if let Some(i) = chunk.iter().position(|&b| b == b'\n') {
                r.consume(i + 1);
                break;
            }
            let len = chunk.len();
            r.consume(len);
        }
    }
    Ok(true)
}

/// Bytes as text: UTF-8, else Shift_JIS (what Sakura's ANSI code page is in Japan).
pub(crate) fn decode_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => encoding_rs::SHIFT_JIS.decode(bytes).0.into_owned(),
    }
}

/// Whether the bytes are UTF-8. A line that is not (Shift_JIS) does not sort in the order of its
/// decoded text, so the ordering of the file cannot be relied on after it.
fn is_utf8(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok()
}

/// `atoi`.
fn atoi(s: &str) -> i64 {
    let s = s.trim_start();
    let (sign, digits) = match s.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, s.strip_prefix('+').unwrap_or(s)),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    sign * digits[..end].parse::<i64>().unwrap_or(0)
}

/// What `sscanf` makes of `%[^\t\r\n]\t%[^\t\r\n]...`: up to `max` fields, each a run of
/// characters other than tab and line breaks, with any white space skipped before the next.
fn scan_fields(line: &str, max: usize) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = line;
    while out.len() < max {
        let end = rest.find(['\t', '\r', '\n']).unwrap_or(rest.len());
        if end == 0 {
            break;
        }
        out.push(&rest[..end]);
        rest = rest[end..].trim_start();
    }
    out
}

/// One parsed line of a tags file.
struct Line<'a> {
    keyword: &'a str,
    file: &'a str,
    line: usize,
    kind: Option<char>,
    note: &'a str,
}

/// `parseTagsLine`: `None` if the line is not a tag with a line number.
fn parse_line(text: &str, format: u8) -> Option<Line<'_>> {
    let fields = scan_fields(text, 2);
    let [keyword, file] = fields[..] else {
        return None;
    };
    // the text after the file name
    let file_end = file.as_ptr() as usize - text.as_ptr() as usize + file.len();
    let after = text[file_end..].trim_start();
    // %d
    let digits_at = after
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '+' || c == '-'))))
        .map_or(after.len(), |(i, _)| i);
    let number = &after[..digits_at];
    if !number.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let line = usize::try_from(atoi(number).clamp(0, i64::from(i32::MAX))).ok()?;
    if line == 0 {
        return None;
    }
    let (mut kind, mut note) = (None, "");
    if format == 2
        && let Some(extension) = after[digits_at..].strip_prefix(";\"")
    {
        let mut tokens = extension.split_whitespace();
        kind = tokens.next().and_then(|t| t.chars().next());
        note = tokens.next().unwrap_or("");
    }
    Some(Line {
        keyword,
        file,
        line,
        kind,
        note,
    })
}

/// A tags file that is being read.
struct Source<'a> {
    dir: &'a Path,
    depth: usize,
    via_next: bool,
    base_dir: Option<PathBuf>,
    format: u8,
}

/// How the keyword of a line is compared (`STagSearchRule`).
struct Rule {
    partial: bool,
    exact: bool,
    ignore_case: bool,
}

/// The state of a search over all tags files (`CSortedTagJumpList` and the counters of
/// `STagFindState`).
struct Search<'a> {
    query: &'a TagQuery<'a>,
    items: Vec<TagItem>,
    matched: usize,
    has_next: bool,
}

impl<'a> Search<'a> {
    fn new(query: &'a TagQuery<'a>) -> Self {
        Self {
            query,
            items: Vec::new(),
            matched: 0,
            has_next: false,
        }
    }

    /// The counters, to go back to them.
    fn mark(&self) -> (usize, usize, bool) {
        (self.items.len(), self.matched, self.has_next)
    }

    fn reset(&mut self, (items, matched, has_next): (usize, usize, bool)) {
        self.items.truncate(items);
        self.matched = matched;
        self.has_next = has_next;
    }

    fn finish(mut self) -> TagPage {
        // `AddParamA` puts a hit before the first one whose keyword is greater (`wcscmp`).
        self.items
            .sort_by(|a, b| a.keyword.encode_utf16().cmp(b.keyword.encode_utf16()));
        TagPage {
            items: self.items,
            matched: self.matched,
            has_next: self.has_next,
        }
    }

    /// Counts a hit and keeps it if it is on the page. `false` when the page is full: the search
    /// of this tags file ends and the next page begins from here.
    fn hit(&mut self, line: &Line<'_>, source: &Source<'_>) -> bool {
        self.matched += 1;
        if self.query.top >= self.matched {
            return true;
        }
        if self.items.len() >= PAGE_SIZE {
            self.has_next = true;
            return false;
        }
        let named = match &source.base_dir {
            Some(base) => base.join(normalize(line.file)),
            None => PathBuf::from(normalize(line.file)),
        };
        let path = if named.has_root() || named.is_absolute() {
            named
        } else {
            source.dir.join(named)
        };
        self.items.push(TagItem {
            keyword: cut(line.keyword),
            file: cut(line.file),
            line: line.line,
            kind: line.kind,
            note: cut(line.note),
            depth: source.depth,
            via_next: source.via_next,
            path,
        });
        true
    }

    /// `find_key_for_LinearSearch`.
    fn scan<R: BufRead>(
        &mut self,
        r: &mut R,
        keyword: &str,
        source: &Source<'_>,
        rule: &Rule,
        sorted: bool,
        fold_case: bool,
    ) -> io::Result<()> {
        let mut buf = Vec::new();
        let mut legacy = false;
        while read_line(r, &mut buf)? {
            if buf.first().is_none_or(|&b| b < b'!') {
                continue;
            }
            legacy |= !is_utf8(&buf);
            let text = decode_text(&buf);
            let Some(line) = parse_line(&text, source.format) else {
                continue;
            };
            match compare(line.keyword, keyword, rule) {
                Ordering::Equal => {
                    if !self.hit(&line, source) {
                        break;
                    }
                }
                // A sorted file has no more hits after the first greater keyword.
                Ordering::Greater
                    if !legacy
                        && !rule.partial
                        && ((!rule.ignore_case && sorted) || (rule.ignore_case && fold_case)) =>
                {
                    break;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// `find_key_for_BinarySearch`: bisection on the byte offsets of a sorted file, then reading
    /// forward over the lines with the keyword.
    fn bisect<R: BufRead + Seek>(
        &mut self,
        r: &mut R,
        keyword: &str,
        source: &Source<'_>,
    ) -> io::Result<bool> {
        #[derive(PartialEq)]
        enum Phase {
            Binary,
            SkipBack,
            StepForward,
        }
        let mut phase = Phase::Binary;
        let mut buf = Vec::new();
        let mut low: u64 = 0;
        let mut high = r.seek(SeekFrom::End(0))?;
        let mut current = low + (high - low) / 2;
        r.seek(SeekFrom::Start(current))?;
        if current > 0 {
            read_line(r, &mut buf)?;
        }
        loop {
            let got = read_line(r, &mut buf)?;
            if !got && phase != Phase::Binary {
                break;
            }
            if got && !is_utf8(&buf) {
                // not in the order of the text: the caller reads the file through
                return Ok(false);
            }
            let text = decode_text(&buf);
            let line = if got {
                parse_line(&text, source.format)
            } else {
                None
            };
            let order = match &line {
                Some(line) => line.keyword.as_bytes().cmp(keyword.as_bytes()),
                // The end of the file counts as past the keyword (Sakura gives up here, and
                // cannot find the last tags of a file).
                None if !got => Ordering::Greater,
                // a line that is not a tag is passed over without moving
                None => continue,
            };
            match order {
                Ordering::Equal => match phase {
                    Phase::Binary => phase = Phase::SkipBack,
                    Phase::SkipBack => {}
                    Phase::StepForward => {
                        if let Some(line) = &line
                            && !self.hit(line, source)
                        {
                            break;
                        }
                    }
                },
                Ordering::Less => match phase {
                    Phase::Binary => low = current,
                    Phase::SkipBack => phase = Phase::StepForward,
                    Phase::StepForward => {}
                },
                Ordering::Greater => match phase {
                    Phase::Binary => high = current,
                    Phase::StepForward => break,
                    Phase::SkipBack => {}
                },
            }
            match phase {
                Phase::Binary => {
                    let next = low + (high - low) / 2;
                    if next == current {
                        if low > 0 {
                            break;
                        }
                        // Every probe was past the keyword, and the lines at the very start of
                        // the file were never read (Sakura leaves them out): read them now.

                        r.seek(SeekFrom::Start(0))?;
                        phase = Phase::StepForward;
                        continue;
                    }
                    current = next;
                    r.seek(SeekFrom::Start(current))?;
                    read_line(r, &mut buf)?;
                }
                Phase::SkipBack => {
                    if current < SKIP_BACK_BYTES {
                        current = 0;
                        phase = Phase::StepForward;
                    } else {
                        current -= SKIP_BACK_BYTES;
                    }
                    r.seek(SeekFrom::Start(current))?;
                    if current > 0 {
                        read_line(r, &mut buf)?;
                    }
                }
                Phase::StepForward => {}
            }
        }
        Ok(true)
    }
}

/// The comparison of a keyword of the file with the one that is looked for. `Equal` is a hit;
/// the other two tell the order for the early end of a search of a sorted file.
fn compare(candidate: &str, keyword: &str, rule: &Rule) -> Ordering {
    if rule.partial {
        let hit = if rule.ignore_case {
            candidate
                .to_ascii_lowercase()
                .contains(&keyword.to_ascii_lowercase())
        } else {
            candidate.contains(keyword)
        };
        return if hit { Ordering::Equal } else { Ordering::Less };
    }
    // a prefix compare looks at the first `keyword.len()` bytes (`strncmp`)
    let head = if rule.exact {
        candidate
    } else {
        let mut n = keyword.len().min(candidate.len());
        while !candidate.is_char_boundary(n) {
            n += 1;
        }
        &candidate[..n]
    };
    if rule.ignore_case {
        head.bytes()
            .map(|c| c.to_ascii_lowercase())
            .cmp(keyword.bytes().map(|c| c.to_ascii_lowercase()))
    } else {
        head.as_bytes().cmp(keyword.as_bytes())
    }
}

/// A string cut at `MAX_TAG_STRING_LENGTH`.
fn cut(s: &str) -> String {
    s.chars().take(MAX_STRING - 1).collect()
}

/// `SearchBestTag`: the hit that is most likely the one wanted, for the first choice in the list:
/// the same file, then the same folder and name, ..., by the order of Sakura's rules.
/// `document` is the file being edited. Returns the index into `items` (0 if nothing is alike).
pub fn best_item(items: &[TagItem], document: &Path) -> usize {
    let src = Parts::of(document);
    let mut found: [Option<usize>; 7] = [None; 7];
    for (i, item) in items.iter().enumerate() {
        let dst = Parts::of(&item.path);
        if src.whole.eq_ignore_ascii_case(&dst.whole) {
            return i;
        }
        let same_dir = src.dir.eq_ignore_ascii_case(&dst.dir);
        let same_stem = src.stem.eq_ignore_ascii_case(&dst.stem);
        let same_ext = src.ext.eq_ignore_ascii_case(&dst.ext);
        let dir_tail = src.dir.len() >= dst.dir.len()
            && src
                .dir
                .get(src.dir.len() - dst.dir.len()..)
                .is_some_and(|tail| tail.eq_ignore_ascii_case(&dst.dir));
        // nMatch1 .. nMatch7 of Sakura, in the order of the checks below
        let conditions = [
            same_dir && same_stem,
            same_dir,
            same_stem && same_ext,
            same_stem,
            same_dir && same_stem && same_ext,
            same_stem && same_ext && dir_tail,
            same_stem && dir_tail,
        ];
        for (slot, condition) in found.iter_mut().zip(conditions) {
            if slot.is_none() && condition {
                *slot = Some(i);
            }
        }
    }
    // nMatch1, 5, 6, 7, 3, 4, 2
    [0, 4, 5, 6, 2, 3, 1]
        .into_iter()
        .find_map(|k| found[k])
        .unwrap_or(0)
}

/// A path split like `_wsplitpath_s`: the folder (with its separator at the end), the name
/// without the extension, and the extension (with its dot).
struct Parts {
    whole: String,
    dir: String,
    stem: String,
    ext: String,
}

impl Parts {
    fn of(path: &Path) -> Self {
        let whole = path.to_string_lossy().replace('\\', "/");
        let (dir, file) = match whole.rfind('/') {
            Some(i) => (whole[..=i].to_owned(), whole[i + 1..].to_owned()),
            None => (String::new(), whole.clone()),
        };
        let (stem, ext) = match file.rfind('.') {
            Some(i) => (file[..i].to_owned(), file[i..].to_owned()),
            None => (file, String::new()),
        };
        Self {
            whole,
            dir,
            stem,
            ext,
        }
    }
}
