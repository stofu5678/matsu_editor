//! The DIFF dialog (`F_DIFF_DIALOG`, DIFF差分表示): Sakura's `IDD_DIFF`
//! (`sakura_core/sakura_rc.rc`, `sakura_core/dlg/CDlgDiff.cpp`).
//!
//! Like the Grep dialog it is a panel in the editor window. The other side is a file (外部ファイル
//! を指定) or another editor window (他の編集中のファイル, selected by name as Sakura does); the
//! check boxes are the options of `diff` (the bits of `ume_core::diff::DiffOptions`), the choice
//! says whether this window is the new or the old file, and two of the boxes are the search options
//! that the jumps to the next and the previous difference follow.
//!
//! Left out: the button ヘルプ.

use std::path::{Path, PathBuf};

use gpui::{
    App, AppContext as _, ClickEvent, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{Selectable as _, h_flex, v_flex};
use ume_core::diff::DiffOptions;

use super::{
    Candidate, DocRef, Owner, Remembered, Target, best_candidate, candidates, remember, remembered,
};
use crate::file_dialog::{self, Ask};
use crate::file_ops::ask::document_title;
use crate::search::SearchOptions;
use crate::view::EditorView;

pub struct DiffPanel {
    editor: WeakEntity<EditorView>,
    owner: Owner,
    /// The label of this window (編集中のファイル).
    own_label: String,
    file: Entity<InputState>,
    /// 外部ファイルを指定 is chosen (else the list).
    use_file: bool,
    selected: Option<DocRef>,
    options: DiffOptions,
    notify_not_found: bool,
    wrap: bool,
    _subscriptions: Vec<Subscription>,
}

impl DiffPanel {
    fn new(
        editor: WeakEntity<EditorView>,
        owner: Owner,
        own_label: String,
        selected: Option<DocRef>,
        search: SearchOptions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let file = cx.new(|cx| InputState::new(window, cx).placeholder("比較するファイル"));
        let subscription = cx.subscribe_in(
            &file,
            window,
            |this: &mut Self, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.ok(window, cx);
                }
            },
        );
        Self {
            editor,
            owner,
            own_label,
            file,
            // "開いているファイルがある場合には初期状態でそちらを優先" (2004.02.22 じゅうじ)
            use_file: selected.is_none(),
            selected,
            options: DiffOptions::from_flags(remembered(cx).flags),
            notify_not_found: search.notify_not_found,
            wrap: search.wrap,
            _subscriptions: vec![subscription],
        }
    }

    /// The flags of the options as the dialog has them now.
    pub(crate) fn flags(&self) -> u32 {
        self.options.flags()
    }

    /// The other side the dialog has now, if it has one (`CDlgDiff::GetData` fails without it).
    pub(crate) fn target(&self, cx: &App) -> Option<Target> {
        if self.use_file {
            let text = self.file.read(cx).value().to_string();
            (!text.is_empty()).then(|| Target::File(PathBuf::from(text)))
        } else {
            self.selected.map(Target::Window)
        }
    }

    pub(crate) fn set_file(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.to_owned();
        self.file
            .update(cx, |state, cx| state.set_value(path, window, cx));
        self.use_file = true;
        cx.notify();
    }

    pub(crate) fn select_window(&mut self, doc: DocRef, cx: &mut Context<Self>) {
        self.selected = Some(doc);
        self.use_file = false;
        cx.notify();
    }

    pub fn focus_file(&self, window: &mut Window, cx: &mut Context<Self>) {
        let file = self.file.clone();
        file.update(cx, |state, cx| state.focus(window, cx));
    }

    /// OK: runs the DIFF. Without a file or a window the dialog just closes
    /// (`GetData` is false and `Command_Diff_Dialog` returns).
    pub(crate) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let target = self.target(cx);
        let flags = self.flags();
        let previous = remembered(cx);
        remember(cx, Remembered { flags, ..previous });
        let (notify_not_found, wrap) = (self.notify_not_found, self.wrap);
        let Some(editor) = self.editor.upgrade() else {
            return;
        };
        editor.update(cx, |editor, cx| {
            // the search options of the dialog are the ones of the whole program in Sakura
            let search = editor.search.options();
            editor.search.set_options(SearchOptions {
                notify_not_found,
                wrap,
                ..search
            });
            editor.close_diff_panel(window, cx);
            if let Some(target) = target {
                editor.run_diff(target, flags, window, cx);
            }
        });
    }

    /// キャンセル.
    pub(crate) fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| editor.close_diff_panel(window, cx))
            .ok();
    }

    /// 参照: chooses the file. The dialog starts in the folder of the file of this window
    /// (`IDC_BUTTON_DIFF_DST` starts in `m_szFile1`).
    fn browse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let start = self
            .editor
            .read_with(cx, |editor, cx| {
                editor
                    .document()
                    .read(cx)
                    .path()
                    .and_then(Path::parent)
                    .map(Path::to_owned)
            })
            .ok()
            .flatten();
        let receiver = file_dialog::ask(window, cx, Ask::open(false).start(start));
        cx.spawn_in(window, async move |this, cx| {
            let chosen = receiver.await.ok().flatten();
            if let Some(path) = chosen.and_then(|paths| paths.into_iter().next()) {
                this.update_in(cx, |this, window, cx| {
                    this.set_file(&path.to_string_lossy(), window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    fn option(
        &self,
        id: &'static str,
        label: &'static str,
        checked: bool,
        change: impl Fn(&mut Self, bool) + 'static,
        cx: &mut Context<Self>,
    ) -> Checkbox {
        Checkbox::new(id)
            .label(label)
            .checked(checked)
            .on_click(cx.listener(move |this, value: &bool, _, cx| {
                change(this, *value);
                cx.notify();
            }))
    }
}

impl Focusable for DiffPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.file.read(cx).focus_handle(cx)
    }
}

impl Render for DiffPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<Candidate> = candidates(cx, self.owner);
        let list = rows.iter().enumerate().map(|(i, row)| {
            let doc = row.doc;
            let mut item = div()
                .id(("diff-window", i))
                .px_2()
                .h(px(22.))
                .flex()
                .items_center()
                .cursor_pointer()
                .child(row.label.clone())
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.select_window(doc, cx);
                }));
            if !self.use_file && self.selected == Some(doc) {
                item = item.bg(rgb(0x2f6fd0)).text_color(rgb(0xffffff));
            }
            item
        });
        let o = self.options;
        let destination = v_flex()
            .gap_1()
            .p_2()
            .border_1()
            .border_color(rgb(0xb0b0b0))
            .child("比較するファイル")
            .child(
                Button::new("diff-use-file")
                    .label("外部ファイルを指定")
                    .selected(self.use_file)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.use_file = true;
                        cx.notify();
                    })),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(div().flex_1().child(Input::new(&self.file)))
                    .child(
                        Button::new("diff-browse")
                            .label("参照(R)...")
                            .on_click(cx.listener(|this, _, window, cx| this.browse(window, cx))),
                    ),
            )
            .child(
                Button::new("diff-use-window")
                    .label("他の編集中のファイル")
                    .selected(!self.use_file)
                    .on_click(cx.listener(|this, _, _, cx| {
                        // "ListBoxが選択されていなかったら，先頭のファイルを選択する"
                        if this.selected.is_none() {
                            this.selected = candidates(cx, this.owner).first().map(|c| c.doc);
                        }
                        this.use_file = this.selected.is_none();
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("diff-windows")
                    .min_h(px(66.))
                    .max_h(px(140.))
                    .overflow_y_scroll()
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(rgb(0xb0b0b0))
                    .children(list),
            );
        let options = v_flex()
            .gap_1()
            .child(self.option(
                "diff-case",
                "大文字小文字の同一視(C)",
                o.ignore_case,
                |s, v| s.options.ignore_case = v,
                cx,
            ))
            .child(self.option(
                "diff-space",
                "空白無視(W)",
                o.ignore_all_space,
                |s, v| s.options.ignore_all_space = v,
                cx,
            ))
            .child(self.option(
                "diff-space-change",
                "空白変更無視(B)",
                o.ignore_space_change,
                |s, v| s.options.ignore_space_change = v,
                cx,
            ))
            .child(self.option(
                "diff-blank-lines",
                "空白行無視(L)",
                o.ignore_blank_lines,
                |s, v| s.options.ignore_blank_lines = v,
                cx,
            ))
            .child(self.option(
                "diff-tab-space",
                "タブ空白変換(T)",
                o.expand_tabs,
                |s, v| s.options.expand_tabs = v,
                cx,
            ));
        let side = v_flex()
            .gap_1()
            .child("編集中ファイルが...")
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("diff-new")
                            .label("新ファイル(N)")
                            .selected(!o.own_is_old)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.options.own_is_old = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("diff-old")
                            .label("旧ファイル(D)")
                            .selected(o.own_is_old)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.options.own_is_old = true;
                                cx.notify();
                            })),
                    ),
            )
            .child("次の差分(前の差分)へ移動時")
            .child(self.option(
                "diff-notify",
                "見つからないときにメッセージを表示(M)",
                self.notify_not_found,
                |s, v| s.notify_not_found = v,
                cx,
            ))
            .child(self.option(
                "diff-wrap",
                "先頭（末尾）から再検索する(Z)",
                self.wrap,
                |s, v| s.wrap = v,
                cx,
            ))
            .child(self.option(
                "diff-notify-none",
                "DIFF差分がないときにメッセージを表示",
                o.notify_no_diff,
                |s, v| s.options.notify_no_diff = v,
                cx,
            ));
        let buttons = h_flex()
            .gap_2()
            .child(
                Button::new("diff-ok")
                    .label("OK")
                    .on_click(cx.listener(|this, _, window, cx| this.ok(window, cx))),
            )
            .child(
                Button::new("diff-cancel")
                    .label("キャンセル(X)")
                    .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
            );
        div()
            .id("diff-panel")
            .absolute()
            // mouse events stop here: a click on the panel must not move the caret
            .occlude()
            .top(px(8.))
            .right(px(24.))
            .w(px(560.))
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
                    .child("DIFF差分表示"),
            )
            .child(format!("編集中のファイル: {}", self.own_label))
            .child(destination)
            .child(h_flex().gap_4().items_start().child(options).child(side))
            .child(buttons)
    }
}

/// The label of this window for the top of the dialogs: the name, ` *` when modified and the
/// encoding unless it is Shift_JIS (`GetMenuFullLabel_WinListNoEscape` without the number).
pub(crate) fn own_label(
    path: Option<&Path>,
    modified: bool,
    encoding: ume_core::encoding::Encoding,
) -> String {
    let bracket = if encoding == ume_core::encoding::Encoding::ShiftJis {
        String::new()
    } else {
        encoding.bracket_name()
    };
    format!(
        "{}{}{}",
        document_title(path),
        if modified { " *" } else { "" },
        bracket
    )
}

impl EditorView {
    /// `F_DIFF_DIALOG`: opens the DIFF dialog with what the last one left. The window that fits
    /// this window's file name best is selected.
    pub fn open_diff_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // one of the dialogs of this kind at a time
        self.close_compare_panel(window, cx);
        let owner = Owner {
            view: cx.entity_id(),
            document: self.document.entity_id(),
        };
        let (path, modified, encoding) = {
            let shared = self.document.read(cx);
            (
                shared.path().map(Path::to_path_buf),
                shared.model().is_modified(),
                shared.file().encoding,
            )
        };
        let others = candidates(cx, owner);
        let selected =
            best_candidate(path.as_deref(), encoding, &others, true).map(|i| others[i].doc);
        let label = own_label(path.as_deref(), modified, encoding);
        let search = self.search.options();
        let editor = cx.weak_entity();
        match &self.diff.panel {
            Some(panel) => panel.update(cx, |panel, cx| panel.focus_file(window, cx)),
            None => {
                let panel =
                    cx.new(|cx| DiffPanel::new(editor, owner, label, selected, search, window, cx));
                panel.update(cx, |panel, cx| panel.focus_file(window, cx));
                self.diff.panel = Some(panel);
            }
        }
        cx.notify();
    }

    /// Closes the dialog and gives the focus back to the text.
    pub fn close_diff_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.diff.panel.take().is_some() {
            window.focus(&self.focus_handle, cx);
            cx.notify();
        }
    }

    /// Whether the dialog is open.
    pub fn diff_panel_open(&self) -> bool {
        self.diff.panel.is_some()
    }
}
