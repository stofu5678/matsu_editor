//! Tag jumps through the tags file: the direct tag jump, the tag jump with a keyword, and making
//! the tags file (`F_DIRECT_TAGJUMP`, `F_TAGJUMP_KEYWORD`, `F_TAGS_MAKE`), and the tags file
//! search that `F_TAGJUMP` falls back to.
//!
//! Sakura: `Command_TagJumpByTagsFile`, `Command_TagJumpByTagsFileMsg`,
//! `Command_TagJumpByTagsFileKeyword`, `Sub_PreProcTagJumpByTagsFile` and `Command_TagsMake`
//! (`sakura_core/cmd/CViewCommander_TagJump.cpp`). The tags file is read by
//! [`ume_core::tag_file`], ctags is run by [`ume_core::tags_make`], the windows are [`list`] and
//! [`make`], and the jump itself, with the stack that `F_TAGJUMPBACK` uses, is
//! `file_ops::tag_jump`.
//!
//! The settings that Sakura keeps in its common settings are kept in [`TagJumpPrefs`] for now: the
//! two check boxes of the keyword dialog, the recent keywords, the two modes 「タグジャンプモード」
//! (what to do about the next tags file), and the options of 「タグファイルの作成」. They live while
//! the application runs; the common settings dialog is to read and write them.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use gpui::{App, AsyncWindowContext, Global, PromptLevel, WeakEntity};
use ume_core::tag_file::{TagItem, TagQuery, find_tags};
use ume_core::tag_jump::TagTarget;
use ume_core::tags_make::{
    CTAGS_COMMAND, Outcome, TagsMakeOptions, ctags_dirs, find_ctags, run_ctags,
};

use crate::file_ops::ask::tell;
use crate::file_ops::tag_jump::{jump_to_target, say_cannot_jump};
use crate::view::EditorView;

pub(crate) mod list;
pub(crate) mod make;
#[cfg(test)]
mod tests;

/// How many recent keywords are kept (`CRecentTagjumpKeyword`).
const MAX_RECENT: usize = 30;

/// What the tag jump functions remember (`m_sTagJump`, `m_nTagJumpMode`, `m_nTagJumpModeKeyword`,
/// `m_nTagsOpt`, `m_szTagsCmdLine`).
#[derive(Clone, Debug)]
pub(crate) struct TagJumpPrefs {
    /// 大文字小文字の同一視 of the keyword dialog.
    pub ignore_case: bool,
    /// 文字列の途中にマッチ of the keyword dialog.
    pub partial: bool,
    /// The keywords of the keyword dialog, the newest first.
    pub recent: Vec<String>,
    /// What the direct tag jump does about the next tags file (`CShareData`: 1, 「ヒットしたら次の
    /// ファイルを検索しない」).
    pub direct_mode: u8,
    /// The same for the keyword dialog (3, 「次のファイルを検索する」).
    pub keyword_mode: u8,
    /// The options of 「タグファイルの作成」.
    pub make: TagsMakeOptions,
    /// The folders to look for ctags in; the folder of the program and `PATH` if `None`.
    pub ctags_dirs: Option<Vec<PathBuf>>,
}

impl Default for TagJumpPrefs {
    fn default() -> Self {
        Self {
            ignore_case: false,
            partial: false,
            recent: Vec::new(),
            direct_mode: 1,
            keyword_mode: 3,
            make: TagsMakeOptions::default(),
            ctags_dirs: None,
        }
    }
}

impl Global for TagJumpPrefs {}

impl TagJumpPrefs {
    /// `AppendItem`: the newest first, without a copy of itself.
    pub(crate) fn remember_keyword(&mut self, keyword: &str) {
        if keyword.is_empty() {
            return;
        }
        self.recent.retain(|k| k != keyword);
        self.recent.insert(0, keyword.to_owned());
        self.recent.truncate(MAX_RECENT);
    }
}

pub(crate) fn prefs(cx: &App) -> TagJumpPrefs {
    cx.try_global::<TagJumpPrefs>().cloned().unwrap_or_default()
}

pub(crate) fn remember_prefs(cx: &mut App, change: impl FnOnce(&mut TagJumpPrefs)) {
    change(cx.default_global::<TagJumpPrefs>());
}

/// `Sub_PreProcTagJumpByTagsFile`: the file that the search starts from. A document without a
/// file is taken as a file in the current folder (Sakura gives it a made-up name); a Grep window
/// has none (its lines are jumped to by their own text).
fn base_file(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) -> Option<PathBuf> {
    let (path, grep) = view
        .read_with(cx, |view, app| {
            (
                view.document().read(app).path().map(Path::to_path_buf),
                view.is_grep_window(),
            )
        })
        .ok()?;
    match path {
        Some(path) => Some(path),
        None if grep => None,
        None => Some(std::env::current_dir().ok()?.join("dmy")),
    }
}

/// A tag as the place to jump to (`TagJumpSub(fileName, CMyPoint(0, fileLine))`).
fn target_of(item: &TagItem) -> TagTarget {
    TagTarget {
        path: item.path.clone(),
        line: Some(item.line),
        column: None,
    }
}

/// Opens the list and waits for the choice. `None` when nothing was chosen.
async fn ask_list(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    form: list::Form,
    document: PathBuf,
    keyword: String,
    first: Option<ume_core::tag_file::TagPage>,
) -> Option<TagItem> {
    let (sender, receiver) = async_channel::bounded(1);
    view.update_in(cx, |view, window, cx| {
        view.open_tag_list(form, document, &keyword, first, sender, window, cx)
    })
    .ok()?;
    receiver.recv().await.ok().flatten()
}

/// `Command_TagJumpByTagsFile`: jumps to the tag named by the word under the caret (or the
/// selection). One hit is jumped to; with several the list asks. `true` when it was handled:
/// jumped to, or asked and cancelled. `false` when there is nothing to jump to.
pub(crate) async fn jump_by_tags_file(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    close: bool,
) -> bool {
    // `GetCurrentTextForSearch(cmemKey, true, true)`
    let keyword = view
        .read_with(cx, |view, app| view.search_seed(app))
        .ok()
        .flatten()
        .unwrap_or_default();
    if keyword.is_empty() {
        return false;
    }
    let Some(document) = base_file(view, cx) else {
        return false;
    };
    let mode = cx.update(|_, app| prefs(app).direct_mode).unwrap_or(1);
    let search = {
        let (document, keyword) = (document.clone(), keyword.clone());
        cx.background_executor()
            .spawn(async move { find_tags(&document, &TagQuery::direct(&keyword, mode)) })
    };
    let page = search.await;
    let item = if page.matched > 1 {
        // `FindDirectTagJump` found several: the user chooses. Cancelling is done.
        match ask_list(view, cx, list::Form::Direct, document, keyword, Some(page)).await {
            Some(item) => item,
            None => return true,
        }
    } else {
        match page.items.into_iter().next() {
            Some(item) => item,
            None => return false,
        }
    };
    jump_to_target(view, cx, &target_of(&item), close).await
}

/// `F_DIRECT_TAGJUMP`: `Command_TagJumpByTagsFileMsg(true)`.
pub(crate) async fn direct_tag_jump(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    if !jump_by_tags_file(view, cx, false).await {
        say_cannot_jump(view, cx);
    }
}

/// `F_TAGJUMP_KEYWORD`: `Command_TagJumpByTagsFileKeyword`. `keyword` is the argument of the
/// macro; without it the dialog starts with the newest keyword.
pub(crate) async fn keyword_tag_jump(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    keyword: Option<String>,
) {
    let Some(document) = base_file(view, cx) else {
        say_cannot_jump(view, cx);
        return;
    };
    let keyword = match keyword {
        Some(keyword) => keyword,
        // `Combo_SetCurSel(hwndKey, 0)`
        None => cx
            .update(|_, app| prefs(app).recent.first().cloned())
            .ok()
            .flatten()
            .unwrap_or_default(),
    };
    // nothing chosen is "cancelled": done
    let Some(item) = ask_list(view, cx, list::Form::Keyword, document, keyword, None).await else {
        return;
    };
    jump_to_target(view, cx, &target_of(&item), false).await;
}

/// `STR_ERR_CEDITVIEW_CMD03`, said for a ctags that is not installed.
fn not_found_message() -> String {
    format!(
        "タグ作成コマンド実行は失敗しました。\n\n{CTAGS_COMMAND} が見つかりません。\n\n\
         ctags (Universal Ctags など) をインストールして、PATH の通ったフォルダーか \
         UME Editor と同じフォルダーに置いてください。UME Editor は ctags を同梱していません。"
    )
}

/// `F_TAGS_MAKE`: `Command_TagsMake`.
pub(crate) async fn make_tags(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    let folder = view
        .read_with(cx, |view, app| {
            view.document()
                .read(app)
                .path()
                .and_then(|p| p.parent())
                .map(Path::to_path_buf)
        })
        .ok()
        .flatten()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();
    let options = cx.update(|_, app| prefs(app).make).unwrap_or_default();
    let (sender, receiver) = async_channel::bounded(1);
    let cancel = Arc::new(AtomicBool::new(false));
    let opened = view.update_in(cx, {
        let cancel = Arc::clone(&cancel);
        move |view, window, cx| {
            view.open_tags_make(
                &folder.display().to_string(),
                &options,
                sender,
                cancel,
                window,
                cx,
            )
        }
    });
    if opened.is_err() {
        return;
    }
    let Some(choice) = receiver.recv().await.ok().flatten() else {
        return;
    };
    // `GetData` keeps the options whatever comes next.
    let options = choice.options.clone();
    cx.update(|_, app| remember_prefs(app, move |p| p.make = options))
        .ok();

    let dirs = cx
        .update(|_, app| prefs(app).ctags_dirs)
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            let program = std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf));
            ctags_dirs(program.as_deref(), std::env::var_os("PATH"))
        });
    let Some(ctags) = find_ctags(&dirs) else {
        close_tags_make(view, cx);
        tell(cx, PromptLevel::Warning, &not_found_message(), None).await;
        return;
    };
    let outcome = {
        let (folder, options, cancel) = (choice.folder.clone(), choice.options, cancel);
        cx.background_executor()
            .spawn(async move { run_ctags(&ctags, &folder, &options, &cancel) })
            .await
    };
    close_tags_make(view, cx);
    match outcome {
        // STR_ERR_CEDITVIEW_CMD07
        Outcome::Finished => {
            tell(
                cx,
                PromptLevel::Info,
                "タグファイルの作成が終了しました。",
                None,
            )
            .await;
        }
        // STR_ERR_CEDITVIEW_CMD06: whatever ctags says is taken as an error
        Outcome::Output(text) => {
            let message = format!("タグ作成コマンド実行は失敗しました。\n\n{text}");
            tell(cx, PromptLevel::Warning, &message, None).await;
        }
        // STR_ERR_CEDITVIEW_CMD04
        Outcome::CannotStart(error) => {
            let message = format!("タグ作成コマンド実行は失敗しました。\n\n{error}");
            tell(cx, PromptLevel::Warning, &message, None).await;
        }
        // Sakura says the job is finished here too; a stopped job is not.
        Outcome::Cancelled => {}
        Outcome::NotFound => {
            tell(cx, PromptLevel::Warning, &not_found_message(), None).await;
        }
    }
}

fn close_tags_make(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    view.update_in(cx, |view, window, cx| {
        if view.support.tags_make.take().is_some() {
            window.focus(&view.focus_handle, cx);
            cx.notify();
        }
    })
    .ok();
}
