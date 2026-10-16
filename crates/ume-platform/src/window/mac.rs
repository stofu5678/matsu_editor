//! Placing windows on macOS through AppKit's `NSWindow` and `NSScreen`. Not run by CI, which has
//! no desktop; the user checks it on a Mac.
//!
//! Cocoa's y axis points up from the bottom left of the primary screen; the rectangles of this
//! module point down from its top left, as everywhere else, so every rectangle is flipped with
//! the height of the primary screen.

use objc2::MainThreadMarker;
use objc2_app_kit::NSScreen;
use objc2_foundation::{NSPoint, NSRect, NSSize};

use super::{Error, RawWindowHandle, Rect};
use crate::print::mac::ns_window;

/// `kCGFloatingWindowLevel`: above the normal windows (Sakura's `HWND_TOPMOST`).
const FLOATING_LEVEL: isize = 3;
/// `kCGNormalWindowLevel` (`HWND_NOTOPMOST`).
const NORMAL_LEVEL: isize = 0;

fn window(handle: RawWindowHandle) -> Result<objc2::rc::Retained<objc2_app_kit::NSWindow>, Error> {
    ns_window(handle).ok_or(Error::Failed("AppKit のウィンドウではない"))
}

/// The height of the primary screen, the one with the menu bar, which is the origin of Cocoa's
/// coordinates.
fn primary_height() -> Result<f64, Error> {
    let mtm = MainThreadMarker::new().ok_or(Error::Failed("メインスレッドでの画面の取得"))?;
    let screens = NSScreen::screens(mtm);
    let primary = screens.firstObject().ok_or(Error::Failed("画面の取得"))?;
    Ok(primary.frame().size.height)
}

fn to_rect(ns: NSRect, primary_height: f64) -> Rect {
    let left = ns.origin.x;
    let top = primary_height - (ns.origin.y + ns.size.height);
    Rect::new(
        left.round() as i32,
        top.round() as i32,
        (left + ns.size.width).round() as i32,
        (top + ns.size.height).round() as i32,
    )
}

fn to_ns(rect: Rect, primary_height: f64) -> NSRect {
    NSRect::new(
        NSPoint::new(
            f64::from(rect.left),
            primary_height - f64::from(rect.bottom),
        ),
        NSSize::new(f64::from(rect.width()), f64::from(rect.height())),
    )
}

pub(super) fn work_area(handle: RawWindowHandle) -> Result<Rect, Error> {
    let window = window(handle)?;
    let height = primary_height()?;
    let screen = window.screen().ok_or(Error::Failed("画面の取得"))?;
    Ok(to_rect(screen.visibleFrame(), height))
}

pub(super) fn frame(handle: RawWindowHandle) -> Result<Rect, Error> {
    let window = window(handle)?;
    Ok(to_rect(window.frame(), primary_height()?))
}

pub(super) fn is_minimized(handle: RawWindowHandle) -> Result<bool, Error> {
    Ok(window(handle)?.isMiniaturized())
}

pub(super) fn place(handle: RawWindowHandle, to: Rect) -> Result<(), Error> {
    let window = window(handle)?;
    let height = primary_height()?;
    if window.isMiniaturized() {
        window.deminiaturize(None);
    }
    // `zoom:` toggles; a zoomed window would keep its zoomed state otherwise.
    if window.isZoomed() {
        window.zoom(None);
    }
    window.setFrame_display(to_ns(to, height), true);
    Ok(())
}

pub(super) fn cascade_step() -> (i32, i32) {
    // The height of a title bar.
    (28, 28)
}

pub(super) fn is_topmost(handle: RawWindowHandle) -> Result<bool, Error> {
    Ok(window(handle)?.level() > NORMAL_LEVEL)
}

pub(super) fn set_topmost(handle: RawWindowHandle, topmost: bool) -> Result<(), Error> {
    window(handle)?.setLevel(if topmost {
        FLOATING_LEVEL
    } else {
        NORMAL_LEVEL
    });
    Ok(())
}
