//! The colors of a print come from the coloring of the window and the color entries of the type.

use std::sync::Arc;

use matsu_coloring::ColorRules;
use matsu_config::settings::DocumentTypes;
use matsu_core::buffer::Document;
use matsu_print::{LineStyler, StyleRun};

use super::DocStyler;

fn rules(configure: impl FnOnce(&mut matsu_config::settings::TypeSettings)) -> Arc<ColorRules> {
    let types = DocumentTypes::builtin();
    let index = types.index_of_name("C/C++").unwrap();
    let mut t = types.get(index).unwrap().clone();
    configure(&mut t);
    Arc::new(ColorRules::from_type(&t, types.keywords()))
}

fn styler(text: &str, rules: Arc<ColorRules>) -> DocStyler {
    DocStyler::new(
        Document::from_utf8(text.as_bytes().to_vec()).unwrap(),
        rules,
    )
}

fn runs(styler: &DocStyler, line: usize, text: &str) -> Vec<StyleRun> {
    let mut out = Vec::new();
    styler.style_line(line, text, true, &mut out);
    out
}

#[test]
fn the_entries_of_the_type_give_the_colors() {
    let text = "int x; // c\n";
    let s = styler(
        text,
        rules(|t| {
            let c = t.colors.get_mut("CMT").unwrap();
            c.text = 0x0000ff; // COLORREF: red
            c.bold = true;
            c.underline = true;
        }),
    );
    let out = runs(&s, 0, "int x; // c");
    let comment = out.iter().find(|r| r.range.start == 7).expect("a comment");
    assert_eq!(comment.range, 7..11);
    // COLORREF 0x0000ff is red; the style is 0xRRGGBB.
    assert_eq!(comment.style.color, 0xff0000);
    assert!(comment.style.bold && comment.style.underline);
    // The line ending is not in a run.
    assert!(out.iter().all(|r| r.range.end <= 11));
}

#[test]
fn an_entry_that_is_not_shown_has_no_color() {
    let text = "// c\n";
    let hidden = styler(
        text,
        rules(|t| {
            t.colors.get_mut("CMT").unwrap().disp = false;
        }),
    );
    // Without the comment strategy the text is in the text color: no run is a comment.
    let comment_color = {
        let types = DocumentTypes::builtin();
        let t = types.get(types.index_of_name("C/C++").unwrap()).unwrap();
        crate::settings::view_settings::rgb_of(t.colors.get("CMT").unwrap().text)
    };
    assert!(
        runs(&hidden, 0, "// c")
            .iter()
            .all(|r| r.style.color != comment_color)
    );
}

#[test]
fn a_block_comment_goes_on_in_the_next_line() {
    let s = styler("/* a\nb */ x\n", rules(|_| {}));
    // Line 1 starts inside the comment, which the states know from line 0.
    let out = runs(&s, 1, "b */ x");
    assert_eq!(out.first().map(|r| r.range.clone()), Some(0..4));
    assert_eq!(out[0].style.color, runs(&s, 0, "/* a")[0].style.color);
}

#[test]
fn the_plain_style_is_the_text_color_without_bold_or_underline() {
    let s = styler(
        "x\n",
        rules(|t| {
            let c = t.colors.get_mut("TXT").unwrap();
            c.text = 0x00ff00;
            c.bold = true;
            c.underline = true;
        }),
    );
    let plain = s.plain();
    assert_eq!(plain.color, 0x00ff00);
    assert!(!plain.bold && !plain.underline);
}
