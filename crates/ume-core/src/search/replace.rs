//! Replacing plain text: the edits to make, and applying them.
//!
//! Sakura's replace commands are `CViewCommander::Command_REPLACE` and `Command_REPLACE_ALL`
//! (`sakura_core/cmd/CViewCommander_Search.cpp`). The parts that do not need a window are here;
//! undo grouping is left to the command that calls them, which gets the old and new text of
//! every edit back.

use std::ops::Range;

use crate::buffer::{BufferError, Document, Pos};

use super::find::{Control, Direction, Found, SearchError, find, find_all, start_after};
use super::matcher::Matcher;

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

/// Makes the edits one after the other. They must be in order and must not overlap, like the
/// result of [`plan_replace_all`]. If one fails, the ones before it stay applied.
pub fn apply_replacements(
    doc: &mut Document,
    replacements: &[Replacement],
) -> Result<Vec<AppliedEdit>, BufferError> {
    let mut applied = Vec::with_capacity(replacements.len());
    // The end of the previous edit, before (`old`) and after (`new`) it was made.
    let mut shift: Option<(Pos, Pos)> = None;
    let map = |shift: Option<(Pos, Pos)>, p: Pos| match shift {
        None => p,
        Some((old, new)) if p.line == old.line => {
            Pos::new(new.line, new.col + p.col.saturating_sub(old.col))
        }
        Some((old, new)) => Pos::new(new.line + p.line.saturating_sub(old.line), p.col),
    };
    for r in replacements {
        let (start, end) = (map(shift, r.start), map(shift, r.end));
        let removed = doc.text_between(start, end)?;
        let edit = doc.replace(start, end, &r.text)?;
        applied.push(AppliedEdit {
            start: edit.start,
            end: edit.end,
            removed,
            inserted: r.text.clone(),
            resume: Pos::new(edit.end.line, edit.end.col + r.skip),
        });
        shift = Some((r.end, edit.end));
    }
    Ok(applied)
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
    // After an empty match the search starts one char later, or it would find the same place
    // again (the guard of `start_after`). A deleted line has moved the next text to `resume`.
    let from = if found.start == found.end && target != ReplaceTarget::DeleteLine {
        start_after(doc, &(edit.resume..edit.resume))?
    } else {
        edit.resume
    };
    let next = find(doc, matcher, from, Direction::Forward, wrap, ctl);
    Ok(ReplaceStep { edit, next })
}

/// The edits that "replace all" makes, found without changing the document.
///
/// * The search goes from the start of the document, or of `range` (replace in the selection),
///   to its end without wrapping. A match must lie in the range; see [`find_all`].
/// * The text that was inserted is not searched again: the next search starts after it. That is
///   the same as searching the unchanged document, because matches do not overlap.
/// * [`ReplaceTarget::DeleteLine`] deletes a line once, however many matches it holds.
/// * If the search is cancelled, nothing has changed and the error is returned. (Sakura keeps
///   what was replaced so far.)
pub fn plan_replace_all(
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

/// The result of [`replace_all`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplaceAllResult {
    /// The edits made, in order. Their number is the count Sakura shows (`nReplaceNum`).
    pub edits: Vec<AppliedEdit>,
    /// Where the caret goes: after the last replacement, or `None` when nothing was replaced.
    pub caret: Option<Pos>,
}

/// Replaces every match; see [`plan_replace_all`] for the rules. The caller records
/// `ReplaceAllResult::edits` as one undo step.
pub fn replace_all(
    doc: &mut Document,
    matcher: &dyn Matcher,
    text: &str,
    target: ReplaceTarget,
    range: Option<Range<Pos>>,
    ctl: Control<'_>,
) -> Result<ReplaceAllResult, SearchError> {
    let plan = plan_replace_all(doc, matcher, text, target, range, ctl)?;
    let edits = apply_replacements(doc, &plan)?;
    let caret = edits.last().map(|e| e.resume);
    Ok(ReplaceAllResult { edits, caret })
}
