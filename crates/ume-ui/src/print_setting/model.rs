//! What the print setting dialog holds, and Sakura's rules for it: no window, so it is tested
//! without one.
//!
//! Sakura: `CDlgPrintSetting` (`sakura_core/dlg/CDlgPrintSetting.cpp`): `GetData` and
//! `DataCheckAndCorrect` for the values, `CalcPrintableLineAndColumn` for the two numbers the
//! dialog shows, `SetFontName` for the label of the font of the header and the footer.

use ume_print::{HEADER_MAX, HeaderFooterFont, Paper, PrintSetting};

/// The number fields of the dialog, in the order of the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Number {
    /// フォント高 (1/10 mm).
    FontHeight,
    /// 行送り (%).
    LineSpacing,
    /// 段数.
    Dansuu,
    /// 段の隙間 (mm).
    DanSpace,
    /// 余白／上 (mm).
    MarginTop,
    /// 余白／下 (mm).
    MarginBottom,
    /// 余白／左 (mm).
    MarginLeft,
    /// 余白／右 (mm).
    MarginRight,
}

impl Number {
    pub(crate) const ALL: [Number; 8] = [
        Number::FontHeight,
        Number::LineSpacing,
        Number::Dansuu,
        Number::DanSpace,
        Number::MarginTop,
        Number::MarginBottom,
        Number::MarginLeft,
        Number::MarginRight,
    ];

    /// The value of the field for `s` (`OnChangeSettingType`): the lengths the dialog shows in mm
    /// are the setting's in 1/10 mm divided by 10.
    pub(crate) fn of(self, s: &PrintSetting) -> i32 {
        match self {
            Number::FontHeight => s.print_font_height,
            Number::LineSpacing => s.print_line_spacing,
            Number::Dansuu => s.print_dansuu,
            Number::DanSpace => s.print_dan_space / 10,
            Number::MarginTop => s.print_margin_ty / 10,
            Number::MarginBottom => s.print_margin_by / 10,
            Number::MarginLeft => s.print_margin_lx / 10,
            Number::MarginRight => s.print_margin_rx / 10,
        }
    }

    /// Puts the value of the field into the setting (`GetData`, before the check): the lengths in
    /// mm are multiplied by 10.
    fn put(self, s: &mut PrintSetting, value: i32) {
        match self {
            Number::FontHeight => s.print_font_height = value,
            Number::LineSpacing => s.print_line_spacing = value,
            Number::Dansuu => s.print_dansuu = value,
            Number::DanSpace => s.print_dan_space = value.saturating_mul(10),
            Number::MarginTop => s.print_margin_ty = value.saturating_mul(10),
            Number::MarginBottom => s.print_margin_by = value.saturating_mul(10),
            Number::MarginLeft => s.print_margin_lx = value.saturating_mul(10),
            Number::MarginRight => s.print_margin_rx = value.saturating_mul(10),
        }
    }
}

/// `GetDlgItemInt( ..., nullptr, FALSE )`: the number in the text, or 0 for text that is not an
/// unsigned number.
pub(crate) fn parse_count(text: &str) -> i32 {
    let text = text.trim();
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return 0;
    }
    text.parse::<i64>()
        .map_or(i32::MAX, |n| i32::try_from(n).unwrap_or(i32::MAX))
}

/// `GetData`: the setting the dialog's controls make. `base` has what the other controls hold
/// (fonts, paper, the check boxes); `numbers` and the six forms are the text of the edit boxes.
/// The numbers are checked and corrected (`DataCheckAndCorrect`), the width of a character
/// follows the height, and the forms are cut to 99 characters (`HEADER_MAX`).
pub(crate) fn collect(
    base: &PrintSetting,
    numbers: &[String; 8],
    header: &[String; 3],
    footer: &[String; 3],
) -> PrintSetting {
    let mut s = base.clone();
    for (number, text) in Number::ALL.into_iter().zip(numbers) {
        number.put(&mut s, parse_count(text));
    }
    s.header_form = header.clone();
    s.footer_form = footer.clone();
    debug_assert_eq!(HEADER_MAX, 100);
    s.normalized()
}

/// `CalcPrintableLineAndColumn`: the number of characters in a row and of rows in a column, for
/// the paper of the setting. `None` when the paper is not one of the list
/// (`CPrint::GetPaperSize` fails). `number_columns` is the width of the line numbers in
/// characters; the dialog takes it into account only when the line numbers are printed.
pub(crate) fn printable(s: &PrintSetting, number_columns: u32) -> Option<(i32, i32)> {
    let paper = Paper::of_setting(s)?;
    let number_cells = if s.print_line_number {
        number_columns
    } else {
        0
    };
    Some((
        s.printable_columns(paper, number_cells),
        s.printable_lines(paper),
    ))
}

/// OK can be pressed when there is room to print on the paper (`CalcPrintableLineAndColumn`
/// disables it otherwise, "2013.5.10 aroka").
pub(crate) fn has_printable_area(s: &PrintSetting, number_columns: u32) -> bool {
    printable(s, number_columns).is_some_and(|(columns, lines)| columns > 0 && lines > 0)
}

/// The size of the characters in points, as the dialog shows it next to the height: "12.5pt"
/// (`nFontPoints = nPrintFontHeight * 720 / 254`, in 1/10 pt).
pub(crate) fn font_points_text(height: i32) -> String {
    let points = height.saturating_mul(720) / 254;
    format!("{}.{}pt", points / 10, points % 10)
}

/// Windows' `MulDiv`: `a * b / c`, rounded to the nearest.
fn mul_div(a: i64, b: i64, c: i64) -> i64 {
    (a * b + c / 2) / c
}

/// `SetFontName`: the label of the font of the header or the footer, such as
/// "Arial(12pt/4.2mm)"; empty when no font is set. The face is cut to 32 characters
/// (`%.32s`). The size has a decimal only when it is not a whole point.
///
/// Sakura writes `nMM/10, nMM/10` for the millimeters, which shows the whole part twice (3.3mm for
/// 10 pt); this shows the tenths.
pub(crate) fn header_font_label(font: Option<&HeaderFooterFont>) -> String {
    let Some(font) = font.filter(|f| !f.face.is_empty()) else {
        return String::new();
    };
    let face: String = font.face.chars().take(32).collect();
    let mm = mul_div(i64::from(font.point_size), 254, 720);
    let size = if font.point_size % 10 == 0 {
        format!("{}", font.point_size / 10)
    } else {
        format!("{}.{}", font.point_size / 10, font.point_size % 10)
    };
    format!("{face}({size}pt/{}.{}mm)", mm / 10, mm % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(values: [&str; 8]) -> [String; 8] {
        values.map(str::to_owned)
    }

    fn defaults() -> ([String; 8], [String; 3], [String; 3]) {
        let s = PrintSetting::default();
        (
            Number::ALL.map(|n| n.of(&s).to_string()),
            s.header_form.clone(),
            s.footer_form.clone(),
        )
    }

    #[test]
    fn the_fields_show_the_setting_in_mm() {
        let s = PrintSetting::default();
        let shown: Vec<i32> = Number::ALL.iter().map(|n| n.of(&s)).collect();
        // 24 (1/10 mm), 30 %, 1, 7 mm, then 10, 20, 20, 10 mm
        assert_eq!(shown, [24, 30, 1, 7, 10, 20, 20, 10]);
    }

    #[test]
    fn the_fields_of_the_default_setting_give_the_default_setting() {
        let (n, h, f) = defaults();
        assert_eq!(
            collect(&PrintSetting::default(), &n, &h, &f),
            PrintSetting::default()
        );
    }

    #[test]
    fn numbers_are_corrected_into_their_ranges() {
        // DataCheckAndCorrect: at most or at least the limits of sDataRange
        let (_, h, f) = defaults();
        let n = texts(["3", "400", "9", "99", "99", "51", "0", "50"]);
        let s = collect(&PrintSetting::default(), &n, &h, &f);
        assert_eq!(s.print_font_height, 7);
        assert_eq!(s.print_line_spacing, 150);
        assert_eq!(s.print_dansuu, 4);
        assert_eq!(s.print_dan_space, 300);
        assert_eq!((s.print_margin_ty, s.print_margin_by), (500, 500));
        assert_eq!((s.print_margin_lx, s.print_margin_rx), (0, 500));
        let n = texts(["500", "0", "0", "0", "0", "0", "0", "0"]);
        let s = collect(&PrintSetting::default(), &n, &h, &f);
        assert_eq!(s.print_font_height, 200);
        assert_eq!(s.print_dansuu, 1);
        assert_eq!(s.print_line_spacing, 0);
    }

    #[test]
    fn the_width_of_a_character_is_half_the_height_rounded_up() {
        let (_, h, f) = defaults();
        for (height, width) in [("24", 12), ("25", 13), ("7", 4), ("200", 100)] {
            let n = texts([height, "30", "1", "7", "10", "20", "20", "10"]);
            let s = collect(&PrintSetting::default(), &n, &h, &f);
            assert_eq!(s.print_font_width, width, "{height}");
        }
    }

    #[test]
    fn text_that_is_not_a_number_is_zero() {
        // GetDlgItemInt: 0, which the check turns into the least value
        assert_eq!(parse_count(""), 0);
        assert_eq!(parse_count("abc"), 0);
        assert_eq!(parse_count("-5"), 0);
        assert_eq!(parse_count("12"), 12);
        assert_eq!(parse_count(" 12 "), 12);
        assert_eq!(parse_count("９"), 0);
        assert_eq!(parse_count("99999999999999"), i32::MAX);
        let (_, h, f) = defaults();
        let n = texts(["", "x", "", "", "", "", "", ""]);
        let s = collect(&PrintSetting::default(), &n, &h, &f);
        assert_eq!(s.print_font_height, 7);
        assert_eq!(s.print_line_spacing, 0);
        assert_eq!(s.print_dansuu, 1);
    }

    #[test]
    fn the_forms_are_cut_to_99_characters() {
        let (n, _, _) = defaults();
        let long = "あ".repeat(120);
        let header = [long.clone(), String::new(), "x".into()];
        let footer = [String::new(), long, String::new()];
        let s = collect(&PrintSetting::default(), &n, &header, &footer);
        assert_eq!(s.header_form[0].chars().count(), 99);
        assert_eq!(s.header_form[2], "x");
        assert_eq!(s.footer_form[1].chars().count(), 99);
    }

    #[test]
    fn the_other_controls_are_kept() {
        let base = PrintSetting {
            print_kinsoku_head: true,
            print_font_face_han: "Courier".into(),
            header_font: Some(HeaderFooterFont {
                face: "Arial".into(),
                point_size: 100,
            }),
            ..PrintSetting::default()
        };
        let (n, h, f) = defaults();
        let s = collect(&base, &n, &h, &f);
        assert!(s.print_kinsoku_head);
        assert_eq!(s.print_font_face_han, "Courier");
        assert_eq!(s.header_font, base.header_font);
    }

    #[test]
    fn the_printable_size_is_what_the_layout_counts() {
        // A4 portrait, 12 / 24 (1/10 mm), margins 20 + 10 mm left and right: (2100 - 300) / 12
        let s = PrintSetting::default();
        let (columns, lines) = printable(&s, 3).unwrap();
        assert_eq!(columns, 150);
        // The header and the footer take 2 x 31 each, the rows are 24 + 7 high.
        assert_eq!(lines, (2970 - 100 - 200 - 31 * 2 - 31 * 2 + 7) / (24 + 7));
        // the line numbers take 3 columns each when they are printed
        let numbered = PrintSetting {
            print_line_number: true,
            ..s.clone()
        };
        assert_eq!(printable(&numbered, 3).unwrap().0, 147);
        assert_eq!(printable(&numbered, 3).unwrap().1, lines);
        // landscape: the paper is turned
        let landscape = PrintSetting {
            print_paper_orientation: ume_print::Orientation::Landscape,
            ..s.clone()
        };
        let (columns, lines_landscape) = printable(&landscape, 3).unwrap();
        assert_eq!(columns, (2970 - 300) / 12);
        assert!(lines_landscape < lines);
        // two columns: the space between is taken off, and each has half
        let two = PrintSetting {
            print_dansuu: 2,
            ..s
        };
        assert_eq!(printable(&two, 3).unwrap().0, (1800 - 70) / 12 / 2);
    }

    #[test]
    fn a_paper_that_is_not_in_the_list_has_no_printable_size() {
        let s = PrintSetting {
            print_paper_size: 0,
            ..PrintSetting::default()
        };
        assert_eq!(printable(&s, 3), None);
        assert!(!has_printable_area(&s, 3));
    }

    #[test]
    fn ok_needs_room_to_print() {
        let s = PrintSetting::default();
        assert!(has_printable_area(&s, 3));
        // margins of 50 mm each on A5 (148 mm wide) leave 48 mm
        let narrow = PrintSetting {
            print_paper_size: ume_print::DMPAPER_A5,
            print_margin_lx: 500,
            print_margin_rx: 500,
            print_font_height: 200,
            print_font_width: 100,
            ..s.clone()
        };
        assert!(has_printable_area(&narrow, 3));
        // 4 columns with 30 mm between them and a 20 mm character cannot fit in 48 mm
        let none = PrintSetting {
            print_dansuu: 4,
            print_dan_space: 300,
            ..narrow
        };
        assert!(!has_printable_area(&none, 3));
        // no room for a row: the margins use the whole height
        let flat = PrintSetting {
            print_margin_ty: 1500,
            print_margin_by: 1500,
            ..s
        };
        assert!(!has_printable_area(&flat, 3));
    }

    #[test]
    fn the_font_size_is_shown_in_points() {
        assert_eq!(font_points_text(24), "6.8pt");
        assert_eq!(font_points_text(7), "1.9pt");
        assert_eq!(font_points_text(200), "56.6pt");
    }

    #[test]
    fn the_font_of_the_header_is_shown_with_its_size() {
        assert_eq!(header_font_label(None), "");
        let arial = |points| HeaderFooterFont {
            face: "Arial".into(),
            point_size: points,
        };
        // 12 pt = 4.2 mm
        assert_eq!(header_font_label(Some(&arial(120))), "Arial(12pt/4.2mm)");
        // not a whole number of points
        assert_eq!(header_font_label(Some(&arial(105))), "Arial(10.5pt/3.7mm)");
        // an empty face is no font
        let none = HeaderFooterFont {
            face: String::new(),
            point_size: 100,
        };
        assert_eq!(header_font_label(Some(&none)), "");
        // the face is cut to 32 characters
        let long = HeaderFooterFont {
            face: "あ".repeat(40),
            point_size: 100,
        };
        assert!(header_font_label(Some(&long)).starts_with(&format!("{}(10pt", "あ".repeat(32))));
    }
}
