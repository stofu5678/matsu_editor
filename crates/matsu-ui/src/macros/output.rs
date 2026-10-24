//! The output panel of macros: what a script prints, what the editor says about a run (start,
//! end, errors), and the panel that shows it under the text.

use gpui::{
    AnyElement, App, ClickEvent, IntoElement, ParentElement, ScrollHandle, Styled, Window, div,
    prelude::*, px, rgb,
};

use std::path::PathBuf;
use std::rc::Rc;

use matsu_core::sql_error::{ErrorStyle, parse};
use matsu_core::tag_jump::TagTarget;

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
    /// What a click on this line does: the run that wrote it decides (a later run has its own).
    pub jump: Option<Rc<JumpContext>>,
}

/// Where a line of the output that names a place of an SQL file leads (ADR-0023: the output of a
/// database client). Set by the run of a database function; a line that `style` finds a place in
/// can be clicked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JumpContext {
    pub style: ErrorStyle,
    /// The file that was run (`None`: a document without a file, run through the standard input).
    pub file: Option<PathBuf>,
    /// The folder that file names in the output are relative to.
    pub folder: Option<PathBuf>,
    /// Added to a line that names no file: the lines the client counts start at this line of the
    /// document (a run of the selection).
    pub line_offset: usize,
    /// Added to the column of an error on the first line of what was sent (a selection that starts
    /// in the middle of a line), in UTF-16 code units.
    pub column_offset: usize,
}

/// Where a click on a line of the output goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    /// A line (and column) of the document that ran the client.
    Here {
        line: usize,
        column: Option<usize>,
    },
    File(TagTarget),
}

impl JumpContext {
    /// The place that the line `text` names.
    pub fn place(&self, text: &str) -> Option<Place> {
        let spot = parse(self.style, text)?;
        match spot.file {
            Some(file) => {
                let path = PathBuf::from(&file);
                let path = match (&self.folder, path.is_relative()) {
                    (Some(folder), true) => folder.join(path),
                    _ => path,
                };
                Some(Place::File(TagTarget {
                    path,
                    line: Some(spot.line),
                    column: spot.column,
                }))
            }
            None => {
                let line = spot.line + self.line_offset;
                // the first line of what was sent starts at a column of the document
                let column = match spot.column {
                    Some(column) if spot.line == 1 => Some(column + self.column_offset),
                    column => column,
                };
                Some(match &self.file {
                    Some(path) => Place::File(TagTarget {
                        path: path.clone(),
                        line: Some(line),
                        column,
                    }),
                    None => Place::Here { line, column },
                })
            }
        }
    }
}

#[derive(Default)]
pub struct Output {
    jump: Option<Rc<JumpContext>>,
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
            jump: None,
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
                    jump: self.jump.clone(),
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

    /// `trace` without a line break, for the output of a database client: the lines it starts lead
    /// somewhere when clicked, as `jump` says. Other output (a macro's `TraceOut`) is not affected.
    pub fn trace_jump(&mut self, text: &str, jump: Option<&JumpContext>) {
        self.jump = jump.cloned().map(Rc::new);
        self.trace(text, false);
        self.jump = None;
    }

    /// Starts a new run: the panel is emptied and shown.
    pub fn begin(&mut self) {
        self.jump = None;
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

    /// The lines that a click leads somewhere from, with the place.
    #[cfg(test)]
    pub fn places(&self) -> Vec<(String, Place)> {
        self.lines
            .iter()
            .filter_map(|l| Some((l.text.clone(), l.jump.as_ref()?.place(&l.text)?)))
            .collect()
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
/// What a click on a line that names a place does.
pub type JumpHandler = Rc<dyn Fn(Place, &mut Window, &mut App)>;

/// What the buttons of the panel do. Each takes the panel's owner (the view).
pub struct PanelActions {
    pub running: bool,
    pub on_cancel: ClickHandler,
    pub on_close: ClickHandler,
    /// A click on a line that names a place ([`JumpContext`]).
    pub on_jump: JumpHandler,
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
            let place = line.jump.as_ref().and_then(|jump| jump.place(&line.text));
            let row = crate::a11y::text(div().id(("macro-line", i)), &line.text)
                .text_color(color)
                .child(text);
            match place {
                Some(place) => {
                    let on_jump = actions.on_jump.clone();
                    row.cursor_pointer()
                        .text_color(rgb(0x0b5cad))
                        .on_click(move |_, window, cx| on_jump(place.clone(), window, cx))
                        .into_any_element()
                }
                None => row.into_any_element(),
            }
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

    fn context(offset: usize, column_offset: usize) -> JumpContext {
        JumpContext {
            style: ErrorStyle::BigQuery,
            file: Some(PathBuf::from("/w/a.sql")),
            folder: Some(PathBuf::from("/w")),
            line_offset: offset,
            column_offset,
        }
    }

    fn line_of(place: &Place) -> Option<usize> {
        match place {
            Place::File(target) => target.line,
            Place::Here { line, .. } => Some(*line),
        }
    }

    #[test]
    fn each_run_keeps_its_own_jump_context() {
        let mut output = Output::default();
        output.trace_jump("error at [1:1]\n", Some(&context(0, 0)));
        output.trace_jump("error at [1:1]\n", Some(&context(5, 0)));
        output.trace_jump("error at [1:1]\n", None);
        output.trace("error at [1:1]", true);
        let places = output.places();
        // the last two lead nowhere (no context; a macro's own output); the first two keep what they had
        assert_eq!(places.len(), 2);
        assert_eq!(line_of(&places[0].1), Some(1));
        assert_eq!(line_of(&places[1].1), Some(6));
    }

    #[test]
    fn only_the_first_line_gets_the_column_of_the_selection() {
        let jump = context(2, 10);
        let column = |text: &str| match jump.place(text) {
            Some(Place::File(target)) => target.column,
            other => panic!("{other:?}"),
        };
        assert_eq!(column("x at [1:5]"), Some(15));
        assert_eq!(column("x at [2:5]"), Some(5));
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
