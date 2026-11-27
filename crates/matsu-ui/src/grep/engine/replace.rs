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
//! * A file of the size of a huge file or more is not opened whole: the `stream` module reads it
//!   line by line and writes the new file as it goes, with the same line code (ADR-0020,
//!   chapter 5).
//!
//! Differences from Sakura: the hits of a file that could not be written are not counted (Sakura
//! counts them); a character that the encoding of the file
//! cannot write is refused, where Sakura writes `?`; the backup is a copy made before the file is
//! replaced, not the old file renamed after the new one is written.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use std::borrow::Cow;
use std::ops::Range;

use matsu_core::buffer::{Document, Pos};
use matsu_core::file::{FileError, SaveOptions, TextFile};
use matsu_core::search::{AppliedEdit, Matcher, Replacement, apply_replacements_descending};

use super::{ReplaceInfo, Runner, Stopped, TextShift, WindowSource, next_hit, shown_path};
use crate::grep::format::{self, Body, FilePaths, LineType, PathInfoState};

mod stream;

/// The suffix of the backup file (`.skrold`).
pub const BACKUP_SUFFIX: &str = ".skrold";

/// How often the stop flag is read, in lines.
const CANCEL_CHECK_LINES: usize = 32;

/// What a Grep 置換 of one document will do: the edits, and the number of replacements.
pub(super) struct Planned {
    /// One edit for each run of lines that changed, in order.
    pub(super) edits: Vec<Replacement>,
    /// What each match did to the text, in order (for a caret that goes with its text).
    pub(super) shifts: Vec<TextShift>,
    pub(super) replaced: usize,
}

/// A run of changed lines: its new text replaces the whole lines `first..=last`.
struct Region {
    first: usize,
    last: usize,
    /// The length of the text of the last line, and whether it has a break.
    last_len: usize,
    last_has_eol: bool,
    text: String,
}

impl Region {
    /// The edit that replaces the lines of the run with its text. The run begins at the start of a
    /// line and ends at the start of the next, or at the end of a last line that has no break, so
    /// no position of it is between a CR and an LF.
    fn into_edit(self) -> Replacement {
        let end = if self.last_has_eol {
            Pos::new(self.last + 1, 0)
        } else {
            Pos::new(self.last, self.last_len)
        };
        Replacement::new(Pos::new(self.first, 0), end, self.text)
    }
}

/// What `replace_line` found in a line.
pub(super) enum LineOutcome {
    /// The line has no hit.
    NoHit,
    /// The line has hits, and `replaced` replacements were made in it; `changed` tells whether
    /// the text is another one now (the new text is in [`LineWork::new_whole`]).
    Hit { replaced: usize, changed: bool },
    /// The replacement could not be made; the result says so.
    Failed,
}

/// The buffers of `replace_line`, kept from line to line, and what it remembers of the file.
#[derive(Default)]
pub(super) struct LineWork<'r> {
    /// The line with its break, as it was read.
    whole: String,
    /// The line with its break, as it is going to be written, after a changed `Hit`.
    pub(super) new_whole: String,
    pieces: Vec<(Range<usize>, Cow<'r, str>)>,
    /// A hit of this file is in the result already (for ファイル毎最初のみ検索).
    shown_any: bool,
}

/// What `replace_line` needs of the file and the search, the same for every line.
pub(super) struct LineEnv<'a, 'r> {
    pub(super) matcher: &'a dyn Matcher,
    pub(super) paths: &'a FilePaths<'a>,
    pub(super) code_name: &'a str,
    pub(super) replace: &'r ReplaceInfo,
}

/// A line that was read: its number (from 1), its text and its break.
#[derive(Clone, Copy)]
pub(super) struct LineRef<'t> {
    pub(super) number: u64,
    pub(super) text: &'t str,
    pub(super) eol: &'t str,
}

impl Runner<'_> {
    /// One line of the loop of `DoGrepReplaceFile`: writes the hits to the result and makes the
    /// new text of the line (in `work`). The line is handled with its line break, as Sakura reads
    /// it, so a match can cover the break. `shifts` takes what each match did to the text, with the
    /// offset of the line in it (for a caret that goes with its text).
    pub(super) fn replace_line<'r>(
        &mut self,
        work: &mut LineWork<'r>,
        env: &LineEnv<'_, 'r>,
        state: &mut PathInfoState,
        line: LineRef<'_>,
        shifts: Option<(&mut Vec<TextShift>, usize)>,
    ) -> LineOutcome {
        let layout = self.info.layout;
        let shown = shown_path(env.paths, layout);
        // A regular expression with a replacement is replaced a line at a time (`Replace` with the
        // `g` option); the rest is replaced match by match.
        let by_match = self.replacer.is_none();
        work.whole.clear();
        work.whole.push_str(line.text);
        work.whole.push_str(line.eol);
        let whole = work.whole.as_str();
        // The hits of the line, in the line as it was. The first is enough where only that
        // one is shown and replaced.
        let mut hits = Vec::new();
        let mut at = 0;
        while let Some(m) = next_hit(env.matcher, whole, &mut at) {
            hits.push(m);
            if !by_match {
                break;
            }
        }
        if hits.is_empty() {
            return LineOutcome::NoHit;
        }
        // The result: the first hit of the line; every hit with 該当部分, and no more than
        // the first of the file with ファイル毎最初のみ検索.
        if !(self.info.file_only && work.shown_any) {
            let all = layout.line_type == LineType::MatchPart && by_match && !self.info.file_only;
            let count = if all { hits.len() } else { 1 };
            for m in &hits[..count] {
                let column = format::utf16_len(&whole[..m.start]) + 1;
                let body = if layout.line_type == LineType::Line {
                    Body::Line(line.text)
                } else {
                    Body::Part(&whole[m.start..m.end])
                };
                format::push_path_info(&mut self.buffer, &layout, env.paths, env.code_name, state);
                format::push_result(
                    &mut self.buffer,
                    &layout,
                    shown,
                    env.code_name,
                    line.number,
                    column,
                    body,
                );
            }
            work.shown_any = true;
        }
        // What each match becomes, in the line with its break.
        work.pieces.clear();
        match &self.replacer {
            Some(replacer) => {
                let pieces = &mut work.pieces;
                let done = replacer.for_each_replacement(whole, &mut |range, text| {
                    pieces.push((range, Cow::Owned(text)));
                });
                if done.is_err() {
                    self.buffer
                        .push_str(&format::file_read_error(env.paths.full));
                    return LineOutcome::Failed;
                }
            }
            None => {
                // A match that begins before the end of the last (a whole word search can) is
                // left alone.
                let mut end = 0;
                for m in &hits {
                    if m.start >= end {
                        work.pieces
                            .push((m.start..m.end, Cow::Borrowed(env.replace.text.as_str())));
                        end = m.end;
                    }
                }
            }
        }
        let replaced = work.pieces.len();
        // The new text of the line with its break, made on the text: a CR and an LF are two
        // characters of it, whatever the document makes of them.
        work.new_whole.clear();
        let mut at = 0;
        let mut shifts = shifts;
        for (range, inserted) in &work.pieces {
            work.new_whole.push_str(&whole[at..range.start]);
            work.new_whole.push_str(inserted);
            at = range.end;
            if let Some((shifts, line_start)) = &mut shifts
                && whole[range.clone()] != **inserted
            {
                shifts.push(TextShift {
                    at: *line_start + range.start,
                    old_len: range.len(),
                    new_len: inserted.len(),
                });
            }
        }
        work.new_whole.push_str(&whole[at..]);
        LineOutcome::Hit {
            replaced,
            changed: work.new_whole != whole,
        }
    }

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
        let env = LineEnv {
            matcher: matcher.as_ref(),
            paths,
            code_name,
            replace,
        };
        let mut plan: Vec<Replacement> = Vec::new();
        let mut shifts: Vec<TextShift> = Vec::new();
        let mut region: Option<Region> = None;
        let mut work = LineWork::default();
        // Where the line begins in the text of the document, in bytes (the text of the lines and
        // their breaks), for `shifts`.
        let mut line_start = 0usize;
        let mut replaced = 0usize;
        let mut any_hit = false;
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
            let this_start = line_start;
            line_start += line.text.len() + eol.len();
            let outcome = self.replace_line(
                &mut work,
                &env,
                state,
                LineRef {
                    number: index as u64 + 1,
                    text: &line.text,
                    eol,
                },
                Some((&mut shifts, this_start)),
            );
            let (count, changed) = match outcome {
                LineOutcome::NoHit => {
                    plan.extend(region.take().map(Region::into_edit));
                    continue;
                }
                LineOutcome::Failed => return Ok(None),
                LineOutcome::Hit { replaced, changed } => (replaced, changed),
            };
            any_hit = true;
            replaced += count;
            if !changed {
                plan.extend(region.take().map(Region::into_edit));
            } else if let Some((from, to, middle)) = content_edit(&line.text, eol, &work.new_whole)
            {
                // The break of the line stays and no break comes into the text: only what changed
                // is replaced, so the line keeps what the document has of it (its bookmark, its
                // marks of a diff and of the function list).
                plan.extend(region.take().map(Region::into_edit));
                plan.push(Replacement::new(
                    Pos::new(index, from),
                    Pos::new(index, to),
                    middle.to_owned(),
                ));
            } else {
                match &mut region {
                    Some(run) if run.last + 1 == index => {
                        run.last = index;
                        run.last_len = line.text.len();
                        run.last_has_eol = !eol.is_empty();
                        run.text.push_str(&work.new_whole);
                    }
                    _ => {
                        plan.extend(region.take().map(Region::into_edit));
                        region = Some(Region {
                            first: index,
                            last: index,
                            last_len: line.text.len(),
                            last_has_eol: !eol.is_empty(),
                            text: std::mem::take(&mut work.new_whole),
                        });
                    }
                }
            }
            self.flush_when_due();
        }
        self.check_cancel()?;
        plan.extend(region.take().map(Region::into_edit));
        Ok(any_hit.then_some(Planned {
            edits: plan,
            shifts,
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
    ) -> Option<(Vec<AppliedEdit>, Vec<TextShift>, usize)> {
        match apply_replacements_descending(doc, planned.edits) {
            Ok(edits) if planned.replaced > 0 => Some((edits, planned.shifts, planned.replaced)),
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
        // A huge file is read in order and the new file is written as it goes (ADR-0020,
        // chapter 5), not opened whole.
        if let Some(from) = info.huge_file_len
            && std::fs::metadata(paths.full).is_ok_and(|m| m.len() >= from)
        {
            return self.replace_file_stream(paths, state, from, replace);
        }
        let Some(mut file) = self.open(paths.full)? else {
            return Ok(());
        };
        let code_name = self.code_name(file.encoding);
        let mut doc = file.document.clone();
        let Some(planned) = self.plan_replacements(&doc, paths, state, &code_name, replace)? else {
            return Ok(());
        };
        let Some((_, _, replaced)) = self.apply_plan(&mut doc, planned, paths.full) else {
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
        if let Some((edits, shifts, replaced)) = self.apply_plan(&mut doc, planned, paths.full) {
            self.hits += replaced;
            self.sink.replaced(edits, shifts);
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
        let staged = if backup {
            let Some(staged) = self.stage_backup(path) else {
                return Ok(false);
            };
            Some(staged)
        } else {
            None
        };
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
            if let Some(staged) = staged {
                let (old, staged) = staged.release();
                self.keep_backup(&staged, &old);
            }
            return Ok(true);
        };
        // The old file is as it was: the copy of it is not needed (`staged` removes it).
        drop(staged);
        if matches!(error, FileError::Cancelled) {
            return Err(Stopped);
        }
        self.report_write_error(path, &error);
        Ok(false)
    }

    /// Copies the file that is about to be replaced to a name of its own next to its backup (an
    /// earlier backup stays until the new content is in place). `None` when it cannot be copied;
    /// the result says so.
    fn stage_backup(&mut self, path: &str) -> Option<Staged> {
        let old = format!("{path}{BACKUP_SUFFIX}");
        let staged = staged_name(&old);
        if copy_new(path, &staged).is_err() {
            self.buffer.push_str(&format::replace_replace_error(&old));
            return None;
        }
        self.kept.insert(old.to_lowercase());
        self.kept.insert(staged.to_lowercase());
        Some(Staged {
            old,
            staged,
            armed: true,
        })
    }

    /// Tells in the result why the new content could not be written (not for a stop).
    fn report_write_error(&mut self, path: &str, error: &FileError) {
        match error {
            // Sakura fails at `DeleteFile` of the old file when it cannot be changed.
            FileError::ReadOnly => self.buffer.push_str(&format::replace_delete_error(path)),
            _ => self.buffer.push_str(&format::file_write_error(path)),
        }
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

/// The copy of the old file that is made before the file is replaced (`staged`), and the name of
/// the backup it becomes (`old`). It is as large as the file, so it is removed when this is
/// dropped, whatever way the replacement ends (a panic too), unless [`Staged::release`] hands it
/// over to [`Runner::keep_backup`].
struct Staged {
    old: String,
    staged: String,
    armed: bool,
}

impl Staged {
    /// Gives up the guard: the caller takes the names, and the copy stays where it is.
    fn release(mut self) -> (String, String) {
        self.armed = false;
        (
            std::mem::take(&mut self.old),
            std::mem::take(&mut self.staged),
        )
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.staged);
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

/// The edit of the text of a line (`text`, with the break `eol`) that makes its new text
/// `new_whole` (the new text and a break), as the byte range of `text` and what goes there: what
/// the old and the new text share at the start and the end (whole characters) is left out. `None`
/// if the break would change: the new line does not end with the break, or its text has a CR or
/// an LF (a new break, or a CR in front of an LF break that would join with it), or it is nothing
/// but an LF break.
fn content_edit<'a>(text: &str, eol: &str, new_whole: &'a str) -> Option<(usize, usize, &'a str)> {
    let content = new_whole.strip_suffix(eol)?;
    if content.contains(['\r', '\n']) {
        return None;
    }
    // A line that is left with nothing but an LF break would put it next to the break of the line
    // before, which may end with a CR and be replaced in the same run: that is a change of breaks.
    if content.is_empty() && eol.starts_with('\n') {
        return None;
    }
    let start: usize = text
        .chars()
        .zip(content.chars())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum();
    let end: usize = text[start..]
        .chars()
        .rev()
        .zip(content[start..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum();
    Some((
        start,
        text.len() - end,
        &content[start..content.len() - end],
    ))
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
