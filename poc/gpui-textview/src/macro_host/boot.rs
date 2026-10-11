//! The boot libraries, which the editor ships inside the executable and writes to a private
//! temporary directory for each run.

use std::io;

pub const JS_BOOT: &str = "boot.cjs";
pub const JS_API: &str = "api.gen.cjs";
pub const PY_BOOT: &str = "boot.py";
pub const PY_API: &str = "api_gen.py";

const FILES: [(&str, &str); 4] = [
    (JS_BOOT, include_str!("boot/boot.cjs")),
    (JS_API, include_str!("boot/api.gen.cjs")),
    (PY_BOOT, include_str!("boot/boot.py")),
    (PY_API, include_str!("boot/api_gen.py")),
];

/// Writes the boot files to a new directory (mode 0700 on Unix; on Windows it is made in the
/// user's temporary directory, and its access rights are not set). The directory is removed
/// when the value is dropped.
pub fn write_boot_files() -> io::Result<tempfile::TempDir> {
    let mut builder = tempfile::Builder::new();
    builder.prefix("ume-macro-");
    // tempfile makes the directory with the default mode (0755 with a umask of 022), so the
    // mode is asked for: only this user may read or write the code that the interpreter loads.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    let dir = builder.tempdir()?;
    for (name, contents) in FILES {
        std::fs::write(dir.path().join(name), contents)?;
    }
    Ok(dir)
}

#[cfg(all(test, unix))]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn the_boot_directory_is_private() {
        let dir = super::write_boot_files().unwrap();
        let mode = std::fs::metadata(dir.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "{mode:o}");
    }
}
