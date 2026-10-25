//! AppleScript macros (`.applescript`, macOS only; ADR-0025).
//!
//! `osascript` runs one script. The editor builds it from the boot prefix (`boot/boot.applescript`:
//! the handlers that talk to the editor through the standard input and output), the user's script
//! inside `script UserMacro ... end script`, and a suffix with `on run`. The program is written to
//! a file in the private boot folder.
//!
//! `osascript` reports an error with the range of characters in the program
//! (`file:START:END: execution error: ...`). [`Program::translate_error`] turns that into the name of
//! the user's script and the line in it.

/// The handlers and the beginning of `script UserMacro`.
const PREFIX: &str = include_str!("boot/boot.applescript");

/// What follows the user's script: the end of the script object, and the `on run` that reads
/// `initialize`, runs the macro and answers with [`END_MARK`] (osascript prints the value that
/// `run` returns; the session drops that line). A script object with no statements of its own
/// runs the `on run` of the script around it, which would start again and wait for `initialize`
/// for good; `editorStarted` ends that.
const SUFFIX: &str = "\nend script\n\non run\n\tif editorStarted then return\n\tset my editorStarted to true\n\tmy editorStart()\n\tset ignoredResult to run UserMacro\n\treturn \"matsu-end-7f3a91c2-applescript-result\"\nend run\n";

/// The line that `osascript` prints for the result of the program. It is not output of the macro.
/// (A long token, so that a macro does not print it by chance.)
pub const END_MARK: &str = "matsu-end-7f3a91c2-applescript-result";

/// The program and where the user's script is in it.
#[derive(Debug, Clone)]
pub struct Program {
    /// The text of the whole program.
    pub text: String,
    /// The number of lines before the first line of the user's script.
    pub lines_before_user: usize,
    /// How many lines the user's script has.
    user_lines: usize,
    /// What the errors call the script (its path).
    pub name: String,
}

impl Program {
    /// The program for the user's script `source`, named `name` in the errors.
    pub fn build(source: &str, name: &str) -> Self {
        // Old Mac files end their lines with CR; the program has LF alone, so the lines are counted
        // the same way by everyone.
        let source = source.replace("\r\n", "\n").replace('\r', "\n");
        let source = source.as_str();
        // A checkout on Windows may have CR LF in the prefix.
        let mut text = PREFIX.replace("\r\n", "\n");
        let lines_before_user = text.matches('\n').count();
        text.push_str(source);
        if !source.ends_with('\n') {
            text.push('\n');
        }
        let user_lines = source.lines().count().max(1);
        text.push_str(SUFFIX);
        Self {
            text,
            lines_before_user,
            user_lines,
            name: name.to_owned(),
        }
    }

    /// A line of the standard error of `osascript`, with the character range of the program in
    /// it turned into `name:LINE` when it is in the user's script (and into `name` when it is in
    /// the boot, whose lines mean nothing to the user). Any other line is returned as it is.
    pub fn translate_error(&self, line: &str) -> String {
        let Some((start, rest)) = split_range(line) else {
            return line.to_owned();
        };
        let Some(program_line) = self.line_at(start) else {
            return line.to_owned();
        };
        let first_user_line = self.lines_before_user + 1;
        if program_line >= first_user_line && program_line < first_user_line + self.user_lines {
            format!(
                "{}:{}{rest}",
                self.name,
                program_line - self.lines_before_user
            )
        } else {
            format!("{}{rest}", self.name)
        }
    }

    /// The line (from 1) of the program where the character offset `offset` is. Offsets count
    /// UTF-16 code units, as Cocoa strings do.
    fn line_at(&self, offset: usize) -> Option<usize> {
        let mut units = 0;
        let mut line = 1;
        for c in self.text.chars() {
            if units >= offset {
                return Some(line);
            }
            units += c.len_utf16();
            if c == '\n' {
                line += 1;
            }
        }
        (units >= offset).then_some(line)
    }
}

/// `path:START:END: rest` as (START, ": rest"), for the messages of `osascript`. (Some versions
/// write the range as `START-END`; both are read.)
fn split_range(line: &str) -> Option<(usize, &str)> {
    // The path may hold colons; the range is the last two numbers before the kind of error.
    // A syntax error of the compiler is a "script error" (-2741), a failure at run time an
    // "execution error".
    let kind = [": execution error", ": script error", ": syntax error"]
        .iter()
        .find_map(|kind| line.find(kind))?;
    let (before, last) = line[..kind].rsplit_once(':')?;
    let number = |text: &str| -> Option<usize> {
        text.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| text.parse().ok())
            .flatten()
    };
    if let Some((start, end)) = last.split_once('-') {
        number(end)?;
        return Some((number(start)?, &line[kind..]));
    }
    number(last)?;
    let (_, start) = before.rsplit_once(':')?;
    Some((number(start)?, &line[kind..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_program_holds_the_user_script_in_a_script_object() {
        let program = Program::build("set a to 1\nset b to 2", "/m/a.applescript");
        assert!(
            program
                .text
                .contains("script UserMacro\nset a to 1\nset b to 2\n")
        );
        assert!(program.text.ends_with("end run\n"));
        assert!(program.text.contains("\n\tmy editorStart()\n"));
        // The prefix ends with the line `script UserMacro`, and the script starts on the next one.
        let line = program.text.lines().nth(program.lines_before_user).unwrap();
        assert_eq!(line, "set a to 1");
    }

    #[test]
    fn line_breaks_are_made_alike() {
        let program = Program::build("set a to 1\rset b to 2\r\nset c to 3", "n");
        assert!(
            program
                .text
                .contains("set a to 1\nset b to 2\nset c to 3\n")
        );
        assert!(!program.text.contains('\r'));
        assert_eq!(program.user_lines, 3);
    }

    #[test]
    fn the_boot_is_ascii() {
        assert!(PREFIX.is_ascii());
    }

    #[test]
    fn an_error_range_becomes_the_line_of_the_user_script() {
        let program = Program::build(
            "set a to 1\nset b to 2\nerror \"boom\"\n",
            "/m/a.applescript",
        );
        // The offset of `error` in line 3 of the user's script.
        let offset: usize = program
            .text
            .lines()
            .take(program.lines_before_user + 2)
            .map(|l| l.encode_utf16().count() + 1)
            .sum::<usize>()
            + 2;
        let line = format!(
            "/tmp/x/macro.applescript:{offset}:{}: execution error: boom (-2700)",
            offset + 5
        );
        assert_eq!(
            program.translate_error(&line),
            "/m/a.applescript:3: execution error: boom (-2700)"
        );
    }

    #[test]
    fn an_error_in_the_boot_has_no_line() {
        let program = Program::build("set a to 1\n", "/m/a.applescript");
        let line =
            "/tmp/x/macro.applescript:100:110: execution error: EditorError -32602: x (-32602)";
        assert_eq!(
            program.translate_error(line),
            "/m/a.applescript: execution error: EditorError -32602: x (-32602)"
        );
    }

    #[test]
    fn offsets_count_utf16_units() {
        // An emoji is two units; the error after it in the user's script is still on line 2.
        let program = Program::build("set a to \"😀\"\nerror \"x\"\n", "n");
        let before: usize = program
            .text
            .lines()
            .take(program.lines_before_user + 1)
            .map(|l| l.encode_utf16().count() + 1)
            .sum();
        let line = format!("f:{before}:{}: execution error: x (-2700)", before + 3);
        assert_eq!(
            program.translate_error(&line),
            "n:2: execution error: x (-2700)"
        );
    }

    #[test]
    fn the_dash_form_is_read_too() {
        let program = Program::build("error \"x\"\n", "n");
        let offset: usize = program
            .text
            .lines()
            .take(program.lines_before_user)
            .map(|l| l.encode_utf16().count() + 1)
            .sum();
        let line = format!("/a:b/f:{offset}-{}: execution error: x (-2700)", offset + 3);
        assert_eq!(
            program.translate_error(&line),
            "n:1: execution error: x (-2700)"
        );
    }

    #[test]
    fn a_syntax_error_is_a_script_error() {
        let program = Program::build("set a to 1\nset b to to 2\n", "n");
        let offset: usize = program
            .text
            .lines()
            .take(program.lines_before_user + 1)
            .map(|l| l.encode_utf16().count() + 1)
            .sum::<usize>()
            + 9;
        let line = format!(
            "/t/m.applescript:{offset}:{}: script error: Expected expression but found \u{201c}to\u{201d}. (-2741)",
            offset + 2
        );
        assert_eq!(
            program.translate_error(&line),
            "n:2: script error: Expected expression but found \u{201c}to\u{201d}. (-2741)"
        );
    }

    #[test]
    fn other_lines_stay() {
        let program = Program::build("", "n");
        assert_eq!(program.translate_error("hello"), "hello");
        assert_eq!(
            program.translate_error("a:b: execution error: x"),
            "a:b: execution error: x"
        );
    }
}
