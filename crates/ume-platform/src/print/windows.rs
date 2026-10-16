//! Printing on Windows (ADR-0011): the print dialog and GDI, as Sakura does
//! (sakura_core/print/CPrint.cpp and CPrintPreview.cpp). The pages of the shared layout are laid
//! out again for the paper of the printer that the user picks. Ported from the PoC of 2026-10-11
//! (`poc/gpui-textview/src/print/windows.rs`).

// The Win32 calls are `unsafe`. This module and `mac` are the only places of the program that
// need it (ADR-0013).
#![allow(unsafe_code)]

use std::path::Path;

use raw_window_handle::RawWindowHandle;
use ume_print::{FontKind, HAlign, Layout, Page, Paper};
use windows_sys::Win32::Foundation::{GlobalFree, HWND};
use windows_sys::Win32::Graphics::Gdi::{
    CLIP_DEFAULT_PRECIS, CreateDCW, CreateFontW, CreatePen, DEFAULT_CHARSET, DEFAULT_QUALITY,
    DeleteDC, DeleteObject, ExtTextOutW, FF_DONTCARE, FW_NORMAL, GetDeviceCaps, HDC, HFONT,
    LOGPIXELSX, LOGPIXELSY, LineTo, MoveToEx, OUT_DEFAULT_PRECIS, PHYSICALHEIGHT, PHYSICALOFFSETX,
    PHYSICALOFFSETY, PHYSICALWIDTH, PS_SOLID, SelectObject, SetBkMode, SetTextAlign, TA_BASELINE,
    TA_CENTER, TA_LEFT, TA_RIGHT, TRANSPARENT,
};
use windows_sys::Win32::Storage::Xps::{AbortDoc, DOCINFOW, EndDoc, EndPage, StartDocW, StartPage};
use windows_sys::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, PD_HIDEPRINTTOFILE, PD_NOPAGENUMS, PD_NOSELECTION, PD_RETURNDC,
    PD_USEDEVMODECOPIESANDCOLLATE, PRINTDLGW, PrintDlgW,
};

use super::draw::{cell_positions, font_spec};
use super::{Error, PrintJob};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// The Win32 window to own the print dialog. A number rather than an `HWND`, so that it can go
/// to the thread that shows the dialog. 0 (no owner) for a handle that is not a Win32 window.
pub fn owner(handle: RawWindowHandle) -> isize {
    match handle {
        RawWindowHandle::Win32(handle) => handle.hwnd.get(),
        _ => 0,
    }
}

/// Shows the print dialog owned by `owner` and prints. Returns whether it printed: false when the
/// user cancels. It blocks until the dialog closes, so call it off the UI thread, as GPUI does
/// for its own dialogs.
pub fn print(owner: isize, job: PrintJob) -> Result<bool, Error> {
    let mut pd: PRINTDLGW = unsafe { std::mem::zeroed() };
    pd.lStructSize = std::mem::size_of::<PRINTDLGW>() as u32;
    pd.hwndOwner = owner as HWND;
    // No page range; the printer driver makes the copies. Print to File is hidden: Sakura shows it
    // and then ignores it, printing to the printer.
    pd.Flags = PD_RETURNDC
        | PD_NOSELECTION
        | PD_NOPAGENUMS
        | PD_HIDEPRINTTOFILE
        | PD_USEDEVMODECOPIESANDCOLLATE;
    pd.nCopies = 1;
    let ok = unsafe { PrintDlgW(&mut pd) };
    // The dialog allocates these; the app owns them after it returns.
    let free = |pd: &PRINTDLGW| unsafe {
        if !pd.hDevMode.is_null() {
            GlobalFree(pd.hDevMode);
        }
        if !pd.hDevNames.is_null() {
            GlobalFree(pd.hDevNames);
        }
    };
    if ok == 0 {
        free(&pd);
        return match unsafe { CommDlgExtendedError() } {
            0 => Ok(false),
            err => Err(Error(format!("印刷ダイアログのエラー {err:#x}"))),
        };
    }
    let result = print_dc(pd.hDC, &job, None);
    unsafe { DeleteDC(pd.hDC) };
    free(&pd);
    result.map(|_| true).map_err(Error)
}

/// Prints without a dialog to `printer`, writing the job to `output` (the PDF file for
/// "Microsoft Print to PDF"). Returns the number of pages. For `--print-test` and the self-check.
pub fn print_to_file(printer: &str, output: &Path, job: &PrintJob) -> Result<usize, Error> {
    let device = wide(printer);
    let hdc = unsafe {
        CreateDCW(
            std::ptr::null(),
            device.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if hdc.is_null() {
        return Err(Error(format!("プリンター {printer} を開けません")));
    }
    let result = print_dc(hdc, job, Some(output));
    unsafe { DeleteDC(hdc) };
    result.map_err(Error)
}

/// 0.1 mm in device pixels, rounded.
fn pixels(v: i32, dpi: i32) -> i32 {
    (v * dpi + 127) / 254
}

/// The printer's paper and how to turn 0.1 mm on it into device pixels.
struct Device {
    dpi_x: i32,
    dpi_y: i32,
    /// The printable area's origin, in device pixels from the paper's corner. GDI draws from it.
    offset_x: i32,
    offset_y: i32,
    paper: Paper,
}

impl Device {
    fn of(hdc: HDC) -> Result<Self, String> {
        let caps = |i| unsafe { GetDeviceCaps(hdc, i as i32) };
        let (dpi_x, dpi_y) = (caps(LOGPIXELSX), caps(LOGPIXELSY));
        if dpi_x <= 0 || dpi_y <= 0 {
            return Err("プリンターの解像度を取得できません".into());
        }
        Ok(Self {
            dpi_x,
            dpi_y,
            offset_x: caps(PHYSICALOFFSETX),
            offset_y: caps(PHYSICALOFFSETY),
            // Rounded, so that A4 is 2100 x 2970 as in the preview: 2099 would take a cell off
            // each line.
            paper: Paper {
                width: (caps(PHYSICALWIDTH) * 254 + dpi_x / 2) / dpi_x,
                height: (caps(PHYSICALHEIGHT) * 254 + dpi_y / 2) / dpi_y,
            },
        })
    }

    /// 0.1 mm across the paper to device pixels. Each position is rounded on its own, so that
    /// the cells of a long line do not drift.
    fn x(&self, v: i32) -> i32 {
        pixels(v, self.dpi_x) - self.offset_x
    }

    fn y(&self, v: i32) -> i32 {
        pixels(v, self.dpi_y) - self.offset_y
    }
}

/// The fonts of a setting, as Sakura makes them (CPrintPreview::CreateFonts): the print fonts
/// for half-width and full-width characters and, when the setting has them, the header's and the
/// footer's. Sakura gives the font height as the cell height of the GDI font; this gives it as
/// the size of the characters (em), as the preview and the Mac do.
struct Fonts {
    han: HFONT,
    zen: HFONT,
    header: Option<HFONT>,
    footer: Option<HFONT>,
}

impl Fonts {
    fn new(layout: &Layout, device: &Device) -> Result<Self, String> {
        // `Fonts` is dropped on an error, which deletes what was made so far.
        let mut fonts = Self {
            han: std::ptr::null_mut(),
            zen: std::ptr::null_mut(),
            header: None,
            footer: None,
        };
        let setting = layout.setting();
        let create = |kind| {
            let spec = font_spec(setting, kind);
            let wide_face = wide(spec.face);
            let font = unsafe {
                CreateFontW(
                    -pixels(spec.height, device.dpi_y),
                    // The width follows the height; the cells come from ExtTextOutW's advances.
                    0,
                    0,
                    0,
                    FW_NORMAL as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET as u32,
                    OUT_DEFAULT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32,
                    DEFAULT_QUALITY as u32,
                    FF_DONTCARE as u32,
                    wide_face.as_ptr(),
                )
            };
            if font.is_null() {
                Err(format!("印刷のフォント {} を作れません", spec.face))
            } else {
                Ok(font)
            }
        };
        fonts.han = create(FontKind::Han)?;
        fonts.zen = create(FontKind::Zen)?;
        if setting
            .header_font
            .as_ref()
            .is_some_and(|f| !f.face.is_empty())
        {
            fonts.header = Some(create(FontKind::Header)?);
        }
        if setting
            .footer_font
            .as_ref()
            .is_some_and(|f| !f.face.is_empty())
        {
            fonts.footer = Some(create(FontKind::Footer)?);
        }
        Ok(fonts)
    }

    fn of(&self, kind: FontKind) -> HFONT {
        match kind {
            FontKind::Han => self.han,
            FontKind::Zen => self.zen,
            FontKind::Header => self.header.unwrap_or(self.han),
            FontKind::Footer => self.footer.unwrap_or(self.han),
        }
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        for font in [Some(self.han), Some(self.zen), self.header, self.footer]
            .into_iter()
            .flatten()
            .filter(|font| !font.is_null())
        {
            unsafe { DeleteObject(font) };
        }
    }
}

/// Lays the text out for the printer's paper and prints every page. Returns the number of pages.
fn print_dc(hdc: HDC, job: &PrintJob, output: Option<&Path>) -> Result<usize, String> {
    let device = Device::of(hdc)?;
    let layout = job.layout(device.paper).map_err(|err| err.to_string())?;
    let pages = layout.page_count();
    if pages == 0 {
        // Sakura's STR_ERR_DLGPRNPRVW7.
        return Err("印刷するページがありません。".into());
    }
    let fonts = Fonts::new(&layout, &device)?;
    let doc_name = wide(&job.title);
    let output = output.map(|path| wide(&path.to_string_lossy()));
    let info = DOCINFOW {
        cbSize: std::mem::size_of::<DOCINFOW>() as i32,
        lpszDocName: doc_name.as_ptr(),
        lpszOutput: output.as_ref().map_or(std::ptr::null(), |o| o.as_ptr()),
        lpszDatatype: std::ptr::null(),
        fwType: 0,
    };
    if unsafe { StartDocW(hdc, &info) } <= 0 {
        return Err("印刷を始められません".into());
    }
    for page in layout.pages(0..pages, &*job.source, &job.header) {
        let printed = if unsafe { StartPage(hdc) } <= 0 {
            Err("ページを始められません")
        } else if !draw_page(hdc, &page, &layout, &fonts, &device) {
            Err("文字を描けません")
        } else if unsafe { EndPage(hdc) } <= 0 {
            Err("ページを終えられません")
        } else {
            Ok(())
        };
        if let Err(err) = printed {
            unsafe { AbortDoc(hdc) };
            return Err(err.into());
        }
    }
    if unsafe { EndDoc(hdc) } <= 0 {
        unsafe { AbortDoc(hdc) };
        return Err("印刷を終えられません".into());
    }
    Ok(pages)
}

/// Draws a page. Returns whether all its text was drawn.
fn draw_page(hdc: HDC, page: &Page, layout: &Layout, fonts: &Fonts, device: &Device) -> bool {
    // StartPage resets the attributes of the DC on some drivers, so they are set on every page.
    // The text is placed by its baseline, which the layout gives.
    let old_font = unsafe {
        SetBkMode(hdc, TRANSPARENT as i32);
        SelectObject(hdc, fonts.han)
    };
    let cell = layout.setting().print_font_width;
    let rule = layout.width_rule();
    let mut drawn = true;
    for text in &page.texts {
        // Each character's advance takes it to the next cell, as Sakura gives ExtTextOut the
        // width of each character. A character outside the BMP takes two UTF-16 units, and the
        // advance goes on the first. A header or a footer in a font of its own is not on the
        // grid: GDI places it by the alignment.
        let mut units = Vec::new();
        let mut dx = Vec::new();
        let on_grid = super::draw::on_grid(text.font);
        if on_grid {
            for (_, ch, x) in cell_positions(text, layout) {
                let mut buf = [0u16; 2];
                let encoded = ch.encode_utf16(&mut buf);
                units.extend_from_slice(encoded);
                let next = x + rule.columns(ch) as i32 * cell;
                dx.push(device.x(next) - device.x(x));
                if encoded.len() == 2 {
                    dx.push(0);
                }
            }
        } else {
            units.extend(text.text.encode_utf16());
        }
        if units.is_empty() {
            continue;
        }
        let align = match text.align {
            HAlign::Left => TA_LEFT,
            HAlign::Center => TA_CENTER,
            HAlign::Right => TA_RIGHT,
        };
        drawn &= unsafe {
            SetTextAlign(hdc, TA_BASELINE | align);
            SelectObject(hdc, fonts.of(text.font));
            ExtTextOutW(
                hdc,
                device.x(text.x),
                device.y(text.baseline),
                0,
                std::ptr::null(),
                units.as_ptr(),
                units.len() as u32,
                if on_grid {
                    dx.as_ptr()
                } else {
                    std::ptr::null()
                },
            ) != 0
        };
    }
    // The rules between the line numbers and the text (CPrintPreview::DrawPageText). Sakura
    // draws them in the DC's pen, a pixel wide; this pen is 0.5 pt wide, as on the Mac.
    if !page.rules.is_empty() {
        let pen = unsafe { CreatePen(PS_SOLID, (device.dpi_x / 144).max(1), 0) };
        let old_pen = unsafe { SelectObject(hdc, pen) };
        for rule in &page.rules {
            unsafe {
                MoveToEx(
                    hdc,
                    device.x(rule.from.0),
                    device.y(rule.from.1),
                    std::ptr::null_mut(),
                );
                LineTo(hdc, device.x(rule.to.0), device.y(rule.to.1));
            }
        }
        unsafe {
            SelectObject(hdc, old_pen);
            DeleteObject(pen);
        }
    }
    unsafe { SelectObject(hdc, old_font) };
    drawn
}
