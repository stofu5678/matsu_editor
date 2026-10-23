//! Replacing plain text: the edits to make, and applying them.
//!
//! Sakura's replace commands are `CViewCommander::Command_REPLACE` and `Command_REPLACE_ALL`
//! (`sakura_core/cmd/CViewCommander_Search.cpp`). The parts that do not need a window are here;
//! undo grouping is left to the command that calls them, which gets the old and new text of
//! every edit back.

use std::ops::Range;

use crate::buffer::{BufferError, Document, Pos};

use super::find::{Control, Direction, Found, SearchError, find, find_all, start_after};
use super::matcher::{Matcher, ceil_boundary};

/// What a replacement is done to (the replace dialog's 置換対象, `m_nReplaceTarget`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplaceTarget {
    /// Replace the match with the text.
    #[default]
    Match,
    /// Insert the text before the match and keep the match (選択始点に挿入).
    InsertBefore,
    /// Insert the text after the match and keep the match (選択終点に挿入).
    InsertAfter,
    /// Delete the whole line of the match; the text is not used (行削除).
    DeleteLine,
}

/// One edit to make: replace `start..end` with `text`. Positions are in the document as it is
/// before any of the edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replacement {
    pub start: Pos,
    pub end: Pos,
    pub text: String,
    /// Columns to skip on the line where `text` ends before the next search starts. Sakura
    /// moves the caret past the match that [`ReplaceTarget::InsertBefore`] kept.
    skip: usize,
}

/// An edit that was made: enough to undo it with `replace(start, end, &removed)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppliedEdit {
    /// Where the edit began, in the document after all edits before it and this one.
    pub start: Pos,
    /// Where the inserted text ends, in the same document.
    pub end: Pos,
    /// The text that was there before. Undo with `replace(start, end, &removed)`, in the reverse
    /// order of the edits. One case does not round-trip: if `inserted` ends with a CR and an LF
    /// follows, the buffer joins them into one CRLF and `end` is placed before it
    /// (`Document::replace`).
    pub removed: String,
    /// The text that is there now.
    pub inserted: String,
    /// Where the next search starts: `end`, moved past a kept match for
    /// [`ReplaceTarget::InsertBefore`] (Sakura: `ptTmp` in `Command_REPLACE`).
    pub resume: Pos,
}

/// The edit that replaces `found` according to `target`.
pub fn replacement_for(
    doc: &Document,
    found: &Range<Pos>,
    text: &str,
    target: ReplaceTarget,
) -> Result<Replacement, BufferError> {
    let (start, end) = (found.start, found.end);
    let plain = |start, end, text: &str, skip| Replacement {
        start,
        end,
        text: text.to_owned(),
        skip,
    };
    Ok(match target {
        ReplaceTarget::Match => plain(start, end, text, 0),
        ReplaceTarget::InsertBefore => {
            let kept = if start.line == end.line {
                end.col.saturating_sub(start.col)
            } else {
                0
            };
            plain(start, start, text, kept)
        }
        ReplaceTarget::InsertAfter => plain(end, end, text, 0),
        ReplaceTarget::DeleteLine => {
            let line = start.line;
            // The last line may have no line ending: then the deletion stops at its end.
            let to = match doc.line(line)? {
                Some(l) if l.eol.is_none() && line + 1 >= doc.line_count() => {
                    Pos::new(line, l.text.len())
                }
                _ => Pos::new(line + 1, 0),
            };
            plain(Pos::new(line, 0), to, "", 0)
        }
    })
}

/// Where position `p` of the old document is after an edit, if `p` was at or after the old end
/// of the edit (`old_end`) and the edit now ends at `new_end`.
fn shift_pos(old_end: Pos, new_end: Pos, p: Pos) -> Pos {
    if p < old_end {
        new_end
    } else if p.line == old_end.line {
        Pos::new(new_end.line, new_end.col + p.col - old_end.col)
    } else {
        Pos::new(new_end.line + p.line - old_end.line, p.col)
    }
}

/// Makes the edits one after the other. They must be in order and must not overlap, like the
/// result of a plan. If one fails, none is made.
pub fn apply_replacements(
    doc: &mut Document,
    replacements: &[Replacement],
) -> Result<Vec<AppliedEdit>, BufferError> {
    let mut work = doc.clone();
    let mut applied = Vec::with_capacity(replacements.len());
    // The end of the previous edit, before and after it was made.
    let mut shift: Option<(Pos, Pos)> = None;
    for r in replacements {
        let (start, end) = match shift {
            Some((old, new)) => (shift_pos(old, new, r.start), shift_pos(old, new, r.end)),
            None => (r.start, r.end),
        };
        applied.push(apply_one(&mut work, start, end, r)?);
        shift = Some((r.end, applied[applied.len() - 1].end));
    }
    *doc = work;
    Ok(applied)
}

/// Makes one edit at `start..end` (positions in `doc` as it is now).
fn apply_one(
    doc: &mut Document,
    start: Pos,
    end: Pos,
    r: &Replacement,
) -> Result<AppliedEdit, BufferError> {
    let removed = doc.text_between(start, end)?;
    let edit = doc.replace(start, end, &r.text)?;
    Ok(AppliedEdit {
        start: edit.start,
        end: edit.end,
        removed,
        inserted: r.text.clone(),
        resume: Pos::new(edit.end.line, edit.end.col + r.skip),
    })
}

/// Replaces `found` once.
pub fn replace_one(
    doc: &mut Document,
    found: &Range<Pos>,
    text: &str,
    target: ReplaceTarget,
) -> Result<AppliedEdit, BufferError> {
    let r = replacement_for(doc, found, text, target)?;
    Ok(apply_replacements(doc, std::slice::from_ref(&r))?.remove(0))
}

/// The result of [`replace_and_find_next`].
#[derive(Debug)]
pub struct ReplaceStep {
    /// The replacement, which has been made whatever happened to the search after it. Record it
    /// for undo.
    pub edit: AppliedEdit,
    /// The next match, or why the search stopped (cancelled, for example).
    pub next: Result<Option<Found>, SearchError>,
}

/// Where the search goes on after `edit` replaced `found`: after the new text, and one char
/// further after an empty match, or the same place would be found again (the guard of
/// [`start_after`]). A deleted line has moved the next text to `resume`.
fn resume_after(
    doc: &Document,
    found: &Range<Pos>,
    edit: &AppliedEdit,
    target: ReplaceTarget,
) -> Result<Pos, BufferError> {
    if found.start == found.end && target != ReplaceTarget::DeleteLine {
        start_after(doc, &(edit.resume..edit.resume))
    } else {
        Ok(edit.resume)
    }
}

/// What Sakura's 置換 button does: replace the match that is selected, then find the next one
/// from there (with `wrap`, the option 先頭（末尾）から再検索).
pub fn replace_and_find_next(
    doc: &mut Document,
    matcher: &dyn Matcher,
    found: &Range<Pos>,
    text: &str,
    target: ReplaceTarget,
    wrap: bool,
    ctl: &mut Control<'_>,
) -> Result<ReplaceStep, BufferError> {
    let edit = replace_one(doc, found, text, target)?;
    let from = resume_after(doc, found, &edit, target)?;
    let next = find(doc, matcher, from, Direction::Forward, wrap, ctl);
    Ok(ReplaceStep { edit, next })
}

/// The edits that "replace all" makes, found without changing the document. This is the same as
/// replacing one after the other only when a match does not depend on the text around it
/// ([`Matcher::is_context_free`]); [`replace_all`] takes care of that.
///
/// * The search goes from the start of the document, or of `range` (replace in the selection),
///   to its end without wrapping. A match must lie in the range; see [`find_all`].
/// * The text that was inserted is not searched again: the next search starts after it.
/// * [`ReplaceTarget::DeleteLine`] deletes a line once, however many matches it holds.
fn plan_replace_all(
    doc: &Document,
    matcher: &dyn Matcher,
    text: &str,
    target: ReplaceTarget,
    range: Option<Range<Pos>>,
    ctl: Control<'_>,
) -> Result<Vec<Replacement>, SearchError> {
    let mut plan = Vec::new();
    let mut deleted_through: Option<usize> = None;
    for found in find_all(doc, matcher, range, ctl) {
        let found = found?;
        if target == ReplaceTarget::DeleteLine {
            if deleted_through.is_some_and(|l| found.start.line <= l) {
                continue;
            }
            deleted_through = Some(found.start.line);
        }
        plan.push(replacement_for(doc, &found.range(), text, target)?);
    }
    Ok(plan)
}

/// Replaces matches one after the other in `work`, searching the text as the earlier
/// replacements left it, as Sakura does (`Command_REPLACE_ALL` searches again from the caret after
/// every replacement). For a whole-word search that matters: a replacement can join or split
/// words next to it.
fn replace_in_sequence(
    work: &mut Document,
    matcher: &dyn Matcher,
    text: &str,
    target: ReplaceTarget,
    range: Option<Range<Pos>>,
    mut ctl: Control<'_>,
) -> Result<Vec<AppliedEdit>, SearchError> {
    let first = range.as_ref().map_or(Pos::new(0, 0), |r| r.start);
    // Where the range ends now; it moves as the text before it changes.
    let mut limit = range.map(|r| r.end);
    ctl.begin(work.line_count());
    let (mut line, mut col) = (first.line, first.col);
    let mut edits = Vec::new();
    loop {
        ctl.tick(line)?;
        if limit.is_some_and(|l| line > l.line || (line == l.line && l.col == 0)) {
            break;
        }
        let Some(l) = work.line(line)? else { break };
        let Some(start) = ceil_boundary(&l.text, col) else {
            (line, col) = (line + 1, 0);
            continue;
        };
        // A whole-word match can begin before `start` (see `Matcher::find_at`); text before the
        // point where the search goes on (or the range starts) is not replaced.
        let mut found = matcher.find_at(&l.text, start)?;
        loop {
            let next_from = match &found {
                Some(m) if m.start < start => m.end.max(start + 1),
                _ => break,
            };
            found = match ceil_boundary(&l.text, next_from) {
                Some(next) => matcher.find_at(&l.text, next)?,
                None => None,
            };
        }
        let Some(m) = found else {
            (line, col) = (line + 1, 0);
            continue;
        };
        if limit.is_some_and(|l| line == l.line && m.end > l.col) {
            break;
        }
        let range = Pos::new(line, m.start)..Pos::new(line, m.end);
        let r = replacement_for(work, &range, text, target)?;
        let edit = apply_one(work, r.start, r.end, &r)?;
        limit = limit.map(|l| shift_pos(r.end, edit.end, l));
        let from = resume_after(work, &range, &edit, target)?;
        (line, col) = (from.line, from.col);
        edits.push(edit);
    }
    ctl.report();
    Ok(edits)
}

/// The result of [`replace_all`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplaceAllResult {
    /// The edits made, in order.
    pub edits: Vec<AppliedEdit>,
    /// Where the caret goes: after the last replacement, or `None` when nothing was replaced.
    pub caret: Option<Pos>,
    /// The number of replacements, the count Sakura shows (`nReplaceNum`). It is the number of
    /// `edits`, except that matches which share one line ending make one edit.
    pub count: usize,
}

/// Replaces every match, from the start of the document or of `range` (replace in the
/// selection), to its end, without wrapping around. The caller records
/// `ReplaceAllResult::edits` as one undo step.
///
/// * A match must lie in the range. Sakura stops at the first match that ends after the
///   selection, and a selection that ends at column 0 leaves that last line out
///   (`Command_REPLACE_ALL`, `sakura_core/cmd/CViewCommander_Search.cpp`).
/// * The text that was inserted is not searched again: the search goes on after it.
/// * [`ReplaceTarget::DeleteLine`] deletes a line once, however many matches it holds.
/// * Every replacement is searched for in the text as the earlier ones left it. A matcher that
///   says [`Matcher::is_context_free`] is searched in the unchanged document instead, which gives
///   the same result and is faster on long lines.
/// * If the search is cancelled or fails, the document is not changed and the error is returned.
///   (Sakura keeps what was replaced so far.)
pub fn replace_all(
    doc: &mut Document,
    matcher: &dyn Matcher,
    text: &str,
    target: ReplaceTarget,
    range: Option<Range<Pos>>,
    ctl: Control<'_>,
) -> Result<ReplaceAllResult, SearchError> {
    let mut work = doc.clone();
    let edits = if matcher.is_context_free() {
        let plan = plan_replace_all(&work, matcher, text, target, range, ctl)?;
        apply_replacements(&mut work, &plan)?
    } else {
        replace_in_sequence(&mut work, matcher, text, target, range, ctl)?
    };
    *doc = work;
    let caret = edits.last().map(|e| e.resume);
    let count = edits.len();
    Ok(ReplaceAllResult {
        edits,
        caret,
        count,
    })
}
