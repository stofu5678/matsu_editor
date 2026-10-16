// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/doc/layout/CLayoutMgr_DoLayout.cpp (_DoLayout, DoLayout_Range: the state at the start
//     of each line, and how far it is made again after an edit)
//   sakura_core/doc/layout/CLayout.h (m_nTypePrev, m_cExInfo)
//
// Original copyright notices of those files:
//   Copyright (C) 2018-2026, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
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

//! The color state at the start of every line of a document, kept so that no screen needs the
//! lines before it to be scanned again.
//!
//! Sakura keeps the state in every layout row (`CLayout::m_nTypePrev` and `m_cExInfo`) and makes
//! it while it lays the document out (`CLayoutMgr::_DoLayout`); after an edit
//! `CLayoutMgr::DoLayout_Range` makes the states of the following lines again until the state of
//! a line is the one it had (`bNeedChangeCOMMENTMODE`), and stops there.
//!
//! This does the same with these differences:
//!
//! * Most lines start in [`ColorState::Normal`], so only the lines where the state *changes* are
//!   stored. A block comment over a million lines is one entry.
//! * The states are made from the top in steps ([`ColorStates::advance`], as the layout does it in
//!   chunks), and [`ColorStates::known_lines`] says how far they are. A line past that is asked
//!   with [`ColorStates::state_at`], which scans at most a window of lines: from the last known
//!   line when that is near, else from [`LOCAL_WINDOW`] lines above, assuming no state there. The
//!   second answer can be wrong (a comment that began further up is missed); it is marked as
//!   not exact, and the caller asks again once [`ColorStates::known_lines`] has passed the line.
//!   Sakura has no such answer, because its layout is done before it paints; the colors of
//!   the lines on the screen here may change once, when the states reach them.
//! * After an edit, [`ColorStates::edited`] scans the changed lines and the lines after them
//!   until a state is the one it was (shifted by the number of lines the edit added). Only those
//!   lines are read.

use std::cell::Cell;
use std::time::{Duration, Instant};

use ume_core::buffer::{Document, Eol};

use crate::rules::ColorRules;
use crate::scan::{ColorState, next_state};

/// How many lines above a line [`ColorStates::state_at`] looks when the states are not known up
/// to it.
pub const LOCAL_WINDOW: usize = 200;

/// The edit of a document, in lines: lines `first..=old_last` of the old text were replaced by
/// lines `first..=new_last` of the new text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineEdit {
    pub first: usize,
    pub old_last: usize,
    pub new_last: usize,
}

/// The states at the start of the lines of a document.
#[derive(Clone, Debug, Default)]
pub struct ColorStates {
    /// The lines where the state changes: from the line on, until the next entry, the state is
    /// the one given. Before the first entry it is `Normal`. Sorted by line.
    points: Vec<(usize, ColorState)>,
    /// The states of the lines `0..=known` are exact; `tail` is the state of line `known`.
    known: usize,
    tail: ColorState,
    /// How many lines were scanned since the start, for the tests.
    scanned: Cell<u64>,
}

impl ColorStates {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets everything (another text, or other rules).
    pub fn reset(&mut self) {
        *self = Self {
            scanned: Cell::new(self.scanned.get()),
            ..Self::default()
        };
    }

    /// How many lines from the top have their state made: the states of the lines up to this one
    /// (the line itself included) are exact.
    pub fn known_lines(&self) -> usize {
        self.known
    }

    /// Whether the states of the whole document are made.
    pub fn is_complete(&self, doc: &Document, rules: &ColorRules) -> bool {
        !rules.has_range_based() || self.known >= doc.line_count()
    }

    /// How many lines were scanned so far to make states (the tests count them).
    pub fn scanned_lines(&self) -> u64 {
        self.scanned.get()
    }

    /// The state at `line` among the entries; exact for `line <= known`.
    fn lookup(points: &[(usize, ColorState)], line: usize) -> ColorState {
        let at = points.partition_point(|(l, _)| *l <= line);
        at.checked_sub(1)
            .map_or(ColorState::Normal, |i| points[i].1.clone())
    }

    /// The state at the start of `line`, and whether it is exact. See the module documentation.
    pub fn state_at(&self, doc: &Document, rules: &ColorRules, line: usize) -> (ColorState, bool) {
        if !rules.has_range_based() {
            return (ColorState::Normal, true);
        }
        if line <= self.known {
            return (Self::lookup(&self.points, line), true);
        }
        let (mut from, mut state, exact) = if line - self.known <= LOCAL_WINDOW {
            (self.known, self.tail.clone(), true)
        } else {
            (line - LOCAL_WINDOW, ColorState::Normal, false)
        };
        while from < line {
            match scan_line(doc, rules, from, &state) {
                Some(next) => state = next,
                None => break,
            }
            self.count_scan();
            from += 1;
        }
        (state, exact)
    }

    /// Makes the states of more lines from where they stopped, for at most `budget`. Returns
    /// whether all lines are done.
    pub fn advance(&mut self, doc: &Document, rules: &ColorRules, budget: Duration) -> bool {
        if !rules.has_range_based() {
            return true;
        }
        let total = doc.line_count();
        let start = Instant::now();
        // The clock is read after every line: a line can be as long as a megabyte (minified code,
        // logs), and then a few lines are all that fits in the budget.
        while self.known < total {
            if !self.step(doc, rules) || start.elapsed() >= budget {
                break;
            }
        }
        self.known >= total
    }

    fn count_scan(&self) {
        self.scanned.set(self.scanned.get() + 1);
    }

    /// Scans the line `known`. Returns false if the document has no such line.
    fn step(&mut self, doc: &Document, rules: &ColorRules) -> bool {
        let Some(next) = scan_line(doc, rules, self.known, &self.tail) else {
            // The document has fewer lines than it said: call it done.
            self.known = doc.line_count().max(self.known);
            return false;
        };
        self.count_scan();
        self.known += 1;
        if next != self.tail {
            self.points.push((self.known, next.clone()));
            self.tail = next;
        }
        true
    }

    /// Forgets the states after `line`, which stays exact: for edits that are not known as one
    /// range (several edits in one command, another view's edit).
    pub fn invalidate_after(&mut self, line: usize) {
        if line >= self.known {
            return;
        }
        self.points
            .truncate(self.points.partition_point(|(l, _)| *l <= line));
        self.known = line;
        self.tail = Self::lookup(&self.points, line);
    }

    /// The text changed as `edit` says; `doc` is the text after it. Makes the states of the
    /// changed lines, and of the lines after them until the state is what it was. A line is
    /// scanned only if it changed, or if its state may have changed because of the lines before
    /// it. If `budget` runs out before that, the states are made as far as they got and the rest
    /// goes on with [`advance`](Self::advance).
    pub fn edited(&mut self, doc: &Document, rules: &ColorRules, edit: LineEdit, budget: Duration) {
        if !rules.has_range_based() {
            return;
        }
        let s = edit.first;
        if s >= self.known {
            // Everything that is known is above the edit.
            return;
        }
        let delta = edit.new_last as isize - edit.old_last as isize;
        let new_after = edit.new_last + 1;
        let old_points = std::mem::take(&mut self.points);
        let old_known = self.known;
        let old_tail = std::mem::take(&mut self.tail);

        let keep = old_points.partition_point(|(l, _)| *l <= s);
        self.points = old_points[..keep].to_vec();
        let mut state = Self::lookup(&old_points, s);
        self.known = s;
        self.tail = state.clone();

        let start = Instant::now();
        let mut line = s;
        let mut converged_at = None;
        loop {
            if line >= new_after {
                let old_line = line as isize - delta;
                if old_line > old_known as isize {
                    break; // the old states end before this line
                }
                if Self::lookup(&old_points, old_line as usize) == state {
                    converged_at = Some(old_line as usize);
                    break;
                }
            }
            let Some(next) = scan_line(doc, rules, line, &state) else {
                break;
            };
            self.count_scan();
            line += 1;
            if next != state {
                self.points.push((line, next.clone()));
                state = next;
            }
            self.known = line;
            self.tail = state.clone();
            // A paste of many lines is not scanned at once either: `known` is where the scan got
            // to, and `advance` does the rest.
            if start.elapsed() >= budget {
                break;
            }
        }

        if let Some(old_line) = converged_at {
            for (l, st) in &old_points {
                if *l > old_line {
                    self.points
                        .push(((*l as isize + delta) as usize, st.clone()));
                }
            }
            self.known = (old_known as isize + delta) as usize;
            self.tail = old_tail;
        }
    }
}

/// The state after line `line`, when it starts in `state`. `None` if there is no such line.
fn scan_line(
    doc: &Document,
    rules: &ColorRules,
    line: usize,
    state: &ColorState,
) -> Option<ColorState> {
    let l = doc.line(line).ok().flatten()?;
    Some(line_end_state(rules, &l.text, l.eol, state))
}

/// The state a line ends in (the pass of the layout) given its text and line ending.
pub fn line_end_state(
    rules: &ColorRules,
    text: &str,
    eol: Option<Eol>,
    state: &ColorState,
) -> ColorState {
    match eol {
        Some(eol) => {
            let mut full = String::with_capacity(text.len() + 3);
            full.push_str(text);
            full.push_str(eol.as_str());
            next_state(rules, &full, text.len(), state)
        }
        None => next_state(rules, text, text.len(), state),
    }
}

#[cfg(test)]
mod tests;
