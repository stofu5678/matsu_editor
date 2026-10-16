//! Generates the JavaScript, Python, Ruby and PHP parts of the macro API from `catalog::entries()` (`api::FUNCS` and every row of Sakura's macro tables), and
//! checks that the committed files (`boot/api.gen.cjs`, `boot/api_gen.py`, `boot/api_gen.rb`,
//! `boot/api_gen.php`) are up to date. Only the tests
//! use it: to regenerate, run `UME_UPDATE_GENERATED=1 cargo test -p ume-plugin generated_files`.

use crate::api::Ty;
use crate::catalog::{Entry, entries};

const GENERATED_NOTE: &str = "GENERATED from crates/ume-plugin/src/catalog.rs (api.rs FUNCS and Sakura's macro tables). Do not edit. \
Regenerate with: UME_UPDATE_GENERATED=1 cargo test -p ume-plugin generated_files";

/// The JavaScript part (`api.gen.cjs`): `module.exports(call)` returns the `Editor` object.
pub(super) fn generate_js() -> String {
    let mut out = String::new();
    out.push_str(&format!("// {GENERATED_NOTE}\n'use strict';\n\n"));
    out.push_str("module.exports = function install(call) {\n  const editor = {\n");
    for func in entries().iter().filter(|e| e.is_identifier()) {
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
        for line in wrap(&func.doc, 90) {
            out.push_str(&format!("     * {line}\n"));
        }
        out.push_str("     */\n");
        let names: Vec<&str> = func.params.iter().map(|p| p.name).collect();
        out.push_str(&format!(
            "    {}({}) {{\n      // A trailing undefined is a missing argument, not a null.\n      const args = Array.prototype.slice.call(arguments);\n      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();\n      return call('{}', args);\n    }},\n",
            func.name,
            names.join(", "),
            func.name
        ));
    }
    out.push_str("  };\n");
    out.push_str(JS_FALLBACK);
    out.push_str("};\n");
    out
}

/// What the macro API does for a name that the table above does not have: any other function of
/// Sakura's macro tables (`Left`, `Char`, ...) is a call by name. The editor answers a name that
/// it does not know with an error (a JavaScript exception).
const JS_FALLBACK: &str = r#"  // Any other name that starts like a Sakura macro name (an upper case letter or a digit) is
  // sent to the editor as it is. The editor answers a name it does not know with an error.
  return new Proxy(editor, {
    get(target, name, receiver) {
      if (typeof name !== 'string' || name in target || !/^[0-9A-Z]/.test(name)) {
        return Reflect.get(target, name, receiver);
      }
      return function () {
        const args = Array.prototype.slice.call(arguments);
        while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
        return call(name, args);
      };
    },
  });
"#;

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
    for func in entries().iter().filter(|e| e.is_identifier()) {
        let mut params = vec!["self".to_string()];
        let mut names = Vec::new();
        for p in &func.params {
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
        let doc_lines = wrap(&func.doc, 80);
        out.push_str(&doc_lines.join("\n            "));
        out.push_str("\"\"\"\n");
        out.push_str(&format!(
            "            return call(\"{}\", _pack({}))\n\n",
            func.name,
            names.join(", ")
        ));
    }
    out.push_str(PY_FALLBACK);
    out.push_str("    return _Editor()\n");
    out
}

/// The Python counterpart of [`JS_FALLBACK`], as a method of `_Editor`.
const PY_FALLBACK: &str = r#"    def _fallback(self, name):
        # Any other name that starts like a Sakura macro name (an upper case letter or a digit)
        # is sent to the editor as it is. The editor answers a name it does not know with an
        # error (EditorError).
        if not name[:1].isupper() and not name[:1].isdigit():
            raise AttributeError(name)

        def method(*args):
            return call(name, _pack(*args))

        return method

    _Editor.__getattr__ = _fallback

"#;

/// The Ruby part (`api_gen.rb`): `UmeEditorApi.install(call)` returns the `Editor` object.
pub(super) fn generate_rb() -> String {
    let mut out = String::new();
    out.push_str(&format!("# {GENERATED_NOTE}\n\n"));
    out.push_str("module UmeEditorApi\n");
    out.push_str("  UNSET = Object.new.freeze\n\n");
    out.push_str("  # The editor's macro API (the names and the arguments of Sakura's macros).\n");
    out.push_str("  class EditorObject\n");
    out.push_str("    attr_accessor :info\n\n");
    out.push_str("    def initialize(call)\n      @call = call\n      @info = nil\n    end\n\n");
    for func in entries().iter().filter(|e| e.is_identifier()) {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| {
                if p.optional {
                    format!("{} = UNSET", p.name)
                } else {
                    p.name.to_string()
                }
            })
            .collect();
        let names: Vec<&str> = func.params.iter().map(|p| p.name).collect();
        let signature = signature_text(func);
        out.push_str(&format!("    # {signature}\n    #\n"));
        for line in wrap(&func.doc, 90) {
            out.push_str(&format!("    # {line}\n"));
        }
        let def_params = if params.is_empty() {
            String::new()
        } else {
            format!("({})", params.join(", "))
        };
        out.push_str(&format!("    def {}{def_params}\n", func.name));
        out.push_str(&format!(
            "      @call.call('{}', _pack({}))\n    end\n\n",
            func.name,
            names.join(", ")
        ));
    }
    out.push_str(RB_FALLBACK);
    out.push_str("  end\n\n");
    out.push_str("  def self.install(call)\n    EditorObject.new(call)\n  end\nend\n");
    out
}

/// The Ruby counterpart of [`JS_FALLBACK`], and the helper that leaves out unset arguments.
const RB_FALLBACK: &str = r#"    # Any other name that starts like a Sakura macro name (an upper case letter or a digit) is
    # sent to the editor as it is. The editor answers a name it does not know with an error
    # (EditorError).
    def method_missing(name, *args)
      return super unless name.to_s.match?(/\A[0-9A-Z]/)

      @call.call(name.to_s, args)
    end

    def respond_to_missing?(name, include_private = false)
      name.to_s.match?(/\A[0-9A-Z]/) || super
    end

    private

    # A trailing UNSET is a missing argument, not a nil.
    def _pack(*args)
      args.pop while !args.empty? && args.last.equal?(UNSET)
      args
    end
"#;

/// The PHP part (`api_gen.php`): the class `Editor`, whose static methods are the API.
/// `Editor::install($call)` gives it the function that sends a request.
pub(super) fn generate_php() -> String {
    let mut out = String::new();
    out.push_str(&format!("<?php\n// {GENERATED_NOTE}\n\n"));
    out.push_str(
        "/**\n * The editor's macro API (the names and the arguments of Sakura's macros).\n */\n",
    );
    out.push_str("final class Editor\n{\n");
    out.push_str("    /** What the editor told the macro in `initialize`, or null. */\n");
    out.push_str("    public static $info = null;\n\n");
    out.push_str("    /** @var callable */\n    private static $call;\n\n");
    out.push_str("    /** Used by the boot library; a macro does not call this. */\n");
    out.push_str("    public static function install(callable $call): void\n    {\n        self::$call = $call;\n    }\n\n");
    for func in entries().iter().filter(|e| e.is_identifier()) {
        let params: Vec<String> = func
            .params
            .iter()
            .map(|p| {
                if p.optional {
                    format!("${} = null", p.name)
                } else {
                    format!("${}", p.name)
                }
            })
            .collect();
        out.push_str("    /**\n");
        out.push_str(&format!("     * {}\n     *\n", signature_text(func)));
        for line in wrap(&func.doc, 90) {
            out.push_str(&format!("     * {line}\n"));
        }
        out.push_str("     */\n");
        // No types in the signature: the editor checks the arguments and answers an error,
        // which is an EditorError (PHP would throw its own TypeError, or convert a string).
        out.push_str(&format!(
            "    public static function {}({})\n    {{\n",
            func.name,
            params.join(", ")
        ));
        out.push_str("        // func_get_args() holds only the arguments that were given.\n");
        out.push_str(&format!(
            "        return (self::$call)('{}', func_get_args());\n    }}\n\n",
            func.name
        ));
    }
    out.push_str(PHP_FALLBACK);
    out.push_str("}\n");
    out
}

/// The PHP counterpart of [`JS_FALLBACK`].
const PHP_FALLBACK: &str = r#"    /**
     * Any other name that starts like a Sakura macro name (an upper case letter or a digit) is
     * sent to the editor as it is. The editor answers a name it does not know with an error
     * (EditorError).
     */
    public static function __callStatic($name, $arguments)
    {
        if (!preg_match('/^[0-9A-Z]/', $name)) {
            throw new BadMethodCallException("Call to undefined method Editor::$name()");
        }
        return (self::$call)($name, $arguments);
    }
"#;

/// `Name(param: type, optional?: type) -> type`, for the comments of the generated code.
fn signature_text(func: &Entry) -> String {
    let params: Vec<String> = func
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
    format!(
        "{}({}) -> {}",
        func.name,
        params.join(", "),
        func.result.name()
    )
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
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/boot");
        let files = [
            ("api.gen.cjs", generate_js()),
            ("api_gen.py", generate_py()),
            ("api_gen.rb", generate_rb()),
            ("api_gen.php", generate_php()),
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
                "{name} is out of date; run: UME_UPDATE_GENERATED=1 cargo test -p ume-plugin generated_files"
            );
        }
    }
}
