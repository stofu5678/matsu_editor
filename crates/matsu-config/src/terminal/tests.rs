use super::*;

fn read(text: &str) -> Profile {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

#[test]
fn an_empty_file_has_the_defaults() {
    let settings = TerminalSettings::from_profile(&Profile::new());
    assert_eq!(settings, TerminalSettings::default());
    assert_eq!(settings.shell, "");
    assert_eq!(settings.scrollback_lines, 5000);
    assert!(!settings.option_as_meta);
}

#[test]
fn the_keys_are_read() {
    let profile = read(
        "[Terminal]\r\nShell=cmd.exe /K title \"a b\"\r\nScrollbackLines=123\r\nOptionAsMeta=1\r\n",
    );
    let settings = TerminalSettings::from_profile(&profile);
    assert_eq!(settings.shell, "cmd.exe /K title \"a b\"");
    assert_eq!(settings.scrollback_lines, 123);
    assert!(settings.option_as_meta);
}

#[test]
fn zero_lines_is_allowed() {
    let profile = read("[Terminal]\r\nScrollbackLines=0\r\n");
    assert_eq!(TerminalSettings::from_profile(&profile).scrollback_lines, 0);
}

#[test]
fn invalid_numbers_are_the_default_and_a_large_one_is_the_limit() {
    for text in ["abc", "", "-1", "-5000"] {
        let profile = read(&format!("[Terminal]\r\nScrollbackLines={text}\r\n"));
        assert_eq!(
            TerminalSettings::from_profile(&profile).scrollback_lines,
            DEFAULT_SCROLLBACK_LINES,
            "{text:?}"
        );
    }
    for text in ["50001", "99999999999999"] {
        let profile = read(&format!("[Terminal]\r\nScrollbackLines={text}\r\n"));
        assert_eq!(
            TerminalSettings::from_profile(&profile).scrollback_lines,
            MAX_SCROLLBACK_LINES,
            "{text:?}"
        );
    }
    let profile = read("[Terminal]\r\nScrollbackLines=50000\r\n");
    assert_eq!(
        TerminalSettings::from_profile(&profile).scrollback_lines,
        50_000
    );
}

#[test]
fn an_unreadable_meta_switch_is_off() {
    let profile = read("[Terminal]\r\nOptionAsMeta=yes\r\n");
    assert!(!TerminalSettings::from_profile(&profile).option_as_meta);
}

#[test]
fn the_shell_is_trimmed() {
    let profile = read("[Terminal]\r\nShell=  /bin/zsh -l  \r\n");
    assert_eq!(
        TerminalSettings::from_profile(&profile).shell,
        "/bin/zsh -l"
    );
}

#[test]
fn from_values_follows_the_same_rules() {
    assert_eq!(
        TerminalSettings::from_values(" sh ", -3, true),
        TerminalSettings {
            shell: "sh".to_owned(),
            scrollback_lines: DEFAULT_SCROLLBACK_LINES,
            option_as_meta: true,
        }
    );
    assert_eq!(
        TerminalSettings::from_values("", 1_000_000, false).scrollback_lines,
        MAX_SCROLLBACK_LINES
    );
}
