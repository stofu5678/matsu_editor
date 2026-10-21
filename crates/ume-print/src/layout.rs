// Derived from Sakura Editor (zlib License):
//   sakura_core/print/CPrint.cpp (CalculatePrintableColumns, CalculatePrintableLines,
//     CalcHeaderHeight, CalcFooterHeight)
//   sakura_core/print/CPrintPreview.cpp (OnChangePrintSetting, OnPrint, DrawHeaderFooter,
//     DrawPageText, Print_DrawLine, Print_DrawBlock)
//   sakura_core/view/CTextArea.cpp (DetectWidthOfLineNumberArea_calculate)
//   sakura_core/view/CTextMetrics.cpp (GenerateDxArray, CalcTextWidth2)
//   sakura_core/charset/charcode.cpp (WCODE::GetFontNo)
// Modified: rewritten in Rust. The pages are given as text and its position instead of being
// drawn on a device context; the wrapping is done by `ume-layout`; color printing and the
// printer's unprintable margin are left to the caller.
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

use std::fmt;
use std::ops::Range;

use ume_layout::{
    CharSet, DEFAULT_HEAD, DEFAULT_KUTO, DEFAULT_TAIL, Kinsoku, LayoutConfig, Row, WidthRule, Wrap,
    WrapIndent, layout_line,
};

use crate::Paper;
use crate::header::{HeaderContext, PageInfo, expand_parameter};
use crate::page::{FontKind, HAlign, Page, PlacedText, Rule};
use crate::setting::{POS_CENTER, POS_LEFT, PrintSetting};
use crate::source::LineSource;

/// The most digits of a line number Sakura makes room for (`LINENUMWIDTH_MAX`).
const LINENUMWIDTH_MAX: u32 = 11;
/// The fewest digits of a line number (`LINENUMWIDTH_MIN`, `config/app_constants.h`).
pub const LINENUMWIDTH_MIN: u32 = 2;

/// What the line numbers are followed by (`m_nLineTermType` and `m_cLineTermChar` of the
/// document type).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LineTerm {
    /// 0: a space.
    None,
    /// 1: a space, and a vertical line in the middle of it. Sakura's default.
    #[default]
    Line,
    /// 2: a character, such as `:`.
    Char(char),
}

/// What the layout takes from the document type (`STypeConfig`) of the document: the print
/// layout is made from a copy of it (`CPrintPreview::m_typePrint`). The defaults are those of
/// Sakura's types (`CType.cpp`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeOptions {
    /// Tab stops every this many cells (`m_nTabSpace`). A tab as wide as the line is 4
    /// (see `LayoutConfig::effective_tab_width`).
    pub tab_width: u32,
    /// The indent of wrapped rows (`m_nIndentLayout` and the like).
    pub indent: WrapIndent,
    /// How wide a character is. Sakura asks the print font instead (`CCharWidthCache`).
    pub width: WidthRule,
    /// Characters that may not start a line (`m_szKinsokuHead`), used when the setting turns
    /// 行頭禁則 on.
    pub kinsoku_head: CharSet,
    /// Characters that may not end a line (`m_szKinsokuTail`).
    pub kinsoku_tail: CharSet,
    /// Punctuation that may hang (`m_szKinsokuKuto`).
    pub kinsoku_kuto: CharSet,
    /// Numbers the lines of the document, and not the printed rows (`m_bLineNumIsCRLF`:
    /// 改行単位). Sakura's default is `true`.
    pub line_number_is_crlf: bool,
    /// The fewest digits of a line number (`m_nLineNumWidth`), at least [`LINENUMWIDTH_MIN`].
    pub line_number_width: u32,
    /// What follows the line numbers.
    pub line_term: LineTerm,
}

impl Default for TypeOptions {
    fn default() -> Self {
        Self {
            tab_width: 4,
            indent: WrapIndent::None,
            width: WidthRule::default(),
            kinsoku_head: DEFAULT_HEAD.into(),
            kinsoku_tail: DEFAULT_TAIL.into(),
            kinsoku_kuto: DEFAULT_KUTO.into(),
            line_number_is_crlf: true,
            line_number_width: LINENUMWIDTH_MIN,
            line_term: LineTerm::Line,
        }
    }
}

/// Why a document cannot be laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrintError {
    /// A number of the setting is out of range (a size of zero or less, no column).
    InvalidSetting(&'static str),
    /// The margins, the columns and the font leave no room for the text.
    NoPrintableArea,
}

impl fmt::Display for PrintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrintError::InvalidSetting(what) => write!(f, "invalid print setting: {what}"),
            // Sakura's STR_ERR_DLGPRNPRVW3_1.
            PrintError::NoPrintableArea => {
                f.write_str("印刷ページ設定エラー:印字可能領域がありません")
            }
        }
    }
}

impl std::error::Error for PrintError {}

/// Where a page begins: the line and the row of that line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PageStart {
    line: usize,
    row: usize,
}

/// A document split into printed rows and pages, for one setting and one paper. It keeps only
/// where each page begins, not the text; [`Layout::page`] reads the lines of a page again from
/// the source.
#[derive(Clone, Debug)]
pub struct Layout {
    setting: PrintSetting,
    options: TypeOptions,
    paper: Paper,
    config: LayoutConfig,
    /// Cells for the text in a row (`m_bPreview_EnableColumns`).
    cells: u32,
    /// Rows in a column (`m_bPreview_EnableLines`).
    rows: usize,
    /// Cells for the line numbers and the space after them (`m_nPreview_LineNumberColumns`).
    number_cells: u32,
    /// Lines of the document and printed rows.
    line_count: usize,
    row_count: usize,
    starts: Vec<PageStart>,
}

impl Layout {
    /// Splits the document into pages. It reads every line once; `Layout` keeps no text.
    ///
    /// The setting is used as it is (see [`PrintSetting::normalized`] for Sakura's dialog
    /// rules). `paper` is the paper as turned, see [`Paper::of_setting`].
    pub fn new<S: LineSource + ?Sized>(
        source: &S,
        setting: &PrintSetting,
        paper: Paper,
        options: &TypeOptions,
    ) -> Result<Self, PrintError> {
        let s = setting;
        if s.print_font_width <= 0 {
            return Err(PrintError::InvalidSetting("print_font_width"));
        }
        if s.print_font_height <= 0 {
            return Err(PrintError::InvalidSetting("print_font_height"));
        }
        if s.print_dansuu <= 0 {
            return Err(PrintError::InvalidSetting("print_dansuu"));
        }
        // The pitch of the rows must be positive (the rows are counted by dividing by it).
        if s.print_font_height + s.print_font_height * s.print_line_spacing / 100 <= 0 {
            return Err(PrintError::InvalidSetting("print_line_spacing"));
        }
        if options.tab_width == 0 {
            return Err(PrintError::InvalidSetting("tab_width"));
        }
        let line_count = source.line_count();
        // The width of the line numbers depends on the rows (when the rows are numbered), and
        // the rows on that width: go on until they agree, as Sakura does by laying out again
        // (OnChangePrintSetting, "2014.07.23").
        let mut number_cells = if s.print_line_number {
            number_cells_for(options, line_count)
        } else {
            0
        };
        for _ in 0..=LINENUMWIDTH_MAX {
            let cells = printable_cells(s, paper.width, number_cells);
            let rows = printable_rows(s, paper.height);
            if cells <= 0 || rows <= 0 {
                return Err(PrintError::NoPrintableArea);
            }
            let config = layout_config(s, options, cells as u32);
            let per_page = rows as usize * s.print_dansuu as usize;
            let (row_count, starts) = scan(source, &config, per_page);
            let wanted = if s.print_line_number {
                let numbered = if options.line_number_is_crlf {
                    line_count
                } else {
                    row_count
                };
                number_cells_for(options, numbered)
            } else {
                0
            };
            if wanted == number_cells {
                return Ok(Self {
                    setting: s.clone(),
                    options: options.clone(),
                    paper,
                    config,
                    cells: cells as u32,
                    rows: rows as usize,
                    number_cells,
                    line_count,
                    row_count,
                    starts,
                });
            }
            number_cells = wanted;
        }
        // The widths ran away: the numbers need more digits than Sakura allows for.
        Err(PrintError::NoPrintableArea)
    }

    pub fn setting(&self) -> &PrintSetting {
        &self.setting
    }

    pub fn paper(&self) -> Paper {
        self.paper
    }

    /// The width rule that decided the width of each character: the one to give
    /// [`PlacedText::positions`].
    pub fn width_rule(&self) -> WidthRule {
        self.options.width
    }

    /// The number of pages (`m_nAllPageNum`): none for an empty document, which Sakura does not
    /// print (`STR_ERR_DLGPRNPRVW7`).
    pub fn page_count(&self) -> usize {
        self.starts.len()
    }

    /// The number of cells for the text in a row, without the line numbers.
    pub fn cells_per_row(&self) -> u32 {
        self.cells
    }

    /// The number of rows in a column.
    pub fn rows_per_column(&self) -> usize {
        self.rows
    }

    /// The cells the line numbers and the space after them take in front of each column.
    pub fn number_cells(&self) -> u32 {
        self.number_cells
    }

    /// The number of printed rows of the whole document (`CLayoutMgr::GetLineCount`).
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// The lines of the document the layout was made from.
    pub fn line_count(&self) -> usize {
        self.line_count
    }

    /// The page `index`, counted from 0, as text with its place on the paper, or none past the
    /// last page. `source` must be the document the layout was made from.
    ///
    /// The order of the pages Sakura prints is that of the page numbers; the range of pages a
    /// user picks in the print dialog (`nFromPage`, `nToPage`) is a range of `index`.
    ///
    /// A page that begins in the middle of a line lays that whole line out again (`ume-layout`
    /// cannot resume in a row), which is slow for a line of many pages; [`Layout::pages`] keeps
    /// the line it laid out last for the page that follows.
    pub fn page<S: LineSource + ?Sized>(
        &self,
        index: usize,
        source: &S,
        ctx: &HeaderContext,
    ) -> Option<Page> {
        self.page_cached(index, source, ctx, &mut LineCache::default())
    }

    fn page_cached<S: LineSource + ?Sized>(
        &self,
        index: usize,
        source: &S,
        ctx: &HeaderContext,
        cache: &mut LineCache,
    ) -> Option<Page> {
        let start = *self.starts.get(index)?;
        let s = &self.setting;
        let mut page = Page {
            index,
            ..Page::default()
        };

        let pages = self.page_count();
        let info = PageInfo {
            number: index + 1,
            total: pages,
            setting_name: &s.name,
        };
        // The header and the footer go 0.5 mm inside the margins (OnPrint, "cRect").
        let rect_left = s.print_margin_lx + 5;
        let rect_right = self.paper.width - (s.print_margin_rx + 5);
        if s.header_height() > 0 {
            self.place_header_footer(true, (rect_left, rect_right), &info, ctx, &mut page.texts);
        }

        // DrawPageText: `top` is the top of the first row, below the header and the blank line
        // after it.
        let line_height = s.print_font_height + s.print_font_height * s.print_line_spacing / 100;
        let column_width = self.cells as i32 * s.print_font_width + s.print_dan_space;
        let number_width = self.number_cells as i32 * s.print_font_width;
        let top = s.print_margin_ty + s.header_height() * 2;
        let columns = s.print_dansuu as usize;
        let per_page = self.rows * columns;
        let mut drawn = vec![0usize; columns];

        let mut at = 0;
        let mut line = start.line;
        let mut skip = start.row;
        while at < per_page && line < self.line_count {
            cache.load(line, source, &self.config);
            let text = &cache.text;
            for (n, row) in cache.rows.iter().enumerate().skip(skip) {
                let column = at / self.rows;
                let row_in_column = at % self.rows;
                // The left edge of the text; the line numbers go to the left of it.
                let base_x = s.print_margin_lx
                    + column_width * column as i32
                    + number_width * (column as i32 + 1);
                let baseline = top + line_height * row_in_column as i32 + s.print_font_height;
                if s.print_line_number {
                    // CRLF mode: the number of the line on its first row, a blank on the
                    // others. Otherwise the number of the printed row.
                    let number = if !self.options.line_number_is_crlf {
                        Some(index * per_page + at + 1)
                    } else if n == 0 {
                        Some(line + 1)
                    } else {
                        None
                    };
                    self.place_line_number(number, base_x, baseline, &mut page.texts);
                }
                let row_text = row.text(text);
                if !row_text.is_empty() {
                    place_runs(
                        row_text,
                        row.start_column,
                        base_x,
                        baseline,
                        s.print_font_width,
                        &self.config,
                        &mut page.texts,
                    );
                }
                drawn[column] = row_in_column + 1;
                at += 1;
                if at == per_page {
                    break;
                }
            }
            skip = 0;
            line += 1;
        }

        // A vertical line in the middle of the cell between the numbers and the text, once for
        // a column (m_nLineTermType 1).
        if s.print_line_number && self.options.line_term == LineTerm::Line {
            for (column, &rows_drawn) in drawn.iter().enumerate() {
                if rows_drawn == 0 {
                    continue;
                }
                let base_x = s.print_margin_lx
                    + column_width * column as i32
                    + number_width * (column as i32 + 1);
                let x = base_x - s.print_font_width / 2;
                page.rules.push(Rule {
                    from: (x, top),
                    to: (x, top + line_height * rows_drawn as i32),
                });
            }
        }

        if s.footer_height() > 0 {
            self.place_header_footer(false, (rect_left, rect_right), &info, ctx, &mut page.texts);
        }
        Some(page)
    }

    /// The pages `range` of the document, in order. The range is clipped to the pages there are.
    pub fn pages<'a, S: LineSource + ?Sized>(
        &'a self,
        range: Range<usize>,
        source: &'a S,
        ctx: &'a HeaderContext,
    ) -> impl Iterator<Item = Page> + 'a {
        let end = range.end.min(self.page_count());
        let mut cache = LineCache::default();
        (range.start..end).filter_map(move |i| self.page_cached(i, source, ctx, &mut cache))
    }

    /// The line number and what follows it (`DrawPageText`). The text ends where the text of the
    /// row begins, so its left edge is as many cells to the left as the text has UTF-16 units
    /// (Sakura counts units, not widths). A space prints nothing, so a trailing one is left
    /// out.
    fn place_line_number(
        &self,
        number: Option<usize>,
        base_x: i32,
        baseline: i32,
        out: &mut Vec<PlacedText>,
    ) {
        let term = match self.options.line_term {
            LineTerm::Char(c) => c,
            LineTerm::None | LineTerm::Line => ' ',
        };
        // A row without a number has a blank in its place, and still the character after it.
        let label = match number {
            Some(number) => format!("{number}{term}"),
            None => format!(" {term}"),
        };
        let cell = self.setting.print_font_width;
        let units: usize = label.chars().map(char::len_utf16).sum();
        let x = base_x - units as i32 * cell;
        let drawn = label.trim_end_matches(' ');
        if !drawn.is_empty() {
            place_runs(drawn, 0, x, baseline, cell, &self.config, out);
        }
    }

    /// `DrawHeaderFooter`: the left, center and right part. `rect` is the left and right edge
    /// of the area.
    fn place_header_footer(
        &self,
        header: bool,
        rect: (i32, i32),
        info: &PageInfo<'_>,
        ctx: &HeaderContext,
        out: &mut Vec<PlacedText>,
    ) {
        let s = &self.setting;
        let (forms, font) = if header {
            (&s.header_form, s.header_font.as_ref())
        } else {
            (&s.footer_form, s.footer_font.as_ref())
        };
        let font = font.filter(|f| !f.face.is_empty());
        let fw = s.print_font_width;
        // The size of the characters of the part, and the baseline: the header's text starts
        // at `rect.top`, the footer's ends at `rect.bottom`. With a font of its own the
        // ascent is not known; the baseline is put one font height from the top.
        let height = font.map_or(s.print_font_height, |f| f.height());
        let baseline = if header {
            s.print_margin_ty + 5 + height
        } else {
            self.paper.height - (s.print_margin_by + 5)
        };
        for (i, form) in forms.iter().enumerate() {
            let mut text = expand_parameter(form, ctx, Some(info));
            if text.is_empty() {
                continue;
            }
            if font.is_some() {
                // The text is measured with the font, so the drawer has to align it.
                text = text.replace('\t', " ");
                let (x, align) = match i {
                    POS_LEFT => (rect.0, HAlign::Left),
                    POS_CENTER => ((rect.0 + rect.1) / 2, HAlign::Center),
                    _ => (rect.1, HAlign::Right),
                };
                out.push(PlacedText {
                    x,
                    baseline,
                    text,
                    font: if header {
                        FontKind::Header
                    } else {
                        FontKind::Footer
                    },
                    align,
                });
            } else {
                let width = header_text_width(&text, fw, self.options.width);
                let x = match i {
                    POS_LEFT => rect.0,
                    POS_CENTER => (rect.1 + rect.0 - width) / 2,
                    _ => rect.1 - width,
                };
                place_runs(&text, 0, x, baseline, fw, &self.config, out);
            }
        }
    }
}

/// The line laid out last: the page after a page that ends in the middle of a line goes on
/// with the same line.
#[derive(Default)]
struct LineCache {
    line: Option<usize>,
    text: String,
    rows: Vec<Row>,
}

impl LineCache {
    fn load<S: LineSource + ?Sized>(&mut self, line: usize, source: &S, config: &LayoutConfig) {
        if self.line != Some(line) {
            let has_eol = source.read_line(line, &mut self.text);
            rows_of(&self.text, has_eol, config, &mut self.rows);
            self.line = Some(line);
        }
    }
}

/// The settings of the wrapping (`OnChangePrintSetting`: the copy of the document type).
fn layout_config(s: &PrintSetting, options: &TypeOptions, cells: u32) -> LayoutConfig {
    LayoutConfig {
        wrap: Wrap::Columns(cells),
        tab_width: options.tab_width,
        word_wrap: s.print_word_wrap,
        kinsoku: Kinsoku {
            head: s.print_kinsoku_head,
            tail: s.print_kinsoku_tail,
            kuto: s.print_kinsoku_kuto,
            hang_eol: s.print_kinsoku_ret,
            head_chars: options.kinsoku_head.clone(),
            tail_chars: options.kinsoku_tail.clone(),
            kuto_chars: options.kinsoku_kuto.clone(),
        },
        width: options.width,
        indent: options.indent.clone(),
    }
}

/// The printed rows of a line. An empty last line without a line ending has none (Sakura keeps
/// no such line; [`LineSource`] does not give it either, but a caller's last line may be empty).
fn rows_of(text: &str, has_eol: bool, config: &LayoutConfig, rows: &mut Vec<Row>) {
    if text.is_empty() && !has_eol {
        rows.clear();
    } else {
        layout_line(text, has_eol, config, rows);
    }
}

/// Counts the printed rows of the document and finds where each page begins.
fn scan<S: LineSource + ?Sized>(
    source: &S,
    config: &LayoutConfig,
    per_page: usize,
) -> (usize, Vec<PageStart>) {
    let mut starts = Vec::new();
    let mut total = 0usize;
    let mut text = String::new();
    let mut rows = Vec::new();
    for line in 0..source.line_count() {
        let has_eol = source.read_line(line, &mut text);
        rows_of(&text, has_eol, config, &mut rows);
        // The first row of this line, if any, on which a page begins.
        let first = (per_page - total % per_page) % per_page;
        let mut row = first;
        while row < rows.len() {
            starts.push(PageStart { line, row });
            row += per_page;
        }
        total += rows.len();
    }
    (total, starts)
}

/// Puts a row on the grid from `x`, starting at the column `start_column`. A tab moves to the
/// next tab stop and prints nothing (`Print_DrawBlock`: "TAB is not printed when there is no
/// color"), so the text is split at it; it is also split where the font changes (`Print_DrawLine`).
fn place_runs(
    text: &str,
    start_column: u32,
    x: i32,
    baseline: i32,
    cell: i32,
    config: &LayoutConfig,
    out: &mut Vec<PlacedText>,
) {
    let mut column = start_column;
    // The start of the run in `text`, its column and its font.
    let mut run: Option<(usize, u32, FontKind)> = None;
    let flush =
        |run: &mut Option<(usize, u32, FontKind)>, end: usize, out: &mut Vec<PlacedText>| {
            if let Some((start, at, font)) = run.take() {
                out.push(PlacedText {
                    x: x + at as i32 * cell,
                    baseline,
                    text: text[start..end].to_string(),
                    font,
                    align: HAlign::Left,
                });
            }
        };
    for (i, ch) in text.char_indices() {
        if ch == '\t' {
            flush(&mut run, i, out);
        } else {
            let font = font_of(ch);
            if run.is_some_and(|(_, _, f)| f != font) {
                flush(&mut run, i, out);
            }
            run.get_or_insert((i, column, font));
        }
        column += config.columns_of(ch, column);
    }
    flush(&mut run, text.len(), out);
}

/// The font of a character: the one for full-width characters from U+0080 on
/// (`WCODE::GetFontNo`, 0x0080..=0xFFFF in UTF-16, so every character that is not ASCII).
fn font_of(ch: char) -> FontKind {
    if ch.is_ascii() {
        FontKind::Han
    } else {
        FontKind::Zen
    }
}

/// The width of the text of a header or a footer in the print font, as `CalcTextWidth2` finds
/// it for centering and right alignment. It measures a tab by the formula of `GenerateDxArray`
/// with the tab width 8 *in 0.1 mm* (Sakura passes the plain 8 there, and the columns of the
/// tab in the drawing are another thing), so a tab is a fraction of a cell here.
fn header_text_width(text: &str, cell: i32, rule: WidthRule) -> i32 {
    let mut x = 0;
    for ch in text.chars() {
        if ch == '\t' {
            x += (8 + cell - 1) - ((x + cell - 1) % 8);
        } else {
            x += rule.columns(ch) as i32 * cell;
        }
    }
    x
}

/// The cells for the line numbers and the space after them when `numbered_lines` lines are
/// numbered (`DetectWidthOfLineNumberArea_calculate`): the digits of the count, at least the
/// setting's. The print setting dialog needs it before there is a layout with the line numbers.
pub fn line_number_cells(options: &TypeOptions, numbered_lines: usize) -> u32 {
    number_cells_for(options, numbered_lines)
}

/// The cells for the line numbers and the space after them for `lines` numbered lines
/// (`DetectWidthOfLineNumberArea_calculate`): the digits of the count, at least the setting's.
fn number_cells_for(options: &TypeOptions, lines: usize) -> u32 {
    let min = options
        .line_number_width
        .clamp(LINENUMWIDTH_MIN, LINENUMWIDTH_MAX);
    if lines == 0 {
        return min + 1;
    }
    // Sakura starts from 10^min and multiplies by 10 while the number of lines reaches it.
    let mut limit = 10u128.pow(min);
    let mut digits = min;
    while digits < LINENUMWIDTH_MAX {
        if limit > lines as u128 {
            break;
        }
        limit *= 10;
        digits += 1;
    }
    digits + 1
}

impl PrintSetting {
    /// The cells for the text in a row of `paper`, with `number_cells` cells for the line numbers
    /// (`CPrint::CalculatePrintableColumns`); 0 for no room. The print setting dialog shows it
    /// as 「行あたりの文字数」.
    pub fn printable_columns(&self, paper: Paper, number_cells: u32) -> i32 {
        printable_cells(self, paper.width, number_cells)
    }

    /// The rows in a column of `paper` (`CPrint::CalculatePrintableLines`); the dialog shows it as
    /// 「縦方向の行数」.
    pub fn printable_lines(&self, paper: Paper) -> i32 {
        printable_rows(self, paper.height)
    }
}

/// The cells for the text in a row (`CPrint::CalculatePrintableColumns`); 0 for no room.
fn printable_cells(s: &PrintSetting, paper_width: i32, number_cells: u32) -> i32 {
    let width = paper_width - s.print_margin_lx - s.print_margin_rx;
    if width < 0 {
        return 0;
    }
    let spaces = (s.print_dansuu - 1) * s.print_dan_space
        + s.print_dansuu * (number_cells as i32 * s.print_font_width);
    if width < spaces {
        return 0;
    }
    (width - spaces) / s.print_font_width / s.print_dansuu
}

/// The rows in a column (`CPrint::CalculatePrintableLines`). The header and the footer take two
/// rows each: their own and a blank one.
fn printable_rows(s: &PrintSetting, paper_height: i32) -> i32 {
    let height = paper_height - s.print_margin_ty - s.print_margin_by;
    if height < 0 {
        return 0;
    }
    let space = s.print_font_height * s.print_line_spacing / 100;
    let rows = (height - s.header_height() * 2 - s.footer_height() * 2 + space)
        / (s.print_font_height + space);
    rows.max(0)
}
