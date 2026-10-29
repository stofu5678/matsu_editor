//! The icons of the tabs (`bDispTabIcon`, 「アイコン表示」).
//!
//! Sakura: `CTabWnd::GetImageIndex` (`sakura_core/window/CTabWnd.cpp`) gives a tab the icon the
//! shell has for the extension of its file (`SHGetFileInfo` with `SHGFI_USEFILEATTRIBUTES`); a tab
//! with no file gets the Grep icon (the open folder of the shell) when it is a Grep window, and the
//! icon of the application otherwise. `InitImageList` makes the list only while the setting is on.
//!
//! The icon of a file type comes from the shell of Windows. GPUI has no such call, and the other
//! systems have other sets, so a file shows one icon of GPUI's set (a sheet), whatever its
//! extension (ADR-0018 type B: the closest form).

use std::sync::{Arc, OnceLock};

use gpui::{AnyElement, IntoElement as _, ParentElement as _, Styled as _, div, img, px};
use gpui_component::{Icon, IconName, Sizable as _};

/// What the icon of a tab stands for (`GetImageIndex`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TabIcon {
    /// A tab with a file: the icon of the shell for its extension.
    File,
    /// A Grep window (`m_bIsGrep`, `m_iIconGrep`).
    Grep,
    /// A document with no file, or a window that is neither (`m_iIconApp`).
    App,
}

impl TabIcon {
    /// The icon of a tab by what it holds, in the order `GetImageIndex` asks: a file path first,
    /// then the Grep flag.
    pub(crate) fn of(has_file: bool, grep: bool) -> Self {
        if has_file {
            Self::File
        } else if grep {
            Self::Grep
        } else {
            Self::App
        }
    }

    /// The 14 pixel picture for the tab.
    pub(crate) fn element(self) -> AnyElement {
        let picture = match self {
            Self::File => Icon::new(IconName::File).xsmall().into_any_element(),
            Self::Grep => Icon::new(IconName::FolderOpen).xsmall().into_any_element(),
            Self::App => img(app_picture()).size(px(14.)).into_any_element(),
        };
        div().flex_none().child(picture).into_any_element()
    }
}

/// The picture of the application, decoded once.
fn app_picture() -> Arc<gpui::Image> {
    static PICTURE: OnceLock<Arc<gpui::Image>> = OnceLock::new();
    PICTURE.get_or_init(crate::app_icon::dialog_icon).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_wins_over_the_grep_flag() {
        // GetImageIndex looks at m_szFilePath first, then at m_bIsGrep.
        assert_eq!(TabIcon::of(true, true), TabIcon::File);
        assert_eq!(TabIcon::of(true, false), TabIcon::File);
        assert_eq!(TabIcon::of(false, true), TabIcon::Grep);
        assert_eq!(TabIcon::of(false, false), TabIcon::App);
    }
}
