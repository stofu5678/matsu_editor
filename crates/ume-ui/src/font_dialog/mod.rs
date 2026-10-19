//! Fonts: the font dialog (`MySelectFont`, `sakura_core/util/shell.cpp`) and what the font
//! functions do with the settings.
//!
//! * [`choose`] asks the user for a font: Windows' `ChooseFont` through `ume_platform`, as Sakura
//!   does; on macOS and Linux (and where the OS dialog fails) [`dialog`], UME Editor's own dialog
//!   with the same parts: the family, the style, the size and a sample.
//! * [`model`] is what the dialog holds and how it becomes a `LOGFONT` and a size.
//! * [`apply`] writes the fonts to the settings file and lets the windows take them.
//!
//! The users of the dialog: `F_FONT`, the font of a document type (the スクリーン page of the type
//! settings) and the font of the tab bar (the タブバー page of the common settings).

pub(crate) mod apply;
pub(crate) mod dialog;
pub(crate) mod model;

use gpui::{App, Window};
use raw_window_handle::HasWindowHandle;
use ume_config::settings::FontSetting;
use ume_platform::font_dialog::Owner;

pub(crate) use dialog::Done;

/// The window the dialog of the OS belongs to.
pub(crate) fn owner_of(window: &Window) -> Owner {
    HasWindowHandle::window_handle(window)
        .ok()
        .map(|handle| Owner::from_handle(handle.as_raw()))
        .unwrap_or_default()
}

/// The dialog of the OS. The tests of the windows have no desktop to show it on.
async fn native_dialog(
    owner: Owner,
    font: FontSetting,
) -> Result<Option<FontSetting>, ume_platform::dialog::Error> {
    #[cfg(test)]
    {
        let _ = (owner, font);
        Err(ume_platform::dialog::Error::Unavailable(
            "no desktop in the tests".to_owned(),
        ))
    }
    #[cfg(not(test))]
    {
        // `CF_INITTOLOGFONTSTRUCT` starts at the height; the point size is what the file means.
        let mut font = font;
        if font.point_size != 0 {
            font.log_font.height = font.height_for_dpi(crate::settings::view_settings::DPI);
        }
        ume_platform::font_dialog::choose(
            owner,
            ume_platform::font_dialog::Request {
                initial: font,
                // `bFixedFont = false` in `Command_FONT` and in the pages: any font is listed.
                fixed_pitch_only: false,
            },
        )
        .await
    }
}

/// Asks the user for a font, starting at `font`; `done` gets the font when the user presses OK
/// (nothing happens when the user cancels). It may run after the call has returned.
pub(crate) fn choose(cx: &mut App, owner: Owner, font: FontSetting, done: Done) {
    cx.spawn(async move |cx| {
        match native_dialog(owner, font.clone()).await {
            Ok(Some(chosen)) => {
                cx.update(|cx| done(cx, chosen));
            }
            Ok(None) => {}
            // Whatever stopped the dialog of the OS, UME Editor's own is still there.
            Err(_) => {
                cx.update(|cx| {
                    dialog::open(cx, font, done);
                });
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests;
