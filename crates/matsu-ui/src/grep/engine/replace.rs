// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/agent/CGrepAgent.cpp (DoGrepReplaceFile, CWriteData)
//
// sakura_core/agent/CGrepAgent.cpp:
//   Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the file above and was modified: the C++ code (`CFileLoad`,
// `CBregexp`, the UTF-16 buffers and the writing of `name.skrnew` with `CBinaryOutputStream`)
// was rewritten in Rust on `matsu_core::file`, `matsu_core::search` and `matsu_regex`, the new file is
// written by the save of `matsu_core::file` (a temporary file that replaces the old one), and the
// tests are new.
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

//! Grep 置換 of one file: the hits are shown as in a Grep, and the file is rewritten.
//!
//! Sakura: `CGrepAgent::DoGrepReplaceFile` and `CWriteData`. The file is read line by line (with
//! its line break), every hit is replaced in the text that is going to be written, and the file
//! is written only if there was a hit. Sakura writes `name.skrnew` and then replaces the old
//! file with it; here the new content is written by the save of `matsu_core::file`, which writes a
//! temporary file in the folder and renames it over the old one, so a file is never half written
//! (a stop, a full disk or a character that the encoding cannot write leave the old file as it
//! was). The encoding, the BOM and the line breaks are the ones of the file: the lines that are
//! not changed are copied byte for byte.
//!
//! * A regular expression (without 貼り付け) replaces the whole line at once with bregonig's
//!   replacement (`$1`, `\n`...), the `g` option of `CBregexp::Replace`; the number of
//!   replacements is the number of matches. Only the first match of a line is shown in the result.
//!   The line is matched with its line break, so a match can cover it (`\r\n`, `[\r\n]+`, `\s`):
//!   the line is joined with the next one, or its break is changed, in the file.
//! * Every other search (plain text, whole words, and a regular expression with the text of the
//!   clipboard) puts the text in for each match, in the line as it was read, with its line break.
//! * With バックアップ作成 the old file is kept as `name.skrold`; an earlier backup is replaced when the new content is in place.
//! * With ファイル毎最初のみ検索 only the first hit of a file is shown, but all are replaced.
//!
//! Differences from Sakura: the hits of a file that could not be written are not counted (Sakura
//! counts them); a character that the encoding of the file
//! cannot write is refused, where Sakura writes `?`; the backup is a copy made before the file is
//! replaced, not the old file renamed after the new one is written.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use matsu_core::buffer::{BufferError, Document, Pos};
use matsu_core::file::{FileError, SaveOptions, TextFile};
use matsu_core::search::{
    AppliedEdit, ReplaceTarget, Replacement, apply_replacements, replacement_for,
};

use super::{ReplaceInfo, Runner, Stopped, WindowSource, next_hit, shown_path};
use crate::grep::format::{self, Body, FilePaths, LineType, PathInfoState};

/// The suffix of the backup file (`.skrold`).
pub const BACKUP_SUFFIX: &str = ".skrold";

/// How often the stop flag is read, in lines.
const CANCEL_CHECK_LINES: usize = 32;

/// What a Grep 置換 of one document will do: the edits, and the number of replacements.
pub(super) struct Planned {
    pub(super) edits: Vec<Replacement>,
    pub(super) replaced: usize,
}

impl Runner<'_> {
    /// The loop of `DoGrepReplaceFile` over the lines of `doc`: writes the hits to the result and
    /// plans the new text of each line. `None` when nothing is to be replaced, or the text could
    /// not be read (the result says so).
    pub(super) fn plan_replacements(
        &mut self,
        doc: &Document,
        paths: &FilePaths<'_>,
        state: &mut PathInfoState,
        code_name: &str,
        replace: &ReplaceInfo,
    ) -> Result<Option<Planned>, Stopped> {
        let Some(matcher) = self.matcher.clone() else {
            return Ok(None);
        };
        let layout = self.info.layout;
        let shown = shown_path(paths, layout);
        // A regular expression with a replacement is replaced a line at a time (`Replace` with the
        // `g` option); the rest is replaced match by match. Either way the line is handled with
        // its line break, as Sakura reads it, so a match can cover the break.
        let by_match = self.replacer.is_none();
        let mut plan: Vec<Replacement> = Vec::new();
        let mut differ = Differ::default();
        let mut replaced = 0usize;
        let mut any_hit = false;
        let mut shown_any = false;
        for index in 0..doc.line_count() {
            if index % CANCEL_CHECK_LINES == 0 {
                self.check_cancel()?;
            }
            let line = match doc.line(index) {
                Ok(Some(line)) => line,
                Ok(None) => break,
                Err(_) => {
                    self.buffer.push_str(&format::file_read_error(paths.full));
                    return Ok(None);
                }
            };
            // The empty line after the last line break is not a line of the file: Sakura reads
            // none there, and nothing is written there.
            if line.text.is_empty() && line.eol.is_none() {
                continue;
            }
            let eol = line.eol.map_or("", matsu_core::model::eol_text);
            let whole = format!("{}{eol}", line.text);
            // The hits of the line, in the line as it was. The first is enough where only that
            // one is shown and replaced.
            let mut hits = Vec::new();
            let mut at = 0;
            while let Some(m) = next_hit(matcher.as_ref(), &whole, &mut at) {
                hits.push(m);
                if !by_match {
                    break;
                }
            }
            if hits.is_empty() {
                continue;
            }
            any_hit = true;
            // The result: the first hit of the line; every hit with 該当部分, and no more than
            // the first of the file with ファイル毎最初のみ検索.
            if !(self.info.file_only && shown_any) {
                let all =
                    layout.line_type == LineType::MatchPart && by_match && !self.info.file_only;
                let count = if all { hits.len() } else { 1 };
                for m in &hits[..count] {
                    let column = format::utf16_len(&whole[..m.start]) + 1;
                    let body = if layout.line_type == LineType::Line {
                        Body::Line(&line.text)
                    } else {
                        Body::Part(&whole[m.start..m.end])
                    };
                    format::push_path_info(&mut self.buffer, &layout, paths, code_name, state);
                    format::push_result(
                        &mut self.buffer,
                        &layout,
                        shown,
                        code_name,
                        index as u64 + 1,
                        column,
                        body,
                    );
                }
                shown_any = true;
            }
            // The new text of the line with its break.
            let (new, count) = match &self.replacer {
                Some(replacer) => match replacer.replace_subject(&whole) {
                    Ok(done) => done,
                    Err(_) => {
                        self.buffer.push_str(&format::file_read_error(paths.full));
                        return Ok(None);
                    }
                },
                None => put_in(&whole, &hits, &replace.text),
            };
            replaced += count;
            match line_replacement(doc, index, &line.text, eol, &new, &mut differ, &mut plan) {
                Ok(()) => {}
                Err(_) => {
                    self.buffer.push_str(&format::file_read_error(paths.full));
                    return Ok(None);
                }
            }
            self.flush_when_due();
        }
        self.check_cancel()?;
        Ok(any_hit.then_some(Planned {
            edits: plan,
            replaced,
        }))
    }

    /// Makes the plan in a copy of `doc`. The edits as they were made, and the number of
    /// replacements, or `None` (the result says why) when the document refuses them or nothing
    /// changes.
    fn apply_plan(
        &mut self,
        doc: &mut Document,
        planned: Planned,
        full: &str,
    ) -> Option<(Vec<AppliedEdit>, usize)> {
        match apply_replacements(doc, &merge_adjacent(planned.edits)) {
            Ok(edits) if planned.replaced > 0 => Some((edits, planned.replaced)),
            Ok(_) => None,
            Err(_) => {
                self.buffer.push_str(&format::file_read_error(full));
                None
            }
        }
    }

    /// `DoGrepReplaceFile`.
    pub(super) fn replace_file(
        &mut self,
        paths: &FilePaths<'_>,
        state: &mut PathInfoState,
    ) -> Result<(), Stopped> {
        let info = self.info;
        let Some(replace) = info.replace.as_ref() else {
            return Ok(());
        };
        let Some(mut file) = self.open(paths.full)? else {
            return Ok(());
        };
        let code_name = self.code_name(file.encoding);
        let mut doc = file.document.clone();
        let Some(planned) = self.plan_replacements(&doc, paths, state, &code_name, replace)? else {
            return Ok(());
        };
        let Some((_, replaced)) = self.apply_plan(&mut doc, planned, paths.full) else {
            return Ok(());
        };
        file.document = doc;
        if self.write_back(paths.full, &mut file, replace.backup)? {
            self.hits += replaced;
        }
        Ok(())
    }

    /// Grep 置換 of the text of a window (`:HWND:`): the same loop as for a file, on the snapshot
    /// of the document. Nothing is written here: the edits go to the sink, and the window that
    /// owns the document makes them (one undo step).
    ///
    /// Sakura reaches `DoGrepFile` for a window even in a Grep 置換, so it replaces nothing and
    /// prints the count of the hits as if it had (`CGrepAgent::DoGrep`, the `hWndTarget` loop).
    /// That is a bug and not something a macro could rely on: the result of a macro is the same
    /// text with the count, and the window is not changed.
    pub(super) fn replace_window(
        &mut self,
        window: &WindowSource,
        paths: &FilePaths<'_>,
        state: &mut PathInfoState,
        code_name: &str,
    ) -> Result<(), Stopped> {
        let info = self.info;
        let Some(replace) = info.replace.as_ref() else {
            return Ok(());
        };
        let mut doc = window.document.clone();
        let Some(planned) = self.plan_replacements(&doc, paths, state, code_name, replace)? else {
            return Ok(());
        };
        if let Some((edits, replaced)) = self.apply_plan(&mut doc, planned, paths.full) {
            self.hits += replaced;
            self.sink.replaced(edits);
        }
        Ok(())
    }

    /// `CWriteData::Close`: keeps the old file when asked, and puts the new content in its place.
    /// Returns whether the file was replaced; what went wrong is told in the result.
    fn write_back(
        &mut self,
        path: &str,
        file: &mut TextFile,
        backup: bool,
    ) -> Result<bool, Stopped> {
        let old = format!("{path}{BACKUP_SUFFIX}");
        // The file is copied before it is replaced, to a name of its own: an earlier backup stays
        // until the new content is in place.
        let staged = staged_name(&old);
        if backup {
            if copy_new(path, &staged).is_err() {
                self.buffer.push_str(&format::replace_replace_error(&old));
                return Ok(false);
            }
            self.kept.insert(old.to_lowercase());
            self.kept.insert(staged.to_lowercase());
        }
        let options = SaveOptions::keep(file);
        let cancel = self.cancel;
        let saved = file.save_as(path, &options, &mut |_, _| {
            if cancel.load(Ordering::Relaxed) {
                std::ops::ControlFlow::Break(())
            } else {
                std::ops::ControlFlow::Continue(())
            }
        });
        let Err(error) = saved else {
            if backup {
                self.keep_backup(&staged, &old);
            }
            return Ok(true);
        };
        if backup {
            // The old file is as it was: the copy of it is not needed.
            let _ = std::fs::remove_file(&staged);
        }
        match error {
            FileError::Cancelled => return Err(Stopped),
            // Sakura fails at `DeleteFile` of the old file when it cannot be changed.
            FileError::ReadOnly => self.buffer.push_str(&format::replace_delete_error(path)),
            _ => self.buffer.push_str(&format::file_write_error(path)),
        }
        Ok(false)
    }

    /// The copy of the old file takes the place of the earlier backup (`DeleteFile`, then
    /// `MoveFile` in `CWriteData::Close`). The new content is already in place, so when the
    /// earlier backup cannot be removed the copy stays under its own name: it is the only copy
    /// of the old content.
    fn keep_backup(&mut self, staged: &str, old: &str) {
        if Path::new(old).exists() && std::fs::remove_file(old).is_err() {
            self.buffer.push_str(&format::replace_delete_error(old));
            self.buffer.push_str(&format::replace_replace_error(staged));
        } else if std::fs::rename(staged, old).is_err() {
            self.buffer.push_str(&format::replace_replace_error(staged));
        }
    }
}

/// A name for the copy of a file that is about to be replaced, next to its backup. Nothing of
/// that name is expected to exist ([`copy_new`] refuses to write over a file).
fn staged_name(old: &str) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("{old}.{}-{n}.tmp", std::process::id())
}

/// The text of a line after the matches `hits` (found in `whole`, the line with its break) are
/// replaced by `text` as it is, and the number replaced. A match that begins before the end of
/// the last (a whole word search can) is left alone.
fn put_in(whole: &str, hits: &[matsu_core::search::Match], text: &str) -> (String, usize) {
    let mut out = String::with_capacity(whole.len());
    let (mut at, mut count) = (0, 0);
    for m in hits {
        if m.start < at {
            continue;
        }
        out.push_str(&whole[at..m.start]);
        out.push_str(text);
        at = m.end;
        count += 1;
    }
    out.push_str(&whole[at..]);
    (out, count)
}

/// Puts edits that touch each other (one ends where the next begins: the lines whose break is
/// replaced) into one. The document joins a CR and an LF that meet at the seam of two edits into
/// one CR LF, and the positions of the second edit are then wrong; as one edit the text is
/// just what the lines were replaced with, as Sakura writes it.
fn merge_adjacent(plan: Vec<Replacement>) -> Vec<Replacement> {
    let mut merged: Vec<Replacement> = Vec::with_capacity(plan.len());
    for next in plan {
        match merged.last_mut() {
            Some(last) if last.end == next.start => {
                last.end = next.end;
                last.text.push_str(&next.text);
            }
            _ => merged.push(next),
        }
    }
    merged
}

/// The edit that makes line `index` (`text` and its break `eol`; `whole` is both) into `new`,
/// which may have no break, another one or more than one. A line that keeps its own break is
/// edited in its text only, so the break stays as it was; otherwise the edit goes up to the
/// start of the next line, and a replacement without a break joins the line with the next one,
/// as the file written by Sakura does (it writes the text it built for each line).
fn line_replacement(
    doc: &Document,
    index: usize,
    text: &str,
    eol: &str,
    new: &str,
    differ: &mut Differ,
    plan: &mut Vec<Replacement>,
) -> Result<(), BufferError> {
    // The line as it was is `text` and its break `eol`.
    if new.strip_prefix(text) == Some(eol) {
        return Ok(());
    }
    if !(eol.is_empty() || new.ends_with(eol)) {
        let range = Pos::new(index, 0)..Pos::new(index + 1, 0);
        plan.push(replacement_for(doc, &range, new, ReplaceTarget::Match)?);
        return Ok(());
    }
    // The break stays: only what differs is edited, one edit for each stretch of it (one for each
    // match, with the text between the matches left alone), so a caret in the rest of the line
    // stays with its text.
    let kept = new.strip_suffix(eol).unwrap_or(new);
    for edit in differ.diff(text, kept) {
        let range = Pos::new(index, edit.from)..Pos::new(index, edit.to);
        plan.push(replacement_for(
            doc,
            &range,
            &kept[edit.new_from..edit.new_to],
            ReplaceTarget::Match,
        )?);
    }
    Ok(())
}

/// The most cells (characters of the old text times characters of the new) of the table that
/// [`Differ::diff`] fills; a line that differs over more than this is edited as one piece.
const DIFF_CELLS: usize = 1_000_000;

/// The fewest characters that two edits must have between them to stay two: a shorter stretch of
/// shared text is a coincidence of the alignment (the `i` of `hit` and `miss`), not text that a
/// caret should stay with, and is put into one edit.
const MIN_GAP: usize = 4;

/// One edit of a line: the bytes `from..to` of the old text become the bytes `new_from..new_to` of
/// the new text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LineEdit {
    from: usize,
    to: usize,
    new_from: usize,
    new_to: usize,
}

/// Finds the edits that make the old text of a line into the new. The buffers are kept between
/// lines, so a file of many changed lines does not allocate for each.
#[derive(Default)]
struct Differ {
    /// `table[i * width + j]`: the length of the longest common subsequence of `a[i..]` and `b[j..]`.
    table: Vec<u32>,
    /// The characters of the middle of the old text, with their byte offsets in it.
    a: Vec<(usize, char)>,
    /// The same for the new text.
    b: Vec<(usize, char)>,
    edits: Vec<LineEdit>,
}

impl Differ {
    /// The edits that make `old` into `new`: what the two share at the start and the end is left
    /// out, and the rest is aligned by its longest common subsequence, so the text between two
    /// changes stays as it is.
    fn diff(&mut self, old: &str, new: &str) -> &[LineEdit] {
        self.edits.clear();
        let start: usize = old
            .chars()
            .zip(new.chars())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a.len_utf8())
            .sum();
        let end: usize = old[start..]
            .chars()
            .rev()
            .zip(new[start..].chars().rev())
            .take_while(|(a, b)| a == b)
            .map(|(a, _)| a.len_utf8())
            .sum();
        let (from, to) = (start, old.len() - end);
        let (new_from, new_to) = (start, new.len() - end);
        let (middle_old, middle_new) = (&old[from..to], &new[new_from..new_to]);
        if middle_old.is_empty() && middle_new.is_empty() {
            return &self.edits;
        }
        let whole = LineEdit {
            from,
            to,
            new_from,
            new_to,
        };
        self.a.clear();
        self.a.extend(middle_old.char_indices());
        self.b.clear();
        self.b.extend(middle_new.char_indices());
        let (n, m) = (self.a.len(), self.b.len());
        if n == 0 || m == 0 || n.saturating_mul(m) > DIFF_CELLS {
            self.edits.push(whole);
            return &self.edits;
        }
        let width = m + 1;
        self.table.clear();
        self.table.resize((n + 1) * width, 0);
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                self.table[i * width + j] = if self.a[i].1 == self.b[j].1 {
                    self.table[(i + 1) * width + j + 1] + 1
                } else {
                    self.table[(i + 1) * width + j].max(self.table[i * width + j + 1])
                };
            }
        }
        let old_at = |a: &[(usize, char)], i: usize| a.get(i).map_or(to, |&(at, _)| from + at);
        let new_at =
            |b: &[(usize, char)], j: usize| b.get(j).map_or(new_to, |&(at, _)| new_from + at);
        // Where the stretch being changed starts, as indexes of `a` and `b`.
        let mut open: Option<(usize, usize)> = None;
        let (mut i, mut j) = (0, 0);
        while i < n || j < m {
            if i < n && j < m && self.a[i].1 == self.b[j].1 {
                if let Some((first_i, first_j)) = open.take() {
                    self.edits.push(LineEdit {
                        from: old_at(&self.a, first_i),
                        to: old_at(&self.a, i),
                        new_from: new_at(&self.b, first_j),
                        new_to: new_at(&self.b, j),
                    });
                }
                i += 1;
                j += 1;
                continue;
            }
            open.get_or_insert((i, j));
            if j >= m || (i < n && self.table[(i + 1) * width + j] >= self.table[i * width + j + 1])
            {
                i += 1;
            } else {
                j += 1;
            }
        }
        if let Some((first_i, first_j)) = open {
            self.edits.push(LineEdit {
                from: old_at(&self.a, first_i),
                to: old_at(&self.a, i),
                new_from: new_at(&self.b, first_j),
                new_to: new_at(&self.b, j),
            });
        }
        self.merge_close(old);
        &self.edits
    }

    /// Puts two edits with less than [`MIN_GAP`] characters between them into one. What is between
    /// them is the same in the old and the new text, so the new range is one range too.
    fn merge_close(&mut self, old: &str) {
        let mut kept = 0;
        for read in 0..self.edits.len() {
            let next = self.edits[read];
            if kept > 0 && old[self.edits[kept - 1].to..next.from].chars().count() < MIN_GAP {
                let last = &mut self.edits[kept - 1];
                last.to = next.to;
                last.new_to = next.new_to;
            } else {
                self.edits[kept] = next;
                kept += 1;
            }
        }
        self.edits.truncate(kept);
    }
}

/// Copies `from` to `to`, which must not exist (a file or a link in the way is an error and is
/// left alone). A copy that fails half way is removed.
fn copy_new(from: &str, to: &str) -> std::io::Result<()> {
    let mut source = std::fs::File::open(from)?;
    let mut target = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)?;
    let copied = std::io::copy(&mut source, &mut target)
        .and_then(|_| matsu_core::file::sync_file(&target))
        .and_then(|()| std::fs::set_permissions(to, source.metadata()?.permissions()));
    if copied.is_err() {
        let _ = std::fs::remove_file(to);
    }
    copied
}

#[cfg(test)]
mod tests {
    use super::Differ;

    /// The edits as byte ranges of `old` with the text they put.
    fn diff_edits(old: &str, new: &str) -> Vec<(usize, usize, String)> {
        Differ::default()
            .diff(old, new)
            .iter()
            .map(|e| (e.from, e.to, new[e.new_from..e.new_to].to_owned()))
            .collect()
    }

    #[test]
    fn each_match_of_a_line_is_one_edit() {
        // the text between the matches is not part of any edit
        assert_eq!(
            diff_edits("hit middle hit", "x middle x"),
            [(0, 3, "x".to_owned()), (11, 14, "x".to_owned())]
        );
    }

    #[test]
    fn what_the_lines_share_at_the_ends_is_left_out() {
        assert_eq!(
            diff_edits("ab needle zz", "ab n zz"),
            [(4, 9, String::new())]
        );
        assert!(diff_edits("same", "same").is_empty());
    }

    #[test]
    fn the_edits_are_on_character_boundaries() {
        // 3-byte characters: the byte ranges are of the old text
        assert_eq!(
            diff_edits("あ猫いいいい猫う", "あ犬いいいい犬う"),
            [(3, 6, "犬".to_owned()), (18, 21, "犬".to_owned())]
        );
    }

    #[test]
    fn a_short_stretch_between_changes_is_one_edit_with_them() {
        // the `i` that `hit` and `miss` share is not text to keep
        assert_eq!(diff_edits("hit", "miss"), [(0, 3, "miss".to_owned())]);
        assert_eq!(
            diff_edits("hit hit", "miss miss"),
            [(0, 7, "miss miss".to_owned())]
        );
    }

    #[test]
    fn the_edits_make_the_new_text() {
        for (old, new) in [
            ("a hit b hit c", "a xx b x c"),
            ("hithit", "x"),
            ("", "abc"),
            ("abc", ""),
            ("aXbXc", "aYYbc"),
        ] {
            let mut made = String::new();
            let mut at = 0;
            for (from, to, inserted) in diff_edits(old, new) {
                made.push_str(&old[at..from]);
                made.push_str(&inserted);
                at = to;
            }
            made.push_str(&old[at..]);
            assert_eq!(made, new, "{old:?} to {new:?}");
        }
    }
}
