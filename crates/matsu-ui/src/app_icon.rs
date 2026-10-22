//! The icon of MATSU Editor (`assets/icons`, see its README.md).
//!
//! - Windows: the icon is a resource of `matsu.exe` (ID 1, `crates/matsu/build.rs`), which GPUI uses for
//!   its windows. Nothing is passed from here.
//! - Linux (X11): GPUI takes the icon as an image in `WindowOptions::icon` and sets
//!   `_NET_WM_ICON`. Wayland and macOS take no icon from the app (a `.desktop` file, an `.app` bundle).
//! - The version dialog shows the 32 px picture (Sakura's `IDC_STATIC_MYICON`, `CDlgAbout.cpp`).

use std::sync::Arc;

/// The 256 px picture: the icon of the windows on X11.
const WINDOW_ICON_PNG: &[u8] = include_bytes!("../../../assets/icons/matsu_editor_256.png");

/// The 32 px picture (the simplified design for small sizes): the icon in the version dialog.
pub(crate) const DIALOG_ICON_PNG: &[u8] =
    include_bytes!("../../../assets/icons/matsu_editor_32.png");

/// The icon of the windows for `WindowOptions::icon`: the picture on Linux, nothing elsewhere.
pub(crate) fn window_icon() -> Option<Arc<image::RgbaImage>> {
    if cfg!(target_os = "linux") {
        decode_window_icon().map(Arc::new)
    } else {
        None
    }
}

/// The picture of the window icon as RGBA.
fn decode_window_icon() -> Option<image::RgbaImage> {
    image::load_from_memory_with_format(WINDOW_ICON_PNG, image::ImageFormat::Png)
        .ok()
        .map(|img| img.into_rgba8())
}

/// The picture for the version dialog, for GPUI's image element.
pub(crate) fn dialog_icon() -> Arc<gpui::Image> {
    Arc::new(gpui::Image::from_bytes(
        gpui::ImageFormat::Png,
        DIALOG_ICON_PNG.to_vec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_icon_is_the_256_px_picture() {
        let icon = decode_window_icon().expect("the PNG decodes");
        assert_eq!(icon.dimensions(), (256, 256));
        // a picture, not a blank one
        assert!(icon.pixels().any(|p| p.0[3] != 0));
    }

    #[test]
    fn the_dialog_icon_is_the_32_px_picture() {
        let icon = image::load_from_memory_with_format(DIALOG_ICON_PNG, image::ImageFormat::Png)
            .expect("the PNG decodes");
        assert_eq!((icon.width(), icon.height()), (32, 32));
    }

    #[test]
    fn only_linux_passes_the_icon_to_the_windows() {
        assert_eq!(window_icon().is_some(), cfg!(target_os = "linux"));
    }
}
