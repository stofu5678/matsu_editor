//! What a document remembers about its file on disk: the time stamp it was read or saved with,
//! how the watching of changes behaves, and whether the file can be written.

use std::path::Path;
use std::time::SystemTime;

/// What happens when another program changed the file (`CAutoReloadAgent::m_eWatchUpdate`,
/// `sakura_core/agent/CAutoReloadAgent.h`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum WatchMode {
    /// `WU_QUERY`: ask. The default.
    #[default]
    Query,
    /// `WU_NOTIFY`: only say it in the status bar (「以後通知メッセージのみ」).
    Notify,
    /// `WU_AUTOLOAD`: reload without asking while the document is unmodified (「以後未編集で再読込」).
    AutoLoad,
    /// `WU_NONE`: do not watch (「以後更新を監視しない」).
    None,
}

/// The file of a document as the disk had it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DiskState {
    /// The last write time seen: when the file was read, written by this editor, or found changed.
    /// `None` for a document without a file or whose file did not exist
    /// (`CDocFile::IsFileTimeZero`): it is not watched.
    pub stamp: Option<SystemTime>,
    pub mode: WatchMode,
    /// `CAutoReloadAgent::m_nDelayCount`: the checks since the last automatic reload.
    pub delay_count: i32,
    /// A check is running (the question is open) or the editor is saving
    /// (`CAutoReloadAgent::PauseWatching`): no other check starts.
    pub pausing: bool,
    /// The file exists and cannot be written (`CDocLocker::m_bIsDocWritable` is false).
    pub unwritable: bool,
}

impl DiskState {
    /// The state of a document just read from `path`.
    pub fn of_path(path: Option<&Path>) -> Self {
        let Some(path) = path else {
            return Self::default();
        };
        Self {
            stamp: modified_time(path),
            unwritable: is_unwritable(path),
            ..Self::default()
        }
    }
}

/// The write time of the file, or `None` if it cannot be asked (not there, no access). It needs
/// no access to the contents (`GetLastWriteTimestamp` in Sakura uses `FindFirstFile`).
pub(crate) fn modified_time(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// `CDocLocker::CheckWritable`: a file that is there but cannot be opened for writing (the read
/// only attribute, no permission, locked by another program). A file that is not there can be
/// written: it is made by the save.
pub(crate) fn is_unwritable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    if meta.permissions().readonly() {
        return true;
    }
    // Opening for appending changes nothing in the file.
    std::fs::OpenOptions::new().append(true).open(path).is_err()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("matsu-ui-state-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_file_that_is_not_there_has_no_stamp_and_can_be_written() {
        let state = DiskState::of_path(Some(Path::new("/nonexistent/matsu-test-file")));
        assert_eq!(state.stamp, None);
        assert!(!state.unwritable);
        assert_eq!(DiskState::of_path(None), DiskState::default());
    }

    #[test]
    fn a_read_only_file_is_unwritable() {
        let dir = temp_dir("readonly");
        let path = dir.join("a.txt");
        std::fs::write(&path, "x").unwrap();
        assert!(!is_unwritable(&path));
        assert!(modified_time(&path).is_some());
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&path, permissions.clone()).unwrap();
        assert!(is_unwritable(&path));
        assert!(DiskState::of_path(Some(&path)).unwritable);
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        std::fs::set_permissions(&path, permissions).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_folder_is_not_unwritable() {
        let dir = temp_dir("folder");
        assert!(!is_unwritable(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
