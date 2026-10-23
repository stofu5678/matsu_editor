//! Starting MATSU Editor: one process for each settings file ([ADR-0016]).
//!
//! `matsu [--separate-process] [Sakura's options] [--] [path...]`. A launch first looks for a MATSU
//! Editor that runs with the same settings file (`matsu_platform::instance`). If there is one, the
//! launch hands it the arguments and the working folder, and ends. If not, it becomes the one that
//! others hand their launches to. Either way, the files named on a command line, the files of a
//! later launch, and the files that macOS sends to a running application (Finder, the Dock) all
//! arrive as an [`OpenRequest`] at the same place: `file_ops::windows::open_from_outside`.
//!
//! The arguments are read where the documents are opened: a handed-over launch brings its
//! arguments as they are, and the process that receives them reads them
//! (`command_line`, Sakura's options). The one option that has to be read
//! before that is `-PROF=`, because the profile decides the settings file, and the settings file
//! decides which running MATSU Editor a launch joins (as Sakura's profiles do).
//!
//! `--separate-process` runs this launch as a process of its own, without looking for another and
//! without being found (tests and development). `--` ends the options, so that a file whose name
//! starts with `-` can be opened; it is kept in the arguments that are handed over, because the
//! receiver reads them again.
//!
//! [ADR-0016]: https://github.com/stofu5678/matsu_editor/blob/main/docs/adr/0016-single-instance.md

use std::ffi::OsString;
use std::path::PathBuf;

use matsu_platform::instance::{Joined, Key, Request};

use crate::command_line::{self, CommandLine};

/// The option that runs a launch as a process of its own.
pub const SEPARATE_PROCESS: &str = "--separate-process";

/// What to open: the files of a command line, of a later launch, or of the OS. No file means one
/// new, empty window (Sakura: a launch without a file, `CControlTray::OpenNewEditor`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenRequest {
    pub paths: Vec<PathBuf>,
    /// Sakura's options of the command line (the files are in `paths`).
    pub(crate) options: CommandLine,
    /// The working folder of the launch, for what the options name without a place (the macro,
    /// the folder of a Grep). Empty for the OS: the folder of this process.
    pub(crate) cwd: PathBuf,
}

impl OpenRequest {
    /// The request of a launch: relative files are in the launch's working folder, and the
    /// options are read (`command_line::parse_with`, with the response file `-@=` read from
    /// the working folder too).
    pub fn from_launch(request: &Request) -> OpenRequest {
        let mut options = command_line::parse_with(&request.args, &mut |path| {
            let bytes = std::fs::read(request.cwd.join(path)).ok()?;
            Some(command_line::response_text(&bytes))
        });
        let paths = std::mem::take(&mut options.files)
            .iter()
            .map(|file| request.cwd.join(file))
            .collect();
        OpenRequest {
            paths,
            options,
            cwd: request.cwd.clone(),
        }
    }

    /// The files of the URLs that macOS passes to a running application. Only `file:` URLs count.
    pub fn from_urls(urls: &[String]) -> OpenRequest {
        OpenRequest {
            paths: urls.iter().filter_map(|u| path_of_file_url(u)).collect(),
            options: CommandLine::default(),
            cwd: PathBuf::new(),
        }
    }
}

/// The arguments without `--separate-process` (before a `--`), and whether it was among them.
pub fn split_options(args: Vec<OsString>) -> (bool, Vec<OsString>) {
    let mut separate = false;
    let mut rest = Vec::new();
    let mut options_end = false;
    for arg in args {
        if options_end {
            rest.push(arg);
        } else if arg == "--" {
            options_end = true;
            rest.push(arg);
        } else if arg == SEPARATE_PROCESS {
            separate = true;
        } else {
            rest.push(arg);
        }
    }
    (separate, rest)
}

/// `file:///Users/me/a%20b.txt` is `/Users/me/a b.txt`. Another kind of URL is no file.
pub(crate) fn path_of_file_url(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("file://")?;
    // `file://localhost/path`: the host is this machine. Any other host is not a local file.
    let path = match rest.find('/') {
        Some(0) => rest,
        Some(i) if rest[..i].eq_ignore_ascii_case("localhost") => &rest[i..],
        _ => return None,
    };
    let path = path.split(['?', '#']).next().unwrap_or(path);
    let mut bytes = Vec::with_capacity(path.len());
    let raw = path.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%'
            && let Some(hex) = raw.get(i + 1..i + 3)
            && let Ok(hex) = std::str::from_utf8(hex)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            bytes.push(byte);
            i += 3;
        } else {
            bytes.push(raw[i]);
            i += 1;
        }
    }
    Some(PathBuf::from(String::from_utf8_lossy(&bytes).into_owned()))
}

/// Runs MATSU Editor for a command line (the arguments after the program name) until the last window
/// closes. Returns at once if the arguments were handed to a MATSU Editor that is running already.
pub fn launch(args: Vec<OsString>) {
    let (separate, args) = split_options(args);
    let request = Request::from_current_process(args);
    let (tx, rx) = async_channel::unbounded();
    let first = OpenRequest::from_launch(&request);
    // `-PROF=`: before anything looks for the settings file.
    choose_profile(&first.options);
    // The settings of the old name are copied to the new places before the file is read.
    crate::settings::migrate_old_settings();
    // The files of this command line come first, through the entry that the others use.
    tx.send_blocking(first).ok();
    let user = crate::settings::UserSettings::load();
    let _receiver = if separate {
        None
    } else {
        match join(&user, &request, tx.clone()) {
            Some(Joined::Delivered) => return,
            Some(Joined::Receiver(receiver)) => Some(receiver),
            Some(Joined::Standalone(why)) => {
                eprintln!("matsu: {why}; running as a process of its own");
                None
            }
            None => {
                eprintln!("matsu: no place for the settings file; running as a process of its own");
                None
            }
        }
    };
    crate::app::run(user, tx, rx);
}

/// `-PROF=name`: the settings file of the profile, a sub folder of the places for `matsu.ini`
/// (`matsu_config::location`). A name that is empty is the default profile; one that would leave
/// the folder is refused. `-PROFMGR` (the profile manager) is not in MATSU Editor.
fn choose_profile(options: &CommandLine) {
    match options.profile.as_deref() {
        None | Some("") => {}
        Some(name) => match command_line::valid_profile_name(name) {
            Some(name) => {
                matsu_config::location::set_process_profile_name(name);
            }
            None => eprintln!("matsu: -PROF={name}: not a name for a folder; ignored"),
        },
    }
}

/// Looks for the MATSU Editor of the settings file `user` uses, and becomes it or reaches it.
/// `None` if the OS has no place for the settings file, so that nothing names the process.
fn join(
    user: &crate::settings::UserSettings,
    request: &Request,
    tx: async_channel::Sender<OpenRequest>,
) -> Option<Joined> {
    let file = crate::settings::history_file(user)?;
    let key = Key::for_settings_file(&file);
    let handler = Box::new(move |later: Request| {
        // The UI thread takes it from the channel; this thread only passes it on.
        tx.send_blocking(OpenRequest::from_launch(&later)).is_ok()
    });
    Some(matsu_platform::instance::join(&key, request, handler))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn os(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn the_separate_process_option_is_taken_out() {
        let (separate, rest) = split_options(os(&["a.txt", SEPARATE_PROCESS, "b.txt"]));
        assert!(separate);
        assert_eq!(rest, os(&["a.txt", "b.txt"]));
        let (separate, rest) = split_options(os(&["a.txt"]));
        assert!(!separate);
        assert_eq!(rest, os(&["a.txt"]));
    }

    #[test]
    fn after_two_dashes_everything_is_a_file() {
        let (separate, rest) = split_options(os(&["--", SEPARATE_PROCESS, "--", "-x"]));
        assert!(!separate);
        // the `--` stays: the process that reads the arguments needs it
        assert_eq!(rest, os(&["--", SEPARATE_PROCESS, "--", "-x"]));
    }

    #[test]
    fn relative_files_of_a_launch_are_in_its_working_folder() {
        let cwd = if cfg!(windows) { r"C:\work" } else { "/work" };
        let absolute = if cfg!(windows) {
            r"D:\x\y.txt"
        } else {
            "/x/y.txt"
        };
        let request = Request {
            cwd: PathBuf::from(cwd),
            args: os(&["a.txt", absolute, "", "日本語.txt"]),
        };
        let open = OpenRequest::from_launch(&request);
        assert_eq!(
            open.paths,
            [
                Path::new(cwd).join("a.txt"),
                PathBuf::from(absolute),
                Path::new(cwd).join("日本語.txt"),
            ]
        );
    }

    #[test]
    fn a_launch_without_files_opens_no_path() {
        let request = Request {
            cwd: PathBuf::from("/work"),
            args: Vec::new(),
        };
        assert!(OpenRequest::from_launch(&request).paths.is_empty());
    }

    #[test]
    fn file_urls_become_paths() {
        let urls = [
            "file:///Users/me/a%20b.txt".to_owned(),
            "file://localhost/tmp/%E6%97%A5%E6%9C%AC.txt".to_owned(),
            "file:///tmp/100%25.txt".to_owned(),
            "file:///tmp/broken%2.txt".to_owned(),
            "https://example.com/a.txt".to_owned(),
            "file://server/share/a.txt".to_owned(),
            "file:relative.txt".to_owned(),
        ];
        assert_eq!(
            OpenRequest::from_urls(&urls).paths,
            [
                PathBuf::from("/Users/me/a b.txt"),
                PathBuf::from("/tmp/日本.txt"),
                PathBuf::from("/tmp/100%.txt"),
                PathBuf::from("/tmp/broken%2.txt"),
            ]
        );
    }
}
