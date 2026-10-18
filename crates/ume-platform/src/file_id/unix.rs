//! The device and the inode.

use std::os::unix::fs::MetadataExt as _;
use std::path::Path;

pub(super) fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}
