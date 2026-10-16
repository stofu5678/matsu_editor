//! End-to-end tests of the bundled engines (Lua, QuickJS) against the editor's commands: the
//! built `ume-script` program runs as a macro process under the editor's own session code, and
//! every call goes through the bridge (`ume_plugin::bridge`) to the command registry, as it
//! does in the editor. Names that are not in the basic API table (`WordRight_Sel`, `Delete`,
//! `DuplicateLine`, ...) must be callable, and an unknown or unimplemented one is an error.

use std::time::Duration;

use ume_commands::context::CommandContext;
use ume_commands::layout::PlainLayout;
use ume_commands::registry::Registry;
use ume_core::buffer::{Document, Pos};
use ume_core::cursor::Cursor;
use ume_core::model::DocumentModel;
use ume_plugin::bridge;
use ume_plugin::session::{self, EndReason, MacroConfig, Report, Stream};

struct Fixture {
    model: DocumentModel,
    cursor: Cursor,
    layout: PlainLayout,
}

impl Fixture {
    fn new(text: &str) -> Self {
        Self {
            model: DocumentModel::new(Document::from_utf8(text.as_bytes().to_vec()).unwrap()),
            cursor: Cursor::default(),
            layout: PlainLayout::default(),
        }
    }

    fn text(&self) -> String {
        let doc = self.model.doc();
        doc.text_between(Pos::new(0, 0), doc.end_pos().unwrap())
            .unwrap()
    }
}

fn run(file: &str, code: &str, text: &str) -> (Report, Fixture) {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join(file);
    std::fs::write(&script, code).unwrap();
    let mut config = MacroConfig::for_script(&script, None).unwrap();
    config.interpreter = env!("CARGO_BIN_EXE_ume-script").into();
    config.timeout = Duration::from_secs(30);
    let mut fixture = Fixture::new(text);
    let registry = Registry::global();
    let report = {
        let mut cx = CommandContext::new(
            &mut fixture.model,
            &mut fixture.cursor,
            &mut fixture.layout,
            None,
        );
        session::run_blocking(config, |m| bridge::handle_message(registry, &mut cx, m))
            .expect("the host starts")
    };
    (report, fixture)
}

fn assert_clean_exit(report: &Report) {
    assert_eq!(
        (report.finished.reason, report.finished.exit_code),
        (EndReason::Exited, Some(0)),
        "stderr: {}",
        report.lines(Stream::Stderr).join("\n")
    );
}

const LUA: &str = r#"
Editor.GoFileTop()
Editor.WordRight_Sel()
Editor.Delete()
Editor.GoLineEnd(0)
Editor.Char(33)
Editor.Down()
Editor.DuplicateLine()
print((Editor.GetLineStr(1):gsub("\n", "<LF>")), Editor.GetLineCount(0))

-- A name with a command that this editor does not have yet is an error with a code.
local ok, e = pcall(Editor.FileSave)
print(ok, e.code)
-- A name that Sakura does not have: the table has no such key.
print(Editor.NoSuchMacro)
-- A string for a number, and too many arguments.
print(select(2, pcall(Editor.Char, "a")).code, select(2, pcall(Editor.Down, 1)).code)
-- A name that starts with a digit is a table key.
print(select(2, pcall(Editor["1PageUp"])).code)
"#;

const QUICKJS: &str = r#"
Editor.GoFileTop();
Editor.WordRight_Sel();
Editor.Delete();
Editor.GoLineEnd(0);
Editor.Char(33);
Editor.Down();
Editor.DuplicateLine();
console.log(JSON.stringify(Editor.GetLineStr(1)), Editor.GetLineCount(0));

try { Editor.FileSave(); } catch (e) { console.log(e.code); }
console.log(typeof Editor.NoSuchMacro);
try { Editor.Char('a'); } catch (e) { console.log(e.code); }
try { Editor.Down(1); } catch (e) { console.log(e.code); }
try { Editor["1PageUp"](); } catch (e) { console.log(e.code); }
"#;

#[test]
fn lua_calls_every_command_by_its_sakura_name() {
    let (report, fixture) = run("macro.lua", LUA, "one two\nthree\n");
    assert_clean_exit(&report);
    assert_eq!(fixture.text(), "two!\nthree\nthree\n");
    assert_eq!(
        report.lines(Stream::Stdout),
        [
            "two!<LF>\t3",
            "false\t-32001",
            "nil",
            "-32602\t-32602",
            "-32001"
        ]
    );
}

#[test]
fn quickjs_calls_every_command_by_its_sakura_name() {
    let (report, fixture) = run("macro.js", QUICKJS, "one two\nthree\n");
    assert_clean_exit(&report);
    assert_eq!(fixture.text(), "two!\nthree\nthree\n");
    assert_eq!(
        report.lines(Stream::Stdout),
        [
            "\"two!\\n\" 3",
            "-32001",
            "undefined",
            "-32602",
            "-32602",
            "-32001"
        ]
    );
}

#[test]
fn both_engines_have_every_name_of_the_catalog() {
    // A macro that lists the names it can call: every entry of the catalog is there.
    let names: Vec<&str> = ume_plugin::catalog::entries()
        .iter()
        .map(|e| e.name)
        .collect();
    assert!(names.len() > 300, "{}", names.len());
    let lua = format!(
        "local missing = 0\nfor _, n in ipairs({{{}}}) do if Editor[n] == nil then missing = missing + 1 end end\nprint(missing)",
        names
            .iter()
            .map(|n| format!("'{n}'"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let (report, _) = run("names.lua", &lua, "");
    assert_clean_exit(&report);
    assert_eq!(report.lines(Stream::Stdout), ["0"]);
    let js = format!(
        "const names = [{}]; console.log(names.filter((n) => typeof Editor[n] !== 'function').length);",
        names
            .iter()
            .map(|n| format!("'{n}'"))
            .collect::<Vec<_>>()
            .join(",")
    );
    let (report, _) = run("names.js", &js, "");
    assert_clean_exit(&report);
    assert_eq!(report.lines(Stream::Stdout), ["0"]);
}

#[test]
fn mruby_calls_every_command_by_its_sakura_name() {
    let code = r#"
Editor.GoFileTop
Editor.WordRight_Sel
Editor.Delete
Editor.GoLineEnd(0)
Editor.Char(33)
Editor.Down
Editor.DuplicateLine
puts Editor.GetLineStr(1).inspect, Editor.GetLineCount(0)
begin; Editor.FileSave; rescue EditorError => e; puts e.code; end
begin; Editor.Char('a'); rescue EditorError => e; puts e.code; end
"#;
    let (report, fixture) = run("macro.mruby", code, "one two\nthree\n");
    assert_clean_exit(&report);
    assert_eq!(fixture.text(), "two!\nthree\nthree\n");
    assert_eq!(
        report.lines(Stream::Stdout),
        ["\"two!\\n\"", "3", "-32001", "-32602"]
    );
}
