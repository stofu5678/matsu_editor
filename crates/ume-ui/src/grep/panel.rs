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
//! Left out: 「この編集中のテキストから検索する」 (searching the text of a window), the combo
//! boxes' history lists, and the list of code pages (the nine encodings of Sakura are offered).

use std::path::Path;

use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, PathPromptOptions, Render, Styled, Subscription, WeakEntity,
    Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{Disableable as _, Selectable as _, h_flex, v_flex};
use ume_core::encoding::Encoding;

use super::format::{LineType, OutputStyle};
use super::keys::split_folders;
use super::settings::{DialogError, GrepSettings, quote_folder};
use super::window::{clipboard_replacement, remember, remembered};
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

pub struct GrepPanel {
    editor: WeakEntity<EditorView>,
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
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
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
                        this.ok(window, cx);
                    }
                },
            ));
        }
        let mut panel = Self {
            editor,
            key,
            file,
            folder,
            exclude_file,
            exclude_folder,
            replacement,
            replace_mode,
            settings,
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
        self.settings.file = text(&self.file);
        self.settings.folder = text(&self.folder);
        self.settings.exclude_file = text(&self.exclude_file);
        self.settings.exclude_folder = text(&self.exclude_folder);
        self.settings.replacement = text(&self.replacement);
    }

    pub fn focus_key(&self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self.key.clone();
        key.update(cx, |state, cx| state.focus(window, cx));
    }

    /// The OK button (検索, 置換 for Grep 置換): checks the boxes, and starts the search.
    pub(crate) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.read_boxes(cx);
        let checked = if self.replace_mode {
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
        // `GetData` writes the folders back as they are checked.
        self.show_settings(window, cx);
        let settings = self.settings.clone();
        remember(cx, settings.clone());
        let replace = self.replace_mode;
        let Some(editor) = self.editor.upgrade() else {
            return;
        };
        editor.update(cx, |editor, cx| {
            if !replace {
                // What is recorded is the Grep, as `HandleCommand(F_GREP)` does after the dialog.
                // Sakura does not record a Grep 置換 (`GrepReplace` is not a key macro).
                editor.record_call("F_GREP", &settings.macro_args());
            }
            editor.close_grep_panel(window, cx);
            editor.start_grep(info, false, window, cx);
        });
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

    /// 参照: chooses a folder.
    fn browse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        cx.spawn_in(window, async move |this, cx| {
            let chosen = receiver.await.ok().and_then(Result::ok).flatten();
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
            .label(label)
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
            .label(label)
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
        let folder_buttons = h_flex()
            .gap_2()
            .child(Input::new(&self.folder))
            .child(
                Button::new("grep-browse")
                    .label("参照(O)")
                    .on_click(cx.listener(|this, _, window, cx| this.browse(window, cx))),
            )
            .child(
                Button::new("grep-up")
                    .label("上(U)")
                    .on_click(cx.listener(|this, _, window, cx| this.folder_up(window, cx))),
            )
            .child(
                Button::new("grep-current-folder")
                    .label("現フォルダー(J)")
                    .on_click(cx.listener(|this, _, window, cx| this.current_folder(window, cx))),
            );
        let paste = s.paste;
        let mut options = v_flex()
            .gap_1()
            .child(self.check_box(
                "grep-sub",
                "サブフォルダーからも検索する(S)",
                s.sub_folder,
                |s, v| s.sub_folder = v,
                cx,
            ))
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
            .child(self.check_box(
                "grep-except-regex",
                "除外ファイルを正規表現で指定(X)",
                s.except_file_regex,
                |s, v| s.except_file_regex = v,
                cx,
            ))
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
                    .label(charset_label(s.charset))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.change(|s| s.charset = next_charset(s.charset), cx);
                    })),
            );
        let buttons = h_flex()
            .gap_2()
            .child(
                Button::new("grep-ok")
                    .label(if self.replace_mode {
                        "置換"
                    } else {
                        "検索"
                    })
                    .on_click(cx.listener(|this, _, window, cx| this.ok(window, cx))),
            )
            .child(
                Button::new("grep-cancel")
                    .label("キャンセル")
                    .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
            );
        div()
            .id("grep-panel")
            .absolute()
            // mouse events stop here: a click on the panel must not move the caret
            .occlude()
            .top(px(8.))
            .right(px(24.))
            .w(px(620.))
            .p_2()
            .gap_2()
            .flex()
            .flex_col()
            .bg(rgb(0xf6f6f6))
            .text_color(rgb(0x1e1e1e))
            .text_size(px(13.))
            .border_1()
            .border_color(rgb(0xb0b0b0))
            .rounded_md()
            .shadow_md()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.cancel(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(if self.replace_mode {
                        "Grep置換"
                    } else {
                        "Grep"
                    }),
            )
            .child(Self::row(
                if self.replace_mode {
                    "置換前(N)"
                } else {
                    "条件(N)"
                },
                Input::new(&self.key),
            ))
            .children(self.replace_mode.then(|| {
                // The box is not used while the text of the clipboard replaces
                // (`DlgItem_Enable(IDC_COMBO_TEXT2, ...)`).
                Self::row("置換後(Z)", Input::new(&self.replacement).disabled(paste))
            }))
            .child(Self::row("ファイル(I)", Input::new(&self.file)))
            .child(Self::row("フォルダー(D)", folder_buttons))
            .child(Self::row("除外ファイル", Input::new(&self.exclude_file)))
            .child(Self::row(
                "除外フォルダー",
                Input::new(&self.exclude_folder),
            ))
            .child(options)
            .child(charset)
            .child(output)
            .child(buttons)
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
        if let Some(seed) = self.search_seed(cx) {
            settings.key = seed;
        }
        let path = self.document.read(cx).path().map(Path::to_path_buf);
        settings.offer_folder(path.as_deref());
        // The search panel is for another dialog; only one of them is open.
        self.close_search_panel(window, cx);
        match &self.grep.panel {
            Some(panel) => {
                panel.update(cx, |panel, cx| {
                    panel.replace_mode = replace;
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
