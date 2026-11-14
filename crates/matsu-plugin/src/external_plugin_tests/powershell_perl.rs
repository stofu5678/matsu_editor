//! PowerShell and Perl plugins (ADR-0026, PR 10 continued). The same plugin objects as in the other
//! languages, in each language's own spelling: `$Plugin.GetOption(...)` in PowerShell,
//! `Plugin->GetOption(...)` in Perl. Each test skips when the interpreter is missing
//! (`MATSU_REQUIRE_INTERPRETERS` makes that a failure on the Linux x64 runner, which has both).
//! The handlers end their lines with a line break: these tests do not depend on how the host treats
//! output that is written before an editor call without one.

use super::*;

const PS: Language = Language::PowerShell;
const PERL: Language = Language::Perl;

/// Whether a call that died should be made again: the pwsh build on the Linux ARM64 CI runners dies
/// with SIGSEGV now and then, before it has printed anything (see `crate::tests`). On the other OSes
/// that would be news, so it is not hidden. A wrong output, another signal or a timeout is never
/// made again.
fn pwsh_crashed(language: Language, report: &Report) -> bool {
    cfg!(all(target_os = "linux", target_arch = "aarch64"))
        && language == PS
        && report.output.is_empty()
        && matches!(&report.outcome, Outcome::Died(text) if text.contains("signal: 11"))
}

fn call_jack(
    world: &mut World,
    host: &mut PluginHost,
    jack: &str,
    handler: &str,
    args: &Value,
    timeout: Duration,
) -> Report {
    let language = host.settings().language;
    let mut report = world.call_jack(host, jack, handler, args, timeout);
    for _ in 0..2 {
        if !pwsh_crashed(language, &report) {
            break;
        }
        eprintln!("RETRY: pwsh crashed with SIGSEGV and printed nothing; calling again");
        host.reset();
        report = world.call_jack(host, jack, handler, args, timeout);
    }
    report
}

fn call(world: &mut World, host: &mut PluginHost, handler: &str) -> Report {
    call_jack(world, host, "Command", handler, &json!({"n": 1}), PATIENT)
}

#[test]
fn a_powershell_or_perl_plugin_runs_with_the_plugin_object_and_keeps_its_state() {
    // `$global:n` (PowerShell) and the package variable `$n` (Perl) stay for the next call.
    let ps = "
$global:n = [int]$global:n + 1
Write-Output \"$($Plugin.Jack) $($Plugin.Args['n']) $($Plugin.Id) $($global:n)\"
Write-Output \"$($Plugin.GetPluginDir()) $($Plugin.GetDef('Plugin', 'Name'))\"
[void]$Plugin.SetOption('Main', 'Level', '7')
Write-Output \"$($Plugin.GetOption('Main', 'Level')) $($Plugin.GetString(2))\"
$Editor.InsText('名前')
";
    let perl = "
our $n;
$n++;
print Plugin->jack, ' ', Plugin->args->{n}, ' ', Plugin->id, \" $n\\n\";
print Plugin->GetPluginDir, ' ', Plugin->GetDef('Plugin', 'Name'), \"\\n\";
Plugin->SetOption('Main', 'Level', '7');
print Plugin->GetOption('Main', 'Level'), ' ', Plugin->GetString(2), \"\\n\";
Editor->InsText('名前');
";
    for (language, name, code) in [(PS, "h.ps1", ps), (PERL, "h.pl", perl)] {
        if !available(language) {
            continue;
        }
        let dir = plugin_dir(&[(name, code)]);
        let mut host = host(dir.path(), language, true);
        let mut world = World::new("");
        let first = call(&mut world, &mut host, name);
        assert_eq!(first.outcome, Outcome::Ok, "{name}: {:?}", first.output);
        assert_eq!(
            stdout(&first),
            [
                "Command 1 jp.example.test 1",
                "/plugins/テスト def:Plugin:Name",
                "7 string 2",
            ],
            "{name}"
        );
        let pid = host.process_id().expect("it stays");
        let second = call(&mut world, &mut host, name);
        assert_eq!(host.process_id(), Some(pid), "{name}");
        assert!(
            stdout(&second)[0].ends_with(" 2"),
            "{name}: {:?}",
            second.output
        );
        assert_eq!(world.doc.text(), "名前名前", "{name}");
        host.shutdown();
    }
}

#[test]
fn a_powershell_handler_has_a_scope_of_its_own_and_global_variables_stay() {
    if !available(PS) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "set.ps1",
            "$local = 5\n$global:kept = 6\nfunction Local-Fn { 'x' }\n$Editor.InsText('s')\n",
        ),
        (
            "get.ps1",
            "Write-Output \"$($null -eq $local) $global:kept $($null -eq (Get-Command Local-Fn -ErrorAction SilentlyContinue))\"\n",
        ),
    ]);
    let mut host = host(dir.path(), PS, true);
    let mut world = World::new("");
    assert_eq!(call(&mut world, &mut host, "set.ps1").outcome, Outcome::Ok);
    let report = call(&mut world, &mut host, "get.ps1");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(stdout(&report), ["True 6 True"], "{:?}", report.output);
    host.shutdown();
}

#[test]
fn a_powershell_handler_cannot_overwrite_the_hosts_variables() {
    if !available(PS) {
        return;
    }
    // The names are those of the locals of the host's loop and of its handler runner.
    let dir = plugin_dir(&[
        (
            "clobber.ps1",
            "$message = 'hello'\n$params = 1\n$name = 2\n$source = 3\n$failure = 4\n$text = 5\n$identity = 6\n$mpName = 7\n$mpSource = 8\n$mpBlock = 9\n$mpAst = 10\n$mpErrors = 11\n$Editor.InsText('c')\n",
        ),
        ("ok.ps1", "$Editor.InsText('o')\n"),
    ]);
    let mut host = host(dir.path(), PS, true);
    let mut world = World::new("");
    for name in ["clobber.ps1", "ok.ps1", "clobber.ps1"] {
        let report = call(&mut world, &mut host, name);
        assert_eq!(report.outcome, Outcome::Ok, "{name}: {:?}", report.output);
    }
    assert_eq!(world.doc.text(), "coc");
    host.shutdown();
}

#[test]
fn a_perl_handler_cannot_see_or_overwrite_the_hosts_variables() {
    if !available(PERL) {
        return;
    }
    // `$json`, `$RS` and `$next_id` are lexicals of the host: the handler has package variables of
    // the same names, which are empty, and `@_` is empty too.
    let dir = plugin_dir(&[
        (
            "clobber.pl",
            "print defined($json) ? 1 : 0, defined($RS) ? 1 : 0, defined($next_id) ? 1 : 0, defined(shift) ? 1 : 0, \"\\n\";\n$json = 'x';\n$next_id = 99;\n@ARGV = ();\nmy $message = 'h';\nEditor->InsText('c');\n",
        ),
        ("ok.pl", "Editor->InsText('o');\n"),
    ]);
    let mut host = host(dir.path(), PERL, true);
    let mut world = World::new("");
    let mut seen = Vec::new();
    for name in ["clobber.pl", "ok.pl", "clobber.pl"] {
        let report = call(&mut world, &mut host, name);
        assert_eq!(report.outcome, Outcome::Ok, "{name}: {:?}", report.output);
        seen.extend(stdout(&report).iter().map(|l| (*l).to_owned()));
    }
    // The second run sees the package variables that the first one set (they stay, as with `our`);
    // the host's own values were never touched, or the calls would have failed.
    assert_eq!(seen, ["0000", "1010"]);
    assert_eq!(world.doc.text(), "coc");
    host.shutdown();
}

#[test]
fn a_powershell_or_perl_handler_may_load_the_files_of_its_folder_again_and_again() {
    let files = [
        (
            "req.ps1",
            ". (Join-Path $Plugin.Dir 'plugin_lib/word.ps1')\n$Editor.InsText((Get-Word))\n",
        ),
        ("plugin_lib/word.ps1", "function Get-Word { 'word' }\n"),
        (
            "req.pl",
            "require 'plugin_lib/word.pl';\nEditor->InsText(word_text());\n",
        ),
        (
            "plugin_lib/word.pl",
            "use utf8;\nsub word_text { return 'こと' }\n1;\n",
        ),
    ];
    for (language, name, expected) in [(PS, "req.ps1", "wordword"), (PERL, "req.pl", "ことこと")]
    {
        if !available(language) {
            continue;
        }
        let dir = plugin_dir(&files);
        let mut host = host(dir.path(), language, true);
        let mut world = World::new("");
        for _ in 0..2 {
            let report = call(&mut world, &mut host, name);
            assert_eq!(report.outcome, Outcome::Ok, "{name}: {:?}", report.output);
        }
        assert_eq!(world.doc.text(), expected, "{name}");
        host.shutdown();
    }
}

#[test]
fn a_powershell_or_perl_handler_that_fails_keeps_the_kind_and_the_place_and_the_host_stays() {
    let cases = [
        (
            PS,
            "bad.ps1",
            "$a = 1\nthrow 'だめ'\n",
            ["bad.ps1:2", "だめ"],
        ),
        (
            PS,
            "runtime.ps1",
            "$a = 1\n[int]'abc'\n",
            ["runtime.ps1:2", "RuntimeException"],
        ),
        (
            PS,
            "syntax.ps1",
            "$a = 1\n$b = (\n",
            ["syntax.ps1:2", "ParseError"],
        ),
        (
            PS,
            "editor.ps1",
            "$a = 1\n$Editor.GetLineStr('x')\n",
            ["editor.ps1:2", "EditorError"],
        ),
        (
            PERL,
            "bad.pl",
            "my $a = 1;\ndie 'だめ';\n",
            ["bad.pl line 2", "だめ"],
        ),
        (
            PERL,
            "runtime.pl",
            "my $a = 1;\nmy $h = undef;\n$h->no_such_method;\n",
            ["runtime.pl line 3", "Can't call method \"no_such_method\""],
        ),
        (
            PERL,
            "syntax.pl",
            "my $a = 1;\nmy $b = ;\n",
            ["syntax.pl line 2", "syntax error"],
        ),
        (
            PERL,
            "editor.pl",
            "my $a = 1;\nEditor->GetLineStr('x');\n",
            ["editor.pl line 2", "EditorError"],
        ),
    ];
    for (language, name, code, needles) in cases {
        if !available(language) {
            continue;
        }
        let (ok, ok_code) = if language == PS {
            ("ok.ps1", "$Editor.InsText('ok')")
        } else {
            ("ok.pl", "Editor->InsText('ok');")
        };
        let dir = plugin_dir(&[(name, code), (ok, ok_code)]);
        let mut host = host(dir.path(), language, true);
        let mut world = World::new("");
        let report = call(&mut world, &mut host, name);
        let Outcome::Failed(text) = &report.outcome else {
            panic!("{name}: {:?}", report.outcome);
        };
        for needle in needles {
            assert!(text.contains(needle), "{name}: {needle} not in {text}");
        }
        // A failed call ends its process (the host does that for every language); the next call
        // starts a new one and is served.
        assert_eq!(
            call(&mut world, &mut host, ok).outcome,
            Outcome::Ok,
            "{name}"
        );
        assert!(host.process_id().is_some(), "{name}");
        assert_eq!(world.doc.text(), "ok", "{name}");
        host.shutdown();
    }
}

#[test]
fn a_return_or_exit_ends_only_the_handler() {
    let cases = [
        (
            PS,
            "early.ps1",
            "$Editor.InsText('a')\nreturn\n$Editor.InsText('b')\n",
        ),
        (
            PS,
            "exit.ps1",
            "$Editor.InsText('a')\nif ($true) {\n    exit 3\n}\n$Editor.InsText('b')\n",
        ),
        (
            PS,
            "exit-in-function.ps1",
            "function Stop-Here { exit 4 }\n$Editor.InsText('a')\nStop-Here\n$Editor.InsText('b')\n",
        ),
        (
            PS,
            "exit-with-argument.ps1",
            "function Get-Code { $global:argument_ran = 'yes'; 3 }\n$Editor.InsText('a')\nexit (Get-Code)\n$Editor.InsText('b')\n",
        ),
        (
            PS,
            "exit-nested.ps1",
            "$Editor.InsText('a')\nexit $(exit 2)\n$Editor.InsText('b')\n",
        ),
        (
            PERL,
            "early.pl",
            "Editor->InsText('a');\nreturn;\nEditor->InsText('b');\n",
        ),
        (
            PERL,
            "exit.pl",
            "Editor->InsText('a');\nexit 3;\nEditor->InsText('b');\n",
        ),
        (
            PERL,
            "exit-in-sub.pl",
            "sub stop_here { exit 4 }\nEditor->InsText('a');\nstop_here();\nEditor->InsText('b');\n",
        ),
        (
            PERL,
            "inner-eval.pl",
            // A failed `eval` of the handler's own before `return` is not a failure of the handler.
            "eval { die 'inner' };\nEditor->InsText('a');\nreturn 1;\n",
        ),
    ];
    for (language, name, code) in cases {
        if !available(language) {
            continue;
        }
        let ok = if language == PS { "ok.ps1" } else { "ok.pl" };
        let ok_code = if language == PS {
            "$Editor.InsText('o')\n"
        } else {
            "Editor->InsText('o');\n"
        };
        let dir = plugin_dir(&[(name, code), (ok, ok_code)]);
        let mut host = host(dir.path(), language, true);
        let mut world = World::new("");
        let report = call(&mut world, &mut host, name);
        assert_eq!(report.outcome, Outcome::Ok, "{name}: {:?}", report.output);
        let pid = host.process_id().expect("it stays");
        assert_eq!(
            call(&mut world, &mut host, ok).outcome,
            Outcome::Ok,
            "{name}"
        );
        assert_eq!(host.process_id(), Some(pid), "{name}: the host did not end");
        assert_eq!(world.doc.text(), "ao", "{name}");
        host.shutdown();
    }
}

#[test]
fn a_powershell_or_perl_handler_that_never_ends_is_killed_at_the_limit() {
    for (language, name, code) in [
        (PS, "loop.ps1", "while ($true) { }\n"),
        (PERL, "loop.pl", "1 while 1;\n"),
    ] {
        if !available(language) {
            continue;
        }
        let dir = plugin_dir(&[(name, code)]);
        let mut host = host(dir.path(), language, true);
        let mut world = World::new("");
        // PowerShell takes a while to start; the limit starts with the call, so give it room.
        let limit = Duration::from_secs(8);
        let report = call_jack(&mut world, &mut host, "Command", name, &Value::Null, limit);
        assert_eq!(report.outcome, Outcome::TimedOut, "{name}");
        assert!(!host.is_running(), "{name}");
    }
}

#[test]
fn a_powershell_or_perl_plugin_serves_the_outline_complement_and_smart_indent_objects() {
    let ps = [
        (
            "outline.ps1",
            "$Outline.SetTitle('見出し')\n$Outline.AddFuncInfo2(3, 1, 'a', 0)\n",
        ),
        (
            "complement.ps1",
            "$w = $Complement.GetCurrentWord()\nWrite-Output $Complement.GetOption()\n[void]$Complement.AddList($w + '1')\nWrite-Output $Complement.AddList($w + '1')\n",
        ),
        (
            "indent.ps1",
            "Write-Output ([int][char]$Indent.GetChar())\n",
        ),
        ("noindent.ps1", "Write-Output \"[$($Indent.GetChar())]\"\n"),
    ];
    let perl = [
        (
            "outline.pl",
            "Outline->SetTitle('見出し');\nOutline->AddFuncInfo2(3, 1, 'a', 0);\n",
        ),
        (
            "complement.pl",
            "my $w = Complement->GetCurrentWord;\nprint Complement->GetOption, \"\\n\";\nComplement->AddList($w . '1');\nprint Complement->AddList($w . '1'), \"\\n\";\n",
        ),
        ("indent.pl", "print ord(Indent->GetChar), \"\\n\";\n"),
        ("noindent.pl", "print '[', Indent->GetChar, \"]\\n\";\n"),
    ];
    for (language, files, ext) in [(PS, ps, "ps1"), (PERL, perl, "pl")] {
        if !available(language) {
            continue;
        }
        let dir = plugin_dir(&files);
        let mut host = host(dir.path(), language, true);
        let mut world = World::new("");
        let outline = format!("outline.{ext}");
        let report = call_jack(
            &mut world,
            &mut host,
            "Outline",
            &outline,
            &json!({}),
            PATIENT,
        );
        assert_eq!(report.outcome, Outcome::Ok, "{ext}: {:?}", report.output);
        assert_eq!(
            world.api.outline,
            [
                "Outline.SetTitle[\"見出し\"]",
                "Outline.AddFuncInfo2[3,1,\"a\",0]"
            ],
            "{ext}"
        );
        let args = json!({"word": "あ", "option": 3});
        let complement = format!("complement.{ext}");
        let report = call_jack(
            &mut world,
            &mut host,
            "Complement",
            &complement,
            &args,
            PATIENT,
        );
        assert_eq!(stdout(&report), ["3", "-1"], "{ext}: {:?}", report.output);
        assert_eq!(world.api.added, ["あ1"], "{ext}");
        let args = json!({"char": "\r"});
        let indent = format!("indent.{ext}");
        let report = call_jack(
            &mut world,
            &mut host,
            "SmartIndent",
            &indent,
            &args,
            PATIENT,
        );
        assert_eq!(stdout(&report), ["13"], "{ext}: {:?}", report.output);
        let plain = format!("noindent.{ext}");
        let report = call_jack(
            &mut world,
            &mut host,
            "Command",
            &plain,
            &json!({}),
            PATIENT,
        );
        assert_eq!(stdout(&report), ["[]"], "{ext}: {:?}", report.output);
        host.shutdown();
    }
}

#[test]
fn the_powershell_and_perl_sample_plugins_run() {
    let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/plugins");
    let text = "# 見出し\n本文\n## 小見出し\n";
    for (language, folder, ext) in [(PS, "HelloPowerShell", "ps1"), (PERL, "HelloPerl", "pl")] {
        if !available(language) {
            continue;
        }
        let mut host = host(&samples.join(folder), language, true);
        let mut world = World::new(text);
        for _ in 0..2 {
            let opened = format!("opened.{ext}");
            let report = call_jack(
                &mut world,
                &mut host,
                "DocumentOpen",
                &opened,
                &json!({}),
                PATIENT,
            );
            assert_eq!(report.outcome, Outcome::Ok, "{folder}: {:?}", report.output);
        }
        let command = format!("command.{ext}");
        let report = call(&mut world, &mut host, &command);
        assert_eq!(report.outcome, Outcome::Ok, "{folder}: {:?}", report.output);
        assert!(world.doc.text().starts_with("Hello (Command)"), "{folder}");
        let outline = format!("outline.{ext}");
        let report = call_jack(
            &mut world,
            &mut host,
            "Outline",
            &outline,
            &json!({}),
            PATIENT,
        );
        assert_eq!(report.outcome, Outcome::Ok, "{folder}: {:?}", report.output);
        assert!(
            world
                .api
                .outline
                .iter()
                .any(|c| c.contains("\"小見出し\",1")),
            "{folder}: {:?}",
            world.api.outline
        );
        let args = json!({"word": "ab", "option": 0});
        let complement = format!("complement.{ext}");
        let report = call_jack(
            &mut world,
            &mut host,
            "Complement",
            &complement,
            &args,
            PATIENT,
        );
        assert_eq!(report.outcome, Outcome::Ok, "{folder}: {:?}", report.output);
        assert_eq!(world.api.added, ["ab_hello", "ab_world"], "{folder}");
        host.shutdown();
    }
}

#[test]
fn a_powershell_exit_runs_its_argument_and_the_output_streams_still_show() {
    if !available(PS) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "exit.ps1",
            "function Get-Code { $global:argument_ran = 'yes'; 3 }\nexit (Get-Code)\n",
        ),
        (
            "read.ps1",
            "Write-Host \"host $global:argument_ran\"\nWrite-Output 'out'\nWrite-Warning 'careful'\n",
        ),
    ]);
    let mut host = host(dir.path(), PS, true);
    let mut world = World::new("");
    assert_eq!(call(&mut world, &mut host, "exit.ps1").outcome, Outcome::Ok);
    let report = call(&mut world, &mut host, "read.ps1");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    let lines = stdout(&report);
    assert!(lines.contains(&"host yes"), "{:?}", report.output);
    assert!(lines.contains(&"out"), "{:?}", report.output);
    // PowerShell's own host prints the warning; it is not lost.
    assert!(
        report.output.iter().any(|(_, l)| l.contains("careful")),
        "{:?}",
        report.output
    );
    host.shutdown();
}

#[test]
fn a_powershell_handler_keeps_its_output_and_its_errors_that_did_not_stop_it() {
    if !available(PS) {
        return;
    }
    let dir = plugin_dir(&[
        ("before.ps1", "Write-Output 'before'\nthrow 'oops'\n"),
        (
            "continue.ps1",
            "Write-Output 'first'\nWrite-Error 'not fatal' -ErrorAction Continue\nWrite-Warning 'careful'\nWrite-Output 'last'\n",
        ),
        (
            "stop.ps1",
            "Write-Output 'seen'\nWrite-Error 'fatal' -ErrorAction Stop\n",
        ),
    ]);
    let mut host = host(dir.path(), PS, true);
    let mut world = World::new("");

    // Output written before a failure comes out, and the failure still has its kind and place.
    let report = call(&mut world, &mut host, "before.ps1");
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(
        text.contains("oops") && text.contains("before.ps1:2"),
        "{text}"
    );
    assert!(stdout(&report).contains(&"before"), "{:?}", report.output);

    // An error that does not stop the handler is shown, and the call succeeds.
    let report = call(&mut world, &mut host, "continue.ps1");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    let shown = |needle: &str| report.output.iter().any(|(_, l)| l.contains(needle));
    assert!(shown("not fatal"), "{:?}", report.output);
    assert!(shown("careful"), "{:?}", report.output);
    // The short form (kind, message, place); never the text of the handler.
    assert!(
        shown("WriteErrorException: not fatal"),
        "{:?}",
        report.output
    );
    assert!(shown("continue.ps1:2"), "{:?}", report.output);
    assert!(!shown("Write-Output 'first'"), "{:?}", report.output);
    assert!(stdout(&report).contains(&"first") && stdout(&report).contains(&"last"));

    // With -ErrorAction Stop the call fails with the kind and the place of the handler.
    let report = call(&mut world, &mut host, "stop.ps1");
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(
        text.contains("fatal") && text.contains("stop.ps1:2"),
        "{text}"
    );
    assert!(text.contains("Exception"), "{text}");
    assert!(stdout(&report).contains(&"seen"), "{:?}", report.output);
    host.shutdown();
}

#[test]
fn an_output_that_cannot_be_written_fails_the_call_and_the_host_still_answers() {
    if !available(PS) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "bad.ps1",
            "class Bad { [string] ToString() { throw 'bad format' } }\n[Bad]::new()\n",
        ),
        ("ok.ps1", "$Editor.InsText('ok')\n"),
    ]);
    let mut host = host(dir.path(), PS, true);
    let mut world = World::new("");
    let report = call(&mut world, &mut host, "bad.ps1");
    let Outcome::Failed(text) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(text.contains("bad format"), "{text}");
    // The host went on to answer, and serves the next call.
    assert_eq!(call(&mut world, &mut host, "ok.ps1").outcome, Outcome::Ok);
    assert_eq!(world.doc.text(), "ok");
    host.shutdown();
}
