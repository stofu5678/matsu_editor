//! The trash of freedesktop.org (Trash specification 1.0), the home trash of the user.

use std::fs;
use std::io::{self, Write as _};
use std::os::unix::fs::DirBuilderExt as _;
use std::path::{Path, PathBuf};

/// The trash folder of the user.
fn home_trash() -> io::Result<PathBuf> {
    let data = match std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(
            std::env::var_os("HOME").ok_or_else(|| io::Error::other("HOME is not set"))?,
        )
        .join(".local/share"),
    };
    Ok(data.join("Trash"))
}

pub(super) fn move_to_trash(path: &Path) -> io::Result<()> {
    move_into(path, &home_trash()?)
}

/// Moves `path` into the trash folder `trash`.
pub(super) fn move_into(path: &Path, trash: &Path) -> io::Result<()> {
    let absolute = std::path::absolute(path)?;
    let name = absolute
        .file_name()
        .ok_or_else(|| io::Error::other("no file name"))?
        .to_string_lossy()
        .into_owned();
    let files = trash.join("files");
    let info = trash.join("info");
    // Private, whatever the umask is: the trash holds what the user took away.
    for dir in [&files, &info] {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
    }

    // A name that neither folder has yet; the info file is made first and kept by `create_new`, so
    // that two programs do not take the same name.
    let mut n = 0u32;
    let (target, info_path) = loop {
        let candidate = if n == 0 {
            name.clone()
        } else {
            format!("{name}.{n}")
        };
        let target = files.join(&candidate);
        let info_path = info.join(format!("{candidate}.trashinfo"));
        if !target.exists() {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&info_path)
            {
                Ok(mut file) => {
                    let text = format!(
                        "[Trash Info]\nPath={}\nDeletionDate={}\n",
                        encode(&absolute.to_string_lossy()),
                        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S")
                    );
                    if let Err(e) = file.write_all(text.as_bytes()) {
                        let _ = fs::remove_file(&info_path);
                        return Err(e);
                    }
                    break (target, info_path);
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
        n += 1;
    };

    let moved = fs::rename(&absolute, &target).or_else(|_| {
        // Another file system than the trash: copy, then remove.
        fs::copy(&absolute, &target)?;
        fs::remove_file(&absolute)
    });
    if moved.is_err() {
        let _ = fs::remove_file(&info_path);
    }
    moved
}

/// Percent-encoding of the path in the info file (the characters of RFC 3986 that need no coding
/// and `/` stay).
fn encode(path: &str) -> String {
    let mut out = String::new();
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_goes_to_files_with_an_info_file() {
        let dir = tempfile::tempdir().unwrap();
        let trash = dir.path().join("Trash");
        let file = dir.path().join("日本 語.txt");
        fs::write(&file, "abc").unwrap();

        move_into(&file, &trash).unwrap();

        assert!(!file.exists());
        assert_eq!(
            fs::read_to_string(trash.join("files/日本 語.txt")).unwrap(),
            "abc"
        );
        let info = fs::read_to_string(trash.join("info/日本 語.txt.trashinfo")).unwrap();
        assert!(info.starts_with("[Trash Info]\nPath="));
        assert!(
            info.contains("%E6%97%A5%E6%9C%AC%20%E8%AA%9E.txt"),
            "{info}"
        );
        assert!(info.contains("\nDeletionDate="));
    }

    #[test]
    fn a_name_in_the_trash_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let trash = dir.path().join("Trash");
        for text in ["one", "two"] {
            let file = dir.path().join("a.txt");
            fs::write(&file, text).unwrap();
            move_into(&file, &trash).unwrap();
        }
        assert_eq!(
            fs::read_to_string(trash.join("files/a.txt")).unwrap(),
            "one"
        );
        assert_eq!(
            fs::read_to_string(trash.join("files/a.txt.1")).unwrap(),
            "two"
        );
    }
}
