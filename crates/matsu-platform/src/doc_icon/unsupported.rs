//! macOS and Linux: a program cannot change the icon of its window per document (see the module
//! documentation).

use std::path::Path;

use super::{Error, RawWindowHandle};

const WHY: &str = "この OS では、ウィンドウのアイコンを文書ごとに変えられない";

pub(super) fn unsupported() -> Option<&'static str> {
    Some(WHY)
}

pub(super) fn set(_: RawWindowHandle, _: Option<&Path>) -> Result<(), Error> {
    Err(Error::Unsupported(WHY))
}
