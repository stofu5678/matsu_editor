//! The output panel of macros: what a script prints, what the editor says about a run (start,
//! end, errors), and the panel that shows it under the text.

use gpui::{
    AnyElement, App, ClickEvent, IntoElement, ParentElement, ScrollHandle, Styled, Window, div,
    prelude::*, px, rgb,
};

use crate::view::FONT_FAMILY;

/// The most lines that the panel keeps (older ones go).
const MAX_LINES: usize = 5000;
/// The most lines that the panel draws (the last ones).
const SHOWN_LINES: usize = 300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Standard output of a script.
    Out,
    /// Standard error of a script (a traceback with the line number, for one).
    Err,
    /// What the editor says: start, end, errors of its own.
    Note,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub kind: Kind,
    pub text: String,
}

#[derive(Default)]
pub struct Output {
    lines: Vec<Line>,
    /// The panel is shown.
    open: bool,
    /// The last line was left without a line break (`TraceOut` with option 2): the next trace
    /// goes on in it.
    open_line: bool,
    scroll: ScrollHandle,
}

impl Output {
    pub fn push(&mut self, kind: Kind, text: impl Into<String>) {
        self.open_line = false;
        self.lines.push(Line {
            kind,
            text: text.into(),
        });
        if self.lines.len() > MAX_LINES {
            self.lines.drain(..self.lines.len() - MAX_LINES);
        }
        self.scroll.scroll_to_bottom();
    }

    /// `TraceOut`: appends `text` to the output and shows the panel. With `line_break` the
    /// text ends a line (Sakura appends CR LF); without it the next trace goes on in the same
    /// line. A line break in `text` (LF, CR LF) starts a new line.
    pub fn trace(&mut self, text: &str, line_break: bool) {
        self.open = true;
        let mut text = text.replace("\r\n", "\n");
        if line_break {
            text.push('\n');
        }
        let pieces: Vec<&str> = text.split('\n').collect();
        for (i, piece) in pieces.iter().enumerate() {
            let last = i + 1 == pieces.len();
            match self.lines.last_mut() {
                Some(line) if i == 0 && self.open_line => line.text.push_str(piece),
                // what follows the last line break is nothing yet
                _ if last && piece.is_empty() => {}
                _ => self.lines.push(Line {
                    kind: Kind::Out,
                    text: (*piece).to_owned(),
                }),
            }
            if !last {
                self.open_line = false;
            } else if !piece.is_empty() {
                self.open_line = true;
            }
        }
        if self.lines.len() > MAX_LINES {
            self.lines.drain(..self.lines.len() - MAX_LINES);
        }
        self.scroll.scroll_to_bottom();
    }

    /// Starts a new run: the panel is emptied and shown.
    pub fn begin(&mut self) {
        self.lines.clear();
        self.open_line = false;
        self.open = true;
    }

    /// Shows the panel (an error that the user has to see).
    pub fn show(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    #[cfg(test)]
    pub fn is_open(&self) -> bool {
        self.open
    }

    #[cfg(test)]
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// Everything in the panel, one line each.
    pub fn text(&self) -> String {
        let lines: Vec<&str> = self.lines.iter().map(|l| l.text.as_str()).collect();
        lines.join("\n")
    }
}

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// What the buttons of the panel do. Each takes the panel's owner (the view).
pub struct PanelActions {
    pub running: bool,
    pub on_cancel: ClickHandler,
    pub on_close: ClickHandler,
}

impl Output {
    /// The panel, while it is open.
    pub fn panel(&self, actions: PanelActions) -> Option<AnyElement> {
        if !self.open {
            return None;
        }
        let skip = self.lines.len().saturating_sub(SHOWN_LINES);
        let lines = self.lines[skip..].iter().enumerate().map(|(i, line)| {
            let color = match line.kind {
                Kind::Out => rgb(0x1e1e1e),
                Kind::Err => rgb(0xb00020),
                Kind::Note => rgb(0x0b5cad),
            };
            // An empty line still needs its height.
            let text = if line.text.is_empty() {
                "\u{a0}".to_string()
            } else {
                line.text.clone()
            };
            crate::a11y::text(div().id(("macro-line", i)), &line.text)
                .text_color(color)
                .child(text)
        });
        let title = if actions.running {
            "マクロの出力 (実行中)"
        } else {
            "マクロの出力"
        };
        let mut buttons = div().flex().flex_row().gap_3();
        if actions.running {
            buttons = buttons.child(
                crate::a11y::button(div().id("macro-panel-cancel"), "中断")
                    .cursor_pointer()
                    .text_color(rgb(0xb00020))
                    .child("中断")
                    .on_click(actions.on_cancel),
            );
        }
        buttons = buttons.child(
            crate::a11y::button(div().id("macro-panel-close"), "閉じる")
                .cursor_pointer()
                .child("閉じる")
                .on_click(actions.on_close),
        );
        Some(
            crate::a11y::group(div().id("macro-panel"), "マクロの出力")
                .flex()
                .flex_col()
                .h(px(170.))
                .border_t_1()
                .border_color(rgb(0xd0d0d0))
                .bg(rgb(0xf7f7f7))
                .text_size(px(12.))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .justify_between()
                        .px_2()
                        .py_0p5()
                        .bg(rgb(0xe8e8e8))
                        .child(title)
                        .child(buttons),
                )
                .child(
                    div()
                        .id("macro-output")
                        .role(gpui::Role::Log)
                        .aria_label("出力")
                        .flex_1()
                        .overflow_y_scroll()
                        .track_scroll(&self.scroll)
                        .px_2()
                        .font_family(FONT_FAMILY)
                        .children(lines),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_last_lines() {
        let mut output = Output::default();
        for i in 0..MAX_LINES + 10 {
            output.push(Kind::Out, i.to_string());
        }
        assert_eq!(output.lines().len(), MAX_LINES);
        assert_eq!(output.lines()[0].text, "10");
    }

    #[test]
    fn trace_ends_the_line_unless_told_not_to() {
        let mut output = Output::default();
        output.trace("a", true);
        output.trace("b", false);
        output.trace("c", false);
        output.trace("d", true);
        output.trace("e", true);
        assert_eq!(output.text(), "a\nbcd\ne");
        assert!(output.is_open());
    }

    #[test]
    fn trace_splits_the_text_at_line_breaks() {
        let mut output = Output::default();
        output.trace("1\r\n2\n3", false);
        output.trace("4\n", false);
        output.trace("", true);
        output.trace("5", true);
        assert_eq!(output.text(), "1\n2\n34\n\n5");
    }

    #[test]
    fn trace_of_nothing_without_a_break_changes_nothing() {
        let mut output = Output::default();
        output.trace("", false);
        assert_eq!(output.text(), "");
        output.trace("a", false);
        output.trace("", false);
        output.trace("b", true);
        assert_eq!(output.text(), "ab");
    }

    #[test]
    fn another_line_ends_the_open_one() {
        let mut output = Output::default();
        output.trace("open", false);
        output.push(Kind::Note, "note");
        output.trace("after", true);
        assert_eq!(output.text(), "open\nnote\nafter");
    }

    #[test]
    fn begin_empties_and_shows_the_panel() {
        let mut output = Output::default();
        output.push(Kind::Note, "old");
        assert!(!output.is_open());
        output.begin();
        assert!(output.is_open() && output.lines().is_empty());
        output.push(Kind::Err, "a");
        output.push(Kind::Out, "b");
        assert_eq!(output.text(), "a\nb");
        output.close();
        assert!(!output.is_open());
    }
}
