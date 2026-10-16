//! The OS clipboard behind `ume_commands::clipboard::Clipboard`, through GPUI.
//!
//! The text goes to the OS clipboard as plain text, so other programs see it. Sakura's marks
//! (a copy of a row, a copy of a box) go to GPUI's metadata of the clipboard text. GPUI keeps
//! the metadata only for the program that wrote it (on Windows and macOS it is dropped if the
//! text was changed by another program), so a row copied in UME Editor is pasted as a row in UME
//! Editor but is plain text elsewhere, and Visual Studio's `MSDEVLineSelect` is not read yet.

use gpui::{App, ClipboardItem, Global};
use ume_commands::clipboard::{ClipData, ClipKind, Clipboard};

/// The metadata of a row copy.
const LINE: &str = "ume:line";
/// The metadata of a box copy.
const BOX: &str = "ume:box";

/// The quote string that `F_SET_QUOTESTRING` set, for every window of the session (Sakura keeps
/// it in the shared common settings, `m_szInyouKigou`). Without it commands use Sakura's default.
pub(crate) struct QuoteString(pub(crate) String);

impl Global for QuoteString {}

/// The clipboard of the application.
pub(crate) struct GpuiClipboard<'a>(pub(crate) &'a mut App);

impl Clipboard for GpuiClipboard<'_> {
    fn get(&mut self) -> Option<ClipData> {
        let item = self.0.read_from_clipboard()?;
        // `clear` leaves an empty text (GPUI cannot empty the clipboard), which is no text here.
        let text = item.text().filter(|text| !text.is_empty())?;
        let kind = match item.metadata().map(String::as_str) {
            Some(LINE) => ClipKind::Line,
            Some(BOX) => ClipKind::Box,
            _ => ClipKind::Normal,
        };
        Some(ClipData { text, kind })
    }

    fn set(&mut self, data: ClipData) -> bool {
        let item = match data.kind {
            ClipKind::Normal => ClipboardItem::new_string(data.text),
            ClipKind::Line => ClipboardItem::new_string_with_metadata(data.text, LINE.to_owned()),
            ClipKind::Box => ClipboardItem::new_string_with_metadata(data.text, BOX.to_owned()),
        };
        self.0.write_to_clipboard(item);
        true
    }

    /// GPUI has no call that empties the clipboard, so this writes an empty text, which `get`
    /// reports as no text. The clipboard is not empty for other programs.
    fn clear(&mut self) {
        self.0
            .write_to_clipboard(ClipboardItem::new_string(String::new()));
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext, VisualTestContext};
    use ume_core::buffer::Document;
    use ume_core::model::DocumentModel;

    use crate::document::SharedDocument;
    use crate::view::EditorView;

    /// A window with an editor view on `text`. Returns the view and the document.
    fn setup<'a>(
        cx: &'a mut TestAppContext,
        text: &str,
    ) -> (
        gpui::Entity<EditorView>,
        gpui::Entity<SharedDocument>,
        &'a mut VisualTestContext,
    ) {
        let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
        let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
        let shared = document.clone();
        let (view, vcx) = cx.add_window_view(move |window, cx| {
            let view = EditorView::new(shared, cx);
            window.focus(&view.focus_handle, cx);
            view
        });
        (view, document, vcx)
    }

    fn text_of(document: &gpui::Entity<SharedDocument>, cx: &mut VisualTestContext) -> String {
        document.read_with(cx, |shared, _| {
            let doc = shared.doc();
            doc.text_between(ume_core::buffer::Pos::new(0, 0), doc.end_pos().unwrap())
                .unwrap()
        })
    }

    // Sakura's Ctrl+C, Ctrl+X and Ctrl+V; Ctrl is Ctrl on every OS (ADR-0008).

    #[gpui::test]
    fn copy_cut_and_paste_go_through_the_clipboard_of_gpui(cx: &mut TestAppContext) {
        let (_view, document, cx) = setup(cx, "abc\r\ndef");
        cx.simulate_keystrokes("shift-right shift-right ctrl-c");
        assert_eq!(
            cx.read_from_clipboard().and_then(|i| i.text()).as_deref(),
            Some("ab")
        );
        cx.simulate_keystrokes("ctrl-end ctrl-v");
        assert_eq!(text_of(&document, cx), "abc\r\ndefab");
        // cut: the selection goes to the clipboard and out of the document
        cx.simulate_keystrokes("ctrl-home shift-right ctrl-x");
        assert_eq!(
            cx.read_from_clipboard().and_then(|i| i.text()).as_deref(),
            Some("a")
        );
        assert_eq!(text_of(&document, cx), "bc\r\ndefab");
    }

    #[gpui::test]
    fn a_row_copy_is_marked_and_pasted_at_the_start_of_the_row(cx: &mut TestAppContext) {
        let (_view, document, cx) = setup(cx, "abc\r\ndef");
        // no selection: the row is copied as a row
        cx.simulate_keystrokes("right ctrl-c");
        let item = cx.read_from_clipboard().expect("something was copied");
        assert_eq!(item.text().as_deref(), Some("abc\r\n"));
        assert_eq!(item.metadata().map(String::as_str), Some(super::LINE));
        cx.simulate_keystrokes("down ctrl-v");
        assert_eq!(text_of(&document, cx), "abc\r\nabc\r\ndef");
    }

    #[gpui::test]
    fn text_from_another_program_is_pasted_at_the_caret(cx: &mut TestAppContext) {
        let (_view, document, cx) = setup(cx, "ab");
        cx.write_to_clipboard(gpui::ClipboardItem::new_string("XY".to_owned()));
        cx.simulate_keystrokes("right shift-insert");
        assert_eq!(text_of(&document, cx), "aXYb");
    }

    #[gpui::test]
    fn shift_delete_and_ctrl_insert_cut_and_copy(cx: &mut TestAppContext) {
        let (_view, document, cx) = setup(cx, "abcd");
        cx.simulate_keystrokes("shift-right shift-right ctrl-insert");
        assert_eq!(
            cx.read_from_clipboard().and_then(|i| i.text()).as_deref(),
            Some("ab")
        );
        cx.simulate_keystrokes("shift-delete");
        assert_eq!(text_of(&document, cx), "cd");
    }
}
