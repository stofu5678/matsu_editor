//! Changing the list of document types: the settings file is changed, and every window takes the
//! new list (`CDlgTypeList` sends `MYWM_CHANGESETTING` to all editors after each change,
//! `SendChangeSetting`, `SendChangeSettingType`, `SendChangeSettingType2`).

use gpui::{App, Context, EntityId};
use matsu_config::Profile;
use matsu_config::settings::type_list::ListError;

use super::with_user;
use crate::settings::UserSettings;
use crate::view::EditorView;

/// Applies `change` to the settings file as it is now (another window or process may have changed it),
/// writes the file, and brings every window up to date. A file that cannot be written (no place for
/// it, no right to write) leaves the change for this session, as the type settings do. When
/// `change` fails nothing is changed. The other lists of types that are open (in other windows) show the new list
/// and keep their type selected; `except` is the list that makes the change (it is being updated).
pub(crate) fn edit_types<R>(
    cx: &mut App,
    except: EntityId,
    change: impl FnOnce(&mut Profile) -> Result<R, ListError>,
) -> Result<R, ListError> {
    let before: Vec<i32> = with_user(cx, |user| {
        user.document_types().iter().map(|t| t.id).collect()
    });
    let mut change = Some(change);
    let mut outcome: Option<Result<R, ListError>> = None;
    let mut written: Option<Profile> = None;
    crate::history::write_settings(cx, |profile| {
        let Some(change) = change.take() else {
            return Ok(false);
        };
        match change(profile) {
            Ok(value) => {
                outcome = Some(Ok(value));
                written = Some(profile.clone());
                Ok(true)
            }
            Err(e) => {
                outcome = Some(Err(e));
                Ok(false)
            }
        }
    });
    let user = cx.try_global::<UserSettings>().cloned();
    if outcome.is_none()
        && let Some(change) = change.take()
    {
        // No settings file to write: the list of this session.
        let mut profile = user
            .as_ref()
            .and_then(|user| user.profile().cloned())
            .unwrap_or_default();
        match change(&mut profile) {
            Ok(value) => {
                outcome = Some(Ok(value));
                written = Some(profile);
            }
            Err(e) => outcome = Some(Err(e)),
        }
    }
    let outcome = outcome.unwrap_or(Err(ListError::NoSuchType));
    if outcome.is_ok()
        && let Some(profile) = written
    {
        let path = user.as_ref().and_then(crate::settings::history_file);
        cx.set_global(UserSettings::from_profile(profile, path));
        follow_the_list(cx, &before, except);
    }
    outcome
}

/// Every view takes the new list. `before` are the ids of the types as the list was.
fn follow_the_list(cx: &mut App, before: &[i32], except: EntityId) {
    for (_, workspace) in crate::file_ops::windows::workspaces(cx) {
        for view in workspace.read(cx).all_views() {
            view.update(cx, |view, cx| view.follow_type_list(before, cx));
            let list = view.read(cx).doc_type.list.clone();
            if let Some(list) = list.filter(|l| l.entity_id() != except) {
                list.update(cx, |list, cx| list.follow_the_list(before, cx));
            }
        }
    }
}

impl EditorView {
    /// The list of types changed. A document whose type the user chose keeps its type, found by the
    /// id (`CDocType::SetDocumentType`: the id is what stays when types are added, deleted or moved), and
    /// takes the type of its file name when that type is gone; the others take the type of their file
    /// name again (`CEditDoc::OnChangeSetting`). `before` are the ids of the types as the list was.
    ///
    /// The temporary changes (zoom, wrap) are dropped when the type of the document changes, and kept
    /// otherwise.
    pub(crate) fn follow_type_list(&mut self, before: &[i32], cx: &mut Context<Self>) {
        let current = self.prefs.settings.type_index;
        let old_id = before.get(current).copied();
        let locked = self.document.read(cx).type_locked();
        let name = self.document.read(cx).file_name();
        let (index, new_id, settings) = with_user(cx, |user| {
            let by_id = old_id
                .filter(|_| locked)
                .and_then(|id| user.document_types().iter().position(|t| t.id == id));
            let index = by_id.unwrap_or_else(|| user.type_index_for(name.as_deref()));
            let new_id = user.document_types().get(index).map(|t| t.id);
            (index, new_id, user.view_settings_of_type(index))
        });
        if index != current || new_id != old_id {
            self.apply_settings(settings, cx);
            if index != current {
                self.document
                    .update(cx, |shared, cx| shared.announce_type(index, cx));
            }
        } else {
            self.apply_changed_settings(settings, cx);
        }
    }
}
