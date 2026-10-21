//! What the E2E tests of the built `ume` program share: a scratch folder, a copy of the program
//! in it, the start with a temporary profile, and a screenshot of the screen.
//!
//! Each test file is its own test binary and uses only some of these.
#![allow(dead_code)]

#[cfg(windows)]
pub mod uia;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

/// A scratch folder that is removed at the end, also when the test fails.
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("ume-e2e-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The way of writing a path in a macro string: a path is written with forward slashes, which
/// every OS accepts, so that no backslash needs an escape in either macro language.
pub fn macro_path(path: &Path) -> String {
    path.to_str().unwrap().replace('\\', "/")
}

/// Copies `ume` and `ume-script` into `home/bin` and returns the copy of `ume`. On Windows the
/// editor reads a `ume.ini` next to its executable before the one in `%APPDATA%`: a copy never
/// meets the portable settings of a developer's build folder. `ume-script` goes next to the copy,
/// where the editor looks for it.
pub fn stage_binaries(home: &Path) -> PathBuf {
    let built = PathBuf::from(env!("CARGO_BIN_EXE_ume"));
    let host_name = if cfg!(windows) {
        "ume-script.exe"
    } else {
        "ume-script"
    };
    let built_host = built.with_file_name(host_name);
    assert!(
        built_host.is_file(),
        "{} is missing: build it in the same profile as ume (cargo build -p ume -p ume-script)",
        built_host.display()
    );
    let bin = home.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let exe = bin.join(built.file_name().unwrap());
    fs::copy(&built, &exe).unwrap();
    fs::copy(&built_host, bin.join(host_name)).unwrap();
    exe
}

/// Starts `exe` with `args` and a profile in `home`: `HOME`, `USERPROFILE`, `APPDATA` and the XDG
/// folders are in `home`, so the program never reads or writes the settings of the user who runs
/// the test. Its output goes to `log`.
pub fn spawn_editor(exe: &Path, home: &Path, args: &[&str], log: &Path) -> Child {
    let out = fs::File::create(log).unwrap();
    Command::new(exe)
        .args(args)
        .current_dir(home)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("APPDATA", home.join("appdata"))
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("RUST_BACKTRACE", "1")
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("XDG_DATA_HOME", home.join("data"))
        .stdin(Stdio::null())
        .stdout(out.try_clone().unwrap())
        .stderr(out)
        .spawn()
        .expect("start ume")
}

/// Where the screenshots go: `UME_E2E_SCREENS_DIR`, or a folder in the temporary folder.
pub fn screens_dir() -> PathBuf {
    let dir = std::env::var_os("UME_E2E_SCREENS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ume-e2e-screens"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The prefix of the file names: `UME_E2E_PLATFORM` (the name of the job), or the OS and the
/// architecture of this build.
pub fn platform_name() -> String {
    std::env::var("UME_E2E_PLATFORM")
        .unwrap_or_else(|_| format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH))
}

/// The path of the screenshot `<platform>-<name>.png`.
pub fn screenshot_path(name: &str) -> PathBuf {
    screens_dir().join(format!("{}-{name}.png", platform_name()))
}

fn run(command: &mut Command) -> Result<(), String> {
    let output = command
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cannot run {:?}: {e}", command.get_program()))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{:?} ended with {}: {}{}",
            command.get_program(),
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// Saves the whole screen as a PNG in `path`.
///
/// * Windows: a copy of the screen by PowerShell and System.Drawing (`capture.ps1`).
/// * macOS: `screencapture`. Without the permission to record the screen it still writes a file,
///   but without the windows of other programs.
/// * Linux: ImageMagick's `import` on the display of the test (`xvfb-run` sets `DISPLAY`), or
///   `xwd` and `convert`.
pub fn capture_screen(path: &Path) -> Result<(), String> {
    let _ = fs::remove_file(path);
    if cfg!(windows) {
        let script = path.with_extension("ps1");
        fs::write(&script, include_str!("capture.ps1")).map_err(|e| e.to_string())?;
        let result = run(Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(&script)
            .arg(path));
        let _ = fs::remove_file(&script);
        result?;
    } else if cfg!(target_os = "macos") {
        run(Command::new("screencapture").arg("-x").arg(path))?;
    } else {
        run(Command::new("import").args(["-window", "root"]).arg(path)).or_else(|first| {
            run(Command::new("sh")
                .arg("-c")
                .arg("xwd -root -silent | convert xwd:- \"$1\"")
                .arg("sh")
                .arg(path))
            .map_err(|second| format!("{first}; {second}"))
        })?;
    }
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("the capture wrote no file at {}", path.display()))
    }
}

/// The width and the height in the header of a PNG file.
pub fn png_size(path: &Path) -> Option<(u32, u32)> {
    let bytes = fs::read(path).ok()?;
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let number = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
    Some((number(16), number(20)))
}

/// A cheap check that a screenshot shows something: it is a PNG of a size a window fits in, and
/// it is not the same as `empty`, the screen before the program started. (Pixels are never
/// compared with a stored image: the text drawing differs between runs and systems.)
pub fn check_screenshot(path: &Path, empty: &Path) -> Result<(), String> {
    let (width, height) =
        png_size(path).ok_or_else(|| format!("{} is not a PNG", path.display()))?;
    if width < 640 || height < 480 {
        return Err(format!("the screen is only {width}x{height}"));
    }
    if fs::read(path).ok() == fs::read(empty).ok() {
        return Err("the screen looks the same as before the program started".to_owned());
    }
    Ok(())
}
