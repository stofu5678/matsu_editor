//! The colored HTML copy (選択範囲内色付きHTMLコピー, `F_COPY_COLOR_HTML` and
//! `F_COPY_COLOR_HTML_LINENUMBER`): the selected text as `<pre>` with the colors and the font
//! styles of the document type.
//!
//! * [`builder`] writes the tags (Sakura's `AppendHTMLColor`);
//! * [`collect`] works out the rows, the part of each that is selected and the style of each
//!   character, and the line numbers.
//!
//! The commands (`crate::ui_commands::clipboard`) put the result on the clipboard: the HTML
//! source as the text, and the HTML itself in the format the OS has ([`ume_platform::clipboard_html`]).
//! Sakura does the same: `cClipboard.SetHtmlText` and then `SetText` with the same string.

mod builder;
mod collect;

use gpui::Context;
use ume_commands::clipboard::{ClipData, ClipKind, Clipboard as _};

use crate::clipboard::GpuiClipboard;
use crate::view::EditorView;

pub(crate) use collect::{Source, color_html};

impl EditorView {
    /// `Command_COPY_COLOR_HTML`: copies the selection as HTML with its colors. Nothing happens
    /// without a selection, as in Sakura (no beep). The clipboard gets the HTML source as text,
    /// and the HTML in the format of the OS where it has one.
    pub(crate) fn copy_color_html(&mut self, line_numbers: bool, cx: &mut Context<Self>) {
        let shared = self.document.read(cx);
        let doc = shared.doc();
        // The rows of a large document are counted, not laid out, until they are asked for.
        let _ = self.layout.layout_all(doc);
        let Some(html) = color_html(&Source {
            doc,
            layout: &self.layout,
            rules: &self.prefs.settings.coloring,
            states: &self.color_states,
            cursor: &self.cursor,
            line_num_is_crlf: self.prefs.settings.outline.line_num_is_crlf,
            line_numbers,
        }) else {
            return;
        };
        if GpuiClipboard(cx).set(ClipData::new(html.clone(), ClipKind::Normal)) {
            // Not on every OS; the text is on the clipboard in any case.
            ume_platform::clipboard_html::add_html(&html);
        }
    }
}

#[cfg(test)]
mod tests;
