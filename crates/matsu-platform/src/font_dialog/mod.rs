//! The font dialog of the OS: Windows' `ChooseFontW`, as Sakura shows it (`MySelectFont`,
//! `sakura_core/util/shell.cpp`).
//!
//! Only Windows has one that Sakura's users know. On macOS and Linux [`choose`] fails with
//! [`Error::Unavailable`] and the caller shows its own dialog (`matsu-ui`'s `font_dialog`), which has
//! the same parts. The call into the OS is not run by CI (it has no desktop); the user checks it.
//!
//! The font goes in and out as a [`FontSetting`] (a `LOGFONT` and the point size in 1/10 point,
//! `matsu-config`), the form the settings file keeps.

#[cfg(windows)]
mod windows;

use matsu_config::settings::FontSetting;

pub use crate::dialog::{Error, Owner};

/// What to ask the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// The font the dialog starts with (`CF_INITTOLOGFONTSTRUCT`).
    pub initial: FontSetting,
    /// Only fixed-pitch fonts are listed (`CF_FIXEDPITCHONLY`, the `FixedFontOnly` of `MySelectFont`).
    pub fixed_pitch_only: bool,
}

/// Whether this OS has a font dialog that [`choose`] can show.
pub const fn is_available() -> bool {
    cfg!(windows)
}

/// Shows the dialog, owned by `owner`, and waits for the user: the font chosen (the `LOGFONT` and
/// the size, as the dialog returns them), or `None` when the user cancels.
pub async fn choose(owner: Owner, request: Request) -> Result<Option<FontSetting>, Error> {
    #[cfg(windows)]
    {
        windows::show(owner, request).await
    }
    #[cfg(not(windows))]
    {
        let _ = (owner, request);
        Err(Error::Unavailable(
            "this OS has no font dialog of its own".to_owned(),
        ))
    }
}

/// `LF_FACESIZE`: the units of `lfFaceName`, with the end mark.
pub const FACE_SIZE: usize = 32;

/// A face name as `lfFaceName` holds it: UTF-16, cut to 31 units, the rest 0.
pub fn face_name_units(name: &str) -> [u16; FACE_SIZE] {
    let mut out = [0; FACE_SIZE];
    for (slot, unit) in out[..FACE_SIZE - 1].iter_mut().zip(name.encode_utf16()) {
        *slot = unit;
    }
    out
}

/// The face name of `lfFaceName`: up to the first 0.
pub fn face_name_from_units(units: &[u16]) -> String {
    let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    String::from_utf16_lossy(&units[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_face_name_round_trips_through_lffacename() {
        for name in ["Consolas", "ＭＳ ゴシック", ""] {
            assert_eq!(face_name_from_units(&face_name_units(name)), name);
        }
    }

    #[test]
    fn a_long_face_name_is_cut_to_31_units() {
        let long = "a".repeat(40);
        let units = face_name_units(&long);
        assert_eq!(units[FACE_SIZE - 1], 0, "the end mark stays");
        assert_eq!(face_name_from_units(&units), "a".repeat(31));
    }

    #[cfg(not(windows))]
    #[test]
    fn other_systems_have_no_dialog() {
        assert!(!is_available());
        let request = Request {
            initial: FontSetting::default(),
            fixed_pitch_only: false,
        };
        let result = ready(choose(Owner::default(), request));
        assert!(matches!(result, Err(Error::Unavailable(_))));
    }

    /// Polls a future that is ready at once.
    #[cfg(not(windows))]
    fn ready<T>(future: impl std::future::Future<Output = T>) -> T {
        use std::task::{Context, Poll, Waker};
        let mut future = std::pin::pin!(future);
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("the future is not ready"),
        }
    }
}
