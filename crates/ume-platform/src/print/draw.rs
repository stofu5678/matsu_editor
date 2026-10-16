//! What every drawer draws. A [`Page`] of `ume-print` is text at places; this turns it into a
//! flat list of drawing operations that need no OS, so that what a drawer is asked to draw can be
//! tested without a printer (ADR-0011: "this glyph, here"). The PDF drawer takes the list as it
//! is; the Windows and macOS drawers draw a [`PlacedText`] at a time (they shape runs of text),
//! and use [`font_spec`] and [`cell_positions`] from here.

use ume_print::{FontKind, HAlign, Layout, Page, PlacedText, PrintSetting, Rule};

/// One thing to draw. Lengths are in 0.1 mm from the top left corner of the paper.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DrawOp {
    /// A character in its cell, text on the grid (`FontKind::Han` and `Zen`).
    Char {
        ch: char,
        x: i32,
        baseline: i32,
        font: FontKind,
    },
    /// A header or a footer in a font of its own. It is not on the grid, so the drawer places it
    /// as a whole, by `align`.
    Text {
        text: String,
        x: i32,
        baseline: i32,
        font: FontKind,
        align: HAlign,
    },
    /// A straight line.
    Line { from: (i32, i32), to: (i32, i32) },
}

/// The face and the height (the size of the characters, 0.1 mm) of a kind of text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontSpec<'a> {
    pub face: &'a str,
    pub height: i32,
}

/// The font that draws `kind` in `setting`: the print fonts for the grid, and the header's or the
/// footer's own font (`m_lfHeader`, `m_lfFooter`) when the setting has one.
pub fn font_spec(setting: &PrintSetting, kind: FontKind) -> FontSpec<'_> {
    let own = match kind {
        FontKind::Header => setting.header_font.as_ref(),
        FontKind::Footer => setting.footer_font.as_ref(),
        FontKind::Han | FontKind::Zen => None,
    };
    if let Some(font) = own.filter(|f| !f.face.is_empty()) {
        return FontSpec {
            face: &font.face,
            height: font.height(),
        };
    }
    FontSpec {
        face: match kind {
            FontKind::Zen => &setting.print_font_face_zen,
            _ => &setting.print_font_face_han,
        },
        height: setting.print_font_height,
    }
}

/// Whether `kind` is text on the grid of cells.
pub fn on_grid(kind: FontKind) -> bool {
    matches!(kind, FontKind::Han | FontKind::Zen)
}

/// The characters of a text on the grid with the left edge of their cells: the byte offset, the
/// character and the `x`. Whatever the width of a glyph in the font is, it goes where this says.
pub fn cell_positions<'a>(
    text: &'a PlacedText,
    layout: &'a Layout,
) -> impl Iterator<Item = (usize, char, i32)> + 'a {
    text.positions(layout.setting().print_font_width, layout.width_rule())
}

/// The operations of a page, in the order Sakura draws: the texts, then the lines.
pub fn page_ops(layout: &Layout, page: &Page) -> Vec<DrawOp> {
    let mut ops = Vec::new();
    for text in &page.texts {
        if on_grid(text.font) {
            ops.extend(cell_positions(text, layout).map(|(_, ch, x)| DrawOp::Char {
                ch,
                x,
                baseline: text.baseline,
                font: text.font,
            }));
        } else {
            ops.push(DrawOp::Text {
                text: text.text.clone(),
                x: text.x,
                baseline: text.baseline,
                font: text.font,
                align: text.align,
            });
        }
    }
    ops.extend(page.rules.iter().map(|Rule { from, to }| DrawOp::Line {
        from: *from,
        to: *to,
    }));
    ops
}

/// 0.1 mm in points (1/72 inch), the unit of PDF and Core Graphics.
pub fn points(tenth_mm: i32) -> f32 {
    tenth_mm as f32 * 72.0 / 254.0
}

#[cfg(test)]
mod tests {
    use ume_print::{HeaderContext, HeaderFooterFont, Paper, TypeOptions};

    use super::*;

    fn layout_of(lines: &[&str], setting: &PrintSetting) -> Layout {
        Layout::new(lines, setting, Paper::A4, &TypeOptions::default()).unwrap()
    }

    #[test]
    fn each_character_goes_to_its_own_cell() {
        // Sakura's defaults: cells are 1.2 mm wide, half-width characters take one and full-width
        // ones two (ExtTextOut is given the width of each character).
        let setting = PrintSetting::default();
        let lines = ["aあb", ""];
        let layout = layout_of(&lines, &setting);
        let page = layout.page(0, &lines, &HeaderContext::default()).unwrap();
        let ops = page_ops(&layout, &page);
        let body: Vec<_> = ops
            .iter()
            .filter_map(|op| match op {
                DrawOp::Char {
                    ch, x, baseline, ..
                } if "aあb".contains(*ch) => Some((*ch, *x, *baseline)),
                _ => None,
            })
            .collect();
        let left = setting.print_margin_lx;
        assert_eq!(body.len(), 3);
        assert_eq!(body[0].1, left);
        assert_eq!(body[1].1, left + 12);
        assert_eq!(body[2].1, left + 12 + 24);
        // All on the same baseline.
        assert!(body.iter().all(|c| c.2 == body[0].2));
    }

    #[test]
    fn the_scripts_get_the_fonts_of_their_kind() {
        let setting = PrintSetting {
            print_font_face_han: "Han".into(),
            print_font_face_zen: "Zen".into(),
            ..PrintSetting::default()
        };
        let lines = ["aあ", ""];
        let layout = layout_of(&lines, &setting);
        let page = layout.page(0, &lines, &HeaderContext::default()).unwrap();
        let kinds: Vec<_> = page_ops(&layout, &page)
            .into_iter()
            .filter_map(|op| match op {
                DrawOp::Char { ch, font, .. } if ch == 'a' || ch == 'あ' => Some((ch, font)),
                _ => None,
            })
            .collect();
        assert_eq!(kinds, [('a', FontKind::Han), ('あ', FontKind::Zen)]);
        assert_eq!(font_spec(&setting, FontKind::Han).face, "Han");
        assert_eq!(font_spec(&setting, FontKind::Zen).face, "Zen");
        assert_eq!(font_spec(&setting, FontKind::Zen).height, 24);
    }

    #[test]
    fn the_header_and_the_footer_are_drawn_like_the_body() {
        // The default header is "$f" at the left ((無題) for a new document) and the footer
        // "- $p -" at the center.
        let setting = PrintSetting::default();
        let lines = ["x", ""];
        let layout = layout_of(&lines, &setting);
        let page = layout.page(0, &lines, &HeaderContext::default()).unwrap();
        let drawn: String = page_ops(&layout, &page)
            .iter()
            .filter_map(|op| match op {
                DrawOp::Char { ch, .. } => Some(*ch),
                _ => None,
            })
            .collect();
        assert_eq!(drawn, "(無題)x- 1 -");
    }

    #[test]
    fn line_numbers_have_a_rule() {
        let setting = PrintSetting {
            print_line_number: true,
            ..PrintSetting::default()
        };
        let lines = ["x", "y", ""];
        let layout = layout_of(&lines, &setting);
        let page = layout.page(0, &lines, &HeaderContext::default()).unwrap();
        let lines: Vec<_> = page_ops(&layout, &page)
            .into_iter()
            .filter(|op| matches!(op, DrawOp::Line { .. }))
            .collect();
        assert_eq!(lines.len(), 1);
        let DrawOp::Line { from, to } = lines[0] else {
            unreachable!()
        };
        // A vertical line from the top of the text down.
        assert_eq!(from.0, to.0);
        assert!(to.1 > from.1);
    }

    #[test]
    fn a_header_in_its_own_font_is_placed_as_a_whole() {
        let setting = PrintSetting {
            header_form: [String::new(), "Title".into(), String::new()],
            header_font: Some(HeaderFooterFont {
                face: "Sans".into(),
                point_size: 120,
            }),
            ..PrintSetting::default()
        };
        let lines = ["x", ""];
        let layout = layout_of(&lines, &setting);
        let page = layout.page(0, &lines, &HeaderContext::default()).unwrap();
        let texts: Vec<_> = page_ops(&layout, &page)
            .into_iter()
            .filter_map(|op| match op {
                DrawOp::Text { text, font, .. } => Some((text, font)),
                _ => None,
            })
            .collect();
        assert_eq!(texts, [("Title".to_string(), FontKind::Header)]);
        let spec = font_spec(&setting, FontKind::Header);
        assert_eq!(spec.face, "Sans");
        assert_eq!(spec.height, 120 * 254 / 720);
        // The footer has no font of its own: the print font.
        assert_eq!(
            font_spec(&setting, FontKind::Footer).face,
            setting.print_font_face_han
        );
    }

    #[test]
    fn a_point_is_a_72th_part_of_an_inch() {
        assert!((points(254) - 72.0).abs() < 1e-4);
        assert!((points(2100) - 595.28).abs() < 0.01); // A4 is 595.28 pt wide
    }
}
