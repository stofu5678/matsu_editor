//! Syntax coloring of MATSU Editor: the color strategies of Sakura Editor (comments, strings,
//! numbers, keywords, regular expression keywords, URLs, here-documents) and the color state at the start of each line.
//!
//! It knows nothing of a window: it takes the text of a line and gives the runs of colors, so it
//! is tested without one and the printing can use it too. The window (`matsu-ui`) paints the runs.
//!
//! * [`ColorRules`] are the rules of one document type: made from its settings, once.
//! * [`line_runs`] colors a line that starts in a given [`ColorState`] (the painting pass of
//!   Sakura, `CEditView::DrawLogicLine`); [`line_end_state`] gives the state the next line starts
//!   in (the layout pass, `CLayoutMgr::_MakeOneLine`).
//! * [`ColorStates`] keeps the state at the start of every line of a document, made in the
//!   background and updated after an edit until the states agree with the old ones again.
//!
//! # Where this differs from Sakura
//!
//! * Which characters are "full-width" for the kind of a character follows East Asian Width, not
//!   the font (`IsHankaku` asks the font in Sakura).
//! * Positions are bytes of UTF-8, Sakura's are UTF-16 units; the columns of line comments that
//!   must start in a column are counted in UTF-16 units like Sakura.
//! * Sakura has the states of all lines before the first paint. Here the states of a very large
//!   document come in steps, and the colors of the lines far from the top can change once
//!   ([`ColorStates::state_at`]).

mod charkind;
mod keyword;
mod kind;
mod number;
mod quote;
mod regex_keyword;
mod rules;
mod scan;
mod states;
mod url;

pub use charkind::{CharKind, kind_at, next_word_break};
pub use kind::{ColorKind, ColorRun, KEYWORD_SLOTS};
pub use number::number_len;
pub use regex_keyword::has_keyword_form;
pub use rules::ColorRules;
pub use scan::{ColorState, QuoteKind};
pub use states::{ColorStates, LOCAL_WINDOW, LineEdit, line_end_state};
pub use url::{mail_len, url_len};

use matsu_core::buffer::Eol;

/// The runs of the line `text`, which ends with `eol` (if it does), from the start up to the byte
/// offset `upto` of `text`, when the line starts in `state`. The runs
/// cover `0..upto`, and when `upto` is the length of the text they go on through the line ending,
/// so that the last run says what color the line ending is in.
pub fn line_runs(
    rules: &ColorRules,
    text: &str,
    eol: Option<Eol>,
    state: &ColorState,
    upto: usize,
) -> Vec<ColorRun> {
    match eol {
        Some(eol) => {
            let mut full = String::with_capacity(text.len() + 3);
            full.push_str(text);
            full.push_str(eol.as_str());
            scan::color_runs(rules, &full, text.len(), state, upto)
        }
        None => scan::color_runs(rules, text, text.len(), state, upto),
    }
}
