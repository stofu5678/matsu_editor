//! What the font dialog holds, and how it becomes the font of the settings file (a `LOGFONT` and the
//! point size): no window, so it is tested without one.
//!
//! The dialog has the parts of the font dialog of Windows that matter to an editor: the family, the
//! style (標準, 斜体, 太字, 太字 斜体) and the size. The `LOGFONT` it returns is the one `ChooseFont`
//! returns for such a choice: the height is the size at 96 dpi (`-MulDiv(points, 96, 72)`), the
//! weight 400 or 700, italic 0 or 1, the width, escapement and orientation 0 and the output
//! precision `OUT_STROKE_PRECIS` (an outline font). The fields that mean nothing without a font
//! mapper (the character set, the clip precision, the quality, the pitch and family, underline and
//! strike-out) stay as the font the dialog started with has them.

use ume_config::settings::{FontSetting, LogFont};

use crate::settings::view_settings::DPI;

/// The least and the most size, in 1/10 point (`nPointSizeMin` and `nPointSizeMax` of
/// `Command_SETFONTSIZE`, `sakura_core/cmd/CViewCommander_Settings.cpp`).
pub const SIZE_MIN: i32 = 10;
pub const SIZE_MAX: i32 = 720;

/// The sizes the list offers, in 1/10 point (the list of the font dialog of Windows).
pub const SIZES: [i32; 16] = [
    80, 90, 100, 110, 120, 140, 160, 180, 200, 220, 240, 260, 280, 360, 480, 720,
];

/// `FW_NORMAL` and `FW_BOLD`.
const WEIGHT_NORMAL: i32 = 400;
const WEIGHT_BOLD: i32 = 700;
/// `FW_SEMIBOLD`: from this weight on, the style list shows 太字.
const WEIGHT_SEMIBOLD: i32 = 600;
/// `OUT_STROKE_PRECIS`: what `Command_SETFONTSIZE` accepts as a TrueType (outline) font.
pub const OUT_STROKE_PRECIS: u8 = 3;
/// `OUT_PS_ONLY_PRECIS`.
const OUT_PS_ONLY_PRECIS: u8 = 10;
/// `LF_FACESIZE` without the end mark: a longer face name does not fit a `LOGFONT`.
const FACE_NAME_MAX_UNITS: usize = 31;

/// The style list of the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Regular,
    Italic,
    Bold,
    BoldItalic,
}

impl Style {
    pub const ALL: [Style; 4] = [
        Style::Regular,
        Style::Italic,
        Style::Bold,
        Style::BoldItalic,
    ];

    /// The name in the list (the names Windows' dialog shows in Japanese).
    pub fn label(self) -> &'static str {
        match self {
            Style::Regular => "標準",
            Style::Italic => "斜体",
            Style::Bold => "太字",
            Style::BoldItalic => "太字 斜体",
        }
    }

    pub fn bold(self) -> bool {
        matches!(self, Style::Bold | Style::BoldItalic)
    }

    pub fn italic(self) -> bool {
        matches!(self, Style::Italic | Style::BoldItalic)
    }

    fn of(font: &LogFont) -> Self {
        match (font.weight >= WEIGHT_SEMIBOLD, font.italic != 0) {
            (false, false) => Style::Regular,
            (false, true) => Style::Italic,
            (true, false) => Style::Bold,
            (true, true) => Style::BoldItalic,
        }
    }
}

/// Whether `lfOutPrecision` says the font is TrueType (`Command_SETFONTSIZE` changes no other).
pub fn is_truetype(out_precision: u8) -> bool {
    matches!(out_precision, OUT_STROKE_PRECIS | OUT_PS_ONLY_PRECIS)
}

/// Whether a family name fits the `lfFaceName` of a `LOGFONT` (31 UTF-16 units).
pub fn face_fits(family: &str) -> bool {
    !family.is_empty() && family.encode_utf16().count() <= FACE_NAME_MAX_UNITS
}

/// What the user has chosen so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub family: String,
    pub style: Style,
    /// 1/10 point.
    pub size: i32,
}

impl Choice {
    /// The choice that shows `font`: its face, its weight and slant, and its size (the point size
    /// when it has one, else worked out from the height).
    pub fn of(font: &FontSetting) -> Self {
        Self {
            family: font.log_font.face_name.clone(),
            style: Style::of(&font.log_font),
            size: font.point_size_for_dpi(DPI).clamp(SIZE_MIN, SIZE_MAX),
        }
    }

    /// The font setting this choice is, over `base`, the font the dialog started with.
    pub fn setting(&self, base: &FontSetting) -> FontSetting {
        let mut family = String::new();
        let mut units = 0;
        for c in self.family.chars() {
            units += c.len_utf16();
            if units > FACE_NAME_MAX_UNITS {
                break;
            }
            family.push(c);
        }
        let mut font = FontSetting {
            log_font: LogFont {
                face_name: family,
                weight: if self.style.bold() {
                    WEIGHT_BOLD
                } else {
                    WEIGHT_NORMAL
                },
                italic: u8::from(self.style.italic()),
                width: 0,
                escapement: 0,
                orientation: 0,
                out_precision: OUT_STROKE_PRECIS,
                ..base.log_font.clone()
            },
            point_size: self.size.clamp(SIZE_MIN, SIZE_MAX),
        };
        font.log_font.height = font.height_for_dpi(DPI);
        font
    }
}

/// The size typed in the size box, in 1/10 point: `10` and `10.5` are numbers, anything else is
/// not. A size outside the limits is brought into them.
pub fn parse_size(text: &str) -> Option<i32> {
    let points: f64 = text.trim().parse().ok()?;
    if !points.is_finite() {
        return None;
    }
    let tenths = (points * 10.0)
        .round()
        .clamp(f64::from(SIZE_MIN), f64::from(SIZE_MAX));
    Some(tenths as i32)
}

/// A size as the size box shows it: `10`, or `10.5`.
pub fn size_text(tenths: i32) -> String {
    if tenths % 10 == 0 {
        (tenths / 10).to_string()
    } else {
        format!("{:.1}", f64::from(tenths) / 10.0)
    }
}

/// The font as the settings pages show it next to the button: the face and the size, `MS Gothic(10pt)`
/// or `Consolas(10.5pt)` (`SetFontLabel`, `sakura_core/prop/CPropCommon.cpp`: `%s(%.0f pt)`, or
/// `%s(%.1f pt)` when the size is not whole). A font with no point size shows the size its height has.
pub fn label(font: &FontSetting) -> String {
    format!(
        "{}({}pt)",
        font.log_font.face_name,
        size_text(font.point_size_for_dpi(DPI))
    )
}

/// The family names to list: the installed ones that fit a `LOGFONT`, and `current` if it is not
/// among them (a font of the settings file that this machine lacks, such as ＭＳ ゴシック on
/// macOS), so that the dialog still shows what the file has.
pub fn families(installed: Vec<String>, current: &str) -> Vec<String> {
    let mut list: Vec<String> = installed.into_iter().filter(|f| face_fits(f)).collect();
    list.sort();
    list.dedup();
    if !current.is_empty() && !list.iter().any(|f| f == current) {
        list.insert(0, current.to_owned());
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consolas(points: i32) -> FontSetting {
        let mut font = FontSetting::default();
        font.log_font.face_name = "Consolas".to_owned();
        font.point_size = points;
        font
    }

    #[test]
    fn the_default_font_is_ms_gothic_regular_ten_points() {
        let choice = Choice::of(&FontSetting::default());
        assert_eq!(choice.family, "ＭＳ ゴシック");
        assert_eq!(choice.style, Style::Regular);
        // Sakura's default has only the height, -13 pixels: 9.75 points, shown as 9.8.
        assert_eq!(choice.size, 98);
    }

    #[test]
    fn a_choice_becomes_the_logfont_chosen_font_returns() {
        let base = FontSetting::default();
        let choice = Choice {
            family: "Consolas".to_owned(),
            style: Style::BoldItalic,
            size: 120,
        };
        let font = choice.setting(&base);
        assert_eq!(font.point_size, 120);
        // 12 pt at 96 dpi is 16 pixels.
        assert_eq!(font.log_font.height, -16);
        assert_eq!(font.log_font.weight, 700);
        assert_eq!(font.log_font.italic, 1);
        assert_eq!(font.log_font.face_name, "Consolas");
        assert_eq!(font.log_font.out_precision, OUT_STROKE_PRECIS);
        assert_eq!((font.log_font.width, font.log_font.escapement), (0, 0));
        // What the dialog cannot tell stays as the starting font has it.
        assert_eq!(font.log_font.char_set, base.log_font.char_set);
        assert_eq!(
            font.log_font.pitch_and_family,
            base.log_font.pitch_and_family
        );
        assert_eq!(font.log_font.quality, base.log_font.quality);
        // And the dialog shows the same choice again.
        assert_eq!(Choice::of(&font), choice);
    }

    #[test]
    fn the_logfont_round_trips_through_the_settings_keys() {
        let choice = Choice {
            family: "ＭＳ 明朝".to_owned(),
            style: Style::Italic,
            size: 105,
        };
        let font = choice.setting(&FontSetting::default());
        let mut profile = ume_config::Profile::new();
        font.write(&mut profile, "Font", "lf", None, None).unwrap();
        let mut read = FontSetting::default();
        assert!(read.read(&profile, "Font", "lf", None, None));
        assert_eq!(read, font);
        assert_eq!(profile.get("Font", "nPointSize"), Some("105"));
        assert_eq!(profile.get("Font", "lfFaceName"), Some("ＭＳ 明朝"));
    }

    #[test]
    fn a_weight_from_semibold_is_bold() {
        let mut font = consolas(100);
        font.log_font.weight = 600;
        assert_eq!(Choice::of(&font).style, Style::Bold);
        font.log_font.weight = 0;
        assert_eq!(Choice::of(&font).style, Style::Regular);
        font.log_font.italic = 1;
        assert_eq!(Choice::of(&font).style, Style::Italic);
    }

    #[test]
    fn a_face_name_that_does_not_fit_is_cut_not_broken() {
        assert!(face_fits("Consolas"));
        assert!(!face_fits(""));
        assert!(!face_fits(&"a".repeat(32)));
        let long = Choice {
            family: "あ".repeat(40),
            style: Style::Regular,
            size: 100,
        };
        assert_eq!(
            long.setting(&FontSetting::default()).log_font.face_name,
            "あ".repeat(31)
        );
    }

    #[test]
    fn the_size_box_takes_numbers_into_the_limits() {
        assert_eq!(parse_size("10"), Some(100));
        assert_eq!(parse_size(" 10.5 "), Some(105));
        assert_eq!(parse_size("0"), Some(SIZE_MIN));
        assert_eq!(parse_size("1000"), Some(SIZE_MAX));
        assert_eq!(parse_size("abc"), None);
        assert_eq!(parse_size(""), None);
        assert_eq!(parse_size("NaN"), None);
        assert_eq!(size_text(100), "10");
        assert_eq!(size_text(105), "10.5");
    }

    #[test]
    fn the_label_is_the_face_and_the_size() {
        assert_eq!(label(&consolas(100)), "Consolas(10pt)");
        assert_eq!(label(&consolas(105)), "Consolas(10.5pt)");
    }

    #[test]
    fn the_list_has_the_current_family_even_when_it_is_not_installed() {
        let installed = vec![
            "B".to_owned(),
            "A".to_owned(),
            "A".to_owned(),
            "x".repeat(40),
        ];
        assert_eq!(families(installed.clone(), "A"), ["A", "B"]);
        assert_eq!(
            families(installed, "ＭＳ ゴシック"),
            ["ＭＳ ゴシック", "A", "B"]
        );
    }
}
