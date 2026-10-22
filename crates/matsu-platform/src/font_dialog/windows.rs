//! `ChooseFontW` with the flags of Sakura's `MySelectFont` (`sakura_core/util/shell.cpp`):
//! `CF_SCREENFONTS | CF_INITTOLOGFONTSTRUCT | CF_EFFECTS`, and `CF_FIXEDPITCHONLY` when only
//! fixed-pitch fonts are wanted. (Sakura also sets `CF_ENABLEHOOK` for its dark mode; MATSU Editor has
//! no such hook.) The `LOGFONT` and `iPointSize` the dialog returns are the result, as they are.
//!
//! Not run by CI, which has no desktop; the user checks it. Like the file dialogs, it runs on a
//! thread of its own, so that the UI thread keeps running while it is open.

// The Win32 API is `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use futures_channel::oneshot;
use matsu_config::settings::{FontSetting, LogFont};
use windows_sys::Win32::Graphics::Gdi::{GetDC, GetDeviceCaps, LOGFONTW, LOGPIXELSY, ReleaseDC};
use windows_sys::Win32::UI::Controls::Dialogs::{
    CF_EFFECTS, CF_FIXEDPITCHONLY, CF_INITTOLOGFONTSTRUCT, CF_SCREENFONTS, CHOOSEFONTW, ChooseFontW,
};

use super::{Error, Owner, Request, face_name_from_units, face_name_units};

pub(super) async fn show(owner: Owner, request: Request) -> Result<Option<FontSetting>, Error> {
    let (done, answer) = oneshot::channel();
    std::thread::Builder::new()
        .name("font dialog".to_owned())
        .spawn(move || {
            done.send(run(owner, &request)).ok();
        })
        .map_err(|e| Error::Failed(e.to_string()))?;
    answer
        .await
        .map_err(|_| Error::Failed("the dialog thread ended".to_owned()))?
}

fn to_logfont(font: &LogFont) -> LOGFONTW {
    LOGFONTW {
        lfHeight: font.height,
        lfWidth: font.width,
        lfEscapement: font.escapement,
        lfOrientation: font.orientation,
        lfWeight: font.weight,
        lfItalic: font.italic,
        lfUnderline: font.underline,
        lfStrikeOut: font.strike_out,
        lfCharSet: font.char_set,
        lfOutPrecision: font.out_precision,
        lfClipPrecision: font.clip_precision,
        lfQuality: font.quality,
        lfPitchAndFamily: font.pitch_and_family,
        lfFaceName: face_name_units(&font.face_name),
    }
}

fn from_logfont(font: &LOGFONTW) -> LogFont {
    LogFont {
        height: font.lfHeight,
        width: font.lfWidth,
        escapement: font.lfEscapement,
        orientation: font.lfOrientation,
        weight: font.lfWeight,
        italic: font.lfItalic,
        underline: font.lfUnderline,
        strike_out: font.lfStrikeOut,
        char_set: font.lfCharSet,
        out_precision: font.lfOutPrecision,
        clip_precision: font.lfClipPrecision,
        quality: font.lfQuality,
        pitch_and_family: font.lfPitchAndFamily,
        face_name: face_name_from_units(&font.lfFaceName),
    }
}

/// The vertical resolution of the screen, which `ChooseFontW` turns the height into points with.
fn screen_dpi() -> i32 {
    // SAFETY: the device context of the whole screen is released at once.
    unsafe {
        let dc = GetDC(std::ptr::null_mut());
        let dpi = GetDeviceCaps(dc, LOGPIXELSY as i32);
        ReleaseDC(std::ptr::null_mut(), dc);
        if dpi > 0 { dpi } else { 96 }
    }
}

fn run(owner: Owner, request: &Request) -> Result<Option<FontSetting>, Error> {
    let mut log_font = to_logfont(&request.initial.log_font);
    // `CF_INITTOLOGFONTSTRUCT` starts at the height; the point size is what the file means, and the
    // height of a file may come from another resolution.
    if request.initial.point_size != 0 {
        log_font.lfHeight = request.initial.height_for_dpi(screen_dpi());
    }
    let mut flags = CF_SCREENFONTS | CF_INITTOLOGFONTSTRUCT | CF_EFFECTS;
    if request.fixed_pitch_only {
        flags |= CF_FIXEDPITCHONLY;
    }
    // SAFETY: `CHOOSEFONTW` is plain data for which all zeros is the empty request (no hook, no
    // template, no device context).
    let mut choose: CHOOSEFONTW = unsafe { std::mem::zeroed() };
    choose.lStructSize = std::mem::size_of::<CHOOSEFONTW>() as u32;
    choose.hwndOwner = owner.hwnd() as _;
    choose.lpLogFont = &mut log_font;
    choose.Flags = flags;
    // SAFETY: `choose` and the `LOGFONTW` it points to outlive the call, which is modal.
    let chosen = unsafe { ChooseFontW(&mut choose) };
    if chosen == 0 {
        // Cancelled, or failed; Sakura treats both as "not chosen".
        return Ok(None);
    }
    Ok(Some(FontSetting {
        log_font: from_logfont(&log_font),
        point_size: choose.iPointSize,
    }))
}
