//! The questions and the error messages of the file flows.
//!
//! The texts follow Sakura's (`sakura_core/sakura_rc.rc`; each function names its string).

use std::path::Path;

use gpui::{AsyncWindowContext, PromptLevel};
use ume_core::encoding::Encoding;
use ume_core::file::{FileError, UnmappableChar};

/// The answer to a question with three buttons (Sakura's Yes / No / Cancel).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    Yes,
    No,
    Cancel,
}

/// Asks with three buttons, in the order Yes, No, Cancel. Closing the prompt is Cancel.
pub(crate) async fn ask(
    cx: &mut AsyncWindowContext,
    level: PromptLevel,
    message: &str,
    detail: Option<&str>,
    labels: [&str; 3],
) -> Answer {
    match cx.prompt(level, message, detail, &labels).await {
        Ok(0) => Answer::Yes,
        Ok(1) => Answer::No,
        _ => Answer::Cancel,
    }
}

/// Asks with two buttons. `true` is the first one.
pub(crate) async fn confirm(
    cx: &mut AsyncWindowContext,
    message: &str,
    detail: Option<&str>,
    labels: [&str; 2],
) -> bool {
    matches!(
        cx.prompt(PromptLevel::Warning, message, detail, &labels)
            .await,
        Ok(0)
    )
}

/// Tells the user something and waits for the OK.
pub(crate) async fn tell(
    cx: &mut AsyncWindowContext,
    level: PromptLevel,
    message: &str,
    detail: Option<&str>,
) {
    let _ = cx.prompt(level, message, detail, &["OK"]).await;
}

/// The name of a document in a question: its path, or 無題 (`STR_NO_TITLE1`).
pub(crate) fn document_title(path: Option<&Path>) -> String {
    path.map_or_else(|| "(無題)".to_owned(), |p| p.display().to_string())
}

/// `STR_ERR_DLGEDITDOC31`: asked before a modified document is closed.
pub(crate) fn save_before_close_message(title: &str) -> String {
    format!("{title}\nは変更されています。 閉じる前に保存しますか？")
}

/// `STR_CHANGE_CHARSET`: asked before a document is closed whose text is unmodified but whose
/// encoding or BOM was changed.
pub(crate) fn charset_change_message(title: &str) -> String {
    format!(
        "{title}\nはテキストの変更は有りませんが、\n文字コードセットが変更されています。\n閉じる前に保存しますか？"
    )
}

/// `STR_ERR_CEDITVIEW_CMD29`: asked before reloading a modified file.
pub(crate) fn reload_message(title: &str) -> String {
    format!(
        "{title}\n\nこのファイルは変更されています。\n再ロードを行うと変更が失われますが、よろしいですか?"
    )
}

/// `STR_CODECHECKER_EOL_UNIFY`: asked when the line endings are mixed.
pub(crate) fn unify_line_endings_message(name: &str) -> String {
    format!("改行コードが混在しています。\n現在の入力改行コード {name} に統一しますか？")
}

/// `STR_CODECHECKER_CONFORM_LOSESOME`: asked when the encoding cannot write some characters.
/// `first` is the first of them; `total` counts all.
pub(crate) fn unmappable_message(
    encoding: Encoding,
    first: Option<&UnmappableChar>,
    total: usize,
) -> (String, String) {
    let message = format!(
        "文字エンコード {} で保存しようとしていますが、\n文字コード変換により一部の文字情報が失われます。\n保存処理を続行しますか？",
        encoding.combo_name()
    );
    let detail = match first {
        Some(c) => format!(
            "最初の場所 {}行 {}桁 文字[{}] U+{:04X}\n全部で {total} 文字\n「該当位置に移動」を選ぶと、保存せずにその場所へ移る",
            c.pos.line + 1,
            c.col_utf16 + 1,
            c.ch,
            u32::from(c.ch)
        ),
        None => format!("全部で {total} 文字"),
    };
    (message, detail)
}

/// `STR_LOADAGENT_ERR_OPEN`, `STR_LOADAGENT_BIG_ERROR` and the like, for a file that cannot be read.
pub(crate) fn open_error_message(path: &Path, error: &FileError) -> (String, String) {
    let message = format!("'{}'\nを開けませんでした。", path.display());
    let detail = match error {
        FileError::TooLarge { len, max } => {
            format!("ファイルが大きすぎる（{len} バイト。開けるのは {max} バイトまで）")
        }
        other => other.to_string(),
    };
    (message, detail)
}

/// A file that could not be written. Nothing was changed on disk (ADR-0005).
pub(crate) fn save_error_message(path: &Path, error: &FileError) -> (String, String) {
    let message = format!("'{}'\nを保存できませんでした。", path.display());
    let detail = match error {
        FileError::ReadOnly => "書き込みが禁止されている".to_owned(),
        other => format!("{other}\n元のファイルは変わっていない"),
    };
    (message, detail)
}

#[cfg(test)]
mod tests {
    use ume_core::buffer::Pos;

    use super::*;

    #[test]
    fn the_questions_name_the_document() {
        assert_eq!(document_title(None), "(無題)");
        assert_eq!(document_title(Some(Path::new("a.txt"))), "a.txt");
        assert!(save_before_close_message("a.txt").starts_with("a.txt\nは変更されています"));
        assert!(reload_message("a.txt").contains("再ロード"));
        assert!(unify_line_endings_message("LF").contains("LF"));
    }

    #[test]
    fn the_unmappable_question_shows_the_first_place() {
        let c = UnmappableChar {
            pos: Pos::new(2, 6),
            col_utf16: 2,
            ch: '😀',
        };
        let (message, detail) = unmappable_message(Encoding::ShiftJis, Some(&c), 3);
        assert!(message.contains("SJIS"));
        assert!(detail.contains("3行 3桁"));
        assert!(detail.contains("U+1F600"));
        assert!(detail.contains("3 文字"));
    }

    #[test]
    fn a_big_file_says_the_limit() {
        let e = FileError::TooLarge { len: 5, max: 4 };
        let (_, detail) = open_error_message(Path::new("big"), &e);
        assert!(detail.contains("大きすぎる"));
    }
}
