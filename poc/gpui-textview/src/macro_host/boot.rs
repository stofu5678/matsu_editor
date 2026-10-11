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

/// Writes the boot files to a new directory that only this user can use (the directory is
/// removed when the value is dropped).
pub fn write_boot_files() -> io::Result<tempfile::TempDir> {
    let dir = tempfile::Builder::new().prefix("ume-macro-").tempdir()?;
    for (name, contents) in FILES {
        std::fs::write(dir.path().join(name), contents)?;
    }
    Ok(dir)
}
