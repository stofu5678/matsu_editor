//! The flow of `F_PLSQL_COMPILE_ON_SQLPLUS` (ADR-0023).
//!
//! Sakura (`Command_PLSQL_COMPILE_ON_SQLPLUS`, `sakura_core/cmd/CViewCommander_File.cpp`):
//!
//! 1. a changed document: ask 「… は変更されています。 Oracle SQL*Plusで実行する前に保存しますか？」
//!    with はい / いいえ / キャンセル; はい saves (the save dialog for a document without a file) and
//!    いいえ and キャンセル stop (`STR_ERR_CEDITVIEW_CMD18`);
//! 2. a document that still has no file: an error, 「SQLをファイルに保存しないとOracle SQL*Plusで実行
//!    できません。」 (`STR_ERR_CEDITVIEW_CMD21`);
//! 3. the file is given to SQL*Plus.
//!
//! Here step 0 finds the connection (Sakura finds the SQL*Plus window and says it is not running
//! when it is not there), and step 3 runs the command line of the connection with 外部コマンド実行,
//! the output going to the output panel. A connection whose template has no `$F` takes the text on
//! its standard input (the selection, or else the whole document), so it needs no file: steps 1 and 2
//! do not apply to it.

use std::path::PathBuf;

use gpui::{AsyncWindowContext, PromptLevel, WeakEntity};
use matsu_config::databases::{self, Connection, Databases, Encoding, Kind, Problem};
use matsu_core::buffer::col_to_utf16;
use matsu_core::exec::ExecFlags;
use matsu_core::sql_error::ErrorStyle;

use super::command::{self, Shell};
use crate::exec::flow::{Request, run_command};
use crate::file_ops::ask::{Answer, ask, confirm, tell};
use crate::file_ops::save::{SaveRequest, save};
use crate::macros::JumpContext;
use crate::view::EditorView;

/// The shown name of a document without a file (`STR_NO_TITLE1`).
const NO_TITLE: &str = "(無題)";

/// The option bits for `connection`: the output is taken (into the output panel), in the encoding of
/// the connection, and the text is sent to the input when the template has no file.
pub(crate) fn flags(connection: &Connection) -> ExecFlags {
    let out = match connection.encoding {
        Encoding::Utf8 => ExecFlags::OUT_UTF8,
        Encoding::Utf16 => ExecFlags::OUT_UNICODE,
        Encoding::ShiftJis => 0,
    };
    let mut bits = ExecFlags::GET_STDOUT | out;
    if !connection.uses_file() {
        bits |= ExecFlags::SEND_STDIN
            | match connection.input_encoding() {
                Encoding::Utf8 => ExecFlags::IN_UTF8,
                Encoding::Utf16 => ExecFlags::IN_UNICODE,
                Encoding::ShiftJis => 0,
            };
    }
    if connection.folder.is_some() {
        bits |= ExecFlags::CUR_DIR;
    }
    ExecFlags(bits)
}

/// What a word of the message says about the problem with a connection.
fn describe(problem: &Problem) -> String {
    match problem {
        Problem::BadName(name) => format!("接続の名前 {name:?} は使えません。"),
        Problem::UnknownKind { connection, value } => {
            format!("接続 {connection} の Kind={value} は知らない種類です。")
        }
        Problem::UnknownEncoding { connection, value } => {
            format!("接続 {connection} の文字コード {value} は知らない名前です。")
        }
        Problem::NoCommand(name) => format!("接続 {name} に Command= がありません。"),
        Problem::NoSuchDefault(name) => {
            format!("Default={name} の接続が、ファイルにありません。")
        }
    }
}

/// Finds the connection to use. Tells the user what to do when there is none, and returns `None`.
async fn connection(cx: &mut AsyncWindowContext) -> Option<Connection> {
    let path = cx.update(|_, app| super::file_path(app)).ok().flatten();
    let Some(path) = path else {
        tell(
            cx,
            PromptLevel::Warning,
            "データベースの接続の設定ファイル（databases.ini）の場所が決められません。",
            None,
        )
        .await;
        return None;
    };
    let shown = path.display().to_string();
    let databases = match Databases::load(&path) {
        Ok(databases) => databases,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            offer_example(cx, &path).await;
            return None;
        }
        Err(error) => {
            let message = format!("接続の設定ファイルを読めません。\n\n{shown}\n\n{error}");
            tell(cx, PromptLevel::Warning, &message, None).await;
            return None;
        }
    };
    let Some(default) = databases.default.clone() else {
        let message = format!(
            "使うデータベースの接続が決まっていません。\n\n{shown}\nの [Database] の Default= に、[Connection.名前] の名前を書いてください。"
        );
        tell(cx, PromptLevel::Info, &message, None).await;
        return None;
    };
    match databases.get(&default) {
        Some(connection) => Some(connection.clone()),
        None => {
            // the problem of the default, else the plain "not in the file"
            let reason = databases
                .problems
                .iter()
                .find(|p| match p {
                    Problem::BadName(n) | Problem::NoCommand(n) => *n == default,
                    Problem::UnknownKind { connection, .. }
                    | Problem::UnknownEncoding { connection, .. } => *connection == default,
                    Problem::NoSuchDefault(_) => true,
                })
                .map(describe)
                .unwrap_or_else(|| describe(&Problem::NoSuchDefault(default.clone())));
            let message = format!("接続を使えません。\n\n{shown}\n\n{reason}");
            tell(cx, PromptLevel::Warning, &message, None).await;
            None
        }
    }
}

/// There is no `databases.ini`: offers to make one with the examples, and says where it is.
async fn offer_example(cx: &mut AsyncWindowContext, path: &std::path::Path) {
    let shown = path.display().to_string();
    let message = format!(
        "データベースの接続の設定ファイルがありません。\n\n{shown}\n\n例の入ったファイルを作りますか？"
    );
    if !confirm(cx, &message, None, ["作る", "作らない"]).await {
        return;
    }
    match databases::create_example_file(path) {
        Ok(_) => {
            let message = format!(
                "作りました。\n\n{shown}\n\n使う接続を選んで、[Database] の Default= にその名前を書いてください。例は実物のクライアントで確かめていないので、ホスト名やデータベース名を直してください。パスワードは書かないでください。"
            );
            tell(cx, PromptLevel::Info, &message, None).await;
        }
        Err(error) => {
            let message = format!("ファイルを作れません。\n\n{shown}\n\n{error}");
            tell(cx, PromptLevel::Warning, &message, None).await;
        }
    }
}

/// `Command_PLSQL_COMPILE_ON_SQLPLUS`.
pub(crate) async fn run_document(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    let Some(connection) = connection(cx).await else {
        return;
    };
    if let Some(word) = databases::password_in_command(connection.kind, &connection.command) {
        let message = format!(
            "接続 {} のコマンドにパスワードが書かれているようです（{word}）。\n\nパスワードはコマンドに書かず、クライアントの仕組み（~/.pgpass、mysql_config_editor、Oracle Wallet、gcloud auth、Windows の統合認証など）に任せてください。実行しません。",
            connection.name
        );
        tell(cx, PromptLevel::Warning, &message, None).await;
        return;
    }

    let file = if connection.uses_file() {
        match prepare_file(view, cx).await {
            Some(file) => Some(file),
            None => return,
        }
    } else {
        None
    };

    let line = match command::build(Shell::current(), &connection, file.as_deref()) {
        Ok(line) => line,
        Err(error) => {
            let message = format!("接続 {} を実行できません。\n\n{error}", connection.name);
            tell(cx, PromptLevel::Warning, &message, None).await;
            return;
        }
    };
    let jump = jump_context(view, cx, &connection);
    run_command(
        view,
        cx,
        Request {
            // the exec flow expands `$` codes in a command line; none is meant here
            command: command::escape_for_expand(&line),
            flags: flags(&connection),
            folder: connection.folder.clone(),
            jump,
        },
    )
    .await;
}

/// The error style of the client of a connection.
pub(crate) fn error_style(kind: Kind) -> ErrorStyle {
    match kind {
        Kind::Oracle => ErrorStyle::Oracle,
        Kind::MySql => ErrorStyle::MySql,
        Kind::PostgreSql => ErrorStyle::PostgreSql,
        Kind::SqlServer => ErrorStyle::SqlServer,
        Kind::Sqlite => ErrorStyle::Sqlite,
        Kind::BigQuery => ErrorStyle::BigQuery,
        Kind::Other => ErrorStyle::Generic,
    }
}

/// What a click on an error line of the output leads to (ADR-0023, step 3): the document that ran,
/// the folder that file names are relative to, and, when only the selection went to the client, the
/// line it starts at (the client counts from the start of what it got).
fn jump_context(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    connection: &Connection,
) -> Option<JumpContext> {
    let sent_selection = !connection.uses_file();
    view.read_with(cx, |view, app| {
        // a rectangle is sent without its left margin, row by row: the places in the output are
        // not the places in the document, so nothing leads anywhere
        if sent_selection && view.cursor().is_box() && view.cursor().has_selection() {
            return None;
        }
        let shared = view.document().read(app);
        // a document without a file cannot be told from the next one that is loaded into its
        // window, so an old error line would lead into the wrong text: nothing leads anywhere
        let file = Some(shared.path().map(PathBuf::from)?);
        let folder = connection
            .folder
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| file.as_ref().and_then(|f| f.parent().map(PathBuf::from)));
        // the lines and the first column of what the client got count from the selection
        let (line_offset, column_offset) = match view.cursor().selection() {
            Some((from, _)) if sent_selection => {
                let before = shared
                    .doc()
                    .line(from.line)
                    .ok()
                    .flatten()
                    .map_or(0, |line| col_to_utf16(&line.text, from.col));
                (from.line, before)
            }
            _ => (0, 0),
        };
        Some(JumpContext {
            style: error_style(connection.kind),
            file,
            folder,
            line_offset,
            column_offset,
        })
    })
    .ok()
    .flatten()
}

/// Steps 1 and 2 of Sakura's command: the question about saving and the file. `None` when the
/// run stops.
async fn prepare_file(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
) -> Option<String> {
    let state = |view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext| {
        view.read_with(cx, |view, app| {
            let shared = view.document().read(app);
            (
                shared.path().map(PathBuf::from),
                // a change of the encoding or the BOM is saved too (the client reads the file on
                // disk), as the save and close flows count it (`CDocFile::IsChgCodeSet`)
                shared.model().is_modified() || shared.format_changed(),
            )
        })
        .ok()
    };
    let (path, modified) = state(view, cx)?;
    if modified {
        let title = path
            .as_ref()
            .map_or_else(|| NO_TITLE.to_owned(), |p| p.display().to_string());
        let message =
            format!("{title}\nは変更されています。 データベースで実行する前に保存しますか？");
        match ask(
            cx,
            PromptLevel::Warning,
            &message,
            None,
            ["はい", "いいえ", "キャンセル"],
        )
        .await
        {
            Answer::Yes => {
                if !save(view, cx, SaveRequest::default()).await.is_safe() {
                    return None;
                }
            }
            Answer::No | Answer::Cancel => return None,
        }
    }
    let (path, modified) = state(view, cx)?;
    match path {
        Some(path) if !modified => match path.to_str() {
            Some(file) => Some(file.to_owned()),
            None => {
                tell(
                    cx,
                    PromptLevel::Warning,
                    "ファイルの名前が文字として読めないので、実行できません。",
                    None,
                )
                .await;
                None
            }
        },
        _ => {
            // STR_ERR_CEDITVIEW_CMD21
            tell(
                cx,
                PromptLevel::Warning,
                "SQLをファイルに保存しないとデータベースで実行できません。",
                None,
            )
            .await;
            None
        }
    }
}
