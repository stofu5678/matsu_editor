//! A plain text model that implements the macro API without a window: for the unit tests, the
//! integration tests and the command line mode (`--run-macro`).

use super::api::MacroDocument;

pub struct TextModel {
    /// Split at "\n", so a text that ends with a line break ends with an empty line (as the
    /// editor does).
    lines: Vec<String>,
    cursor: (usize, usize),
    anchor: Option<(usize, usize)>,
    /// The column (in characters) that vertical moves keep, as the editor keeps its x position.
    goal_column: Option<usize>,
    path: Option<String>,
}

impl TextModel {
    pub fn new(text: &str, path: Option<String>) -> Self {
        Self {
            lines: split_lines(text),
            cursor: (0, 0),
            anchor: None,
            goal_column: None,
            path,
        }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    #[cfg(test)]
    pub fn cursor(&self) -> (usize, usize) {
        self.cursor
    }

    fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let anchor = self.anchor?;
        match anchor.cmp(&self.cursor) {
            std::cmp::Ordering::Less => Some((anchor, self.cursor)),
            std::cmp::Ordering::Greater => Some((self.cursor, anchor)),
            std::cmp::Ordering::Equal => None,
        }
    }

    fn text_in(&self, from: (usize, usize), to: (usize, usize)) -> String {
        if from.0 == to.0 {
            return self.lines[from.0][from.1..to.1].to_string();
        }
        let mut text = self.lines[from.0][from.1..].to_string();
        for line in &self.lines[from.0 + 1..to.0] {
            text.push('\n');
            text.push_str(line);
        }
        text.push('\n');
        text.push_str(&self.lines[to.0][..to.1]);
        text
    }

    fn delete_selection(&mut self) {
        let selection = self.selection();
        self.anchor = None;
        if let Some((from, to)) = selection {
            let tail = self.lines[to.0][to.1..].to_string();
            self.lines[from.0].truncate(from.1);
            self.lines[from.0].push_str(&tail);
            self.lines.drain(from.0 + 1..=to.0);
            self.cursor = from;
        }
    }
}

/// Splits as the editor splits the text it opens: at "\n", with a CR at the end of a line
/// dropped.
pub fn split_lines(text: &str) -> Vec<String> {
    text.split('\n')
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect()
}

impl MacroDocument for TextModel {
    fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn line_len(&self, row: usize) -> usize {
        self.lines[row].len()
    }

    fn line_text(&self, row: usize) -> String {
        self.lines[row].clone()
    }

    fn cursor(&self) -> (usize, usize) {
        self.cursor
    }

    fn has_selection(&self) -> bool {
        self.selection().is_some()
    }

    fn selected_text(&self) -> String {
        self.selection()
            .map(|(from, to)| self.text_in(from, to))
            .unwrap_or_default()
    }

    fn move_cursor(&mut self, pos: (usize, usize), extend: bool) {
        self.goal_column = None;
        if !extend {
            self.anchor = None;
        } else if self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        }
        self.cursor = pos;
    }

    fn move_vertical(&mut self, delta: isize) {
        self.anchor = None;
        let row = (self.cursor.0 as isize + delta).clamp(0, self.lines.len() as isize - 1) as usize;
        let column = *self
            .goal_column
            .get_or_insert_with(|| self.lines[self.cursor.0][..self.cursor.1].chars().count());
        let line = &self.lines[row];
        let byte = line
            .char_indices()
            .nth(column)
            .map_or(line.len(), |(offset, _)| offset);
        self.cursor = (row, byte);
    }

    fn select_all(&mut self) {
        self.goal_column = None;
        self.anchor = Some((0, 0));
        let last = self.lines.len() - 1;
        self.cursor = (last, self.lines[last].len());
    }

    fn insert(&mut self, text: &str) {
        self.goal_column = None;
        self.delete_selection();
        let (row, col) = self.cursor;
        let mut parts = text.split('\n').map(|p| p.trim_end_matches('\r'));
        let first = parts.next().unwrap_or_default();
        let rest: Vec<&str> = parts.collect();
        if rest.is_empty() {
            self.lines[row].insert_str(col, first);
            self.cursor = (row, col + first.len());
        } else {
            let tail = self.lines[row].split_off(col);
            self.lines[row].push_str(first);
            let last_row = row + rest.len();
            let last_len = rest.last().map_or(0, |l| l.len());
            for (i, line) in rest.into_iter().enumerate() {
                self.lines.insert(row + 1 + i, line.to_string());
            }
            self.lines[last_row].push_str(&tail);
            self.cursor = (last_row, last_len);
        }
    }

    fn path(&self) -> Option<String> {
        self.path.clone()
    }
}
