//! The options of Sakura's common settings page ファイル that the file operations read
//! (`CommonSetting_File`, `sakura_core/env/CommonSetting.h`). The history's own options
//! (`m_bRestoreCurPosition` and the like) are `crate::history::HistoryOptions`.

use gpui::{App, Global};

/// The options. Each field names the Sakura setting it stands for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FilePrefs {
    /// `m_bEnableUnmodifiedOverwrite` (`bEnableUnmodifiedOverwrite`): a save writes the file even
    /// when the document is unmodified. Off by default.
    pub enable_unmodified_overwrite: bool,
}

impl Global for FilePrefs {}

/// The options now.
pub(crate) fn prefs(cx: &App) -> FilePrefs {
    cx.try_global::<FilePrefs>().copied().unwrap_or_default()
}
