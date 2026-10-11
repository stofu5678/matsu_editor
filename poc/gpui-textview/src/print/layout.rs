//! Page layout for printing (ADR-0011). Puts the text on pages as Sakura does, and gives each
//! page as text with its place on the paper, so that the print preview (GPUI) and the printing
//! of each OS draw the same pages. It uses neither GPUI nor the OS; a real app keeps it in a
//! crate of its own (`ume-print`, ADR-0013).
//!
//! Lengths are in 0.1 mm, as in Sakura's print settings. The characters go on a grid of cells:
//! a half-width character takes one cell and a full-width one two, whatever their widths in the
//! font are, as Sakura gives ExtTextOut the width of each character.
//!
//! Sakura: sakura_core/print/CPrint.cpp (the settings and the sizes),
//! sakura_core/print/CPrintPreview.cpp (OnChangePrintSetting, DrawPageText, DrawHeaderFooter),
//! sakura_core/doc/layout/CLayoutMgr_DoLayout.cpp (wrapping) and
//! sakura_core/env/CSakuraEnvironment.cpp (ExpandParameter, for the header and the footer).

use std::fmt;
use std::ops::Range;

use unicode_width::UnicodeWidthChar;

/// Sakura's print settings (PRINTSETTING in sakura_core/print/CPrint.h), without the printer,
/// the paper, color printing, kinsoku, and the fonts of the header and the footer.
#[derive(Clone, Debug, PartialEq)]
pub struct PrintSetting {
    /// The font for half-width characters (m_szPrintFontFaceHan).
    pub font_han: String,
    /// The font for the other characters (m_szPrintFontFaceZen).
    pub font_zen: String,
    /// The width of a half-width character, which is the width of a cell (m_nPrintFontWidth).
    pub font_width: i32,
    /// The height of the characters (m_nPrintFontHeight).
    pub font_height: i32,
    /// Columns on a page, 段組 (m_nPrintDansuu).
    pub columns: i32,
    /// The space between columns (m_nPrintDanSpace).
    pub column_space: i32,
    /// The space between lines, in percent of the font height (m_nPrintLineSpacing).
    pub line_spacing: i32,
    pub margin_top: i32,
    pub margin_bottom: i32,
    pub margin_left: i32,
    pub margin_right: i32,
    /// Breaks a line before an English word rather than in it (m_bPrintWordWrap).
    pub word_wrap: bool,
    /// Prints the line numbers (m_bPrintLineNumber).
    pub line_number: bool,
    /// The left, center and right parts of the header, with `$f` and the like
    /// (m_szHeaderForm). An empty part is not printed.
    pub header: [String; 3],
    /// The same for the footer (m_szFooterForm).
    pub footer: [String; 3],
    /// Tab stops every this many cells. Sakura takes it from the document type (m_nTabSpace).
    pub tab_width: i32,
}

impl Default for PrintSetting {
    /// Sakura's defaults (CPrint::SettingInitialize), and the tab width of its document types
    /// (sakura_core/types/CType.cpp).
    fn default() -> Self {
        Self {
            font_han: "ＭＳ 明朝".into(),
            font_zen: "ＭＳ 明朝".into(),
            font_width: 12,
            font_height: 24,
            columns: 1,
            column_space: 70,
            line_spacing: 30,
            margin_top: 100,
            margin_bottom: 200,
            margin_left: 200,
            margin_right: 100,
            word_wrap: true,
            line_number: false,
            header: ["$f".into(), String::new(), String::new()],
            footer: [String::new(), "- $p -".into(), String::new()],
            tab_width: 4,
        }
    }
}

/// The size of the paper as it is turned, in 0.1 mm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paper {
    pub width: i32,
    pub height: i32,
}

impl Paper {
    /// Sakura's default paper, which it also takes when it cannot ask the printer.
    pub const A4: Paper = Paper {
        width: 2100,
        height: 2970,
    };
}

/// What the header and the footer may show of the document.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DocInfo {
    /// The file name with its extension, for `$f`.
    pub file_name: Option<String>,
    /// The full path, for `$F`.
    pub path: Option<String>,
}

/// The margins and the font leave no room for the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoPrintableArea;

impl fmt::Display for NoPrintableArea {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Sakura's STR_ERR_DLGPRNPRVW3_1.
        f.write_str("印刷ページ設定エラー:印字可能領域がありません")
    }
}

/// A page as text with its place on the paper.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Page {
    pub texts: Vec<PlacedText>,
    pub rules: Vec<Rule>,
}

/// Text on the grid. Its characters go to the right from `x`, each taking `char_cells` cells
/// of the setting's `font_width`. It holds no tabs.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedText {
    /// The left edge of the first cell, from the left edge of the paper.
    pub x: i32,
    /// The baseline, from the top edge of the paper.
    pub baseline: i32,
    pub text: String,
}

impl PlacedText {
    /// Each character's byte offset and the left edge of its cells.
    pub fn positions(&self, cell: i32) -> impl Iterator<Item = (usize, char, i32)> + '_ {
        let mut x = self.x;
        self.text.char_indices().map(move |(i, ch)| {
            let at = x;
            x += char_cells(ch) * cell;
            (i, ch, at)
        })
    }
}

/// A straight line on the page, from the top edge of the paper and its left edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rule {
    pub from: (i32, i32),
    pub to: (i32, i32),
}

/// The text split into printed lines and pages, for one setting and one paper.
#[derive(Clone, Debug)]
pub struct Layout {
    setting: PrintSetting,
    paper: Paper,
    /// Cells for the text in a line (m_bPreview_EnableColumns).
    cells: i32,
    /// Lines in a column (m_bPreview_EnableLines).
    rows: i32,
    /// Cells for the line numbers (m_nPreview_LineNumberColumns).
    number_cells: i32,
    lines: Vec<PrintedLine>,
}

/// A line as printed: a document line, or a part of one when it wraps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PrintedLine {
    /// The index of the document line.
    line: usize,
    /// Its bytes on this printed line.
    start: usize,
    end: usize,
}

impl Layout {
    /// Lays out the lines on pages. `lines` is the text split at its line breaks, so an empty
    /// last element is the end of a text that ends with a line break, which Sakura does not
    /// count as a line.
    pub fn new<S: AsRef<str>>(
        lines: &[S],
        setting: &PrintSetting,
        paper: Paper,
    ) -> Result<Self, NoPrintableArea> {
        let s = setting;
        if s.font_width <= 0 || s.font_height <= 0 || s.columns <= 0 || s.tab_width <= 0 {
            return Err(NoPrintableArea);
        }
        let count = document_lines(lines);
        let number_cells = if s.line_number {
            line_number_cells(count)
        } else {
            0
        };
        let cells = printable_cells(s, paper.width, number_cells);
        let rows = printable_rows(s, paper.height);
        if cells <= 0 || rows <= 0 {
            return Err(NoPrintableArea);
        }
        let mut printed = Vec::with_capacity(count);
        for (line, text) in lines[..count].iter().enumerate() {
            // Each line but the last one of the text ends with a line break.
            let has_break = line + 1 < lines.len();
            wrap(text.as_ref(), has_break, cells, s, |range| {
                printed.push(PrintedLine {
                    line,
                    start: range.start,
                    end: range.end,
                })
            });
        }
        Ok(Self {
            setting: s.clone(),
            paper,
            cells,
            rows,
            number_cells,
            lines: printed,
        })
    }

    pub fn setting(&self) -> &PrintSetting {
        &self.setting
    }

    pub fn paper(&self) -> Paper {
        self.paper
    }

    /// The number of pages: none for an empty text, which Sakura does not print.
    pub fn page_count(&self) -> usize {
        let per_page = (self.rows * self.setting.columns) as usize;
        self.lines.len().div_ceil(per_page)
    }

    /// Characters in a line and lines in a column.
    #[cfg(test)]
    fn grid(&self) -> (i32, i32) {
        (self.cells, self.rows)
    }

    /// The page `index`, counted from 0, as text with its place on the paper. The lines are
    /// the same as for `new`.
    pub fn page<S: AsRef<str>>(&self, index: usize, lines: &[S], doc: &DocInfo) -> Page {
        let s = &self.setting;
        let mut page = Page::default();

        // The header and the footer go 0.5 mm inside the margins (CPrintPreview::OnPaint and
        // OnPrint), on the baselines that DrawHeaderFooter and Print_DrawBlock give them.
        let left = s.margin_left + 5;
        let right = self.paper.width - (s.margin_right + 5);
        let header_baseline = s.margin_top + 5 + s.font_height;
        let footer_baseline = self.paper.height - (s.margin_bottom + 5);
        let pages = self.page_count();
        for (parts, baseline) in [(&s.header, header_baseline), (&s.footer, footer_baseline)] {
            for (i, form) in parts.iter().enumerate() {
                // Sakura lays a tab out here as in the text; a space is close enough.
                let text = expand(form, index, pages, doc).replace('\t', " ");
                if text.is_empty() {
                    continue;
                }
                let width = text.chars().map(char_cells).sum::<i32>() * s.font_width;
                let x = match i {
                    0 => left,
                    1 => (left + right - width) / 2,
                    _ => right - width,
                };
                page.texts.push(PlacedText { x, baseline, text });
            }
        }

        // CPrintPreview::DrawPageText
        let line_height = s.font_height + s.font_height * s.line_spacing / 100;
        let column_width = self.cells * s.font_width + s.column_space;
        let number_width = self.number_cells * s.font_width;
        let top = s.margin_top + header_height(s, &s.header) * 2;
        for column in 0..s.columns {
            // The left edge of the text; the line numbers go to the left of it.
            let base_x = s.margin_left + column_width * column + number_width * (column + 1);
            let mut drawn = 0;
            for row in 0..self.rows {
                let n = ((index as i32 * s.columns + column) * self.rows + row) as usize;
                let Some(printed) = self.lines.get(n) else {
                    break;
                };
                drawn = row + 1;
                let baseline = top + line_height * row + s.font_height;
                // Numbered by document line (m_bLineNumIsCRLF, the default of the document
                // types): a wrapped part gets none. The number ends a cell before the text.
                if s.line_number && printed.start == 0 {
                    let number = (printed.line + 1).to_string();
                    page.texts.push(PlacedText {
                        x: base_x - (number.len() as i32 + 1) * s.font_width,
                        baseline,
                        text: number,
                    });
                }
                let text = &lines[printed.line].as_ref()[printed.start..printed.end];
                place_line(text, base_x, baseline, s, &mut page.texts);
            }
            // A rule in the middle of the cell between the numbers and the text
            // (m_nLineTermType 1, the default of the document types).
            if s.line_number && drawn > 0 {
                let x = base_x - s.font_width / 2;
                page.rules.push(Rule {
                    from: (x, top),
                    to: (x, top + line_height * drawn),
                });
            }
        }
        page
    }
}

/// The number of document lines: without the empty element after a final line break.
fn document_lines<S: AsRef<str>>(lines: &[S]) -> usize {
    match lines.last() {
        Some(last) if last.as_ref().is_empty() => lines.len() - 1,
        _ => lines.len(),
    }
}

/// Puts a printed line on the grid from `x`. Tabs move to the next tab stop and print nothing
/// (CPrintPreview::Print_DrawLine and Print_DrawBlock), so the text is split at them.
fn place_line(text: &str, x: i32, baseline: i32, s: &PrintSetting, out: &mut Vec<PlacedText>) {
    let mut cell = 0;
    let mut run: Option<(usize, i32)> = None;
    for (i, ch) in text.char_indices() {
        if ch == '\t' {
            if let Some((start, at)) = run.take() {
                out.push(PlacedText {
                    x: x + at * s.font_width,
                    baseline,
                    text: text[start..i].to_string(),
                });
            }
            cell += s.tab_width - cell % s.tab_width;
        } else {
            run.get_or_insert((i, cell));
            cell += char_cells(ch);
        }
    }
    if let Some((start, at)) = run {
        out.push(PlacedText {
            x: x + at * s.font_width,
            baseline,
            text: text[start..].to_string(),
        });
    }
}

/// Splits a document line into printed lines of at most `max` cells (CLayoutMgr::_MakeOneLine,
/// without kinsoku). With word wrap, a line breaks before an English word that would reach its
/// end (_DoWordWrap). A line break takes a cell, as Sakura counts it (CNativeW::GetHabaOfChar),
/// so a line that fills the width puts its line break on a printed line of its own.
fn wrap(
    text: &str,
    has_break: bool,
    max: i32,
    s: &PrintSetting,
    mut push: impl FnMut(Range<usize>),
) {
    let mut begin = 0;
    let mut x = 0;
    // The end of the English word being placed, which is not checked again.
    let mut word_end = 0;
    let mut chars = text.char_indices().peekable();
    while let Some(&(i, ch)) = chars.peek() {
        if s.word_wrap && i >= word_end && is_word_char(ch) {
            let (end, width) = word_at(text, i);
            word_end = end;
            if x + width >= max && i > begin {
                push(begin..i);
                begin = i;
                x = 0;
            }
        }
        let width = if ch == '\t' {
            s.tab_width - x % s.tab_width
        } else {
            char_cells(ch)
        };
        // A character wider than the whole line still goes on it, which Sakura never meets.
        if x + width > max && i > begin {
            push(begin..i);
            begin = i;
            x = 0;
            continue;
        }
        x += width;
        chars.next();
    }
    if has_break && x + 1 > max {
        push(begin..text.len());
        begin = text.len();
    }
    push(begin..text.len());
}

/// The end of the English word at `start` and its width in cells (_GetKeywordLength).
fn word_at(text: &str, start: usize) -> (usize, i32) {
    let mut width = 0;
    for (i, ch) in text[start..].char_indices() {
        if !is_word_char(ch) {
            return (start + i, width);
        }
        width += char_cells(ch);
    }
    (text.len(), width)
}

/// Characters of an English word: letters, digits, `_`, and `#$@\` (IS_KEYWORD_CHAR,
/// gm_keyword_char in sakura_core/charset/charcode.cpp).
fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '#' | '$' | '@' | '\\')
}

/// Cells for a character: two for a full-width one (East Asian Wide, Fullwidth and Ambiguous,
/// as Japanese fonts draw them), none for a combining mark, and one for the others. Sakura
/// measures the characters in the print font instead (CCharWidthCache).
pub fn char_cells(ch: char) -> i32 {
    ch.width_cjk().map_or(1, |w| w as i32)
}

/// Whether a character is drawn in the font for full-width characters: Sakura uses it for all
/// but ASCII (WCODE::GetFontNo).
pub fn uses_zen_font(ch: char) -> bool {
    ch as u32 >= 0x80
}

/// Splits text into runs drawn in the same font: `true` for the font for full-width characters.
pub fn font_runs(text: &str) -> Vec<(Range<usize>, bool)> {
    let mut runs: Vec<(Range<usize>, bool)> = Vec::new();
    for (i, ch) in text.char_indices() {
        let zen = uses_zen_font(ch);
        let end = i + ch.len_utf8();
        match runs.last_mut() {
            Some((range, z)) if *z == zen => range.end = end,
            _ => runs.push((i..end, zen)),
        }
    }
    runs
}

/// Expands `$f`, `$F`, `$p`, `$P` and `$$` (CSakuraEnvironment::ExpandParameter). Other `$`
/// sequences stay as they are, as Sakura leaves those it does not know.
fn expand(form: &str, page: usize, pages: usize, doc: &DocInfo) -> String {
    // Sakura's STR_NO_TITLE1.
    const NO_TITLE: &str = "(無題)";
    let mut out = String::new();
    let mut chars = form.chars();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('$') => out.push('$'),
            Some('f') => out.push_str(doc.file_name.as_deref().unwrap_or(NO_TITLE)),
            Some('F') => out.push_str(doc.path.as_deref().unwrap_or(NO_TITLE)),
            Some('p') => out.push_str(&(page + 1).to_string()),
            Some('P') => out.push_str(&pages.to_string()),
            Some(other) => {
                out.push('$');
                out.push(other);
            }
            None => out.push('$'),
        }
    }
    out
}

/// The height of the header or the footer with its line spacing, or 0 when all its parts are
/// empty (CPrint::CalcHeaderHeight and CalcFooterHeight).
fn header_height(s: &PrintSetting, parts: &[String; 3]) -> i32 {
    if parts.iter().all(String::is_empty) {
        0
    } else {
        s.font_height * (s.line_spacing + 100) / 100
    }
}

/// Cells for the line numbers: the digits of the last line number, at least two
/// (LINENUMWIDTH_MIN, the document types' default), and one more for a space
/// (CTextArea::DetectWidthOfLineNumberArea_calculate).
fn line_number_cells(lines: usize) -> i32 {
    lines.to_string().len().max(2) as i32 + 1
}

/// Cells for the text in a line (CPrint::CalculatePrintableColumns).
fn printable_cells(s: &PrintSetting, paper_width: i32, number_cells: i32) -> i32 {
    let width = paper_width - s.margin_left - s.margin_right;
    let spaces = (s.columns - 1) * s.column_space + s.columns * (number_cells * s.font_width);
    if width < 0 || width < spaces {
        return 0;
    }
    (width - spaces) / s.font_width / s.columns
}

/// Lines in a column (CPrint::CalculatePrintableLines). The header and the footer take two
/// lines each: their own and a blank one.
fn printable_rows(s: &PrintSetting, paper_height: i32) -> i32 {
    let height = paper_height - s.margin_top - s.margin_bottom;
    if height < 0 {
        return 0;
    }
    let space = s.font_height * s.line_spacing / 100;
    let rows = (height - header_height(s, &s.header) * 2 - header_height(s, &s.footer) * 2 + space)
        / (s.font_height + space);
    rows.max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printed(layout: &Layout) -> Vec<(usize, usize, usize)> {
        layout
            .lines
            .iter()
            .map(|l| (l.line, l.start, l.end))
            .collect()
    }

    /// A setting with `cells` cells in a line, and no header or footer.
    fn narrow(cells: i32) -> (PrintSetting, Paper) {
        let s = PrintSetting {
            header: Default::default(),
            footer: Default::default(),
            ..PrintSetting::default()
        };
        let width = s.margin_left + s.margin_right + cells * s.font_width;
        (s, Paper { width, ..Paper::A4 })
    }

    #[test]
    fn sakura_defaults_on_a4() {
        let layout = Layout::new(&["a"], &PrintSetting::default(), Paper::A4).unwrap();
        // (2100 - 200 - 100) / 12 and (2970 - 300 - 31 * 4 + 7) / 31.
        assert_eq!(layout.grid(), (150, 82));
    }

    #[test]
    fn header_and_footer() {
        let doc = DocInfo {
            file_name: Some("memo.txt".into()),
            path: None,
        };
        let page = Layout::new(&["a"], &PrintSetting::default(), Paper::A4)
            .unwrap()
            .page(0, &["a"], &doc);
        let header = &page.texts[0];
        assert_eq!((header.x, header.baseline), (205, 129));
        assert_eq!(header.text, "memo.txt");
        // "- 1 -" is 5 cells, centered between 205 and 1995, on the baseline 0.5 mm inside the
        // bottom margin.
        let footer = &page.texts[1];
        assert_eq!(footer.text, "- 1 -");
        assert_eq!((footer.x, footer.baseline), ((205 + 1995 - 60) / 2, 2765));
        // The text starts below the header and its blank line.
        let body = &page.texts[2];
        assert_eq!((body.x, body.baseline, body.text.as_str()), (200, 186, "a"));
    }

    #[test]
    fn expands_header_forms() {
        let doc = DocInfo::default();
        assert_eq!(
            expand("$f $F $p/$P $$ $x $", 1, 3, &doc),
            "(無題) (無題) 2/3 $ $x $"
        );
    }

    #[test]
    fn wraps_full_width_characters_on_cells() {
        let (s, paper) = narrow(5);
        // "ab" takes 2 cells, "あ" 2 more; "い" would end at 6.
        let layout = Layout::new(&["abあい"], &s, paper).unwrap();
        assert_eq!(printed(&layout), [(0, 0, 5), (0, 5, 8)]);
    }

    #[test]
    fn breaks_before_an_english_word() {
        let (s, paper) = narrow(10);
        let layout = Layout::new(&["one twelve"], &s, paper).unwrap();
        // "twelve" would end at the 10th cell, which _DoWordWrap does not allow.
        assert_eq!(printed(&layout), [(0, 0, 4), (0, 4, 10)]);
        let no_wrap = PrintSetting {
            word_wrap: false,
            ..s.clone()
        };
        let layout = Layout::new(&["one twelve"], &no_wrap, paper).unwrap();
        assert_eq!(printed(&layout), [(0, 0, 10)]);
        // A word longer than a line breaks where the line ends.
        let layout = Layout::new(&["abcdefghijkl"], &s, paper).unwrap();
        assert_eq!(printed(&layout), [(0, 0, 10), (0, 10, 12)]);
    }

    #[test]
    fn a_full_line_puts_its_line_break_on_the_next_line() {
        let (s, paper) = narrow(4);
        let layout = Layout::new(&["あい", "あい"], &s, paper).unwrap();
        // The last line has no line break.
        assert_eq!(printed(&layout), [(0, 0, 6), (0, 6, 6), (1, 0, 6)]);
    }

    #[test]
    fn tabs_move_to_the_next_stop_and_split_the_text() {
        let (s, paper) = narrow(20);
        let page = Layout::new(&["a\tbc\td"], &s, paper).unwrap().page(
            0,
            &["a\tbc\td"],
            &DocInfo::default(),
        );
        let placed: Vec<_> = page.texts.iter().map(|t| (t.x, t.text.as_str())).collect();
        assert_eq!(
            placed,
            [(200, "a"), (200 + 4 * 12, "bc"), (200 + 8 * 12, "d")]
        );
    }

    #[test]
    fn empty_text_has_no_pages() {
        let s = PrintSetting::default();
        assert_eq!(Layout::new(&[""], &s, Paper::A4).unwrap().page_count(), 0);
        // A final line break is not a line of its own.
        let layout = Layout::new(&["a", ""], &s, Paper::A4).unwrap();
        assert_eq!((layout.page_count(), layout.lines.len()), (1, 1));
    }

    #[test]
    fn pages_and_columns() {
        let lines: Vec<String> = (1..=200).map(|i| i.to_string()).collect();
        let s = PrintSetting::default();
        assert_eq!(Layout::new(&lines, &s, Paper::A4).unwrap().page_count(), 3);
        let two = PrintSetting { columns: 2, ..s };
        let layout = Layout::new(&lines, &two, Paper::A4).unwrap();
        assert_eq!(layout.page_count(), 2);
        // (1800 - 70) / 12 / 2 cells in each column.
        assert_eq!(layout.grid(), (72, 82));
        // Line 83 starts the second column, 72 cells and the space to the right.
        let page = layout.page(0, &lines, &DocInfo::default());
        let line_83 = page.texts.iter().find(|t| t.text == "83").unwrap();
        assert_eq!((line_83.x, line_83.baseline), (200 + 72 * 12 + 70, 186));
    }

    #[test]
    fn line_numbers() {
        let lines: Vec<String> = (0..120).map(|_| "あ".repeat(80)).collect();
        let s = PrintSetting {
            line_number: true,
            ..PrintSetting::default()
        };
        let layout = Layout::new(&lines, &s, Paper::A4).unwrap();
        // Three digits for 120 lines and a space, out of 150 cells.
        assert_eq!(layout.grid(), (146, 82));
        let page = layout.page(0, &lines, &DocInfo::default());
        let numbers: Vec<_> = page
            .texts
            .iter()
            .filter(|t| t.text.chars().all(|c| c.is_ascii_digit()))
            .map(|t| (t.x, t.text.as_str()))
            .take(2)
            .collect();
        // Each line wraps once, and the wrapped part gets no number. "1" ends a cell before
        // the text, which starts after the 4 cells of the numbers.
        let base_x = 200 + 4 * 12;
        assert_eq!(numbers, [(base_x - 2 * 12, "1"), (base_x - 2 * 12, "2")]);
        assert_eq!(page.texts.iter().filter(|t| t.text == "2").count(), 1);
        assert_eq!(
            page.rules,
            [Rule {
                from: (base_x - 6, 186 - 24),
                to: (base_x - 6, 162 + 31 * 82),
            }]
        );
    }

    #[test]
    fn grid_positions() {
        let text = PlacedText {
            x: 100,
            baseline: 0,
            text: "aあb".into(),
        };
        let xs: Vec<_> = text.positions(12).map(|(i, _, x)| (i, x)).collect();
        assert_eq!(xs, [(0, 100), (1, 112), (4, 136)]);
        assert_eq!(
            font_runs("ab日本c"),
            [(0..2, false), (2..8, true), (8..9, false)]
        );
    }

    #[test]
    fn no_room_for_text() {
        let s = PrintSetting {
            margin_left: 1500,
            margin_right: 600,
            ..PrintSetting::default()
        };
        assert_eq!(
            Layout::new(&["a"], &s, Paper::A4).unwrap_err(),
            NoPrintableArea
        );
    }
}
