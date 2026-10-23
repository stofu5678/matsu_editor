//! What the minimap (`crate::minimap`) takes from a view: the colored text of some rows, the rows
//! the view shows, and moving the caret to a row.
//!
//! The minimap is a second view of the same text (`CMiniMapView` is a `CEditView`:
//! `sakura_core/view/CMiniMapView.cpp`), so it lays out and colors the text the way the view does.
//! Sakura draws it with the code of the view and switches some parts off for it (`m_bMiniMap`:
//! the line numbers, the ruler, the caret line, the odd lines, the note lines and the line-end
//! marks are not drawn). This file gives the pieces of text the same way, without those parts.

use std::ops::Range;

use gpui::{App, Context};
use matsu_coloring::ColorState;

use super::EditorView;
use super::colors::LineColorRequest;
use super::syntax::{self, Label};
use crate::search::highlight::{highlight_columns, slot_of};
use crate::settings::view_settings::Rgb;

/// A piece of text of one color on a row of the minimap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MiniPiece {
    /// The column where the piece starts.
    pub col: u32,
    pub text: String,
    pub color: Rgb,
}

/// A row to draw.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MiniRow {
    /// The row in the document.
    pub row: usize,
    pub pieces: Vec<MiniPiece>,
    /// Backgrounds of pieces that are not the background of the row: from column, to column, color.
    pub backs: Vec<(u32, u32, Rgb)>,
}

/// The colors of the minimap besides the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MiniLook {
    /// The background of a row (`TXT`).
    pub background: Rgb,
    /// The background of the rows the view shows (`PGV`), if it is on.
    pub page_view: Option<Rgb>,
}

impl EditorView {
    /// The colors of the minimap.
    pub(crate) fn minimap_look(&self) -> MiniLook {
        let colors = &self.prefs.settings.colors;
        MiniLook {
            background: colors.background,
            page_view: colors.page_view_back,
        }
    }

    /// How many rows the layout has.
    pub(crate) fn minimap_total_rows(&self) -> usize {
        self.layout.row_count()
    }

    /// The rows the view shows now (`GetViewTopLine()` to `GetBottomLine()`), the part of the text
    /// the minimap marks.
    pub(crate) fn minimap_page(&self) -> Range<usize> {
        let first = (self.scroll_y / self.metrics.line_height.max(1.0))
            .floor()
            .max(0.0) as usize;
        first..first + self.metrics.visible_rows()
    }

    /// Up to `count` rows from `first_row`, each cut into pieces of one color: the same colors as
    /// the view's, with the matches of the search. What is right of the column `max_cols` is not
    /// given (the minimap is not as wide), so a very long line costs no more than a short one.
    pub(crate) fn minimap_rows(
        &self,
        first_row: usize,
        count: usize,
        max_cols: u32,
        cx: &App,
    ) -> Vec<MiniRow> {
        let shared = self.document.read(cx);
        let doc = shared.doc();
        let layout = &self.layout;
        let colors = &self.prefs.settings.colors;
        let rules = &self.prefs.settings.coloring;
        let rows = layout.rows(doc, first_row, count).unwrap_or_default();
        // A character is a column wide at least (but for the rare ones of no width, so a few more).
        let max_chars = max_cols as usize + 16;

        let search = &self.search;
        let shown_found = colors.found.len();
        let words = search.plain().filter(|_| search.options().whole_word);
        let mut found = match search
            .matcher()
            .filter(|_| search.mark() && shown_found > 0)
        {
            Some(matcher) => highlight_columns(layout, doc, &rows, &**matcher, &|text| {
                slot_of(words.and_then(|w| w.word_slot(text)), shown_found)
            }),
            None => Vec::new(),
        }
        .into_iter();

        let version = shared.version();
        let mut line_colors: Option<(usize, std::sync::Arc<Vec<matsu_coloring::ColorRun>>)> = None;
        let mut carried: Option<(usize, ColorState)> = None;
        let mut out = Vec::with_capacity(rows.len());
        for (row_index, row) in rows.iter().enumerate() {
            let row_found = found.next().unwrap_or_default();
            let mut mini = MiniRow {
                row: row.row,
                ..MiniRow::default()
            };
            if row.is_eof_row {
                out.push(mini);
                continue;
            }
            // Only the text that can be seen goes through the coloring.
            let clipped = &row.text()[..clip_len(row.text(), max_chars)];
            // The same walk as the view's: the line is scanned from its start, in the state the
            // lines before it left, as far as the last row of it that is asked for.
            let row_runs = if rules.colors_nothing() {
                Vec::new()
            } else {
                if line_colors.as_ref().map(|l| l.0) != Some(row.line)
                    && let Ok(Some(ll)) = layout.line_layout(doc, row.line)
                {
                    let state = match carried.take() {
                        Some((line, state)) if line == row.line => state,
                        _ => self.color_states.state_at(doc, rules, row.line).0,
                    };
                    let upto = rows[row_index..]
                        .iter()
                        .take_while(|r| r.line == row.line)
                        .map(|r| r.start.col + clip_len(r.text(), max_chars))
                        .max()
                        .unwrap_or(0);
                    let want_next = rows[row_index..]
                        .iter()
                        .any(|r| !r.is_eof_row && r.line > row.line);
                    let next_known = (row.line < self.color_states.known_lines())
                        .then(|| self.color_states.state_at(doc, rules, row.line + 1).0);
                    let colored = self.colored_line(LineColorRequest {
                        rules,
                        version,
                        line: row.line,
                        layout: &ll,
                        state: &state,
                        upto,
                        want_next,
                        next_known,
                    });
                    carried = colored.next.map(|next| (row.line + 1, next));
                    line_colors = Some((row.line, colored.runs));
                }
                match &line_colors {
                    Some((line, runs)) if *line == row.line => {
                        syntax::row_runs(runs, row.start.col, row.start.col + clipped.len())
                    }
                    _ => Vec::new(),
                }
            };
            let styling = syntax::style_row(
                colors,
                rules,
                &syntax::RowInput {
                    text: clipped,
                    start_col: row.start_column,
                    cfg: layout.config(),
                    runs: &row_runs,
                    found: &row_found,
                    selected: None,
                    row_back: colors.background,
                },
            );
            for item in styling.items {
                // The marks of tabs and spaces are too small to see at this size.
                if let Label::Text(text) = item.label
                    && item.col < max_cols
                {
                    // a character is at least one column wide
                    let room = (max_cols - item.col) as usize;
                    mini.pieces.push(MiniPiece {
                        col: item.col,
                        text: text.chars().take(room).collect(),
                        color: item.style.text,
                    });
                }
            }
            mini.backs = styling
                .backs
                .into_iter()
                .filter(|&(from, _, _)| from < max_cols)
                .map(|(from, to, color)| (from, to.min(max_cols), color))
                .collect();
            out.push(mini);
        }
        out
    }

    /// A click or a drag in the minimap (`CEditView::OnMOUSEMOVE` with `m_bMiniMapMouseDown`):
    /// the caret goes to the start of `row`, or the selection grows to it with `extend` (Shift),
    /// and the view scrolls to show it.
    pub(crate) fn minimap_go_to_row(&mut self, row: usize, extend: bool, cx: &mut Context<Self>) {
        // `OnLBUTTONDOWN` ends the incremental search first.
        self.isearch_exit(cx);
        self.interrupt_composition(cx);
        let last = self.layout.row_count().saturating_sub(1);
        let target = {
            let doc = self.document.read(cx).doc();
            self.layout
                .rows(doc, row.min(last), 1)
                .ok()
                .and_then(|rows| rows.first().map(|r| r.start))
        };
        let Some(pos) = target else {
            return;
        };
        self.run_with(cx, |command_cx| command_cx.move_caret(pos, extend))
            .ok();
    }
}

/// The byte length of the first `max_chars` characters of `text`.
fn clip_len(text: &str, max_chars: usize) -> usize {
    text.char_indices()
        .nth(max_chars)
        .map_or(text.len(), |(at, _)| at)
}
