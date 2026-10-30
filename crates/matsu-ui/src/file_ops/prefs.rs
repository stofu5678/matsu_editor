//! The options of Sakura's common settings page ファイル that the file operations read
//! (`CommonSetting_File`, `sakura_core/env/CommonSetting.h`). The history's own options
//! (`m_bRestoreCurPosition` and the like) are `crate::history::HistoryOptions`.

use gpui::{App, Global};

/// The options. Each field names the Sakura setting it stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FilePrefs {
    /// `m_bEnableUnmodifiedOverwrite` (`bEnableUnmodifiedOverwrite`): a save writes the file even
    /// when the document is unmodified. Off by default.
    pub enable_unmodified_overwrite: bool,
    /// `m_nFileShareMode` (`nFileShareMode`): 0 does not lock the file, 1 denies writing, 2 denies
    /// reading and writing. The watching of the file is off while it is not 0
    /// (`CAutoReloadAgent::_ToDoChecking`).
    pub file_share_mode: i32,
    /// `m_bCheckFileTimeStamp` (`bCheckFileTimeStamp`): ask when another program changed the file.
    pub check_file_time_stamp: bool,
    /// `m_nAutoloadDelay` (`nAutoloadDelay`): how many checks to wait before an automatic reload.
    pub autoload_delay: i32,
    /// `m_bUneditableIfUnwritable` (`bUneditableIfUnwritable`): a file that cannot be written opens
    /// without editing (`CEditDoc::IsEditable`).
    pub uneditable_if_unwritable: bool,
    /// `m_bAlertIfFileNotExist` (`bAlertIfFileNotExist`): warn when the file to open is not there.
    pub alert_if_file_not_exist: bool,
    /// `m_bAlertIfLargeFile` (`bAlertIfLargeFile`): ask before opening a file of
    /// [`alert_file_size`](Self::alert_file_size) MB or more.
    pub alert_if_large_file: bool,
    /// `m_nAlertFileSize` (`nAlertFileSize`), in MB.
    pub alert_file_size: i32,
    /// `BKUP_AUTO` of `bBackUpType2_Opt1` (`CommonSetting_Backup::IsAutoBackupEnabled`): the
    /// automatic save.
    pub auto_save: bool,
    /// `bBackUpType2_Opt3` (`GetAutoBackupInterval`): the interval of the automatic save, in minutes.
    pub auto_save_interval: i32,
    /// `m_bNoFilterSaveNew` (`bNoFilterSaveNew`): the save dialog of a document without a file also
    /// shows all files (`;*.*` after the extensions of the type, `CDocFileOperation::SaveFileDialog`).
    pub no_filter_save_new: bool,
    /// `m_bNoFilterSaveFile` (`bNoFilterSaveFile`): the same for a document that has a file.
    pub no_filter_save_file: bool,
    /// `m_bAutoMIMEdecode` (`bAutoMIMEdecode`): MIME encoded words in an ISO-2022-JP file are
    /// decoded when it is read.
    pub auto_mime_decode: bool,
    /// `nHugeFileSizeMB` (not Sakura's, ADR-0020 chapter 7): a file of this many MB or more is
    /// opened by ranges, with its line endings found in the background.
    pub huge_file_size_mb: i32,
    /// Sizes and pace of the scan of such a file. Only the tests change them.
    pub huge_scan: matsu_core::buffer::ScanOptions,
}

/// The default of [`FilePrefs::huge_file_size_mb`].
pub(crate) const HUGE_FILE_SIZE_MB_DEFAULT: i32 = 256;

impl FilePrefs {
    /// The size in bytes from which a file is opened by ranges.
    pub(crate) fn huge_file_len(&self) -> u64 {
        u64::try_from(self.huge_file_size_mb.max(1)).unwrap_or(1) * 1024 * 1024
    }
}

impl Default for FilePrefs {
    /// Sakura's defaults (`CShareData::InitShareData`).
    fn default() -> Self {
        Self {
            enable_unmodified_overwrite: false,
            file_share_mode: 0,
            check_file_time_stamp: true,
            autoload_delay: 0,
            uneditable_if_unwritable: true,
            alert_if_file_not_exist: false,
            alert_if_large_file: false,
            alert_file_size: 10,
            auto_save: false,
            auto_save_interval: 5,
            no_filter_save_new: true,
            no_filter_save_file: true,
            auto_mime_decode: false,
            huge_file_size_mb: HUGE_FILE_SIZE_MB_DEFAULT,
            huge_scan: matsu_core::buffer::ScanOptions::default(),
        }
    }
}

impl Global for FilePrefs {}

/// The options now.
pub(crate) fn prefs(cx: &App) -> FilePrefs {
    cx.try_global::<FilePrefs>().copied().unwrap_or_default()
}
