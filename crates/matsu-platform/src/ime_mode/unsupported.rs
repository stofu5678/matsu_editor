//! macOS and Linux: a program cannot open or close the IME (see the module documentation).

use super::{Error, RawWindowHandle, Steps};

const WHY: &str = "この OS では、プログラムから IME のオン・オフと入力モードを決められない";

pub(super) fn unsupported() -> Option<&'static str> {
    Some(WHY)
}

pub(super) fn apply(_: RawWindowHandle, _: Steps) -> Result<(), Error> {
    Err(Error::Unsupported(WHY))
}
