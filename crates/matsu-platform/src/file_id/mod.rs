//! Whether two paths are the same file as the OS sees it: the same file system and the same file
//! index, so a hard link to a file is the file, and so is the file under another name.
//!
//! Sakura does not ask (its `CopyFile` just fails on the same file); the backup of MATSU Editor must
//! never write into the file it protects (`CBackupAgent`, `crates/matsu-ui/src/file_ops/backup.rs`).
//!
//! * Unix: the device and the inode.
//! * Windows: the serial number of the volume and the index of the file
//!   (`GetFileInformationByHandle`).

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::path::Path;

/// Whether `a` and `b` are the same file. A path that does not exist is not the same file as any
/// other path, unless the two paths are equal.
pub fn same_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    #[cfg(unix)]
    {
        unix::same_file(a, b)
    }
    #[cfg(windows)]
    {
        windows::same_file(a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hard_link_and_a_symbolic_path_are_the_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        std::fs::write(&a, "abc").unwrap();
        let b = dir.path().join("b.txt");
        std::fs::hard_link(&a, &b).unwrap();
        let c = dir.path().join("c.txt");
        std::fs::write(&c, "abc").unwrap();
        assert!(same_file(&a, &b));
        assert!(same_file(&a, &dir.path().join(".").join("a.txt")));
        assert!(!same_file(&a, &c));
        assert!(!same_file(&a, &dir.path().join("missing")));
        assert!(!same_file(
            &dir.path().join("missing"),
            &dir.path().join("other")
        ));
    }
}
