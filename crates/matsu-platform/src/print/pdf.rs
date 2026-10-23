//! Writes the pages of the layout to a PDF with krilla (MIT OR Apache-2.0), embedding only the
//! glyphs used. Linux prints this PDF through the print portal; the tests use it to check the
//! pages without a printer. Ported from the PoC of 2026-10-11 (`poc/gpui-textview/src/print/pdf.rs`).
//!
//! krilla neither shapes the text nor falls back to other fonts: each character is drawn on its
//! own in the print font, so a character that the font lacks prints as the font's empty glyph
//! (ADR-0011, "残っている確認"). A header or a footer in a font of its own is drawn from its left
//! edge: centering needs the width of the text, which is a follow-up.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use krilla::Document;
use krilla::geom::{PathBuilder, Point};
use krilla::page::PageSettings;
use krilla::paint::Stroke;
use krilla::text::{Font, TextDirection};
use matsu_print::Paper;

use super::draw::{DrawOp, font_spec, page_ops, points};
use super::{Error, PrintJob};

/// Lays the document out on `paper` and writes the pages to a PDF. Returns the PDF and the
/// number of pages.
pub fn render(job: &PrintJob, paper: Paper) -> Result<(Vec<u8>, usize), Error> {
    let layout = job.layout(paper)?;
    let pages = layout.page_count();
    if pages == 0 {
        // Sakura's STR_ERR_DLGPRNPRVW7.
        return Err("印刷するページがありません。".into());
    }
    let setting = layout.setting();
    let mut fonts = Fonts::default();
    let settings = PageSettings::from_wh(points(paper.width), points(paper.height))
        .ok_or("用紙の大きさが不正です")?;
    let mut document = Document::new();
    for page in layout.pages(0..pages, &*job.source, &job.header) {
        let mut pdf_page = document.start_page_with(settings.clone());
        let mut surface = pdf_page.surface();
        let mut buf = [0u8; 4];
        let ops = page_ops(&layout, &page);
        // One character at a time, so that each lands on its cell whatever its advance in the
        // font is.
        for op in &ops {
            match op {
                DrawOp::Char {
                    ch,
                    x,
                    baseline,
                    font,
                } => {
                    let spec = font_spec(setting, *font);
                    surface.draw_text(
                        Point::from_xy(points(*x), points(*baseline)),
                        fonts.get(spec.face)?,
                        points(spec.height),
                        ch.encode_utf8(&mut buf),
                        false,
                        TextDirection::Auto,
                    );
                }
                DrawOp::Text {
                    text,
                    x,
                    baseline,
                    font,
                    ..
                } => {
                    let spec = font_spec(setting, *font);
                    surface.draw_text(
                        Point::from_xy(points(*x), points(*baseline)),
                        fonts.get(spec.face)?,
                        points(spec.height),
                        text,
                        false,
                        TextDirection::Auto,
                    );
                }
                DrawOp::Line { .. } => {}
            }
        }
        // The rules between the line numbers and the text, 0.5 pt wide as on the Mac.
        surface.set_stroke(Some(Stroke {
            width: 0.5,
            ..Stroke::default()
        }));
        for op in &ops {
            if let DrawOp::Line { from, to } = op {
                let mut path = PathBuilder::new();
                path.move_to(points(from.0), points(from.1));
                path.line_to(points(to.0), points(to.1));
                if let Some(path) = path.finish() {
                    surface.draw_path(&path);
                }
            }
        }
        surface.finish();
        pdf_page.finish();
    }
    let pdf = document
        .finish()
        .map_err(|err| Error(format!("PDF を作れません: {err:?}")))?;
    Ok((pdf, pages))
}

/// The fonts of the families used, each loaded once.
#[derive(Default)]
struct Fonts(HashMap<String, Font>);

impl Fonts {
    fn get(&mut self, family: &str) -> Result<Font, Error> {
        if let Some(font) = self.0.get(family) {
            return Ok(font.clone());
        }
        let font = load_font(family)?;
        self.0.insert(family.to_owned(), font.clone());
        Ok(font)
    }
}

fn load_font(family: &str) -> Result<Font, Error> {
    let (path, index) = find_font(family)?;
    let data = std::fs::read(&path).map_err(|err| Error(format!("{}: {err}", path.display())))?;
    Font::new(Arc::new(data).into(), index)
        .ok_or_else(|| Error(format!("フォントを読めません: {}", path.display())))
}

/// The environment variable that names a font file (`<path>` or `<path>:<index>`) to use for
/// every family, for a machine where the print font is not installed.
pub const FONT_ENV: &str = "MATSU_PRINT_FONT";

/// The file of a font family and the index of the font in it.
fn find_font(family: &str) -> Result<(PathBuf, u32), Error> {
    if let Ok(spec) = std::env::var(FONT_ENV) {
        return Ok(match spec.rsplit_once(':') {
            Some((path, index)) if let Ok(index) = index.parse() => (path.into(), index),
            _ => (spec.into(), 0),
        });
    }
    system_font(family).ok_or_else(|| {
        Error(format!(
            "印刷のフォント {family} が見つかりません（{FONT_ENV} で指定できます）"
        ))
    })
}

/// fontconfig finds the font, and the face of a collection such as Noto Serif CJK that has the
/// Japanese glyphs. For a family that is not installed, it gives the font it would use instead.
#[cfg(not(windows))]
fn system_font(family: &str) -> Option<(PathBuf, u32)> {
    let out = std::process::Command::new("fc-match")
        .args(["-f", "%{file}\n%{index}", family])
        .output()
        .ok()?;
    let out = String::from_utf8_lossy(&out.stdout);
    let mut lines = out.lines();
    let file = lines.next().filter(|file| !file.is_empty())?;
    Some((file.into(), lines.next()?.parse().unwrap_or(0)))
}

/// The Japanese fonts that come with Windows, by the names that GDI also takes.
#[cfg(windows)]
fn system_font(family: &str) -> Option<(PathBuf, u32)> {
    let file = match family {
        "MS Mincho" | "ＭＳ 明朝" => "msmincho.ttc",
        "MS Gothic" | "ＭＳ ゴシック" => "msgothic.ttc",
        "BIZ UDMincho" | "BIZ UD明朝" => "BIZ-UDMinchoM.ttc",
        "BIZ UDGothic" | "BIZ UDゴシック" => "BIZ-UDGothicR.ttc",
        _ => return None,
    };
    let windir = std::env::var_os("WINDIR").unwrap_or_else(|| r"C:\Windows".into());
    let path = PathBuf::from(windir).join("Fonts").join(file);
    path.exists().then_some((path, 0))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use matsu_print::{HeaderContext, TypeOptions};

    use super::*;
    use crate::print::default_setting;

    fn job(lines: Vec<String>) -> PrintJob {
        PrintJob {
            source: Arc::new(lines),
            setting: default_setting(),
            options: TypeOptions::default(),
            header: HeaderContext::default(),
            title: "test".into(),
        }
    }

    #[test]
    fn an_empty_document_has_nothing_to_print() {
        // Sakura's STR_ERR_DLGPRNPRVW7. Needs no font.
        let err = render(&job(Vec::new()), Paper::A4).unwrap_err();
        assert_eq!(err.0, "印刷するページがありません。");
    }

    /// The font needs a machine that has it; without one the test says so and passes (CI images
    /// have no Japanese font on some of the OSes).
    #[test]
    fn the_pages_are_written() {
        let lines: Vec<String> = (0..90).map(|i| format!("line {i} 日本語")).collect();
        let mut lines = lines;
        lines.push(String::new());
        let job = job(lines);
        let (pdf, pages) = match render(&job, Paper::A4) {
            Ok(done) => done,
            Err(err) => {
                eprintln!("skipped: {err}");
                return;
            }
        };
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pages > 1, "90 lines do not fit a page");
        let layout = job.layout(Paper::A4).unwrap();
        assert_eq!(pages, layout.page_count());
        assert!(pdf.ends_with(b"%%EOF\n") || pdf.ends_with(b"%%EOF"));
    }
}
