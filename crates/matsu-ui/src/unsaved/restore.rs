//! Bringing the kept documents back at the start (ADR-0022, section 4).
//!
//! [`restore_all`] runs once, before the files of the command line are opened:
//!
//! 1. The folder `unsaved/` next to `matsu.ini` is opened (its lock is taken; if another process has it, nothing is
//!    restored and nothing is written). Temporary files of a write that was cut off are removed, and what has been
//!    in `rejected/` for 30 days is deleted.
//! 2. `session.ini` says which documents there are, and in which windows and tabs. The ids it lists whose file is
//!    gone are skipped. The files it does not list (a new document whose `session.ini` was not written yet when the
//!    process stopped) are restored too, as tabs at the end of the first window.
//! 3. A file that cannot be read goes to `rejected/`, and the user is told.
//! 4. Each document is made as it was: its text is the base of its history (Undo starts from here), it is
//!    modified, its encoding, BOM, line ending, type (if the user chose one), caret, view, bookmarks and untitled
//!    number are kept. A document with a file keeps the time stamp the file had, and a mark 「確認まだ」: the save
//!    asks before it overwrites a file that is not as it was (`file_ops::save`).
//! 5. The keeper takes the folder over, and each restored document keeps its copy under the same name, so that no
//!    second copy is made until it changes.
//! 6. The status bar of the front window says how many documents came back.

use std::io::Read as _;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::{AnyWindowHandle, App, Entity};
use matsu_config::unsaved::{DocFile, DocInfo, REJECTED_KEEP, Store, unsaved_dir};
use matsu_core::buffer::{Document, Eol, Pos, col_from_utf16};
use matsu_core::encoding::Encoding;
use matsu_core::file::TextFile;

use super::keeper::{Adopted, adopt_restored, settings_path};
use super::prefs::prefs;
use crate::document::{RestoredCheck, SharedDocument};
use crate::file_ops::windows::{editors, open_editor_window, workspace_of};
use crate::view::EditorView;

/// A kept document that was read.
struct Kept {
    id: String,
    info: DocInfo,
    text: String,
}

/// What the restore did.
#[derive(Debug, Default)]
pub(crate) struct Restored {
    /// How many documents came back.
    pub documents: usize,
    /// How many of them have a file that is not as it was.
    pub changed_files: usize,
    /// How many files could not be read and were moved to `rejected/`.
    pub rejected: usize,
}

impl Restored {
    /// The text for the status bar, or `None` when nothing happened.
    fn notice(&self) -> Option<String> {
        if self.documents == 0 && self.rejected == 0 {
            return None;
        }
        let mut text = String::new();
        if self.documents > 0 {
            text.push_str(&format!(
                "前回の未保存の文書を{}件戻しました。",
                self.documents
            ));
        }
        if self.changed_files > 0 {
            text.push_str(&format!(
                "{}件はディスク上のファイルが変わっています。",
                self.changed_files
            ));
        }
        if self.rejected > 0 {
            text.push_str(&format!(
                "{}件は読めなかったため rejected フォルダーに移しました。",
                self.rejected
            ));
        }
        Some(text)
    }
}

/// Brings the kept documents back. Returns what it did; `documents` is 0 when the feature is off, the folder is
/// not there, another process has it, or nothing was kept.
pub(crate) fn restore_all(cx: &mut App) -> Restored {
    if !prefs(cx).keep {
        return Restored::default();
    }
    let Some(ini) = settings_path(cx) else {
        return Restored::default();
    };
    let dir = unsaved_dir(&ini);
    // Nothing is made for a first start.
    if !dir.is_dir() {
        return Restored::default();
    }
    // Locked by another process, or not usable: the keeper says so when it starts. Nothing is restored.
    let Ok(store) = Store::open(&dir) else {
        return Restored::default();
    };
    let _ = store.remove_tmp();
    let _ = store.purge_rejected(SystemTime::now(), REJECTED_KEEP);
    let session = store.read_session();
    let scan = store.scan(&session).unwrap_or_default();

    // The windows: the listed ones with their tabs, then the documents that were not listed.
    let mut plan: Vec<(Vec<String>, usize)> = session
        .windows
        .iter()
        .map(|w| {
            let docs: Vec<String> = w
                .docs
                .iter()
                .filter(|id| !scan.dangling.contains(id))
                .cloned()
                .collect();
            (docs, w.active)
        })
        .filter(|(docs, _)| !docs.is_empty())
        .collect();
    // The index of the front window among the ones that are left.
    let front = session
        .windows
        .iter()
        .take(session.active_window)
        .filter(|w| w.docs.iter().any(|id| !scan.dangling.contains(id)))
        .count();
    if !scan.orphans.is_empty() {
        match plan.first_mut() {
            Some((docs, _)) => docs.extend(scan.orphans.iter().cloned()),
            None => plan.push((scan.orphans.clone(), 0)),
        }
    }

    let mut result = Restored::default();
    let mut adopted = Vec::new();
    let mut front_handle: Option<AnyWindowHandle> = None;
    let mut last_view: Option<Entity<EditorView>> = None;
    for (window_index, (ids, active)) in plan.iter().enumerate() {
        let mut handle: Option<AnyWindowHandle> = None;
        let mut tab_views: Vec<Entity<EditorView>> = Vec::new();
        for id in ids {
            let kept = match read_kept(&store, id) {
                Ok(kept) => kept,
                Err(()) => {
                    // Unreadable: kept in `rejected/`, not deleted.
                    let _ = store.reject(id);
                    result.rejected += 1;
                    continue;
                }
            };
            let (shared, changed) = make_document(kept.text, &kept.info);
            if changed {
                result.changed_files += 1;
            }
            let opened = match handle {
                None => open_first(cx, shared),
                Some(handle) => {
                    crate::tabs::flows::open_document(cx, None, handle, false, shared, None).ok()
                }
            };
            let Some((opened_handle, view)) = opened else {
                // The window could not be made: the copy stays where it is for the next start.
                continue;
            };
            handle.get_or_insert(opened_handle);
            apply_view_state(&view, &kept.info, cx);
            adopted.push(Adopted {
                view: view.clone(),
                id: kept.id,
            });
            result.documents += 1;
            tab_views.push(view.clone());
            last_view = Some(view);
        }
        if let Some(handle) = handle {
            // The tab that was in front.
            if let Some(view) = tab_views.get((*active).min(tab_views.len().saturating_sub(1)))
                && tab_views.len() > 1
            {
                activate_tab_of(cx, handle, view);
            }
            if window_index == front {
                front_handle = Some(handle);
            }
        }
    }
    if let Some(handle) = front_handle {
        crate::file_ops::windows::activate(cx, handle);
    }
    adopt_restored(cx, store, adopted);
    if let (Some(view), Some(text)) = (last_view, result.notice()) {
        view.update(cx, |view, cx| {
            view.notice = Some(text);
            cx.notify();
        });
    }
    result
}

/// Reads a kept document: its information and its text (UTF-8).
fn read_kept(store: &Store, id: &str) -> Result<Kept, ()> {
    let DocFile { info, mut text, .. } = store.open_doc(id).map_err(|_| ())?;
    let mut bytes = Vec::new();
    text.read_to_end(&mut bytes).map_err(|_| ())?;
    let text = String::from_utf8(bytes).map_err(|_| ())?;
    Ok(Kept {
        id: id.to_owned(),
        info,
        text,
    })
}

/// The document as it was, and whether its file is not as it was when the copy was made.
fn make_document(text: String, info: &DocInfo) -> (SharedDocument, bool) {
    let encoding = info
        .encoding
        .trim()
        .parse::<i32>()
        .ok()
        .and_then(Encoding::from_sakura_code)
        .unwrap_or(Encoding::Utf8);
    let eol = match info.line_ending.as_str() {
        "CRLF" => Eol::CrLf,
        "CR" => Eol::Cr,
        _ => Eol::Lf,
    };
    let ext_eol = matsu_commands::edit_options::get().ext_eol;
    // The characters that stand for bytes that were not valid in the encoding (U+10FF00..) are written as the
    // bytes they stand for by the encoder of the text that was typed or added, not by a copy of the original
    // bytes: such a text is added, the rest is the base of the document.
    let has_stand_in = text
        .chars()
        .any(|c| ('\u{10FF00}'..='\u{10FFFF}').contains(&c));
    let document = if has_stand_in {
        let mut document = Document::empty();
        document.set_ext_eol(ext_eol);
        let _ = document.insert(Pos::new(0, 0), &text);
        document
    } else {
        Document::from_utf8_with(text.into_bytes(), ext_eol).unwrap_or_else(|_| Document::empty())
    };
    let path = info.path.as_ref().map(PathBuf::from);
    let modified = info.file_modified_secs.and_then(|secs| {
        let secs = u64::try_from(secs).ok()?;
        let nanos = info.file_modified_nanos.unwrap_or(0);
        UNIX_EPOCH.checked_add(Duration::new(secs, nanos))
    });
    let check = path.as_ref().map(|_| RestoredCheck {
        modified,
        size: info.file_size,
    });
    let changed = path
        .as_deref()
        .zip(check)
        .is_some_and(|(path, check)| file_changed(path, check));
    let file = TextFile::restored(document, path, encoding, info.bom, eol);
    let mut shared = SharedDocument::restored(file, info.untitled_number, check);
    if shared.path().is_none() && shared.untitled_number().is_none() {
        shared.number_untitled();
    }
    (shared, changed)
}

/// Whether the file is not as it was: another time stamp or size. A file that is gone is not "changed": the
/// document is saved under its name as Sakura does for a name that does not exist yet.
pub(crate) fn file_changed(path: &std::path::Path, was: RestoredCheck) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let time_differs = was
        .modified
        .is_some_and(|t| meta.modified().ok() != Some(t));
    let size_differs = was.size.is_some_and(|s| s != meta.len());
    time_differs || size_differs
}

/// Opens the first document of a window as a window of its own.
fn open_first(
    cx: &mut App,
    shared: SharedDocument,
) -> Option<(AnyWindowHandle, Entity<EditorView>)> {
    let handle = open_editor_window(cx, shared, None).ok()?;
    let view = editors(cx)
        .into_iter()
        .rfind(|(h, _)| *h == handle)
        .map(|(_, view)| view)?;
    Some((handle, view))
}

/// Puts the caret, the view, the bookmarks and the chosen type back.
fn apply_view_state(view: &Entity<EditorView>, info: &DocInfo, cx: &mut App) {
    let document = view.read(cx).document().clone();
    if let Some(index) = info.doc_type.and_then(|i| usize::try_from(i).ok()) {
        document.update(cx, |shared, cx| {
            shared.set_type_locked(true);
            shared.announce_type(index, cx);
        });
    }
    let lines = document.read(cx).doc().line_count();
    document.update(cx, |shared, _| {
        let (model, _) = shared.parts_mut();
        for line in info
            .bookmarks
            .iter()
            .filter_map(|l| usize::try_from(*l).ok())
        {
            if line < lines {
                model.bookmarks_mut().set(line, true);
            }
        }
    });
    let line = usize::try_from(info.caret_line)
        .unwrap_or(0)
        .min(lines.saturating_sub(1));
    let text = document
        .read(cx)
        .doc()
        .line(line)
        .ok()
        .flatten()
        .map(|l| l.text)
        .unwrap_or_default();
    let col = col_from_utf16(&text, usize::try_from(info.caret_column).unwrap_or(0));
    view.update(cx, |view, cx| {
        view.scroll_top_to(isize::try_from(info.top_line).unwrap_or(0), cx);
        view.go_to(Pos::new(line, col), cx);
        cx.notify();
    });
}

/// Brings the tab of `view` to the front of its window.
fn activate_tab_of(cx: &mut App, handle: AnyWindowHandle, view: &Entity<EditorView>) {
    let Some(workspace) = workspace_of(cx, handle) else {
        return;
    };
    let Some(index) = workspace.read(cx).tab_of(view) else {
        return;
    };
    handle
        .update(cx, |_, window, app| {
            workspace.update(app, |ws, wcx| ws.activate_tab(index, window, wcx));
        })
        .ok();
}
