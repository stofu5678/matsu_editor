// Derived from Sakura Editor (zlib License):
//   sakura_core/print/CPrint.h (PRINTSETTING, PAPER_INFO)
//   sakura_core/print/CPrint.cpp (SettingInitialize, m_paperInfoArr, GetPaperSize)
//   sakura_core/dlg/CDlgPrintSetting.cpp (GetData, sDataRange, DataCheckAndCorrect)
// Modified: rewritten in Rust, without the printer (DEVMODE), the fonts of the header and the
// footer other than their face and size, and the dialog.
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

//! The print settings (印刷ページ設定) and the paper.

/// The most characters a part of the header or the footer may have (`HEADER_MAX`, which counts
/// the terminating NUL: the dialog limits the edit boxes to 99 characters).
pub const HEADER_MAX: usize = 100;

/// The left, center and right part of the header or the footer (`POS_LEFT`, `POS_CENTER`,
/// `POS_RIGHT`).
pub const POS_LEFT: usize = 0;
pub const POS_CENTER: usize = 1;
pub const POS_RIGHT: usize = 2;

/// The direction of the paper (`m_nPrintPaperOrientation`: `DMORIENT_PORTRAIT` 1 and
/// `DMORIENT_LANDSCAPE` 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

/// The font of the header or the footer (`m_lfHeader` and `m_nHeaderPointSize`). Sakura keeps a
/// whole `LOGFONT`; only the face and the size matter to the layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderFooterFont {
    /// `lfFaceName`. Sakura takes an empty name as "no font set".
    pub face: String,
    /// The size in 1/10 pt (`m_nHeaderPointSize`, which the font dialog gives).
    pub point_size: i32,
}

impl HeaderFooterFont {
    /// The height of the characters in 0.1 mm (`pt * 254 / 720`, CPrint::CalcHeaderHeight).
    pub fn height(&self) -> i32 {
        self.point_size * 254 / 720
    }
}

/// Sakura's print settings (`PRINTSETTING` in `sakura_core/print/CPrint.h`). The names follow
/// Sakura's, so that a later change can read them from `sakura.ini`.
///
/// Lengths are in 0.1 mm, as in Sakura (including the margins, which the dialog shows in mm and
/// multiplies by 10). Left out: the printer (`m_mdmDevMode`, which the caller turns into a
/// [`crate::Paper`]) and the `m_bHeaderUse` / `m_bFooterUse` flags, which Sakura's layout never
/// reads (it takes an empty form as unused).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrintSetting {
    /// The name of the setting (`m_szPrintSettingName`, `$Q` in the header).
    pub name: String,
    /// The font for half-width characters (`m_szPrintFontFaceHan`).
    pub print_font_face_han: String,
    /// The font for the other characters (`m_szPrintFontFaceZen`).
    pub print_font_face_zen: String,
    /// The width of a half-width character, which is the width of a cell
    /// (`m_nPrintFontWidth`).
    pub print_font_width: i32,
    /// The height of the characters (`m_nPrintFontHeight`).
    pub print_font_height: i32,
    /// Columns on a page, 段組 (`m_nPrintDansuu`).
    pub print_dansuu: i32,
    /// The space between columns (`m_nPrintDanSpace`).
    pub print_dan_space: i32,
    /// The space between lines, in percent of the font height (`m_nPrintLineSpacing`).
    pub print_line_spacing: i32,
    /// The margin at the top (`m_nPrintMarginTY`).
    pub print_margin_ty: i32,
    /// The margin at the bottom (`m_nPrintMarginBY`).
    pub print_margin_by: i32,
    /// The margin at the left (`m_nPrintMarginLX`).
    pub print_margin_lx: i32,
    /// The margin at the right (`m_nPrintMarginRX`).
    pub print_margin_rx: i32,
    /// `m_nPrintPaperOrientation`.
    pub print_paper_orientation: Orientation,
    /// The paper as a `DMPAPER_*` id (`m_nPrintPaperSize`), see [`crate::PAPER_INFO`].
    pub print_paper_size: i16,
    /// Prints in color (`m_bColorPrint`). The layout does not color anything; the drawer does.
    pub color_print: bool,
    /// Breaks a line before an English word rather than in it (`m_bPrintWordWrap`).
    pub print_word_wrap: bool,
    /// 行頭禁則 (`m_bPrintKinsokuHead`).
    pub print_kinsoku_head: bool,
    /// 行末禁則 (`m_bPrintKinsokuTail`).
    pub print_kinsoku_tail: bool,
    /// 改行文字のぶら下げ (`m_bPrintKinsokuRet`).
    pub print_kinsoku_ret: bool,
    /// 句読点のぶら下げ (`m_bPrintKinsokuKuto`).
    pub print_kinsoku_kuto: bool,
    /// Prints the line numbers (`m_bPrintLineNumber`).
    pub print_line_number: bool,
    /// The left, center and right parts of the header, with `$f` and the like
    /// (`m_szHeaderForm`, see [`crate::expand_parameter`]). An empty part is not printed.
    pub header_form: [String; 3],
    /// The same for the footer (`m_szFooterForm`).
    pub footer_form: [String; 3],
    /// The font of the header, or none for the print font (`m_lfHeader`).
    pub header_font: Option<HeaderFooterFont>,
    /// The font of the footer, or none for the print font (`m_lfFooter`).
    pub footer_font: Option<HeaderFooterFont>,
}

impl Default for PrintSetting {
    /// Sakura's defaults (`CPrint::SettingInitialize`).
    fn default() -> Self {
        Self {
            name: String::new(),
            print_font_face_han: "ＭＳ 明朝".into(),
            print_font_face_zen: "ＭＳ 明朝".into(),
            print_font_width: 12,
            print_font_height: 24,
            print_dansuu: 1,
            print_dan_space: 70,
            print_line_spacing: 30,
            print_margin_ty: 100,
            print_margin_by: 200,
            print_margin_lx: 200,
            print_margin_rx: 100,
            print_paper_orientation: Orientation::Portrait,
            print_paper_size: DMPAPER_A4,
            color_print: false,
            print_word_wrap: true,
            print_kinsoku_head: false,
            print_kinsoku_tail: false,
            print_kinsoku_ret: false,
            print_kinsoku_kuto: false,
            print_line_number: false,
            header_form: ["$f".into(), String::new(), String::new()],
            footer_form: [String::new(), "- $p -".into(), String::new()],
            header_font: None,
            footer_font: None,
        }
    }
}

impl PrintSetting {
    /// The setting as the print setting dialog leaves it (`CDlgPrintSetting::GetData`): the
    /// numbers are kept in their ranges (`sDataRange`), the margins and the column space are
    /// whole mm, and the width of a character is half of its height, rounded up.
    ///
    /// Settings read from `sakura.ini` are not corrected by Sakura until the dialog opens, so
    /// the layout does not do it by itself either.
    pub fn normalized(&self) -> Self {
        let mut s = self.clone();
        s.print_font_height = s.print_font_height.clamp(7, 200);
        s.print_font_width = (s.print_font_height + 1) / 2;
        s.print_line_spacing = s.print_line_spacing.clamp(0, 150);
        s.print_dansuu = s.print_dansuu.clamp(1, 4);
        let mm = |v: i32, max: i32| (v / 10).clamp(0, max) * 10;
        s.print_dan_space = mm(s.print_dan_space, 30);
        s.print_margin_ty = mm(s.print_margin_ty, 50);
        s.print_margin_by = mm(s.print_margin_by, 50);
        s.print_margin_lx = mm(s.print_margin_lx, 50);
        s.print_margin_rx = mm(s.print_margin_rx, 50);
        for part in s.header_form.iter_mut().chain(s.footer_form.iter_mut()) {
            if let Some((i, _)) = part.char_indices().nth(HEADER_MAX - 1) {
                part.truncate(i);
            }
        }
        s
    }

    /// The height of the header with its line spacing, or 0 when all its parts are empty
    /// (`CPrint::CalcHeaderHeight`).
    pub fn header_height(&self) -> i32 {
        self.form_height(&self.header_form, self.header_font.as_ref())
    }

    /// The same for the footer (`CPrint::CalcFooterHeight`).
    pub fn footer_height(&self) -> i32 {
        self.form_height(&self.footer_form, self.footer_font.as_ref())
    }

    fn form_height(&self, form: &[String; 3], font: Option<&HeaderFooterFont>) -> i32 {
        if form.iter().all(String::is_empty) {
            return 0;
        }
        let height = match font {
            Some(f) if !f.face.is_empty() => f.height(),
            _ => self.print_font_height,
        };
        // The line spacing is added.
        height * (self.print_line_spacing + 100) / 100
    }
}

// The ids of some of the papers (`DMPAPER_*` in wingdi.h), which `m_nPrintPaperSize` holds.
pub const DMPAPER_LETTER: i16 = 1;
pub const DMPAPER_LETTERSMALL: i16 = 2;
pub const DMPAPER_TABLOID: i16 = 3;
pub const DMPAPER_LEDGER: i16 = 4;
pub const DMPAPER_LEGAL: i16 = 5;
pub const DMPAPER_STATEMENT: i16 = 6;
pub const DMPAPER_EXECUTIVE: i16 = 7;
pub const DMPAPER_A3: i16 = 8;
pub const DMPAPER_A4: i16 = 9;
pub const DMPAPER_A4SMALL: i16 = 10;
pub const DMPAPER_A5: i16 = 11;
pub const DMPAPER_B4: i16 = 12;
pub const DMPAPER_B5: i16 = 13;
pub const DMPAPER_FOLIO: i16 = 14;
pub const DMPAPER_QUARTO: i16 = 15;

/// One paper of Sakura's list (`PAPER_INFO`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaperInfo {
    /// The `DMPAPER_*` id.
    pub id: i16,
    /// The width of the paper, held upright, in 0.1 mm.
    pub width: i32,
    /// The height of the paper, held upright, in 0.1 mm.
    pub height: i32,
    /// The name Sakura shows in the dialog.
    pub name: &'static str,
}

const fn p(id: i16, width: i32, height: i32, name: &'static str) -> PaperInfo {
    PaperInfo {
        id,
        width,
        height,
        name,
    }
}

/// The papers Sakura knows, in its order (`CPrint::m_paperInfoArr`).
pub static PAPER_INFO: [PaperInfo; 41] = [
    p(9, 2100, 2970, "A4 (210 x 297 mm)"),
    p(8, 2970, 4200, "A3 (297 x 420 mm)"),
    p(10, 2100, 2970, "A4 small(210 x 297 mm)"),
    p(11, 1480, 2100, "A5 (148 x 210 mm)"),
    p(12, 2500, 3540, "B4 (250 x 354 mm)"),
    p(13, 1820, 2570, "B5 (182 x 257 mm)"),
    p(15, 2150, 2750, "Quarto(215 x 275 mm)"),
    p(27, 1100, 2200, "DL Envelope(110 x 220 mm)"),
    p(28, 1620, 2290, "C5 Envelope(162 x 229 mm)"),
    p(29, 3240, 4580, "C3 Envelope(324 x 458 mm)"),
    p(30, 2290, 3240, "C4 Envelope(229 x 324 mm)"),
    p(31, 1140, 1620, "C6 Envelope(114 x 162 mm)"),
    p(32, 1140, 2290, "C65 Envelope(114 x 229 mm)"),
    p(33, 2500, 3530, "B4 Envelope(250 x 353 mm)"),
    p(34, 1760, 2500, "B5 Envelope(176 x 250 mm)"),
    p(35, 1760, 1250, "B6 Envelope(176 x 125 mm)"),
    p(36, 1100, 2300, "Italy Envelope(110 x 230 mm)"),
    p(1, 2159, 2794, "Letter (8 1/2 x 11 inch)"),
    p(5, 2159, 3556, "Legal  (8 1/2 x 14 inch)"),
    p(24, 4318, 5588, "C sheet (17 x 22 inch)"),
    p(25, 5588, 8634, "D sheet (22 x 34 inch)"),
    p(26, 8634, 11176, "E sheet (34 x 44 inch)"),
    p(2, 2159, 2794, "Letter Small (8 1/2 x 11 inch)"),
    p(3, 2794, 4318, "Tabloid (11 x 17 inch)"),
    p(4, 4318, 2794, "Ledger  (17 x 11 inch)"),
    p(6, 1397, 2159, "Statement (5 1/2 x 8 1/2 inch)"),
    p(7, 1841, 2667, "Executive (7 1/4 x 10 1/2 inch)"),
    p(14, 2159, 3302, "Folio (8 1/2 x 13 inch)"),
    p(16, 2540, 3556, "10x14 inch sheet"),
    p(17, 2794, 4318, "11x17 inch sheet"),
    p(18, 2159, 2794, "Note (8 1/2 x 11 inch)"),
    p(19, 984, 2254, "#9 Envelope  (3 7/8 x 8 7/8 inch)"),
    p(20, 1047, 2413, "#10 Envelope (4 1/8 x 9 1/2 inch)"),
    p(21, 1143, 2635, "#11 Envelope (4 1/2 x 10 3/8 inch)"),
    p(22, 1206, 2794, "#12 Envelope (4 3/4 x 11 inch)"),
    p(23, 1270, 2921, "#14 Envelope (5 x 11 1/2 inch)"),
    p(37, 984, 1905, "Monarch Envelope (3 7/8 x 7 1/2 inch)"),
    p(38, 920, 1651, "6 3/4 Envelope (3 5/8 x 6 1/2 inch)"),
    p(39, 3778, 2794, "US Std Fanfold (14 7/8 x 11 inch)"),
    p(40, 2159, 3048, "German Std Fanfold   (8 1/2 x 12 inch)"),
    p(41, 2159, 3302, "German Legal Fanfold (8 1/2 x 13 inch)"),
];

/// The paper with the id `id` (`CPrint::FindPaperInfo`).
pub fn find_paper_info(id: i16) -> Option<&'static PaperInfo> {
    PAPER_INFO.iter().find(|info| info.id == id)
}

/// The size of the paper as it is turned, in 0.1 mm, measured from the edges of the paper
/// (`m_nPreview_PaperAllWidth` and `m_nPreview_PaperAllHeight`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paper {
    pub width: i32,
    pub height: i32,
}

impl Paper {
    /// Sakura's default paper, which it also takes when it cannot ask the printer
    /// (`CPrintPreview::OnChangePrintSetting`: "A4 portrait on an error").
    pub const A4: Paper = Paper {
        width: 2100,
        height: 2970,
    };

    /// The paper of the setting, from Sakura's list, turned as the setting says
    /// (`CPrint::GetPaperSize`). `None` for an id that is not in the list, as Sakura fails.
    pub fn of_setting(setting: &PrintSetting) -> Option<Self> {
        let info = find_paper_info(setting.print_paper_size)?;
        Some(
            Paper {
                width: info.width,
                height: info.height,
            }
            .turned(setting.print_paper_orientation),
        )
    }

    /// The paper held in `orientation`: landscape swaps the width and the height. The size is
    /// taken as upright, so a paper that is already turned must not be turned again.
    pub fn turned(self, orientation: Orientation) -> Self {
        match orientation {
            Orientation::Portrait => self,
            Orientation::Landscape => Paper {
                width: self.height,
                height: self.width,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sakuras() {
        // CPrint::SettingInitialize
        let s = PrintSetting::default();
        assert_eq!((s.print_font_width, s.print_font_height), (12, 24));
        assert_eq!((s.print_dansuu, s.print_dan_space), (1, 70));
        assert_eq!(s.print_line_spacing, 30);
        assert_eq!(
            (
                s.print_margin_ty,
                s.print_margin_by,
                s.print_margin_lx,
                s.print_margin_rx
            ),
            (100, 200, 200, 100)
        );
        assert!(s.print_word_wrap && !s.print_line_number && !s.color_print);
        assert_eq!(s.header_form, ["$f", "", ""]);
        assert_eq!(s.footer_form, ["", "- $p -", ""]);
    }

    #[test]
    fn header_height_follows_the_form_and_the_font() {
        let mut s = PrintSetting::default();
        // 24 * (30 + 100) / 100
        assert_eq!(s.header_height(), 31);
        s.footer_form = Default::default();
        assert_eq!(s.footer_height(), 0);
        s.header_form = Default::default();
        assert_eq!(s.header_height(), 0);
        // 100 (10 pt) * 254 / 720 = 35, with the spacing 45.
        s.footer_form[1] = "x".into();
        s.footer_font = Some(HeaderFooterFont {
            face: "Arial".into(),
            point_size: 100,
        });
        assert_eq!(s.footer_height(), 35 * 130 / 100);
        // A font without a face is no font.
        s.footer_font = Some(HeaderFooterFont {
            face: String::new(),
            point_size: 100,
        });
        assert_eq!(s.footer_height(), 31);
    }

    #[test]
    fn the_dialog_keeps_values_in_range() {
        // CDlgPrintSetting::sDataRange and GetData
        let s = PrintSetting {
            print_font_height: 3,
            print_line_spacing: 400,
            print_dansuu: 9,
            print_dan_space: 999,
            print_margin_ty: 1234,
            print_margin_by: 75,
            print_margin_lx: -5,
            print_margin_rx: 100,
            ..PrintSetting::default()
        }
        .normalized();
        assert_eq!((s.print_font_height, s.print_font_width), (7, 4));
        assert_eq!((s.print_line_spacing, s.print_dansuu), (150, 4));
        assert_eq!(s.print_dan_space, 300);
        assert_eq!(
            (s.print_margin_ty, s.print_margin_by, s.print_margin_lx),
            (500, 70, 0)
        );
        assert_eq!(s.print_margin_rx, 100);
        // 24 is even: the width is 12.
        let d = PrintSetting::default().normalized();
        assert_eq!(d, PrintSetting::default());
    }

    #[test]
    fn papers() {
        let mut s = PrintSetting::default();
        assert_eq!(Paper::of_setting(&s), Some(Paper::A4));
        s.print_paper_orientation = Orientation::Landscape;
        assert_eq!(
            Paper::of_setting(&s),
            Some(Paper {
                width: 2970,
                height: 2100
            })
        );
        s.print_paper_size = DMPAPER_B5;
        assert_eq!(
            Paper::of_setting(&s),
            Some(Paper {
                width: 2570,
                height: 1820
            })
        );
        // An id that is not in the list fails, as GetPaperSize does.
        s.print_paper_size = 0;
        assert_eq!(Paper::of_setting(&s), None);
        s.print_paper_size = 999;
        assert_eq!(Paper::of_setting(&s), None);
    }

    #[test]
    fn the_paper_list_is_sakuras() {
        let ids: std::collections::HashSet<_> = PAPER_INFO.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), 41);
        assert_eq!(find_paper_info(DMPAPER_A3).unwrap().width, 2970);
        assert_eq!(find_paper_info(DMPAPER_LETTER).unwrap().height, 2794);
    }
}
