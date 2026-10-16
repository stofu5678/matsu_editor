//! The basic macro API, defined in one place: the name, the parameters and the result of every
//! function, and the code that runs it on a document. The JavaScript and Python parts that the
//! macro calls are generated from `FUNCS` (see `codegen.rs`).
//!
//! The names and the arguments follow Sakura's macro API (sakura_core/macro/CSMacroMgr.cpp,
//! the help in help/macro/source/reference/), and the behaviour follows sakura_core/macro/
//! CMacro.cpp and the commands that it calls. The differences are listed in the "マクロ
//! (JSON-RPC)" section of poc/gpui-textview/README.md.

use serde_json::Value;

use crate::protocol::{
    INTERNAL_ERROR, METHOD_NOT_FOUND, Request, RpcError, parse_request, response_error, response_ok,
};

/// The type of a parameter or of a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    Int,
    Str,
    /// No value (JSON `null`: `None` in Python, `null` in JavaScript).
    Null,
}

impl Ty {
    pub fn name(self) -> &'static str {
        match self {
            Ty::Int => "int",
            Ty::Str => "string",
            Ty::Null => "null",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Param {
    pub name: &'static str,
    pub ty: Ty,
    /// The macro may leave it out (it must be the last ones).
    pub optional: bool,
}

pub struct Func {
    pub name: &'static str,
    pub params: &'static [Param],
    // Read by the code generator (codegen.rs, tests only).
    #[allow(dead_code)]
    pub result: Ty,
    #[allow(dead_code)]
    pub doc: &'static str,
    pub run: fn(&mut dyn MacroDocument, &Args) -> Result<Value, RpcError>,
}

const fn req(name: &'static str, ty: Ty) -> Param {
    Param {
        name,
        ty,
        optional: false,
    }
}

const fn opt(name: &'static str, ty: Ty) -> Param {
    Param {
        name,
        ty,
        optional: true,
    }
}

/// What the macro API needs of the editor, for the 14 functions of `FUNCS`.
///
/// The editor implements it for the length of one request, on top of the borrowed bundle that
/// a command gets (`CommandContext`, ADR-0013 section 4: the document, the cursor and
/// selection, the layout and the path). That is why the trait names no GPUI type, takes
/// `&mut self` for every change, and reads the text by line instead of handing out the whole
/// text. The tests use `TextModel`.
///
/// Positions are (line index from 0, byte offset in the line) in the lines of the text model,
/// which has one more line after a final line break.
pub trait MacroDocument {
    /// The number of lines in the text model (1 or more).
    fn line_count(&self) -> usize;
    /// The length of a line in bytes, without its line break.
    fn line_len(&self, row: usize) -> usize;
    /// A line without its line break.
    fn line_text(&self, row: usize) -> String;
    fn cursor(&self) -> (usize, usize);
    fn has_selection(&self) -> bool;
    /// The selected text, with "\n" for line breaks.
    fn selected_text(&self) -> String;
    /// Moves the cursor. Without `extend` the selection goes away; with it the selection starts
    /// at the old cursor position (or stays, if there is one) and ends at the new one.
    fn move_cursor(&mut self, pos: (usize, usize), extend: bool);
    /// Moves the cursor up (negative) or down by lines, and drops the selection.
    fn move_vertical(&mut self, delta: isize);
    fn select_all(&mut self);
    /// Inserts at the cursor, replacing the selection.
    fn insert(&mut self, text: &str);
    /// The full path of the file, if the text came from one.
    fn path(&self) -> Option<String>;
}

/// The arguments of a call, already checked against the parameters of the function.
pub struct Args<'a>(&'a [Value]);

impl Args<'_> {
    pub fn int(&self, index: usize) -> i64 {
        self.0[index].as_i64().unwrap_or_default()
    }

    pub fn int_or(&self, index: usize, default: i64) -> i64 {
        self.0
            .get(index)
            .map_or(default, |v| v.as_i64().unwrap_or(default))
    }

    pub fn str(&self, index: usize) -> &str {
        self.0[index].as_str().unwrap_or_default()
    }
}

pub const FUNCS: &[Func] = &[
    Func {
        name: "GetLineCount",
        params: &[req("reserved", Ty::Int)],
        result: Ty::Int,
        doc: "The number of logical lines (lines separated by line breaks). The text after the \
              last line break is not a line unless it is not empty. The argument is reserved \
              and must be 0.",
        run: get_line_count,
    },
    Func {
        name: "GetLineStr",
        params: &[req("line", Ty::Int)],
        result: Ty::Str,
        doc: "A logical line with its line break (\"\\n\"; the last line has none unless the \
              text ends with one). 0 is the line of the cursor, 1 is the first line. Past the \
              last line it is an empty string.",
        run: get_line_str,
    },
    Func {
        name: "InsText",
        params: &[req("text", Ty::Str)],
        result: Ty::Null,
        doc: "Inserts the text at the cursor, replacing the selection. The cursor ends after \
              the text.",
        run: ins_text,
    },
    Func {
        name: "AddTail",
        params: &[req("text", Ty::Str)],
        result: Ty::Null,
        doc: "Moves to the end of the file, drops the selection, and inserts the text there.",
        run: add_tail,
    },
    Func {
        name: "GoFileTop",
        params: &[],
        result: Ty::Null,
        doc: "Moves the cursor to the start of the file and drops the selection.",
        run: go_file_top,
    },
    Func {
        name: "GoFileEnd",
        params: &[],
        result: Ty::Null,
        doc: "Moves the cursor to the end of the file and drops the selection.",
        run: go_file_end,
    },
    Func {
        name: "Up",
        params: &[],
        result: Ty::Null,
        doc: "Moves the cursor up one line and drops the selection.",
        run: up,
    },
    Func {
        name: "Down",
        params: &[],
        result: Ty::Null,
        doc: "Moves the cursor down one line and drops the selection.",
        run: down,
    },
    Func {
        name: "SelectAll",
        params: &[],
        result: Ty::Null,
        doc: "Selects the whole text. The cursor goes to the end.",
        run: select_all,
    },
    Func {
        name: "GetSelectedString",
        params: &[req("reserved", Ty::Int)],
        result: Ty::Str,
        doc: "The selected text with its line breaks (\"\\n\"), or an empty string when nothing \
              is selected. The argument is reserved and ignored.",
        run: get_selected_string,
    },
    Func {
        name: "IsTextSelected",
        params: &[],
        result: Ty::Int,
        doc: "0 when nothing is selected, 1 when text is selected. (Sakura returns 2 for a box \
              selection, which is not supported yet.)",
        run: is_text_selected,
    },
    Func {
        name: "MoveCursor",
        params: &[
            req("line", Ty::Int),
            req("column", Ty::Int),
            req("option", Ty::Int),
        ],
        result: Ty::Null,
        doc: "Moves the cursor to a line and a column, both counted from 1. The column counts \
              UTF-16 code units. Option: 0 does not select, 1 selects from the old position. \
              (2, the box selection, is not supported.) Past the end of the text it goes to \
              the end of the text.",
        run: move_cursor,
    },
    Func {
        name: "Jump",
        params: &[req("line", Ty::Int), opt("flag", Ty::Int)],
        result: Ty::Null,
        doc: "Moves to the start of a line, counted from 1 in logical lines. Lines before 1 \
              and after the last one are limited to the first and the last line. Flag bit 0 \
              (1, the default) is the line break unit, which is the only unit supported yet; \
              bit 1 (PL/SQL) is not supported.",
        run: jump,
    },
    Func {
        name: "GetFilename",
        params: &[],
        result: Ty::Str,
        doc: "The full path of the file, or an empty string when the text has no file.",
        run: get_filename,
    },
];

/// The number of logical lines: a final empty line is only the position after the last line
/// break, and an empty text has none (CDocLineMgr::GetLineCount counts the lines that hold text
/// or a line break).
fn logical_line_count(doc: &dyn MacroDocument) -> usize {
    let n = doc.line_count();
    if n > 0 && doc.line_len(n - 1) == 0 {
        n - 1
    } else {
        n
    }
}

fn end_of_text(doc: &dyn MacroDocument) -> (usize, usize) {
    let last = doc.line_count() - 1;
    (last, doc.line_len(last))
}

/// CMacro::HandleFunction, F_GETLINECOUNT: an argument other than 0 is an error.
fn get_line_count(doc: &mut dyn MacroDocument, args: &Args) -> Result<Value, RpcError> {
    if args.int(0) != 0 {
        return Err(RpcError::invalid_params(
            "GetLineCount: the argument is reserved and must be 0",
        ));
    }
    Ok(logical_line_count(doc).into())
}

/// CMacro::HandleFunction, F_GETLINESTR: GetDocLineStrWithEOL, so the line break is included;
/// a negative number is an error and a number past the last line gives an empty string.
fn get_line_str(doc: &mut dyn MacroDocument, args: &Args) -> Result<Value, RpcError> {
    let n = args.int(0);
    if n < 0 {
        return Err(RpcError::invalid_params(
            "GetLineStr: the line number must be 0 or more",
        ));
    }
    let row = if n == 0 {
        doc.cursor().0
    } else {
        usize::try_from(n - 1).unwrap_or(usize::MAX)
    };
    if row >= logical_line_count(doc) {
        return Ok("".into());
    }
    let mut text = doc.line_text(row);
    if row + 1 < doc.line_count() {
        text.push('\n');
    }
    Ok(text.into())
}

fn ins_text(doc: &mut dyn MacroDocument, args: &Args) -> Result<Value, RpcError> {
    doc.insert(args.str(0));
    Ok(Value::Null)
}

/// CViewCommander::Command_ADDTAIL: GoFileEnd without selecting, then insert.
fn add_tail(doc: &mut dyn MacroDocument, args: &Args) -> Result<Value, RpcError> {
    let end = end_of_text(doc);
    doc.move_cursor(end, false);
    doc.insert(args.str(0));
    Ok(Value::Null)
}

fn go_file_top(doc: &mut dyn MacroDocument, _: &Args) -> Result<Value, RpcError> {
    doc.move_cursor((0, 0), false);
    Ok(Value::Null)
}

fn go_file_end(doc: &mut dyn MacroDocument, _: &Args) -> Result<Value, RpcError> {
    let end = end_of_text(doc);
    doc.move_cursor(end, false);
    Ok(Value::Null)
}

fn up(doc: &mut dyn MacroDocument, _: &Args) -> Result<Value, RpcError> {
    doc.move_vertical(-1);
    Ok(Value::Null)
}

fn down(doc: &mut dyn MacroDocument, _: &Args) -> Result<Value, RpcError> {
    doc.move_vertical(1);
    Ok(Value::Null)
}

fn select_all(doc: &mut dyn MacroDocument, _: &Args) -> Result<Value, RpcError> {
    doc.select_all();
    Ok(Value::Null)
}

fn get_selected_string(doc: &mut dyn MacroDocument, _: &Args) -> Result<Value, RpcError> {
    Ok(if doc.has_selection() {
        doc.selected_text().into()
    } else {
        "".into()
    })
}

fn is_text_selected(doc: &mut dyn MacroDocument, _: &Args) -> Result<Value, RpcError> {
    Ok(i64::from(doc.has_selection()).into())
}

/// CMacro::HandleCommand, F_MOVECURSOR, then Command_MOVECURSOR: the line and the column are
/// counted from 1; a number below 1 makes the editor beep and do nothing (no error). The column
/// is a logical position, a count of UTF-16 code units on Windows.
fn move_cursor(doc: &mut dyn MacroDocument, args: &Args) -> Result<Value, RpcError> {
    let (line, column, option) = (args.int(0), args.int(1), args.int(2));
    if option & 0x02 != 0 {
        return Err(RpcError::invalid_params(
            "MoveCursor: the box selection (option 2) is not supported yet",
        ));
    }
    if line < 1 || column < 1 {
        return Ok(Value::Null);
    }
    let row = usize::try_from(line - 1).unwrap_or(usize::MAX);
    let pos = if row >= logical_line_count(doc) {
        // CCaret::GetAdjustCursorPos: past the last line is the end of the file.
        end_of_text(doc)
    } else {
        let units = usize::try_from(column - 1).unwrap_or(usize::MAX);
        (row, byte_offset_for_utf16(&doc.line_text(row), units))
    };
    doc.move_cursor(pos, option & 0x01 != 0);
    Ok(Value::Null)
}

/// CViewCommander::Command_JUMP with the line break unit (flag bit 0): below 1 is the first
/// line, and past the last line is the last one. The default flag is 1 (CMacro.cpp, F_JUMP).
fn jump(doc: &mut dyn MacroDocument, args: &Args) -> Result<Value, RpcError> {
    let flag = args.int_or(1, 1);
    if flag & 0x02 != 0 {
        return Err(RpcError::invalid_params(
            "Jump: PL/SQL (flag bit 1) is not supported yet",
        ));
    }
    if flag & 0x01 == 0 {
        return Err(RpcError::invalid_params(
            "Jump: wrapped lines are not supported yet; use flag 1 (line break unit)",
        ));
    }
    let count = logical_line_count(doc);
    if count == 0 {
        // Command_JUMP beeps when there are no layout lines.
        return Ok(Value::Null);
    }
    let row = usize::try_from(args.int(0).max(1) - 1)
        .unwrap_or(usize::MAX)
        .min(count - 1);
    doc.move_cursor((row, 0), false);
    Ok(Value::Null)
}

fn get_filename(doc: &mut dyn MacroDocument, _: &Args) -> Result<Value, RpcError> {
    Ok(doc.path().unwrap_or_default().into())
}

/// The byte offset in `line` of the UTF-16 code unit `units` (counted from 0). A position in
/// the middle of a surrogate pair goes to the start of the character, and a position past the
/// end goes to the end.
pub fn byte_offset_for_utf16(line: &str, units: usize) -> usize {
    let mut seen = 0;
    for (offset, ch) in line.char_indices() {
        let width = ch.len_utf16();
        if seen + width > units {
            return offset;
        }
        seen += width;
    }
    line.len()
}

/// The error for a call whose arguments do not fit the function, or `None` when they do.
fn check_params(func: &Func, params: &[Value]) -> Result<(), RpcError> {
    let required = func.params.iter().filter(|p| !p.optional).count();
    if params.len() < required || params.len() > func.params.len() {
        let expected = if required == func.params.len() {
            required.to_string()
        } else {
            format!("{required} to {}", func.params.len())
        };
        return Err(RpcError::invalid_params(format!(
            "{}: expected {expected} argument(s), got {}",
            func.name,
            params.len()
        )));
    }
    for (param, value) in func.params.iter().zip(params) {
        let fits = match param.ty {
            // Only what fits an i64: a bigger number must not turn into 0 silently.
            Ty::Int => value.is_i64(),
            Ty::Str => value.is_string(),
            Ty::Null => value.is_null(),
        };
        if !fits {
            return Err(RpcError::invalid_params(format!(
                "{}: argument \"{}\" must be {}",
                func.name,
                param.name,
                param.ty.name()
            )));
        }
    }
    Ok(())
}

/// Runs one call: -32601 for an unknown method, -32602 for bad arguments.
pub fn call(
    doc: &mut dyn MacroDocument,
    method: &str,
    params: &[Value],
) -> Result<Value, RpcError> {
    let func = FUNCS
        .iter()
        .find(|f| f.name == method)
        .ok_or_else(|| RpcError::new(METHOD_NOT_FOUND, format!("unknown method: {method}")))?;
    check_params(func, params)?;
    (func.run)(doc, &Args(params))
}

/// Handles one message line from the macro and returns the response line, or `None` for a
/// notification.
pub fn handle_message(doc: &mut dyn MacroDocument, text: &str) -> Option<String> {
    respond(text, |method, params| call(doc, method, params))
}

/// Parses one message line, runs the call with `run` and builds the response line (`None` for a
/// notification). `handle_message` runs the call on a `MacroDocument`; the bridge
/// (`crate::bridge`) runs it on the commands.
pub fn respond(
    text: &str,
    run: impl FnOnce(&str, &[Value]) -> Result<Value, RpcError>,
) -> Option<String> {
    let request = match parse_request(text) {
        Ok(request) => request,
        Err(rejection) => return Some(response_error(&rejection.id, &rejection.error)),
    };
    let Request { id, method, params } = request;
    // A panic in a handler must not take the editor down or leave the macro waiting.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&method, &params)))
        .unwrap_or_else(|_| {
            Err(RpcError::new(
                INTERNAL_ERROR,
                "internal error in the editor",
            ))
        });
    let id = id?;
    Some(match outcome {
        Ok(result) => response_ok(&id, result),
        Err(error) => response_error(&id, &error),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::model::TextModel;

    fn run(doc: &mut TextModel, method: &str, params: Value) -> Result<Value, RpcError> {
        call(doc, method, params.as_array().unwrap())
    }

    fn model(text: &str) -> TextModel {
        TextModel::new(text, None)
    }

    #[test]
    fn get_line_count_counts_logical_lines() {
        assert_eq!(
            run(&mut model(""), "GetLineCount", json!([0])),
            Ok(json!(0))
        );
        assert_eq!(
            run(&mut model("a"), "GetLineCount", json!([0])),
            Ok(json!(1))
        );
        assert_eq!(
            run(&mut model("a\n"), "GetLineCount", json!([0])),
            Ok(json!(1))
        );
        assert_eq!(
            run(&mut model("a\nb"), "GetLineCount", json!([0])),
            Ok(json!(2))
        );
        assert_eq!(
            run(&mut model("a\n\n"), "GetLineCount", json!([0])),
            Ok(json!(2))
        );
        let err = run(&mut model("a"), "GetLineCount", json!([1])).unwrap_err();
        assert_eq!(err.code, crate::protocol::INVALID_PARAMS);
    }

    #[test]
    fn get_line_str_includes_the_line_break() {
        let mut doc = model("あ😀\nb\nlast");
        assert_eq!(run(&mut doc, "GetLineStr", json!([1])), Ok(json!("あ😀\n")));
        assert_eq!(run(&mut doc, "GetLineStr", json!([3])), Ok(json!("last")));
        // Past the last line is an empty string.
        assert_eq!(run(&mut doc, "GetLineStr", json!([4])), Ok(json!("")));
        assert_eq!(run(&mut doc, "GetLineStr", json!([1000])), Ok(json!("")));
        // 0 is the line of the cursor.
        doc.move_cursor((1, 0), false);
        assert_eq!(run(&mut doc, "GetLineStr", json!([0])), Ok(json!("b\n")));
        assert!(run(&mut doc, "GetLineStr", json!([-1])).is_err());
        // The empty line after a final line break is not a line.
        let mut doc = model("a\n");
        assert_eq!(run(&mut doc, "GetLineStr", json!([1])), Ok(json!("a\n")));
        assert_eq!(run(&mut doc, "GetLineStr", json!([2])), Ok(json!("")));
        doc.move_cursor((1, 0), false);
        assert_eq!(run(&mut doc, "GetLineStr", json!([0])), Ok(json!("")));
    }

    #[test]
    fn ins_text_and_add_tail() {
        let mut doc = model("abc");
        doc.move_cursor((0, 1), false);
        run(&mut doc, "InsText", json!(["あ\n😀"])).unwrap();
        assert_eq!(doc.text(), "aあ\n😀bc");
        assert_eq!(doc.cursor(), (1, "😀".len()));
        run(&mut doc, "AddTail", json!(["!"])).unwrap();
        assert_eq!(doc.text(), "aあ\n😀bc!");
        assert_eq!(doc.cursor(), (1, "😀bc!".len()));
        // InsText replaces the selection.
        run(&mut doc, "SelectAll", json!([])).unwrap();
        run(&mut doc, "InsText", json!(["x"])).unwrap();
        assert_eq!(doc.text(), "x");
    }

    #[test]
    fn moves_and_selection() {
        let mut doc = model("one\ntwo\nthree");
        run(&mut doc, "GoFileEnd", json!([])).unwrap();
        assert_eq!(doc.cursor(), (2, 5));
        run(&mut doc, "Up", json!([])).unwrap();
        assert_eq!(doc.cursor(), (1, 3));
        run(&mut doc, "Down", json!([])).unwrap();
        // The column that Up started from (5) is kept through the short line.
        assert_eq!(doc.cursor(), (2, 5));
        run(&mut doc, "GoFileTop", json!([])).unwrap();
        assert_eq!(doc.cursor(), (0, 0));
        assert_eq!(run(&mut doc, "IsTextSelected", json!([])), Ok(json!(0)));
        assert_eq!(
            run(&mut doc, "GetSelectedString", json!([0])),
            Ok(json!(""))
        );
        run(&mut doc, "SelectAll", json!([])).unwrap();
        assert_eq!(run(&mut doc, "IsTextSelected", json!([])), Ok(json!(1)));
        assert_eq!(
            run(&mut doc, "GetSelectedString", json!([0])),
            Ok(json!("one\ntwo\nthree"))
        );
        // Moving without selecting drops the selection.
        run(&mut doc, "GoFileTop", json!([])).unwrap();
        assert_eq!(run(&mut doc, "IsTextSelected", json!([])), Ok(json!(0)));
    }

    #[test]
    fn vertical_moves_keep_the_column_through_a_short_line() {
        let mut doc = model("abcdef\nx\nabcdef");
        run(&mut doc, "MoveCursor", json!([1, 6, 0])).unwrap();
        run(&mut doc, "Down", json!([])).unwrap();
        assert_eq!(doc.cursor(), (1, 1));
        run(&mut doc, "Down", json!([])).unwrap();
        assert_eq!(doc.cursor(), (2, 5));
        // Any other move forgets the column.
        run(&mut doc, "Up", json!([])).unwrap();
        run(&mut doc, "GoFileTop", json!([])).unwrap();
        run(&mut doc, "Down", json!([])).unwrap();
        assert_eq!(doc.cursor(), (1, 0));
    }

    #[test]
    fn move_cursor_counts_from_1_in_utf16_units() {
        let mut doc = model("a😀b\nxyz");
        // 'a' is 1 unit and the emoji is 2: column 4 is before 'b'.
        run(&mut doc, "MoveCursor", json!([1, 4, 0])).unwrap();
        assert_eq!(doc.cursor(), (0, "a😀".len()));
        // The middle of the surrogate pair goes to its start.
        run(&mut doc, "MoveCursor", json!([1, 3, 0])).unwrap();
        assert_eq!(doc.cursor(), (0, 1));
        // Past the end of the line goes to the end of the line.
        run(&mut doc, "MoveCursor", json!([2, 100, 0])).unwrap();
        assert_eq!(doc.cursor(), (1, 3));
        // Past the last line goes to the end of the text.
        run(&mut doc, "MoveCursor", json!([100, 1, 0])).unwrap();
        assert_eq!(doc.cursor(), (1, 3));
        // Below 1 does nothing.
        run(&mut doc, "MoveCursor", json!([0, 1, 0])).unwrap();
        run(&mut doc, "MoveCursor", json!([1, 0, 0])).unwrap();
        assert_eq!(doc.cursor(), (1, 3));
        // Option 1 selects from the old position.
        run(&mut doc, "MoveCursor", json!([1, 1, 1])).unwrap();
        assert_eq!(
            run(&mut doc, "GetSelectedString", json!([0])),
            Ok(json!("a😀b\nxyz"))
        );
        assert_eq!(run(&mut doc, "IsTextSelected", json!([])), Ok(json!(1)));
        // The box selection is not supported.
        assert!(run(&mut doc, "MoveCursor", json!([1, 1, 2])).is_err());
    }

    #[test]
    fn jump_goes_to_the_start_of_a_line() {
        let mut doc = model("a\nb\nc\n");
        run(&mut doc, "Jump", json!([2])).unwrap();
        assert_eq!(doc.cursor(), (1, 0));
        run(&mut doc, "Jump", json!([2, 1])).unwrap();
        run(&mut doc, "Jump", json!([99])).unwrap();
        assert_eq!(doc.cursor(), (2, 0));
        run(&mut doc, "Jump", json!([0])).unwrap();
        assert_eq!(doc.cursor(), (0, 0));
        run(&mut doc, "Jump", json!([-5, 1])).unwrap();
        assert_eq!(doc.cursor(), (0, 0));
        assert!(run(&mut doc, "Jump", json!([1, 2])).is_err());
        assert!(run(&mut doc, "Jump", json!([1, 0])).is_err());
    }

    #[test]
    fn get_filename_is_a_path_or_empty() {
        let mut doc = TextModel::new("", Some("C:\\work\\あ.txt".into()));
        assert_eq!(
            run(&mut doc, "GetFilename", json!([])),
            Ok(json!("C:\\work\\あ.txt"))
        );
        assert_eq!(run(&mut model(""), "GetFilename", json!([])), Ok(json!("")));
    }

    #[test]
    fn bad_calls_get_the_standard_errors() {
        use crate::protocol::{INVALID_PARAMS, METHOD_NOT_FOUND};
        let mut doc = model("a");
        assert_eq!(
            run(&mut doc, "NoSuchThing", json!([])).unwrap_err().code,
            METHOD_NOT_FOUND
        );
        // Too few, too many, and the wrong type.
        for (method, params) in [
            ("GetLineStr", json!([])),
            ("GetLineStr", json!([1, 2])),
            ("GetLineStr", json!(["1"])),
            ("GetLineStr", json!([1.5])),
            ("GetLineStr", json!([true])),
            ("GetLineStr", json!([18446744073709551615u64])),
            ("InsText", json!([5])),
            ("InsText", json!([null])),
            ("GoFileTop", json!([1])),
            ("MoveCursor", json!([1, 1])),
            ("Jump", json!([1, 1, 1])),
        ] {
            let err = run(&mut doc, method, params.clone()).unwrap_err();
            assert_eq!(err.code, INVALID_PARAMS, "{method} {params}");
        }
        assert_eq!(doc.text(), "a");
    }

    #[test]
    fn handle_message_answers_with_ascii() {
        let mut doc = model("日本語😀");
        let line = handle_message(
            &mut doc,
            r#"{"jsonrpc":"2.0","id":3,"method":"GetLineStr","params":[1]}"#,
        )
        .unwrap();
        assert!(line.is_ascii());
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["id"], 3);
        assert_eq!(value["result"], "日本語😀");
    }

    #[test]
    fn handle_message_reports_errors_and_skips_notifications() {
        let mut doc = model("");
        let line = handle_message(&mut doc, "not json").unwrap();
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["error"]["code"], -32700);
        assert_eq!(value["id"], Value::Null);
        let line =
            handle_message(&mut doc, r#"{"jsonrpc":"2.0","id":"x","method":"Nope"}"#).unwrap();
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["error"]["code"], -32601);
        assert_eq!(value["id"], "x");
        assert_eq!(
            handle_message(&mut doc, r#"{"jsonrpc":"2.0","method":"GoFileTop"}"#),
            None
        );
    }

    #[test]
    fn utf16_offsets() {
        assert_eq!(byte_offset_for_utf16("a😀b", 0), 0);
        assert_eq!(byte_offset_for_utf16("a😀b", 1), 1);
        assert_eq!(byte_offset_for_utf16("a😀b", 2), 1);
        assert_eq!(byte_offset_for_utf16("a😀b", 3), 5);
        assert_eq!(byte_offset_for_utf16("a😀b", 4), 6);
        assert_eq!(byte_offset_for_utf16("a😀b", 99), 6);
    }
}
