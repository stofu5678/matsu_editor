//! What Sakura's settings mean for the editor view: font, tab width, wrap, what is shown, and the
//! colors. [`ViewSettings::from_profile`] reads them from the text of a settings file (`ume.ini`, in the format of `sakura.ini`) without a
//! window; a missing or broken value is Sakura's default (`ume-config` does that part).
//!
//! Where each value comes from (the keys are read by `ume-config`, `settings/types.rs` and
//! `settings/color.rs`, which cite `sakura_core/env/CShareData_IO.cpp`):
//!
//! * font: `[Font]` `lf`, `lfFaceName`, `nPointSize`, unless the document type has its own font
//!   (`Types(N)` `bUseTypeFont`).
//! * tab width, wrap method and column, English word wrap: `Types(N)` `nInts` (`tabSpace`,
//!   `maxLineKetas`, `wordWrap`) and `nTextWrapMethod`.
//! * what is shown and the colors: the `C[XXX]` entries of the type. Sakura keeps "show line
//!   numbers", "show tabs", "show full-width spaces", "show line ends" and "show EOF" as the
//!   `disp` flag of `LNO`, `TAB`, `ZEN`, `EOL` and `EOF` (`sakura_core/view/colors/EColorIndexType.h`).
//! * the status bar: `[Common]` `bDispSTATUSBAR`.
//! * the ruler and the vertical lines: `[Common]` `nRulerHeight`, `nRulerBottomSpace` and
//!   `nVertLineOffset`; the columns of the vertical lines and the offset of the note lines are
//!   per type (`nVertLineIdx`, `nNoteLineOffset`).
//! * which type a file has: [`super::doc_type`].

use std::sync::Arc;

use ume_coloring::ColorRules;
use ume_config::Profile;
use ume_config::location::Os;
use ume_config::settings::color::{ColorEntry, ColorRef, names};
use ume_config::settings::types::wrap_method;
use ume_config::settings::{DocumentTypes, ViewFont};
use ume_layout::{LayoutConfig, MAX_LINE_COLUMNS, MIN_LINE_COLUMNS, Wrap};

/// A color as `0xRRGGBB`, which is what GPUI's `rgb()` takes.
pub type Rgb = u32;

/// A Windows `COLORREF` (`0x00BBGGRR`) as `0xRRGGBB`.
pub fn rgb_of(color: ColorRef) -> Rgb {
    let (r, g, b) = (color & 0xff, (color >> 8) & 0xff, (color >> 16) & 0xff);
    (r << 16) | (g << 8) | b
}

/// How the text is wrapped (`m_nTextWrapMethod`, `WRAP_NO_TEXT_WRAP` and so on).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrapMethod {
    /// Do not wrap (折り返さない).
    None,
    /// Wrap at the column of the setting (指定桁で折り返す).
    SettingWidth,
    /// Wrap at the right edge of the window (右端で折り返す).
    WindowWidth,
}

impl WrapMethod {
    fn from_setting(value: i32) -> Self {
        match value {
            wrap_method::SETTING_WIDTH => Self::SettingWidth,
            wrap_method::WINDOW_WIDTH => Self::WindowWidth,
            _ => Self::None,
        }
    }
}

/// Which marks are drawn (the `disp` flag of their colors).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Marks {
    pub tab: bool,
    pub full_width_space: bool,
    pub eol: bool,
    pub eof: bool,
    /// The bookmark color ("MRK") is shown: a bookmarked line gets its colors on the line number.
    /// If not, a bookmark is a short vertical line at the left edge of the line number area
    /// (`CTextDrawer::DispLineNumber`).
    pub bookmark: bool,
    /// The color of modified lines ("MOD") is shown: the line number of a line that differs from
    /// the saved version gets its colors (`CTypeSupport( COLORIDX_GYOU_MOD ).IsDisp()`).
    pub modified: bool,
}

/// The colors of one kind of DIFF mark (追加 "DFA", 変更 "DFC", 削除 "DFD"): the line number of
/// a marked line gets them while `shown` (the `disp` flag of the color,
/// `CDiffLineGetter::GetDiffColor`). The mark itself is drawn whether they are shown or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffColor {
    pub text: Rgb,
    pub background: Rgb,
    pub bold: bool,
    pub shown: bool,
}

impl DiffColor {
    fn of(entry: &ColorEntry) -> Self {
        Self {
            text: rgb_of(entry.text),
            background: rgb_of(entry.back),
            bold: entry.bold,
            shown: entry.disp,
        }
    }
}

/// The search colors that are shown: the first search color, then `FN2` to `FN5`
/// (`CColor_Found`'s `highlightColors`). A match takes one of them by its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoundColors {
    entries: [ColorEntry; 5],
    len: usize,
}

impl FoundColors {
    /// The names of the five search colors.
    const NAMES: [&'static str; 5] = [concat!("FN", "D"), "FN2", "FN3", "FN4", "FN5"];

    /// The colors of `shown` (the entries whose `disp` is on, in order).
    pub fn new(shown: impl IntoIterator<Item = ColorEntry>) -> Self {
        let mut list = shown.into_iter();
        let mut len = 0;
        let first = list.next();
        let mut entries = [first.unwrap_or(NO_ENTRY); 5];
        if first.is_some() {
            len = 1;
            for entry in list.take(4) {
                entries[len] = entry;
                len += 1;
            }
        }
        Self { entries, len }
    }

    /// No search color is shown: matches are not marked at all.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// How many are shown.
    pub fn len(&self) -> usize {
        self.len
    }

    /// The color of the match numbered `slot` (from 0); `None` when none is shown.
    pub fn get(&self, slot: u8) -> Option<ColorEntry> {
        (self.len > 0).then(|| self.entries[usize::from(slot) % self.len])
    }
}

/// A placeholder for the unused places of [`FoundColors`].
const NO_ENTRY: ColorEntry = ColorEntry {
    disp: false,
    bold: false,
    underline: false,
    text: 0,
    back: 0,
};

/// The colors the view paints with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Colors {
    pub text: Rgb,
    pub background: Rgb,
    pub line_number: Rgb,
    pub line_number_background: Rgb,
    pub caret: Rgb,
    /// The background of the selection (see [`selection_colors`]).
    pub selection: Rgb,
    /// The color of selected text; the same as `text` when the selection leaves it alone.
    pub selection_text: Rgb,
    pub tab: Rgb,
    pub full_width_space: Rgb,
    pub eol: Rgb,
    pub eof: Rgb,
    /// The text and the background of the line number of a bookmarked line (color "MRK").
    pub bookmark: Rgb,
    pub bookmark_background: Rgb,
    /// The text and the background of the line number of a modified line (color "MOD").
    pub modified: Rgb,
    pub modified_background: Rgb,
    /// Whether the line numbers of the three kinds are bold (the colors "LNO", "MRK" and "MOD").
    /// Sakura's default makes a modified line bold and nothing else.
    pub line_number_bold: bool,
    pub bookmark_bold: bool,
    pub modified_bold: bool,
    /// The DIFF marks: 追加, 変更 and 削除 (also the 削除 of the end of the file).
    pub diff_append: DiffColor,
    pub diff_change: DiffColor,
    pub diff_delete: DiffColor,
    /// The entries `TXT` and `SEL` as they are: the colors of selected text depend on the color
    /// of the text under it (see [`selection_colors_over`]).
    pub text_entry: ColorEntry,
    pub selection_entry: ColorEntry,
    /// The entries `TAB`, `ZEN` and `EOL`: the marks follow the text around them where they have
    /// the color of the text.
    pub tab_entry: ColorEntry,
    pub full_width_space_entry: ColorEntry,
    pub eol_entry: ColorEntry,
    /// The colors of the matches of the search that are shown (`CColor_Found::OnStartScanLogic`).
    pub found: FoundColors,
    /// The entries of the ruler `RUL`, the underline of the caret line `UND`, the vertical line
    /// of the caret column `CVL`, the note lines `NOT`, the vertical lines at given columns `VER`
    /// and the bracket pair `BRC`: each is drawn only while its `disp` is on.
    pub ruler_entry: ColorEntry,
    pub underline_entry: ColorEntry,
    pub cursor_vline_entry: ColorEntry,
    pub note_line_entry: ColorEntry,
    pub vert_line_entry: ColorEntry,
    pub bracket_entry: ColorEntry,
    /// The background of the row with the caret (`CBK`), if it is shown.
    pub caret_line_back: Option<Rgb>,
    /// The background of the rows with an odd number (`EBK`), if it is shown.
    pub even_line_back: Option<Rgb>,
}

/// The settings of the view for one file.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewSettings {
    /// The index of the document type in the file (0 is "基本").
    pub type_index: usize,
    /// A font family to ask GPUI for, or `None` for the default of the OS.
    pub font_family: Option<String>,
    /// The size in 1/10 point (`nPointSize`).
    pub point_size: i32,
    /// The weight of the font (`lfWeight`; 400 normal, 700 bold). 0 is "don't care", drawn as normal.
    pub font_weight: i32,
    /// The font is italic (`lfItalic`).
    pub font_italic: bool,
    /// The font is a TrueType (outline) font (`lfOutPrecision` is `OUT_STROKE_PRECIS` or
    /// `OUT_PS_ONLY_PRECIS`): `Command_SETFONTSIZE` changes the size of such fonts only.
    pub font_truetype: bool,
    pub tab_width: u32,
    pub word_wrap: bool,
    pub wrap_method: WrapMethod,
    /// The wrap column of [`WrapMethod::SettingWidth`] (`nMaxLineKetas`).
    pub wrap_columns: u32,
    /// The room Sakura keeps right of the wrap column for the wrap mark and for hanging
    /// punctuation (`CEditView::GetWrapOverhang`), in columns.
    pub wrap_overhang: u32,
    pub show_line_numbers: bool,
    pub marks: Marks,
    pub colors: Colors,
    /// The rules of the syntax coloring of the type (comments, strings, numbers, keywords).
    pub coloring: Arc<ColorRules>,
    pub show_status_bar: bool,
    /// `[Common]` `nRulerHeight`: the height of the ruler in pixels, when it is shown.
    pub ruler_height: u32,
    /// `[Common]` `nRulerBottomSpace`: the space between the ruler and the text.
    pub ruler_bottom_space: u32,
    /// `[Common]` `nVertLineOffset`: how far the vertical lines are moved right, in pixels.
    pub vert_line_offset: i32,
    /// `nNoteLineOffset` of the type: how far the note lines are moved down, in pixels.
    pub note_line_offset: i32,
    /// `nVertLineIdx` of the type: the columns of the vertical lines; a negative column starts
    /// a repeating line (see `crate::view::lines::vertical_columns`).
    pub vert_lines: Vec<i32>,
    /// What the outline of the type is made of.
    pub outline: OutlineSettings,
}

/// The settings of the type that the outline pane uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutlineSettings {
    /// `nDefaultOutline` ([`ume_config::settings::type_rules::outline_type`]): the kind of the
    /// analysis of F_OUTLINE.
    pub kind: i32,
    /// `bLineNumIsCRLF`: the pane numbers the lines of the text, not the rows of the wrapped
    /// view.
    pub line_num_is_crlf: bool,
}

/// `m_nRulerHeight` of a new `CommonSetting` (`sakura_core/env/CShareData.cpp`).
const DEFAULT_RULER_HEIGHT: u32 = 13;

/// `m_nVertLineOffset` of a new `CommonSetting`.
const DEFAULT_VERT_LINE_OFFSET: i32 = -1;

/// A size in pixels of `[Common]`: a value below 0 is 0.
fn common_size(profile: &Profile, key: &str, default: u32) -> u32 {
    profile
        .get_i32("Common", key)
        .map_or(default, |v| u32::try_from(v).unwrap_or(0))
}

/// The font size of Sakura's default font, 10 pt (`FontSetting::default`), when the file has none.
const DEFAULT_POINT_SIZE: i32 = 100;

/// The resolution Sakura converts points to pixels at; GPUI's pixels are logical, 96 per inch.
pub const DPI: i32 = 96;

/// Font sizes in 1/10 point to GPUI pixels: `DpiPointsToPixels` (`MulDiv` rounding), at 96 dpi.
pub fn points_to_px(tenths: i32) -> f32 {
    let px = (i64::from(tenths) * i64::from(DPI) + 360) / 720;
    px.max(1) as f32
}

/// The face names that exist on Windows only, as `lfFaceName` has them, and the family name that
/// GPUI (DirectWrite) finds on Windows. The full-width forms are the Japanese names.
const WINDOWS_FONTS: [(&str, &str); 8] = [
    ("ＭＳ ゴシック", "MS Gothic"),
    ("ＭＳ Ｐゴシック", "MS PGothic"),
    ("ＭＳ 明朝", "MS Mincho"),
    ("ＭＳ Ｐ明朝", "MS PMincho"),
    ("MS Gothic", "MS Gothic"),
    ("MS PGothic", "MS PGothic"),
    ("MS Mincho", "MS Mincho"),
    ("MS PMincho", "MS PMincho"),
];

/// The family to ask GPUI for a face name of the settings file: the name itself, except that the
/// fonts that come with Windows only are renamed on Windows and left to the OS's default
/// elsewhere. `None` is the default font of the program.
pub fn font_family_for(face: &str, os: Os) -> Option<String> {
    let face = face.trim();
    if face.is_empty() {
        return None;
    }
    match WINDOWS_FONTS.iter().find(|(name, _)| *name == face) {
        Some((_, family)) if os == Os::Windows => Some((*family).to_owned()),
        Some(_) => None,
        None => Some(face.to_owned()),
    }
}

/// `GetBackColorByColorInfo2` (`sakura_core/view/CEditView_Paint.cpp`) for the selection: a
/// selection color whose text and background are the same means "mix": the background of the
/// window turned inside out if the selection color is the window background, else 30% of the
/// selection color over the background that is under it. Different text and background colors
/// are used as they are.
pub fn selection_background(
    selection: &ColorEntry,
    window_background: ColorRef,
    under: ColorRef,
) -> ColorRef {
    if selection.text != selection.back {
        return selection.back;
    }
    if selection.back == window_background {
        return under ^ 0x00ff_ffff;
    }
    mix(selection.back, under, 255 * 30 / 100)
}

/// `GetTextColorByColorInfo2`: the same rule for the text color of the selection.
pub fn selection_text(
    selection: &ColorEntry,
    window_background: ColorRef,
    under: ColorRef,
) -> ColorRef {
    if selection.text != selection.back {
        return selection.text;
    }
    if selection.back == window_background {
        return under ^ 0x00ff_ffff;
    }
    mix(selection.text, under, 255 * 30 / 100)
}

/// The text and background colors of selected text, given the entry `SEL`, the text entry
/// `TXT` and the background that is under the selection.
///
/// With `SEL` shown (`disp`), Sakura paints the selection with its own colors by the two rules
/// above (`CEditView::GetTextColorByColorInfo2` and `GetBackColorByColorInfo2`, applied over
/// the colors of the text under it). With `SEL` not shown it inverts the screen area
/// (`CViewSelect::DrawSelectArea`, `bDispText`); here both colors are inverted.
pub fn selection_colors(selection: &ColorEntry, text: &ColorEntry) -> (ColorRef, ColorRef) {
    selection_colors_over(selection, text.back, text.text, text.back)
}

/// The same for selected text over text of another color: `under_text` and `under_back` are the
/// colors the text has without the selection (the entry of the strategy that colors it, and its
/// background, or the background of the row where that is the window's).
/// `window_background` is the background of `TXT`.
pub fn selection_colors_over(
    selection: &ColorEntry,
    window_background: ColorRef,
    under_text: ColorRef,
    under_back: ColorRef,
) -> (ColorRef, ColorRef) {
    if !selection.disp {
        return (under_text ^ 0x00ff_ffff, under_back ^ 0x00ff_ffff);
    }
    (
        selection_text(selection, window_background, under_text),
        selection_background(selection, window_background, under_back),
    )
}

/// `MakeColor2`: `(a * alpha + b * (256 - alpha)) / 256` for each of R, G and B.
fn mix(a: ColorRef, b: ColorRef, alpha: u32) -> ColorRef {
    let channel = |shift: u32| {
        let (x, y) = ((a >> shift) & 0xff, (b >> shift) & 0xff);
        ((x * alpha + y * (256 - alpha)) / 256) << shift
    };
    channel(0) | channel(8) | channel(16)
}

impl Default for ViewSettings {
    /// Sakura's defaults for a new document, as without a settings file: the basic type of the
    /// built-in types.
    fn default() -> Self {
        Self::from_types(&DocumentTypes::builtin(), 0, &Profile::new(), Os::current())
    }
}

impl ViewSettings {
    /// The settings for a file called `file_name` (the name only, no folder; `None` for a new
    /// document, which is the basic type), from the contents of `profile`.
    pub fn from_profile(profile: &Profile, file_name: Option<&str>, os: Os) -> Self {
        let types = DocumentTypes::from_profile(profile);
        let type_index = file_name.map_or(0, |name| types.type_index_for_file_name(name));
        Self::from_types(&types, type_index, profile, os)
    }

    /// The settings of type `type_index` of `types` (the basic type for an index that does not
    /// exist). `profile` has what is not per type: the common font and the status bar.
    pub fn from_types(types: &DocumentTypes, type_index: usize, profile: &Profile, os: Os) -> Self {
        let type_index = if type_index < types.len() {
            type_index
        } else {
            0
        };
        let t = types
            .get(type_index)
            .expect("the document types have type 0");

        // The font of the type, if it has its own, else the common one.
        let view_font = ViewFont::read(profile);
        let font = if t.use_type_font {
            &t.font
        } else {
            &view_font.font
        };
        let has_face = if t.use_type_font {
            true
        } else {
            profile
                .get(ViewFont::SECTION, "lfFaceName")
                .is_some_and(|f| !f.is_empty())
        };
        let font_family = has_face
            .then(|| font_family_for(&font.log_font.face_name, os))
            .flatten();
        let point_size = match font.point_size_for_dpi(DPI) {
            n if n > 0 => n,
            _ => DEFAULT_POINT_SIZE,
        };

        let max_columns = u32::try_from(t.max_line_ketas)
            .unwrap_or(MAX_LINE_COLUMNS)
            .clamp(MIN_LINE_COLUMNS, MAX_LINE_COLUMNS);

        // `GetWrapOverhang`: one column for the wrap mark; unless hanging punctuation is hidden,
        // one more for a line break that hangs and two for punctuation that hangs.
        let mut overhang = 1;
        if !t.kinsoku_hide {
            if t.kinsoku_ret {
                overhang += 1;
            }
            if t.kinsoku_kuto_enabled {
                overhang += 2;
            }
        }

        let c = &t.colors;
        let entry = |name: &str| *c.get(name).expect("a color of Sakura's table");
        let text = entry(names::TEXT);
        let line_number = entry(names::LINE_NUMBER);
        let selection = entry("SEL");
        let caret = entry(names::CARET);
        let tab = entry(names::TAB);
        let zen = entry(names::FULL_SPACE);
        let eol = entry(names::EOL);
        let eof = entry(names::EOF);
        let bookmark = entry(names::BOOKMARK);
        let modified = entry(names::LINE_NUMBER_MODIFIED);
        let (diff_append, diff_change, diff_delete) = (entry("DFA"), entry("DFC"), entry("DFD"));
        let found = FoundColors::new(FoundColors::NAMES.into_iter().map(entry).filter(|e| e.disp));
        let shown_back = |name: &str| Some(entry(name)).filter(|e| e.disp).map(|e| rgb_of(e.back));

        ViewSettings {
            type_index,
            font_family,
            point_size,
            font_weight: font.log_font.weight,
            font_italic: font.log_font.italic != 0,
            font_truetype: crate::font_dialog::model::is_truetype(font.log_font.out_precision),
            tab_width: u32::try_from(t.tab_space).unwrap_or(4).max(1),
            word_wrap: t.word_wrap,
            wrap_method: WrapMethod::from_setting(t.text_wrap_method),
            wrap_columns: max_columns,
            wrap_overhang: overhang,
            show_line_numbers: line_number.disp,
            marks: Marks {
                tab: tab.disp,
                full_width_space: zen.disp,
                eol: eol.disp,
                eof: eof.disp,
                bookmark: bookmark.disp,
                modified: modified.disp,
            },
            colors: Colors {
                text: rgb_of(text.text),
                background: rgb_of(text.back),
                line_number: rgb_of(line_number.text),
                line_number_background: rgb_of(line_number.back),
                caret: rgb_of(caret.text),
                selection: rgb_of(selection_colors(&selection, &text).1),
                selection_text: rgb_of(selection_colors(&selection, &text).0),
                tab: rgb_of(tab.text),
                full_width_space: rgb_of(zen.text),
                eol: rgb_of(eol.text),
                eof: rgb_of(eof.text),
                bookmark: rgb_of(bookmark.text),
                bookmark_background: rgb_of(bookmark.back),
                modified: rgb_of(modified.text),
                modified_background: rgb_of(modified.back),
                line_number_bold: line_number.bold,
                bookmark_bold: bookmark.bold,
                modified_bold: modified.bold,
                diff_append: DiffColor::of(&diff_append),
                diff_change: DiffColor::of(&diff_change),
                diff_delete: DiffColor::of(&diff_delete),
                text_entry: text,
                selection_entry: selection,
                tab_entry: tab,
                full_width_space_entry: zen,
                eol_entry: eol,
                found,
                ruler_entry: entry("RUL"),
                underline_entry: entry("UND"),
                cursor_vline_entry: entry("CVL"),
                note_line_entry: entry("NOT"),
                vert_line_entry: entry(names::VERT_LINE),
                bracket_entry: entry(names::BRACKET_PAIR),
                caret_line_back: shown_back("CBK"),
                even_line_back: shown_back("EBK"),
            },
            coloring: Arc::new(ColorRules::from_type(t, types.keywords())),
            show_status_bar: profile.get_bool("Common", "bDispSTATUSBAR").unwrap_or(true),
            ruler_height: common_size(profile, "nRulerHeight", DEFAULT_RULER_HEIGHT),
            ruler_bottom_space: common_size(profile, "nRulerBottomSpace", 0),
            vert_line_offset: profile
                .get_i32("Common", "nVertLineOffset")
                .unwrap_or(DEFAULT_VERT_LINE_OFFSET),
            note_line_offset: t.note_line_offset,
            vert_lines: t
                .vert_line_idx
                .iter()
                .copied()
                .take_while(|&column| column != 0)
                .collect(),
            outline: OutlineSettings {
                kind: t.default_outline,
                line_num_is_crlf: t.line_num_is_crlf,
            },
        }
    }

    /// The height of the ruler, 0 when it is not shown (`COLORIDX_RULER`'s `disp`).
    pub fn ruler_px(&self) -> u32 {
        if self.colors.ruler_entry.disp {
            self.ruler_height
        } else {
            0
        }
    }

    /// The space above the first row of text: the ruler, if shown, and the space below it
    /// (`CTextArea::GetAreaTop`).
    pub fn top_margin(&self) -> u32 {
        self.ruler_px() + self.ruler_bottom_space
    }

    /// The font size in GPUI pixels.
    pub fn font_px(&self) -> f32 {
        points_to_px(self.point_size)
    }

    /// The weight GPUI draws the text with: `lfWeight`, and normal for 0 (`FW_DONTCARE`).
    pub fn gpui_font_weight(&self) -> f32 {
        if self.font_weight > 0 {
            self.font_weight as f32
        } else {
            400.0
        }
    }

    /// The wrap the layout starts with, for a view `window_columns` columns wide.
    pub fn wrap(&self, window_columns: u32) -> Wrap {
        wrap_for(
            self.wrap_method,
            self.wrap_columns,
            window_columns,
            self.wrap_overhang,
        )
    }

    /// The layout settings of this type for a view `window_columns` columns wide.
    pub fn layout_config(&self, window_columns: u32) -> LayoutConfig {
        LayoutConfig {
            wrap: self.wrap(window_columns),
            tab_width: self.tab_width,
            word_wrap: self.word_wrap,
            ..LayoutConfig::default()
        }
    }
}

/// The wrap of a method: none, the column of the setting, or the width of the view
/// (`CEditView::ViewColNumToWrapColNum`: the columns of the view without the overhang, and not
/// less than `MINLINEKETAS`).
pub fn wrap_for(
    method: WrapMethod,
    setting_columns: u32,
    window_columns: u32,
    overhang: u32,
) -> Wrap {
    match method {
        WrapMethod::None => Wrap::None,
        WrapMethod::SettingWidth => Wrap::Columns(setting_columns),
        WrapMethod::WindowWidth => Wrap::Columns(window_wrap_columns(window_columns, overhang)),
    }
}

/// `ViewColNumToWrapColNum`.
pub fn window_wrap_columns(window_columns: u32, overhang: u32) -> u32 {
    window_columns
        .saturating_sub(overhang)
        .clamp(MIN_LINE_COLUMNS, MAX_LINE_COLUMNS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(text: &str) -> Profile {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        Profile::from_bytes(&bytes)
    }

    fn settings(text: &str, file: Option<&str>, os: Os) -> ViewSettings {
        ViewSettings::from_profile(&profile(text), file, os)
    }

    #[test]
    fn without_a_file_the_values_are_sakuras_defaults() {
        let s = ViewSettings::default();
        assert_eq!(s.type_index, 0);
        assert_eq!(s.tab_width, 4);
        assert_eq!(s.wrap_method, WrapMethod::None);
        assert_eq!(s.wrap_columns, 10240);
        assert!(s.show_line_numbers);
        assert!(s.show_status_bar);
        assert_eq!(
            s.marks,
            Marks {
                tab: true,
                full_width_space: true,
                eol: true,
                eof: true,
                bookmark: true,
                modified: true
            }
        );
        // `ColorInfo_DEFAULT`: black text on (255, 251, 240).
        assert_eq!(s.colors.text, 0x000000);
        assert_eq!(s.colors.background, 0xfffbf0);
        assert_eq!(s.colors.line_number, 0x0000ff);
        assert_eq!(s.colors.line_number_background, 0xefefef);
        // the bookmark color: (255, 251, 240) on (0, 128, 192)
        assert_eq!(s.colors.bookmark, 0xfffbf0);
        assert_eq!(s.colors.bookmark_background, 0x0080c0);
        // the color of modified lines: (0, 0, 255) on (239, 239, 239), as the line number
        assert_eq!(s.colors.modified, 0x0000ff);
        assert_eq!(s.colors.modified_background, 0xefefef);
        // and bold, which is all that tells a modified line from the others by default
        assert!(s.colors.modified_bold);
        assert!(!s.colors.line_number_bold);
        assert!(!s.colors.bookmark_bold);
        // Sakura's default font has no point size, only the height -13: 13 * 72 / 96 = 9.75 pt, rounded.
        assert_eq!(s.point_size, 98);
        assert_eq!(s.font_px(), 13.0);
        assert_eq!(s.font_family, None);
    }

    #[test]
    fn colorref_is_blue_green_red() {
        assert_eq!(rgb_of(0x00ff_0000), 0x0000ff);
        assert_eq!(rgb_of(0x0000_00ff), 0xff0000);
        assert_eq!(rgb_of(0x0011_2233), 0x332211);
    }

    #[test]
    fn points_become_pixels_at_96_dpi() {
        assert_eq!(points_to_px(100), 13.0); // 10 pt = 13.33 px
        assert_eq!(points_to_px(120), 16.0); // 12 pt
        assert_eq!(points_to_px(10), 1.0);
        assert_eq!(points_to_px(720), 96.0);
    }

    const BASIC: &str = "[Types(0)]\r\n\
        nInts=0,60,0,8,-1,-1,0,1,1,1,0,0\r\n\
        nTextWrapMethod=1\r\n\
        C[LNO]=0,0,ff0000,efefef,0\r\n\
        C[TAB]=0,0,808080,fffbf0,0\r\n\
        C[TXT]=1,0,ffffff,000000,0\r\n\
        [Font]\r\n\
        nPointSize=140\r\n\
        lf=-19,0,0,0,400,0,0,0,128,3,2,1,49\r\n\
        lfFaceName=ＭＳ ゴシック\r\n\
        [Common]\r\n\
        bDispSTATUSBAR=0\r\n";

    #[test]
    fn the_basic_type_sets_tab_wrap_marks_and_colors() {
        let s = settings(BASIC, None, Os::Windows);
        assert_eq!(s.tab_width, 8);
        assert_eq!(s.wrap_method, WrapMethod::SettingWidth);
        assert_eq!(s.wrap_columns, 60);
        assert!(s.word_wrap);
        assert!(!s.show_line_numbers);
        assert!(!s.marks.tab);
        assert!(s.marks.full_width_space);
        // C[TXT] is "disp,bold,text,back,underline" and the colors are 0xBBGGRR.
        assert_eq!(s.colors.text, 0xffffff);
        assert_eq!(s.colors.background, 0x000000);
        assert!(!s.show_status_bar);
        assert_eq!(s.point_size, 140);
        assert_eq!(s.font_px(), 19.0);
    }

    #[test]
    fn the_wrap_follows_the_method() {
        let s = settings(BASIC, None, Os::Windows);
        assert_eq!(s.wrap(100), Wrap::Columns(60));
        let layout = s.layout_config(100);
        assert_eq!(layout.tab_width, 8);
        assert!(layout.word_wrap);
        assert_eq!(wrap_for(WrapMethod::None, 60, 100, 1), Wrap::None);
        // The view's 100 columns, less the wrap mark's column.
        assert_eq!(
            wrap_for(WrapMethod::WindowWidth, 60, 100, 1),
            Wrap::Columns(99)
        );
        // A narrow window wraps at the smallest column (`MINLINEKETAS`).
        assert_eq!(
            wrap_for(WrapMethod::WindowWidth, 60, 4, 1),
            Wrap::Columns(MIN_LINE_COLUMNS)
        );
    }

    #[test]
    fn broken_values_fall_back_to_the_defaults() {
        let s = settings(
            "[Types(0)]\r\nnInts=1,2,3\r\nC[TXT]=zzz\r\n[Common]\r\nbDispSTATUSBAR=maybe\r\n",
            None,
            Os::Windows,
        );
        let d = ViewSettings::default();
        assert_eq!(s.tab_width, d.tab_width);
        assert_eq!(s.colors, d.colors);
        assert!(s.show_status_bar);
    }

    #[test]
    fn the_type_is_chosen_by_the_extension_and_has_its_own_values() {
        let text = "[Other]\r\nnTypesCount=3\r\n\
            [Types(0)]\r\nnInts=0,10240,0,4,-1,-1,0,1,1,0,0,0\r\n\
            [Types(1)]\r\nszTypeExts=txt\r\nnInts=1,10240,0,2,-1,-1,0,1,1,0,0,0\r\n\
            [Types(2)]\r\nszTypeExts=c cpp\r\nnInts=2,10240,0,8,-1,-1,0,1,1,0,0,0\r\n";
        let tab_of = |file| settings(text, file, Os::Windows).tab_width;
        assert_eq!(tab_of(None), 4);
        assert_eq!(tab_of(Some("a.rs")), 4);
        assert_eq!(tab_of(Some("a.txt")), 2);
        assert_eq!(tab_of(Some("A.CPP")), 8);
        assert_eq!(settings(text, Some("a.c"), Os::Windows).type_index, 2);
    }

    #[test]
    fn a_type_with_its_own_font_uses_it() {
        let text = "[Font]\r\nnPointSize=100\r\nlf=-13,0,0,0,400,0,0,0,128,3,2,1,49\r\nlfFaceName=Courier New\r\n\
            [Types(0)]\r\nbUseTypeFont=1\r\nnPointSize=160\r\nlf=-21,0,0,0,400,0,0,0,0,3,2,1,49\r\nlfFaceName=Consolas\r\n";
        let s = settings(text, None, Os::Linux);
        assert_eq!(s.point_size, 160);
        assert_eq!(s.font_family.as_deref(), Some("Consolas"));
        let s = settings(
            &text.replace("bUseTypeFont=1", "bUseTypeFont=0"),
            None,
            Os::Linux,
        );
        assert_eq!(s.point_size, 100);
        assert_eq!(s.font_family.as_deref(), Some("Courier New"));
    }

    #[test]
    fn windows_only_fonts_are_renamed_on_windows_and_dropped_elsewhere() {
        assert_eq!(
            font_family_for("ＭＳ ゴシック", Os::Windows).as_deref(),
            Some("MS Gothic")
        );
        assert_eq!(font_family_for("ＭＳ ゴシック", Os::MacOs), None);
        assert_eq!(font_family_for("ＭＳ 明朝", Os::Linux), None);
        assert_eq!(
            font_family_for("BIZ UDGothic", Os::MacOs).as_deref(),
            Some("BIZ UDGothic")
        );
        assert_eq!(font_family_for("  ", Os::Windows), None);
    }

    #[test]
    fn a_file_without_a_font_face_keeps_the_programs_font() {
        assert_eq!(
            settings("[Font]\r\nnPointSize=120\r\n", None, Os::Windows).font_family,
            None
        );
    }

    #[test]
    fn the_selection_color_is_mixed_the_way_sakura_does() {
        // Default SEL: text == back == (49, 106, 197): 30% of it over the window background.
        let sel = ColorEntry {
            disp: true,
            bold: false,
            underline: false,
            text: 0x00c5_6a31,
            back: 0x00c5_6a31,
        };
        let under = 0x00f0_fbff; // (255, 251, 240)
        let mixed = selection_background(&sel, under, under);
        // 76 / 256 of (49,106,197) and 180 / 256 of (255,251,240): (49*76 + 255*180) / 256 = 190.
        assert_eq!(mixed & 0xff, (49 * 76 + 255 * 180) / 256);
        assert_eq!((mixed >> 8) & 0xff, (106 * 76 + 251 * 180) / 256);
        assert_eq!((mixed >> 16) & 0xff, (197 * 76 + 240 * 180) / 256);
        // Different text and back colors are used as they are.
        let own = ColorEntry {
            text: 0x1,
            back: 0x00aa_bbcc,
            ..sel
        };
        assert_eq!(selection_background(&own, under, under), 0x00aa_bbcc);
        // The selection color is the window background: turn the text area inside out.
        let same = ColorEntry {
            text: under,
            back: under,
            ..sel
        };
        assert_eq!(
            selection_background(&same, under, under),
            under ^ 0x00ff_ffff
        );
    }

    fn entry(disp: bool, text: ColorRef, back: ColorRef) -> ColorEntry {
        ColorEntry {
            disp,
            bold: false,
            underline: false,
            text,
            back,
        }
    }

    #[test]
    fn a_selection_with_its_own_colors_keeps_both() {
        // White on black over black text on white: the text must not stay black.
        let (text, back) = selection_colors(
            &entry(true, 0x00ff_ffff, 0x0000_0000),
            &entry(true, 0x0000_0000, 0x00ff_ffff),
        );
        assert_eq!((text, back), (0x00ff_ffff, 0x0000_0000));
    }

    #[test]
    fn a_selection_without_distinct_colors_mixes_the_text_color_too() {
        let sel = entry(true, 0x00c5_6a31, 0x00c5_6a31);
        let (text, _) = selection_colors(&sel, &entry(true, 0, 0x00f0_fbff));
        // 30% of the selection color over black text: (49*76/256, ...).
        assert_eq!(text & 0xff, 49 * 76 / 256);
        assert_eq!((text >> 16) & 0xff, 197 * 76 / 256);
    }

    #[test]
    fn a_hidden_selection_color_inverts() {
        let (text, back) =
            selection_colors(&entry(false, 1, 2), &entry(true, 0x0000_0000, 0x00ff_ffff));
        assert_eq!((text, back), (0x00ff_ffff, 0x0000_0000));
    }

    #[test]
    fn the_ruler_and_the_lines_have_sakuras_defaults() {
        let s = ViewSettings::default();
        // RUL and UND are shown; CVL, NOT, VER and BRC are not
        assert!(s.colors.ruler_entry.disp && s.colors.underline_entry.disp);
        assert!(!s.colors.cursor_vline_entry.disp);
        assert!(!s.colors.note_line_entry.disp);
        assert!(!s.colors.vert_line_entry.disp);
        assert!(!s.colors.bracket_entry.disp);
        // `sWindow.m_nRulerHeight = 13`, no space under it, the vertical lines move 1 pixel left
        assert_eq!((s.ruler_height, s.ruler_bottom_space), (13, 0));
        assert_eq!(s.vert_line_offset, -1);
        assert_eq!(s.note_line_offset, 0);
        assert!(s.vert_lines.is_empty());
        assert_eq!((s.ruler_px(), s.top_margin()), (13, 13));
    }

    #[test]
    fn the_common_section_sets_the_ruler_and_the_vertical_line_offset() {
        let s = settings(
            "[Common]\r\nnRulerHeight=20\r\nnRulerBottomSpace=4\r\nnVertLineOffset=2\r\n",
            None,
            Os::Windows,
        );
        assert_eq!((s.ruler_height, s.ruler_bottom_space), (20, 4));
        assert_eq!(s.vert_line_offset, 2);
        assert_eq!((s.ruler_px(), s.top_margin()), (20, 24));
        // a size below 0 is 0
        let s = settings("[Common]\r\nnRulerHeight=-5\r\n", None, Os::Windows);
        assert_eq!(s.ruler_height, 0);
    }

    #[test]
    fn a_hidden_ruler_takes_no_room_but_its_space_stays() {
        let s = settings(
            "[Common]\r\nnRulerBottomSpace=4\r\n[Types(0)]\r\nC[RUL]=0,0,000000,efefef,0\r\n",
            None,
            Os::Windows,
        );
        assert_eq!((s.ruler_px(), s.top_margin()), (0, 4));
    }

    #[test]
    fn the_type_sets_the_vertical_lines_and_the_note_line_offset() {
        let s = settings(
            "[Types(0)]\r\nnVertLineIdx1=10\r\nnVertLineIdx2=-20\r\nnVertLineIdx3=60\r\n\
             nVertLineIdx4=10\r\nnVertLineIdx5=0\r\nnVertLineIdx6=99\r\nnNoteLineOffset=3\r\n\
             C[VER]=1,1,808080,fffbf0,1\r\n",
            None,
            Os::Windows,
        );
        // the list ends at the first 0
        assert_eq!(s.vert_lines, [10, -20, 60, 10]);
        assert_eq!(s.note_line_offset, 3);
        let ver = s.colors.vert_line_entry;
        assert!(ver.disp && ver.bold && ver.underline);
    }

    #[test]
    fn the_search_colors_that_are_shown_are_taken_in_order() {
        let found = |names: &[&str]| {
            let mut ini = String::from("[Types(0)]\r\n");
            for (i, name) in [concat!("FN", "D"), "FN2", "FN3", "FN4", "FN5"]
                .into_iter()
                .enumerate()
            {
                let disp = u8::from(names.contains(&name));
                ini.push_str(&format!("C[{name}]={disp},0,0000{i:02x},ffffff,0\r\n"));
            }
            settings(&ini, None, Os::Windows).colors.found
        };
        let none = found(&[]);
        assert!(none.is_empty());
        assert_eq!(none.get(0), None);
        let some = found(&["FN2", "FN4"]);
        assert_eq!(some.len(), 2);
        // the match numbered 0 and 2 have the first shown color, 1 the second
        assert_eq!(some.get(0), some.get(2));
        assert_ne!(some.get(0), some.get(1));
        let all = found(&[concat!("FN", "D"), "FN2", "FN3", "FN4", "FN5"]);
        assert_eq!(all.len(), 5);
        assert_eq!(all.get(5), all.get(0));
    }
}
