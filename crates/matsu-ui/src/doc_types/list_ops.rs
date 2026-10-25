//! The buttons of the type list that change the list: 追加, 複製, 削除, ↑, ↓ and 初期化
//! (`CDlgTypeList::AddType`, `CopyType`, `DelType`, `UpType`, `DownType`, `InitializeType`,
//! `sakura_core/typeprop/CDlgTypeList.cpp`). The work on the settings file is in
//! `matsu_config::settings::type_list`.

use gpui::{App, Context, PromptLevel, Window};
use matsu_config::settings::type_list::{self, ListError};

use super::edit::edit_types;
use super::list::{TypeList, label};
use super::with_user;

/// `MAX_TYPES`: the most types the list has.
const MAX_TYPES: usize = matsu_config::settings::types::MAX_TYPES as usize;

/// The number of the type with this id in the settings file as it is now (the transaction reads the latest
/// file, which another process may have changed since this one loaded it).
fn id_index(profile: &matsu_config::Profile, id: i32) -> Result<usize, ListError> {
    matsu_config::settings::DocumentTypes::from_profile(profile)
        .iter()
        .position(|t| t.id == id)
        .ok_or(ListError::NoSuchType)
}

/// What a button does for the selected type (`LBN_SELCHANGE` of `CDlgTypeList::DispatchEvent`, `SetData`).
impl TypeList {
    /// 追加 and 複製: while there is room.
    pub(super) fn has_room(&self) -> bool {
        self.labels.len() < MAX_TYPES
    }

    /// ↑: from the third type on (not for 基本, and not past it).
    pub(super) fn can_move_up(&self) -> bool {
        self.selected > 1
    }

    /// ↓: not for 基本 and not for the last.
    pub(super) fn can_move_down(&self) -> bool {
        self.selected != 0 && self.selected + 1 < self.labels.len()
    }

    /// 削除: not for 基本.
    pub(super) fn can_delete(&self) -> bool {
        self.selected != 0
    }

    /// Shows the list as the settings have it, with type `select` chosen (`SetData(selIdx)`).
    pub(super) fn reload(&mut self, select: usize, cx: &mut Context<Self>) {
        let labels = with_user(cx, |user| user.document_types().iter().map(label).collect());
        self.labels = labels;
        self.selected = select.min(self.labels.len().saturating_sub(1));
        self.scroll.scroll_to_item(self.selected);
        cx.notify();
    }

    /// The list was changed from another window: shows the new list, with the same type selected.
    /// `before` are the ids of the types as the list was.
    pub(super) fn follow_the_list(&mut self, before: &[i32], cx: &mut Context<Self>) {
        let index = before
            .get(self.selected)
            .and_then(|&id| Self::index_of_id(cx, id))
            .unwrap_or(self.selected);
        self.reload(index, cx);
    }

    /// `InfoMessage`. Nobody waits for the answer.
    pub(super) fn say(&self, message: &str, window: &mut Window, cx: &mut Context<Self>) {
        drop(window.prompt(PromptLevel::Info, message, None, &["OK"], cx));
    }

    /// Why an operation did not work, when the user has something to do about it.
    pub(super) fn explain(&self, error: &ListError, window: &mut Window, cx: &mut Context<Self>) {
        let message = match error {
            ListError::ThirtyTypes => "タイプを 30 個にはできません。\
                 設定ファイルの nTypesCount=30 は 36 個として読むためです。"
                .to_owned(),
            ListError::Profile(e) => format!("設定ファイルに書けませんでした。\n\n{e}"),
            // The buttons are disabled in these cases.
            ListError::NoSuchType | ListError::Full | ListError::NotAllowed => return,
        };
        self.say(&message, window, cx);
    }

    /// `ConfirmMessage`: asks はい / いいえ, and runs `then` for はい.
    pub(super) fn confirm(
        &mut self,
        message: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        let answer = window.prompt(
            PromptLevel::Info,
            message,
            None,
            &["はい(Y)", "いいえ(N)"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(0) = answer.await {
                this.update_in(cx, |this, window, cx| then(this, window, cx))
                    .ok();
            }
        })
        .detach();
    }

    /// 追加 (`AddType`).
    pub(super) fn add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_room() {
            return;
        }
        let me = cx.entity_id();
        match edit_types(cx, me, type_list::add_type) {
            Ok(index) => self.reload(index, cx),
            Err(e) => self.explain(&e, window, cx),
        }
    }

    /// 複製 (`CopyType`).
    pub(super) fn copy(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.has_room() {
            return;
        }
        let selected = self.selected;
        let me = cx.entity_id();
        match edit_types(cx, me, |profile| type_list::copy_type(profile, selected)) {
            Ok(index) => self.reload(index, cx),
            Err(e) => self.explain(&e, window, cx),
        }
    }

    /// ↑ and ↓ (`UpType`, `DownType`).
    pub(super) fn move_selected(&mut self, up: bool, window: &mut Window, cx: &mut Context<Self>) {
        let allowed = if up {
            self.can_move_up()
        } else {
            self.can_move_down()
        };
        if !allowed {
            return;
        }
        let selected = self.selected;
        let me = cx.entity_id();
        match edit_types(cx, me, |profile| {
            type_list::move_type(profile, selected, up)
        }) {
            Ok(index) => self.reload(index, cx),
            Err(e) => self.explain(&e, window, cx),
        }
    }

    /// The id, the name and whether it has extensions of the selected type.
    fn selected_type(&self, cx: &App) -> Option<(i32, String, bool)> {
        with_user(cx, |user| {
            user.document_types()
                .get(self.selected)
                .map(|t| (t.id, t.type_name.clone(), !t.type_exts.is_empty()))
        })
    }

    /// The number of the type with this id now. The list may have changed while a question was open.
    fn index_of_id(cx: &App, id: i32) -> Option<usize> {
        with_user(cx, |user| {
            user.document_types().iter().position(|t| t.id == id)
        })
    }

    /// 削除 (`DelType`): asks first.
    pub(super) fn delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_delete() {
            return;
        }
        let Some((id, name, _)) = self.selected_type(cx) else {
            return;
        };
        self.confirm(
            &format!("{name} を削除します。 よろしいですか？"),
            window,
            cx,
            move |this, window, cx| this.delete_now(id, window, cx),
        );
    }

    /// The part of 削除 after the answer.
    pub(super) fn delete_now(&mut self, id: i32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = Self::index_of_id(cx, id) else {
            return;
        };
        let me = cx.entity_id();
        match edit_types(cx, me, |profile| {
            type_list::delete_type(profile, id_index(profile, id)?)
        }) {
            // `if (m_nTypesCount <= iDocType) iDocType = m_nTypesCount - 1`: `reload` limits it.
            Ok(()) => self.reload(index, cx),
            Err(e) => self.explain(&e, window, cx),
        }
    }

    /// 初期化 (`InitializeType`): asks first for 基本 and for a type with extensions.
    pub(super) fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, name, has_exts)) = self.selected_type(cx) else {
            return;
        };
        if has_exts || self.selected == 0 {
            self.confirm(
                &format!("{name} を初期化します。 よろしいですか？"),
                window,
                cx,
                move |this, window, cx| this.reset_now(id, window, cx),
            );
        } else {
            self.reset_now(id, window, cx);
        }
    }

    /// The part of 初期化 after the answer.
    pub(super) fn reset_now(&mut self, id: i32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = Self::index_of_id(cx, id) else {
            return;
        };
        let me = cx.entity_id();
        match edit_types(cx, me, |profile| {
            type_list::reset_type(profile, id_index(profile, id)?)
        }) {
            Ok(name) => {
                self.reload(index, cx);
                self.say(&format!("{name} を初期化しました。"), window, cx);
            }
            Err(e) => self.explain(&e, window, cx),
        }
    }
}
