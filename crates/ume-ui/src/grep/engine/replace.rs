// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/agent/CGrepAgent.cpp (DoGrepReplaceFile, CWriteData)
//
// sakura_core/agent/CGrepAgent.cpp:
//   Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the file above and was modified: the C++ code (`CFileLoad`,
// `CBregexp`, the UTF-16 buffers and the writing of `name.skrnew` with `CBinaryOutputStream`)
// was rewritten in Rust on `ume_core::file`, `ume_core::search` and `ume_regex`, the new file is
// written by the save of `ume_core::file` (a temporary file that replaces the old one), and the
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
//! file with it; here the new content is written by the save of `ume_core::file`, which writes a
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

use ume_core::buffer::{Document, Pos};
use ume_core::file::{FileError, SaveOptions, TextFile};
use ume_core::search::{ReplaceTarget, Replacement, apply_replacements, replacement_for};

use super::{Runner, Stopped, next_hit, shown_path};
use crate::grep::format::{self, Body, FilePaths, LineType, PathInfoState};

/// The suffix of the backup file (`.skrold`).
pub const BACKUP_SUFFIX: &str = ".skrold";

/// How often the stop flag is read, in lines.
const CANCEL_CHECK_LINES: usize = 32;

impl Runner<'_> {
    /// `DoGrepReplaceFile`.
    pub(super) fn replace_file(
        &mut self,
        paths: &FilePaths<'_>,
        state: &mut PathInfoState,
    ) -> Result<(), Stopped> {
        let Some(replace) = self.info.replace.clone() else {
            return Ok(());
        };
        let Some(mut file) = self.open(paths.full)? else {
            return Ok(());
        };
        let code_name = self.code_name(&file);
        let Some(matcher) = self.matcher.clone() else {
            return Ok(());
        };
        let layout = self.info.layout;
        let shown = shown_path(paths, &layout);
        // A regular expression with a replacement is replaced a line at a time (`Replace` with the
        // `g` option); the rest is replaced match by match. Either way the line is handled with
        // its line break, as Sakura reads it, so a match can cover the break.
        let by_match = self.replacer.is_none();
        let mut doc = file.document.clone();
        let mut plan: Vec<Replacement> = Vec::new();
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
                    return Ok(());
                }
            };
            // The empty line after the last line break is not a line of the file: Sakura reads
            // none there, and nothing is written there.
            if line.text.is_empty() && line.eol.is_none() {
                continue;
            }
            let eol = line.eol.map_or("", ume_core::model::eol_text);
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
                    format::push_path_info(&mut self.buffer, &layout, paths, &code_name, state);
                    format::push_result(
                        &mut self.buffer,
                        &layout,
                        shown,
                        &code_name,
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
                        return Ok(());
                    }
                },
                None => put_in(&whole, &hits, &replace.text),
            };
            replaced += count;
            if let Some(edit) = line_replacement(&doc, index, &line.text, eol, &whole, new) {
                plan.push(edit);
            }
            self.flush_when_due();
        }
        self.check_cancel()?;
        if !any_hit {
            return Ok(());
        }
        let replaced = apply_replacements(&mut doc, &plan).map(|_| replaced).ok();
        let Some(replaced) = replaced else {
            self.buffer.push_str(&format::file_read_error(paths.full));
            return Ok(());
        };
        if replaced == 0 {
            return Ok(());
        }
        file.document = doc;
        if self.write_back(paths.full, &mut file, replace.backup)? {
            self.hits += replaced;
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
fn put_in(whole: &str, hits: &[ume_core::search::Match], text: &str) -> (String, usize) {
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
    whole: &str,
    new: String,
) -> Option<Replacement> {
    if new == whole {
        return None;
    }
    let start = Pos::new(index, 0);
    let (end, new) = if eol.is_empty() {
        (Pos::new(index, text.len()), new)
    } else if let Some(kept) = new.strip_suffix(eol) {
        (Pos::new(index, text.len()), kept.to_owned())
    } else {
        (Pos::new(index + 1, 0), new)
    };
    replacement_for(doc, &(start..end), &new, ReplaceTarget::Match).ok()
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
        .and_then(|_| target.sync_all())
        .and_then(|()| std::fs::set_permissions(to, source.metadata()?.permissions()));
    if copied.is_err() {
        let _ = std::fs::remove_file(to);
    }
    copied
}
