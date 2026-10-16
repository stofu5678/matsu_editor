// Parts of this file are derived from Sakura Editor (https://github.com/sakura-editor/sakura,
// commit 48db6c7c25e2):
//   sakura_core/docplus/CDiffManager.cpp (CDiffLineMgr::SetDiffMarkRange, SearchDiffMark,
//                                         ResetAllDiffMark)
//   sakura_core/view/CEditView_Diff.cpp (CEditView::AnalyzeDiffInfo, the option flags of
//                                        CEditView::ViewDiffInfo)
//
// Copyright (C) 2002, MIK
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: Sakura runs an external diff.exe
// and reads the lines of its normal output ("99,99c99,99"); here the difference is computed in
// the program ([`diff_lines`]) and the marks are set from its [`Hunk`]s directly. The marks are
// kept next to the text, not in the line objects, and the tests are new.
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

//! DIFF marks (Sakura's DIFF差分表示): the difference of the document from another text, as marks
//! on the lines, and the difference itself.
//!
//! Sakura writes both texts to files, runs `diff.exe` on them and reads the lines of its normal
//! output; each says which lines of the first file became which lines of the second one
//! (`99a99,99`, `99,99c99,99`, `99,99d99`). The lines of the document get a mark in the line
//! object (`CDocLine::m_sMark.m_cDiffmarked`). Here [`diff_lines`] computes the same hunks with
//! the Myers algorithm (the `similar` crate, Apache-2.0; the hunks are those of `diff` for the
//! normal format, a minimal edit script), and [`DiffMarks`] keeps the marks next to the text and
//! follows the edits the same way [`Bookmarks`](crate::bookmarks::Bookmarks) does: a flag lives
//! and dies with its line.

use std::collections::BTreeMap;
use std::ops::Range;

use similar::{Algorithm, DiffTag, capture_diff_slices};

use crate::bookmarks::LineDelta;
use crate::buffer::{Document, Pos, Result};
use crate::model::eol_text;

/// The options of Sakura's DIFF dialog, the bits of the flags of the macro `Diff`
/// (`CEditView::ViewDiffInfo`, `CMacro::HandleCommand` `F_DIFF`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiffOptions {
    /// 0x0001, `-i` ignore-case: 大文字小文字同一視.
    pub ignore_case: bool,
    /// 0x0002, `-w` ignore-all-space: 空白無視.
    pub ignore_all_space: bool,
    /// 0x0004, `-b` ignore-space-change: 空白変更無視.
    pub ignore_space_change: bool,
    /// 0x0008, `-B` ignore-blank-lines: 空行無視.
    pub ignore_blank_lines: bool,
    /// 0x0010, `-t` expand-tabs: TAB-SPACE変換. In `diff` it only changes the alignment of the
    /// output, and the marks do not depend on it; it is kept so that the flags round-trip.
    pub expand_tabs: bool,
    /// 0x0020: the document is the old file (else the new one).
    pub own_is_old: bool,
    /// 0x0040: say so when there is no difference (DIFF差分が見つからないときにメッセージを表示).
    pub notify_no_diff: bool,
}

impl DiffOptions {
    pub const IGNORE_CASE: u32 = 0x0001;
    pub const IGNORE_ALL_SPACE: u32 = 0x0002;
    pub const IGNORE_SPACE_CHANGE: u32 = 0x0004;
    pub const IGNORE_BLANK_LINES: u32 = 0x0008;
    pub const EXPAND_TABS: u32 = 0x0010;
    pub const OWN_IS_OLD: u32 = 0x0020;
    pub const NOTIFY_NO_DIFF: u32 = 0x0040;

    pub fn from_flags(flags: u32) -> Self {
        Self {
            ignore_case: flags & Self::IGNORE_CASE != 0,
            ignore_all_space: flags & Self::IGNORE_ALL_SPACE != 0,
            ignore_space_change: flags & Self::IGNORE_SPACE_CHANGE != 0,
            ignore_blank_lines: flags & Self::IGNORE_BLANK_LINES != 0,
            expand_tabs: flags & Self::EXPAND_TABS != 0,
            own_is_old: flags & Self::OWN_IS_OLD != 0,
            notify_no_diff: flags & Self::NOTIFY_NO_DIFF != 0,
        }
    }

    pub fn flags(self) -> u32 {
        [
            (self.ignore_case, Self::IGNORE_CASE),
            (self.ignore_all_space, Self::IGNORE_ALL_SPACE),
            (self.ignore_space_change, Self::IGNORE_SPACE_CHANGE),
            (self.ignore_blank_lines, Self::IGNORE_BLANK_LINES),
            (self.expand_tabs, Self::EXPAND_TABS),
            (self.own_is_old, Self::OWN_IS_OLD),
            (self.notify_no_diff, Self::NOTIFY_NO_DIFF),
        ]
        .into_iter()
        .filter(|(on, _)| *on)
        .fold(0, |flags, (_, bit)| flags | bit)
    }
}

/// What kind of change a [`Hunk`] is (`a`, `c` and `d` of the normal format of `diff`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HunkKind {
    /// Lines only in the new text.
    Add,
    /// Lines of the old text were replaced by lines of the new text.
    Change,
    /// Lines only in the old text.
    Delete,
}

/// A run of lines that differ: the lines `old` of the old text correspond to the lines `new` of
/// the new one (0-based, half-open). For `Add` the old range is empty and starts where the new
/// lines come in; for `Delete` the new range is empty and starts where the old lines were.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    pub kind: HunkKind,
    pub old: Range<usize>,
    pub new: Range<usize>,
}

/// The line as `diff` compares it, with the options applied (`-i`, `-w`, `-b`). GNU `diff -w`
/// ignores all white space, `-b` ignores changes in the amount of it and white space at the end
/// of a line; a line ending counts as white space for both.
fn key(line: &str, options: DiffOptions) -> String {
    let mut out = String::with_capacity(line.len());
    if options.ignore_all_space {
        out.extend(line.chars().filter(|c| !c.is_whitespace()));
    } else if options.ignore_space_change {
        let mut in_space = false;
        for c in line.chars() {
            if c.is_whitespace() {
                in_space = true;
            } else {
                if in_space && !out.is_empty() {
                    out.push(' ');
                }
                in_space = false;
                out.push(c);
            }
        }
        // white space before the line's first char counts as nothing, and so does the end
        if line.starts_with(char::is_whitespace) && !out.is_empty() {
            out.insert(0, ' ');
        }
    } else {
        out.push_str(line);
    }
    if options.ignore_case {
        out = out.chars().flat_map(char::to_lowercase).collect();
    }
    out
}

/// Whether a line is blank for `-B`: nothing but its line ending (or, with `-w` or `-b`, nothing
/// but white space).
fn is_blank(line: &str, options: DiffOptions) -> bool {
    let body = line.trim_end_matches(['\r', '\n', '\u{85}', '\u{2028}', '\u{2029}']);
    if options.ignore_all_space || options.ignore_space_change {
        body.chars().all(char::is_whitespace)
    } else {
        body.is_empty()
    }
}

/// The difference of two texts, line by line. A line is the text of the line with its line ending
/// (Sakura writes the lines with their endings to the files that `diff` reads, so lines that
/// differ only in the line ending, or in having none at the end of the file, are different).
///
/// The hunks are in order, and do not touch each other.
pub fn diff_lines<A, B>(old: &[A], new: &[B], options: DiffOptions) -> Vec<Hunk>
where
    A: AsRef<str>,
    B: AsRef<str>,
{
    let old_keys: Vec<String> = old.iter().map(|l| key(l.as_ref(), options)).collect();
    let new_keys: Vec<String> = new.iter().map(|l| key(l.as_ref(), options)).collect();
    let mut hunks: Vec<Hunk> = Vec::new();
    for op in capture_diff_slices(Algorithm::Myers, &old_keys, &new_keys) {
        let (tag, o, n) = op.as_tag_tuple();
        if tag == DiffTag::Equal {
            continue;
        }
        match hunks.last_mut() {
            Some(last) if last.old.end == o.start && last.new.end == n.start => {
                last.old.end = o.end;
                last.new.end = n.end;
            }
            _ => hunks.push(Hunk {
                kind: HunkKind::Change,
                old: o,
                new: n,
            }),
        }
    }
    for hunk in &mut hunks {
        hunk.kind = match (hunk.old.is_empty(), hunk.new.is_empty()) {
            (true, _) => HunkKind::Add,
            (_, true) => HunkKind::Delete,
            _ => HunkKind::Change,
        };
    }
    if options.ignore_blank_lines {
        // `-B`: changes that only insert or delete blank lines do not count
        hunks.retain(|h| {
            let blank_old = old[h.old.clone()]
                .iter()
                .all(|l| is_blank(l.as_ref(), options));
            let blank_new = new[h.new.clone()]
                .iter()
                .all(|l| is_blank(l.as_ref(), options));
            !(blank_old && blank_new)
        });
    }
    hunks
}

/// The mark of a line (`EDiffMark`, `sakura_core/docplus/CDiffManager.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffMark {
    /// 追加: the line is not in the other text (`MARK_DIFF_APPEND`).
    Append,
    /// 変更 (`MARK_DIFF_CHANGE`).
    Change,
    /// 削除: lines of the other text were here, between this line and the one before
    /// (`MARK_DIFF_DELETE`).
    Delete,
    /// 削除(EOF): lines of the other text were after the last line (`MARK_DIFF_DEL_EX`).
    DeleteEx,
}

/// The marks of the lines of a document (0-based line numbers).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffMarks {
    marks: BTreeMap<usize, DiffMark>,
    /// `CDiffManager::IsDiffUse`: a difference is shown (also when it has no mark).
    in_use: bool,
    /// Counts the clearings, so that a comparison that is still being computed can tell that it
    /// has been replaced or reset meanwhile.
    epoch: u64,
}

impl DiffMarks {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a DIFF is shown (`CDiffManager::IsDiffUse`).
    pub fn in_use(&self) -> bool {
        self.in_use
    }

    /// How many times the marks were cleared (by a reset or by the start of a new comparison).
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Whether any line has a mark.
    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.marks.len()
    }

    pub fn get(&self, line: usize) -> Option<DiffMark> {
        self.marks.get(&line).copied()
    }

    /// The marked lines with their marks, smallest line first.
    pub fn iter(&self) -> impl Iterator<Item = (usize, DiffMark)> + '_ {
        self.marks.iter().map(|(&line, &mark)| (line, mark))
    }

    /// Removes every mark (`CDiffLineMgr::ResetAllDiffMark`).
    pub fn clear(&mut self) {
        self.marks.clear();
        self.in_use = false;
        self.epoch = self.epoch.wrapping_add(1);
    }

    /// Marks the lines `start..=end` (`CDiffLineMgr::SetDiffMarkRange`; `start` may be -1 and
    /// `end` may be past the last line, of which `line_count` says how many there are). An `end`
    /// past the last line first gives the last line the mark 削除(EOF); a range that then still
    /// reaches it replaces that mark.
    pub fn set_range(&mut self, mark: DiffMark, start: i64, end: i64, line_count: usize) {
        self.in_use = true;
        let start = start.max(0);
        let lines = i64::try_from(line_count).unwrap_or(i64::MAX);
        let mut end = end;
        if lines <= end {
            end = lines - 1;
            if let Ok(last) = usize::try_from(end) {
                self.marks.insert(last, DiffMark::DeleteEx);
            }
        }
        for line in start..=end {
            if let Ok(line) = usize::try_from(line) {
                self.marks.insert(line, mark);
            }
        }
    }

    /// Marks the lines of a document with `line_count` lines for the hunks of its comparison with
    /// another text (`CEditView::AnalyzeDiffInfo`). `own_is_old`: the hunks go from the document
    /// to the other text (the other text is the new one), else from the other text to the
    /// document. Marks that are there stay (Sakura clears them first, `Command_Diff_Reset`; see
    /// [`replace_with_hunks`](Self::replace_with_hunks)).
    pub fn add_hunks(&mut self, hunks: &[Hunk], own_is_old: bool, line_count: usize) {
        for h in hunks {
            let (own, other_is_new) = if own_is_old {
                (&h.old, true)
            } else {
                (&h.new, false)
            };
            // `diff` numbers lines from 1 and names an empty range by the line before it, so
            // `s` and `e` are its numbers; the 0-based numbers are one less, and for a range
            // of deleted lines (`a` in the old file, `d` in the new one) the line after.
            let (s, e) = (own.start as i64, own.end as i64);
            let kind = match (h.kind, other_is_new) {
                (HunkKind::Add, false) | (HunkKind::Delete, true) => DiffMark::Append,
                (HunkKind::Delete, false) | (HunkKind::Add, true) => DiffMark::Delete,
                (HunkKind::Change, _) => DiffMark::Change,
            };
            match kind {
                // the line after the gap; `set_range` handles the gap at the end of the text
                DiffMark::Delete => self.set_range(kind, s, s, line_count),
                _ => self.set_range(kind, s, e - 1, line_count),
            }
        }
    }

    /// Clears the marks and sets them for `hunks` (`ViewDiffInfo`: the old DIFF goes first).
    pub fn replace_with_hunks(&mut self, hunks: &[Hunk], own_is_old: bool, line_count: usize) {
        self.clear();
        self.add_hunks(hunks, own_is_old, line_count);
    }

    /// The first marked line after `line` (`CDiffLineMgr::SearchDiffMark`, forward).
    pub fn next_after(&self, line: usize) -> Option<usize> {
        self.marks
            .range(line.saturating_add(1)..)
            .next()
            .map(|(&l, _)| l)
    }

    /// The last marked line before `line` (`SearchDiffMark`, backward).
    pub fn prev_before(&self, line: usize) -> Option<usize> {
        self.marks.range(..line).next_back().map(|(&l, _)| l)
    }

    /// The first marked line.
    pub fn first(&self) -> Option<usize> {
        self.marks.keys().next().copied()
    }

    /// The last marked line.
    pub fn last(&self) -> Option<usize> {
        self.marks.keys().next_back().copied()
    }

    /// Follows a change of the line list, as the bookmarks do.
    pub fn apply(&mut self, delta: LineDelta) {
        if delta.is_noop() || self.marks.is_empty() {
            return;
        }
        let tail = self.marks.split_off(&delta.start);
        let end = delta.start + delta.removed;
        for (line, mark) in tail {
            if line >= end {
                self.marks
                    .insert(line - delta.removed + delta.inserted, mark);
            }
        }
    }

    /// Drops the marks of lines that do not exist (a safety net after a whole-text change).
    pub fn truncate(&mut self, line_count: usize) {
        let _ = self.marks.split_off(&line_count);
    }
}

/// The line as the comparison of two documents sees it: the text and its line ending.
fn with_eol(doc: &Document, line: usize) -> Result<Option<String>> {
    Ok(doc.line(line)?.map(|l| {
        let mut text = l.text;
        if let Some(eol) = l.eol {
            text.push_str(eol_text(eol));
        }
        text
    }))
}

/// `col` rounded down to a char boundary of `text`.
fn floor_boundary(text: &str, col: usize) -> usize {
    let mut col = col.min(text.len());
    while !text.is_char_boundary(col) {
        col -= 1;
    }
    col
}

/// The first place where two documents differ, comparing from `from_a` in `a` and from `from_b` in
/// `b` (Sakura's ファイル内容比較, `Commander_COMPARE_core` in
/// `sakura_core/cmd/CViewCommander_Diff.cpp`): char by char, the line endings included. `None`
/// when both reach their end together; when only one of them does, the place is the start of the
/// row after its last line.
///
/// The places are caret positions: a place that falls inside a line ending (the lines end
/// differently) is the end of the text of the line.
pub fn first_difference(
    a: &Document,
    from_a: Pos,
    b: &Document,
    from_b: Pos,
) -> Result<Option<(Pos, Pos)>> {
    let (mut line_a, mut col_a) = (from_a.line, from_a.col);
    let (mut line_b, mut col_b) = (from_b.line, from_b.col);
    loop {
        let (text_a, text_b) = match (with_eol(a, line_a)?, with_eol(b, line_b)?) {
            (None, None) => return Ok(None),
            (Some(x), Some(y)) => (x, y),
            _ => return Ok(Some((Pos::new(line_a, 0), Pos::new(line_b, 0)))),
        };
        // a place in the middle of a char (the other document's byte column) is moved to its start
        col_a = floor_boundary(&text_a, col_a);
        col_b = floor_boundary(&text_b, col_b);
        let (mut rest_a, mut rest_b) = (text_a[col_a..].chars(), text_b[col_b..].chars());
        loop {
            match (rest_a.clone().next(), rest_b.clone().next()) {
                (Some(x), Some(y)) if x == y => {
                    col_a += x.len_utf8();
                    col_b += y.len_utf8();
                    rest_a.next();
                    rest_b.next();
                }
                (None, None) => break,
                _ => {
                    let end_a = a.line(line_a)?.map_or(0, |l| l.text.len());
                    let end_b = b.line(line_b)?.map_or(0, |l| l.text.len());
                    return Ok(Some((
                        Pos::new(line_a, col_a.min(end_a)),
                        Pos::new(line_b, col_b.min(end_b)),
                    )));
                }
            }
        }
        (line_a, col_a) = (line_a + 1, 0);
        (line_b, col_b) = (line_b + 1, 0);
    }
}

#[cfg(test)]
mod tests;
