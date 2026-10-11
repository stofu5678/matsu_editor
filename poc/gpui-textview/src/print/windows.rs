//! Printing on Windows (ADR-0011): the print dialog and GDI, as Sakura does
//! (sakura_core/print/CPrint.cpp and CPrintPreview.cpp). The pages of the shared layout are laid
//! out again for the paper of the printer that the user picks. A real app keeps this in its
//! platform crate (`ume-platform`, ADR-0013), the only place with `unsafe`.

use std::path::Path;

use windows_sys::Win32::Foundation::{GlobalFree, HWND};
use windows_sys::Win32::Graphics::Gdi::{
    CLIP_DEFAULT_PRECIS, CreateDCW, CreateFontW, CreatePen, DEFAULT_CHARSET, DEFAULT_QUALITY,
    DeleteDC, DeleteObject, ExtTextOutW, FF_DONTCARE, FW_NORMAL, GetDeviceCaps, HDC, HFONT,
    LOGPIXELSX, LOGPIXELSY, LineTo, MoveToEx, OUT_DEFAULT_PRECIS, PHYSICALHEIGHT, PHYSICALOFFSETX,
    PHYSICALOFFSETY, PHYSICALWIDTH, PS_SOLID, SelectObject, SetBkMode, SetTextAlign, TA_BASELINE,
    TA_LEFT, TRANSPARENT,
};
use windows_sys::Win32::Storage::Xps::{AbortDoc, DOCINFOW, EndDoc, EndPage, StartDocW, StartPage};
use windows_sys::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, PD_HIDEPRINTTOFILE, PD_NOPAGENUMS, PD_NOSELECTION, PD_RETURNDC,
    PD_USEDEVMODECOPIESANDCOLLATE, PRINTDLGW, PrintDlgW,
};

use super::PrintJob;
use super::layout::{Layout, Page, Paper, PrintSetting, char_cells, font_runs};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// The Win32 window of a GPUI window, to own the print dialog. A number rather than an `HWND`,
/// so that it can go to the thread that shows the dialog.
pub fn owner(window: &gpui::Window) -> isize {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match HasWindowHandle::window_handle(window).map(|handle| handle.as_raw()) {
        Ok(RawWindowHandle::Win32(handle)) => handle.hwnd.get(),
        _ => 0,
    }
}

/// The title of the print job, as the header's `$f` shows the document.
fn title(job: &PrintJob) -> String {
    job.doc.file_name.clone().unwrap_or_else(|| "(無題)".into())
}

/// Shows the print dialog owned by `owner` and prints. Returns whether it printed: false when the
/// user cancels. It blocks until the dialog closes, so call it off the UI thread, as GPUI does
/// for its own dialogs.
pub fn print(owner: isize, job: PrintJob) -> Result<bool, String> {
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
            err => Err(format!("印刷ダイアログのエラー {err:#x}")),
        };
    }
    let result = print_dc(pd.hDC, &job, None);
    unsafe { DeleteDC(pd.hDC) };
    free(&pd);
    result.map(|_| true)
}

/// Prints without a dialog to `printer`, writing the job to `output` (the PDF file for
/// "Microsoft Print to PDF"). Returns the number of pages. For `--print-test` and the self-check.
pub fn print_to_file(printer: &str, output: &Path, job: &PrintJob) -> Result<usize, String> {
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
        return Err(format!("プリンター {printer} を開けません"));
    }
    let result = print_dc(hdc, job, Some(output));
    unsafe { DeleteDC(hdc) };
    result
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

/// The fonts for half-width and full-width characters, as Sakura makes them
/// (CPrintPreview::CreateFonts). Sakura gives the font height as the cell height of the GDI
/// font; this gives it as the size of the characters (em), as the preview and the Mac do.
struct Fonts {
    han: HFONT,
    zen: HFONT,
}

impl Fonts {
    fn new(setting: &PrintSetting, device: &Device) -> Result<Self, String> {
        let create = |face: &str| {
            let wide_face = wide(face);
            let font = unsafe {
                CreateFontW(
                    -pixels(setting.font_height, device.dpi_y),
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
                Err(format!("印刷のフォント {face} を作れません"))
            } else {
                Ok(font)
            }
        };
        let han = create(&setting.font_han)?;
        // Dropped on an error below, which deletes `han`.
        let mut fonts = Self { han, zen: han };
        if setting.font_zen != setting.font_han {
            fonts.zen = create(&setting.font_zen)?;
        }
        Ok(fonts)
    }
}

impl Drop for Fonts {
    fn drop(&mut self) {
        unsafe {
            if self.zen != self.han {
                DeleteObject(self.zen);
            }
            DeleteObject(self.han);
        }
    }
}

/// Lays the text out for the printer's paper and prints every page. Returns the number of pages.
fn print_dc(hdc: HDC, job: &PrintJob, output: Option<&Path>) -> Result<usize, String> {
    let device = Device::of(hdc)?;
    let layout =
        Layout::new(&job.lines, &job.setting, device.paper).map_err(|err| err.to_string())?;
    let pages = layout.page_count();
    if pages == 0 {
        // Sakura's STR_ERR_DLGPRNPRVW7.
        return Err("印刷するページがありません。".into());
    }
    let fonts = Fonts::new(layout.setting(), &device)?;
    let doc_name = wide(&title(job));
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
    for index in 0..pages {
        let page = layout.page(index, &job.lines, &job.doc);
        let printed = if unsafe { StartPage(hdc) } <= 0 {
            Err("ページを始められません")
        } else if !draw_page(hdc, &page, layout.setting(), &fonts, &device) {
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
fn draw_page(
    hdc: HDC,
    page: &Page,
    setting: &PrintSetting,
    fonts: &Fonts,
    device: &Device,
) -> bool {
    // StartPage resets the attributes of the DC on some drivers, so they are set on every page.
    // The text is placed by its baseline, which the layout gives.
    let old_font = unsafe {
        SetBkMode(hdc, TRANSPARENT as i32);
        SetTextAlign(hdc, TA_BASELINE | TA_LEFT);
        SelectObject(hdc, fonts.han)
    };
    let cell = setting.font_width;
    let mut drawn = true;
    for text in &page.texts {
        let positions: Vec<(usize, char, i32)> = text.positions(cell).collect();
        for (range, zen) in font_runs(&text.text) {
            let chars: Vec<_> = positions
                .iter()
                .filter(|(i, _, _)| range.contains(i))
                .collect();
            let Some(&&(_, _, start)) = chars.first() else {
                continue;
            };
            // Each character's advance takes it to the next cell, as Sakura gives ExtTextOut the
            // width of each character. A character outside the BMP takes two UTF-16 units, and
            // the advance goes on the first.
            let mut units = Vec::new();
            let mut dx = Vec::new();
            for &&(_, ch, x) in &chars {
                let mut buf = [0u16; 2];
                let encoded = ch.encode_utf16(&mut buf);
                units.extend_from_slice(encoded);
                let next = x + char_cells(ch) * cell;
                dx.push(device.x(next) - device.x(x));
                if encoded.len() == 2 {
                    dx.push(0);
                }
            }
            drawn &= unsafe {
                SelectObject(hdc, if zen { fonts.zen } else { fonts.han });
                ExtTextOutW(
                    hdc,
                    device.x(start),
                    device.y(text.baseline),
                    0,
                    std::ptr::null(),
                    units.as_ptr(),
                    units.len() as u32,
                    dx.as_ptr(),
                ) != 0
            };
        }
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
