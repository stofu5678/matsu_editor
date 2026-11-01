//! The colors of a color print: the coloring of the window (`matsu-coloring`), read through the
//! snapshot of the document that the print reads its lines from.
//!
//! Sakura colors the print with the same strategies as the screen (`CColorStrategyPool`,
//! `CPrintPreview::GetColorStrategy`) and the color entries of the type of the document
//! (`m_ColorInfoArr`); here that is [`ColorRules`], which are made from the type once and drop
//! the strategies whose entry is not shown. The print takes the text color, the bold and the
//! underline of the entry of each run (`Print_DrawBlock`), not its background.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use matsu_coloring::{ColorRules, ColorStates, SKIP_LINE_BYTES, line_runs};
use matsu_core::buffer::{Document, Eol};
use matsu_print::{LineStyler, StyleRun, TextStyle};

use crate::settings::view_settings::rgb_of;

/// How long the states of the lines are made at a time while a page waits for them.
const STATES_STEP: Duration = Duration::from_millis(50);

/// [`LineStyler`] over a snapshot of a document.
pub(crate) struct DocStyler {
    doc: Document,
    rules: Arc<ColorRules>,
    /// The state at the start of each line, made from the top as far as the printed pages need.
    /// (Printing is the one place that needs the states in a given order of the pages, and a
    /// page that is far down makes all the states above it, as Sakura's layout does before it
    /// prints.)
    states: Mutex<ColorStates>,
}

impl DocStyler {
    pub(crate) fn new(doc: Document, rules: Arc<ColorRules>) -> Self {
        Self {
            doc,
            rules,
            states: Mutex::new(ColorStates::new()),
        }
    }

    /// The state `line` starts in, made exactly.
    fn state_at(&self, line: usize) -> matsu_coloring::ColorState {
        let mut states = self.states.lock().unwrap_or_else(|e| e.into_inner());
        while states.known_lines() < line {
            if states.advance(&self.doc, &self.rules, STATES_STEP) {
                break;
            }
        }
        states.state_at(&self.doc, &self.rules, line).0
    }
}

/// The style of a color entry.
fn style_of(entry: &matsu_config::settings::color::ColorEntry) -> TextStyle {
    TextStyle {
        color: rgb_of(entry.text),
        bold: entry.bold,
        underline: entry.underline,
    }
}

impl LineStyler for DocStyler {
    fn plain(&self) -> TextStyle {
        // The header, the footer and the line numbers are in the text color of the type, but
        // neither bold nor underlined (they are drawn in the plain print font).
        TextStyle {
            bold: false,
            underline: false,
            ..style_of(self.rules.entry(matsu_coloring::ColorKind::Text))
        }
    }

    fn style_line(&self, line: usize, text: &str, has_eol: bool, out: &mut Vec<StyleRun>) {
        out.clear();
        // The text color of the type colors all that no strategy does; nothing to say then.
        if self.rules.colors_nothing() {
            return;
        }
        // A very long line of a file read by ranges has no colors (ADR-0020, 9.10), as on the
        // screen.
        if self.doc.is_ranged()
            && self
                .doc
                .line_raw_len(line)
                .ok()
                .flatten()
                .is_some_and(|n| n >= SKIP_LINE_BYTES)
        {
            return;
        }
        let state = self.state_at(line);
        let eol = has_eol.then_some(Eol::Lf);
        for run in line_runs(&self.rules, text, eol, &state, text.len()) {
            // The runs go through the line ending, which is not printed.
            let end = run.end.min(text.len());
            if run.start >= end {
                continue;
            }
            out.push(StyleRun {
                range: run.start..end,
                style: style_of(self.rules.entry(run.kind)),
            });
        }
    }
}

#[cfg(test)]
mod tests;
