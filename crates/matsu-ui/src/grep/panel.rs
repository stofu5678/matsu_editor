//! The Grep dialog (`F_GREP_DIALOG`): Sakura's `IDD_GREP` (`sakura_core/sakura_rc.rc`,
//! `sakura_core/dlg/CDlgGrep.cpp`).
//!
//! Like the find and replace dialogs ([`crate::search::panel`]) it is a panel in the editor window,
//! not a second window. The boxes are the condition (条件), the files (ファイル), the folder
//! (フォルダー) and the excluded files and folders; the check boxes and the choices are Sakura's.
//! The values live in [`GrepSettings`]; OK checks them (`GrepSettings::check`, which is
//! `CDlgGrep::GetData`) and starts the search.
//!
//! The same panel is the dialog of Grep 置換 (`F_GREP_REPLACE_DLG`, `IDD_GREP_REPLACE`,
//! `sakura_core/dlg/CDlgGrepReplace.cpp`): it has the box 置換後 and the check boxes
//! クリップボードの内容で置換 and バックアップ作成 besides, and its button replaces.
//!
//! The boxes keep their history, as the combo boxes of Sakura do ([`crate::search::history`]): a
//! button beside each opens the list of the strings used before. 「編集中のテキストから検索」 (the
//! check box that searches the text of the window, see [`super::window::window_token`]) is
//! offered by Grep and greyed out for Grep 置換 (`CDlgGrepReplace::SetData`).
//!
//! Left out: the list of code pages (the nine encodings of Sakura are offered).

use crate::dialog_focus::KeepTabInside as _;
use std::path::Path;

use gpui::prelude::FluentBuilder as _;

use crate::a11y::Labelled as _;
use crate::input_select::{focus_selecting, input_field};
use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, ScrollHandle, SharedString, StatefulInteractiveElement,
    Styled, Subscription, WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::{Scrollbar, ScrollbarMode};
use gpui_component::{Disableable as _, Selectable as _, h_flex, v_flex};
use matsu_core::encoding::Encoding;

use super::format::{LineType, OutputStyle};
use super::keys::split_folders;
use super::settings::{DialogError, GrepSettings, browse_start, quote_folder};
use super::window::{THIS_DOCUMENT, clipboard_replacement, remember, remembered};
use crate::file_dialog::{self, Ask};
use crate::search::history::{self, GrepBoxes};
use crate::view::EditorView;

/// The encodings of the combo box, after 自動選択 (`CCodeTypesForCombobox`).
fn charset_choices() -> Vec<Option<Encoding>> {
    std::iter::once(None)
        .chain(Encoding::ALL.into_iter().map(Some))
        .collect()
}

/// The name in the combo box (`SCodeSet::m_sLong`; 自動選択 for none).
pub fn charset_label(charset: Option<Encoding>) -> &'static str {
    charset.map_or("自動選択", Encoding::combo_name)
}

/// The next choice of the encoding after `current`.
pub fn next_charset(current: Option<Encoding>) -> Option<Encoding> {
    let choices = charset_choices();
    let at = choices.iter().position(|c| *c == current).unwrap_or(0);
    choices[(at + 1) % choices.len()]
}

/// `STR_DLGREPLC_CLIPBOARD`.
pub(crate) const NO_CLIPBOARD: &str = "クリップボードに有効なデータがありません！";

/// A box of the dialog that has a history list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    /// 条件 (`IDC_COMBO_TEXT`): the search strings.
    Key,
    /// ファイル (`IDC_COMBO_FILE`).
    File,
    /// フォルダー (`IDC_COMBO_FOLDER`).
    Folder,
    /// 除外ファイル (`IDC_COMBO_EXCLUDE_FILE`).
    ExcludeFile,
    /// 除外フォルダー (`IDC_COMBO_EXCLUDE_FOLDER`).
    ExcludeFolder,
    /// 置換後 (`IDC_COMBO_TEXT2`): the replace strings.
    Replacement,
}

impl Field {
    const ALL: [Field; 6] = [
        Field::Key,
        Field::File,
        Field::Folder,
        Field::ExcludeFile,
        Field::ExcludeFolder,
        Field::Replacement,
    ];

    /// The name of the box and of its history button for a screen reader, and the element ids.
    fn names(self) -> (&'static str, &'static str, &'static str, &'static str) {
        match self {
            Field::Key => ("条件", "条件の履歴", "grep-key-history", "grep-key-toggle"),
            Field::File => (
                "ファイル",
                "ファイルの履歴",
                "grep-file-history",
                "grep-file-toggle",
            ),
            Field::Folder => (
                "フォルダー",
                "フォルダーの履歴",
                "grep-folder-history",
                "grep-folder-toggle",
            ),
            Field::ExcludeFile => (
                "除外ファイル",
                "除外ファイルの履歴",
                "grep-exclude-file-history",
                "grep-exclude-file-toggle",
            ),
            Field::ExcludeFolder => (
                "除外フォルダー",
                "除外フォルダーの履歴",
                "grep-exclude-folder-history",
                "grep-exclude-folder-toggle",
            ),
            Field::Replacement => (
                "置換後",
                "置換後の履歴",
                "grep-replacement-history",
                "grep-replacement-toggle",
            ),
        }
    }

    /// The strings of the list, newest first (`CDlgGrep::OnInitDialog` fills the combo boxes
    /// from `m_aSearchKeys`, `m_aGrepFiles` and the like).
    fn entries(self, cx: &App) -> Vec<String> {
        match self {
            Field::Key => history::search_keys(cx),
            Field::File => history::grep_files(cx),
            Field::Folder => history::grep_folders(cx),
            Field::ExcludeFile => history::exclude_files(cx),
            Field::ExcludeFolder => history::exclude_folders(cx),
            Field::Replacement => history::replace_keys(cx),
        }
    }
}

/// A history list that is open, with the entry that Enter takes.
#[derive(Clone, Copy, Debug)]
struct DropDown {
    field: Field,
    highlight: usize,
}

pub struct GrepPanel {
    editor: WeakEntity<EditorView>,
    /// The panel as a whole: Tab stays inside it.
    pub(crate) scope: FocusHandle,
    pub(crate) key: Entity<InputState>,
    pub(crate) file: Entity<InputState>,
    pub(crate) folder: Entity<InputState>,
    pub(crate) exclude_file: Entity<InputState>,
    pub(crate) exclude_folder: Entity<InputState>,
    /// 置換後, for Grep 置換.
    pub(crate) replacement: Entity<InputState>,
    /// Whether this is the dialog of Grep 置換.
    pub(crate) replace_mode: bool,
    /// The values besides the text of the boxes (the boxes are read when OK is pressed).
    pub(crate) settings: GrepSettings,
    /// The scroll position of the body; the scroll bar draws from it.
    pub(crate) body_scroll: ScrollHandle,
    /// The history list that is open.
    dropdown: Option<DropDown>,
    /// Scrolls the open list to the highlighted entry.
    list_scroll: ScrollHandle,
    /// The boxes of the files and folders as they were before 編集中のテキストから検索 was
    /// turned on, to put back when it is turned off (`SetDataFromThisText`).
    saved_boxes: Option<[String; 4]>,
    _subscriptions: Vec<Subscription>,
}

impl GrepPanel {
    fn new(
        editor: WeakEntity<EditorView>,
        settings: GrepSettings,
        replace_mode: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = |placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| input_field(window, cx).placeholder(placeholder))
        };
        let key = input("検索する文字列（空ならファイル名だけ探す）", window, cx);
        let file = input("*.*", window, cx);
        let folder = input("検索するフォルダー", window, cx);
        let exclude_file = input("除外するファイル", window, cx);
        let exclude_folder = input("除外するフォルダー", window, cx);
        let replacement = input("置換後の文字列", window, cx);
        let mut subscriptions = Vec::new();
        for state in [
            &key,
            &file,
            &folder,
            &exclude_file,
            &exclude_folder,
            &replacement,
        ] {
            subscriptions.push(cx.subscribe_in(
                state,
                window,
                |this: &mut Self, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        // an open list took the key already (`combo_key`)
                        this.ok(window, cx);
                    }
                },
            ));
        }
        let mut panel = Self {
            editor,
            scope: cx.focus_handle(),
            key,
            file,
            folder,
            exclude_file,
            exclude_folder,
            replacement,
            replace_mode,
            settings,
            body_scroll: ScrollHandle::new(),
            dropdown: None,
            list_scroll: ScrollHandle::new(),
            saved_boxes: None,
            _subscriptions: subscriptions,
        };
        panel.show_settings(window, cx);
        panel
    }

    /// Puts the values into the boxes.
    fn show_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let s = self.settings.clone();
        for (state, value) in [
            (&self.key, s.key),
            (&self.file, s.file),
            (&self.folder, s.folder),
            (&self.exclude_file, s.exclude_file),
            (&self.exclude_folder, s.exclude_folder),
            (&self.replacement, s.replacement),
        ] {
            state.update(cx, |state, cx| state.set_value(value, window, cx));
        }
    }

    /// Takes the text of the boxes into the values.
    fn read_boxes(&mut self, cx: &App) {
        let text = |state: &Entity<InputState>| state.read(cx).value().to_string();
        self.settings.key = text(&self.key);
        // The boxes of the files and folders show the document while the option is on: what the
        // user typed is kept in `settings` (`SetDataFromThisText`).
        if !self.settings.from_this_text {
            self.settings.file = text(&self.file);
            self.settings.folder = text(&self.folder);
            self.settings.exclude_file = text(&self.exclude_file);
            self.settings.exclude_folder = text(&self.exclude_folder);
        }
        self.settings.replacement = text(&self.replacement);
    }

    /// The check box 編集中のテキストから検索 (`SetDataFromThisText`): the boxes of the files and
    /// folders show 「(現在のドキュメント)」 and the excluded ones are emptied and greyed out; turned
    /// off, what they held comes back (a file box that was `:HWND:` becomes `*.*`).
    pub(crate) fn set_from_this_text(
        &mut self,
        on: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if on == self.settings.from_this_text {
            return;
        }
        self.dropdown = None;
        if on {
            self.read_boxes(cx);
            self.saved_boxes = Some([
                self.settings.file.clone(),
                self.settings.folder.clone(),
                self.settings.exclude_file.clone(),
                self.settings.exclude_folder.clone(),
            ]);
            self.settings.from_this_text = true;
            self.set_file_boxes(
                [THIS_DOCUMENT, THIS_DOCUMENT, "", ""].map(str::to_owned),
                window,
                cx,
            );
        } else {
            self.settings.from_this_text = false;
            let [mut file, folder, exclude_file, exclude_folder] =
                self.saved_boxes.take().unwrap_or_else(|| {
                    [
                        self.settings.file.clone(),
                        self.settings.folder.clone(),
                        self.settings.exclude_file.clone(),
                        self.settings.exclude_folder.clone(),
                    ]
                });
            if file.starts_with(super::window::WINDOW_PREFIX) {
                file = "*.*".to_owned();
            }
            self.settings.file.clone_from(&file);
            self.set_file_boxes([file, folder, exclude_file, exclude_folder], window, cx);
        }
        cx.notify();
    }

    fn set_file_boxes(&mut self, values: [String; 4], window: &mut Window, cx: &mut Context<Self>) {
        for (state, value) in [
            &self.file,
            &self.folder,
            &self.exclude_file,
            &self.exclude_folder,
        ]
        .into_iter()
        .zip(values)
        {
            state.update(cx, |state, cx| state.set_value(value, window, cx));
        }
    }

    pub fn focus_key(&self, window: &mut Window, cx: &mut Context<Self>) {
        focus_selecting(&self.key, window, cx);
    }

    /// The OK button (検索, 置換 for Grep 置換): checks the boxes, and starts the search.
    pub(crate) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.read_boxes(cx);
        self.dropdown = None;
        let document = self
            .editor
            .read_with(cx, |editor, _| editor.document().entity_id().as_u64())
            .ok();
        let checked = if let (true, Some(document)) = (self.settings.from_this_text, document) {
            self.settings.check_from_this_text(document)
        } else if self.replace_mode {
            // `CDlgGrepReplace::OnBnClicked`: pasting needs something on the clipboard.
            let clipboard = if self.settings.paste {
                let text = clipboard_replacement(cx);
                if text.is_none() {
                    self.settings.paste = false;
                    cx.notify();
                    self.show_error(
                        DialogError {
                            message: NO_CLIPBOARD.to_owned(),
                            regex: false,
                        },
                        window,
                        cx,
                    );
                    return;
                }
                text
            } else {
                None
            };
            self.settings.check_replace(clipboard)
        } else {
            self.settings.check()
        };
        let info = match checked {
            Ok(info) => info,
            Err(error) => {
                self.show_error(error, window, cx);
                return;
            }
        };
        let from_this_text = self.settings.from_this_text;
        if !from_this_text {
            // `GetData` writes the folders back as they are checked.
            self.show_settings(window, cx);
        }
        let settings = self.settings.clone();
        remember(cx, settings.clone());
        // The strings go into the histories (`CDlgGrep::GetData`; `CDlgGrepReplace::GetData` adds
        // the replacement). Not the boxes of the files and folders when the text of the window is
        // searched: they were not used.
        if !from_this_text {
            history::remember_grep_boxes(
                cx,
                &GrepBoxes {
                    file: &settings.file,
                    folder: &settings.folder,
                    exclude_file: &settings.exclude_file,
                    exclude_folder: &settings.exclude_folder,
                },
            );
        }
        if self.replace_mode {
            history::remember_replace(cx, &settings.replacement);
        }
        let replace = self.replace_mode;
        let Some(editor) = self.editor.upgrade() else {
            return;
        };
        editor.update(cx, |editor, cx| {
            if !replace {
                // What is recorded is the Grep, as `HandleCommand(F_GREP)` does after the dialog.
                // Sakura does not record a Grep 置換 (`GrepReplace` is not a key macro).
                let recorded = match document.filter(|_| from_this_text) {
                    Some(document) => settings.for_window(document).macro_args(),
                    None => settings.macro_args(),
                };
                editor.record_call("F_GREP", &recorded);
            }
            editor.close_grep_panel(window, cx);
            editor.start_grep(info, false, window, cx);
        });
    }

    fn field_input(&self, field: Field) -> &Entity<InputState> {
        match field {
            Field::Key => &self.key,
            Field::File => &self.file,
            Field::Folder => &self.folder,
            Field::ExcludeFile => &self.exclude_file,
            Field::ExcludeFolder => &self.exclude_folder,
            Field::Replacement => &self.replacement,
        }
    }

    /// Whether a box can be used now: the boxes of the files and folders are not while the text
    /// of the window is searched, nor 置換後 while the clipboard replaces.
    fn field_enabled(&self, field: Field) -> bool {
        match field {
            Field::Key => true,
            Field::Replacement => !self.settings.paste,
            _ => !self.settings.from_this_text,
        }
    }

    /// The box that has the focus.
    fn focused_field(&self, window: &Window, cx: &App) -> Option<Field> {
        Field::ALL.into_iter().find(|field| {
            self.field_input(*field)
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        })
    }

    /// Opens the history list of a box, or closes it. A box with no history has nothing to open.
    fn toggle_dropdown(&mut self, field: Field, cx: &mut Context<Self>) {
        if self.dropdown.is_some_and(|d| d.field == field) {
            self.dropdown = None;
        } else {
            let entries = field.entries(cx);
            let current = self.field_input(field).read(cx).value();
            self.dropdown = (self.field_enabled(field) && !entries.is_empty()).then(|| DropDown {
                field,
                highlight: entries
                    .iter()
                    .position(|e| e == current.as_str())
                    .unwrap_or(0),
            });
            if let Some(open) = self.dropdown {
                self.list_scroll.scroll_to_item(open.highlight);
            }
        }
        cx.notify();
    }

    /// Puts a string of the list in its box, closes the list and gives the focus back to the box.
    fn pick(&mut self, field: Field, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.dropdown = None;
        let input = self.field_input(field).clone();
        input.update(cx, |state, cx| {
            state.set_value(text, window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    /// Up and Down in a box whose list is closed go through the list, as in a combo box. From a
    /// string that is not in the list, both take the first.
    fn step(&mut self, field: Field, down: bool, window: &mut Window, cx: &mut Context<Self>) {
        let entries = field.entries(cx);
        if entries.is_empty() {
            return;
        }
        let current = self.field_input(field).read(cx).value();
        let next = match (entries.iter().position(|e| e == current.as_str()), down) {
            (Some(i), true) => (i + 1).min(entries.len() - 1),
            (Some(i), false) => i.saturating_sub(1),
            (None, _) => 0,
        };
        self.pick(field, entries[next].clone(), window, cx);
    }

    /// The keys of the combo boxes (as those of the find dialog): Alt+Down and F4 open or close
    /// the list; with it open Up, Down, Enter and Escape work in it; with it closed Up and Down go
    /// through the strings. Returns whether the key was used.
    fn combo_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let focused = self.focused_field(window, cx);
        if self.dropdown.is_some_and(|d| Some(d.field) != focused) {
            // The focus left the box: the list closes.
            self.dropdown = None;
            cx.notify();
        }
        let Some(field) = focused else { return false };
        let key = event.keystroke.key.as_str();
        let m = &event.keystroke.modifiers;
        let plain = !m.alt && !m.control && !m.platform && !m.shift;
        if key == "f4" || (key == "down" && m.alt) {
            self.toggle_dropdown(field, cx);
            return true;
        }
        let Some(open) = self.dropdown else {
            return match key {
                "down" | "up" if plain && self.field_enabled(field) => {
                    self.step(field, key == "down", window, cx);
                    true
                }
                _ => false,
            };
        };
        let entries = field.entries(cx);
        match key {
            "down" | "up" if plain && !entries.is_empty() => {
                let highlight = if key == "down" {
                    (open.highlight + 1).min(entries.len() - 1)
                } else {
                    open.highlight.saturating_sub(1)
                };
                self.dropdown = Some(DropDown { highlight, ..open });
                self.list_scroll.scroll_to_item(highlight);
                cx.notify();
            }
            "enter" if plain && open.highlight < entries.len() => {
                self.pick(field, entries[open.highlight].clone(), window, cx);
            }
            "escape" => {
                self.dropdown = None;
                cx.notify();
            }
            _ => return false,
        }
        true
    }

    /// A box with the button that opens its history, and the list under it while it is open.
    fn combo_box(&self, field: Field, cx: &mut Context<Self>) -> gpui::Div {
        let (name, toggle_name, id, toggle_id) = field.names();
        let enabled = self.field_enabled(field);
        let mut column = v_flex().gap_1().flex_1().child(
            h_flex()
                .gap_1()
                .child(
                    div().flex_1().child(
                        Input::new(self.field_input(field))
                            .aria_label(name)
                            .disabled(!enabled),
                    ),
                )
                .child(
                    Button::new(toggle_id)
                        .label("▼")
                        .accessibility_label(toggle_name)
                        .disabled(!enabled)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            // The button does not take the focus; the keys of the list go to
                            // the box that has it, so give it to this one.
                            let input = this.field_input(field).clone();
                            input.update(cx, |state, cx| state.focus(window, cx));
                            this.toggle_dropdown(field, cx);
                        })),
                ),
        );
        if let Some(open) = self.dropdown.filter(|d| d.field == field) {
            let entries = field.entries(cx).into_iter().enumerate().map(|(i, text)| {
                let picked = text.clone();
                div()
                    .id((id, i))
                    .px_2()
                    .truncate()
                    .when(i == open.highlight, |d| d.bg(rgb(0xcce4f7)))
                    .hover(|d| d.bg(rgb(0xe5f1fb)))
                    .child(SharedString::from(text))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.pick(field, picked.clone(), window, cx);
                    }))
            });
            column = column.child(
                div()
                    .id(id)
                    .max_h(px(150.))
                    .overflow_y_scroll()
                    .track_scroll(&self.list_scroll)
                    .border_1()
                    .border_color(rgb(0xb0b0b0))
                    .bg(rgb(0xffffff))
                    .children(entries),
            );
        }
        column
    }

    fn show_error(&self, error: DialogError, window: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| {
                editor.grep_dialog_error_box(&error, window, cx)
            })
            .ok();
    }

    /// キャンセル.
    pub(crate) fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.read_boxes(cx);
        remember(cx, self.settings.clone());
        self.editor
            .update(cx, |editor, cx| editor.close_grep_panel(window, cx))
            .ok();
    }

    /// 参照: chooses a folder. The dialog starts in the folder of the box, else in the current folder
    /// (`IDC_BUTTON_FOLDER`).
    fn browse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let start = browse_start(self.folder.read(cx).value().as_ref());
        let ask = Ask::folder()
            .start(start)
            .titled("検索するフォルダーを選んでください", "選択");
        let receiver = file_dialog::ask(window, cx, ask);
        cx.spawn_in(window, async move |this, cx| {
            let chosen = receiver.await.ok().flatten();
            if let Some(path) = chosen.and_then(|paths| paths.into_iter().next()) {
                this.update_in(cx, |this, window, cx| {
                    this.set_folder(&quote_folder(&path.to_string_lossy()), window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    fn set_folder(&mut self, folder: &str, window: &mut Window, cx: &mut Context<Self>) {
        let folder = folder.to_owned();
        self.folder
            .update(cx, |state, cx| state.set_value(folder, window, cx));
    }

    /// 現フォルダー: the folder of the file in the window, else the folder of the program
    /// (`IDC_BUTTON_CURRENTFOLDER`).
    fn current_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let from_file = self
            .editor
            .read_with(cx, |editor, cx| {
                editor
                    .document()
                    .read(cx)
                    .path()
                    .and_then(Path::parent)
                    .map(|p| p.to_string_lossy().into_owned())
            })
            .ok()
            .flatten();
        let folder = from_file.or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
        });
        if let Some(folder) = folder {
            self.set_folder(&quote_folder(&folder), window, cx);
        }
    }

    /// 上: the last folder of the box goes up one (`IDC_BUTTON_FOLDER_UP`).
    fn folder_up(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut folders = split_folders(self.folder.read(cx).value().as_ref());
        let Some(last) = folders.last_mut() else {
            return;
        };
        let Some(parent) = Path::new(last.as_str()).parent() else {
            return;
        };
        if parent.as_os_str().is_empty() {
            return;
        }
        *last = parent.to_string_lossy().into_owned();
        let joined = folders
            .iter()
            .map(|f| quote_folder(f))
            .collect::<Vec<_>>()
            .join(";");
        self.set_folder(&joined, window, cx);
    }

    fn change(&mut self, change: impl FnOnce(&mut GrepSettings), cx: &mut Context<Self>) {
        self.read_boxes(cx);
        change(&mut self.settings);
        cx.notify();
    }

    fn check_box(
        &self,
        id: &'static str,
        label: &'static str,
        checked: bool,
        change: impl Fn(&mut GrepSettings, bool) + 'static,
        cx: &mut Context<Self>,
    ) -> Checkbox {
        Checkbox::new(id)
            .labelled(label)
            .checked(checked)
            .on_click(cx.listener(move |this, value: &bool, _, cx| {
                let value = *value;
                this.change(|s| change(s, value), cx);
            }))
    }

    fn choice(
        &self,
        id: &'static str,
        label: &'static str,
        selected: bool,
        change: impl Fn(&mut GrepSettings) + 'static,
        cx: &mut Context<Self>,
    ) -> Button {
        Button::new(id)
            .labelled(label)
            .selected(selected)
            .on_click(cx.listener(move |this, _, _, cx| this.change(|s| change(s), cx)))
    }

    fn row(label: &'static str, content: impl IntoElement) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .child(div().w(px(110.)).child(label))
            .child(div().flex_1().child(content))
    }
}

impl Focusable for GrepPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.key.read(cx).focus_handle(cx)
    }
}

impl Render for GrepPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let s = self.settings.clone();
        let regex = s.options.regex;
        let result_only = s.style == OutputStyle::ResultOnly;
        // The boxes and buttons of the files and folders are not used while the text of the window
        // is searched (`SetDataFromThisText`).
        let this_text = s.from_this_text;
        let folder_buttons = h_flex()
            .gap_2()
            .child(self.combo_box(Field::Folder, cx))
            .child(
                Button::new("grep-browse")
                    .labelled("参照(O)")
                    .disabled(this_text)
                    .on_click(cx.listener(|this, _, window, cx| this.browse(window, cx))),
            )
            .child(
                Button::new("grep-up")
                    .labelled("上(U)")
                    .disabled(this_text)
                    .on_click(cx.listener(|this, _, window, cx| this.folder_up(window, cx))),
            )
            .child(
                Button::new("grep-current-folder")
                    .labelled("現フォルダー(J)")
                    .disabled(this_text)
                    .on_click(cx.listener(|this, _, window, cx| this.current_folder(window, cx))),
            );
        let paste = s.paste;
        let mut options = v_flex()
            .gap_1()
            // Sakura greys it out for a replacement (`CDlgGrepReplace::SetData`).
            .child(
                Checkbox::new("grep-this-text")
                    .labelled("編集中のテキストから検索(M)")
                    .checked(this_text)
                    .disabled(self.replace_mode)
                    .on_click(cx.listener(|this, value: &bool, window, cx| {
                        this.set_from_this_text(*value, window, cx);
                    })),
            )
            .child(
                self.check_box(
                    "grep-sub",
                    "サブフォルダーからも検索する(S)",
                    s.sub_folder,
                    |s, v| s.sub_folder = v,
                    cx,
                )
                .disabled(this_text),
            )
            .child(self.check_box(
                "grep-case",
                "英大文字と小文字を区別する(C)",
                s.options.match_case,
                |s, v| s.options.match_case = v,
                cx,
            ))
            // Sakura greys the word box out while 正規表現 is on.
            .child(
                self.check_box(
                    "grep-word",
                    "単語単位で探す(W)",
                    s.options.whole_word,
                    |s, v| s.options.whole_word = v,
                    cx,
                )
                .disabled(regex),
            )
            .child(self.check_box(
                "grep-regex",
                "正規表現(E)",
                s.options.regex,
                |s, v| s.options.regex = v,
                cx,
            ))
            .child(
                self.check_box(
                    "grep-except-regex",
                    "除外ファイルを正規表現で指定(X)",
                    s.except_file_regex,
                    |s, v| s.except_file_regex = v,
                    cx,
                )
                .disabled(this_text),
            )
            .child(self.check_box(
                "grep-default-folder",
                "フォルダーの初期値をカレントフォルダーにする(F)",
                s.default_folder,
                |s, v| s.default_folder = v,
                cx,
            ));
        if self.replace_mode {
            options = options
                .child(self.check_box(
                    "grep-paste",
                    "クリップボードの内容で置換(V)",
                    paste,
                    |s, v| s.paste = v,
                    cx,
                ))
                .child(self.check_box(
                    "grep-backup",
                    "バックアップ作成(B)",
                    s.backup,
                    |s, v| s.backup = v,
                    cx,
                ));
        }
        let output = h_flex()
            .gap_4()
            .child(
                v_flex()
                    .gap_1()
                    .child("結果出力")
                    .child(self.choice(
                        "grep-line",
                        "該当行(L)",
                        s.line_type == LineType::Line,
                        |s| s.line_type = LineType::Line,
                        cx,
                    ))
                    .child(self.choice(
                        "grep-part",
                        "該当部分(M)",
                        s.line_type == LineType::MatchPart,
                        |s| s.line_type = LineType::MatchPart,
                        cx,
                    ))
                    .child(self.choice(
                        "grep-nohit",
                        "否該当行(N)",
                        s.line_type == LineType::NoHitLine,
                        |s| s.line_type = LineType::NoHitLine,
                        cx,
                    )),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child("出力形式")
                    .child(self.choice(
                        "grep-normal",
                        "ノーマル(1)",
                        s.style == OutputStyle::Normal,
                        |s| s.style = OutputStyle::Normal,
                        cx,
                    ))
                    .child(self.choice(
                        "grep-per-file",
                        "ファイル毎(2)",
                        s.style == OutputStyle::PerFile,
                        |s| s.style = OutputStyle::PerFile,
                        cx,
                    ))
                    .child(self.choice(
                        "grep-result-only",
                        "結果のみ(3)",
                        s.style == OutputStyle::ResultOnly,
                        |s| s.style = OutputStyle::ResultOnly,
                        cx,
                    )),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(self.check_box(
                        "grep-file-only",
                        "ファイル毎最初のみ検索(H)",
                        s.file_only,
                        |s, v| s.file_only = v,
                        cx,
                    ))
                    // "結果のみ" has no folders to show (`CDlgGrep::OnBnClicked`).
                    .child(
                        self.check_box(
                            "grep-base-folder",
                            "ベースフォルダー表示(B)",
                            s.base_folder,
                            |s, v| s.base_folder = v,
                            cx,
                        )
                        .disabled(result_only),
                    )
                    .child(
                        self.check_box(
                            "grep-separate-folder",
                            "フォルダー毎に表示(D)",
                            s.separate_folder,
                            |s, v| s.separate_folder = v,
                            cx,
                        )
                        .disabled(result_only),
                    ),
            );
        let charset = h_flex()
            .gap_2()
            .items_center()
            .child("文字コードセット(S)")
            .child(
                Button::new("grep-charset")
                    .labelled(charset_label(s.charset))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.change(|s| s.charset = next_charset(s.charset), cx);
                    })),
            );
        let buttons = h_flex()
            .gap_2()
            .child(
                Button::new("grep-ok")
                    .labelled(if self.replace_mode {
                        "置換"
                    } else {
                        "検索"
                    })
                    .on_click(cx.listener(|this, _, window, cx| this.ok(window, cx))),
            )
            .child(
                Button::new("grep-cancel")
                    .labelled("キャンセル")
                    .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
            );
        // The panel is as tall as its rows, up to the height of the area between its margins; a
        // taller one scrolls. The wrapper has that definite height (top and bottom), the frame
        // limits itself to it, and the scroll bar is a sibling of the scrolling body, not its
        // child (the same as the settings pages: inside it, the bar's overlay made a body that
        // fits overflow by its padding).
        div()
            .absolute()
            .top(px(8.))
            .bottom(px(8.))
            .right(px(24.))
            .w(px(620.))
            .flex()
            .flex_col()
            .child(
                div()
                    .id("grep-panel")
                    .track_focus(&self.scope)
                    .keep_tab_inside(&self.scope)
                    .relative()
                    // mouse events stop here: a click on the panel must not move the caret
                    .occlude()
                    .max_h_full()
                    .flex()
                    .flex_col()
                    .bg(rgb(0xf6f6f6))
                    .text_color(rgb(0x1e1e1e))
                    .text_size(px(13.))
                    .border_1()
                    .border_color(rgb(0xb0b0b0))
                    .rounded_md()
                    .shadow_md()
                    // The combo box keys come first: Up, Down and Enter would otherwise reach the
                    // input.
                    .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        if this.combo_key(event, window, cx) {
                            cx.stop_propagation();
                        }
                    }))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        if event.keystroke.key == "escape" {
                            this.cancel(window, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .child(
                        div()
                            .id("grep-body")
                            .debug_selector(|| "grep-body".to_owned())
                            .flex_shrink(1.)
                            .min_h_0()
                            .p_2()
                            .gap_2()
                            .flex()
                            .flex_col()
                            .overflow_y_scroll()
                            .track_scroll(&self.body_scroll)
                            .child(div().font_weight(gpui::FontWeight::BOLD).child(
                                if self.replace_mode {
                                    "Grep置換"
                                } else {
                                    "Grep"
                                },
                            ))
                            .child(Self::row(
                                if self.replace_mode {
                                    "置換前(N)"
                                } else {
                                    "条件(N)"
                                },
                                self.combo_box(Field::Key, cx),
                            ))
                            .children(self.replace_mode.then(|| {
                                // The box is not used while the text of the clipboard replaces
                                // (`DlgItem_Enable(IDC_COMBO_TEXT2, ...)`).
                                Self::row("置換後(Z)", self.combo_box(Field::Replacement, cx))
                            }))
                            .child(Self::row("ファイル(I)", self.combo_box(Field::File, cx)))
                            .child(Self::row("フォルダー(D)", folder_buttons))
                            .child(Self::row(
                                "除外ファイル",
                                self.combo_box(Field::ExcludeFile, cx),
                            ))
                            .child(Self::row(
                                "除外フォルダー",
                                self.combo_box(Field::ExcludeFolder, cx),
                            ))
                            .child(options)
                            .child(charset)
                            .child(output)
                            .child(buttons),
                    )
                    // A dialog taller than the window scrolls; the bar shows that more is below. It
                    // is always drawn while the body overflows: the default fades out when idle.
                    .child(
                        div().absolute().inset_0().child(
                            Scrollbar::vertical(&self.body_scroll).mode(ScrollbarMode::Always),
                        ),
                    ),
            )
    }
}

impl EditorView {
    /// `F_GREP_DIALOG`: opens the dialog with the values the last Grep left, the selection or the
    /// word at the caret as the condition and the folder of the file in the window
    /// (`Command_GREP_DIALOG`, `CDlgGrep::OnInitDialog`).
    pub fn open_grep_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_grep_panel_as(false, window, cx);
    }

    /// `F_GREP_REPLACE_DLG`: opens the dialog of Grep 置換, in the same way
    /// (`Command_GREP_REPLACE_DLG`, `CDlgGrepReplace::DoModal`: クリップボードの内容で置換 starts off).
    pub fn open_grep_replace_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_grep_panel_as(true, window, cx);
    }

    fn open_grep_panel_as(&mut self, replace: bool, window: &mut Window, cx: &mut Context<Self>) {
        let mut settings = remembered(cx);
        settings.paste = false;
        // The check box starts off (`m_bFromThisText` is FALSE in a new dialog).
        settings.from_this_text = false;
        match self.dialog_seed(cx) {
            Some(seed) => settings.key = seed,
            // As the find dialog does: the newest search string, if it is news to this window
            // (`CControlTray`, `GetCurrentTextForSearchDlg`).
            None => {
                // The window's sequence is left alone: the dialog may be cancelled, and the
                // find dialog should still find the string new.
                if let Some((newest, _)) =
                    crate::search::history::newer_search_key(cx, self.search.key_sequence())
                {
                    settings.key = newest;
                }
            }
        }
        let path = self.document.read(cx).path().map(Path::to_path_buf);
        settings.offer_folder(path.as_deref());
        self.show_grep_panel(settings, replace, window, cx);
    }

    /// `-GREPDLG`: opens the dialog with `settings` as they are (the conditions of the command
    /// line, `CNormalProcess::InitializeProcess`).
    pub(crate) fn open_grep_panel_with(
        &mut self,
        settings: GrepSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_grep_panel(settings, false, window, cx);
    }

    fn show_grep_panel(
        &mut self,
        settings: GrepSettings,
        replace: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The search panel is for another dialog; only one of them is open.
        self.close_search_panel(window, cx);
        match &self.grep.panel {
            Some(panel) => {
                panel.update(cx, |panel, cx| {
                    panel.replace_mode = replace;
                    panel.dropdown = None;
                    panel.saved_boxes = None;
                    panel.settings = settings;
                    panel.show_settings(window, cx);
                    panel.focus_key(window, cx);
                    cx.notify();
                });
            }
            None => {
                let editor = cx.weak_entity();
                let panel = cx.new(|cx| GrepPanel::new(editor, settings, replace, window, cx));
                panel.update(cx, |panel, cx| panel.focus_key(window, cx));
                self.grep.panel = Some(panel);
            }
        }
        cx.notify();
    }

    /// Closes the dialog and gives the focus back to the text.
    pub fn close_grep_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.grep.panel.take().is_some() {
            window.focus(&self.focus_handle, cx);
            cx.notify();
        }
    }

    /// Whether the dialog is open.
    pub fn grep_panel_open(&self) -> bool {
        self.grep.panel.is_some()
    }
}
