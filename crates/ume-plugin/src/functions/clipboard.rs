//! The clipboard: its text, and the "by format" functions for the text formats.
//!
//! The clipboard of this editor is the text of the platform clipboard (GPUI) plus Sakura's two
//! marks, a copy of a row and a copy of a box ([`ClipKind`]). The platform clipboard of GPUI
//! gives and takes text only, so the functions that name a format work for the text formats of
//! Windows (`CF_TEXT`, `CF_OEMTEXT`, `CF_UNICODETEXT`, or their numbers 1, 7 and 13) and say so
//! for the others. `CClipboard` (`sakura_core/_os/CClipboard.cpp`) is the Sakura source.

use serde_json::Value;
use ume_commands::clipboard::ClipKind;
use ume_commands::context::CommandContext;
use ume_core::model::eol_text;

use super::{EditorFunc, command};
use crate::api::{Args, Ty, opt, req};
use crate::bridge::NOT_IMPLEMENTED;
use crate::host::Editor;
use crate::protocol::RpcError;

type Result<T = Value> = std::result::Result<T, RpcError>;

/// The bits of the option of `SetClipboard` (CMacro.cpp, F_SETCLIPBOARD): a box copy and a row
/// copy.
const OPT_BOX: i64 = 0x01;
const OPT_LINE: i64 = 0x02;

/// `GetClipboardByFormat`'s mode for "the usual way of Sakura" (`CClipboard::GetClipboardByFormat`:
/// "-2: 通常のサクラの処理"), and the code of UTF-16 (`CODE_UNICODE`) that `CF_UNICODETEXT` has.
const MODE_NORMAL: i64 = -2;
const CODE_UNICODE: i64 = 3;

/// The text of the clipboard, or an empty string (`CEditView::MyGetClipboardData`).
fn clipboard_text(cx: &mut CommandContext) -> Result<String> {
    Ok(cx
        .clipboard_get()
        .map_err(command)?
        .map(|data| data.text)
        .unwrap_or_default())
}

/// F_GETCLIPBOARD: the text of the clipboard as it is (the line endings of the source), or an
/// empty string when there is no text. The option is read and ignored, as in Sakura.
fn get_clipboard(ed: &mut Editor, _: &Args) -> Result {
    Ok(clipboard_text(ed.cx)?.into())
}

/// F_SETCLIPBOARD: puts a string on the clipboard. Option bit 0 marks it as a box copy and bit 1
/// as a row copy (`MySetClipboardData`); both marks cannot be on one clip here, so the box wins.
/// The result is 1 when it worked.
fn set_clipboard(ed: &mut Editor, args: &Args) -> Result {
    let option = args.int_or(0, 0);
    let text = if args.len() > 1 {
        args.str(1).to_owned()
    } else {
        String::new()
    };
    let kind = if option & OPT_BOX != 0 {
        ClipKind::Box
    } else if option & OPT_LINE != 0 {
        ClipKind::Line
    } else {
        ClipKind::Normal
    };
    let done = ed.cx.clipboard_set(text, kind).map_err(command)?;
    Ok(i64::from(done).into())
}

/// `GetClipFormat` of `CClipboard.cpp`, for the formats that are text: the names are not case
/// sensitive and a number is the format number.
fn is_text_format(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    matches!(
        name.as_str(),
        "CF_TEXT" | "CF_OEMTEXT" | "CF_UNICODETEXT" | "1" | "7" | "13"
    )
}

/// F_ISINCLUDECLIPBOARDFORMAT: 1 when the clipboard has the format. A text format is there when
/// the clipboard has text; every other format is not (this editor sees only text).
fn is_include_clipboard_format(ed: &mut Editor, args: &Args) -> Result {
    let name = args.str(0);
    let has = is_text_format(name) && ed.cx.clipboard_get().map_err(command)?.is_some();
    Ok(i64::from(has).into())
}

fn text_only(function: &str, format: &str, mode: i64) -> Result<()> {
    if !is_text_format(format) {
        return Err(RpcError::new(
            NOT_IMPLEMENTED,
            format!(
                "{function}: the format \"{format}\" is not supported: the clipboard of this \
                 editor holds text (CF_TEXT, CF_OEMTEXT, CF_UNICODETEXT)"
            ),
        ));
    }
    if mode != MODE_NORMAL && mode != CODE_UNICODE {
        return Err(RpcError::new(
            NOT_IMPLEMENTED,
            format!(
                "{function}: the mode {mode} (binary or another encoding) is not supported: \
                 use -2, or 3 (UTF-16) for CF_UNICODETEXT"
            ),
        ));
    }
    Ok(())
}

/// F_GETCLIPBOARDBYFORMAT: the text of the clipboard in a text format. The line endings become
/// the one of the document (`GetText(..., cEol)`). The mode -2 (the usual) and 3 (UTF-16, the
/// text itself) work; the other modes and the other formats are errors. Without text: an empty
/// string.
fn get_clipboard_by_format(ed: &mut Editor, args: &Args) -> Result {
    text_only("GetClipboardByFormat", args.str(0), args.int(1))?;
    let eol = eol_text(ed.cx.new_line());
    let text = clipboard_text(ed.cx)?;
    // CR LF, CR and LF are line endings (the extended ones are not, as in Sakura's default).
    let unified = text.replace("\r\n", "\n").replace('\r', "\n");
    Ok(unified.replace('\n', eol).into())
}

/// F_SETCLIPBOARDBYFORMAT: puts the string on the clipboard as text, in a text format. Same
/// modes and formats as `GetClipboardByFormat`. The result is 1 when it worked.
fn set_clipboard_by_format(ed: &mut Editor, args: &Args) -> Result {
    text_only("SetClipboardByFormat", args.str(1), args.int(2))?;
    let done = ed
        .cx
        .clipboard_set(args.str(0).to_owned(), ClipKind::Normal)
        .map_err(command)?;
    Ok(i64::from(done).into())
}

pub const FUNCS: &[EditorFunc] = &[
    EditorFunc {
        name: "GetClipboard",
        params: &[opt("option", Ty::Int)],
        result: Ty::Str,
        doc: "The text on the clipboard, or an empty string when there is none. The line \
              endings are those of the source. The option is reserved.",
        run: get_clipboard,
    },
    EditorFunc {
        name: "SetClipboard",
        params: &[opt("option", Ty::Int), opt("text", Ty::Str)],
        result: Ty::Int,
        doc: "Puts the text on the clipboard. Option bit 0 marks a box copy (a paste puts it in \
              as a box), bit 1 a row copy (a paste puts it in front of the row); with both, the \
              box wins. Returns 1 when it worked, 0 when not.",
        run: set_clipboard,
    },
    EditorFunc {
        name: "IsIncludeClipboardFormat",
        params: &[req("format", Ty::Str)],
        result: Ty::Int,
        doc: "1 when the clipboard has the format. Only text is known: CF_TEXT, CF_OEMTEXT and \
              CF_UNICODETEXT (or 1, 7, 13) are 1 when the clipboard has text; every other \
              format is 0.",
        run: is_include_clipboard_format,
    },
    EditorFunc {
        name: "GetClipboardByFormat",
        params: &[
            req("format", Ty::Str),
            req("mode", Ty::Int),
            opt("endMode", Ty::Int),
        ],
        result: Ty::Str,
        doc: "The text of the clipboard in a text format (CF_TEXT, CF_OEMTEXT, CF_UNICODETEXT), \
              with the line endings of the document. Mode -2 is the usual way and 3 is UTF-16 \
              (the text itself); the other modes and formats are errors (-32001). The end mode \
              is not used.",
        run: get_clipboard_by_format,
    },
    EditorFunc {
        name: "SetClipboardByFormat",
        params: &[
            req("text", Ty::Str),
            req("format", Ty::Str),
            req("mode", Ty::Int),
            opt("endMode", Ty::Int),
        ],
        result: Ty::Int,
        doc: "Puts the text on the clipboard as text. The formats and modes are those of \
              GetClipboardByFormat. Returns 1 when it worked.",
        run: set_clipboard_by_format,
    },
];

#[cfg(test)]
mod tests {
    use serde_json::json;
    use ume_commands::clipboard::{ClipKind, MemoryClipboard};

    use super::super::testing::Fixture;

    #[test]
    fn the_text_of_the_clipboard() {
        let mut f = Fixture::new("");
        assert_eq!(f.ok("GetClipboard", json!([])), json!(""));
        f.clipboard = MemoryClipboard::with_text("日本語\r\nabc");
        assert_eq!(f.ok("GetClipboard", json!([])), json!("日本語\r\nabc"));
        assert_eq!(f.ok("GetClipboard", json!([0])), json!("日本語\r\nabc"));
    }

    #[test]
    fn set_clipboard_marks_boxes_and_rows() {
        let mut f = Fixture::new("");
        assert_eq!(f.ok("SetClipboard", json!([0, "a"])), json!(1));
        assert_eq!(f.clipboard.data().unwrap().kind, ClipKind::Normal);
        assert_eq!(f.ok("SetClipboard", json!([1, "b"])), json!(1));
        assert_eq!(f.clipboard.data().unwrap().kind, ClipKind::Box);
        assert_eq!(f.ok("SetClipboard", json!([2, "c"])), json!(1));
        assert_eq!(f.clipboard.data().unwrap().kind, ClipKind::Line);
        assert_eq!(f.ok("SetClipboard", json!([3, "d"])), json!(1));
        assert_eq!(f.clipboard.data().unwrap().kind, ClipKind::Box);
        // Sakura accepts no arguments: it sets an empty string
        assert_eq!(f.ok("SetClipboard", json!([])), json!(1));
        assert_eq!(f.clipboard.data().unwrap().text, "");
        assert_eq!(f.ok("SetClipboard", json!([0, "日本\n語"])), json!(1));
        assert_eq!(f.ok("GetClipboard", json!([])), json!("日本\n語"));
    }

    #[test]
    fn formats_are_text_or_nothing() {
        let mut f = Fixture::new("");
        for name in ["CF_UNICODETEXT", "cf_text", "CF_OEMTEXT", "13", "1", "7"] {
            assert_eq!(f.ok("IsIncludeClipboardFormat", json!([name])), json!(0));
        }
        f.clipboard = MemoryClipboard::with_text("x");
        for name in ["CF_UNICODETEXT", "cf_text", "CF_OEMTEXT", "13", "1", "7"] {
            assert_eq!(f.ok("IsIncludeClipboardFormat", json!([name])), json!(1));
        }
        for name in ["CF_HDROP", "CF_BITMAP", "SAKURAClipW", "", "x", "2"] {
            assert_eq!(
                f.ok("IsIncludeClipboardFormat", json!([name])),
                json!(0),
                "{name}"
            );
        }
    }

    #[test]
    fn by_format_converts_line_endings_to_the_document_s() {
        // The document is LF; the clipboard has CR LF.
        let mut f = Fixture::new("a\nb");
        f.clipboard = MemoryClipboard::with_text("x\r\ny\rz");
        assert_eq!(
            f.ok("GetClipboardByFormat", json!(["CF_UNICODETEXT", -2])),
            json!("x\ny\nz")
        );
        assert_eq!(
            f.ok("GetClipboardByFormat", json!(["CF_UNICODETEXT", 3, 2])),
            json!("x\ny\nz")
        );
        // Nothing there: empty
        f.clipboard = MemoryClipboard::default();
        assert_eq!(
            f.ok("GetClipboardByFormat", json!(["CF_TEXT", -2])),
            json!("")
        );
    }

    #[test]
    fn by_format_refuses_what_the_clipboard_cannot_do() {
        let mut f = Fixture::new("");
        for params in [
            json!(["CF_HDROP", -2]),
            json!(["CF_UNICODETEXT", -1]),
            json!(["CF_TEXT", 0]),
        ] {
            let e = f.call("GetClipboardByFormat", params).unwrap_err();
            assert_eq!(e.code, crate::bridge::NOT_IMPLEMENTED);
        }
        let e = f
            .call("SetClipboardByFormat", json!(["x", "CF_BITMAP", -2]))
            .unwrap_err();
        assert_eq!(e.code, crate::bridge::NOT_IMPLEMENTED);
        assert_eq!(
            f.ok(
                "SetClipboardByFormat",
                json!(["日本", "CF_UNICODETEXT", -2])
            ),
            json!(1)
        );
        assert_eq!(f.clipboard.data().unwrap().text, "日本");
        assert_eq!(f.clipboard.data().unwrap().kind, ClipKind::Normal);
    }

    #[test]
    fn without_a_clipboard_the_call_is_unavailable() {
        use ume_commands::context::CommandContext;

        use crate::host::{Editor, HostEnv};
        let mut f = Fixture::new("");
        let mut cx = CommandContext::new(&mut f.model, &mut f.cursor, &mut f.layout, None);
        let mut env = HostEnv::detached();
        let mut editor = Editor {
            cx: &mut cx,
            env: &mut env,
        };
        let e = super::super::call(&mut editor, "GetClipboard", &[])
            .unwrap()
            .unwrap_err();
        assert_eq!(e.code, crate::bridge::UNAVAILABLE);
    }
}
