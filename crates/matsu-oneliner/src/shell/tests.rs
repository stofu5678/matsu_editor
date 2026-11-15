use super::*;

#[test]
fn the_kind_comes_from_the_file_name() {
    for program in [
        "pwsh",
        "powershell",
        "PWSH.EXE",
        "powershell.exe",
        "C:\\Program Files\\PowerShell\\7\\pwsh.exe",
        "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\PowerShell.exe",
        "/usr/local/bin/pwsh",
    ] {
        assert_eq!(ShellKind::of(program), ShellKind::PowerShell, "{program}");
    }
    for program in [
        "cmd",
        "CMD.EXE",
        "cmd.exe",
        "C:\\Windows\\System32\\cmd.exe",
        "c:/windows/system32/Cmd.exe",
    ] {
        assert_eq!(ShellKind::of(program), ShellKind::Cmd, "{program}");
    }
    for program in [
        "sh",
        "/bin/sh",
        "/bin/bash",
        "/bin/zsh",
        "/usr/bin/fish",
        "bash.exe",
        "nu",
        "",
        // the directory does not count, and neither does a name that only contains the word
        "/opt/cmd/zsh",
        "mycmd",
        "cmd-wrapper.exe",
        "pwsh-preview",
    ] {
        assert_eq!(ShellKind::of(program), ShellKind::Posix, "{program}");
    }
}

#[test]
fn the_encoding_is_utf8_but_for_cmd() {
    assert_eq!(ShellKind::PowerShell.text_encoding(), TextEncoding::Utf8);
    assert_eq!(ShellKind::Posix.text_encoding(), TextEncoding::Utf8);
    assert_eq!(ShellKind::Cmd.text_encoding(), TextEncoding::Oem);
}

#[test]
fn a_posix_shell_gets_dash_c_and_nothing_else() {
    let line = "tr a-z A-Z | sort -u  # \"quoted\" 'text'";
    let inv = invocation("/bin/zsh", line);
    assert_eq!(inv.kind, ShellKind::Posix);
    assert_eq!(inv.program, "/bin/zsh");
    assert_eq!(inv.args, ["-c", line]);
    assert_eq!(inv.raw_args, None);
    // no login shell and no interactive shell
    assert!(!inv.args.iter().any(|a| a == "-l" || a == "-i"));
}

#[test]
fn cmd_gets_its_command_line_verbatim() {
    let inv = invocation(
        "C:\\Windows\\System32\\cmd.exe",
        "findstr /n \".\" & echo \"a  b\"",
    );
    assert_eq!(inv.kind, ShellKind::Cmd);
    assert!(inv.args.is_empty());
    assert_eq!(
        inv.raw_args.as_deref(),
        Some("/D /S /C \"findstr /n \".\" & echo \"a  b\"\"")
    );
}

#[test]
fn powershell_gets_an_encoded_script() {
    let inv = invocation("pwsh", "$input | Sort-Object -Unique");
    assert_eq!(inv.kind, ShellKind::PowerShell);
    assert_eq!(
        inv.args[..3],
        ["-NoProfile", "-NonInteractive", "-EncodedCommand"]
    );
    assert_eq!(inv.args.len(), 4);
    assert_eq!(inv.raw_args, None);
    assert_eq!(
        inv.args[3],
        powershell::encoded_command("$input | Sort-Object -Unique")
    );
}

#[test]
fn the_setting_gives_its_program_only() {
    // Windows: as CreateProcess reads it
    let program = |s: &str| program_of_setting(s, true).unwrap();
    assert_eq!(program("pwsh.exe -NoLogo"), Some("pwsh.exe".to_owned()));
    assert_eq!(
        program("\"C:\\Program Files\\PowerShell\\7\\pwsh.exe\" -NoExit -Command x"),
        Some("C:\\Program Files\\PowerShell\\7\\pwsh.exe".to_owned())
    );
    assert_eq!(program("cmd.exe /K echo hi"), Some("cmd.exe".to_owned()));
    assert_eq!(program("  cmd  "), Some("cmd".to_owned()));
    // Unix: the first word as the shell tab splits it
    let program = |s: &str| program_of_setting(s, false).unwrap();
    assert_eq!(program("/bin/zsh -l"), Some("/bin/zsh".to_owned()));
    assert_eq!(
        program("'/opt/my shell/zsh' -i"),
        Some("/opt/my shell/zsh".to_owned())
    );
    assert_eq!(program("/bin/fish"), Some("/bin/fish".to_owned()));
}

#[test]
fn an_empty_setting_is_the_default_of_the_tab() {
    for windows in [true, false] {
        assert_eq!(program_of_setting("", windows).unwrap(), None);
        assert_eq!(program_of_setting("  \t ", windows).unwrap(), None);
    }
    assert_eq!(
        choose_program("", true, || "pwsh.exe".to_owned()).unwrap(),
        "pwsh.exe"
    );
    assert_eq!(
        choose_program("cmd.exe /K", true, || unreachable!()).unwrap(),
        "cmd.exe"
    );
}

#[test]
fn a_setting_with_an_unclosed_quote_is_an_error() {
    assert_eq!(
        program_of_setting("\"C:\\Program Files\\x.exe", true),
        Err(ShellError::BadSetting(CommandLineError::UnclosedQuote))
    );
    assert_eq!(
        program_of_setting("'/bin/zsh", false),
        Err(ShellError::BadSetting(CommandLineError::UnclosedQuote))
    );
}

#[cfg(unix)]
#[test]
fn the_default_is_an_absolute_path_on_unix() {
    let shell = default_program();
    assert!(std::path::Path::new(&shell).is_absolute(), "{shell}");
}
