//! The boot libraries, which the editor ships inside the executable and writes to a private
//! temporary directory for each run.

use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

pub const JS_BOOT: &str = "boot.cjs";
pub const JS_API: &str = "api.gen.cjs";
pub const PY_BOOT: &str = "boot.py";
pub const PY_API: &str = "api_gen.py";
pub const RB_BOOT: &str = "boot.rb";
pub const RB_API: &str = "api_gen.rb";
pub const PHP_BOOT: &str = "boot.php";
pub const PHP_API: &str = "api_gen.php";
pub const PS_BOOT: &str = "boot.ps1";
pub const PS_API: &str = "api_gen.ps1";
pub const EL_BOOT: &str = "boot.el";
pub const EL_API: &str = "api_gen.el";

/// The Perl program: the runner of the script (`run_user.pl`), the API, the boot, and `__END__`.
/// The editor writes it to the standard input of `perl -` before the conversation (nothing is
/// written to a file for Perl).
pub const PERL_PROGRAM: &str = concat!(
    include_str!("boot/run_user.pl"),
    include_str!("boot/api_gen.pl"),
    "\n",
    include_str!("boot/boot.pl"),
    "\npackage main;\n__END__"
);

const FILES: [(&str, &str); 12] = [
    (JS_BOOT, include_str!("boot/boot.cjs")),
    (JS_API, include_str!("boot/api.gen.cjs")),
    (PY_BOOT, include_str!("boot/boot.py")),
    (PY_API, include_str!("boot/api_gen.py")),
    (RB_BOOT, include_str!("boot/boot.rb")),
    (RB_API, include_str!("boot/api_gen.rb")),
    (PHP_BOOT, include_str!("boot/boot.php")),
    (PHP_API, include_str!("boot/api_gen.php")),
    (PS_BOOT, include_str!("boot/boot.ps1")),
    (PS_API, include_str!("boot/api_gen.ps1")),
    (EL_BOOT, include_str!("boot/boot.el")),
    (EL_API, include_str!("boot/api_gen.el")),
];

/// How many times the removal is tried, and the wait between the tries. On Windows a file that
/// was just closed can still be held for a moment (a virus scanner, for one).
const REMOVE_TRIES: u32 = 40;
const REMOVE_WAIT: Duration = Duration::from_millis(50);

/// The private directory that holds the boot files of one run. It is removed by `remove` (the
/// session does this before it reports the end of the macro) or, if that was not done, when
/// the value is dropped.
#[derive(Debug)]
pub struct BootDir {
    path: Option<PathBuf>,
}

impl BootDir {
    pub fn path(&self) -> &Path {
        self.path
            .as_deref()
            .expect("the directory is taken away only by a `remove` that succeeded")
    }

    /// Removes the directory and what is in it, trying again for a short time if a file is
    /// still in use. After the last try the error is returned and the directory stays (the
    /// drop tries once more).
    pub fn remove(&mut self) -> io::Result<()> {
        let Some(path) = self.path.take() else {
            return Ok(());
        };
        let mut result = remove_dir_all(&path);
        for _ in 1..REMOVE_TRIES {
            if result.is_ok() {
                break;
            }
            thread::sleep(REMOVE_WAIT);
            result = remove_dir_all(&path);
        }
        if result.is_err() {
            self.path = Some(path);
        }
        result
    }
}

fn remove_dir_all(path: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(path) {
        // Already gone is what we wanted.
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

impl Drop for BootDir {
    fn drop(&mut self) {
        let _ = self.remove();
    }
}

/// Writes the boot files to a new directory inside `parent` (the user's temporary directory
/// when `None`). The mode is 0700 on Unix; on Windows the directory gets the access rights
/// of its parent, which is the user's own temporary directory.
pub fn write_boot_files(parent: Option<&Path>) -> io::Result<BootDir> {
    let mut builder = tempfile::Builder::new();
    builder.prefix("matsu-macro-");
    // tempfile makes the directory with the default mode (0755 with a umask of 022), so the
    // mode is asked for: only this user may read or write the code that the interpreter loads.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    let dir = match parent {
        Some(parent) => builder.tempdir_in(parent)?,
        None => builder.tempdir()?,
    };
    // From here on `BootDir` owns the removal, including when a write below fails.
    let boot = BootDir {
        path: Some(dir.keep()),
    };
    for (name, contents) in FILES {
        std::fs::write(boot.path().join(name), contents)?;
    }
    Ok(boot)
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    #[test]
    fn the_boot_directory_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = super::write_boot_files(None).unwrap();
        let mode = std::fs::metadata(dir.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "{mode:o}");
    }

    #[test]
    fn remove_deletes_the_directory_and_dropping_does_too() {
        let parent = tempfile::tempdir().unwrap();
        let mut dir = super::write_boot_files(Some(parent.path())).unwrap();
        let path = dir.path().to_path_buf();
        assert!(path.join(super::PY_BOOT).is_file());
        dir.remove().unwrap();
        assert!(!path.exists());
        dir.remove().unwrap(); // a second call is fine

        let dir = super::write_boot_files(Some(parent.path())).unwrap();
        let path = dir.path().to_path_buf();
        drop(dir);
        assert!(!path.exists());
        assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 0);
    }
}
