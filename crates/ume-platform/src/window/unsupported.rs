//! Linux (and other Unix without a native module): a program cannot place its own windows.

use super::{Error, RawWindowHandle, Rect};

const WHY: &str =
    "ウィンドウの配置 (Wayland のコンポジタは、プログラムがウィンドウを動かすことを認めない)";

pub(super) fn unsupported() -> Option<&'static str> {
    Some(WHY)
}

pub(super) fn work_area(_: RawWindowHandle) -> Result<Rect, Error> {
    Err(Error::Unsupported(WHY))
}

pub(super) fn frame(_: RawWindowHandle) -> Result<Rect, Error> {
    Err(Error::Unsupported(WHY))
}

pub(super) fn is_minimized(_: RawWindowHandle) -> Result<bool, Error> {
    Err(Error::Unsupported(WHY))
}

pub(super) fn place(_: RawWindowHandle, _: Rect) -> Result<(), Error> {
    Err(Error::Unsupported(WHY))
}

pub(super) fn cascade_step() -> (i32, i32) {
    (30, 30)
}

pub(super) fn is_topmost(_: RawWindowHandle) -> Result<bool, Error> {
    Err(Error::Unsupported("常に手前に表示"))
}

pub(super) fn set_topmost(_: RawWindowHandle, _: bool) -> Result<(), Error> {
    Err(Error::Unsupported("常に手前に表示"))
}

pub(super) fn restore(_: RawWindowHandle) -> Result<(), Error> {
    Err(Error::Unsupported("ウィンドウを元の大きさに戻す"))
}
