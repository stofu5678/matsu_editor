//! The screen as the UI reads it: plain copies of `alacritty_terminal`'s cells, colors and modes, so the UI
//! neither depends on its types nor keeps the terminal locked while it draws.

use std::sync::Arc;

use alacritty_terminal::term::TermMode;
use alacritty_terminal::term::cell::{Cell as GridCell, Flags};
use alacritty_terminal::vte::ansi::{
    Color as GridColor, CursorShape as GridCursorShape, NamedColor,
};

/// An RGB color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// The color of a cell's foreground or background.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    /// The terminal's default foreground or background (which one depends on where the color is used).
    Default,
    /// An entry of the 256-color palette: 0 to 7 the ANSI colors, 8 to 15 the bright ones, 16 to 231 the
    /// color cube, 232 to 255 the grays. The UI picks the actual colors (its theme).
    Indexed(u8),
    /// A 24-bit color.
    Rgb(Rgb),
}

/// One cell of the grid.
#[derive(Clone, Debug, PartialEq)]
pub struct Cell {
    /// The character; a space for an empty cell. For the right half of a wide character it is a space, and
    /// [`Cell::wide_spacer`] is set.
    pub ch: char,
    /// Combining and other zero-width characters that follow `ch` (usually empty).
    pub zerowidth: Vec<char>,
    pub fg: Color,
    pub bg: Color,
    pub underline_color: Option<Color>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    /// Any kind of underline (single, double, curly, dotted, dashed).
    pub underline: bool,
    pub strikeout: bool,
    /// Foreground and background are swapped.
    pub inverse: bool,
    /// The text is hidden (SGR 8).
    pub hidden: bool,
    /// The left half of a wide character: the cell is two columns wide.
    pub wide: bool,
    /// The right half of a wide character; draw nothing, the left cell covers it.
    pub wide_spacer: bool,
    /// A blank cell at the end of a line, where a wide character did not fit and went to the next line.
    pub leading_wide_spacer: bool,
    /// The link (OSC 8) the cell belongs to. Only the address is kept ([`crate::link_at`] finds the link under a
    /// cell).
    pub hyperlink: Option<Arc<str>>,
}

impl Cell {
    fn from_grid(cell: &GridCell, previous_link: Option<&Arc<str>>) -> Self {
        let flags = cell.flags;
        let hyperlink = cell.hyperlink().map(|link| match previous_link {
            // Cells of one link are next to each other: share the allocation.
            Some(previous) if **previous == *link.uri() => Arc::clone(previous),
            _ => Arc::from(link.uri()),
        });
        Self {
            ch: cell.c,
            zerowidth: cell.zerowidth().map(<[char]>::to_vec).unwrap_or_default(),
            fg: convert_color(cell.fg),
            bg: convert_color(cell.bg),
            underline_color: cell.underline_color().map(convert_color),
            bold: flags.contains(Flags::BOLD),
            dim: flags.contains(Flags::DIM),
            italic: flags.contains(Flags::ITALIC),
            underline: flags.intersects(Flags::ALL_UNDERLINES),
            strikeout: flags.contains(Flags::STRIKEOUT),
            inverse: flags.contains(Flags::INVERSE),
            hidden: flags.contains(Flags::HIDDEN),
            wide: flags.contains(Flags::WIDE_CHAR),
            wide_spacer: flags.contains(Flags::WIDE_CHAR_SPACER),
            leading_wide_spacer: flags.contains(Flags::LEADING_WIDE_CHAR_SPACER),
            hyperlink,
        }
    }
}

/// One line of the grid.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub cells: Vec<Cell>,
    /// The line continues on the next one (the program wrote past the last column).
    pub wrapped: bool,
}

impl Row {
    pub(crate) fn from_grid(row: &alacritty_terminal::grid::Row<GridCell>) -> Self {
        let mut cells: Vec<Cell> = Vec::with_capacity(row.len());
        let mut link: Option<Arc<str>> = None;
        for grid_cell in row {
            let cell = Cell::from_grid(grid_cell, link.as_ref());
            link.clone_from(&cell.hyperlink);
            cells.push(cell);
        }
        let wrapped = row
            .last()
            .is_some_and(|cell| cell.flags.contains(Flags::WRAPLINE));
        Self { cells, wrapped }
    }

    /// The text of the line: wide characters once (their spacer cells are skipped), zero-width characters
    /// after their base, trailing spaces cut.
    pub fn text(&self) -> String {
        let mut out = String::with_capacity(self.cells.len());
        for cell in &self.cells {
            if cell.wide_spacer || cell.leading_wide_spacer {
                continue;
            }
            out.push(cell.ch);
            out.extend(cell.zerowidth.iter());
        }
        out.truncate(out.trim_end_matches(' ').len());
        out
    }
}

fn convert_color(color: GridColor) -> Color {
    match color {
        GridColor::Spec(rgb) => Color::Rgb(Rgb {
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        }),
        GridColor::Indexed(index) => Color::Indexed(index),
        GridColor::Named(named) => match named {
            NamedColor::Black | NamedColor::DimBlack => Color::Indexed(0),
            NamedColor::Red | NamedColor::DimRed => Color::Indexed(1),
            NamedColor::Green | NamedColor::DimGreen => Color::Indexed(2),
            NamedColor::Yellow | NamedColor::DimYellow => Color::Indexed(3),
            NamedColor::Blue | NamedColor::DimBlue => Color::Indexed(4),
            NamedColor::Magenta | NamedColor::DimMagenta => Color::Indexed(5),
            NamedColor::Cyan | NamedColor::DimCyan => Color::Indexed(6),
            NamedColor::White | NamedColor::DimWhite => Color::Indexed(7),
            NamedColor::BrightBlack => Color::Indexed(8),
            NamedColor::BrightRed => Color::Indexed(9),
            NamedColor::BrightGreen => Color::Indexed(10),
            NamedColor::BrightYellow => Color::Indexed(11),
            NamedColor::BrightBlue => Color::Indexed(12),
            NamedColor::BrightMagenta => Color::Indexed(13),
            NamedColor::BrightCyan => Color::Indexed(14),
            NamedColor::BrightWhite => Color::Indexed(15),
            NamedColor::Foreground
            | NamedColor::Background
            | NamedColor::Cursor
            | NamedColor::BrightForeground
            | NamedColor::DimForeground => Color::Default,
        },
    }
}

/// The shape of the cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorShape {
    Block,
    Underline,
    Beam,
    HollowBlock,
    /// The program hid the cursor.
    Hidden,
}

impl From<GridCursorShape> for CursorShape {
    fn from(shape: GridCursorShape) -> Self {
        match shape {
            GridCursorShape::Block => Self::Block,
            GridCursorShape::Underline => Self::Underline,
            GridCursorShape::Beam => Self::Beam,
            GridCursorShape::HollowBlock => Self::HollowBlock,
            GridCursorShape::Hidden => Self::Hidden,
        }
    }
}

/// The cursor. `row` counts from the top of the screen (line 0 is the first line of the live screen, as in
/// [`crate::Terminal::read_lines`]); with the view scrolled back by `display_offset` lines, the cursor is
/// drawn on viewport row `row + display_offset`, if that is on the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub row: i32,
    pub column: usize,
    pub shape: CursorShape,
}

/// The terminal mode bits the UI needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modes {
    /// The alternate screen is on (full-screen programs); it has no scrollback.
    pub alt_screen: bool,
    /// The program asked for bracketed paste (DECSET 2004).
    pub bracketed_paste: bool,
    /// Application cursor keys (DECCKM).
    pub app_cursor: bool,
    /// Application keypad (DECKPAM).
    pub app_keypad: bool,
    /// Mouse reporting of button presses (DECSET 1000).
    pub mouse_click: bool,
    /// Mouse reporting of drags (DECSET 1002).
    pub mouse_drag: bool,
    /// Mouse reporting of all motion (DECSET 1003).
    pub mouse_motion: bool,
    /// SGR mouse encoding (DECSET 1006).
    pub sgr_mouse: bool,
    /// UTF-8 mouse encoding (DECSET 1005).
    pub utf8_mouse: bool,
    /// The wheel is sent as arrow keys on the alternate screen (DECSET 1007).
    pub alternate_scroll: bool,
    /// Focus in and out reports (DECSET 1004).
    pub focus_in_out: bool,
}

impl Default for Modes {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Modes {
    /// The state of a terminal at start (as `Default`, usable in constants): every mode off except
    /// `alternate_scroll`, which `alacritty_terminal` turns on from the start.
    pub const DEFAULT: Self = Self {
        alt_screen: false,
        bracketed_paste: false,
        app_cursor: false,
        app_keypad: false,
        mouse_click: false,
        mouse_drag: false,
        mouse_motion: false,
        sgr_mouse: false,
        utf8_mouse: false,
        alternate_scroll: true,
        focus_in_out: false,
    };

    /// Any of the mouse reporting modes is on.
    pub fn mouse_reporting(&self) -> bool {
        self.mouse_click || self.mouse_drag || self.mouse_motion
    }

    pub(crate) fn from_term(mode: TermMode) -> Self {
        Self {
            alt_screen: mode.contains(TermMode::ALT_SCREEN),
            bracketed_paste: mode.contains(TermMode::BRACKETED_PASTE),
            app_cursor: mode.contains(TermMode::APP_CURSOR),
            app_keypad: mode.contains(TermMode::APP_KEYPAD),
            mouse_click: mode.contains(TermMode::MOUSE_REPORT_CLICK),
            mouse_drag: mode.contains(TermMode::MOUSE_DRAG),
            mouse_motion: mode.contains(TermMode::MOUSE_MOTION),
            sgr_mouse: mode.contains(TermMode::SGR_MOUSE),
            utf8_mouse: mode.contains(TermMode::UTF8_MOUSE),
            alternate_scroll: mode.contains(TermMode::ALTERNATE_SCROLL),
            focus_in_out: mode.contains(TermMode::FOCUS_IN_OUT),
        }
    }
}

/// The state of the screen, without the cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenInfo {
    pub columns: usize,
    /// Lines of the live screen.
    pub rows: usize,
    /// Lines in the scrollback above the live screen.
    pub history_size: usize,
    /// How many lines the view is scrolled back (0 is the bottom, the live screen).
    pub display_offset: usize,
    pub cursor: Cursor,
    pub modes: Modes,
}

/// What kind of clipboard a program wrote to (OSC 52).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardKind {
    /// The system clipboard (`c`).
    Clipboard,
    /// The primary selection (`p` and `s`); systems without one use the clipboard.
    Selection,
}
