//! The colored HTML copy (選択範囲内色付きHTMLコピー, `F_COPY_COLOR_HTML` and
//! `F_COPY_COLOR_HTML_LINENUMBER`): the selected text as `<pre>` with the colors and the font
//! styles of the document type.
//!
//! * [`builder`] writes the tags (Sakura's `AppendHTMLColor`);
//! * [`collect`] works out the rows, the part of each that is selected and the style of each
//!   character, and the line numbers.
//!
//! The commands (`crate::ui_commands::clipboard`) put the result on the clipboard: the HTML
//! source as the text, and the HTML itself in the format the OS has ([`matsu_platform::clipboard_html`]).
//! Sakura does the same: `cClipboard.SetHtmlText` and then `SetText` with the same string.

mod builder;
mod collect;

use gpui::Context;
use matsu_commands::clipboard::{
    ClipData, ClipKind, Clipboard as _, copy_limit, mib, too_big_message,
};
use matsu_core::buffer::DEFAULT_LONG_LINE;

use crate::clipboard::GpuiClipboard;
use crate::view::EditorView;

pub(crate) use collect::{Found, OutputFull, Source, color_html_within, line_over};

/// The bytes of a line from which the colored HTML copy is refused: a very long line of the core
/// (1 MiB), see [`line_over`].
pub(crate) const LINE_LIMIT: usize = DEFAULT_LONG_LINE;

/// What the editor says when a line of the selection is too long for the colored HTML copy (Sakura
/// has no such limit, ADR-0018 type F).
pub(crate) fn long_line_message(bytes: usize) -> String {
    format!(
        "選択範囲に長すぎる行（{} MB。上限は {} MB）があるので、色付きHTMLをコピーできません。範囲を小さくしてやり直してください。",
        mib(bytes),
        mib(LINE_LIMIT)
    )
}

/// Lays out the lines up to `last_line`, and no more (the copy looks at the rows of the selection
/// only, see `collect`), so that the row numbers up to there are exact. The rows of a large
/// document are only counted until the background layout reaches them; the rest of the
/// document stays for the background.
fn lay_out_through(
    layout: &mut matsu_layout::DocumentLayout,
    doc: &matsu_core::buffer::Document,
    last_line: usize,
) {
    let wanted = last_line.saturating_add(1).min(layout.line_count());
    let todo = wanted.saturating_sub(layout.progress().done);
    if todo > 0 {
        let _ = layout.layout_chunk(doc, todo);
    }
}

impl EditorView {
    /// `Command_COPY_COLOR_HTML`: copies the selection as HTML with its colors. Nothing happens
    /// without a selection, as in Sakura (no beep). The clipboard gets the HTML source as text,
    /// and the HTML in the format of the OS where it has one.
    pub(crate) fn copy_color_html(&mut self, line_numbers: bool, cx: &mut Context<Self>) {
        let Some((from, to)) = self.cursor.selection() else {
            return;
        };
        // The same limit as the copy of the text (ADR-0020), asked before anything is read, and
        // box-aware. A size that cannot be told is a refusal, as for the copy of the text.
        let limit = copy_limit();
        let bytes = self
            .run_with(cx, |c| c.selection_bytes(limit))
            .unwrap_or(usize::MAX);
        if bytes >= limit {
            self.queue_message_box(too_big_message(bytes, false), cx);
            return;
        }
        // Before anything is laid out or colored: a long line is not taken in parts.
        if let Some(bytes) = line_over(self.document.read(cx).doc(), &self.cursor, LINE_LIMIT) {
            self.queue_message_box(long_line_message(bytes), cx);
            return;
        }
        let shared = self.document.read(cx);
        let doc = shared.doc();
        let rules = &self.prefs.settings.coloring;
        lay_out_through(&mut self.layout, doc, to.line);
        // The colors of a large document are made in the background; the state at the start of
        // the selection must be exact, or the copy would color a comment as code.
        // It goes on only as far as the selection: the lines after it are not needed.
        while !self.color_states.state_at(doc, rules, from.line).1
            && !self
                .color_states
                .advance(doc, rules, std::time::Duration::from_millis(200))
        {}
        let search = &self.search;
        let found_colors = self.prefs.settings.colors.found;
        let found = search
            .matcher()
            .filter(|_| search.mark() && !found_colors.is_empty())
            .map(|matcher| Found {
                matcher: &**matcher,
                colors: found_colors,
                words: search
                    .plain()
                    .filter(|_| search.options().whole_word)
                    .map(|plain| &**plain),
            });
        let html = color_html_within(
            &Source {
                doc,
                layout: &self.layout,
                rules: &self.prefs.settings.coloring,
                states: &self.color_states,
                cursor: &self.cursor,
                line_num_is_crlf: self.prefs.settings.outline.line_num_is_crlf,
                line_numbers,
                found,
            },
            limit,
        );
        // The HTML is several times the text it colors: it is not built past the limit either.
        let html = match html {
            Ok(Some(html)) => html,
            Ok(None) => return,
            Err(OutputFull(written)) => {
                self.queue_message_box(too_big_message(written, false), cx);
                return;
            }
        };
        if GpuiClipboard(cx).set(ClipData::new(html.clone(), ClipKind::Normal)) {
            // Not on every OS; the text is on the clipboard in any case.
            matsu_platform::clipboard_html::add_html(&html);
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod layout_tests {
    use matsu_core::buffer::Document;
    use matsu_layout::{DocumentLayout, LayoutConfig};

    use super::lay_out_through;

    #[test]
    fn a_copy_near_the_top_does_not_lay_out_the_rest_of_a_long_document() {
        let text: String = (0..5000).map(|i| format!("line {i}\r\n")).collect();
        let doc = Document::from_utf8(text.into_bytes()).unwrap();
        let mut layout = DocumentLayout::new(&doc, LayoutConfig::default()).unwrap();
        assert_eq!(layout.progress().done, 0);
        lay_out_through(&mut layout, &doc, 3);
        let progress = layout.progress();
        // through the last line of the selection, and no further
        assert_eq!(progress.done, 4);
        assert!(!progress.is_complete());
        // what is laid out already is not done again, and a later line is laid out as far as it is
        lay_out_through(&mut layout, &doc, 1);
        assert_eq!(layout.progress().done, 4);
        lay_out_through(&mut layout, &doc, 99);
        assert_eq!(layout.progress().done, 100);
    }
}
