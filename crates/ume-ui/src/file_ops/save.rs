//! The save flow (`CDocFileOperation::FileSave`, `FileSaveAs`, `DoSaveFlow` in
//! `sakura_core/doc/CDocFileOperation.cpp`, and the questions of `CCodeChecker::OnCheckSave` in
//! `sakura_core/env/CCodeChecker.cpp`).
//!
//! 1. Find the path: the file's own, an explicit one from a macro, or the save dialog (always for
//!    a document without a file).
//!    A path from the save dialog is followed by the options dialog (encoding, BOM, line ending),
//!    unless a macro passed the encoding or the line ending (`save_as_dialog`).
//! 2. A plain save of an unmodified document writes nothing (`DoSaveFlow`, when
//!    `m_bEnableUnmodifiedOverwrite` is off, which is Sakura's default).
//! 3. If the line endings are mixed and the save does not convert them, ask whether to unify them
//!    to the current line ending (Sakura's default asks, `m_bChkEnterAtEnd`).
//! 4. If the encoding cannot write some characters, show the first place and ask: continue
//!    (they become `?`), stop, or go to the place.
//! 5. Write. A failure shows the error; the old file is as it was (ADR-0005).

use std::path::PathBuf;

use gpui::{AsyncWindowContext, PromptLevel, WeakEntity};
use ume_core::buffer::Eol;
use ume_core::encoding::Encoding;
use ume_core::file::{SaveOptions, no_progress};

use super::ask::{
    Answer, ask, save_error_message, tell, unify_line_endings_message, unmappable_message,
};
use super::save_as_dialog::ask_options;
use crate::view::EditorView;

/// What to save and how. The default is a plain save (`F_FILESAVE`).
#[derive(Clone, Debug)]
pub(crate) struct SaveRequest {
    /// Write to this file (a macro's argument) without the dialog.
    pub path: Option<PathBuf>,
    /// Ask for the file with the save dialog (`F_FILESAVEAS_DIALOG`).
    pub dialog: bool,
    /// Save in this encoding instead of the document's.
    pub encoding: Option<Encoding>,
    /// Convert every line ending to this one.
    pub eol: Option<Eol>,
    /// Ask for a name when the document has none. `F_FILESAVE_QUIET` does not.
    pub ask_name: bool,
}

impl Default for SaveRequest {
    fn default() -> Self {
        Self {
            path: None,
            dialog: false,
            encoding: None,
            eol: None,
            ask_name: true,
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

/// Shows the save dialog. `None` if the user cancelled.
async fn choose_path(
    cx: &mut AsyncWindowContext,
    current: Option<&std::path::Path>,
) -> Option<PathBuf> {
    let dir = current
        .and_then(|p| p.parent())
        .map(std::path::Path::to_owned)
        .filter(|d| d.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();
    // Sakura suggests *.txt for a document without a name (`SaveFileDialog`).
    let name = current.and_then(|p| p.file_name()).map_or_else(
        || "無題.txt".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    let receiver = cx
        .update(|_, app| app.prompt_for_new_path(&dir, Some(&name)))
        .ok()?;
    receiver.await.ok()?.ok()?
}

/// Runs the save flow on `view`.
pub(crate) async fn save(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    request: SaveRequest,
) -> SaveOutcome {
    let Ok((current, modified)) = view.read_with(cx, |view, app| {
        let shared = view.document().read(app);
        (
            shared.path().map(PathBuf::from),
            shared.model().is_modified(),
        )
    }) else {
        return SaveOutcome::Cancelled;
    };

    // 1. The path.
    let plain = request.path.is_none()
        && !request.dialog
        && request.encoding.is_none()
        && request.eol.is_none();
    let mut from_dialog = false;
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
        match choose_path(cx, current.as_deref()).await {
            Some(path) => {
                from_dialog = true;
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
    if plain && on_disk && !modified {
        view.update(cx, |view, cx| {
            view.notice = Some("変更がないので保存しなかった".to_owned());
            cx.notify();
        })
        .ok();
        return SaveOutcome::NotNeeded;
    }

    // The options: the document's own, then what the request changes.
    let Ok((mut options, current_format)) = view.read_with(cx, |view, app| {
        let mut options = SaveOptions::keep(view.document().read(app).file());
        if let Some(encoding) = request.encoding.filter(|e| *e != options.encoding) {
            // The BOM follows the default of the new encoding (`CDocFileOperation::FileSaveAs`).
            options.encoding = encoding;
            options.bom = encoding.bom_default_on();
        }
        if request.eol.is_some() {
            options.eol = request.eol;
        }
        let file = view.document().read(app).file();
        (options, (file.encoding, file.bom))
    }) else {
        return SaveOutcome::Cancelled;
    };

    // The options Sakura's save dialog has next to the name. Not asked when a macro gave the
    // encoding or the line ending.
    if from_dialog && request.encoding.is_none() && request.eol.is_none() {
        let Some(choice) = ask_options(view, cx, current_format, options.eol).await else {
            return SaveOutcome::Cancelled;
        };
        options.encoding = choice.encoding;
        options.bom = choice.bom;
        options.eol = choice.eol;
    }

    // 3. Mixed line endings.
    if options.eol.is_none() {
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
                    Answer::Cancel => return SaveOutcome::Cancelled,
                }
            }
            Some(Ok(false)) => {}
            Some(Err(error)) => return fail(view, cx, &path, &error).await,
            None => return SaveOutcome::Cancelled,
        }
    }

    // 4. Characters the encoding cannot write.
    let checked = view
        .update(cx, |view, cx| {
            let options = &options;
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
                Answer::No => return SaveOutcome::Cancelled,
                Answer::Cancel => {
                    if let Some(first) = report.first.first() {
                        let pos = first.pos;
                        view.update(cx, |view, cx| view.go_to(pos, cx)).ok();
                    }
                    return SaveOutcome::Cancelled;
                }
            }
        }
        Some(Ok(_)) => {}
        Some(Err(error)) => return fail(view, cx, &path, &error).await,
        None => return SaveOutcome::Cancelled,
    }

    // 5. Write.
    let target = path.clone();
    let written = view
        .update(cx, |view, cx| view.save_now(target, &options, cx))
        .ok();
    match written {
        Some(Ok(report)) => {
            let mut notice = format!("{} に保存した", path.display());
            if report.substituted > 0 {
                notice.push_str(&format!("（{} 文字を ? にした）", report.substituted));
            }
            view.update(cx, |view, cx| {
                view.notice = Some(notice);
                cx.notify();
            })
            .ok();
            SaveOutcome::Saved
        }
        Some(Err(error)) => fail(view, cx, &path, &error).await,
        None => SaveOutcome::Cancelled,
    }
}

/// Shows why the save failed.
async fn fail(
    _view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: &std::path::Path,
    error: &ume_core::file::FileError,
) -> SaveOutcome {
    let (message, detail) = save_error_message(path, error);
    tell(cx, PromptLevel::Critical, &message, Some(&detail)).await;
    SaveOutcome::Cancelled
}
