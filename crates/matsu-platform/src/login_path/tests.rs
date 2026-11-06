use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use super::*;

fn os(text: &str) -> OsString {
    OsString::from(text)
}

// ---- when to read ----

#[test]
fn reads_on_macos_without_a_terminal_only() {
    assert!(should_read(true, false));
    assert!(!should_read(true, true));
    assert!(!should_read(false, false));
    assert!(!should_read(false, true));
}

#[test]
fn shell_is_shell_variable_if_absolute() {
    assert_eq!(
        choose_shell(Some(OsStr::new("/opt/homebrew/bin/fish"))),
        PathBuf::from("/opt/homebrew/bin/fish")
    );
    assert_eq!(
        choose_shell(Some(OsStr::new("zsh"))),
        PathBuf::from("/bin/zsh")
    );
    assert_eq!(
        choose_shell(Some(OsStr::new(""))),
        PathBuf::from("/bin/zsh")
    );
    assert_eq!(choose_shell(None), PathBuf::from("/bin/zsh"));
}

// ---- the command ----

#[test]
fn script_has_the_mark_twice_and_no_quote_inside_quotes() {
    let mark = "MATSU_PATH_1_2_";
    let script = shell_script(mark);
    assert_eq!(script.matches(mark).count(), 2);
    assert_eq!(script.matches('\'').count(), 2, "{script}");
    assert!(script.starts_with("/bin/sh -c '") && script.ends_with('\''));
    assert!(mark.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
}

#[test]
fn marks_differ_and_are_plain() {
    let a = make_mark();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let b = make_mark();
    assert_ne!(a, b);
    assert!(a.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
}

// ---- reading the output ----

const MARK: &str = "MATSU_PATH_9_9_";

fn wrap(path: &str) -> Vec<u8> {
    format!("{MARK}{path}{MARK}").into_bytes()
}

#[test]
fn path_is_between_the_marks() {
    let mut out = b"Welcome!\nlast login\n".to_vec();
    out.extend(wrap("/a:/b"));
    out.extend(b"\nbye\n");
    assert_eq!(parse_output(&out, MARK).unwrap(), b"/a:/b");
}

#[test]
fn one_mark_or_none_is_a_failure() {
    let one = format!("junk{MARK}/a:/b");
    assert!(parse_output(one.as_bytes(), MARK).is_err());
    assert!(parse_output(b"/a:/b\n", MARK).is_err());
    assert!(parse_output(b"", MARK).is_err());
}

#[test]
fn empty_path_is_a_failure() {
    assert!(parse_output(&wrap(""), MARK).is_err());
}

#[test]
fn path_with_nul_is_a_failure() {
    let mut out = format!("{MARK}/a").into_bytes();
    out.push(0);
    out.extend(format!("/b{MARK}").bytes());
    assert!(parse_output(&out, MARK).is_err());
}

#[test]
fn path_need_not_be_utf8() {
    let mut out = MARK.as_bytes().to_vec();
    out.extend([b'/', 0xff, b'x']);
    out.extend(MARK.bytes());
    assert_eq!(parse_output(&out, MARK).unwrap(), [b'/', 0xff, b'x']);
}

#[test]
fn more_than_the_limit_before_the_marks_is_not_a_path() {
    // what the reader of the shell keeps is `OUTPUT_LIMIT` bytes; the marks after them are lost
    let mut out = vec![b'x'; OUTPUT_LIMIT];
    out.truncate(OUTPUT_LIMIT);
    assert!(parse_output(&out, MARK).is_err());
}

// ---- mixing ----

#[test]
fn shell_folders_keep_their_order_and_first_repeat() {
    assert_eq!(mix(&os("/b:/a:/b:/c"), None), Some(os("/b:/a:/c")));
}

#[test]
fn empty_and_relative_folders_are_dropped() {
    assert_eq!(mix(&os("/a::.:bin:../x:/b:"), None), Some(os("/a:/b")));
}

#[test]
fn startup_folders_follow_if_new() {
    assert_eq!(
        mix(
            &os("/opt/homebrew/bin:/usr/bin"),
            Some(OsStr::new("/usr/bin:/bin:/test/bin:."))
        ),
        Some(os("/opt/homebrew/bin:/usr/bin:/bin:/test/bin"))
    );
}

#[test]
fn shell_without_an_absolute_folder_is_a_failure() {
    assert_eq!(mix(&os(".:bin::"), Some(OsStr::new("/usr/bin"))), None);
    assert_eq!(mix(&os(""), None), None);
}

#[test]
fn missing_startup_path_is_fine() {
    assert_eq!(mix(&os("/a"), None), Some(os("/a")));
}

// ---- a shell of the test's own ----

#[cfg(unix)]
mod fake_shell {
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::super::unix::read;
    use super::*;

    fn write_shell(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("fake-shell");
        let record = dir.display();
        let text = format!(
            "#!/bin/sh\n\
             printf '%s\\n' \"$*\" > '{record}/args'\n\
             printf '%s\\n' \"$MATSU_RESOLVING_ENVIRONMENT\" > '{record}/env'\n\
             echo $$ > '{record}/pid'\n\
             {body}\n"
        );
        std::fs::write(&path, text).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Reads with the shell; starting a file that was just written can fail with "text file
    /// busy" while another test forks, so that is tried again.
    fn run(shell: &Path, limit: Duration, startup: Option<&OsStr>) -> Result<OsString, String> {
        let mut result = Err(String::new());
        for _ in 0..20 {
            result = read(shell, Instant::now() + limit, startup);
            match &result {
                Err(why) if why.to_lowercase().contains("busy") => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                _ => break,
            }
        }
        result
    }

    fn gone(pid: &str) -> bool {
        for _ in 0..60 {
            let alive = std::process::Command::new("kill")
                .args(["-0", pid])
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|s| s.success());
            if !alive {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    #[test]
    fn arguments_and_variable_reach_the_shell_and_path_is_read_past_garbage() {
        let dir = tempfile::tempdir().unwrap();
        let shell = write_shell(
            dir.path(),
            "echo 'garbage on the way in'\n\
             PATH=/fake/a:rel:/fake/b:/fake/a:\n\
             export PATH\n\
             eval \"$4\"\n\
             echo 'trailing garbage'",
        );
        let path = run(
            &shell,
            Duration::from_secs(20),
            Some(OsStr::new("/start/bin:/fake/b")),
        )
        .unwrap();
        assert_eq!(path, os("/fake/a:/fake/b:/start/bin"));
        let args = std::fs::read_to_string(dir.path().join("args")).unwrap();
        assert!(args.starts_with("-l -i -c /bin/sh -c 'printf "), "{args}");
        let env = std::fs::read_to_string(dir.path().join("env")).unwrap();
        assert_eq!(env.trim(), "1");
    }

    #[test]
    fn a_shell_that_sleeps_is_ended_at_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        let shell = write_shell(
            dir.path(),
            "sleep 60 &\necho $! > \"$(dirname \"$0\")/child\"\nsleep 60",
        );
        let started = Instant::now();
        let result = run(&shell, Duration::from_millis(700), None);
        assert_eq!(result, Err("timed out".to_owned()));
        assert!(started.elapsed() < Duration::from_secs(15));
        let pid = std::fs::read_to_string(dir.path().join("pid")).unwrap();
        assert!(gone(pid.trim()), "the shell is still there");
        let child = std::fs::read_to_string(dir.path().join("child")).unwrap();
        assert!(gone(child.trim()), "its child is still there");
    }

    #[test]
    fn a_shell_that_prints_no_mark_is_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        let shell = write_shell(dir.path(), "echo hello");
        let result = run(&shell, Duration::from_secs(20), None);
        assert!(result.is_err(), "{result:?}");
        assert_ne!(result, Err("timed out".to_owned()));
    }

    #[test]
    fn a_shell_that_cannot_be_started_is_a_failure() {
        let result = read(
            Path::new("/nonexistent/shell"),
            Instant::now() + Duration::from_secs(5),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn a_shell_with_only_relative_folders_is_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        let shell = write_shell(dir.path(), "PATH=.:bin\nexport PATH\neval \"$4\"");
        let result = run(
            &shell,
            Duration::from_secs(20),
            Some(OsStr::new("/usr/bin")),
        );
        assert!(result.is_err(), "{result:?}");
    }
}
