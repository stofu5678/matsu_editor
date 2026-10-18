//! The two warnings before a file is opened (`CLoadAgent::OnCheckLoad`,
//! `sakura_core/agent/CLoadAgent.cpp`): the file is not there, and the file is big.

use std::path::Path;

use gpui::{AsyncWindowContext, PromptLevel};

use crate::file_ops::ask::{confirm, tell};
use crate::file_ops::prefs::{FilePrefs, prefs};

const MB: u64 = 1 << 20;

/// `STR_NOT_EXSIST_SAVE`.
pub(crate) fn not_exist_message(path: &Path) -> String {
    format!(
        "{}\nというファイルは存在しません。\n\nファイルを保存したときに、ディスク上にこのファイルが作成されます。",
        path.display()
    )
}

/// `STR_LOADAGENT_BIG_WARNING`.
pub(crate) fn big_file_message(path: &Path, limit_mb: i32) -> String {
    format!(
        "'{}'\nファイルサイズが{limit_mb}MB以上あります。開きますか？",
        path.display()
    )
}

/// Whether a file of `len` bytes is big enough to ask about. Sakura compares whole megabytes
/// (`nFileSize.QuadPart >> 20`), so 10 MB less one byte is 9 MB.
pub(crate) fn is_big(prefs: &FilePrefs, len: u64) -> bool {
    prefs.alert_if_large_file && (len / MB) >= u64::try_from(prefs.alert_file_size).unwrap_or(0)
}

/// Runs the checks for `path`. `false` if the user said not to open the file.
pub(crate) async fn allow_open(cx: &mut AsyncWindowContext, path: &Path) -> bool {
    let prefs = cx.update(|_, app| prefs(app)).unwrap_or_default();
    let meta = std::fs::metadata(path);
    // 開こうとしたファイルが存在しないとき警告する: a message only; the file is opened as a new one.
    if prefs.alert_if_file_not_exist && !path.exists() {
        tell(cx, PromptLevel::Info, &not_exist_message(path), None).await;
    }
    if let Ok(meta) = meta
        && is_big(&prefs, meta.len())
        && !confirm(
            cx,
            &big_file_message(path, prefs.alert_file_size),
            None,
            ["開く(Y)", "開かない(N)"],
        )
        .await
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big(size: i32) -> FilePrefs {
        FilePrefs {
            alert_if_large_file: true,
            alert_file_size: size,
            ..FilePrefs::default()
        }
    }

    #[test]
    fn a_file_is_big_by_whole_megabytes() {
        assert!(!is_big(&big(10), 10 * MB - 1));
        assert!(is_big(&big(10), 10 * MB));
        assert!(is_big(&big(1), MB));
        assert!(
            !is_big(&FilePrefs::default(), 100 * MB),
            "the warning is off"
        );
    }

    #[test]
    fn the_messages_name_the_file_and_the_size() {
        assert!(not_exist_message(Path::new("a.txt")).starts_with("a.txt\nというファイル"));
        let text = big_file_message(Path::new("a.txt"), 10);
        assert!(text.contains("10MB以上"));
    }
}
