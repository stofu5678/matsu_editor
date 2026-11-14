//! Ruby and PHP plugins (ADR-0026, PR 10 continued). The same plugin objects as in Node.js and
//! Python, in each language's own spelling: `Plugin.GetOption` in Ruby, `Plugin::GetOption` in PHP.
//! Each test skips when the interpreter is missing (`MATSU_REQUIRE_INTERPRETERS` makes that a failure
//! on the Linux x64 runner, which has both).

use super::*;

const RUBY: Language = Language::Ruby;
const PHP: Language = Language::Php;

#[test]
fn a_ruby_or_php_plugin_runs_with_the_plugin_object_and_keeps_its_state() {
    // `@n` of the file (Ruby) and the global `$n` (PHP) stay for the next call.
    let ruby = "
@n = (@n || 0) + 1
puts \"#{Plugin.jack} #{Plugin.args['n']} #{Plugin.id} #{@n}\"
puts \"#{Plugin.GetPluginDir} #{Plugin.GetDef('Plugin', 'Name')}\"
Plugin.SetOption('Main', 'Level', '7')
puts \"#{Plugin.GetOption('Main', 'Level')} #{Plugin.GetString(2)}\"
Editor.InsText('名前')
";
    let php = "<?php
$n = ($n ?? 0) + 1;
echo Plugin::$jack, ' ', Plugin::$args['n'], ' ', Plugin::$id, ' ', $n, \"\\n\";
echo Plugin::GetPluginDir() . ' ' . Plugin::GetDef('Plugin', 'Name') . \"\\n\";
Plugin::SetOption('Main', 'Level', '7');
echo Plugin::GetOption('Main', 'Level') . ' ' . Plugin::GetString(2) . \"\\n\";
Editor::InsText('名前');
";
    for (language, name, code) in [(RUBY, "h.rb", ruby), (PHP, "h.php", php)] {
        if !available(language) {
            continue;
        }
        let dir = plugin_dir(&[(name, code)]);
        let mut host = host(dir.path(), language, true);
        let mut world = World::new("");
        let first = world.call(&mut host, name);
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
        let second = world.call(&mut host, name);
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
fn a_ruby_file_keeps_its_own_state_and_methods_and_no_other_file_sees_them() {
    if !available(RUBY) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "a.rb",
            "@x = (@x || 0) + 1\ndef hi\n  'hi'\nend\nEditor.InsText(hi + @x.to_s)\n",
        ),
        ("b.rb", "puts @x.inspect\n"),
    ]);
    let mut host = host(dir.path(), RUBY, true);
    let mut world = World::new("");
    for _ in 0..2 {
        // A method that the file defines again is not an error.
        assert_eq!(world.call(&mut host, "a.rb").outcome, Outcome::Ok);
    }
    let report = world.call(&mut host, "b.rb");
    assert_eq!(stdout(&report), ["nil"], "{:?}", report.output);
    assert_eq!(world.doc.text(), "hi1hi2");
    host.shutdown();
}

#[test]
fn a_ruby_or_php_handler_may_load_the_files_of_its_folder_again_and_again() {
    let files = [
        (
            "req.rb",
            "require 'plugin_lib/word'\nEditor.InsText(Word.text)\n",
        ),
        (
            "plugin_lib/word.rb",
            "module Word\n  def self.text\n    'こと'\n  end\nend\n",
        ),
        (
            "req.php",
            "<?php\nrequire_once 'plugin_lib/word.php';\nEditor::InsText(plugin_word());\n",
        ),
        (
            "plugin_lib/word.php",
            "<?php\nfunction plugin_word() {\n    return 'こと';\n}\n",
        ),
    ];
    for (language, name) in [(RUBY, "req.rb"), (PHP, "req.php")] {
        if !available(language) {
            continue;
        }
        let dir = plugin_dir(&files);
        let mut host = host(dir.path(), language, true);
        let mut world = World::new("");
        // The second call must not fail on a function that is declared already.
        for _ in 0..2 {
            let report = world.call(&mut host, name);
            assert_eq!(report.outcome, Outcome::Ok, "{name}: {:?}", report.output);
        }
        assert_eq!(world.doc.text(), "ことこと", "{name}");
        host.shutdown();
    }
}

#[test]
fn a_ruby_or_php_handler_that_raises_fails_with_its_place_and_the_host_stays() {
    let cases = [
        (
            RUBY,
            "bad.rb",
            "a = 1\nraise ArgumentError, 'だめ'\n",
            "bad.rb:2",
        ),
        (RUBY, "syntax.rb", "def (:\n", "SyntaxError"),
        (RUBY, "editor.rb", "Editor.GetLineStr('x')\n", "EditorError"),
        (
            PHP,
            "bad.php",
            "<?php\n$a = 1;\nthrow new RuntimeException('だめ');\n",
            "bad.php:3",
        ),
        (PHP, "syntax.php", "<?php\n$a = ;\n", "ParseError"),
        (
            PHP,
            "editor.php",
            "<?php\nEditor::GetLineStr('x');\n",
            "EditorError",
        ),
    ];
    for (language, name, code, needle) in cases {
        if !available(language) {
            continue;
        }
        let (ok, ok_code) = if language == RUBY {
            ("ok.rb", "Editor.InsText('ok')")
        } else {
            ("ok.php", "<?php Editor::InsText('ok');")
        };
        let dir = plugin_dir(&[(name, code), (ok, ok_code)]);
        let mut host = host(dir.path(), language, true);
        let mut world = World::new("");
        let report = world.call(&mut host, name);
        let Outcome::Failed(text) = &report.outcome else {
            panic!("{name}: {:?}", report.outcome);
        };
        assert!(text.contains(needle), "{name}: {text}");
        // The process stays and serves the next call.
        assert_eq!(world.call(&mut host, ok).outcome, Outcome::Ok, "{name}");
        assert_eq!(world.doc.text(), "ok", "{name}");
        host.shutdown();
    }
}

#[test]
fn a_ruby_handler_cannot_overwrite_the_hosts_variables() {
    if !available(RUBY) {
        return;
    }
    // The names are those of the locals of the host's loop.
    let dir = plugin_dir(&[
        (
            "clobber.rb",
            "message = 'hello'\nplugin = 1\nspaces = 2\nparams = 3\nname = 4\nfailure = 5\nEditor.InsText('c')\n",
        ),
        ("ok.rb", "Editor.InsText('o')\n"),
    ]);
    let mut host = host(dir.path(), RUBY, true);
    let mut world = World::new("");
    for name in ["clobber.rb", "ok.rb", "clobber.rb"] {
        let report = world.call(&mut host, name);
        assert_eq!(report.outcome, Outcome::Ok, "{name}: {:?}", report.output);
    }
    assert_eq!(world.doc.text(), "coc");
    host.shutdown();
}

#[test]
fn a_ruby_handler_may_define_methods_named_like_the_hosts_helpers() {
    if !available(RUBY) {
        return;
    }
    // `source`, `name` and `space` are not locals of the host's method, so these are the handler's own.
    let dir = plugin_dir(&[(
        "names.rb",
        "def source; 'snippet'; end\ndef name; 'n'; end\ndef space; 's'; end\nputs source\nputs name\nputs space\nsource ||= 'x'\nputs source\n",
    )]);
    let mut host = host(dir.path(), RUBY, true);
    let mut world = World::new("");
    let report = world.call(&mut host, "names.rb");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert_eq!(stdout(&report), ["snippet", "n", "s", "x"]);
    host.shutdown();
}

#[test]
fn a_ruby_return_ends_only_the_handler() {
    if !available(RUBY) {
        return;
    }
    let dir = plugin_dir(&[
        (
            "early.rb",
            "Editor.InsText('a')\nreturn\nEditor.InsText('b')\n",
        ),
        ("ok.rb", "Editor.InsText('o')\n"),
    ]);
    let mut host = host(dir.path(), RUBY, true);
    let mut world = World::new("");
    let pid = {
        assert_eq!(world.call(&mut host, "early.rb").outcome, Outcome::Ok);
        host.process_id().expect("it stays")
    };
    assert_eq!(world.call(&mut host, "ok.rb").outcome, Outcome::Ok);
    assert_eq!(host.process_id(), Some(pid), "the host did not end");
    assert_eq!(world.doc.text(), "ao");
    host.shutdown();
}

#[test]
fn a_ruby_exit_ends_the_handler_and_is_not_a_failure() {
    if !available(RUBY) {
        return;
    }
    let dir = plugin_dir(&[(
        "exit.rb",
        "Editor.InsText('a')\nexit 3\nEditor.InsText('b')\n",
    )]);
    let mut host = host(dir.path(), RUBY, true);
    let mut world = World::new("");
    assert_eq!(world.call(&mut host, "exit.rb").outcome, Outcome::Ok);
    assert_eq!(world.doc.text(), "a");
    host.shutdown();
}

#[test]
fn a_ruby_or_php_handler_that_never_ends_is_killed_at_the_limit() {
    for (language, name, code) in [
        (RUBY, "loop.rb", "loop {}\n"),
        (PHP, "loop.php", "<?php while (true) {}\n"),
    ] {
        if !available(language) {
            continue;
        }
        let dir = plugin_dir(&[(name, code)]);
        let mut host = host(dir.path(), language, true);
        let mut world = World::new("");
        let limit = Duration::from_secs(2);
        let report = world.call_jack(&mut host, "Command", name, &Value::Null, limit);
        assert_eq!(report.outcome, Outcome::TimedOut, "{name}");
        assert!(!host.is_running(), "{name}");
    }
}

#[test]
fn a_ruby_or_php_plugin_serves_the_outline_complement_and_smart_indent_objects() {
    let ruby = [
        (
            "outline.rb",
            "Outline.SetTitle('見出し')\nOutline.AddFuncInfo2(3, 1, 'a', 0)\n",
        ),
        (
            "complement.rb",
            "w = Complement.GetCurrentWord\nputs Complement.GetOption\nComplement.AddList(w + '1')\nputs Complement.AddList(w + '1')\n",
        ),
        ("indent.rb", "puts Indent.GetChar.ord\n"),
        ("noindent.rb", "puts Object.const_defined?(:Indent)\n"),
    ];
    let php = [
        (
            "outline.php",
            "<?php\nOutline::SetTitle('見出し');\nOutline::AddFuncInfo2(3, 1, 'a', 0);\n",
        ),
        (
            "complement.php",
            "<?php\n$w = Complement::GetCurrentWord();\necho Complement::GetOption(), \"\\n\";\nComplement::AddList($w . '1');\necho Complement::AddList($w . '1'), \"\\n\";\n",
        ),
        (
            "indent.php",
            "<?php\necho ord(Indent::GetChar()), \"\\n\";\n",
        ),
        (
            "noindent.php",
            "<?php\necho var_export(Indent::GetChar(), true), \"\\n\";\n",
        ),
    ];
    for (language, files, ext, noindent) in [(RUBY, ruby, "rb", "false"), (PHP, php, "php", "''")] {
        if !available(language) {
            continue;
        }
        let dir = plugin_dir(&files);
        let mut host = host(dir.path(), language, true);
        let mut world = World::new("");
        let outline = format!("outline.{ext}");
        let report = world.call_jack(&mut host, "Outline", &outline, &json!({}), PATIENT);
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
        let report = world.call_jack(&mut host, "Complement", &complement, &args, PATIENT);
        assert_eq!(stdout(&report), ["3", "-1"], "{ext}: {:?}", report.output);
        assert_eq!(world.api.added, ["あ1"], "{ext}");
        let args = json!({"char": "\r"});
        let indent = format!("indent.{ext}");
        let report = world.call_jack(&mut host, "SmartIndent", &indent, &args, PATIENT);
        assert_eq!(stdout(&report), ["13"], "{ext}: {:?}", report.output);
        let plain = format!("noindent.{ext}");
        let report = world.call_jack(&mut host, "Command", &plain, &json!({}), PATIENT);
        assert_eq!(stdout(&report), [noindent], "{ext}: {:?}", report.output);
        host.shutdown();
    }
}

#[test]
fn a_php_handler_may_start_with_a_byte_order_mark_or_outside_php() {
    if !available(PHP) {
        return;
    }
    let dir = plugin_dir(&[
        ("bom.php", "\u{feff}<?php\necho 'bom';\n"),
        ("html.php", "text <?php echo 'x'; ?>\n"),
        (
            "strict.php",
            "<?php\ndeclare(strict_types=1);\necho 'strict';\n",
        ),
        ("noeol.php", "<?php echo 'abc';"),
    ]);
    let mut host = host(dir.path(), PHP, true);
    let mut world = World::new("");
    for (name, expected) in [
        ("bom.php", "bom"),
        ("html.php", "text x"),
        ("strict.php", "strict"),
        // No line break at the end: the answer must still start on its own line.
        ("noeol.php", "abc"),
    ] {
        let report = world.call(&mut host, name);
        assert_eq!(report.outcome, Outcome::Ok, "{name}: {:?}", report.output);
        assert_eq!(stdout(&report), [expected], "{name}");
    }
    host.shutdown();
}

#[test]
fn the_ruby_and_php_sample_plugins_run() {
    let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/plugins");
    let text = "# 見出し\n本文\n## 小見出し\n";
    for (language, folder, ext) in [(RUBY, "HelloRuby", "rb"), (PHP, "HelloPhp", "php")] {
        if !available(language) {
            continue;
        }
        let mut host = host(&samples.join(folder), language, true);
        let mut world = World::new(text);
        for _ in 0..2 {
            let opened = format!("opened.{ext}");
            let report = world.call_jack(&mut host, "DocumentOpen", &opened, &json!({}), PATIENT);
            assert_eq!(report.outcome, Outcome::Ok, "{folder}: {:?}", report.output);
        }
        let command = format!("command.{ext}");
        let report = world.call(&mut host, &command);
        assert_eq!(report.outcome, Outcome::Ok, "{folder}: {:?}", report.output);
        assert!(world.doc.text().starts_with("Hello (Command)"), "{folder}");
        let outline = format!("outline.{ext}");
        let report = world.call_jack(&mut host, "Outline", &outline, &json!({}), PATIENT);
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
        let report = world.call_jack(&mut host, "Complement", &complement, &args, PATIENT);
        assert_eq!(report.outcome, Outcome::Ok, "{folder}: {:?}", report.output);
        assert_eq!(world.api.added, ["ab_hello", "ab_world"], "{folder}");
        host.shutdown();
    }
}
