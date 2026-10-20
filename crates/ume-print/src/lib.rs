//! Page layout for printing and the print preview of UME Editor ([ADR-0011]): from the lines of a
//! document and Sakura's print settings to pages, each a list of text and its position.
//!
//! It knows nothing of a UI, a font, a printer or an OS. The print preview (GPUI) and the
//! printers of each OS only draw what it gives, and put each piece of text where it says, so
//! that the preview and the printing agree. The wrapping of the lines is done by `ume-layout`
//! ([ADR-0013], decision 2), with the wrap column that follows from the paper, the margins and
//! the size of the cell.
//!
//! ```
//! use ume_print::{HeaderContext, Layout, Paper, PrintSetting, TypeOptions};
//!
//! let lines = ["Hello", "世界", ""]; // "Hello\n世界\n"
//! let layout = Layout::new(&lines, &PrintSetting::default(), Paper::A4, &TypeOptions::default())?;
//! assert_eq!(layout.page_count(), 1);
//! let page = layout.page(0, &lines, &HeaderContext::default()).unwrap();
//! // The header "(無題)" (in three pieces: the parentheses are in the font for half-width
//! // characters), the two lines, and the footer "- 1 -".
//! let pieces: Vec<_> = page.texts.iter().map(|t| t.text.as_str()).collect();
//! assert_eq!(pieces, ["(", "無題", ")", "Hello", "世界", "- 1 -"]);
//! # Ok::<(), ume_print::PrintError>(())
//! ```
//!
//! # What it does
//!
//! - [`PrintSetting`] has Sakura's print settings (`PRINTSETTING`) with its field names,
//!   [`Paper`] and [`PAPER_INFO`] the papers, and [`TypeOptions`] what Sakura takes from the
//!   document type (tab width, kinsoku characters, how lines are numbered).
//! - [`Layout::new`] reads the lines through [`LineSource`], wraps them with `ume-layout` and
//!   finds where each page begins. It keeps no text, so it works for a document of any size;
//!   [`Layout::page`] reads the lines of one page again and gives the [`Page`].
//! - The characters go on a grid of cells: a half-width character takes one cell and a
//!   full-width one two, whatever their widths in the font are (see [`PlacedText`]). Line numbers
//!   are on the first row of each line, with the vertical line after them; the columns (段組)
//!   follow each other down a page; the header and the footer have the variables of Sakura's
//!   `ExpandParameter` ([`expand_parameter`]).
//!
//! # Not done here
//!
//! - Color printing (`color_print`) and the colors of the text. A page holds plain text.
//! - The printer: the paper comes from the caller, and so does the unprintable margin of the
//!   printer, which Sakura takes off every position. The range of pages is picked in the OS
//!   print dialog (Sakura passes `PD_NOSELECTION`: it never prints the selection), so a page
//!   range is a range of [`Layout::page`] indexes; [`Layout::pages`] is the iterator.
//! - The fonts. The faces of the settings are for the drawer, and so is the width of the text
//!   of a header or a footer in a font of its own ([`HAlign`]).
//! - Reading the settings from `sakura.ini` (`[Print]`) and the dialog that edits them: they are
//!   `ume-config` and `ume-ui`.
//!
//! [ADR-0011]: https://github.com/stofu5678/ume_editor/blob/main/docs/adr/0011-printing.md
//! [ADR-0013]: https://github.com/stofu5678/ume_editor/blob/main/docs/adr/0013-module-structure.md

mod header;
mod layout;
mod page;
mod setting;
mod source;

pub use header::{HeaderContext, PageInfo, WindowKind, WriteState, expand_parameter};
pub use layout::{LINENUMWIDTH_MIN, Layout, LineTerm, PrintError, TypeOptions, line_number_cells};
pub use page::{FontKind, HAlign, Page, PlacedText, Rule};
pub use setting::{
    DMPAPER_A3, DMPAPER_A4, DMPAPER_A4SMALL, DMPAPER_A5, DMPAPER_B4, DMPAPER_B5, DMPAPER_EXECUTIVE,
    DMPAPER_FOLIO, DMPAPER_LEDGER, DMPAPER_LEGAL, DMPAPER_LETTER, DMPAPER_LETTERSMALL,
    DMPAPER_QUARTO, DMPAPER_STATEMENT, DMPAPER_TABLOID, HEADER_MAX, HeaderFooterFont, Orientation,
    PAPER_INFO, POS_CENTER, POS_LEFT, POS_RIGHT, Paper, PaperInfo, PrintSetting, find_paper_info,
};
pub use source::LineSource;

#[cfg(test)]
mod tests;
