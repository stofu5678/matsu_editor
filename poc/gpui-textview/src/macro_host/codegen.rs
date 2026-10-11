//! Generates the JavaScript and Python parts of the macro API from `api::FUNCS`, and checks that
//! the committed files (`boot/api.gen.cjs`, `boot/api_gen.py`) are up to date. Only the tests
//! use it: to regenerate, run `UME_UPDATE_GENERATED=1 cargo test generated_files`.

use super::api::{FUNCS, Ty};

const GENERATED_NOTE: &str = "GENERATED from src/macro_host/api.rs (FUNCS). Do not edit. \
Regenerate with: UME_UPDATE_GENERATED=1 cargo test generated_files";

/// The JavaScript part (`api.gen.cjs`): `module.exports(call)` returns the `Editor` object.
pub(super) fn generate_js() -> String {
    let mut out = String::new();
    out.push_str(&format!("// {GENERATED_NOTE}\n'use strict';\n\n"));
    out.push_str("module.exports = function install(call) {\n  return {\n");
    for func in FUNCS {
        let signature: Vec<String> = func
            .params
            .iter()
            .map(|p| {
                format!(
                    "{}: {}{}",
                    p.name,
                    p.ty.name(),
                    if p.optional { "?" } else { "" }
                )
            })
            .collect();
        out.push_str("    /**\n");
        out.push_str(&format!(
            "     * {}({}) -> {}\n     *\n",
            func.name,
            signature.join(", "),
            func.result.name()
        ));
        for line in wrap(func.doc, 90) {
            out.push_str(&format!("     * {line}\n"));
        }
        out.push_str("     */\n");
        let names: Vec<&str> = func.params.iter().map(|p| p.name).collect();
        out.push_str(&format!(
            "    {}({}) {{\n      return call('{}', Array.prototype.slice.call(arguments));\n    }},\n",
            func.name,
            names.join(", "),
            func.name
        ));
    }
    out.push_str("  };\n};\n");
    out
}

/// The Python part (`api_gen.py`): `install(call)` returns the `Editor` object.
pub(super) fn generate_py() -> String {
    let mut out = String::new();
    out.push_str(&format!("# {GENERATED_NOTE}\n\n"));
    out.push_str("_UNSET = object()\n\n\n");
    out.push_str("def _pack(*args):\n");
    out.push_str("    args = list(args)\n");
    out.push_str("    while args and args[-1] is _UNSET:\n");
    out.push_str("        args.pop()\n");
    out.push_str("    return args\n\n\n");
    out.push_str("def install(call):\n");
    out.push_str("    class _Editor:\n");
    out.push_str("        \"\"\"The editor's macro API (the names and the arguments of Sakura's macros).\"\"\"\n\n");
    for func in FUNCS {
        let mut params = vec!["self".to_string()];
        let mut names = Vec::new();
        for p in func.params {
            let py_ty = match p.ty {
                Ty::Int => "int",
                Ty::Str => "str",
                Ty::Null => "None",
            };
            if p.optional {
                params.push(format!("{}: {py_ty} = _UNSET", p.name));
            } else {
                params.push(format!("{}: {py_ty}", p.name));
            }
            names.push(p.name);
        }
        let result = match func.result {
            Ty::Int => "int",
            Ty::Str => "str",
            Ty::Null => "None",
        };
        out.push_str(&format!(
            "        def {}({}) -> {result}:\n",
            func.name,
            params.join(", ")
        ));
        out.push_str("            \"\"\"");
        let doc_lines = wrap(func.doc, 80);
        out.push_str(&doc_lines.join("\n            "));
        out.push_str("\"\"\"\n");
        out.push_str(&format!(
            "            return call(\"{}\", _pack({}))\n\n",
            func.name,
            names.join(", ")
        ));
    }
    out.push_str("    return _Editor()\n");
    out
}

/// Breaks a text into lines of at most `width` characters at spaces.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The generated files are committed. Run with `UME_UPDATE_GENERATED=1` to rewrite them.
    #[test]
    fn generated_files_are_up_to_date() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/macro_host/boot");
        let files = [
            ("api.gen.cjs", generate_js()),
            ("api_gen.py", generate_py()),
        ];
        let update = std::env::var_os("UME_UPDATE_GENERATED").is_some();
        for (name, expected) in files {
            let path = dir.join(name);
            if update {
                std::fs::write(&path, &expected).unwrap();
                continue;
            }
            // Git on Windows may check the file out with CR LF.
            let actual = std::fs::read_to_string(&path)
                .unwrap_or_default()
                .replace("\r\n", "\n");
            assert!(
                actual == expected,
                "{name} is out of date; run: UME_UPDATE_GENERATED=1 cargo test generated_files"
            );
        }
    }
}
