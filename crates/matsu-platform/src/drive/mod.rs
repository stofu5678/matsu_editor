//! Whether a file is on a local disk (`IsLocalDrive`, `sakura_core/util/file.cpp`).
//!
//! Sakura asks it for the backups: a backup in a folder only for removable media (`bBackUpFolderRM`),
//! and a backup of a file on a network or removable drive is not moved to the recycle bin
//! (`CBackupAgent`).
//!
//! * Windows: as Sakura does. A path with a drive letter is not local when the drive is removable,
//!   a CD-ROM or remote (`GetDriveType`); a path that starts with two backslashes (a network path)
//!   is not local.
//! * macOS: the file system is local when it has the flag `MNT_LOCAL` (`statfs`).
//! * Linux: the file system is not local when it is a network one (NFS, SMB, CIFS, 9P, Ceph, AFS,
//!   Coda, NCP; by the type that `statfs` says). Removable disks cannot be told from the others.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::path::Path;

/// Whether `path` is on a local disk. A path whose disk cannot be told is local, as in Sakura.
pub fn is_local(path: &Path) -> bool {
    #[cfg(windows)]
    {
        windows::is_local(path)
    }
    #[cfg(unix)]
    {
        unix::is_local(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_temporary_folder_is_local() {
        assert!(is_local(&std::env::temp_dir()));
    }

    #[test]
    fn a_path_that_does_not_exist_is_judged_by_the_folder_above_it() {
        let path = std::env::temp_dir()
            .join("matsu-drive-no-such-folder")
            .join("a.txt");
        assert!(is_local(&path));
    }
}
