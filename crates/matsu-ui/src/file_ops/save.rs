//! The save flow (`CDocFileOperation::FileSave`, `FileSaveAs`, `DoSaveFlow` in
//! `sakura_core/doc/CDocFileOperation.cpp`, and the questions of `CCodeChecker::OnCheckSave` in
//! `sakura_core/env/CCodeChecker.cpp`).
//!
//! 1. Find the path: the file's own, an explicit one from a macro, or the save dialog (always for
//!    a document without a file).
//!    The save dialog has the encoding, the BOM and the line ending next to the name
//!    (`file_dialog::save_fields`). A dialog that cannot show them (GPUI's, the fallback) is
//!    followed by the options dialog of ours (`save_as_dialog`). A macro that passed the encoding
//!    or the line ending skips both.
//! 2. A plain save of an unmodified document whose encoding and BOM are as they were read writes
//!    nothing (`DoSaveFlow`, `bChgCodeSet`, when
//!    `m_bEnableUnmodifiedOverwrite` is off, which is Sakura's default; the page ファイル of the
//!    common settings turns it on).
//!    In view mode the file itself is never overwritten (`CSaveAgent::OnCheckSave`).
//! 3. If the line endings are mixed and the save does not convert them, ask whether to unify them
//!    to the current line ending. Sakura's default asks; the type's 「保存時に改行コードの混在を警告する」
//!    (`m_bChkEnterAtEnd`, `TypeSettings::warn_mixed_eol`) turns the question off.
//! 4. If the encoding cannot write some characters, show the first place and ask: continue
//!    (they become `?`), stop, or go to the place. The macro `FileSaveAs` does not ask and
//!    writes the `?` (the real Sakura does so; the user, 2026-10-22).
//! 5. Make the backup of the file that is there, if the common settings say so (`backup`), then
//!    write. A failure shows the error; the old file is as it was (ADR-0005).

use std::path::PathBuf;

use gpui::{AsyncWindowContext, PromptLevel, WeakEntity};
use matsu_core::buffer::Eol;
use matsu_core::encoding::Encoding;
use matsu_core::file::{SaveOptions, no_progress};

use super::ask::{
    Answer, ask, save_error_message, tell, unify_line_endings_message, unmappable_message,
};
use super::save_as_dialog::{SaveChoice, ask_options};
use crate::file_dialog::save_fields::SaveFields;
use crate::file_dialog::{self, Ask};
use crate::plugin_events::fire_and_wait;
use crate::view::EditorView;
use matsu_config::plugins::def::Jack;

/// What to save and how. The default is a plain save (`F_FILESAVE`).
#[derive(Clone, Debug)]
pub(crate) struct SaveRequest {
    /// Write to this file (a macro's argument) without the dialog.
    pub path: Option<PathBuf>,
    /// Ask for the file with the save dialog (`F_FILESAVEAS_DIALOG`).
    pub dialog: bool,
    /// Save in this encoding instead of the document's.
    pub encoding: Option<Encoding>,
    /// The BOM to write when `encoding` differs from the document's. `None`: the default of the
    /// new encoding (`CDocFileOperation::FileSaveAs`).
    pub bom: Option<bool>,
    /// Convert every line ending to this one.
    pub eol: Option<Eol>,
    /// Ask for a name when the document has none. `F_FILESAVE_QUIET` does not.
    pub ask_name: bool,
    /// Ask whether to unify mixed line endings when the save does not convert them. The macro
    /// `FileSaveAs` does not (see `ui_commands/file/file_save_as.rs`).
    pub ask_mixed_eol: bool,
    /// Ask whether to continue when the encoding cannot write some characters. The macro
    /// `FileSaveAs` does not: it saves them as `?` (see `ui_commands/file/file_save_as.rs`).
    pub ask_unmappable: bool,
}

impl SaveRequest {
    /// `F_FILESAVE_QUIET`: a plain save that does not ask for a name.
    pub fn quiet() -> Self {
        Self {
            ask_name: false,
            ..Self::default()
        }
    }
}

impl Default for SaveRequest {
    fn default() -> Self {
        Self {
            path: None,
            dialog: false,
            encoding: None,
            bom: None,
            eol: None,
            ask_name: true,
            ask_mixed_eol: true,
            ask_unmappable: true,
        }
    }
}

/// How a save ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveOutcome {
    Saved,
    /// Nothing to write: the document is unmodified.
    NotNeeded,
    /// The user stopped it, or it failed. The document is as it was.
    Cancelled,
}

impl SaveOutcome {
    /// Whether the document is on disk as it is now.
    pub fn is_safe(self) -> bool {
        self != Self::Cancelled
    }
}

/// Shows the save dialog. `None` if the user cancelled. The dialog starts in the folder of the
/// `current` file, else where 「ファイルダイアログの初期位置」 says (`GetDlgInitialDir`), and suggests
/// `name`, else the name of the current file, else `無題_<date>_<time>` (`SaveFileDialog`).
///
/// `document_type` is the type of the document when the dialog is its 名前を付けて保存: the dialog
/// then has the file types of Sakura's (`file_dialog::save_types`). `None` for a file that is
/// not a document (the decoded data of `save_decoded`), which Sakura saves without them.
pub(crate) async fn choose_path(
    cx: &mut AsyncWindowContext,
    current: Option<&std::path::Path>,
    name: Option<&str>,
    document_type: Option<usize>,
) -> Option<PathBuf> {
    choose_path_and_options(cx, current, name, document_type, None)
        .await
        .map(|(path, _)| path)
}

/// [`choose_path`] whose dialog also has the encoding, the BOM and the line ending next to the
/// name when `fields` is given (Sakura's dialog). The second value is what the user chose there:
/// `None` when the dialog could not show them (GPUI's dialog, the fallback), and the caller asks
/// another way.
pub(crate) async fn choose_path_and_options(
    cx: &mut AsyncWindowContext,
    current: Option<&std::path::Path>,
    name: Option<&str>,
    document_type: Option<usize>,
    fields: Option<&SaveFields>,
) -> Option<(PathBuf, Option<SaveChoice>)> {
    let name = name.map(str::to_owned).unwrap_or_else(|| {
        current.and_then(|p| p.file_name()).map_or_else(
            || file_dialog::save_types::untitled_name(chrono::Local::now().naive_local()),
            |n| n.to_string_lossy().into_owned(),
        )
    });
    let answer = cx
        .update(|window, app| {
            let start = file_dialog::start_folder(app, current);
            let mut ask = Ask::save().start(start).name(Some(name));
            if let Some(index) = document_type {
                ask = ask.save_types(file_dialog::save_types::for_document(app, index, current));
            }
            if let Some(fields) = fields {
                ask = ask.save_fields(fields.clone()).recent(app);
            }
            file_dialog::ask_chosen(window, app, ask)
        })
        .ok()?;
    let mut chosen = answer.await.ok().flatten()?;
    let choice = fields.and_then(|fields| fields.choice(&chosen.extras));
    Some((chosen.paths.pop()?, choice))
}

/// Runs the save flow on `view`.
pub(crate) async fn save(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    request: SaveRequest,
) -> SaveOutcome {
    let generation = view
        .read_with(cx, |view, app| view.document().read(app).generation())
        .ok();
    let Ok((current, modified)) = view.read_with(cx, |view, app| {
        let shared = view.document().read(app);
        (
            shared.path().map(PathBuf::from),
            // a change of the encoding or the BOM is saved too (`CDocFile::IsChgCodeSet`)
            shared.model().is_modified() || shared.format_changed(),
        )
    }) else {
        return SaveOutcome::Cancelled;
    };

    // 0. A document that shows only the scanned start of its file (ADR-0020) would lose the rest.
    let partial = view
        .read_with(cx, |view, app| {
            view.document().read(app).partial_reason("保存")
        })
        .ok()
        .flatten();
    if let Some(why) = partial {
        tell(cx, PromptLevel::Critical, &format!("{why}。"), None).await;
        return SaveOutcome::Cancelled;
    }

    // 1. The path.
    let plain = request.path.is_none()
        && !request.dialog
        && request.encoding.is_none()
        && request.eol.is_none();
    let mut from_dialog = false;
    // What the user chose in the fields of the save dialog itself.
    let mut dialog_choice = None;
    let path = if let Some(path) = request.path.clone() {
        path
    } else if request.dialog || current.is_none() {
        if current.is_none() && !request.ask_name {
            return SaveOutcome::Cancelled;
        }
        // Another Save As is still asking for its options (a menu or a macro started this one).
        let busy = view
            .update(cx, |view, cx| {
                let busy = view.save_as_dialog.is_some();
                if busy {
                    view.notice = Some("別の名前を付けて保存の設定を選んでいる途中".to_owned());
                    cx.notify();
                }
                busy
            })
            .unwrap_or(true);
        if busy {
            return SaveOutcome::Cancelled;
        }
        let document_type = view.read_with(cx, |view, _| view.type_index()).ok();
        // The encoding, the BOM and the line ending go into the save dialog, unless a macro
        // gave the encoding or the line ending.
        let fields = if request.encoding.is_none() && request.eol.is_none() {
            view.read_with(cx, |view, app| {
                let file = view.document().read(app).file();
                SaveFields::new((file.encoding, file.bom), file.eol_on_save)
            })
            .ok()
        } else {
            None
        };
        match choose_path_and_options(cx, current.as_deref(), None, document_type, fields.as_ref())
            .await
        {
            Some((path, choice)) => {
                from_dialog = true;
                dialog_choice = choice;
                path
            }
            None => return SaveOutcome::Cancelled,
        }
    } else {
        current.clone().unwrap_or_default()
    };

    // 2. Nothing to write.
    // A document that was named but never written (a missing file that was opened) is saved
    // even when it is empty, so that the file exists afterwards.
    let on_disk = current.as_deref().is_some_and(std::path::Path::is_file);
    let overwrite_unmodified = view
        .read_with(cx, |_, app| {
            super::prefs::prefs(app).enable_unmodified_overwrite
        })
        .unwrap_or(false);
    if plain && on_disk && !modified && !overwrite_unmodified {
        view.update(cx, |view, cx| {
            view.notice = Some("変更がないので保存しなかった".to_owned());
            cx.notify();
        })
        .ok();
        return SaveOutcome::NotNeeded;
    }

    // View mode never overwrites the file it shows (`CSaveAgent::OnCheckSave`,
    // `STR_SAVEAGENT_VIEW_FILE`). Saving under another name is allowed.
    let view_mode = view
        .read_with(cx, |view, app| view.document().read(app).view_mode())
        .unwrap_or(false);
    if view_mode
        && current
            .as_deref()
            .is_some_and(|current| super::windows::same_file(current, &path))
    {
        tell(
            cx,
            PromptLevel::Critical,
            "ビューモードでは同一ファイルへの保存はできません。",
            None,
        )
        .await;
        return SaveOutcome::Cancelled;
    }

    // The rest of `CSaveAgent::OnCheckSave`: another window must not hold the file, and the file
    // must be writable now.
    if let Some(outcome) = check_target(view, cx, current.as_deref(), &path).await {
        return outcome;
    }

    // A document brought back from a copy of unsaved text (ADR-0022) has not looked at its file yet: if the file is
    // not as it was, the user decides before it is overwritten.
    if let Some(outcome) = confirm_restored_file(view, cx, current.as_deref(), &path).await {
        return outcome;
    }

    // The options: the document's own, then what the request changes.
    let Ok((mut options, current_format)) = view.read_with(cx, |view, app| {
        let mut options = SaveOptions::keep(view.document().read(app).file());
        if let Some(encoding) = request.encoding.filter(|e| *e != options.encoding) {
            // The BOM follows the default of the new encoding (`CDocFileOperation::FileSaveAs`),
            // unless the request says otherwise (the macro `FileSaveAs`).
            options.encoding = encoding;
            options.bom = request.bom.unwrap_or_else(|| encoding.bom_default_on());
        }
        if request.eol.is_some() {
            options.eol = request.eol;
        }
        let file = view.document().read(app).file();
        (options, (file.encoding, file.bom))
    }) else {
        return SaveOutcome::Cancelled;
    };

    // The options Sakura's save dialog has next to the name. A dialog that could not show them
    // (the fallback) is followed by ours. Not asked when a macro gave the encoding or the line
    // ending.
    if from_dialog && request.encoding.is_none() && request.eol.is_none() {
        let choice = match dialog_choice {
            Some(choice) => choice,
            None => match ask_options(view, cx, current_format, options.eol).await {
                Some(choice) => choice,
                None => return SaveOutcome::Cancelled,
            },
        };
        options.encoding = choice.encoding;
        options.bom = choice.bom;
        options.eol = choice.eol;
    }

    // 3 and 4. The questions about the content.
    if let Err(outcome) = confirm_content(view, cx, &path, &mut options, &request).await {
        return outcome;
    }

    // 4b. The backup of the file that is there (`CBackupAgent::OnPreBeforeSave`). The lock of the
    // file is let go of while it is copied and taken again after, whatever the answer.
    let backup_allowed = {
        let copies = path.exists()
            && cx
                .update(|_, app| super::backup::prefs(app).enabled)
                .unwrap_or(false);
        if copies {
            view.update(cx, |view, cx| {
                view.document().update(cx, |shared, _| shared.unlock_file())
            })
            .ok();
        }
        let allowed = super::backup::before_save(cx, &path).await;
        if copies {
            view.update(cx, |view, cx| {
                let share_mode = super::prefs::prefs(cx).file_share_mode;
                view.document()
                    .update(cx, |shared, _| shared.lock_file(share_mode))
            })
            .ok();
            super::lock::report_failure(view, cx).await;
        }
        allowed
    };
    if !backup_allowed {
        return SaveOutcome::Cancelled;
    }

    // 4c. The macro of 保存前 (`RunAutoMacro(nMacroOnSave, path)`), told the file to be written.
    // Sakura runs it after the questions; here they are asked again if it changed the text, so
    // that characters it added are not lost without a word.
    let version = view
        .read_with(cx, |view, app| view.document().read(app).version())
        .ok();
    crate::macros::run_before_save(view, cx, &path).await;
    let same_document = |view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext| {
        view.read_with(cx, |view, app| view.document().read(app).generation())
            .ok()
            == generation
    };
    if !same_document(view, cx) {
        return SaveOutcome::Cancelled;
    }
    // The plugins of DocumentBeforeSave (`DoSaveFlow`), after the macro. Under a name the document that
    // was is closed first (`!bOverwriteMode`). A plugin that fails or is slow does not stop the save.
    let named = from_dialog || request.path.is_some();
    // The window can be used while the plugins run: if its document was replaced since the save began
    // (the file was closed), this save is not for the new document.
    fire_and_wait(view, cx, Jack::DocumentBeforeSave).await;
    if !same_document(view, cx) {
        return SaveOutcome::Cancelled;
    }
    if named {
        fire_and_wait(view, cx, Jack::DocumentClose).await;
        if !same_document(view, cx) {
            return SaveOutcome::Cancelled;
        }
    }
    let changed = view
        .read_with(cx, |view, app| view.document().read(app).version())
        .ok()
        != version;
    if changed && let Err(outcome) = confirm_content(view, cx, &path, &mut options, &request).await
    {
        return outcome;
    }

    // 5. Write.
    let target = path.clone();
    let written = view
        .update(cx, |view, cx| view.save_now(target, &options, cx))
        .ok();
    match written {
        Some(Ok(report)) => {
            super::lock::report_failure(view, cx).await;
            let mut notice = format!("{} に保存した", path.display());
            if report.substituted > 0 {
                notice.push_str(&format!("（{} 文字を ? にした）", report.substituted));
            }
            view.update(cx, |view, cx| {
                view.notice = Some(notice);
                // Saved under another name: the new name may be of another document type.
                view.follow_file_name(cx);
                cx.notify();
            })
            .ok();
            view.update(cx, |view, cx| view.begin_edit_watch(cx)).ok();
            fire_and_wait(view, cx, Jack::DocumentAfterSave).await;
            if named {
                // `FileSaveAs` runs the macro of オープン後 after a save under a name, and then the
                // plugins of DocumentOpen.
                crate::macros::run_opened(view, cx).await;
                fire_and_wait(view, cx, Jack::DocumentOpen).await;
            }
            // Text the user typed while the plugins ran is not in the file: a caller that closes after the
            // save must not take the document for saved. What a plugin wrote does not count (the file is
            // written, as in Sakura).
            if super::close::user_changed_text(view, cx) {
                return SaveOutcome::Cancelled;
            }
            SaveOutcome::Saved
        }
        Some(Err(error)) => {
            super::lock::report_failure(view, cx).await;
            fail(view, cx, &path, &error).await
        }
        None => SaveOutcome::Cancelled,
    }
}

/// The question before a document that was brought back overwrites a file that has changed since the copy was
/// made. `None` to go on.
async fn confirm_restored_file(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    current: Option<&std::path::Path>,
    path: &std::path::Path,
) -> Option<SaveOutcome> {
    let check = view
        .read_with(cx, |view, app| view.document().read(app).restored_check())
        .ok()
        .flatten()?;
    // Only the file the document came from.
    if !current.is_some_and(|c| super::windows::same_file(c, path)) {
        return None;
    }
    if !crate::unsaved::restore::file_changed(path, check) {
        return None;
    }
    let message = format!(
        "{}\nこのファイルは外部のエディタ等で変更されています。",
        super::ask::document_title(Some(path), None)
    );
    let overwrite = super::ask::confirm(
        cx,
        &message,
        Some("前回の未保存の内容で上書きしますか?"),
        ["上書きする(Y)", "キャンセル"],
    )
    .await;
    if overwrite {
        None
    } else {
        Some(SaveOutcome::Cancelled)
    }
}

/// `CSaveAgent::OnCheckSave` after the view mode check: `Some` stops the save.
///
/// * Under another name, a file that another window has open is refused
///   (`STR_SAVEAGENT_OTHER`, `CShareData::IsPathOpened`).
/// * The file is opened for appending to see that it can be written, with the lock of this
///   window let go of for the moment and taken again after (`STR_SAVEAGENT_OTHER_APP`). A file
///   that is not there is made and deleted. On Windows another program that denies writing
///   refuses the open; on macOS and Linux an exclusive `flock` is tried too, which another
///   editor's lock refuses (`matsu_platform::file_lock::check_writable`).
async fn check_target(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    current: Option<&std::path::Path>,
    path: &std::path::Path,
) -> Option<SaveOutcome> {
    let same = current.is_some_and(|current| super::windows::same_file(current, path));
    if !same {
        let this = view.entity_id();
        let open_elsewhere = cx
            .update(|_, app| super::windows::find_open_except(app, path, Some(this)).is_some())
            .unwrap_or(false);
        if open_elsewhere {
            tell(
                cx,
                PromptLevel::Critical,
                &format!(
                    "'{}'\nファイルを保存できません。\n他のウィンドウで使用中です。",
                    path.display()
                ),
                None,
            )
            .await;
            return Some(SaveOutcome::Cancelled);
        }
    }
    let released = view
        .update(cx, |view, cx| {
            view.document()
                .update(cx, |shared, _| shared.release_lock_for(path))
        })
        .unwrap_or(false);
    let writable = matsu_platform::file_lock::check_writable(path);
    // The lock is taken again whatever the answer, without the message of a failed lock
    // (`DoFileLock(false)`).
    view.update(cx, |view, cx| {
        view.document().update(cx, |shared, _| {
            shared.restore_lock(released);
            shared.forget_lock_failure();
        });
    })
    .ok();
    if writable.is_err() {
        tell(
            cx,
            PromptLevel::Critical,
            &format!(
                "'{}'\nファイルを保存できません。\nパスが存在しないか、他のアプリケーションで使用されている可能性があります。",
                path.display()
            ),
            None,
        )
        .await;
        return Some(SaveOutcome::Cancelled);
    }
    None
}

/// Steps 3 and 4 of the flow: the questions about mixed line endings and about characters the
/// encoding cannot write. They are asked again when the macro of 保存前 changed the text.
async fn confirm_content(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: &std::path::Path,
    options: &mut SaveOptions,
    request: &SaveRequest,
) -> Result<(), SaveOutcome> {
    // 3. Mixed line endings, unless the document type turned the question off
    // (`m_bChkEnterAtEnd`, 「保存時に改行コードの混在を警告する」; `CCodeChecker::OnCheckSave`).
    let warn_mixed_eol = view
        .read_with(cx, |view, app| view.type_settings(app).warn_mixed_eol)
        .unwrap_or(true);
    if request.ask_mixed_eol && warn_mixed_eol && options.eol.is_none() {
        let mixed = view
            .update(cx, |view, cx| {
                view.document()
                    .update(cx, |shared, _| shared.has_mixed_line_endings())
            })
            .ok();
        match mixed {
            Some(Ok(true)) => {
                let name = view
                    .read_with(cx, |view, app| {
                        crate::status::eol_name(view.document().read(app).new_line()).to_owned()
                    })
                    .unwrap_or_default();
                let message = unify_line_endings_message(&name);
                match ask(
                    cx,
                    PromptLevel::Warning,
                    &message,
                    None,
                    ["統一する(Y)", "そのまま保存(N)", "キャンセル"],
                )
                .await
                {
                    Answer::Yes => {
                        options.eol = view
                            .read_with(cx, |view, app| view.document().read(app).new_line())
                            .ok();
                    }
                    Answer::No => {}
                    Answer::Cancel => return Err(SaveOutcome::Cancelled),
                }
            }
            Some(Ok(false)) => {}
            Some(Err(error)) => return Err(fail(view, cx, path, &error).await),
            None => return Err(SaveOutcome::Cancelled),
        }
    }

    // 4. Characters the encoding cannot write. A macro's `FileSaveAs` saves `?` without a
    // question: the oracle run of the real Sakura did so, although `CCodeChecker::OnCheckSave`
    // (sakura_core/env/CCodeChecker.cpp) has the question; the user decided to follow it
    // (2026-10-22). The window's save keeps asking.
    if !request.ask_unmappable {
        options.allow_unmappable = true;
        return Ok(());
    }
    let checked = view
        .update(cx, |view, cx| {
            let options = &*options;
            view.document()
                .update(cx, |shared, _| shared.check_save(options, &mut no_progress))
        })
        .ok();
    match checked {
        Some(Ok(report)) if report.total > 0 => {
            let (message, detail) =
                unmappable_message(options.encoding, report.first.first(), report.total);
            match ask(
                cx,
                PromptLevel::Warning,
                &message,
                Some(&detail),
                ["続行(Y)", "中断(N)", "該当位置に移動"],
            )
            .await
            {
                Answer::Yes => options.allow_unmappable = true,
                Answer::No => return Err(SaveOutcome::Cancelled),
                Answer::Cancel => {
                    if let Some(first) = report.first.first() {
                        let pos = first.pos;
                        view.update(cx, |view, cx| view.go_to(pos, cx)).ok();
                    }
                    return Err(SaveOutcome::Cancelled);
                }
            }
        }
        Some(Ok(_)) => {}
        Some(Err(error)) => return Err(fail(view, cx, path, &error).await),
        None => return Err(SaveOutcome::Cancelled),
    }
    Ok(())
}

/// `F_FILESAVEALL`: a quiet save in every editor window, one after the other.
///
/// Sakura posts `F_FILESAVE_QUIET` to every editor and does not wait for the results
/// (`Command_FILESAVEALL`, `sakura_core/cmd/CViewCommander_File.cpp`). A window without a file or
/// without changes does nothing, as for the quiet save; a window that has a question (mixed line
/// endings, a character the encoding cannot write) asks it in its own window.
pub(crate) async fn save_all(cx: &mut AsyncWindowContext) {
    let editors = cx
        .update(|_, app| super::windows::editors(app))
        .unwrap_or_default();
    for (handle, view) in editors {
        let weak = view.downgrade();
        let Ok(mut window_cx) = handle.update(cx, |_, window, app| window.to_async(app)) else {
            continue;
        };
        save(&weak, &mut window_cx, SaveRequest::quiet()).await;
    }
}

/// Shows why the save failed.
async fn fail(
    _view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: &std::path::Path,
    error: &matsu_core::file::FileError,
) -> SaveOutcome {
    let (message, detail) = save_error_message(path, error);
    tell(cx, PromptLevel::Critical, &message, Some(&detail)).await;
    SaveOutcome::Cancelled
}
