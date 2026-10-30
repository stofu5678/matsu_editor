//! The document type (タイプ別設定) of the document in the view: choosing it, and changing it.
//!
//! Sakura gives a document the type of its file name when the file is opened and when it is
//! saved under another name (`CDocTypeManager::GetDocumentTypeOfPath`, `CEditDoc::OnChangeSetting`,
//! `sakura_core/doc/CEditDoc.cpp`), unless the user chose a type for it: `F_CHANGETYPE`
//! ("タイプ別設定一時適用", also the 一時適用 button of the list `F_TYPE_LIST`) locks the type
//! (`CDocType::LockDocumentType`) until the next file is loaded into the window.
//!
//! * `EditorView::apply_type_of_document` (in `view/settings.rs`) chooses by the file name and
//!   unlocks.
//! * `EditorView::change_type` is `Command_CHANGETYPE`.
//! * `EditorView::follow_file_name` is for a document that got another name (Save As).
//! * `list` is the dialog of `F_TYPE_LIST`.

use gpui::{App, Context, Entity};

use crate::settings::UserSettings;
use crate::view::EditorView;

mod edit;
pub(crate) mod list;
mod list_ops;
mod transfer;

/// What the view keeps of the document type besides the settings of the type (`ViewPrefs`). The
/// type itself, and whether the user chose it, belong to the document
/// ([`SharedDocument::type_locked`](crate::document::SharedDocument::type_locked)): all panes of
/// a document have the same type.
#[derive(Default)]
pub(crate) struct DocTypeState {
    /// The list of types (`F_TYPE_LIST`), while it is open.
    pub list: Option<Entity<list::TypeList>>,
}

/// Runs `f` on the settings of the application: matsu.ini, or the built-in types without one.
pub(crate) fn with_user<R>(cx: &App, f: impl FnOnce(&UserSettings) -> R) -> R {
    match cx.try_global::<UserSettings>() {
        Some(user) => f(user),
        None => f(&UserSettings::none()),
    }
}

impl EditorView {
    /// `Command_CHANGETYPE` (`sakura_core/cmd/CViewCommander_Settings.cpp`): applies document type
    /// `number - 1` to this document (0 keeps the type it has, and locks it). A number that is
    /// not a type does nothing. The temporary changes (zoom, wrap) are dropped, as for any change of
    /// type (`CEditDoc::OnChangeType`), and a new empty document gets the encoding and the line
    /// ending of the type.
    ///
    /// Returns whether a type was applied.
    pub(crate) fn change_type(&mut self, number: i64, cx: &mut Context<Self>) -> bool {
        let current = self.prefs.settings.type_index;
        let index = if number == 0 {
            current
        } else {
            match usize::try_from(number - 1) {
                Ok(i) => i,
                Err(_) => return false,
            }
        };
        let applied = with_user(cx, |user| {
            (index < user.document_types().len()).then(|| {
                (
                    user.view_settings_of_type(index),
                    user.new_document_defaults_of_type(index),
                )
            })
        });
        let Some((settings, defaults)) = applied else {
            return false;
        };
        self.apply_settings(settings, cx);
        self.document.update(cx, |shared, cx| {
            shared.set_type_locked(true);
            shared.announce_type(index, cx);
        });
        // `OnChangeType`: an untitled, unmodified, empty document takes the type's default
        // encoding and line ending.
        let shared = self.document.read(cx);
        let untouched = shared.path().is_none()
            && !shared.model().is_modified()
            && shared.doc().line_count() == 0;
        if untouched {
            self.document.update(cx, |shared, cx| {
                shared.set_new_document_defaults(&defaults);
                cx.notify();
            });
        }
        true
    }

    /// The document got another name (Save As): gives it the type of the new name, unless the
    /// user chose a type (`CEditDoc::OnChangeSetting` calls `SetDocumentType` without force). The
    /// settings are applied only when the type changes, so a plain save keeps the zoom and the
    /// wrap of the view.
    pub(crate) fn follow_file_name(&mut self, cx: &mut Context<Self>) {
        if self.document.read(cx).type_locked() {
            return;
        }
        let name = self.document.read(cx).file_name();
        let wanted = with_user(cx, |user| {
            let index = user.type_index_for(name.as_deref());
            (index != self.prefs.settings.type_index)
                .then(|| (index, user.view_settings_of_type(index)))
        });
        if let Some((index, settings)) = wanted {
            self.apply_settings(settings, cx);
            self.document
                .update(cx, |shared, cx| shared.announce_type(index, cx));
        }
    }

    /// The names of the document types with the extensions, as the list shows them
    /// (`CDlgTypeList::SetData`), and the index of the type of this document.
    pub(crate) fn type_labels(&self, cx: &App) -> (Vec<String>, usize) {
        let labels = with_user(cx, |user| {
            user.document_types().iter().map(list::label).collect()
        });
        (labels, self.prefs.settings.type_index)
    }

    /// The number of the type of this document.
    pub(crate) fn type_index(&self) -> usize {
        self.prefs.settings.type_index
    }

    /// Document type `changed` has new settings (the type settings dialog, OK): this view takes
    /// them if its document has that type, and a document that has no type of the user's choice
    /// gets the type of its file name again (the extensions may have changed). The temporary
    /// changes (zoom, wrap) are dropped when the type of the document changes, and kept otherwise
    /// unless they end (`apply_changed_settings`).
    pub(crate) fn refresh_type(&mut self, changed: usize, cx: &mut Context<Self>) {
        let current = self.prefs.settings.type_index;
        let locked = self.document.read(cx).type_locked();
        let name = self.document.read(cx).file_name();
        let wanted = if locked {
            current
        } else {
            with_user(cx, |user| user.type_index_for(name.as_deref()))
        };
        if wanted != current || wanted == changed {
            let settings = with_user(cx, |user| user.view_settings_of_type(wanted));
            if wanted == current {
                // The same type with new values: the temporary changes stay unless they end.
                self.apply_changed_settings(settings, cx);
            } else {
                self.apply_settings(settings, cx);
            }
        }
        if wanted != current {
            self.document
                .update(cx, |shared, cx| shared.announce_type(wanted, cx));
        }
    }

    /// Whether the user chose the type of this document (`F_CHANGETYPE`).
    pub fn type_is_locked(&self, cx: &App) -> bool {
        self.document.read(cx).type_locked()
    }

    /// Another pane of the document got a type: this pane takes it too (the document has one type).
    pub(crate) fn follow_document_type(&mut self, index: usize, cx: &mut Context<Self>) {
        if index == self.prefs.settings.type_index {
            return;
        }
        let settings = with_user(cx, |user| user.view_settings_of_type(index));
        self.apply_settings(settings, cx);
    }
}

/// Document type `index` has new settings: every view of every window takes them
/// (`MYWM_CHANGESETTING` with `PM_CHANGESETTING_TYPE` to all editors).
pub(crate) fn refresh_open_views(cx: &mut App, index: usize) {
    for (_, workspace) in crate::file_ops::windows::workspaces(cx) {
        for view in workspace.read(cx).all_views() {
            view.update(cx, |view, cx| view.refresh_type(index, cx));
        }
    }
}

#[cfg(test)]
mod keep_tests;
#[cfg(test)]
mod list_tests;
#[cfg(test)]
mod shared_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod transfer_tests;
