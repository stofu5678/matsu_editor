//! Reads the macro function tables of `sakura_core/macro/CSMacroMgr.cpp` and writes them as Rust.
//!
//! `m_MacroFuncInfoCommandArr` lists the macro names of commands, `m_MacroFuncInfoArr` the
//! functions that return a value, and `CanFuncIsKeyMacro` the functions that a key macro
//! records. Like the other parsers, this one is a small scan that fails loudly.

use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;

use crate::parse::{Tables, read_text, strip_comments};
use crate::ume_extras;

type Result<T> = std::result::Result<T, String>;

/// Where the table is, relative to the Sakura clone.
pub const SOURCE: &str = "sakura_core/macro/CSMacroMgr.cpp";

/// The generated file, relative to the repository root.
pub const OUTPUT: &str = "crates/ume-commands/src/sakura/macro_functions.rs";

/// The copyright notices in the header of `CSMacroMgr.cpp` and the license of the Sakura
/// repository (`LICENSE`).
const NOTICE: &str = "\
Copyright (C) 1998-2001, Norio Nakatani
Copyright (C) 2000, jepro
Copyright (C) 2001, genta, aroka, MIK, asa-o, hor, Misaka, jepro, Stonee
Copyright (C) 2002, YAZAKI, MIK, aroka, hor, genta, ai
Copyright (C) 2003, MIK, genta, Moca
Copyright (C) 2004, genta, zenryaku
Copyright (C) 2005, MIK, genta, maru, FILE
Copyright (C) 2006, かろと, fon, ryoji
Copyright (C) 2007, ryoji, maru
Copyright (C) 2008, nasukoji, ryoji
Copyright (C) 2011, nasukoji
Copyright (C) 2018-2026, Sakura Editor Organization

zlib License

This software is provided 'as-is', without any express or implied
warranty.  In no event will the authors be held liable for any damages
arising from the use of this software.

Permission is granted to anyone to use this software for any purpose,
including commercial applications, and to alter it and redistribute it
freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not
   claim that you wrote the original software. If you use this software
   in a product, an acknowledgment in the product documentation would be
   appreciated but is not required.
2. Altered source versions must be plainly marked as such, and must not be
   misrepresented as being the original software.
3. This notice may not be removed or altered from any source distribution.";

/// The types the generated file defines (they belong to the data, so they are written with it).
const TYPES: &str = "\
/// The type of an argument or of a result (`VT_I4` and `VT_BSTR` of the table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgType {
    Int,
    Str,
}

/// One row of Sakura's macro tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MacroFunction {
    /// The name a macro calls, for example `Down`.
    pub name: &'static str,
    /// Sakura's name of the function, for example `F_DOWN` (a name in `FUNCTIONS`).
    pub function: &'static str,
    /// Sakura's function code.
    pub code: u16,
    /// The types of the arguments. A macro may leave out the last ones.
    pub args: &'static [ArgType],
    /// The type of the result, or `None` for a command.
    pub result: Option<ArgType>,
    /// Whether a key macro records this function (`CSMacroMgr::CanFuncIsKeyMacro`).
    pub key_macro: bool,
}
";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Vt {
    Empty,
    Int,
    Str,
}

struct Row {
    function: String,
    name: String,
    args: Vec<Vt>,
    result: Vt,
}

struct MacroTables {
    commands: Vec<Row>,
    functions: Vec<Row>,
    key_macro: Vec<String>,
}

/// Returns `(path relative to the repository root, file content)`.
pub fn render(sakura: &Path, tables: &Tables, commit: &str) -> Result<(String, String)> {
    let text = strip_comments(&read_text(&sakura.join(SOURCE))?);
    let parsed = parse(&text)?;
    Ok((OUTPUT.to_string(), write(&parsed, tables, commit)?))
}

fn parse(text: &str) -> Result<MacroTables> {
    let extras = parse_extras(text)?;
    let mut parsed = MacroTables {
        commands: parse_array(text, "m_MacroFuncInfoCommandArr", &extras)?,
        functions: parse_array(text, "m_MacroFuncInfoArr", &extras)?,
        key_macro: parse_key_macro(text)?,
    };
    add_ume_commands(&mut parsed)?;
    Ok(parsed)
}

/// UME-only commands (`ume_extras`): a command without arguments, recorded by key macros, placed
/// after the row of the function it follows in the function table.
fn add_ume_commands(parsed: &mut MacroTables) -> Result<()> {
    for extra in ume_extras::EXTRAS {
        let position = parsed
            .commands
            .iter()
            .position(|r| r.function == extra.after)
            .ok_or_else(|| format!("no macro row for {}", extra.after))?;
        if parsed.commands.iter().any(|r| r.name == extra.macro_name) {
            return Err(format!("macro name {} is already used", extra.macro_name));
        }
        parsed.commands.insert(
            position + 1,
            Row {
                function: extra.name.to_string(),
                name: extra.macro_name.to_string(),
                args: Vec::new(),
                result: Vt::Empty,
            },
        );
        parsed.key_macro.push(extra.name.to_string());
    }
    Ok(())
}

fn vt(token: &str) -> Result<Vt> {
    match token.trim() {
        "VT_EMPTY" => Ok(Vt::Empty),
        "VT_I4" => Ok(Vt::Int),
        "VT_BSTR" => Ok(Vt::Str),
        other => Err(format!("{SOURCE}: unknown type {other:?}")),
    }
}

fn brace_list(text: &str) -> Result<Vec<&str>> {
    let inner = text
        .trim()
        .strip_prefix('{')
        .and_then(|t| t.strip_suffix('}'))
        .ok_or_else(|| format!("{SOURCE}: expected a {{...}} list, got {text:?}"))?;
    Ok(inner
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect())
}

/// The extra arguments (`MacroFuncInfoEx`) by the name that a row refers to them with
/// (`&s_MacroInfoEx_i`): the types of the arguments after the fourth.
fn parse_extras(text: &str) -> Result<HashMap<String, Vec<Vt>>> {
    let mut arrays: HashMap<String, Vec<Vt>> = HashMap::new();
    let mut extras = HashMap::new();
    let mut disabled = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("#if 0") {
            disabled = true;
        } else if line.starts_with("#endif") {
            disabled = false;
        }
        if disabled {
            continue;
        }
        if let Some(rest) = line.strip_prefix("VARTYPE s_MacroArgEx_") {
            let (name, list) = rest
                .split_once("[] =")
                .ok_or_else(|| format!("{SOURCE}: cannot read {line:?}"))?;
            let types = brace_list(list.trim().trim_end_matches(';'))?
                .into_iter()
                .map(vt)
                .collect::<Result<Vec<_>>>()?;
            arrays.insert(format!("s_MacroArgEx_{name}"), types);
        } else if let Some(rest) = line.strip_prefix("MacroFuncInfoEx s_MacroInfoEx_") {
            let (name, body) = rest
                .split_once(" = ")
                .ok_or_else(|| format!("{SOURCE}: cannot read {line:?}"))?;
            let fields = brace_list(body.trim().trim_end_matches(';'))?;
            let [min, max, array] = fields[..] else {
                return Err(format!("{SOURCE}: cannot read {line:?}"));
            };
            let array = arrays
                .get(array)
                .ok_or_else(|| format!("{SOURCE}: unknown argument array in {line:?}"))?;
            let (min, max): (usize, usize) = (
                min.parse().map_err(|_| format!("{SOURCE}: {line:?}"))?,
                max.parse().map_err(|_| format!("{SOURCE}: {line:?}"))?,
            );
            if min != max || max != 4 + array.len() {
                return Err(format!(
                    "{SOURCE}: {line:?}: the argument counts do not fit the array"
                ));
            }
            extras.insert(format!("&s_MacroInfoEx_{name}"), array.clone());
        }
    }
    Ok(extras)
}

fn parse_array(text: &str, name: &str, extras: &HashMap<String, Vec<Vt>>) -> Result<Vec<Row>> {
    let marker = format!("CSMacroMgr::{name}[]");
    let start = text
        .find(&marker)
        .ok_or_else(|| format!("{SOURCE}: {marker} not found"))?;
    let body = &text[start..];
    let open = body
        .find('{')
        .ok_or_else(|| format!("{SOURCE}: no body for {marker}"))?;
    let mut rows = Vec::new();
    let (mut depth, mut row_start, mut closed) = (0usize, 0usize, false);
    for (i, c) in body[open..].char_indices() {
        match c {
            '{' => {
                depth += 1;
                if depth == 2 {
                    row_start = open + i + 1;
                }
            }
            '}' => {
                depth -= 1;
                if depth == 1 {
                    rows.push(&body[row_start..open + i]);
                } else if depth == 0 {
                    closed = true;
                    break;
                }
            }
            _ => {}
        }
    }
    if !closed {
        return Err(format!("{SOURCE}: {marker} is not closed"));
    }
    let mut out = Vec::new();
    for row in rows {
        out.extend(parse_row(row, extras)?);
    }
    Ok(out)
}

/// Splits at the commas that are not inside braces or a string.
fn split_top_level(row: &str) -> Vec<&str> {
    let (mut parts, mut depth, mut in_string, mut from) = (Vec::new(), 0, false, 0);
    for (i, c) in row.char_indices() {
        match c {
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => depth -= 1,
            ',' if !in_string && depth == 0 => {
                parts.push(row[from..i].trim());
                from = i + 1;
            }
            _ => {}
        }
    }
    parts.push(row[from..].trim());
    parts
}

/// One row, or `None` for the terminator (`F_INVALID`, no name).
fn parse_row(row: &str, extras: &HashMap<String, Vec<Vt>>) -> Result<Option<Row>> {
    let fields = split_top_level(row);
    let [function, name, args, result, data] = fields[..] else {
        return Err(format!("{SOURCE}: cannot read the row {row:?}"));
    };
    if name == "nullptr" {
        return Ok(None);
    }
    let name = name
        .strip_prefix("L\"")
        .and_then(|n| n.strip_suffix('"'))
        .ok_or_else(|| format!("{SOURCE}: cannot read the name in {row:?}"))?;
    let mut types = brace_list(args)?
        .into_iter()
        .map(vt)
        .collect::<Result<Vec<_>>>()?;
    if types.len() != 4 {
        return Err(format!("{SOURCE}: {name}: expected 4 argument types"));
    }
    // The arguments end at the first VT_EMPTY, and nothing may follow it.
    let used = types.iter().position(|&t| t == Vt::Empty).unwrap_or(4);
    if types[used..].iter().any(|&t| t != Vt::Empty) {
        return Err(format!(
            "{SOURCE}: {name}: an argument follows an empty one"
        ));
    }
    types.truncate(used);
    if data != "nullptr" {
        let extra = extras
            .get(data)
            .ok_or_else(|| format!("{SOURCE}: {name}: unknown extra arguments {data:?}"))?;
        if used != 4 {
            return Err(format!("{SOURCE}: {name}: extra arguments after {used}"));
        }
        types.extend(extra);
    }
    Ok(Some(Row {
        function: function.to_string(),
        name: name.to_string(),
        args: types,
        result: vt(result)?,
    }))
}

/// The functions that `CSMacroMgr::CanFuncIsKeyMacro` lets through (comments already removed).
fn parse_key_macro(text: &str) -> Result<Vec<String>> {
    let start = text
        .find("CSMacroMgr::CanFuncIsKeyMacro")
        .ok_or_else(|| format!("{SOURCE}: CanFuncIsKeyMacro not found"))?;
    let body = &text[start..];
    let end = body
        .find("return TRUE;")
        .ok_or_else(|| format!("{SOURCE}: CanFuncIsKeyMacro has no `return TRUE;`"))?;
    let names: Vec<String> = body[..end]
        .lines()
        .filter_map(|line| line.trim().strip_prefix("case "))
        .map(|rest| {
            rest.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .next()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    if names.is_empty() {
        return Err(format!("{SOURCE}: CanFuncIsKeyMacro lists nothing"));
    }
    Ok(names)
}

fn write(parsed: &MacroTables, tables: &Tables, commit: &str) -> Result<String> {
    let code_of = |name: &str| -> Result<u16> {
        tables
            .functions
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.code)
            .ok_or_else(|| format!("{SOURCE}: {name} is not a function of Funccode_x.hsrc"))
    };
    for name in &parsed.key_macro {
        code_of(name)?;
    }
    let mut out = String::new();
    out.push_str(
        "// @generated by `cargo xtask sakura-tables --sakura <path to a Sakura clone>`.\n",
    );
    out.push_str(
        "// Do not edit by hand; change the generator (xtask/src) and run it again.\n//\n",
    );
    let _ = writeln!(out, "// Source: Sakura Editor, commit {commit}");
    out.push_str("//   https://github.com/sakura-editor/sakura\n");
    let _ = writeln!(out, "//   {SOURCE}");
    out.push_str(
        "//\n// This file is derived from the file above and was modified: the data was converted\n",
    );
    out.push_str(
        "// to Rust tables, and the function names were resolved to function codes. UME Editor's own\n\
         // commands (xtask/src/ume_extras.rs) were added to the commands.\n//\n",
    );
    for line in NOTICE.lines() {
        if line.is_empty() {
            out.push_str("//\n");
        } else {
            let _ = writeln!(out, "// {line}");
        }
    }
    out.push('\n');
    out.push_str("use ArgType::{Int, Str};\n\n");
    out.push_str(TYPES);
    out.push('\n');
    for (title, constant, rows) in [
        (
            "`m_MacroFuncInfoCommandArr`: the macro names of commands, in table order. Several\n/// names may share one function code (old spellings); the first is the one that a saved\n/// key macro uses.",
            "MACRO_COMMANDS",
            &parsed.commands,
        ),
        (
            "`m_MacroFuncInfoArr`: the macro functions that return a value, in table order.",
            "MACRO_FUNCTIONS",
            &parsed.functions,
        ),
    ] {
        let _ = writeln!(out, "/// {title}");
        out.push_str("#[rustfmt::skip]\n");
        let _ = writeln!(out, "pub static {constant}: &[MacroFunction] = &[");
        for row in rows {
            let args: Vec<&str> = row
                .args
                .iter()
                .map(|t| if *t == Vt::Int { "Int" } else { "Str" })
                .collect();
            let result = match row.result {
                Vt::Empty => "None",
                Vt::Int => "Some(Int)",
                Vt::Str => "Some(Str)",
            };
            let key_macro = parsed.key_macro.contains(&row.function);
            let _ = writeln!(
                out,
                "    MacroFunction {{ name: {:?}, function: {:?}, code: {}, args: &[{}], result: {result}, key_macro: {key_macro} }},",
                row.name,
                row.function,
                code_of(&row.function)?,
                args.join(", "),
            );
        }
        out.push_str("];\n\n");
    }
    out.truncate(out.trim_end().len());
    out.push('\n');
    Ok(out)
}
