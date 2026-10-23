//! The caption of a tab: Sakura's `szTabWndCaption` (「タイトル」 of the タブバー page) expanded by
//! `ExpandParameter` for the document of the tab.
//!
//! `CEditWnd::UpdateCaption` (`sakura_core/window/CEditWnd.cpp`) expands the format for the
//! document and hands the result to the tab bar (`ChangeFileNameNotify`), which shows it as the
//! text of the tab (`CTabWnd::GetTabName`). The expansion is the one of the window title
//! ([`crate::file_names::title`]).

use gpui::{App, Global};

/// Sakura's default format (`CShareData::InitShareData`): the name of the file, or `(無題)`, then
/// the marks of unsaved changes, view mode (or a file that cannot be written) and key macro
/// recording; a Grep window shows `【Grep】` and its key, an output window `【アウトプット】`.
pub(crate) const DEFAULT: &str = "${w?【Grep】$h$:【アウトプット】$:$f$n$}${U?(更新)$}${R?(ビューモード)$:(上書き禁止)$}${M?【キーマクロの記録中】$}";

/// The format the tabs use now (`szTabWndCaption` of `[Common]`, set by the common settings).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TabCaption(pub String);

impl Default for TabCaption {
    fn default() -> Self {
        Self(DEFAULT.to_owned())
    }
}

impl Global for TabCaption {}

/// The format the tabs use now: Sakura's default before the application installs one.
pub(crate) fn format(cx: &App) -> String {
    cx.try_global::<TabCaption>()
        .map_or_else(|| DEFAULT.to_owned(), |caption| caption.0.clone())
}
