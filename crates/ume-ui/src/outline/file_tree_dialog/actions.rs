//! What the buttons of the dialog do (`CDlgFileTree::OnBnClicked`).

use std::path::{Path, PathBuf};

use gpui::{Context, PromptLevel, Window};
use ume_config::settings::file_tree::{FileTreeItem, ItemType};

use super::model::Path as NodePath;
use super::{FileTreeDialog, Kind, store};
use crate::common_settings::transfer;
use crate::file_dialog::{self, Ask, import_folder};
use crate::input_dialog::InputRequest;
use crate::outline::file_tree::source::{SettingFrom, read_default_project_file};
use crate::support::browse::path_for_box;

/// The items of the list of `▼` (`STR_FILETREE_MENU_*`) with the text each puts into the path.
/// The access keys are Sakura's. The names of the program's folders say `ume` where Sakura says
/// `sakura`; the text they insert is the meta folder, which keeps Sakura's name.
pub(crate) const PATH_MENU: [(&str, &str); 8] = [
    ("INIルート(I)", "<iniroot>"),
    ("マイ ドキュメント(Y)", "%MYDOC%"),
    ("マイ ミュージック(M)", "%MYMUSIC%"),
    ("マイ ビデオ(V)", "%MYVIDEO%"),
    ("デスクトップ(D)", "%DESKTOP%"),
    ("Temp(T)", "%TEMP%"),
    ("ume(S)", "%SAKURA%"),
    ("umeデータ(A)", "%SAKURADATA%"),
];

/// `STR_PROPCOMMAINMENU_DEL`.
const DELETE_QUESTION: &str = "選択している項目を下位項目ごと削除します。\nよろしいですか？";
/// `STR_DLGREPLC_STR`, `STR_FILETREE_REPLACE_PATH_FROM` and `STR_FILETREE_REPLACE_PATH_TO`.
const REPLACE_TITLE: &str = "文字列を指定してください。";
const REPLACE_FROM: &str = "置換元パスを指定してください";
const REPLACE_TO: &str = "置換先文字列を指定してください";
/// `_MAX_PATH - 1`: the longest text the boxes of the replace dialog take.
const REPLACE_MAX_LEN: usize = 259;

impl FileTreeDialog {
    // --- 種類 ---------------------------------------------------------------------------------

    /// A radio button of 種類 (`ChangeEnableItemType`).
    pub(crate) fn set_kind(&mut self, kind: Kind, cx: &mut Context<Self>) {
        self.kind = kind;
        cx.notify();
    }

    /// `IDC_CHECK_LOADINI`.
    pub(crate) fn set_project(&mut self, on: bool, cx: &mut Context<Self>) {
        self.project = on;
        cx.notify();
    }

    // --- the tree -----------------------------------------------------------------------------

    /// A click on a row (`TVN_SELCHANGED`): selects it, and the boxes show it.
    pub(crate) fn select_row(
        &mut self,
        path: &NodePath,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tree.select(path);
        self.show_selected(window, cx);
    }

    /// The `+` of a row.
    pub(crate) fn toggle_row(
        &mut self,
        path: &NodePath,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selected = self.tree.selected().cloned();
        self.tree.toggle(path);
        if self.tree.selected() != selected.as_ref() {
            self.show_selected(window, cx);
        }
        cx.notify();
    }

    /// The arrow keys in the tree: the selected row moves, or the row opens or closes.
    pub(crate) fn tree_key(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let rows = self.tree.rows();
        let at = self
            .tree
            .selected()
            .and_then(|path| rows.iter().position(|row| &row.path == path));
        let target = match (key, at) {
            ("down", Some(i)) => rows.get(i + 1).map(|r| r.path.clone()),
            ("down", None) | ("home", _) => rows.first().map(|r| r.path.clone()),
            ("up", Some(i)) if i > 0 => rows.get(i - 1).map(|r| r.path.clone()),
            ("end", _) => rows.last().map(|r| r.path.clone()),
            ("right", Some(i)) => {
                let row = &rows[i];
                if row.has_children && !row.expanded {
                    self.tree.toggle(&row.path.clone());
                    cx.notify();
                    return true;
                }
                rows.get(i + 1)
                    .filter(|next| next.depth > row.depth)
                    .map(|next| next.path.clone())
            }
            ("left", Some(i)) => {
                let row = &rows[i];
                if row.has_children && row.expanded {
                    self.tree.toggle(&row.path.clone());
                    cx.notify();
                    return true;
                }
                (row.path.len() > 1).then(|| row.path[..row.path.len() - 1].to_vec())
            }
            ("up" | "left" | "right", _) => None,
            _ => return false,
        };
        if let Some(path) = target {
            self.select_row(&path, window, cx);
        }
        true
    }

    // --- IDC_BUTTON_ADD, _INSERT, _INSERT_A, _UPDATE, _DELETE ----------------------------------

    /// `>>(B)`.
    pub(crate) fn add_item(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.can_add() {
            let item = self.item_from_boxes(cx);
            self.tree.add(item);
            self.show_selected(window, cx);
        }
    }

    /// `上⇒(Q)`.
    pub(crate) fn insert_above(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.can_add() {
            let item = self.item_from_boxes(cx);
            self.tree.insert_above(item);
            self.show_selected(window, cx);
        }
    }

    /// `下⇒(A)`.
    pub(crate) fn insert_below(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.can_add() {
            let item = self.item_from_boxes(cx);
            self.tree.insert_below(item);
            self.show_selected(window, cx);
        }
    }

    /// `更新(P)`.
    pub(crate) fn update_item(&mut self, cx: &mut Context<Self>) {
        if self.can_update() {
            let item = self.item_from_boxes(cx);
            self.tree.update(item);
            cx.notify();
        }
    }

    /// `削除(4)`: an item with items below it is deleted after the question.
    pub(crate) fn delete_item(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tree.selected().is_none() {
            return;
        }
        if !self.tree.selected_has_children() {
            self.delete_now(window, cx);
            return;
        }
        let answer = window.prompt(
            PromptLevel::Info,
            DELETE_QUESTION,
            None,
            &["OK", "キャンセル"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if answer.await == Ok(0) {
                this.update_in(cx, |this, window, cx| this.delete_now(window, cx))
                    .ok();
            }
        })
        .detach();
    }

    pub(crate) fn delete_now(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tree.delete() {
            self.show_selected(window, cx);
        }
    }

    // --- the moves ----------------------------------------------------------------------------

    /// `↑(U)`. The boxes keep what was typed (Sakura selects nothing new).
    pub(crate) fn move_up(&mut self, cx: &mut Context<Self>) {
        if self.tree.move_up() {
            cx.notify();
        }
    }

    /// `↓(D)`.
    pub(crate) fn move_down(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tree.move_down() {
            self.show_selected(window, cx);
        }
    }

    /// `→(R)`.
    pub(crate) fn move_right(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tree.move_right() {
            self.show_selected(window, cx);
        }
    }

    /// `←(L)`.
    pub(crate) fn move_left(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tree.move_left() {
            self.show_selected(window, cx);
        }
    }

    // --- the path -----------------------------------------------------------------------------

    /// `▼(3)`: opens or closes the list.
    pub(crate) fn toggle_path_menu(&mut self, cx: &mut Context<Self>) {
        if self.path_enabled() {
            self.path_menu = !self.path_menu;
            self.path_menu_at = 0;
            cx.notify();
        }
    }

    /// A key while the list is open (a menu of Windows takes the arrow keys, Enter and the letter
    /// of an entry): returns whether the list took it.
    pub(crate) fn path_menu_key(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let last = PATH_MENU.len() - 1;
        match key {
            "down" => self.path_menu_at = (self.path_menu_at + 1).min(last),
            "up" => self.path_menu_at = self.path_menu_at.saturating_sub(1),
            "home" => self.path_menu_at = 0,
            "end" => self.path_menu_at = last,
            "enter" => {
                let (_, text) = PATH_MENU[self.path_menu_at.min(last)];
                self.put_in_path(text, window, cx);
                // The box still gets this Enter as its own (`PressEnter` is OK): the list stays
                // open for that, and goes when the key has been dealt with.
                self.path_menu = true;
                cx.defer_in(window, |this, _, cx| {
                    this.path_menu = false;
                    cx.notify();
                });
                return true;
            }
            letter => {
                let mut chars = letter.chars();
                let (Some(letter), None) = (chars.next(), chars.next()) else {
                    return false;
                };
                let letter = letter.to_lowercase().next().unwrap_or(letter);
                match PATH_MENU
                    .iter()
                    .find(|(label, _)| menu_key_of(label) == Some(letter))
                {
                    Some((_, text)) => {
                        self.put_in_path(text, window, cx);
                        return true;
                    }
                    None => return false,
                }
            }
        }
        cx.notify();
        true
    }

    /// An entry of the list: its text replaces the selection of the path box
    /// (`EditCtl_ReplaceSel`).
    pub(crate) fn put_in_path(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let text = text.to_owned();
        self.path
            .update(cx, |state, cx| state.replace(text, window, cx));
        self.path_menu = false;
        self.path.update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }

    /// `...(2)` of the path: a folder for a Grep item, a file for a File item (the `%` of its name
    /// is doubled, because `%` begins a meta folder).
    pub(crate) fn browse_path(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.path_enabled() || self.busy {
            return;
        }
        let folder = self.kind == Kind::Grep;
        let text = self.path.read(cx).value().to_string();
        let start = if folder {
            // `SelectDir( .., szDir, szDir )`: the folder in the box
            let typed = PathBuf::from(&text);
            typed.is_dir().then_some(typed)
        } else {
            // `GetInidir`: the folder of the settings file
            file_dialog::settings_dir(cx)
        };
        let ask = if folder {
            Ask::folder()
                .start(start)
                .titled("検索するフォルダーを選んでください", "選択")
        } else {
            Ask::open(false).start(start)
        };
        let receiver = file_dialog::ask(window, cx, ask);
        self.busy = true;
        cx.spawn_in(window, async move |this, cx| {
            let chosen = receiver.await.ok().flatten();
            this.update_in(cx, |this, window, cx| {
                this.busy = false;
                if let Some(path) = chosen.and_then(|paths| paths.into_iter().next()) {
                    let mut text = path.to_string_lossy().into_owned();
                    if !folder {
                        text = text.replace('%', "%%");
                    }
                    this.path
                        .update(cx, |state, cx| state.set_value(text, window, cx));
                }
            })
            .ok();
        })
        .detach();
    }

    // --- ファイル挿入 ---------------------------------------------------------------------------

    /// `ファイル挿入(C)...`: File items for the files chosen (the dialog starts in the current
    /// folder, which is the folder of the document).
    pub(crate) fn add_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let start = self.inputs.base_dir.clone();
        let receiver = file_dialog::ask(window, cx, Ask::open(true).start(Some(start)));
        self.busy = true;
        cx.spawn_in(window, async move |this, cx| {
            let chosen = receiver.await.ok().flatten().unwrap_or_default();
            this.update_in(cx, |this, window, cx| {
                this.busy = false;
                this.insert_files(&chosen, window, cx);
            })
            .ok();
        })
        .detach();
    }

    /// The File items for the files (`CNativeW::Replace( L"%", L"%%" )` on the path; the label is
    /// the file name, `GetFileTitlePointer`).
    pub(crate) fn insert_files(
        &mut self,
        files: &[PathBuf],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if files.is_empty() {
            return;
        }
        let items = files
            .iter()
            .map(|file| FileTreeItem {
                item_type: ItemType::File,
                target_path: file.to_string_lossy().replace('%', "%%"),
                label: file
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                ..FileTreeItem::default()
            })
            .collect();
        self.tree.add_files(items);
        self.show_selected(window, cx);
    }

    // --- パスの一括置換 --------------------------------------------------------------------------

    /// `パスの一括置換(N)...`: asks for the text to look for and for its replacement, then
    /// replaces it in the path of every item. Cancelling the first question ends it; cancelling
    /// the second replaces nothing. The boxes show the selected item again after both.
    pub(crate) fn replace_paths(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let from = view.update(cx, |view, cx| {
            view.open_input_dialog(replace_request(REPLACE_FROM), window, cx)
        });
        self.busy = true;
        cx.spawn_in(window, async move |this, cx| {
            let from = from.await;
            let mut to = None;
            if from.is_some() {
                // (a second box is cancelled at once while the first is open: ask it afterwards)
                let asked = this
                    .update_in(cx, |this, window, cx| {
                        let view = this.view.upgrade()?;
                        Some(view.update(cx, |view, cx| {
                            view.open_input_dialog(replace_request(REPLACE_TO), window, cx)
                        }))
                    })
                    .ok()
                    .flatten();
                if let Some(asked) = asked {
                    to = asked.await;
                }
            }
            this.update_in(cx, |this, window, cx| {
                this.busy = false;
                if let Some(from) = &from {
                    if let Some(to) = &to {
                        this.tree.replace_paths(from, to);
                    }
                    this.show_selected(window, cx);
                }
                this.focus_first(window, cx);
            })
            .ok();
        })
        .detach();
    }

    // --- デフォルト設定ファイル名 ----------------------------------------------------------------

    /// `...(1)`: chooses the default project file (a file under the folder of the settings file is
    /// given relative to it, `CDlgOpenFile::SelectFile`).
    pub(crate) fn browse_default_ini(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let ini_dir = file_dialog::settings_dir(cx);
        let text = self.default_ini.read(cx).value().to_string();
        let typed = Path::new(&text);
        let full = match (&ini_dir, typed.is_relative()) {
            (Some(dir), true) => dir.join(typed),
            _ => typed.to_owned(),
        };
        let start = if text.is_empty() {
            ini_dir.clone()
        } else {
            full.parent().map(Path::to_owned)
        };
        let name = full
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|_| !text.is_empty());
        let receiver = file_dialog::ask(window, cx, Ask::open(false).start(start).name(name));
        self.busy = true;
        cx.spawn_in(window, async move |this, cx| {
            let chosen = receiver.await.ok().flatten();
            this.update_in(cx, |this, window, cx| {
                this.busy = false;
                if let Some(path) = chosen.and_then(|paths| paths.into_iter().next()) {
                    let text = path_for_box(&path, ini_dir.as_deref());
                    this.default_ini
                        .update(cx, |state, cx| state.set_value(text, window, cx));
                }
            })
            .ok();
        })
        .detach();
    }

    /// `読み込み(V)`: reads the default project file and shows its items; the items go back to that
    /// file at OK. Nothing is read when the box is empty or the file cannot be read.
    pub(crate) fn load_default_ini(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loaded_from() == SettingFrom::File {
            return;
        }
        let name = self.default_ini.read(cx).value().to_string();
        if name.is_empty() {
            return;
        }
        let Some(loaded) = read_default_project_file(&self.inputs, &name) else {
            return;
        };
        self.loaded_project_ini = loaded.project_ini;
        self.set_data(&loaded.items, window, cx);
    }

    // --- インポート, エクスポート -------------------------------------------------------------------

    /// `インポート(M)`: the items of a project file replace the items of the tree.
    pub(crate) fn import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let ask = import_folder::ask_for(cx, Ask::open(false), "*.ini");
        let receiver = file_dialog::ask(window, cx, ask);
        self.busy = true;
        cx.spawn_in(window, async move |this, cx| {
            let path = receiver
                .await
                .ok()
                .flatten()
                .and_then(|paths| paths.into_iter().next());
            this.update_in(cx, |this, window, cx| {
                this.busy = false;
                if let Some(path) = path {
                    import_folder::remember(cx, &path);
                    this.import_from(&path, window, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// The import of a file that was chosen.
    pub(crate) fn import_from(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let (current, _) = self.tree.to_items(0);
        match store::import_items(path, &current) {
            Ok(items) => self.set_data(&items, window, cx),
            Err(_) => self.say(&transfer::cannot_open(path), window, cx),
        }
    }

    /// `エクスポート(Y)`: the items of the tree go to a file.
    pub(crate) fn export(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let ask = import_folder::ask_for(cx, Ask::save(), "*.ini");
        let receiver = file_dialog::ask(window, cx, ask);
        self.busy = true;
        cx.spawn_in(window, async move |this, cx| {
            let path = receiver
                .await
                .ok()
                .flatten()
                .and_then(|paths| paths.into_iter().next());
            this.update_in(cx, |this, window, cx| {
                this.busy = false;
                if let Some(path) = path {
                    import_folder::remember(cx, &path);
                    this.export_to(&path, window, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// The export to a file that was chosen.
    pub(crate) fn export_to(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let (items, _) = self.tree.to_items(0);
        let message = match store::export_items(path, &items) {
            Ok(()) => transfer::exported(path),
            Err(_) => transfer::cannot_export(path),
        };
        self.say(&message, window, cx);
    }

    // --- the keys -----------------------------------------------------------------------------

    /// Alt with the access key of a label (`IsDialogMessage`): uses that control. Returns whether
    /// the key was one of the dialog's.
    pub(crate) fn use_access_key(
        &mut self,
        key: char,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = key.to_lowercase().next().unwrap_or(key);
        let focus = |input: &gpui::Entity<gpui_component::input::InputState>,
                     window: &mut Window,
                     cx: &mut Context<Self>| {
            input.update(cx, |state, cx| state.focus(window, cx));
        };
        match key {
            'i' => self.set_project(!self.project, cx),
            'j' => focus(&self.default_ini.clone(), window, cx),
            '1' => self.browse_default_ini(window, cx),
            'v' => self.load_default_ini(window, cx),
            'g' => self.set_kind(Kind::Grep, cx),
            'f' => self.set_kind(Kind::File, cx),
            '2' => self.set_kind(Kind::Folder, cx),
            't' if self.path_enabled() => focus(&self.path.clone(), window, cx),
            '.' => self.browse_path(window, cx),
            '3' => self.toggle_path_menu(cx),
            'e' => focus(&self.label.clone(), window, cx),
            'k' if self.grep_enabled() => focus(&self.file.clone(), window, cx),
            'w' if self.grep_enabled() => {
                self.hidden = !self.hidden;
                cx.notify();
            }
            'z' if self.grep_enabled() => {
                self.read_only = !self.read_only;
                cx.notify();
            }
            's' if self.grep_enabled() => {
                self.system = !self.system;
                cx.notify();
            }
            '4' => self.delete_item(window, cx),
            'q' => self.insert_above(window, cx),
            'a' => self.insert_below(window, cx),
            'b' => self.add_item(window, cx),
            'p' => self.update_item(cx),
            'c' => self.add_files(window, cx),
            'n' => self.replace_paths(window, cx),
            'u' => self.move_up(cx),
            'd' => self.move_down(window, cx),
            'r' => self.move_right(window, cx),
            'l' => self.move_left(window, cx),
            'm' => self.import(window, cx),
            'y' => self.export(window, cx),
            'o' => self.ok(window, cx),
            'x' => self.cancel(window, cx),
            _ => return false,
        }
        true
    }
}

/// The access key of an entry of the list: `マイ ドキュメント(Y)` is `y`.
pub(super) fn menu_key_of(label: &str) -> Option<char> {
    let (_, key) = label.strip_suffix(')')?.rsplit_once('(')?;
    let mut chars = key.chars();
    let (Some(key), None) = (chars.next(), chars.next()) else {
        return None;
    };
    Some(key.to_lowercase().next().unwrap_or(key))
}

/// A question of the replace (`CDlgInput1`).
fn replace_request(message: &str) -> InputRequest {
    InputRequest {
        title: REPLACE_TITLE.to_owned(),
        message: message.to_owned(),
        default: String::new(),
        max_len: REPLACE_MAX_LEN,
    }
}
