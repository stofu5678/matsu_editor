//! The working folder of an AI one-liner (ADR-0030 decision 8): a new, empty folder for each run, so that a tool that
//! reads or writes files around its working folder sees none of the document's neighbours. It is removed when the
//! run ends, whatever the outcome (the value is dropped).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// A folder made for one run and removed when this is dropped.
#[derive(Debug)]
pub(crate) struct WorkDir {
    path: PathBuf,
}

impl WorkDir {
    /// Makes a new folder under the temporary folder of the OS. `create_dir` fails when the name exists, so a folder
    /// that somebody else made first (or a link put there) is never used; on Unix only the user can enter it.
    pub(crate) fn create() -> std::io::Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let base = std::env::temp_dir();
        let mut last = None;
        for _ in 0..16 {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            let name = format!(
                "matsu-ai-{}-{nanos:x}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            let path = base.join(name);
            match make_private_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last = Some(e),
                Err(e) => return Err(e),
            }
        }
        Err(last.unwrap_or_else(|| std::io::Error::other("no free folder name")))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        if std::fs::remove_dir_all(&self.path).is_ok() {
            return;
        }
        // A process that is still ending can hold the folder (Windows): try again from a thread, a few times. What
        // still cannot be removed is left to the OS's temporary folder.
        let path = self.path.clone();
        std::thread::spawn(move || {
            for _ in 0..20 {
                std::thread::sleep(std::time::Duration::from_millis(250));
                if std::fs::remove_dir_all(&path).is_ok() || !path.exists() {
                    return;
                }
            }
        });
    }
}

#[cfg(unix)]
fn make_private_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt as _;
    std::fs::DirBuilder::new().mode(0o700).create(path)
}

#[cfg(not(unix))]
fn make_private_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir(path)
}
