//! The free space of the disk a folder is on, for the check before a save (ADR-0020, chapter 4).
//!
//! Sakura has no such check (it writes into the file in place). MATSU writes a whole new file
//! before it replaces the old one (ADR-0005), so it asks first.
//!
//! * Windows: `GetDiskFreeSpaceExW`, the bytes available to the user of the process (quotas count).
//! * macOS and Linux: `statvfs`, `f_bavail * f_frsize`: the blocks a user without privilege can use.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::io;
use std::path::Path;

/// The bytes that can be written in the folder `dir`.
pub fn available(dir: &Path) -> io::Result<u64> {
    #[cfg(windows)]
    {
        windows::available(dir)
    }
    #[cfg(unix)]
    {
        unix::available(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_temporary_folder_has_some_space() {
        assert!(available(&std::env::temp_dir()).unwrap() > 0);
    }

    #[test]
    fn a_folder_that_does_not_exist_is_an_error() {
        let dir = std::env::temp_dir().join("matsu-free-space-no-such-folder");
        assert!(available(&dir).is_err());
    }
}
