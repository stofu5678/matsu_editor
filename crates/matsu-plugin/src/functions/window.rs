//! The functions that wait for the user or for time: the message boxes, the file and folder
//! dialogs, and `Sleep`.
//!
//! A macro waits for the answer, but the window must go on (it shows the dialog, and the user can
//! cancel the macro), so the bridge cannot answer these on the spot. [`parse`] recognises such a
//! call; the window opens the dialog (or starts the timer), and sends [`Pending::reply`] when it
//! is answered. Without a window these functions answer [`crate::bridge::UNAVAILABLE`] (they are
//! in [`FUNCS`] so that the catalog and the generated code know their parameter names).
//!
//! `InputBox` (a dialog with a text field) and `CreateMenu` (a pop-up menu built from a string,
//! see [`super::menu`]) wait for the user the same way.
//!
//! Sakura: `sakura_core/macro/CMacro.cpp`, `HandleFunction` (`F_MESSAGEBOX` and the others).

use serde_json::Value;

use super::menu::{self, MenuNode};
use super::{EditorFunc, unavailable};
use crate::api::{Args, Ty, check_params, opt, req};
use crate::host::Editor;
use crate::protocol::{Request, RpcError, parse_request, response_error, response_ok};

type Result<T = Value> = std::result::Result<T, RpcError>;

/// The buttons of a message box (`MB_OK` ... `MB_CANCELTRYCONTINUE`, the low nibble of the type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Buttons {
    Ok,
    OkCancel,
    AbortRetryIgnore,
    YesNoCancel,
    YesNo,
    RetryCancel,
    CancelTryContinue,
}

/// The icon of a message box (`MB_ICONSTOP`, `MB_ICONQUESTION`, `MB_ICONEXCLAMATION`,
/// `MB_ICONINFORMATION`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    None,
    Error,
    Question,
    Warning,
    Information,
}

/// The results of `MessageBox` (`IDOK` ...), which are Windows' numbers.
mod id {
    pub const OK: i64 = 1;
    pub const CANCEL: i64 = 2;
    pub const ABORT: i64 = 3;
    pub const RETRY: i64 = 4;
    pub const IGNORE: i64 = 5;
    pub const YES: i64 = 6;
    pub const NO: i64 = 7;
    pub const TRY_AGAIN: i64 = 10;
    pub const CONTINUE: i64 = 11;
}

impl Buttons {
    /// `MB_*` low nibble: 0 OK, 1 OK/Cancel, 2 Abort/Retry/Ignore, 3 Yes/No/Cancel, 4 Yes/No,
    /// 5 Retry/Cancel, 6 Cancel/Try again/Continue. Anything else is OK (as `MessageBox` does for
    /// a value it does not know, it fails; a macro that passes one gets a plain box).
    fn from_type(kind: i64) -> Self {
        match kind & 0x0F {
            1 => Self::OkCancel,
            2 => Self::AbortRetryIgnore,
            3 => Self::YesNoCancel,
            4 => Self::YesNo,
            5 => Self::RetryCancel,
            6 => Self::CancelTryContinue,
            _ => Self::Ok,
        }
    }

    /// The labels, left to right.
    pub fn labels(self) -> &'static [&'static str] {
        match self {
            Self::Ok => &["OK"],
            Self::OkCancel => &["OK", "キャンセル"],
            Self::AbortRetryIgnore => &["中止", "再試行", "無視"],
            Self::YesNoCancel => &["はい", "いいえ", "キャンセル"],
            Self::YesNo => &["はい", "いいえ"],
            Self::RetryCancel => &["再試行", "キャンセル"],
            Self::CancelTryContinue => &["キャンセル", "再実行", "続行"],
        }
    }

    /// The result of each button (`IDOK`, ...), in the order of the labels.
    fn ids(self) -> &'static [i64] {
        match self {
            Self::Ok => &[id::OK],
            Self::OkCancel => &[id::OK, id::CANCEL],
            Self::AbortRetryIgnore => &[id::ABORT, id::RETRY, id::IGNORE],
            Self::YesNoCancel => &[id::YES, id::NO, id::CANCEL],
            Self::YesNo => &[id::YES, id::NO],
            Self::RetryCancel => &[id::RETRY, id::CANCEL],
            Self::CancelTryContinue => &[id::CANCEL, id::TRY_AGAIN, id::CONTINUE],
        }
    }

    /// The result for the button at `index`, or for a box that was closed without an answer: the
    /// Cancel button if there is one, else the last button.
    pub fn result(self, index: Option<usize>) -> i64 {
        let ids = self.ids();
        index
            .and_then(|i| ids.get(i).copied())
            .or_else(|| ids.iter().copied().find(|&i| i == id::CANCEL))
            .unwrap_or(ids[ids.len() - 1])
    }
}

impl Icon {
    fn from_type(kind: i64) -> Self {
        match kind & 0xF0 {
            0x10 => Self::Error,
            0x20 => Self::Question,
            0x30 => Self::Warning,
            0x40 => Self::Information,
            _ => Self::None,
        }
    }
}

/// What the window has to do before it can answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Deferred {
    /// A message box. The result is the number of the button ([`Buttons::result`]).
    Message {
        text: String,
        buttons: Buttons,
        icon: Icon,
    },
    /// The dialog to choose a file to open. The result is the path, or an empty string.
    FileOpen { default: String, filter: String },
    /// The dialog to choose a name to save under.
    FileSave { default: String, filter: String },
    /// The dialog to choose a folder.
    Folder { message: String, default: String },
    /// Wait for this many milliseconds. The result is 0.
    Sleep { millis: u64 },
    /// The text input dialog. The result is the text, or an empty string when it is cancelled.
    /// `default` is already cut to `max_len` (UTF-16 code units, as Sakura's edit control counts).
    Input {
        message: String,
        default: String,
        max_len: usize,
    },
    /// A pop-up menu. The result is the number of the item chosen ([`MenuNode::Item::number`]),
    /// or 0 when the menu is closed without a choice. `at_caret`: show it at the caret (else at
    /// the pointer).
    Menu {
        at_caret: bool,
        nodes: Vec<MenuNode>,
    },
}

/// A call that waits: the id to answer under, and what to wait for.
#[derive(Debug, Clone, PartialEq)]
pub struct Pending {
    pub id: Value,
    pub what: Deferred,
}

impl Pending {
    /// The response line for the answer.
    pub fn reply(&self, result: Value) -> String {
        response_ok(&self.id, result)
    }
}

/// `MB_OK | MB_ICONSTOP` and the like for the message functions that have a fixed type
/// (CMacro.cpp, F_ERRORMSG ... F_YESNOBOX).
fn message_kind(method: &str, params: &[Value]) -> Option<(Buttons, Icon)> {
    Some(match method {
        "MessageBox" => {
            let kind = params.get(1).and_then(Value::as_i64).unwrap_or(0);
            (Buttons::from_type(kind), Icon::from_type(kind))
        }
        "ErrorMsg" => (Buttons::Ok, Icon::Error),
        "WarnMsg" => (Buttons::Ok, Icon::Warning),
        "InfoMsg" => (Buttons::Ok, Icon::Information),
        "OkCancelBox" => (Buttons::OkCancel, Icon::Question),
        "YesNoBox" => (Buttons::YesNo, Icon::Question),
        _ => return None,
    })
}

/// The buttons and the icon of the message function `method` (`MessageBox`, `ErrorMsg`, ...) for a
/// `MessageBox` type of `kind` (only `MessageBox` reads it). `None` for any other name. A key macro
/// shows its boxes with this (`CMacro::HandleCommand`).
pub fn message_style(method: &str, kind: i64) -> Option<(Buttons, Icon)> {
    message_kind(method, &[Value::Null, Value::from(kind)])
}

/// Longest default path for the file dialogs (`MAX_PATH`; CMacro.cpp refuses a longer one).
const MAX_PATH: usize = 260;

/// The longest input of `InputBox` when the macro gives no length (`_MAX_PATH`).
const INPUT_DEFAULT_LEN: usize = 260;

/// `text` cut to at most `max` UTF-16 code units, without splitting a character.
pub fn truncate_utf16(text: &str, max: usize) -> &str {
    let mut units = 0;
    for (at, c) in text.char_indices() {
        units += c.len_utf16();
        if units > max {
            return &text[..at];
        }
    }
    text
}

/// Whether the message line is a call that waits, and if so what it waits for.
///
/// * `None`: not such a call (or a notification, which gets no answer): run it as usual.
/// * `Some(Err(line))`: it is, but its arguments are wrong; `line` is the error response.
/// * `Some(Ok(pending))`: open the dialog (or start the timer) and send `pending.reply(..)`.
pub fn parse(text: &str) -> Option<std::result::Result<Pending, String>> {
    let request = parse_request(text).ok()?;
    let Request { id, method, params } = request;
    let id = id?;
    let func = FUNCS.iter().find(|f| f.name == method)?;
    if let Err(error) = check_params(func.name, func.params, &params) {
        return Some(Err(response_error(&id, &error)));
    }
    let text_at = |i: usize| {
        params
            .get(i)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let what = if let Some((buttons, icon)) = message_kind(&method, &params) {
        Deferred::Message {
            text: text_at(0),
            buttons,
            icon,
        }
    } else {
        match method.as_str() {
            "FileOpenDialog" | "FileSaveDialog" => {
                let default = text_at(0);
                if default.encode_utf16().count() >= MAX_PATH {
                    let error = RpcError::invalid_params(format!(
                        "{method}: the default file name is too long"
                    ));
                    return Some(Err(response_error(&id, &error)));
                }
                let filter = text_at(1);
                if method == "FileOpenDialog" {
                    Deferred::FileOpen { default, filter }
                } else {
                    Deferred::FileSave { default, filter }
                }
            }
            "FolderDialog" => {
                let default = text_at(1);
                if default.encode_utf16().count() >= MAX_PATH {
                    let error =
                        RpcError::invalid_params("FolderDialog: the default folder is too long");
                    return Some(Err(response_error(&id, &error)));
                }
                Deferred::Folder {
                    message: text_at(0),
                    default,
                }
            }
            "InputBox" => {
                // F_INPUTBOX: a length of 0 or less is _MAX_PATH; the default text is cut to it.
                let max_len = params
                    .get(2)
                    .and_then(Value::as_i64)
                    .and_then(|n| usize::try_from(n).ok())
                    .filter(|&n| n > 0)
                    .unwrap_or(INPUT_DEFAULT_LEN);
                let default = text_at(1);
                Deferred::Input {
                    message: text_at(0),
                    default: truncate_utf16(&default, max_len).to_owned(),
                    max_len,
                }
            }
            "CreateMenu" => Deferred::Menu {
                at_caret: params.first().and_then(Value::as_i64) == Some(1),
                nodes: menu::parse(&text_at(1)),
            },
            "Sleep" => Deferred::Sleep {
                millis: params
                    .first()
                    .and_then(Value::as_i64)
                    .map_or(0, |ms| u64::try_from(ms).unwrap_or(0)),
            },
            _ => return None,
        }
    };
    Some(Ok(Pending { id, what }))
}

/// The run function of these entries: the window answers them (see [`parse`]).
fn needs_window(_: &mut Editor, args: &Args) -> Result {
    let _ = args;
    Err(unavailable("a dialog or a wait"))
}

/// Whether a function of the catalog is answered by the window with [`parse`].
pub fn is_deferred(name: &str) -> bool {
    FUNCS.iter().any(|f| f.name == name)
}

pub const FUNCS: &[EditorFunc] = &[
    EditorFunc {
        name: "MessageBox",
        params: &[req("message", Ty::Str), opt("type", Ty::Int)],
        result: Ty::Int,
        doc: "Shows a message box and waits. The type is Windows' MB_ value: the low nibble \
              chooses the buttons (0 OK, 1 OK/Cancel, 2 Abort/Retry/Ignore, 3 Yes/No/Cancel, \
              4 Yes/No, 5 Retry/Cancel, 6 Cancel/Try again/Continue), the icon bits (0x10 \
              stop, 0x20 question, 0x30 exclamation, 0x40 information) the icon; the default \
              button and the modality are ignored. Returns the button as Windows numbers it: \
              1 OK, 2 Cancel, 3 Abort, 4 Retry, 5 Ignore, 6 Yes, 7 No, 10 Try again, 11 \
              Continue. A box that is closed without an answer gives Cancel (the last button \
              when there is none).",
        run: needs_window,
    },
    EditorFunc {
        name: "ErrorMsg",
        params: &[req("message", Ty::Str)],
        result: Ty::Int,
        doc: "Shows an error message with an OK button and waits. Returns 1.",
        run: needs_window,
    },
    EditorFunc {
        name: "WarnMsg",
        params: &[req("message", Ty::Str)],
        result: Ty::Int,
        doc: "Shows a warning with an OK button and waits. Returns 1.",
        run: needs_window,
    },
    EditorFunc {
        name: "InfoMsg",
        params: &[req("message", Ty::Str)],
        result: Ty::Int,
        doc: "Shows an information message with an OK button and waits. Returns 1.",
        run: needs_window,
    },
    EditorFunc {
        name: "OkCancelBox",
        params: &[req("message", Ty::Str)],
        result: Ty::Int,
        doc: "Asks a question with OK and Cancel and waits. Returns 1 (OK) or 2 (Cancel).",
        run: needs_window,
    },
    EditorFunc {
        name: "YesNoBox",
        params: &[req("message", Ty::Str)],
        result: Ty::Int,
        doc: "Asks a question with Yes and No and waits. Returns 6 (Yes) or 7 (No).",
        run: needs_window,
    },
    EditorFunc {
        name: "FileOpenDialog",
        params: &[opt("defaultName", Ty::Str), opt("filter", Ty::Str)],
        result: Ty::Str,
        doc: "Lets the user choose a file to open and returns its full path, or an empty string \
              when the dialog is cancelled. The default name and the filter are not used: the \
              dialog of the platform takes neither.",
        run: needs_window,
    },
    EditorFunc {
        name: "FileSaveDialog",
        params: &[opt("defaultName", Ty::Str), opt("filter", Ty::Str)],
        result: Ty::Str,
        doc: "Lets the user choose a file name to save under and returns the full path, or an \
              empty string when the dialog is cancelled. The dialog starts with the default \
              name. The filter is not used.",
        run: needs_window,
    },
    EditorFunc {
        name: "FolderDialog",
        params: &[opt("message", Ty::Str), opt("defaultFolder", Ty::Str)],
        result: Ty::Str,
        doc: "Lets the user choose a folder and returns its full path, or an empty string when \
              the dialog is cancelled. The message becomes the prompt text of the platform \
              dialog where it has one; the default folder is not used.",
        run: needs_window,
    },
    EditorFunc {
        name: "InputBox",
        params: &[
            req("message", Ty::Str),
            opt("defaultValue", Ty::Str),
            opt("maxLength", Ty::Int),
        ],
        result: Ty::Str,
        doc: "Shows a dialog with a text field and waits. The message is shown above the field, \
              the default value is in the field, and the text cannot be longer than the maximum \
              length (in UTF-16 code units; 0 or less, or no value, is 260). Returns the text \
              of the field when OK is pressed, or an empty string when the dialog is cancelled.",
        run: needs_window,
    },
    EditorFunc {
        name: "CreateMenu",
        params: &[req("position", Ty::Int), req("menu", Ty::Str)],
        result: Ty::Int,
        doc: "Shows a pop-up menu and waits. The position is 1 for the caret (the pointer when \
              the caret is out of the view) and anything else for the pointer. The menu is a \
              list of items separated by commas; an item can start with flags in brackets: C \
              checked, D disabled, R radio, S opens a submenu, E ends a submenu (one for each \
              E), B (a new column) is ignored. A lone - is a separator. Write [][text] for a \
              text that starts with a bracket. Returns the number of the item chosen, counted \
              from 1 over the items only (separators and submenus are not counted), or 0 when \
              the menu is closed without a choice or has no item.",
        run: needs_window,
    },
    EditorFunc {
        name: "Sleep",
        params: &[req("milliseconds", Ty::Int)],
        result: Ty::Int,
        doc: "Waits for the given number of milliseconds (the window stays alive, and the \
              macro can be cancelled). Returns 0.",
        run: needs_window,
    },
];

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn request(method: &str, params: Value) -> String {
        json!({"jsonrpc": "2.0", "id": 7, "method": method, "params": params}).to_string()
    }

    fn pending(method: &str, params: Value) -> Pending {
        parse(&request(method, params))
            .expect("a call that waits")
            .expect("good arguments")
    }

    #[test]
    fn the_message_types_choose_buttons_and_icons() {
        let p = pending("MessageBox", json!(["本文", 0x24]));
        assert_eq!(
            p.what,
            Deferred::Message {
                text: "本文".into(),
                buttons: Buttons::YesNo,
                icon: Icon::Question
            }
        );
        let p = pending("MessageBox", json!(["x"]));
        assert_eq!(
            p.what,
            Deferred::Message {
                text: "x".into(),
                buttons: Buttons::Ok,
                icon: Icon::None
            }
        );
        for (kind, buttons) in [
            (1, Buttons::OkCancel),
            (2, Buttons::AbortRetryIgnore),
            (3, Buttons::YesNoCancel),
            (5, Buttons::RetryCancel),
            (6, Buttons::CancelTryContinue),
            (0x100, Buttons::Ok),
        ] {
            let Deferred::Message { buttons: got, .. } =
                pending("MessageBox", json!(["x", kind])).what
            else {
                panic!()
            };
            assert_eq!(got, buttons, "{kind}");
        }
    }

    #[test]
    fn the_fixed_message_functions() {
        for (method, buttons, icon) in [
            ("ErrorMsg", Buttons::Ok, Icon::Error),
            ("WarnMsg", Buttons::Ok, Icon::Warning),
            ("InfoMsg", Buttons::Ok, Icon::Information),
            ("OkCancelBox", Buttons::OkCancel, Icon::Question),
            ("YesNoBox", Buttons::YesNo, Icon::Question),
        ] {
            assert_eq!(
                pending(method, json!(["m"])).what,
                Deferred::Message {
                    text: "m".into(),
                    buttons,
                    icon
                },
                "{method}"
            );
        }
    }

    #[test]
    fn results_are_windows_numbers() {
        assert_eq!(Buttons::Ok.result(Some(0)), 1);
        assert_eq!(Buttons::OkCancel.result(Some(0)), 1);
        assert_eq!(Buttons::OkCancel.result(Some(1)), 2);
        assert_eq!(Buttons::YesNo.result(Some(0)), 6);
        assert_eq!(Buttons::YesNo.result(Some(1)), 7);
        assert_eq!(Buttons::YesNoCancel.result(Some(2)), 2);
        assert_eq!(Buttons::AbortRetryIgnore.result(Some(1)), 4);
        assert_eq!(Buttons::CancelTryContinue.result(Some(1)), 10);
        assert_eq!(Buttons::CancelTryContinue.result(Some(2)), 11);
        // closed without an answer: Cancel where there is one, else the last button
        assert_eq!(Buttons::OkCancel.result(None), 2);
        assert_eq!(Buttons::YesNoCancel.result(None), 2);
        assert_eq!(Buttons::Ok.result(None), 1);
        assert_eq!(Buttons::YesNo.result(None), 7);
        assert_eq!(Buttons::AbortRetryIgnore.result(None), 5);
        // a button that is not there
        assert_eq!(Buttons::Ok.result(Some(5)), 1);
    }

    #[test]
    fn dialogs_and_sleep() {
        assert_eq!(
            pending("FileOpenDialog", json!([])).what,
            Deferred::FileOpen {
                default: String::new(),
                filter: String::new()
            }
        );
        assert_eq!(
            pending("FileSaveDialog", json!(["メモ.txt", "*.txt"])).what,
            Deferred::FileSave {
                default: "メモ.txt".into(),
                filter: "*.txt".into()
            }
        );
        assert_eq!(
            pending("FolderDialog", json!(["選んで", "/tmp"])).what,
            Deferred::Folder {
                message: "選んで".into(),
                default: "/tmp".into()
            }
        );
        assert_eq!(
            pending("Sleep", json!([250])).what,
            Deferred::Sleep { millis: 250 }
        );
        // a negative time is no time
        assert_eq!(
            pending("Sleep", json!([-5])).what,
            Deferred::Sleep { millis: 0 }
        );
    }

    #[test]
    fn input_box_takes_the_message_the_default_and_the_length() {
        assert_eq!(
            pending("InputBox", json!(["名前は？"])).what,
            Deferred::Input {
                message: "名前は？".into(),
                default: String::new(),
                max_len: 260
            }
        );
        assert_eq!(
            pending("InputBox", json!(["m", "既定値", 10])).what,
            Deferred::Input {
                message: "m".into(),
                default: "既定値".into(),
                max_len: 10
            }
        );
        // 0 or less is the default length
        for length in [0, -3] {
            let Deferred::Input { max_len, .. } =
                pending("InputBox", json!(["m", "", length])).what
            else {
                panic!()
            };
            assert_eq!(max_len, 260, "{length}");
        }
    }

    #[test]
    fn the_default_of_input_box_is_cut_to_the_length_in_utf16_units() {
        let Deferred::Input { default, .. } =
            pending("InputBox", json!(["m", "あいうえお", 3])).what
        else {
            panic!()
        };
        assert_eq!(default, "あいう");
        // a character outside the BMP is two units: it is not split
        let Deferred::Input { default, .. } = pending("InputBox", json!(["m", "a𠮷b", 2])).what
        else {
            panic!()
        };
        assert_eq!(default, "a");
        assert_eq!(truncate_utf16("a𠮷b", 3), "a𠮷");
        assert_eq!(truncate_utf16("abc", 10), "abc");
        assert_eq!(truncate_utf16("abc", 0), "");
    }

    #[test]
    fn create_menu_takes_the_place_and_the_menu_string() {
        let p = pending("CreateMenu", json!([1, "[C]あ,-,い"]));
        let Deferred::Menu { at_caret, nodes } = p.what else {
            panic!()
        };
        assert!(at_caret);
        assert_eq!(menu::item_count(&nodes), 2);
        for place in [0, 2, -1] {
            let Deferred::Menu { at_caret, .. } = pending("CreateMenu", json!([place, "a"])).what
            else {
                panic!()
            };
            assert!(!at_caret, "{place}");
        }
        // both arguments are needed
        let line = parse(&request("CreateMenu", json!([1])))
            .unwrap()
            .unwrap_err();
        assert!(line.contains("-32602"), "{line}");
    }

    #[test]
    fn bad_arguments_get_an_error_line_and_others_are_not_ours() {
        let line = parse(&request("MessageBox", json!([5])))
            .unwrap()
            .unwrap_err();
        assert!(line.contains("-32602"), "{line}");
        let long = "x".repeat(300);
        let line = parse(&request("FileOpenDialog", json!([long])))
            .unwrap()
            .unwrap_err();
        assert!(line.contains("too long"), "{line}");
        // not a call that waits
        assert!(parse(&request("GetClipboard", json!([]))).is_none());
        assert!(parse("not json").is_none());
        // a notification gets no answer, so nothing waits for it
        assert!(parse(r#"{"jsonrpc":"2.0","method":"Sleep","params":[1]}"#).is_none());
        assert!(is_deferred("Sleep"));
        assert!(!is_deferred("GetFilename"));
    }

    #[test]
    fn the_reply_carries_the_id() {
        let p = pending("InfoMsg", json!(["m"]));
        let line = p.reply(json!(1));
        let value: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["id"], json!(7));
        assert_eq!(value["result"], json!(1));
    }

    #[test]
    fn without_a_window_they_are_unavailable() {
        let mut f = super::super::testing::Fixture::new("");
        let e = f.call("InfoMsg", json!(["m"])).unwrap_err();
        assert_eq!(e.code, crate::bridge::UNAVAILABLE);
    }
}
