//! Keeping the documents that are not saved, so that a crash or a power loss does not lose them (ADR-0022).
//!
//! This is the writing side. The files and their order are in `matsu_config::unsaved`.
//!
//! * [`prefs`]: the three items of `[MATSU]` (`KeepUnsaved`, `KeepUnsavedNoAsk`, `KeepUnsavedMaxMB`).
//! * [`writer`]: the thread that does every operation on the folder, in the order they were queued.
//! * [`keeper`]: the timers (2 seconds after the last change, at most 30 seconds while typing), the decision
//!   which documents have a copy, and the discard of the copy of a document that was saved or closed.
//!
//! Bringing the documents back at the next start, and not asking 「保存しますか」 at the end (hot exit), are
//! separate changes (ADR-0022, section 9).

pub(crate) mod keeper;
pub(crate) mod prefs;
pub(crate) mod writer;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod window_tests;

pub(crate) use keeper::{clear_copies, discard_document, start};

use gpui::App;

/// The user turned the feature off: asks whether to delete what is kept (ADR-0022, section 8). 「消す」 deletes
/// the copies; 「残す」 leaves them where they are (they are not used until the feature is turned on again).
pub(crate) fn ask_to_clear(cx: &mut App) {
    // An editor window, not the dialog: the dialog closes right after OK, and the question would go with it.
    let windows = crate::file_ops::windows::workspaces(cx);
    let active = cx.active_window();
    let Some(window) = windows
        .iter()
        .find(|(handle, _)| Some(*handle) == active)
        .or(windows.first())
        .map(|(handle, _)| *handle)
    else {
        return;
    };
    cx.spawn(async move |cx| {
        let answer = window
            .update(cx, |_, window, cx| {
                window.prompt(
                    gpui::PromptLevel::Warning,
                    "保存していない文書を残す機能をオフにしました。",
                    Some("今まで残した内容（次の起動で戻すためのコピー）を消しますか。"),
                    &["消す", "残す"],
                    cx,
                )
            })
            .ok()?;
        if let Ok(0) = answer.await {
            cx.update(clear_copies);
        }
        Some(())
    })
    .detach();
}
