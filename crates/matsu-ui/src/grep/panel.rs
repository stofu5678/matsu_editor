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
use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;

use crate::a11y::Labelled as _;
use crate::access_keys::{Entry, Found, OnAccessKey as _, pick};
use crate::combo_rows::{arrow_row, row_of};
use crate::input_select::{focus_selecting, input_field};
use crate::radio_group::{Group, Ring};
use crate::support::help::OnHelpKey as _;
use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, ScrollHandle, SharedString, StatefulInteractiveElement,
    Styled, Subscription, WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::{Scrollbar, ScrollbarMode};
use gpui_component::{Disableable as _, h_flex, v_flex};
use matsu_core::encoding::Encoding;

use super::format::{LineType, OutputStyle};
use super::keys::split_folders;
use super::settings::{
    DEFAULT_EXCLUDE_FILE, DEFAULT_EXCLUDE_FOLDER, DialogError, GrepSettings, browse_start,
    quote_folder,
};
use super::window::{THIS_DOCUMENT, clipboard_replacement, remember, remembered};
use crate::file_dialog::{self, Ask};
use crate::search::history::{self, GrepBoxes};
use crate::view::EditorView;

/// The labels with their access keys (Alt+letter, [`GrepPanel::use_access_key`]). The letters are
/// Sakura's (`IDD_GREP` and `IDD_GREP_REPLACE` in `sakura_core/sakura_rc.rc`), so that no two controls
/// of one dialog share one.
const KEY_LABEL: &str = "条件(N)";
const REPLACE_KEY_LABEL: &str = "置換前(N)";
const REPLACEMENT_LABEL: &str = "置換後(Z)";
const FILE_LABEL: &str = "ファイル(I)";
const FOLDER_LABEL: &str = "フォルダー(L)";
const EXCLUDE_FILE_LABEL: &str = "除外ファイル(J)";
const EXCLUDE_FOLDER_LABEL: &str = "除外フォルダー(K)";
const BROWSE: &str = "参照(O)";
const UP: &str = "上(U)";
const CURRENT_FOLDER: &str = "現フォルダー(G)";
const THIS_TEXT: &str = "編集中のテキストから検索(M)";
const SUB_FOLDER: &str = "サブフォルダーからも検索する(S)";
const MATCH_CASE: &str = "英大文字と小文字を区別する(C)";
const WHOLE_WORD: &str = "単語単位で探す(W)";
const REGEX: &str = "正規表現(E)";
const EXCEPT_REGEX: &str = "除外ファイルを正規表現で指定(Q)";
const DEFAULT_FOLDER: &str = "フォルダーの初期値をカレントフォルダーにする(D)";
const PASTE: &str = "クリップボードの内容で置換(V)";
const BACKUP: &str = "バックアップ作成(B)";
const LINE: &str = "該当行(1)";
const PART: &str = "該当部分(2)";
const NO_HIT: &str = "否該当行(3)";
const NORMAL: &str = "ノーマル(4)";
const PER_FILE: &str = "ファイル毎(5)";
const RESULT_ONLY: &str = "結果のみ(6)";
const FILE_ONLY: &str = "ファイル毎最初のみ検索(7)";
const SEPARATE_FOLDER: &str = "フォルダー毎に表示(8)";
const BASE_FOLDER: &str = "ベースフォルダー表示(9)";
const CHARSET_LABEL: &str = "文字コードセット(A)";
const FIND: &str = "検索(F)";
const REPLACE: &str = "置換(R)";
const CANCEL: &str = "キャンセル(X)";
const HELP: &str = crate::support::help::BUTTON_LABEL;

/// The radio buttons of 結果出力 in the order they are drawn in: the id, the label and the value.
const LINE_TYPES: [(&str, &str, LineType); 3] = [
    ("grep-line", LINE, LineType::Line),
    ("grep-part", PART, LineType::MatchPart),
    ("grep-nohit", NO_HIT, LineType::NoHitLine),
];
/// The radio buttons of 結果出力形式: the id, the label and the value.
const STYLES: [(&str, &str, OutputStyle); 3] = [
    ("grep-normal", NORMAL, OutputStyle::Normal),
    ("grep-per-file", PER_FILE, OutputStyle::PerFile),
    ("grep-result-only", RESULT_ONLY, OutputStyle::ResultOnly),
];

/// What an access key does.
#[derive(Clone, Copy)]
enum Use {
    /// The label in front of a box: the focus goes to the box.
    Focus(Field),
    /// A check box: switches its value.
    Toggle(fn(&mut GrepSettings)),
    /// A radio button of 結果出力: chooses.
    LineType(LineType),
    /// A radio button of 結果出力形式: chooses.
    Style(OutputStyle),
    ThisText,
    DefaultFolder,
    Browse,
    Up,
    CurrentFolder,
    /// 文字コードセット: the next code set (the combo box is a button that goes round).
    Charset,
    Ok,
    Cancel,
    /// ヘルプ(H): the page of the dialog.
    Help,
}

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
    ///
    /// `f` reads the list in place: the history is not copied for a look at it.
    fn with_entries<R>(self, cx: &App, f: impl FnOnce(&[String]) -> R) -> R {
        match self {
            Field::Key => history::with_search_keys(cx, f),
            Field::File => history::with_grep_files(cx, f),
            Field::Folder => history::with_grep_folders(cx, f),
            Field::ExcludeFile => history::with_exclude_files(cx, f),
            Field::ExcludeFolder => history::with_exclude_folders(cx, f),
            Field::Replacement => history::with_replace_keys(cx, f),
        }
    }
}

/// Fills the boxes that are empty from the histories, as `CDlgGrep::DoModal` does when the dialog
/// opens: the file and the folder take the newest string of their history, and so do the excluded
/// files and folders, which take the default patterns when the history is empty and put them in
/// it, to be chosen later. A value that was given (the command line, the last search) comes
/// first. The file box takes `*.*` that way too (Sakura starts with it in the history).
fn fill_from_histories(settings: &mut GrepSettings, cx: &mut App) {
    fn newest(list: &[String]) -> Option<String> {
        list.first().cloned()
    }
    let mut keep = GrepBoxes {
        file: "",
        folder: "",
        exclude_file: "",
        exclude_folder: "",
    };
    if settings.file.is_empty() {
        match history::with_grep_files(cx, newest) {
            Some(file) => settings.file = file,
            None => {
                settings.file = "*.*".to_owned();
                keep.file = "*.*";
            }
        }
    }
    if settings.folder.is_empty()
        && let Some(folder) = history::with_grep_folders(cx, newest)
    {
        settings.folder = folder;
    }
    if settings.exclude_file.is_empty() {
        match history::with_exclude_files(cx, newest) {
            Some(exclude) => settings.exclude_file = exclude,
            None => {
                settings.exclude_file = DEFAULT_EXCLUDE_FILE.to_owned();
                keep.exclude_file = DEFAULT_EXCLUDE_FILE;
            }
        }
    }
    if settings.exclude_folder.is_empty() {
        match history::with_exclude_folders(cx, newest) {
            Some(exclude) => settings.exclude_folder = exclude,
            None => {
                settings.exclude_folder = DEFAULT_EXCLUDE_FOLDER.to_owned();
                keep.exclude_folder = DEFAULT_EXCLUDE_FOLDER;
            }
        }
    }
    if [keep.file, keep.exclude_file, keep.exclude_folder]
        .iter()
        .any(|text| !text.is_empty())
    {
        history::remember_grep_boxes(cx, &keep);
    }
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
    /// The box whose history list is open. The highlighted row is not kept: it is the row that
    /// holds the text of the box (`row_of`), read when it is needed, so that typing in the box
    /// cannot leave a stale row.
    dropdown: Option<Field>,
    /// Scrolls the open list to the highlighted entry.
    list_scroll: ScrollHandle,
    /// The boxes of the files and folders as they were before 編集中のテキストから検索 was
    /// turned on, to put back when it is turned off (`SetDataFromThisText`).
    saved_boxes: Option<[String; 4]>,
    /// The focus of the radio buttons of 結果出力 and of 結果出力形式 (`IDD_GREP`, one group each).
    pub(crate) line_type_ring: Ring,
    pub(crate) style_ring: Ring,
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
        for (field, state) in [
            (Field::Key, &key),
            (Field::File, &file),
            (Field::Folder, &folder),
            (Field::ExcludeFile, &exclude_file),
            (Field::ExcludeFolder, &exclude_folder),
            (Field::Replacement, &replacement),
        ] {
            subscriptions.push(cx.subscribe_in(
                state,
                window,
                move |this: &mut Self, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        // The key binding of Enter runs before the key listeners
                        // (`combo_key`), so the open list of this box takes the key here: only
                        // the row is taken, the dialog is not run.
                        if this.dropdown == Some(field) {
                            this.close_open_list(field, cx);
                        } else {
                            this.ok(window, cx);
                        }
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
            line_type_ring: Ring::default(),
            style_ring: Ring::default(),
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

    /// What `GetData` keeps before it checks anything: the character set, the form of the result
    /// and the check boxes of the output, 除外ファイルを正規表現で指定する, and for Grep 置換, once
    /// it has a condition, バックアップ作成 (`CDlgGrepReplace::GetData`).
    fn write_before_checks(&self, cx: &mut App) {
        if self.replace_mode {
            if self.settings.key.is_empty() {
                return;
            }
            crate::saved_state::grep::remember_backup(cx, self.settings.backup);
        }
        crate::saved_state::grep::remember_output(cx, &self.settings);
    }

    /// What `GetData` keeps once the boxes are accepted: the options of the pattern when there is
    /// a pattern that is short enough to be put in the history, and サブフォルダーからも検索する
    /// unless the text of the window is what is searched.
    fn write_after_checks(&self, cx: &mut App) {
        let s = &self.settings;
        // `_MAX_PATH`
        if !s.key.is_empty() && s.key.encode_utf16().count() < 260 {
            crate::saved_state::search::remember_pattern_options(
                cx,
                s.options.regex,
                s.options.match_case,
                s.options.whole_word,
            );
        }
        if !s.from_this_text {
            crate::saved_state::grep::remember_sub_folder(cx, s.sub_folder);
        }
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
            self.write_before_checks(cx);
            self.settings.check_from_this_text(document)
        } else if self.replace_mode {
            // `CDlgGrepReplace::OnBnClicked`: pasting needs something on the clipboard.
            let clipboard = if self.settings.paste {
                let text = clipboard_replacement(cx);
                if text.is_none() {
                    self.settings.paste = false;
                    cx.notify();
                    self.show_error(
                        &DialogError {
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
            self.write_before_checks(cx);
            self.settings.check_replace(clipboard)
        } else {
            self.write_before_checks(cx);
            self.settings.check()
        };
        let info = match checked {
            Ok(info) => info,
            Err(error) => {
                self.show_error(&error, window, cx);
                return;
            }
        };
        let from_this_text = self.settings.from_this_text;
        self.write_after_checks(cx);
        if !from_this_text {
            // `GetData` writes the folders back as they are checked.
            self.show_settings(window, cx);
        }
        remember(cx, self.settings.clone());
        let settings = &self.settings;
        super::window::remember_window(cx, document.filter(|_| from_this_text));
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
        if self.dropdown == Some(field) {
            self.dropdown = None;
        } else {
            let current = self.field_input(field).read(cx).value();
            let (empty, at) = field.with_entries(cx, |entries| {
                (entries.is_empty(), row_of(entries, current.as_str()))
            });
            self.dropdown = (self.field_enabled(field) && !empty).then_some(field);
            if let Some(row) = at.filter(|_| self.dropdown.is_some()) {
                self.list_scroll.scroll_to_item(row);
            }
        }
        cx.notify();
    }

    /// Whether a history list is open, for the tests.
    #[cfg(test)]
    pub(crate) fn list_open(&self) -> bool {
        self.dropdown.is_some()
    }

    /// Enter in an open list: the list closes and the box keeps its text, which the arrow keys
    /// have put there.
    fn close_open_list(&mut self, field: Field, cx: &mut Context<Self>) {
        if self.dropdown == Some(field) {
            self.dropdown = None;
            cx.notify();
        }
    }

    /// Puts a string of the list in its box, closes the list and gives the focus back to the box.
    fn pick(
        &mut self,
        field: Field,
        text: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
        let current = self.field_input(field).read(cx).value();
        let next = field.with_entries(cx, |entries| {
            if entries.is_empty() {
                return None;
            }
            let next = arrow_row(row_of(entries, current.as_str()), down, entries.len());
            Some(entries[next].clone())
        });
        if let Some(next) = next {
            self.pick(field, next.into(), window, cx);
        }
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
        if self.dropdown.is_some_and(|open| Some(open) != focused) {
            // The focus left the box: the list closes.
            self.dropdown = None;
            cx.notify();
        }
        let Some(field) = focused else { return false };
        let key = event.keystroke.key.as_str();
        let m = &event.keystroke.modifiers;
        let plain = !m.alt && !m.control && !m.platform && !m.shift;
        // Alt+Up closes an open list, as F4 does (measured on a Windows combo box).
        let alt_up_closes = key == "up" && m.alt && !m.control && !m.platform && !m.shift;
        if key == "f4"
            || (key == "down" && m.alt)
            || (alt_up_closes && self.dropdown == Some(field))
        {
            self.toggle_dropdown(field, cx);
            return true;
        }
        if self.dropdown.is_none() {
            return match key {
                "down" | "up" if plain && self.field_enabled(field) => {
                    self.step(field, key == "down", window, cx);
                    true
                }
                _ => false,
            };
        }
        let count = field.with_entries(cx, <[String]>::len);
        match key {
            "down" | "up" if plain && count > 0 => {
                // From the row that holds the text now, which may have been typed since the last
                // arrow: a text that is not a row starts at the first row.
                let current = self.field_input(field).read(cx).value();
                let (row, text) = field.with_entries(cx, |entries| {
                    let row = arrow_row(row_of(entries, current.as_str()), key == "down", count);
                    (row, entries[row].clone())
                });
                self.list_scroll.scroll_to_item(row);
                self.field_input(field)
                    .update(cx, |state, cx| state.set_value(text, window, cx));
                cx.notify();
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
                    div().debug_selector(|| toggle_id.to_owned()).child(
                        Button::new(toggle_id)
                            .label("▼")
                            .accessibility_label(toggle_name)
                            .disabled(!enabled)
                            // The boxes of `IDD_GREP` are combo boxes (`CBS_DROPDOWN`): Tab
                            // skips the arrow.
                            .tab_stop(false)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                // The button does not take the focus; the keys of the list go
                                // to the box that has it, so give it to this one.
                                let input = this.field_input(field).clone();
                                input.update(cx, |state, cx| state.focus(window, cx));
                                this.toggle_dropdown(field, cx);
                            })),
                    ),
                ),
        );
        if self.dropdown == Some(field) {
            let current = self.field_input(field).read(cx).value();
            let (texts, highlight): (Vec<SharedString>, Option<usize>) =
                field.with_entries(cx, |entries| {
                    (
                        entries.iter().cloned().map(SharedString::from).collect(),
                        row_of(entries, current.as_str()),
                    )
                });
            let entries = texts.into_iter().enumerate().map(|(i, text)| {
                let picked = text.clone();
                crate::a11y::list_row(div().id((id, i)), &text, Some(i) == highlight)
                    .px_2()
                    .truncate()
                    .when(Some(i) == highlight, |d| d.bg(rgb(0xcce4f7)))
                    .hover(|d| d.bg(rgb(0xe5f1fb)))
                    .child(text)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.pick(field, picked.clone(), window, cx);
                    }))
            });
            column = column.child(
                crate::a11y::list_box(div().id(id), toggle_name)
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

    fn show_error(&self, error: &DialogError, window: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| {
                editor.grep_dialog_error_box(error, window, cx)
            })
            .ok();
    }

    /// `IDC_BUTTON_HELP`: `CDlgGrep` opens the page of `F_GREP_DIALOG`, `CDlgGrepReplace` the one of
    /// `F_GREP_REPLACE_DLG`.
    fn open_help(&self, cx: &mut Context<Self>) {
        crate::support::help::open_function_page(cx, self.help_function());
    }

    fn help_function(&self) -> &'static str {
        if self.replace_mode {
            "F_GREP_REPLACE_DLG"
        } else {
            "F_GREP_DIALOG"
        }
    }

    /// キャンセル.
    pub(crate) fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.read_boxes(cx);
        remember(cx, self.settings.clone());
        self.editor
            .update(cx, |editor, cx| {
                editor.close_grep_panel(window, cx);
                // No Grep follows: the launch of `-GREPDLG` (if this is its dialog) does not wait for one.
                editor.release_launch_open();
            })
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
                    this.set_folder(quote_folder(&path.to_string_lossy()), window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    fn set_folder(&mut self, folder: String, window: &mut Window, cx: &mut Context<Self>) {
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
            self.set_folder(quote_folder(&folder), window, cx);
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
        self.set_folder(joined, window, cx);
    }

    /// フォルダーの初期値をカレントフォルダーにする: written to the settings as soon as it is
    /// clicked (`CDlgGrep::OnBnClicked`).
    fn set_default_folder(&mut self, value: bool, cx: &mut Context<Self>) {
        self.change(|s| s.default_folder = value, cx);
        crate::saved_state::grep::remember_default_folder(cx, value);
    }

    /// Alt with the letter of a label (`IsDialogMessage`): switches the check box, chooses the radio
    /// button, presses the button, or moves the focus to the box after the label. Returns whether the
    /// letter was one of the dialog's (then the key is not typed into the box).
    pub(crate) fn use_access_key(
        &mut self,
        letter: char,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let s = &self.settings;
        let this_text = s.from_this_text;
        let replace = self.replace_mode;
        let result_only = s.style == OutputStyle::ResultOnly;
        let files = !this_text;
        let entries = [
            Entry::labelled(
                if replace {
                    REPLACE_KEY_LABEL
                } else {
                    KEY_LABEL
                },
                true,
                Use::Focus(Field::Key),
            ),
            Entry::labelled(
                REPLACEMENT_LABEL,
                replace && self.field_enabled(Field::Replacement),
                Use::Focus(Field::Replacement),
            ),
            Entry::labelled(FILE_LABEL, files, Use::Focus(Field::File)),
            Entry::labelled(FOLDER_LABEL, files, Use::Focus(Field::Folder)),
            Entry::labelled(EXCLUDE_FILE_LABEL, files, Use::Focus(Field::ExcludeFile)),
            Entry::labelled(
                EXCLUDE_FOLDER_LABEL,
                files,
                Use::Focus(Field::ExcludeFolder),
            ),
            Entry::labelled(BROWSE, files, Use::Browse),
            Entry::labelled(UP, files, Use::Up),
            Entry::labelled(CURRENT_FOLDER, files, Use::CurrentFolder),
            Entry::labelled(THIS_TEXT, !replace, Use::ThisText),
            Entry::labelled(
                SUB_FOLDER,
                files,
                Use::Toggle(|s| s.sub_folder = !s.sub_folder),
            ),
            Entry::labelled(
                MATCH_CASE,
                true,
                Use::Toggle(|s| s.options.match_case = !s.options.match_case),
            ),
            Entry::labelled(
                WHOLE_WORD,
                !s.options.regex,
                Use::Toggle(|s| s.options.whole_word = !s.options.whole_word),
            ),
            Entry::labelled(
                REGEX,
                true,
                Use::Toggle(|s| s.options.regex = !s.options.regex),
            ),
            Entry::labelled(
                EXCEPT_REGEX,
                files,
                Use::Toggle(|s| s.except_file_regex = !s.except_file_regex),
            ),
            Entry::labelled(DEFAULT_FOLDER, true, Use::DefaultFolder),
            Entry::labelled(PASTE, replace, Use::Toggle(|s| s.paste = !s.paste)),
            Entry::labelled(BACKUP, replace, Use::Toggle(|s| s.backup = !s.backup)),
            Entry::labelled(LINE, true, Use::LineType(LineType::Line)),
            Entry::labelled(PART, true, Use::LineType(LineType::MatchPart)),
            Entry::labelled(NO_HIT, true, Use::LineType(LineType::NoHitLine)),
            Entry::labelled(NORMAL, true, Use::Style(OutputStyle::Normal)),
            Entry::labelled(PER_FILE, true, Use::Style(OutputStyle::PerFile)),
            Entry::labelled(RESULT_ONLY, true, Use::Style(OutputStyle::ResultOnly)),
            Entry::labelled(FILE_ONLY, true, Use::Toggle(|s| s.file_only = !s.file_only)),
            Entry::labelled(
                SEPARATE_FOLDER,
                !result_only,
                Use::Toggle(|s| s.separate_folder = !s.separate_folder),
            ),
            Entry::labelled(
                BASE_FOLDER,
                !result_only,
                Use::Toggle(|s| s.base_folder = !s.base_folder),
            ),
            Entry::labelled(CHARSET_LABEL, true, Use::Charset),
            Entry::labelled(if replace { REPLACE } else { FIND }, true, Use::Ok),
            Entry::labelled(CANCEL, true, Use::Cancel),
            Entry::labelled(HELP, true, Use::Help),
        ];
        let used = match pick(letter, &entries) {
            Found::Nothing => return false,
            Found::Several => return true,
            Found::One(used) => used,
        };
        match used {
            Use::Focus(field) => {
                let input = self.field_input(field).clone();
                focus_selecting(&input, window, cx);
            }
            Use::Toggle(toggle) => self.change(toggle, cx),
            Use::LineType(line_type) => {
                self.set_line_type(line_type, cx);
                if let Some(index) = LINE_TYPES.iter().position(|(_, _, t)| *t == line_type) {
                    self.line_type_ring.focus(index, window, cx);
                }
            }
            Use::Style(style) => {
                self.set_style(style, cx);
                if let Some(index) = STYLES.iter().position(|(_, _, t)| *t == style) {
                    self.style_ring.focus(index, window, cx);
                }
            }
            Use::ThisText => {
                self.set_from_this_text(!self.settings.from_this_text, window, cx);
            }
            Use::DefaultFolder => self.set_default_folder(!self.settings.default_folder, cx),
            Use::Browse => self.browse(window, cx),
            Use::Up => self.folder_up(window, cx),
            Use::CurrentFolder => self.current_folder(window, cx),
            Use::Charset => self.change(|s| s.charset = next_charset(s.charset), cx),
            Use::Ok => self.ok(window, cx),
            Use::Cancel => self.cancel(window, cx),
            Use::Help => self.open_help(cx),
        }
        true
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

    /// 結果出力: the one writer of the choice (the click, the arrow keys and the access keys).
    fn set_line_type(&mut self, line_type: LineType, cx: &mut Context<Self>) {
        self.change(|s| s.line_type = line_type, cx);
    }

    /// 結果出力形式: the one writer of the choice.
    fn set_style(&mut self, style: OutputStyle, cx: &mut Context<Self>) {
        self.change(|s| s.style = style, cx);
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
        let this = cx.entity();
        let s = &self.settings;
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
                    .labelled(BROWSE)
                    .disabled(this_text)
                    .on_click(cx.listener(|this, _, window, cx| this.browse(window, cx))),
            )
            .child(
                Button::new("grep-up")
                    .labelled(UP)
                    .disabled(this_text)
                    .on_click(cx.listener(|this, _, window, cx| this.folder_up(window, cx))),
            )
            .child(
                Button::new("grep-current-folder")
                    .labelled(CURRENT_FOLDER)
                    .disabled(this_text)
                    .on_click(cx.listener(|this, _, window, cx| this.current_folder(window, cx))),
            );
        let paste = s.paste;
        let mut options = v_flex()
            .gap_1()
            // Sakura greys it out for a replacement (`CDlgGrepReplace::SetData`).
            .child(
                Checkbox::new("grep-this-text")
                    .labelled(THIS_TEXT)
                    .checked(this_text)
                    .disabled(self.replace_mode)
                    .on_click(cx.listener(|this, value: &bool, window, cx| {
                        this.set_from_this_text(*value, window, cx);
                    })),
            )
            .child(
                self.check_box(
                    "grep-sub",
                    SUB_FOLDER,
                    s.sub_folder,
                    |s, v| s.sub_folder = v,
                    cx,
                )
                .disabled(this_text),
            )
            .child(self.check_box(
                "grep-case",
                MATCH_CASE,
                s.options.match_case,
                |s, v| s.options.match_case = v,
                cx,
            ))
            // Sakura greys the word box out while 正規表現 is on.
            .child(
                self.check_box(
                    "grep-word",
                    WHOLE_WORD,
                    s.options.whole_word,
                    |s, v| s.options.whole_word = v,
                    cx,
                )
                .disabled(regex),
            )
            .child(self.check_box(
                "grep-regex",
                REGEX,
                s.options.regex,
                |s, v| s.options.regex = v,
                cx,
            ))
            .child(
                self.check_box(
                    "grep-except-regex",
                    EXCEPT_REGEX,
                    s.except_file_regex,
                    |s, v| s.except_file_regex = v,
                    cx,
                )
                .disabled(this_text),
            )
            .child(
                // Written to the settings as soon as it is clicked (`CDlgGrep::OnBnClicked`).
                Checkbox::new("grep-default-folder")
                    .labelled(DEFAULT_FOLDER)
                    .checked(s.default_folder)
                    .on_click(cx.listener(|this, value: &bool, _, cx| {
                        this.set_default_folder(*value, cx);
                    })),
            );
        if self.replace_mode {
            options = options
                .child(self.check_box("grep-paste", PASTE, paste, |s, v| s.paste = v, cx))
                .child(self.check_box("grep-backup", BACKUP, s.backup, |s, v| s.backup = v, cx));
        }
        // `IDD_GREP`: the first radio button of each of the two groups has `WS_GROUP | WS_TABSTOP`
        // (`sakura_rc.rc`).
        let line_types = Group::new(
            &self.line_type_ring,
            LINE_TYPES.len(),
            LINE_TYPES.iter().position(|(_, _, t)| *t == s.line_type),
            Rc::new(cx.listener(|this, index: &usize, _, cx| {
                this.set_line_type(LINE_TYPES[*index].2, cx);
            })),
        );
        let styles = Group::new(
            &self.style_ring,
            STYLES.len(),
            STYLES.iter().position(|(_, _, t)| *t == s.style),
            Rc::new(cx.listener(|this, index: &usize, _, cx| {
                this.set_style(STYLES[*index].2, cx);
            })),
        );
        let output = h_flex()
            .gap_4()
            .child(
                v_flex().gap_1().items_start().child("結果出力").children(
                    LINE_TYPES
                        .iter()
                        .enumerate()
                        .map(|(index, (id, label, t))| {
                            line_types.radio(index, *id, true, |radio| {
                                radio.labelled(*label).checked(s.line_type == *t)
                            })
                        }),
                ),
            )
            .child(v_flex().gap_1().items_start().child("出力形式").children(
                STYLES.iter().enumerate().map(|(index, (id, label, t))| {
                    styles.radio(index, *id, true, |radio| {
                        radio.labelled(*label).checked(s.style == *t)
                    })
                }),
            ))
            .child(
                v_flex()
                    .gap_1()
                    .child(self.check_box(
                        "grep-file-only",
                        FILE_ONLY,
                        s.file_only,
                        |s, v| s.file_only = v,
                        cx,
                    ))
                    // "結果のみ" has no folders to show (`CDlgGrep::OnBnClicked`).
                    .child(
                        self.check_box(
                            "grep-base-folder",
                            BASE_FOLDER,
                            s.base_folder,
                            |s, v| s.base_folder = v,
                            cx,
                        )
                        .disabled(result_only),
                    )
                    .child(
                        self.check_box(
                            "grep-separate-folder",
                            SEPARATE_FOLDER,
                            s.separate_folder,
                            |s, v| s.separate_folder = v,
                            cx,
                        )
                        .disabled(result_only),
                    ),
            );
        let charset = h_flex().gap_2().items_center().child(CHARSET_LABEL).child(
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
                    .labelled(if self.replace_mode { REPLACE } else { FIND })
                    .on_click(cx.listener(|this, _, window, cx| this.ok(window, cx))),
            )
            .child(
                Button::new("grep-cancel")
                    .labelled(CANCEL)
                    .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
            )
            .child(
                Button::new("grep-help")
                    .labelled(HELP)
                    .on_click(cx.listener(|this, _, _, cx| this.open_help(cx))),
            );
        let help_function = self.help_function();
        let title = if self.replace_mode {
            "Grep置換"
        } else {
            "Grep"
        };
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
                crate::a11y::dialog(div().id("grep-panel"), title)
                    .track_focus(&self.scope)
                    .keep_tab_inside(&self.scope)
                    // Alt+letter works from the boxes too, and does not type the letter
                    .on_access_key(move |letter, window, cx| {
                        this.update(cx, |panel, cx| panel.use_access_key(letter, window, cx))
                    })
                    // F1: the page of the dialog
                    .on_help_key(help_function)
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
                                    REPLACE_KEY_LABEL
                                } else {
                                    KEY_LABEL
                                },
                                self.combo_box(Field::Key, cx),
                            ))
                            .children(self.replace_mode.then(|| {
                                // The box is not used while the text of the clipboard replaces
                                // (`DlgItem_Enable(IDC_COMBO_TEXT2, ...)`).
                                Self::row(REPLACEMENT_LABEL, self.combo_box(Field::Replacement, cx))
                            }))
                            .child(Self::row(FILE_LABEL, self.combo_box(Field::File, cx)))
                            .child(Self::row(FOLDER_LABEL, folder_buttons))
                            .child(Self::row(
                                EXCLUDE_FILE_LABEL,
                                self.combo_box(Field::ExcludeFile, cx),
                            ))
                            .child(Self::row(
                                EXCLUDE_FOLDER_LABEL,
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
        // The choices the file keeps (`DoModal`): the dialog starts with them, not with what a
        // dialog that was cancelled held.
        crate::saved_state::grep::saved(cx).apply(&mut settings, replace);
        settings.options = crate::saved_state::search::saved(cx).options();
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
        // The histories first, then the folder of the file in the window (`OnInitDialog` takes it
        // when the folder is still empty or 初期値をカレントフォルダーにする is on).
        fill_from_histories(&mut settings, cx);
        settings.offer_folder(self.document.read(cx).path());
        self.show_grep_panel(settings, replace, window, cx);
    }

    /// `-GREPDLG`: opens the dialog with `settings` as they are (the conditions of the command
    /// line, `CNormalProcess::InitializeProcess`).
    pub(crate) fn open_grep_panel_with(
        &mut self,
        mut settings: GrepSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        fill_from_histories(&mut settings, cx);
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
