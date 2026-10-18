//! `statfs` of the nearest folder that exists.

// `statfs` is `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub(super) fn is_local(path: &Path) -> bool {
    // The file may not exist (a backup that is not made yet): the first folder above it that does.
    let mut probe = path;
    while !probe.exists() {
        match probe.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => probe = parent,
            _ => return true,
        }
    }
    let Ok(c_path) = CString::new(probe.as_os_str().as_bytes()) else {
        return true;
    };
    let mut stat = MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: `c_path` is a NUL-terminated string and `stat` is a buffer for one `statfs`.
    let result = unsafe { libc::statfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if result != 0 {
        return true;
    }
    // SAFETY: `statfs` filled it when it returned 0.
    let stat = unsafe { stat.assume_init() };
    local_by(&stat)
}

#[cfg(target_os = "macos")]
fn local_by(stat: &libc::statfs) -> bool {
    stat.f_flags & (libc::MNT_LOCAL as u32) != 0
}

#[cfg(not(target_os = "macos"))]
fn local_by(stat: &libc::statfs) -> bool {
    // `linux/magic.h`
    const NETWORK: [u64; 9] = [
        0x6969,      // NFS_SUPER_MAGIC
        0x517B,      // SMB_SUPER_MAGIC
        0xFF53_4D42, // CIFS_MAGIC_NUMBER
        0xFE53_4D42, // SMB2_MAGIC_NUMBER
        0x0102_1997, // V9FS_MAGIC
        0x00C3_6400, // CEPH_SUPER_MAGIC
        0x5346_414F, // AFS_SUPER_MAGIC
        0x7375_7245, // CODA_SUPER_MAGIC
        0x564C,      // NCP_SUPER_MAGIC
    ];
    #[allow(clippy::unnecessary_cast)]
    let kind = (stat.f_type as u64) & 0xFFFF_FFFF;
    !NETWORK.contains(&kind)
}
