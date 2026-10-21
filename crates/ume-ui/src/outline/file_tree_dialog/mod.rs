//! 「ファイルツリー設定」 (`IDD_FILETREE`, `CDlgFileTree`): the dialog that the 設定 button of the
//! outline pane opens when the pane shows the file tree.
//!
//! Sakura: `sakura_core/outline/CDlgFileTree.cpp`; the layout, the texts and the access keys are
//! `IDD_FILETREE` and the `STR_FILETREE_*` strings of `sakura_core/sakura_rc.rc`; the button is
//! `IDC_BUTTON_SETTING` of `CDlgFuncList` (`sakura_core/outline/CDlgFuncList.cpp`).
//!
//! * [`model`] is the tree and what its buttons do to it.
//! * [`store`] is what OK writes: the settings in `ume.ini` (common or the type's, as
//!   `LoadFileTreeSetting` decided) and the project file, and the import and export files.
//! * `actions` are the buttons and `render` is the layout.
//!
//! The dialog is a panel over the view, like the other dialogs of this program, and it is modal:
//! it covers the view and takes the keys. OK writes the settings, closes it and makes the pane
//! read the tree again (`F_FILETREE` with `SHOW_RELOAD`); キャンセル changes nothing.
//!
//! Not done: the ヘルプ button (the help of Sakura is not shown by UME Editor yet; it is shown
//! gray as in the common settings).

mod actions;
pub(crate) mod model;
mod render;
pub(crate) mod store;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use gpui::{
    AppContext as _, Context, Entity, FocusHandle, Focusable, PromptLevel, ScrollHandle,
    Subscription, WeakEntity, Window,
};
use gpui_component::input::{InputEvent, InputState};
use ume_config::settings::file_tree::{FileTreeItem, ItemType, MAX_ITEMS};

use super::file_tree::source::{DialogSetting, Inputs, SettingFrom};
use super::pane::OutlinePane;
use crate::view::EditorView;
use model::Tree;

/// 種類: the three kinds of item (`IDC_RADIO_GREP`, `IDC_RADIO_FILE`, `IDC_RADIO_FOLDER`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Grep,
    File,
    Folder,
}

impl Kind {
    fn of(item_type: ItemType) -> Self {
        match item_type {
            ItemType::File => Self::File,
            ItemType::Folder => Self::Folder,
            // a number that Sakura does not define shows as the first radio button
            ItemType::Grep | ItemType::Other(_) => Self::Grep,
        }
    }

    fn item_type(self) -> ItemType {
        match self {
            Self::Grep => ItemType::Grep,
            Self::File => ItemType::File,
            Self::Folder => ItemType::Folder,
        }
    }
}

pub(crate) struct FileTreeDialog {
    view: WeakEntity<EditorView>,
    pane: WeakEntity<OutlinePane>,
    /// What the lookup of the settings used: the settings file as it was, the folders for the
    /// project file, the meta folders. 読み込み looks up the default project file with it.
    inputs: Inputs,
    /// `[Common]` or `[Types(N)]`: where the settings are written.
    section: String,
    /// `m_eFileTreeSettingLoadType`.
    loaded_from: SettingFrom,
    /// `m_szLoadProjectIni`: the project file the items came from. OK writes them back to it.
    pub(crate) loaded_project_ini: Option<PathBuf>,
    /// 各フォルダーの設定を読み込む (`IDC_CHECK_LOADINI`).
    pub(crate) project: bool,
    /// デフォルト設定ファイル名 (`IDC_EDIT_DEFINI`).
    pub(crate) default_ini: Entity<InputState>,
    pub(crate) tree: Tree,
    /// 種類.
    pub(crate) kind: Kind,
    /// パス (`IDC_EDIT_PATH`).
    pub(crate) path: Entity<InputState>,
    /// ラベル (`IDC_EDIT_LABEL`).
    pub(crate) label: Entity<InputState>,
    /// ファイル (`IDC_EDIT_FILE`).
    pub(crate) file: Entity<InputState>,
    /// 隠しファイル, 読み取り専用 and システムファイル.
    pub(crate) hidden: bool,
    pub(crate) read_only: bool,
    pub(crate) system: bool,
    /// The list of `▼` is open.
    path_menu: bool,
    /// A question is being asked (the dialog of the OS or the input box): the keys wait.
    busy: bool,
    focus: FocusHandle,
    tree_focus: FocusHandle,
    /// The scroll position of the tree, and the node that was last brought into view.
    tree_scroll: ScrollHandle,
    revealed: Option<model::Path>,
    _subscriptions: Vec<Subscription>,
}

impl FileTreeDialog {
    pub(crate) fn new(
        setting: DialogSetting,
        inputs: Inputs,
        view: WeakEntity<EditorView>,
        pane: WeakEntity<OutlinePane>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let field = |window: &mut Window, cx: &mut Context<Self>, name: &'static str| {
            cx.new(|cx| InputState::new(window, cx).placeholder(name))
        };
        let default_ini = field(window, cx, "デフォルト設定ファイル名");
        let path = field(window, cx, "パス");
        let label = field(window, cx, "ラベル");
        let file = field(window, cx, "ファイル");
        let mut subscriptions = Vec::new();
        for input in [&default_ini, &path, &label, &file] {
            // Enter in a box is the default button (`DEFPUSHBUTTON` OK)
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                |this: &mut Self, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.ok(window, cx);
                    }
                },
            ));
        }
        let mut dialog = Self {
            view,
            pane,
            inputs,
            section: setting.section.clone(),
            loaded_from: setting.loaded_from,
            loaded_project_ini: setting.loaded.project_ini.clone(),
            project: setting.project,
            default_ini,
            tree: Tree::default(),
            kind: Kind::Grep,
            path,
            label,
            file,
            hidden: true,
            read_only: false,
            system: false,
            path_menu: false,
            busy: false,
            focus: cx.focus_handle(),
            tree_focus: cx.focus_handle(),
            tree_scroll: ScrollHandle::new(),
            revealed: None,
            _subscriptions: subscriptions,
        };
        // `SetDataInit`
        let default = setting.default_project_ini.clone();
        dialog
            .default_ini
            .update(cx, |state, cx| state.set_value(default, window, cx));
        dialog.set_data(&setting.loaded.items, window, cx);
        dialog
    }

    /// `SetData`: shows the items as the tree, with the first one selected.
    pub(crate) fn set_data(
        &mut self,
        items: &[FileTreeItem],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tree = Tree::from_items(items);
        if self.saves_to_settings() && items.len() > MAX_ITEMS {
            self.say(&too_many(MAX_ITEMS), window, cx);
        }
        self.show_selected(window, cx);
    }

    /// The items go to the settings (and not to a project file): the tree holds at most
    /// [`MAX_ITEMS`] of them (`bSaveShareData`).
    pub(crate) fn saves_to_settings(&self) -> bool {
        self.loaded_project_ini.is_none()
    }

    /// `m_eFileTreeSettingLoadType`.
    pub(crate) fn loaded_from(&self) -> SettingFrom {
        self.loaded_from
    }

    /// `ChangeEnableAddInsert`: another item fits. (Sakura compares with `nMaxCount < nCount`,
    /// which lets a 21st item in, and OK then cuts it off with the message of the limit.)
    pub(crate) fn can_add(&self) -> bool {
        !self.saves_to_settings() || self.tree.count() < MAX_ITEMS
    }

    /// `SetDataItem`: the boxes show the selected item, or the blank item when nothing is selected
    /// (the patterns `*.*`).
    pub(crate) fn show_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let item = self.tree.selected_item().cloned();
        let item = item.unwrap_or_else(|| FileTreeItem {
            target_file: "*.*".to_owned(),
            ..FileTreeItem::default()
        });
        self.kind = Kind::of(item.item_type);
        for (input, text) in [
            (&self.path, item.target_path),
            (&self.label, item.label),
            (&self.file, item.target_file),
        ] {
            input.update(cx, |state, cx| state.set_value(text, window, cx));
        }
        self.hidden = item.ignore_hidden;
        self.read_only = item.ignore_read_only;
        self.system = item.ignore_system;
        cx.notify();
    }

    /// `GetDataItem`: the item the boxes describe. Only the boxes of the kind count; the others
    /// have the values of a new item.
    pub(crate) fn item_from_boxes(&self, cx: &gpui::App) -> FileTreeItem {
        let mut item = FileTreeItem {
            item_type: self.kind.item_type(),
            ..FileTreeItem::default()
        };
        if self.kind != Kind::Folder {
            item.target_path = self.path.read(cx).value().to_string();
        }
        item.label = self.label.read(cx).value().to_string();
        if self.kind == Kind::Grep {
            item.target_file = self.file.read(cx).value().to_string();
            item.ignore_hidden = self.hidden;
            item.ignore_read_only = self.read_only;
            item.ignore_system = self.system;
        }
        item
    }

    /// `ChangeEnableItemType`: the path boxes are for Grep and ファイル items, the pattern and
    /// the attributes for Grep items.
    pub(crate) fn path_enabled(&self) -> bool {
        self.kind != Kind::Folder
    }

    pub(crate) fn grep_enabled(&self) -> bool {
        self.kind == Kind::Grep
    }

    /// 更新 is not for a Folder item with items in it when it would become another kind (that
    /// would leave its children without a parent).
    pub(crate) fn can_update(&self) -> bool {
        self.tree.selected().is_some()
            && !(self.tree.selected_has_children() && self.kind != Kind::Folder)
    }

    /// Tells the user something (`InfoMessage`). Nobody waits for the answer.
    pub(crate) fn say(&self, message: &str, window: &mut Window, cx: &mut Context<Self>) {
        drop(window.prompt(PromptLevel::Info, message, None, &["OK"], cx));
    }

    pub(crate) fn focus_first(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.tree_focus.focus(window, cx);
    }

    /// Takes the dialog away from the view and gives the focus back to the text.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.support.file_tree_dialog = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    /// キャンセル(X).
    pub(crate) fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close(window, cx);
    }

    /// OK (`GetData`): writes the settings, and the project file when the items came from one,
    /// closes the dialog and makes the pane read the tree again.
    pub(crate) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let to_settings = self.saves_to_settings();
        let limit = if to_settings { MAX_ITEMS } else { 0 };
        let (items, complete) = self.tree.to_items(limit);
        if !complete {
            self.say(&too_many(MAX_ITEMS), window, cx);
        }
        let outcome = store::Outcome {
            section: self.section.clone(),
            project: self.project,
            project_ini: self.default_ini.read(cx).value().to_string(),
            items: to_settings.then(|| items.clone()),
        };
        // (Sakura writes the settings first and closes the dialog whatever happens to the file;
        // here a project file that cannot be written leaves the dialog open with the edited tree,
        // and nothing written, so that the user can try again or export the items elsewhere.)
        if let Some(path) = &self.loaded_project_ini
            && store::write_project_file(path, &items).is_err()
        {
            let message = crate::common_settings::transfer::cannot_export(path);
            self.say(&message, window, cx);
            return;
        }
        store::commit(cx, &outcome);
        self.close(window, cx);
        // `HandleCommand( F_FILETREE, SHOW_RELOAD )`
        self.pane.update(cx, |pane, cx| pane.reload(cx)).ok();
    }
}

/// `STR_FILETREE_MAXCOUNT`.
pub(crate) fn too_many(max: usize) -> String {
    format!("最大数({max})を超えたため切り捨てました")
}

impl Focusable for FileTreeDialog {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl EditorView {
    /// Opens the dialog of the file tree settings over the view and focuses it. A dialog that is
    /// open stays the only one.
    pub(crate) fn open_file_tree_dialog(
        &mut self,
        setting: DialogSetting,
        inputs: Inputs,
        pane: WeakEntity<OutlinePane>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.support.file_tree_dialog.is_some() {
            return;
        }
        let view = cx.entity().downgrade();
        let dialog = cx.new(|cx| FileTreeDialog::new(setting, inputs, view, pane, window, cx));
        dialog.update(cx, |dialog, cx| dialog.focus_first(window, cx));
        self.support.file_tree_dialog = Some(dialog);
        cx.notify();
    }
}
