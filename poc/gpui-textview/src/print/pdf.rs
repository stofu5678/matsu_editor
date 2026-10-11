//! Writes the pages of the shared layout to a PDF with krilla, embedding only the glyphs used.
//! Linux prints this PDF through the print portal; it also backs `--print-pdf`, on Windows too.
//!
//! krilla neither shapes the text nor falls back to other fonts: each character is drawn on its
//! own in the print font, so a character that the font lacks prints as the font's empty glyph.

use std::path::PathBuf;
use std::sync::Arc;

use krilla::Document;
use krilla::geom::{PathBuilder, Point};
use krilla::page::PageSettings;
use krilla::paint::Stroke;
use krilla::text::{Font, TextDirection};

use super::PrintJob;
use super::layout::{Layout, Paper, uses_zen_font};

/// 0.1 mm in points, the unit of PDF.
fn pt(v: i32) -> f32 {
    v as f32 * 72.0 / 254.0
}

/// Lays the text out on `paper` and writes the pages to a PDF. Returns the PDF and the number of
/// pages.
pub fn render(job: &PrintJob, paper: Paper) -> Result<(Vec<u8>, usize), String> {
    let layout = Layout::new(&job.lines, &job.setting, paper).map_err(|err| err.to_string())?;
    let s = layout.setting();
    let han = load_font(&s.font_han)?;
    let zen = if s.font_zen == s.font_han {
        han.clone()
    } else {
        load_font(&s.font_zen)?
    };
    let size = pt(s.font_height);
    let settings =
        PageSettings::from_wh(pt(paper.width), pt(paper.height)).ok_or("用紙の大きさが不正です")?;
    let mut document = Document::new();
    let pages = layout.page_count();
    for index in 0..pages {
        let page = layout.page(index, &job.lines, &job.doc);
        let mut pdf_page = document.start_page_with(settings.clone());
        let mut surface = pdf_page.surface();
        // One character at a time, so that each lands on its cell whatever its advance in the
        // font is.
        let mut buf = [0u8; 4];
        for text in &page.texts {
            for (_, ch, x) in text.positions(s.font_width) {
                let font = if uses_zen_font(ch) { &zen } else { &han };
                surface.draw_text(
                    Point::from_xy(pt(x), pt(text.baseline)),
                    font.clone(),
                    size,
                    ch.encode_utf8(&mut buf),
                    false,
                    TextDirection::Auto,
                );
            }
        }
        // The rules between the line numbers and the text, 0.5 pt wide as on the Mac.
        surface.set_stroke(Some(Stroke {
            width: 0.5,
            ..Stroke::default()
        }));
        for rule in &page.rules {
            let mut path = PathBuilder::new();
            path.move_to(pt(rule.from.0), pt(rule.from.1));
            path.line_to(pt(rule.to.0), pt(rule.to.1));
            if let Some(path) = path.finish() {
                surface.draw_path(&path);
            }
        }
        surface.finish();
        pdf_page.finish();
    }
    let pdf = document
        .finish()
        .map_err(|err| format!("PDF を作れません: {err:?}"))?;
    Ok((pdf, pages))
}

fn load_font(family: &str) -> Result<Font, String> {
    let (path, index) = find_font(family)?;
    let data = std::fs::read(&path).map_err(|err| format!("{}: {err}", path.display()))?;
    Font::new(Arc::new(data).into(), index)
        .ok_or_else(|| format!("フォントを読めません: {}", path.display()))
}

/// The file of a font family and the index of the font in it. `POC_PRINT_FONT=<path>[:<index>]`
/// overrides it for every family.
fn find_font(family: &str) -> Result<(PathBuf, u32), String> {
    if let Ok(spec) = std::env::var("POC_PRINT_FONT") {
        return Ok(match spec.rsplit_once(':') {
            Some((path, index)) if let Ok(index) = index.parse() => (path.into(), index),
            _ => (spec.into(), 0),
        });
    }
    system_font(family).ok_or_else(|| {
        format!("印刷のフォント {family} が見つかりません（POC_PRINT_FONT で指定できます）")
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
