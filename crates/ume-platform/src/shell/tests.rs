use std::path::Path;

use super::*;

#[test]
fn unc_paths_are_found() {
    assert!(is_unc_path(r"\\server\share\dir"));
    assert!(is_unc_path(r"\\?\UNC\server\share"));
    assert!(is_unc_path(r"\\?\unc\server\share"));
    assert!(!is_unc_path(r"\\?\C:\dir"));
    assert!(!is_unc_path(r"C:\dir"));
    assert!(!is_unc_path("/home/user"));
    assert!(!is_unc_path(r"\\?\"));
    assert!(!is_unc_path(""));
}

#[test]
fn the_command_prompt_opens_without_parameters() {
    // Sakura: ShellExecuteEx with the verb, COMSPEC, no parameters and the folder.
    let launch = windows_launch(
        Shell::CommandPrompt,
        false,
        Path::new(r"C:\日本語 dir"),
        Some(Path::new(r"C:\Windows\system32\cmd.exe")),
    )
    .unwrap();
    assert_eq!(
        launch,
        WindowsLaunch {
            verb: "open",
            file: r"C:\Windows\system32\cmd.exe".to_owned(),
            parameters: None,
            directory: r"C:\日本語 dir".to_owned(),
        }
    );
}

#[test]
fn the_admin_variants_use_runas() {
    let cmd = Some(Path::new("cmd.exe"));
    let folder = Path::new(r"C:\a");
    assert_eq!(
        windows_launch(Shell::CommandPrompt, true, folder, cmd)
            .unwrap()
            .verb,
        "runas"
    );
    assert_eq!(
        windows_launch(Shell::PowerShell, true, folder, cmd)
            .unwrap()
            .verb,
        "runas"
    );
    assert_eq!(
        windows_launch(Shell::PowerShell, false, folder, cmd)
            .unwrap()
            .verb,
        "open"
    );
}

#[test]
fn the_command_prompt_refuses_a_network_folder_and_a_missing_comspec() {
    let unc = Path::new(r"\\server\share");
    assert!(matches!(
        windows_launch(Shell::CommandPrompt, false, unc, Some(Path::new("cmd.exe"))),
        Err(ShellError::Unavailable(_))
    ));
    assert!(matches!(
        windows_launch(Shell::CommandPrompt, false, Path::new(r"C:\a"), None),
        Err(ShellError::Failed(_))
    ));
    // PowerShell does not mind a network folder, and does not need COMSPEC.
    assert!(windows_launch(Shell::PowerShell, false, unc, None).is_ok());
}

#[test]
fn powershell_gets_set_location_with_the_quotes_doubled() {
    // Sakura: -NoExit -Command "Set-Location -Path ..." (here -LiteralPath, so that [ and * in the
    // folder name are not wildcards) with ' written as ''.
    let launch =
        windows_launch(Shell::PowerShell, false, Path::new(r"C:\it's here"), None).unwrap();
    assert_eq!(launch.file, "powershell.exe");
    assert_eq!(
        launch.parameters.as_deref(),
        Some(r#"-NoExit -Command "Set-Location -LiteralPath 'C:\it''s here'""#)
    );
    assert_eq!(launch.directory, r"C:\it's here");
    assert_eq!(powershell_quote("a'b''c"), "a''b''''c");
    // The typographic quotes end a PowerShell string too: they are doubled, so that a folder name
    // cannot add commands.
    assert_eq!(
        powershell_quote("x\u{2019}; Write-Output INJECTED; #"),
        "x\u{2019}\u{2019}; Write-Output INJECTED; #"
    );
    assert_eq!(
        powershell_quote("\u{2018}\u{201A}\u{201B}"),
        "\u{2018}\u{2018}\u{201A}\u{201A}\u{201B}\u{201B}"
    );
    // A folder with brackets is a literal path.
    assert_eq!(
        powershell_parameters(r"C:\work\[ab]"),
        r#"-NoExit -Command "Set-Location -LiteralPath 'C:\work\[ab]'""#
    );
}

#[test]
fn quoting_for_a_posix_shell_and_applescript() {
    assert_eq!(sh_quote("a b"), "'a b'");
    assert_eq!(sh_quote("it's"), r"'it'\''s'");
    assert_eq!(applescript_quote(r#"a"b\c"#), r#""a\"b\\c""#);
}

#[test]
fn macos_opens_terminal_or_runs_pwsh_in_it() {
    let plain = mac_launch(Path::new("/Users/a b/日本語"), None);
    assert_eq!(plain.program, "open");
    assert_eq!(plain.args, ["-a", "Terminal", "/Users/a b/日本語"]);

    let pwsh = mac_launch(
        Path::new("/Users/it's"),
        Some(Path::new("/usr/local/bin/pwsh")),
    );
    assert_eq!(pwsh.program, "osascript");
    assert_eq!(pwsh.args[0], "-e");
    let script = &pwsh.args[1];
    assert!(script.contains("tell application \"Terminal\""));
    // the shell command inside the AppleScript string: cd, then pwsh with its arguments
    assert!(
        script.contains(
            r#"do script "cd '/Users/it'\\''s' && exec '/usr/local/bin/pwsh' '-NoExit' '-Command' 'Set-Location -LiteralPath '\\''/Users/it'\\'''\\''s'\\'''""#
        ),
        "{script}"
    );
}

#[test]
fn linux_terminals_start_in_the_folder_and_run_the_inner_command() {
    let folder = Path::new("/home/a b");
    let plain = linux_terminals(folder, None);
    let names: Vec<_> = plain.iter().map(|s| s.program.as_str()).collect();
    assert_eq!(
        names,
        [
            "x-terminal-emulator",
            "gnome-terminal",
            "konsole",
            "xfce4-terminal",
            "xterm"
        ]
    );
    assert!(plain[0].args.is_empty());
    assert_eq!(plain[1].args, ["--working-directory=/home/a b"]);
    assert_eq!(plain[2].args, ["--workdir", "/home/a b"]);

    let inner = ["pwsh".to_owned(), "-NoExit".to_owned()];
    let with = linux_terminals(folder, Some(&inner));
    assert_eq!(with[0].args, ["-e", "pwsh", "-NoExit"]);
    assert_eq!(
        with[1].args,
        ["--working-directory=/home/a b", "--", "pwsh", "-NoExit"]
    );
    assert_eq!(with[3].args[1..], ["-x", "pwsh", "-NoExit"]);
}

#[test]
fn pwsh_gets_the_folder_as_a_literal_path() {
    assert_eq!(
        pwsh_arguments("/home/it's"),
        [
            "-NoExit",
            "-Command",
            "Set-Location -LiteralPath '/home/it''s'"
        ]
    );
}

#[test]
fn a_program_is_found_on_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let name = if cfg!(windows) { "tool.exe" } else { "tool" };
    let file = dir.path().join(name);
    std::fs::write(&file, b"").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // not executable yet: not found
        let path = std::env::join_paths([other.path(), dir.path()]).unwrap();
        assert_eq!(find_in_path(name, &path), None);
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = std::env::join_paths([other.path(), dir.path()]).unwrap();
    assert_eq!(find_in_path(name, &path), Some(file));
    assert_eq!(find_in_path("no-such-tool", &path), None);
}

#[cfg(not(windows))]
#[test]
fn the_admin_variants_and_missing_folders_are_refused_here() {
    assert!(matches!(
        open_shell(Shell::CommandPrompt, true, Path::new("/")),
        Err(ShellError::Unavailable(_))
    ));
    assert!(matches!(
        open_shell(Shell::PowerShell, true, Path::new("/")),
        Err(ShellError::Unavailable(_))
    ));
    assert!(matches!(
        open_shell(Shell::CommandPrompt, false, Path::new("/no/such/folder")),
        Err(ShellError::Failed(_))
    ));
}
