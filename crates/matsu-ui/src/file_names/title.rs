//! The title of the window: Sakura's title formats (`szWinCaptionActive` and
//! `szWinCaptionInactive`, 「ウィンドウ」 page) expanded by `ExpandParameter`.
//!
//! `CEditWnd::UpdateCaption` (`sakura_core/window/CEditWnd.cpp`) expands the format of the active
//! or the inactive window. The expansion is `matsu_print::expand_parameter`, the port the print
//! headers use; the file name display of [`FileNames`] gives it the short path (`$N`) and the short
//! folder (`$E`).
//!
//! The caret (`$x`, `$y`) is that of the document, as the print headers take it. Not supported, as
//! in the print headers: the selection (`$C`), the date and time (`$d`, `$t`, `$D`, `$T`), the document type (`$B`), the path of the program, the settings file and the macro (`$S`, `$I`, `$M`) and the
//! profile (`$<profile>`); they stay empty. `${I?...}` (the window is a symbol) is never the case.
//! `${R?...}` has all three cases: the view mode, a file that cannot be written (上書き禁止) and
//! neither.

use gpui::{App, Global};
use matsu_print::{HeaderContext, WindowKind, WriteState};

use super::FileNames;

/// Sakura's default format of the active window (`CShareData::InitShareData`).
pub(crate) const CAPTION_ACTIVE: &str = "${w?$h$:アウトプット$:${I?$f$n$:$N$n$}$}${U?(更新)$} - $A $V ${R?(ビューモード)$:(上書き禁止)$}${M?  【キーマクロの記録中】$} $<profile>";
/// Sakura's default format of an inactive window.
pub(crate) const CAPTION_INACTIVE: &str = "${w?$h$:アウトプット$:$f$n$}${U?(更新)$} - $A $V ${R?(ビューモード)$:(上書き禁止)$}${M?  【キーマクロの記録中】$} $<profile>";

/// The two formats of the 「ウィンドウ」 page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TitleFormats {
    pub active: String,
    pub inactive: String,
}

impl Default for TitleFormats {
    fn default() -> Self {
        Self {
            active: CAPTION_ACTIVE.to_owned(),
            inactive: CAPTION_INACTIVE.to_owned(),
        }
    }
}

impl Global for TitleFormats {}

impl TitleFormats {
    pub(crate) fn of(&self, active: bool) -> &str {
        if active { &self.active } else { &self.inactive }
    }
}

/// The format of the window that is active or not.
pub(crate) fn format(cx: &App, active: bool) -> String {
    match cx.try_global::<TitleFormats>() {
        Some(formats) => formats.of(active).to_owned(),
        None => TitleFormats::default().of(active).to_owned(),
    }
}

/// What the title tells about the window.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Subject<'a> {
    /// The path of the file, or none for a document that has none.
    pub path: Option<&'a str>,
    pub modified: bool,
    pub view_mode: bool,
    /// The file cannot be written (`!CDocLocker::IsDocWritable()`).
    pub unwritable: bool,
    pub recording_macro: bool,
    /// The key of the Grep the window shows, for a Grep window.
    pub grep_key: Option<&'a str>,
    /// The caret: the line and the column, both counted from 0.
    pub caret: (usize, usize),
    /// The number of an untitled document (`$n`).
    pub untitled_number: Option<u32>,
}

/// `ExpandParameter` of `format` for `subject`.
#[cfg(test)]
pub(crate) fn expand(format: &str, subject: &Subject<'_>, names: &FileNames) -> String {
    matsu_print::expand_parameter(format, &context(subject, names), None)
}

/// What `ExpandParameter` reads from the window: the title, the tab caption and the headers of
/// the print all take it (`CSakuraEnvironment::ExpandParameter`).
pub(crate) fn context(subject: &Subject<'_>, names: &FileNames) -> HeaderContext {
    let folder = subject
        .path
        .and_then(|path| path.rfind(std::path::is_separator).map(|i| &path[..=i]));
    HeaderContext {
        app_name: crate::support::about::PRODUCT.to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        file_path: subject.path.map(str::to_owned),
        untitled_number: subject.untitled_number,
        file_path_short: subject.path.map(|path| names.transform(path, true)),
        folder_short: folder.map(|folder| names.transform(folder, true)),
        grep_key: subject.grep_key.unwrap_or_default().to_owned(),
        window: if subject.grep_key.is_some() {
            WindowKind::Grep
        } else {
            WindowKind::Normal
        },
        // `$R` (`_ExParam_Evaluate`, `CSakuraEnvironment.cpp`): the view mode first, then a file
        // that cannot be written (上書き禁止), else neither.
        write_state: if subject.view_mode {
            WriteState::ViewMode
        } else if subject.unwritable {
            WriteState::Locked
        } else {
            WriteState::Writable
        },
        modified: subject.modified,
        recording_macro: subject.recording_macro,
        caret_x: subject.caret.1 + 1,
        caret_y: subject.caret.0 + 1,
        ..HeaderContext::default()
    }
}

/// The title Sakura's default format makes for a window with `name` and its marks, for the tests of
/// the windows.
#[cfg(test)]
pub(crate) fn default_title(name: &str) -> String {
    format!(
        "{name} - {} {}  ",
        crate::support::about::PRODUCT,
        env!("CARGO_PKG_VERSION")
    )
}

/// The title of a window as the tests of the file operations compare it: the inactive format (the
/// name of the file without its folder), without the version and the spaces at the end, so that
/// `a.txt(更新) - MATSU Editor` is the document `a.txt` with unsaved changes.
#[cfg(test)]
pub(crate) fn plain_title(view: &crate::view::EditorView, cx: &gpui::App) -> String {
    let product = crate::support::about::PRODUCT;
    view.title_for(false, cx)
        .replacen(
            &format!("{product} {} ", env!("CARGO_PKG_VERSION")),
            &format!("{product} "),
            1,
        )
        .trim_end()
        .to_owned()
}
