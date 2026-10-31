//! Tests of the database functions (ADR-0023). No database is needed: the "client" of a
//! connection is `echo` (the file given to it is printed) or a program that copies its input to
//! its output, which every CI runner has. The quoting of the file is also tested on its own for
//! both shells; the `sh` one is tested against the real `sh` below (the Windows one by the
//! Windows jobs of the CI).

use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use matsu_commands::sakura::FUNCTIONS;
use matsu_config::databases::{Connection, Encoding, Kind};
use matsu_core::buffer::Pos;

use super::command::{BuildError, Shell, build, escape_for_expand, quote_path};
use super::run::flags;
use super::{DatabasePrefs, install};
use crate::document::SharedDocument;
use crate::file_ops::windows::{LoadRequest, editors, open_editor_window};
use crate::view::EditorView;

/// A program that copies its input to its output.
const COPY: &str = if cfg!(windows) {
    "findstr \"^\""
} else {
    "cat"
};

fn connection(command: &str) -> Connection {
    Connection {
        name: "test".into(),
        kind: Kind::Other,
        command: command.into(),
        folder: None,
        encoding: Encoding::Auto,
        input_encoding: None,
    }
}

// ---- the command line ----

#[test]
fn a_path_is_one_word_for_sh() {
    let q = |p: &str| quote_path(Shell::Posix, p).unwrap();
    assert_eq!(q("/a/b.sql"), "'/a/b.sql'");
    assert_eq!(q("/a b/c.sql"), "'/a b/c.sql'");
    assert_eq!(q("/it's.sql"), "'/it'\\''s.sql'");
    assert_eq!(q("/$HOME/`id`;&|<>*?.sql"), "'/$HOME/`id`;&|<>*?.sql'");
    assert_eq!(q("/日本語.sql"), "'/日本語.sql'");
    // a control character is refused, not passed on
    assert_eq!(
        quote_path(Shell::Posix, "/a\nb.sql"),
        Err(BuildError::Unquotable('\n'))
    );
}

#[test]
fn a_path_is_one_word_for_cmd() {
    let q = |p: &str| quote_path(Shell::Cmd, p);
    assert_eq!(q("C:\\a b\\c.sql").unwrap(), "\"C:\\a b\\c.sql\"");
    // between double quotes these mean nothing to cmd.exe
    assert_eq!(
        q("C:\\a&b|c<d>e^f(g).sql").unwrap(),
        "\"C:\\a&b|c<d>e^f(g).sql\""
    );
    // a variable is still expanded between quotes, and a quote cannot be in the name
    assert_eq!(q("C:\\100%.sql"), Err(BuildError::Unquotable('%')));
    assert_eq!(q("C:\\a\"b.sql"), Err(BuildError::Unquotable('"')));
    // delayed expansion (`cmd /V:ON`) expands `!x!` between quotes too
    assert_eq!(q("C:\\report!TEMP!.sql"), Err(BuildError::Unquotable('!')));
    assert_eq!(q("C:\\a\tb.sql"), Err(BuildError::Unquotable('\t')));
}

// ---- what goes into the command line (the security review of ADR-0023, step 6) ----

fn built(shell: Shell, template: &str, file: &str) -> Result<String, BuildError> {
    build(shell, &connection(template), Some(file))
}

#[test]
fn a_template_that_quotes_the_file_itself_gets_plain_names_only() {
    // The name is part of an argument that the client or another shell reads itself (`psql -c '\i $F'`
    // runs a backquote), and no escaping is known to be right for that: only a plain name goes in, as it is.
    for template in [
        "psql -f '$F'",
        "psql -f \"$F\"",
        "psql -c '\\i $F'",
        "psql -c \"\\i $F\"",
        "sqlite3 db \".read $F\"",
    ] {
        assert_eq!(
            built(Shell::Posix, template, "/w/a.sql").unwrap(),
            template.replace("$F", "/w/a.sql")
        );
        for name in [
            "/w/a b.sql",
            "/w/a;touch pwned;b.sql",
            "/w/it's.sql",
            "/w/$(id).sql",
            "/w/`id`.sql",
            "/w/a\"b.sql",
            "/w/a\\b.sql",
        ] {
            assert!(
                built(Shell::Posix, template, name).is_err(),
                "{template} {name}"
            );
        }
    }
    // a quote of the other kind inside, or an escaped quote, does not change where the name is
    assert_eq!(
        built(Shell::Posix, "psql \"a'b\" $F", "/w/a b.sql").unwrap(),
        "psql \"a'b\" '/w/a b.sql'"
    );
    assert_eq!(
        built(Shell::Posix, "psql \\\" $F", "/w/a b.sql").unwrap(),
        "psql \\\" '/w/a b.sql'"
    );
    // a name that touches a quote of the template is spliced into the argument of the client
    assert!(built(Shell::Posix, "psql -c \"\\i '\"$F\"'\"", "/tmp/x'`id`'.sql").is_err());
    assert!(built(Shell::Posix, "psql -c \"\\i '\"$F\"'\"", "/tmp/x.sql").is_ok());
    assert!(
        built(
            Shell::Cmd,
            "psql -c \"\\i '\"$F\"'\"",
            "C:\\w\\x'`whoami`'.sql"
        )
        .is_err()
    );
    // outside quotes it is one word, whatever the name is
    assert_eq!(
        built(Shell::Posix, "psql -f $F", "/w/it's a;b.sql").unwrap(),
        "psql -f '/w/it'\\''s a;b.sql'"
    );
    // an escape right before the name would take its first character
    assert_eq!(
        built(Shell::Posix, "psql \\$F", "/w/a.sql"),
        Err(BuildError::EscapeBeforeFile)
    );
}

#[test]
fn the_same_for_cmd() {
    assert_eq!(
        built(Shell::Cmd, "sqlcmd -i \"$F\"", "C:\\w\\a.sql").unwrap(),
        "sqlcmd -i \"C:\\w\\a.sql\""
    );
    // a space or an ampersand in a quoted `$F` is not let in; unquoted, the name is one word
    assert!(built(Shell::Cmd, "sqlcmd -i \"$F\"", "C:\\w\\a b&c.sql").is_err());
    assert_eq!(
        built(Shell::Cmd, "sqlcmd -i $F", "C:\\w\\a b&c.sql").unwrap(),
        "sqlcmd -i \"C:\\w\\a b&c.sql\""
    );
    // a caret-escaped quote opens nothing
    assert_eq!(
        built(Shell::Cmd, "sqlcmd ^\" $F", "C:\\w\\a b.sql").unwrap(),
        "sqlcmd ^\" \"C:\\w\\a b.sql\""
    );
    assert_eq!(
        built(Shell::Cmd, "sqlcmd ^$F", "C:\\w\\a.sql"),
        Err(BuildError::EscapeBeforeFile)
    );
    // `%` and `!` expand between quotes too
    assert_eq!(
        built(Shell::Cmd, "sqlcmd -i $F", "C:\\100%.sql"),
        Err(BuildError::Unquotable('%'))
    );
    assert_eq!(
        built(Shell::Cmd, "sqlcmd -i $F", "C:\\report!TEMP!.sql"),
        Err(BuildError::Unquotable('!'))
    );
}

#[test]
fn only_a_known_client_gets_a_name_that_is_not_plain() {
    // another program (PowerShell, a script of the user's, a shell, `eval`, ...) may read the name as
    // code, so it gets letters, digits and `. _ - / + , : = @ ~` only
    for program in [
        "powershell -File $F",
        "pwsh.exe -File $F",
        "\"C:\\Program Files\\PowerShell\\7\\pwsh.exe\" -c $F",
        "./run-sql.sh $F",
        "eval \"psql -f $F\"",
        "%COMSPEC% /c \"sqlcmd -i $F\"",
        "p\\wsh -NoProfile -Command \"Write-Output '$F'\"",
        "env mysql --defaults-file=x < $F",
    ] {
        assert!(
            built(Shell::Cmd, program, "C:\\w\\a.sql").is_ok(),
            "{program}"
        );
        for name in [
            "C:\\w\\a b.sql",
            "C:\\w\\a;b.sql",
            "C:\\w\\$x.sql",
            "C:\\w\\it's.sql",
            "C:\\w\\(a).sql",
        ] {
            assert!(
                built(Shell::Cmd, program, name).is_err(),
                "{program} {name}"
            );
        }
    }
    // a client, with its arguments, takes any name as one word
    for client in [
        "sqlcmd -i $F",
        "psql -f $F",
        "\"C:\\Program Files\\Microsoft SQL Server\\sqlcmd.exe\" -i $F",
    ] {
        assert!(
            built(Shell::Cmd, client, "C:\\w\\a;b(1) c.sql").is_ok(),
            "{client}"
        );
    }
    // but not a template of more than one command, or with a substitution
    for compound in [
        "psql -f $F ; sh",
        "psql -f $F && echo x",
        "psql -f $F | sh",
        "psql -f $F < $F",
        "psql -f \"$(realpath $F)\"",
        "psql -f `realpath $F`",
    ] {
        assert!(
            built(Shell::Posix, compound, "/w/a b.sql").is_err(),
            "{compound}"
        );
        assert!(
            built(Shell::Posix, compound, "/w/a.sql").is_ok(),
            "{compound}"
        );
    }
}

#[test]
fn the_payloads_that_were_tried_are_refused() {
    for (template, name) in [
        ("sh -c $F", "/tmp/a;id;#.sql"),
        (
            "env pwsh -Command Write-Output $F",
            "/tmp/a;Write-Output INJECTED;#.sql",
        ),
        ("echo x ; bash -c $F", "/tmp/a;id;.sql"),
        ("eval \"psql -f $F\"", "/tmp/x; printf INJECTED; #.sql"),
        (
            "p\\wsh -NoProfile -Command \"Write-Output '$F'\"",
            "/tmp/x'; Write-Output INJECTED; #.sql",
        ),
        ("psql -f \"$(realpath $F)\"", "/tmp/a;id;.sql"),
        ("psql -c '\\i $F'", "/tmp/`id`.sql"),
    ] {
        assert!(
            built(Shell::Posix, template, name).is_err(),
            "{template} {name}"
        );
    }
    assert!(
        built(
            Shell::Cmd,
            "echo Running & cmd /c \"sqlcmd -i $F\"",
            "C:\\w\\a & echo INJECTED & rem .sql"
        )
        .is_err()
    );
}

/// The real `sh` reads the lines that are made (macOS and Linux): whatever the name is, it comes out
/// as one argument and nothing else runs.
#[cfg(unix)]
#[test]
fn the_real_sh_gets_the_name_as_one_argument_and_runs_nothing_else() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    // a client that writes each argument in angle brackets
    let client = dir.path().join("psql");
    std::fs::write(
        &client,
        "#!/bin/sh\nfor a in \"$@\"; do printf '<%s>' \"$a\"; done\n",
    )
    .unwrap();
    std::fs::set_permissions(&client, std::fs::Permissions::from_mode(0o755)).unwrap();
    let client = client.to_str().unwrap();
    let run = |line: &str| {
        let output = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(line)
            .current_dir(dir.path())
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).into_owned()
    };
    let names = [
        "plain.sql",
        "with space.sql",
        "semi;touch pwned.sql",
        "amp&touch pwned.sql",
        "pipe|touch pwned.sql",
        "quote'touch pwned'.sql",
        "dq\"touch pwned\".sql",
        "dollar$(touch pwned).sql",
        "back`touch pwned`.sql",
        "star*.sql",
        "-dash.sql",
        "hash#.sql",
        "tilde~.sql",
        "日本語 ファイル.sql",
        "back\\slash.sql",
        "percent%s.sql",
    ];
    for name in names {
        let file = dir.path().join(name);
        let file = file.to_str().unwrap();
        // unquoted in the template: one argument, any name
        let line = built(Shell::Posix, &format!("{client} -f $F"), file).unwrap();
        assert_eq!(run(&line), format!("<-f><{file}>"), "{name}: {line}");
        // quoted in the template: a plain name as it is, any other refused, and nothing ran either way
        for template in [format!("{client} -f '$F'"), format!("{client} -f \"$F\"")] {
            match built(Shell::Posix, &template, file) {
                Ok(line) => assert_eq!(run(&line), format!("<-f><{file}>"), "{name}: {line}"),
                Err(_) => assert!(
                    name.chars()
                        .any(|c| !c.is_alphanumeric() && !"._-".contains(c))
                ),
            }
        }
        assert!(!dir.path().join("pwned").exists(), "{name}");
    }
}

#[test]
fn the_template_gets_the_quoted_file() {
    let c = connection("psql -d db -f $F -v x=$$1");
    assert_eq!(
        build(Shell::Posix, &c, Some("/w/a b.sql")).unwrap(),
        "psql -d db -f '/w/a b.sql' -v x=$1"
    );
    assert_eq!(
        build(Shell::Cmd, &c, Some("C:\\w\\a b.sql")).unwrap(),
        "psql -d db -f \"C:\\w\\a b.sql\" -v x=$1"
    );
    // a template without the file needs none
    assert_eq!(
        build(Shell::Posix, &connection("sqlite3 db"), None).unwrap(),
        "sqlite3 db"
    );
    assert_eq!(build(Shell::Posix, &c, None), Err(BuildError::NoFile));
}

#[test]
fn a_line_too_long_for_the_exec_flow_is_refused_not_cut() {
    // a cut line could end inside the quotes
    let long = format!("/{}.sql", "a".repeat(1100));
    assert_eq!(
        build(Shell::Posix, &connection("x $F"), Some(&long)),
        Err(BuildError::TooLong)
    );
    assert!(build(Shell::Posix, &connection("x $F"), Some("/ok.sql")).is_ok());
}

#[test]
fn every_dollar_is_doubled_for_the_expansion_of_the_exec_flow() {
    // `ExpandParameter` turns `$$` back into `$`; the hostile-name test below goes through it
    assert_eq!(escape_for_expand("a $F $$ b"), "a $$F $$$$ b");
}

#[test]
fn the_option_bits_follow_the_connection() {
    let mut c = connection("tool $F");
    // the output is taken, line by line as UTF-8 or Shift_JIS by default; nothing is sent to the input
    assert_eq!(flags(&c).0, 0x01 | 0x400);
    // an explicit encoding is kept as it is
    c.encoding = Encoding::Utf8;
    assert_eq!(flags(&c).0, 0x01 | 0x80);
    c.encoding = Encoding::ShiftJis;
    assert_eq!(flags(&c).0, 0x01);
    c.encoding = Encoding::Utf16;
    c.folder = Some("/w".into());
    assert_eq!(flags(&c).0, 0x01 | 0x08 | 0x200);
    // without the file the text goes to the input, in the encoding of the output unless told
    let mut c = connection("sqlite3 db");
    assert_eq!(flags(&c).0, 0x01 | 0x400 | 0x04 | 0x100);
    c.encoding = Encoding::ShiftJis;
    c.input_encoding = Some(Encoding::Utf16);
    assert_eq!(flags(&c).0, 0x01 | 0x04 | 0x10);
}

// ---- the flow ----

struct Win {
    handle: AnyWindowHandle,
    view: Entity<EditorView>,
}

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

impl Win {
    fn call(&self, cx: &mut TestAppContext, function: &str) {
        let code = code_of(function);
        let view = self.view.clone();
        self.handle
            .update(cx, |_, window, app| {
                view.update(app, |view, cx| view.run_function(code, &[], window, cx))
            })
            .unwrap();
        cx.run_until_parked();
    }

    fn run(&self, cx: &mut TestAppContext) {
        self.call(cx, "F_PLSQL_COMPILE_ON_SQLPLUS");
    }

    fn cx<'a>(&self, cx: &'a mut TestAppContext) -> &'a mut VisualTestContext {
        VisualTestContext::from_window(self.handle, cx).into_mut()
    }

    fn prompt(&self, cx: &mut TestAppContext) -> Option<String> {
        self.cx(cx).pending_prompt().map(|(message, _)| message)
    }

    fn answer(&self, cx: &mut TestAppContext, button: &str) {
        let c = self.cx(cx);
        assert!(c.has_pending_prompt(), "no prompt to answer {button:?}");
        c.simulate_prompt_answer(button);
        c.run_until_parked();
    }

    fn output(&self, cx: &mut TestAppContext) -> String {
        self.view.read_with(cx, |view, _| {
            view.macros.output.text().replace("\r\n", "\n")
        })
    }

    fn running(&self, cx: &mut TestAppContext) -> bool {
        self.view
            .read_with(cx, |view, _| view.support.exec_running.is_some())
    }

    fn type_text(&self, cx: &mut TestAppContext, text: &str) {
        let text = text.to_owned();
        self.view.update(cx, |view, cx| {
            view.run_with(cx, |c| c.insert_text(&text)).unwrap();
        });
    }

    fn select(&self, cx: &mut TestAppContext, from: Pos, to: Pos) {
        self.view.update(cx, |view, _| view.cursor.select(from, to));
    }

    fn wait_for_end(&self, cx: &mut TestAppContext) {
        wait_until(cx, "the end of the run", |cx| {
            self.output(cx).contains("終了コード") && !self.running(cx)
        });
    }
}

/// Lets the window, the clock and the threads of the command run until `done` is true.
fn wait_until(
    cx: &mut TestAppContext,
    what: &str,
    mut done: impl FnMut(&mut TestAppContext) -> bool,
) {
    cx.executor().allow_parking();
    let begun = std::time::Instant::now();
    while begun.elapsed() < Duration::from_secs(60) {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if done(cx) {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("never happened: {what}");
}

fn open_file(cx: &mut TestAppContext, path: Option<&Path>) -> Win {
    cx.update(gpui_component::init);
    let request = path.map(|path| LoadRequest {
        path: path.to_owned(),
        encoding: None,
    });
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), request))
        .unwrap();
    cx.run_until_parked();
    let view = cx.update(|app| {
        editors(app)
            .into_iter()
            .find(|(h, _)| *h == handle)
            .map(|(_, view)| view)
            .unwrap()
    });
    Win { handle, view }
}

/// A folder with `databases.ini` (`ini` is its text; none: no file) and a window on `name`.
fn setup(
    cx: &mut TestAppContext,
    ini: Option<&str>,
    name: &str,
    text: &str,
) -> (tempfile::TempDir, PathBuf, Win) {
    let dir = tempfile::tempdir().unwrap();
    let settings = dir.path().join("settings");
    std::fs::create_dir(&settings).unwrap();
    let databases = settings.join("databases.ini");
    if let Some(ini) = ini {
        std::fs::write(&databases, ini).unwrap();
    }
    cx.update(|app| install(app, DatabasePrefs::at(databases.clone())));
    let file = dir.path().join(name);
    let text = if cfg!(windows) {
        text.replace('\n', "\r\n")
    } else {
        text.to_owned()
    };
    std::fs::write(&file, text).unwrap();
    let win = open_file(cx, Some(&file));
    (dir, file, win)
}

const ECHO: &str = "[Database]\nDefault=t\n\n[Connection.t]\nCommand=echo run [$F]\n";

#[gpui::test]
fn the_file_goes_to_the_client_and_the_output_to_the_panel(cx: &mut TestAppContext) {
    let (_dir, file, w) = setup(cx, Some(ECHO), "a.sql", "select 1;\n");
    w.run(cx);
    w.wait_for_end(cx);
    let output = w.output(cx);
    assert!(output.contains("run ["), "{output}");
    assert!(output.contains(&file.display().to_string()), "{output}");
    assert!(output.contains("#CmdLine  : echo run ["), "{output}");
    // no question, the panel is open
    assert!(w.prompt(cx).is_none());
    assert!(w.view.read_with(cx, |v, _| v.macros.output.is_open()));
}

#[gpui::test]
fn a_run_leaves_no_trace_in_the_history_of_commands(cx: &mut TestAppContext) {
    // the commands the user ran by hand are kept in `matsu.ini` (`[Cmd]`); the command of a connection is
    // in `databases.ini` and nowhere else, so what it holds does not get copied into another file
    let (dir, _file, w) = setup(cx, Some(ECHO), "a.sql", "select 1;\n");
    let ini = dir.path().join("matsu.ini");
    cx.update(|app| {
        crate::exec::install(app, crate::exec::ExecPrefs::load(None, Some(ini.clone())))
    });
    w.run(cx);
    w.wait_for_end(cx);
    assert!(
        cx.update(|app| crate::exec::history(app))
            .commands
            .items()
            .is_empty()
    );
    assert!(!ini.exists(), "nothing was written to the settings file");
}

#[gpui::test]
fn a_hostile_file_name_is_refused_for_a_program_that_is_not_a_client(cx: &mut TestAppContext) {
    // `echo` is not a database client, so it might read the name as code: only plain names go to it
    let name = "it's $F & echo INJECTED $(echo INJECTED).sql";
    let (_dir, _file, w) = setup(cx, Some(ECHO), name, "select 1;\n");
    w.run(cx);
    let message = w.prompt(cx).expect("the refusal");
    assert!(message.contains("安全に渡せない文字"), "{message}");
    w.answer(cx, "OK");
    assert_eq!(w.output(cx), "");
}

/// A client that writes each argument in angle brackets, named like a real one (`psql`).
#[cfg(unix)]
#[gpui::test]
fn a_hostile_file_name_stays_one_word_for_a_client(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt;

    let bin = tempfile::tempdir().unwrap();
    let client = bin.path().join("psql");
    std::fs::write(
        &client,
        "#!/bin/sh\nfor a in \"$@\"; do printf '<%s>\\n' \"$a\"; done\n",
    )
    .unwrap();
    std::fs::set_permissions(&client, std::fs::Permissions::from_mode(0o755)).unwrap();
    let ini = format!(
        "[Database]\nDefault=t\n\n[Connection.t]\nKind=postgresql\nCommand={} -f $F\n",
        client.display()
    );
    // quotes, `$F` (which would expand twice), `&` and `$(...)`
    let name = "it's $F & echo INJECTED $(echo INJECTED).sql";
    let (dir, file, w) = setup(cx, Some(&ini), name, "select 1;\n");
    w.run(cx);
    w.wait_for_end(cx);
    let output = w.output(cx);
    assert!(
        output.contains(&format!("<{}>", file.display())),
        "{output}"
    );
    assert!(
        output.lines().all(|line| line.trim() != "INJECTED"),
        "{output}"
    );
    assert!(file.exists() && dir.path().join("settings").exists());
}

#[gpui::test]
fn a_changed_document_asks_and_no_stops(cx: &mut TestAppContext) {
    let (_dir, file, w) = setup(cx, Some(ECHO), "a.sql", "select 1;\n");
    w.type_text(cx, "-- new\n");
    w.run(cx);
    let message = w.prompt(cx).expect("the question");
    assert!(message.contains(&file.display().to_string()), "{message}");
    assert!(message.contains("変更されています"), "{message}");
    w.answer(cx, "いいえ");
    // nothing ran, nothing was saved
    assert_eq!(w.output(cx), "");
    assert_eq!(std::fs::read_to_string(&file).unwrap().trim(), "select 1;");
}

#[gpui::test]
fn a_changed_encoding_alone_also_asks_to_save(cx: &mut TestAppContext) {
    // the client reads the file on disk, so an unsaved BOM is a change too
    let (_dir, file, w) = setup(cx, Some(ECHO), "a.sql", "select 1;\n");
    w.view.update(cx, |view, cx| {
        let document = view.document().clone();
        document.update(cx, |shared, _| {
            shared.set_format(matsu_core::encoding::Encoding::Utf8, true)
        });
    });
    w.run(cx);
    let message = w.prompt(cx).expect("the question");
    assert!(message.contains("変更されています"), "{message}");
    w.answer(cx, "いいえ");
    assert_eq!(w.output(cx), "");
    assert!(
        !std::fs::read(&file)
            .unwrap()
            .starts_with(&[0xEF, 0xBB, 0xBF])
    );
}

#[gpui::test]
fn cancel_also_stops(cx: &mut TestAppContext) {
    let (_dir, _file, w) = setup(cx, Some(ECHO), "a.sql", "select 1;\n");
    w.type_text(cx, "x");
    w.run(cx);
    w.answer(cx, "キャンセル");
    assert_eq!(w.output(cx), "");
}

#[gpui::test]
fn yes_saves_and_runs(cx: &mut TestAppContext) {
    let (_dir, file, w) = setup(cx, Some(ECHO), "a.sql", "select 1;\n");
    // no line break typed: on Windows the file has CR LF, and a lone LF would make the save ask
    // about the mixed line endings
    w.type_text(cx, "-- new ");
    w.run(cx);
    w.answer(cx, "はい");
    w.wait_for_end(cx);
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .starts_with("-- new"),
        "saved before the run"
    );
    assert!(w.output(cx).contains("run ["));
}

#[gpui::test]
fn a_document_without_a_file_cannot_be_run_by_file(cx: &mut TestAppContext) {
    let (_dir, _file, _w) = setup(cx, Some(ECHO), "a.sql", "");
    // a second window on no file
    let w = open_file(cx, None);
    w.run(cx);
    let message = w.prompt(cx).expect("the error");
    assert!(message.contains("保存しないと"), "{message}");
    w.answer(cx, "OK");
    assert_eq!(w.output(cx), "");
}

#[gpui::test]
fn a_connection_without_the_file_takes_the_text_on_its_input(cx: &mut TestAppContext) {
    let ini = format!("[Database]\nDefault=t\n\n[Connection.t]\nCommand={COPY}\n");
    let (_dir, _file, w) = setup(cx, Some(&ini), "a.sql", "select 1;\nselect 2;\n");
    // no file is needed and no question about saving is asked
    w.type_text(cx, "-- edited\n");
    w.run(cx);
    assert!(w.prompt(cx).is_none());
    w.wait_for_end(cx);
    let output = w.output(cx);
    assert!(output.contains("-- edited"), "{output}");
    assert!(output.contains("select 2;"), "{output}");
}

#[gpui::test]
fn the_selection_is_what_a_connection_without_the_file_gets(cx: &mut TestAppContext) {
    let ini = format!("[Database]\nDefault=t\n\n[Connection.t]\nCommand={COPY}\n");
    let (_dir, _file, w) = setup(cx, Some(&ini), "a.sql", "select 1;\nselect 2;\n");
    w.select(cx, Pos::new(1, 0), Pos::new(1, 9));
    w.run(cx);
    w.wait_for_end(cx);
    let output = w.output(cx);
    assert!(output.contains("select 2;"), "{output}");
    assert!(!output.contains("select 1;"), "{output}");
}

#[gpui::test]
fn the_folder_of_the_connection_is_where_the_client_runs(cx: &mut TestAppContext) {
    let list = if cfg!(windows) { "cmd /C dir /B" } else { "ls" };
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("marker-in-the-folder.txt"), "").unwrap();
    // an INI value is one line; backslashes of a Windows path are kept
    let ini = format!(
        "[Database]\nDefault=t\n\n[Connection.t]\nCommand={list}\nFolder={}\n",
        work.path().display()
    );
    let (_dir, _file, w) = setup(cx, Some(&ini), "a.sql", "x\n");
    w.run(cx);
    w.wait_for_end(cx);
    assert!(w.output(cx).contains("marker-in-the-folder.txt"));
}

#[gpui::test]
fn a_password_in_the_command_is_not_run(cx: &mut TestAppContext) {
    let ini = "[Database]\nDefault=t\n\n[Connection.t]\nKind=mysql\nCommand=echo mysql -u me -psecret db\n";
    let (_dir, _file, w) = setup(cx, Some(ini), "a.sql", "x\n");
    w.run(cx);
    let message = w.prompt(cx).expect("the refusal");
    assert!(message.contains("パスワード"), "{message}");
    // the secret is not repeated in the message
    assert!(!message.contains("secret"), "{message}");
    w.answer(cx, "OK");
    assert_eq!(w.output(cx), "");
}

#[gpui::test]
fn without_a_file_the_example_is_offered_and_made(cx: &mut TestAppContext) {
    let (dir, _file, w) = setup(cx, None, "a.sql", "x\n");
    let databases = dir.path().join("settings").join("databases.ini");
    w.run(cx);
    let message = w.prompt(cx).expect("the offer");
    assert!(message.contains("databases.ini"), "{message}");
    w.answer(cx, "作る");
    let told = w.prompt(cx).expect("where it is");
    assert!(told.contains("Default="), "{told}");
    w.answer(cx, "OK");
    let text = std::fs::read_to_string(&databases).unwrap();
    assert!(text.contains("[Connection.postgresql]"), "{text}");
    // examples only: nothing is chosen, so the next run asks for the choice
    w.run(cx);
    let message = w.prompt(cx).expect("no default");
    assert!(message.contains("Default="), "{message}");
    w.answer(cx, "OK");
    assert_eq!(w.output(cx), "");
}

#[gpui::test]
fn declining_the_example_makes_nothing(cx: &mut TestAppContext) {
    let (dir, _file, w) = setup(cx, None, "a.sql", "x\n");
    w.run(cx);
    w.answer(cx, "作らない");
    assert!(w.prompt(cx).is_none());
    assert!(!dir.path().join("settings").join("databases.ini").exists());
}

#[gpui::test]
fn a_default_that_is_not_in_the_file_says_so(cx: &mut TestAppContext) {
    let ini = "[Database]\nDefault=gone\n\n[Connection.t]\nCommand=echo x\n";
    let (_dir, _file, w) = setup(cx, Some(ini), "a.sql", "x\n");
    w.run(cx);
    let message = w.prompt(cx).expect("the problem");
    assert!(message.contains("Default=gone"), "{message}");
    w.answer(cx, "OK");
    assert_eq!(w.output(cx), "");
}

#[gpui::test]
fn a_mistake_in_the_default_is_told_with_its_reason(cx: &mut TestAppContext) {
    let ini = "[Database]\nDefault=t\n\n[Connection.t]\nKind=cobol\nCommand=echo x\n";
    let (_dir, _file, w) = setup(cx, Some(ini), "a.sql", "x\n");
    w.run(cx);
    let message = w.prompt(cx).expect("the problem");
    assert!(message.contains("cobol"), "{message}");
    w.answer(cx, "OK");
}

#[gpui::test]
fn activating_shows_the_output_panel(cx: &mut TestAppContext) {
    let (_dir, _file, w) = setup(cx, Some(ECHO), "a.sql", "x\n");
    assert!(!w.view.read_with(cx, |v, _| v.macros.output.is_open()));
    w.call(cx, "F_ACTIVATE_SQLPLUS");
    assert!(w.view.read_with(cx, |v, _| v.macros.output.is_open()));
}

#[gpui::test]
fn f10_and_shift_f11_are_bound_to_them(cx: &mut TestAppContext) {
    // the keys of Sakura's table reach the commands (no "まだ使えない")
    let (_dir, _file, w) = setup(cx, Some(ECHO), "a.sql", "x\n");
    w.cx(cx).simulate_keystrokes("shift-f11");
    assert!(w.view.read_with(cx, |v, _| v.macros.output.is_open()));
    assert!(w.view.read_with(cx, |v, _| v.notice.is_none()));
    w.cx(cx).simulate_keystrokes("f10");
    w.wait_for_end(cx);
    assert!(w.output(cx).contains("run ["));
}

// ---- the jump from an error line (step 3 of ADR-0023) ----

use crate::macros::Place;

impl Win {
    fn places(&self, cx: &mut TestAppContext) -> Vec<(String, Place)> {
        self.view
            .read_with(cx, |view, _| view.macros.output.places())
    }

    fn click(&self, cx: &mut TestAppContext, place: Place) {
        let view = self.view.clone();
        self.handle
            .update(cx, |_, window, app| {
                view.update(app, |view, cx| view.jump_to_place(place, window, cx))
            })
            .unwrap();
        cx.run_until_parked();
    }

    fn caret(&self, cx: &mut TestAppContext) -> Pos {
        self.view.read_with(cx, |view, _| view.cursor().caret())
    }
}

#[gpui::test]
fn a_client_error_line_leads_to_the_line_of_the_file(cx: &mut TestAppContext) {
    // a fake client that writes `FILE:LINE: ERROR`, with the file relative to the folder of the run
    let ini = "[Database]\nDefault=t\n\n[Connection.t]\nCommand=echo a.sql:3: ERROR: boom\n";
    let (_dir, file, w) = setup(cx, Some(ini), "a.sql", "one;\ntwo;\nthree;\nfour;\n");
    w.run(cx);
    w.wait_for_end(cx);
    let places = w.places(cx);
    assert_eq!(places.len(), 1, "{places:?}");
    assert_eq!(places[0].0, "a.sql:3: ERROR: boom");
    let Place::File(target) = &places[0].1 else {
        panic!("{:?}", places[0]);
    };
    assert_eq!(target.path, file);
    assert_eq!(target.line, Some(3));
    // the click moves the caret there
    w.click(cx, places[0].1.clone());
    assert_eq!(w.caret(cx).line, 2);
}

#[gpui::test]
fn the_line_counts_from_the_selection_when_only_it_was_sent(cx: &mut TestAppContext) {
    // the client (`cat`) writes back what it got; psql names the line of the input
    let ini = format!("[Database]\nDefault=t\n\n[Connection.t]\nKind=postgresql\nCommand={COPY}\n");
    let text = "select 1;\npsql:<stdin>:1: ERROR: boom\nselect 3;\n";
    let (_dir, file, w) = setup(cx, Some(&ini), "a.sql", text);
    w.select(cx, Pos::new(1, 0), Pos::new(1, 27));
    w.run(cx);
    w.wait_for_end(cx);
    let places = w.places(cx);
    assert_eq!(places.len(), 1, "{places:?}");
    let Place::File(target) = &places[0].1 else {
        panic!("{:?}", places[0]);
    };
    // line 1 of the selection is line 2 of the document
    assert_eq!(
        (target.path.as_path(), target.line),
        (file.as_path(), Some(2))
    );
}

#[gpui::test]
fn another_command_takes_the_jump_away(cx: &mut TestAppContext) {
    let ini = "[Database]\nDefault=t\n\n[Connection.t]\nCommand=echo a.sql:1: ERROR: x\n";
    let (_dir, _file, w) = setup(cx, Some(ini), "a.sql", "x\n");
    w.run(cx);
    w.wait_for_end(cx);
    // 外部コマンド実行 of a macro: its output has no errors to lead anywhere
    let code = code_of("F_EXECMD");
    let view = w.view.clone();
    w.handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| {
                view.run_function(
                    code,
                    &[
                        matsu_commands::command::Arg::Text("echo other a.sql:1: x".into()),
                        matsu_commands::command::Arg::Int(1),
                    ],
                    window,
                    cx,
                )
            })
        })
        .unwrap();
    wait_until(cx, "the second run", |cx| {
        w.output(cx).contains("other a.sql") && !w.running(cx)
    });
    // the line of the second run leads nowhere; the first run's error line keeps its place
    let places = w.places(cx);
    assert!(places.iter().all(|(text, _)| !text.starts_with("other")));
    assert!(places.iter().any(|(text, _)| text == "a.sql:1: ERROR: x"));
}

#[gpui::test]
fn a_document_without_a_file_gets_no_jump(cx: &mut TestAppContext) {
    let ini = format!("[Database]\nDefault=t\n\n[Connection.t]\nKind=postgresql\nCommand={COPY}\n");
    let (_dir, _file, _w) = setup(cx, Some(&ini), "a.sql", "x\n");
    let w = open_file(cx, None);
    w.type_text(cx, "psql:<stdin>:1: ERROR: boom");
    w.run(cx);
    w.wait_for_end(cx);
    assert!(w.output(cx).contains("ERROR: boom"));
    assert!(w.places(cx).is_empty());
}

// ---- choosing the connection ----

const CHOICE: &str = "F_PLSQL_COMPILE_ON_DATABASE_CHOICE";

/// Two connections; `a` is the default.
const TWO: &str = "[Database]\nDefault=a\n\n[Connection.a]\nKind=other\nCommand=echo from-a [$F]\n\n[Connection.b]\nKind=sqlite\nCommand=echo from-b [$F]\n";

impl Win {
    fn choose(&self, cx: &mut TestAppContext) {
        self.call(cx, CHOICE);
    }

    /// The name of the row that is chosen in the open dialog.
    fn chosen(&self, cx: &mut TestAppContext) -> Option<String> {
        self.view.read_with(cx, |view, app| {
            view.connection_dialog
                .as_ref()
                .map(|d| d.read(app).chosen_name().to_owned())
        })
    }

    fn keys(&self, cx: &mut TestAppContext, keys: &str) {
        let c = self.cx(cx);
        c.simulate_keystrokes(keys);
        c.run_until_parked();
    }
}

#[test]
fn the_dialog_lists_the_usable_connections_and_notes_the_others() {
    use matsu_config::databases::Databases;
    let ini = "[Database]\nDefault=b\n\n[Connection.a]\nKind=oracle\nCommand=sqlplus -S /@x @$F\n\n[Connection.b]\nKind=postgresql\nCommand=psql -f $F\n\n[Connection.c]\nKind=cobol\nCommand=x\n\n[Connection.d]\nKind=mysql\n";
    let databases =
        Databases::from_profile(&matsu_config::profile::Profile::from_bytes(ini.as_bytes()));
    let request = super::run::pick_request(&databases);
    let names: Vec<_> = request.choices.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["a", "b"]);
    assert_eq!(request.choices[0].kind, "oracle");
    assert_eq!(request.choices[1].command, "psql -f $F");
    // the default starts chosen
    assert_eq!(request.initial, Some(1));
    let note = request.note.expect("the unusable ones");
    assert!(note.contains("cobol"), "{note}");
    assert!(note.contains("Command="), "{note}");
}

#[test]
fn a_default_that_is_not_there_chooses_the_first_and_is_no_note() {
    use matsu_config::databases::Databases;
    let ini = "[Database]\nDefault=gone\n\n[Connection.a]\nCommand=x\n";
    let databases =
        Databases::from_profile(&matsu_config::profile::Profile::from_bytes(ini.as_bytes()));
    let request = super::run::pick_request(&databases);
    assert_eq!(request.initial, None);
    assert_eq!(request.note, None);
}

#[gpui::test]
fn the_default_message_points_at_the_choosing_command(cx: &mut TestAppContext) {
    let ini = "[Database]\n\n[Connection.t]\nCommand=echo x\n";
    let (_dir, _file, w) = setup(cx, Some(ini), "a.sql", "x\n");
    w.run(cx);
    let message = w.prompt(cx).expect("no default");
    let label = FUNCTIONS
        .iter()
        .find(|f| f.name == CHOICE)
        .and_then(|f| f.label)
        .unwrap();
    assert!(message.contains(label), "{message}");
    w.answer(cx, "OK");
}

#[gpui::test]
fn choosing_without_the_file_offers_the_example(cx: &mut TestAppContext) {
    let (dir, _file, w) = setup(cx, None, "a.sql", "x\n");
    w.choose(cx);
    let message = w.prompt(cx).expect("the offer");
    assert!(message.contains("databases.ini"), "{message}");
    w.answer(cx, "作らない");
    assert!(!dir.path().join("settings").join("databases.ini").exists());
    assert_eq!(w.chosen(cx), None);
}

#[gpui::test]
fn choosing_with_no_connection_says_so(cx: &mut TestAppContext) {
    let (_dir, _file, w) = setup(cx, Some("[Database]\nDefault=t\n"), "a.sql", "x\n");
    w.choose(cx);
    let message = w.prompt(cx).expect("nothing to choose");
    assert!(message.contains("[Connection.名前]"), "{message}");
    w.answer(cx, "OK");
    assert_eq!(w.chosen(cx), None);
    assert_eq!(w.output(cx), "");
}

#[gpui::test]
fn with_one_connection_the_list_has_one_row_and_enter_runs_it(cx: &mut TestAppContext) {
    let (_dir, file, w) = setup(cx, Some(ECHO), "a.sql", "select 1;\n");
    w.choose(cx);
    // the list is shown even for one connection
    assert_eq!(w.chosen(cx).as_deref(), Some("t"));
    assert_eq!(w.output(cx), "");
    w.keys(cx, "enter");
    assert_eq!(w.chosen(cx), None, "the dialog is closed");
    w.wait_for_end(cx);
    let output = w.output(cx);
    assert!(output.contains(&file.display().to_string()), "{output}");
}

#[gpui::test]
fn with_two_connections_the_chosen_one_runs_and_the_file_is_not_changed(cx: &mut TestAppContext) {
    let (dir, _file, w) = setup(cx, Some(TWO), "a.sql", "select 1;\n");
    let databases = dir.path().join("settings").join("databases.ini");
    w.choose(cx);
    // the default starts chosen; Down moves to the next, Up back, and the ends hold
    assert_eq!(w.chosen(cx).as_deref(), Some("a"));
    w.keys(cx, "down");
    assert_eq!(w.chosen(cx).as_deref(), Some("b"));
    w.keys(cx, "down");
    assert_eq!(w.chosen(cx).as_deref(), Some("b"));
    w.keys(cx, "up");
    assert_eq!(w.chosen(cx).as_deref(), Some("a"));
    w.keys(cx, "end");
    assert_eq!(w.chosen(cx).as_deref(), Some("b"));
    w.keys(cx, "enter");
    w.wait_for_end(cx);
    let output = w.output(cx);
    assert!(output.contains("from-b"), "{output}");
    assert!(!output.contains("from-a"), "{output}");
    // databases.ini is only read: the default is still a
    assert_eq!(std::fs::read_to_string(&databases).unwrap(), TWO);
    // and F10 still runs the default
    w.run(cx);
    wait_until(cx, "the second run", |cx| w.output(cx).contains("from-a"));
}

#[gpui::test]
fn escape_runs_nothing(cx: &mut TestAppContext) {
    let (_dir, _file, w) = setup(cx, Some(TWO), "a.sql", "x\n");
    w.choose(cx);
    w.keys(cx, "escape");
    assert_eq!(w.chosen(cx), None);
    assert_eq!(w.output(cx), "");
    assert!(!w.running(cx));
    // it can be opened once more; Tab Tab reaches Cancel
    w.choose(cx);
    assert_eq!(w.chosen(cx).as_deref(), Some("a"));
    w.keys(cx, "tab tab enter");
    assert_eq!(w.chosen(cx), None);
    assert_eq!(w.output(cx), "");
}

#[gpui::test]
fn a_connection_with_a_mistake_is_not_in_the_list(cx: &mut TestAppContext) {
    let ini = format!("{TWO}\n[Connection.bad]\nKind=cobol\nCommand=echo bad\n");
    let (_dir, _file, w) = setup(cx, Some(&ini), "a.sql", "x\n");
    w.choose(cx);
    w.keys(cx, "end");
    assert_eq!(w.chosen(cx).as_deref(), Some("b"), "bad is not a row");
    w.keys(cx, "escape");
}

#[gpui::test]
fn a_chosen_connection_goes_through_the_same_checks_as_the_default(cx: &mut TestAppContext) {
    let ini = format!("{TWO}\n[Connection.pw]\nKind=mysql\nCommand=echo mysql -u me -psecret db\n");
    let (_dir, _file, w) = setup(cx, Some(&ini), "a.sql", "x\n");
    w.choose(cx);
    w.keys(cx, "end enter");
    let message = w.prompt(cx).expect("the refusal");
    assert!(message.contains("パスワード"), "{message}");
    assert!(!message.contains("secret"), "{message}");
    w.answer(cx, "OK");
    assert_eq!(w.output(cx), "");
}

#[gpui::test]
fn a_changed_document_asks_before_the_chosen_connection_runs(cx: &mut TestAppContext) {
    let (_dir, _file, w) = setup(cx, Some(TWO), "a.sql", "x\n");
    w.type_text(cx, "y");
    w.choose(cx);
    w.keys(cx, "enter");
    let message = w.prompt(cx).expect("the question");
    assert!(
        message.contains("データベースで実行する前に保存"),
        "{message}"
    );
    w.answer(cx, "いいえ");
    assert_eq!(w.output(cx), "");
}
