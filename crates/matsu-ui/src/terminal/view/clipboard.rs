//! Paste, selection and copy in the terminal (ADR-0028 points 4 and 9).
//!
//! * Paste writes the clipboard's text through `Terminal::encode_paste`: in brackets when the program asked
//!   for them, with the control characters taken out. Text of several lines that the program did not ask to
//!   have in brackets runs line by line as commands, so the user is asked first; after the answer the state is
//!   looked at again (the shell may have ended, or asked for brackets, meanwhile).
//! * The mouse selects: a drag selects cells, a double click a word and a triple click a line, as in Windows
//!   Terminal. Output that moves the lines moves the selection with them (`alacritty_terminal` does; see
//!   `matsu_terminal::SelectionSpan`).
//! * Copy writes the selected text to the clipboard with the line ends of the platform, and clears the
//!   selection.

use gpui::{
    ClipboardItem, Context, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Point, Window,
};
use matsu_terminal::{CellSide, SelectionMode};

use super::TerminalView;
use crate::file_ops::ask::confirm;

/// The text for the clipboard: the line ends of the platform (`crlf`: Windows), as an editor copies them.
pub(super) fn with_line_ends(text: &str, crlf: bool) -> String {
    if crlf {
        text.replace('\n', "\r\n")
    } else {
        text.to_owned()
    }
}

/// The question before several lines are pasted where a shell reads them (ADR-0028 point 4). The lines may
/// run as commands, one after the other, before the user sees what they do.
fn multi_line_message(lines: usize) -> String {
    format!(
        "{lines} 行のテキストを貼り付けます。シェルでは、貼り付けた行が改行ごとにコマンドとして実行されることがあります。貼り付けますか?"
    )
}

/// How many lines the bytes of a paste hold (they carry the line breaks as CR).
fn lines_of(bytes: &[u8]) -> usize {
    let breaks = bytes.iter().filter(|&&b| b == b'\r').count();
    breaks + 1 - usize::from(bytes.last() == Some(&b'\r'))
}

impl TerminalView {
    /// Whether anything is selected.
    pub(crate) fn has_selection(&self) -> bool {
        self.terminal.has_selection()
    }

    /// Clears the selection, if there is one.
    pub(super) fn clear_selection(&mut self) {
        if self.terminal.has_selection() {
            self.terminal.clear_selection();
        }
    }

    /// Copies the selection to the clipboard and clears it. `false` when nothing was selected.
    pub(super) fn copy_selection(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(text) = self.terminal.selected_text() else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(with_line_ends(
            &text,
            cfg!(windows),
        )));
        self.terminal.clear_selection();
        true
    }

    /// Pastes the text of the clipboard.
    pub(super) fn paste(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_running() {
            return;
        }
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        self.paste_text(text, window, cx);
    }

    /// Writes `text` as a paste, asking first when it would run as several commands.
    fn paste_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let encoding = self.terminal.encode_paste(&text);
        if encoding.bytes.is_empty() {
            return;
        }
        if !encoding.needs_confirmation {
            self.write_paste(&text);
            return;
        }
        // One question at a time: a second paste while it is open is dropped.
        if std::mem::replace(&mut self.paste_asking, true) {
            return;
        }
        let message = multi_line_message(lines_of(&encoding.bytes));
        cx.spawn_in(window, async move |this, cx| {
            let yes = confirm(cx, &message, None, ["はい(Y)", "いいえ(N)"]).await;
            this.update(cx, |this, _| {
                this.paste_asking = false;
                // The state is looked at again: the terminal may have ended while the question was open,
                // and the program may have asked for brackets, which changes what is written. Nothing is
                // shown for a terminal that ended.
                if yes && this.is_running() {
                    this.write_paste(&text);
                }
            })
            .ok();
        })
        .detach();
    }

    /// Writes a paste now, encoded for the terminal as it is.
    fn write_paste(&mut self, text: &str) {
        let encoding = self.terminal.encode_paste(text);
        if encoding.bytes.is_empty() {
            return;
        }
        self.clear_selection();
        self.send(&encoding.bytes);
    }

    /// The cell under a point of the window, and which half of it: a point outside the screen is pulled to
    /// the nearest cell (a drag goes on past the edge).
    fn cell_at(&self, position: Point<Pixels>) -> Option<(i32, usize, CellSide)> {
        let layout = self.layout?;
        let x = f32::from(position.x - layout.bounds.left()).max(0.0);
        let y = f32::from(position.y - layout.bounds.top()).max(0.0);
        let (columns, rows) = (usize::from(self.size.columns), usize::from(self.size.rows));
        let column = ((x / layout.cell.width) as usize).min(columns - 1);
        let row = ((y / layout.cell.height) as usize).min(rows - 1);
        let side = if x - column as f32 * layout.cell.width >= layout.cell.width / 2.0 {
            CellSide::Right
        } else {
            CellSide::Left
        };
        Some((i32::try_from(row).unwrap_or(0), column, side))
    }

    /// A point in a cell, in the window, `across` (0 to 1) of the way through the cell from its left edge
    /// (for the tests: below a half is the left half of the cell).
    #[cfg(test)]
    pub(crate) fn cell_point(
        &self,
        row: usize,
        column: usize,
        across: f32,
    ) -> Option<Point<Pixels>> {
        let layout = self.layout?;
        Some(Point {
            x: layout.bounds.left() + gpui::px((column as f32 + across) * layout.cell.width),
            y: layout.bounds.top() + gpui::px((row as f32 + 0.5) * layout.cell.height),
        })
    }

    /// The cell of the terminal's cursor on the view, `(row, column)` (for the tests).
    #[cfg(test)]
    pub(crate) fn cursor(&self) -> (usize, usize) {
        let info = self.terminal.info();
        let row = i64::from(info.cursor.row) + info.display_offset as i64;
        (usize::try_from(row).unwrap_or(0), info.cursor.column)
    }

    /// The selected text (for the tests).
    #[cfg(test)]
    pub(crate) fn selected_text(&self) -> Option<String> {
        self.terminal.selected_text()
    }

    /// The program asked for bracketed paste (for the tests).
    #[cfg(test)]
    pub(crate) fn bracketed_paste(&self) -> bool {
        self.terminal.modes().bracketed_paste
    }

    /// A question about a paste is open (for the tests).
    #[cfg(test)]
    pub(crate) fn asking_about_a_paste(&self) -> bool {
        self.paste_asking
    }

    pub(super) fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus, cx);
        if event.button != MouseButton::Left {
            return;
        }
        let Some((row, column, side)) = self.cell_at(event.position) else {
            return;
        };
        let mode = match event.click_count {
            0 | 1 => SelectionMode::Cells,
            2 => SelectionMode::Words,
            _ => SelectionMode::Lines,
        };
        self.terminal.start_selection(mode, row, column, side);
        self.dragging = true;
        cx.notify();
    }

    pub(super) fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.dragging {
            return;
        }
        if event.pressed_button != Some(MouseButton::Left) {
            // The button went up where the window did not see it.
            self.dragging = false;
            return;
        }
        if let Some((row, column, side)) = self.cell_at(event.position) {
            self.terminal.update_selection(row, column, side);
            cx.notify();
        }
    }

    pub(super) fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.dragging) {
            return;
        }
        // A click that did not drag selects nothing, and clears what was selected before.
        if !self.terminal.has_selection() {
            self.terminal.clear_selection();
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_copy_has_the_line_ends_of_the_platform() {
        assert_eq!(with_line_ends("a\nb\n\nc", true), "a\r\nb\r\n\r\nc");
        assert_eq!(with_line_ends("a\nb", false), "a\nb");
        assert_eq!(with_line_ends("", true), "");
    }

    #[test]
    fn the_lines_of_a_paste_are_counted_by_its_breaks() {
        assert_eq!(lines_of(b"a"), 1);
        assert_eq!(lines_of(b"a\rb"), 2);
        assert_eq!(lines_of(b"a\rb\r"), 2, "a break at the end ends the line");
        assert_eq!(lines_of(b"\r\r\r"), 3);
    }
}
