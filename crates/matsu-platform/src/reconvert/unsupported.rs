//! macOS and Linux: a program cannot start a reconversion (see the module documentation).

use super::{Error, RawWindowHandle, Reconvert};

const WHY: &str = "この OS では、プログラムから再変換を始められない (日本語入力のほうから始める)";

pub(super) fn query(_: RawWindowHandle, _: &mut Reconvert) -> Result<(), Error> {
    Err(Error::Unsupported(WHY))
}

pub(super) fn set(_: RawWindowHandle, _: &Reconvert) -> Result<(), Error> {
    Err(Error::Unsupported(WHY))
}
