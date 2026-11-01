//! What the layout gives: pages as positioned text and lines. The print preview and the OS
//! printers only draw this.

use matsu_layout::WidthRule;

/// Which font draws a piece of text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontKind {
    /// The print font for half-width characters (`m_szPrintFontFaceHan`): ASCII.
    Han,
    /// The print font for the other characters (`m_szPrintFontFaceZen`): everything from
    /// U+0080 on, as `WCODE::GetFontNo` in Sakura.
    Zen,
    /// The font of the header (`m_lfHeader`), which is set only when the setting has one.
    Header,
    /// The font of the footer (`m_lfFooter`).
    Footer,
}

/// Where the `x` of a piece of text is on it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HAlign {
    /// `x` is the left edge of the first cell. All text on the grid is like this.
    #[default]
    Left,
    /// `x` is the middle of the text. Only a header or a footer in a font of its own: Sakura
    /// measures such text with the font (`GetTextExtentPoint32W`), which the layout cannot.
    Center,
    /// `x` is the right edge of the text. Same as `Center`.
    Right,
}

/// A color as `0xRRGGBB`.
pub type Rgb = u32;

/// How color printing draws a piece of text (`Print_DrawBlock`): the text color of the color
/// entry, and its bold and underline. The background of the entry is not printed (Sakura has
/// that line commented out).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextStyle {
    pub color: Rgb,
    pub bold: bool,
    pub underline: bool,
}

/// Text at a place on the paper. All lengths are in 0.1 mm, from the top left corner of the
/// paper; Sakura also takes the printer's unprintable margin off (`m_nPreview_PaperOffsetLeft`
/// and `m_nPreview_PaperOffsetTop`), which is for the code that talks to the printer.
///
/// Text on the grid ([`FontKind::Han`] and [`FontKind::Zen`]) holds no tab and no line ending.
/// Its characters go to the right from `x`, each in as many cells as [`PlacedText::positions`]
/// says, whatever their widths in the font are: Sakura gives `ExtTextOut` the width of each
/// character for this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedText {
    /// The left edge of the first cell (see [`HAlign`]).
    pub x: i32,
    /// The baseline. Sakura puts the top of the text at `baseline - print_font_height` (and the
    /// header and the footer so that the same holds); the baseline is that plus the ascent of
    /// the font, in whose terms the font height is the whole em.
    pub baseline: i32,
    pub text: String,
    pub font: FontKind,
    pub align: HAlign,
    /// The color, bold and underline when the setting prints in color ([`crate::LineStyler`]
    /// gave them); `None` is plain black text.
    pub style: Option<TextStyle>,
}

impl PlacedText {
    /// Each character's byte offset, the character, and the left edge of its cells, for text
    /// on the grid. `cell` is the width of a cell (`print_font_width`) and `rule` the width
    /// rule of the layout ([`crate::Layout::width_rule`]).
    pub fn positions(
        &self,
        cell: i32,
        rule: WidthRule,
    ) -> impl Iterator<Item = (usize, char, i32)> + '_ {
        let mut x = self.x;
        self.text.char_indices().map(move |(i, ch)| {
            let at = x;
            x += rule.columns(ch) as i32 * cell;
            (i, ch, at)
        })
    }
}

/// A straight line, from `from` to `to` as `(x, y)`. Sakura draws one: the line between the line
/// numbers and the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rule {
    pub from: (i32, i32),
    pub to: (i32, i32),
}

/// A page: the header, the footer, and the text of every column, in the order Sakura draws
/// them (the header first, then the columns from the left, the footer last).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Page {
    /// The number of the page, from 0.
    pub index: usize,
    pub texts: Vec<PlacedText>,
    pub rules: Vec<Rule>,
}
