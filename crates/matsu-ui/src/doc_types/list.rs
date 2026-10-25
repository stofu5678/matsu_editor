//! The list of document types: the dialog of `F_TYPE_LIST` (タイプ別設定一覧).
//!
//! Sakura: `Command_TYPE_LIST` (`sakura_core/cmd/CViewCommander_Settings.cpp`) shows
//! `CDlgTypeList` (`sakura_core/typeprop/CDlgTypeList.cpp`): a list of the types, each as
//! `名前 ( 拡張子 )` (just the name when the type has no extensions), the current type selected.
//! 一時適用 (or a double click) applies the selected type to this document with `F_CHANGETYPE`.
//! 設定変更 (the default button, so Enter; `IDOK`) opens the type settings of the selected type
//! (`OpenPropertySheetTypes`, `crate::type_settings`).
//!
//! The buttons that change the list are 追加 (`AddType`), 複製 (`CopyType`), 削除 (`DelType`), ↑ and ↓
//! (`UpType`, `DownType`), 初期化 (`InitializeType`), and インポート and エクスポート (`Import`, `Export`,
//! in `transfer`). The list stays open after each of them, with the type they made or moved selected.
//! Whether a button works for the selected type is `LBN_SELCHANGE` of `DispatchEvent`: ↑ from the
//! third type on, ↓ for every type but 基本 and the last, 削除 for every type but 基本; 追加 and
//! 複製 while there are less than 60 types (`SetData`).
//!
//! Drawn over the editor view, like the window list: the dialog of Sakura is a Windows dialog.
//! Not here: the file association (the two check boxes and `AlertFileAssociation`, which write to the
//! registry of Windows) and the help button.

use crate::dialog_focus::KeepTabInside as _;
use gpui::{
    App, AppContext as _, ClickEvent, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, ScrollHandle, StatefulInteractiveElement,
    Styled, WeakEntity, Window, div, px, rgb, rgba,
};
use matsu_config::settings::TypeSettings;

use super::transfer::Ascertain;
use crate::view::EditorView;

/// The line of a type in the list (`CDlgTypeList::SetData`).
pub(crate) fn label(t: &TypeSettings) -> String {
    if t.type_exts.is_empty() {
        t.type_name.clone()
    } else {
        format!("{} ( {} )", t.type_name, t.type_exts)
    }
}

pub(crate) struct TypeList {
    pub(super) view: WeakEntity<EditorView>,
    pub(super) labels: Vec<String>,
    pub(super) selected: usize,
    pub(super) focus: FocusHandle,
    pub(super) scroll: ScrollHandle,
    /// The question of the import (`CDlgTypeAscertain`), while it is open.
    pub(super) ascertain: Option<Ascertain>,
}

impl TypeList {
    fn new(
        view: WeakEntity<EditorView>,
        labels: Vec<String>,
        current: usize,
        cx: &mut App,
    ) -> Self {
        Self {
            view,
            selected: current.min(labels.len().saturating_sub(1)),
            labels,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            ascertain: None,
        }
    }

    /// Closes the list and gives the focus back to the editor.
    pub(super) fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.doc_type.list = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    /// 設定変更 (`IDOK`): closes the list and opens the type settings of the selected type, once
    /// the click or the key has ended (the sheet looks at the window's view).
    pub(super) fn change_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let index = self.selected;
        let opener = window.window_handle();
        self.close(window, cx);
        window.defer(cx, move |_, app| {
            crate::type_settings::open(app, index, None, Some(opener));
        });
    }

    /// 一時適用: `F_CHANGETYPE` with the selected type (the argument is the number, 1 origin).
    pub(super) fn apply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let number = i64::try_from(self.selected + 1).unwrap_or(0);
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.doc_type.list = None;
            window.focus(&view.focus_handle, cx);
            if view.change_type(number, cx) {
                view.run_auto_macro(crate::macros::Auto::TypeChanged, None, window, cx);
            }
            cx.notify();
        })
        .ok();
    }

    pub(super) fn on_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.ascertain.is_some() {
            self.on_ascertain_key(event, window, cx);
            return;
        }
        let last = self.labels.len().saturating_sub(1);
        match event.keystroke.key.as_str() {
            "escape" => self.close(window, cx),
            "enter" => self.change_settings(window, cx),
            "up" => self.selected = self.selected.saturating_sub(1),
            "down" => self.selected = (self.selected + 1).min(last),
            "pageup" => self.selected = self.selected.saturating_sub(10),
            "pagedown" => self.selected = (self.selected + 10).min(last),
            "home" => self.selected = 0,
            "end" => self.selected = last,
            _ => return,
        }
        self.scroll.scroll_to_item(self.selected);
        cx.stop_propagation();
        cx.notify();
    }
}

impl Focusable for TypeList {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

pub(super) const TEXT: u32 = 0x222222;
pub(super) const SELECTED_BG: u32 = 0x2f6fd0;

impl Render for TypeList {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.ascertain.is_some() {
            return self.render_ascertain(cx).into_any_element();
        }
        let items: Vec<_> = self
            .labels
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let mut item =
                    crate::a11y::list_row(div().id(("type", i)), text, i == self.selected)
                        .px_2()
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .cursor_pointer();
                if i == self.selected {
                    item = item.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff));
                }
                item.child(text.clone()).on_click(cx.listener(
                    move |this, event: &ClickEvent, window, cx| {
                        this.selected = i;
                        // A double click is 一時適用 (`OnLbnDblclk`).
                        if event.click_count() >= 2 {
                            this.apply(window, cx);
                        }
                        cx.notify();
                    },
                ))
            })
            .collect();
        let (has_room, can_up, can_down, can_delete) = (
            self.has_room(),
            self.can_move_up(),
            self.can_move_down(),
            self.can_delete(),
        );
        let max_rows = (window.viewport_size().height - px(170.)).max(px(66.));
        let rows = div()
            .id("type-list-rows")
            .role(gpui::Role::ListBox)
            .aria_label("タイプ")
            .max_h(max_rows)
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .children(items);
        let button = |id: &'static str, text: &'static str| {
            crate::a11y::button(div().id(id), text)
                .px_4()
                .h(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(rgb(0xaaaaaa))
                .bg(rgb(0xf4f4f4))
                .cursor_pointer()
                .child(text)
        };
        // A button that does not work for the selected type is grey and says so to a screen reader.
        let grey = |id: &'static str, text: &'static str, enabled: bool| {
            let b = button(id, text);
            if enabled { b } else { b.opacity(0.45) }
        };
        let side =
            div()
                .ml_3()
                .w(px(150.))
                .flex_none()
                .flex()
                .flex_col()
                .gap_1()
                .child(button("type-list-settings", "設定変更(S)...").on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.change_settings(window, cx)
                    }),
                ))
                .child(button("type-list-apply", "一時適用(R)").on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.apply(window, cx)),
                ))
                .child(button("type-list-cancel", "キャンセル(X)").on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.close(window, cx)),
                ))
                .child(div().h(px(10.)))
                .child(button("type-list-import", "インポート(I)...").on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.import(window, cx)),
                ))
                .child(button("type-list-export", "エクスポート(E)...").on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.export(window, cx)),
                ))
                .child(button("type-list-reset", "初期化(N)").on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.reset(window, cx)),
                ))
                .child(crate::a11y::disabled(
                    grey("type-list-copy", "複製(C)", has_room).on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.copy(window, cx)),
                    ),
                    !has_room,
                ))
                .child(crate::a11y::disabled(
                    grey("type-list-up", "↑(U)", can_up).on_click(cx.listener(
                        |this, _: &ClickEvent, window, cx| this.move_selected(true, window, cx),
                    )),
                    !can_up,
                ))
                .child(crate::a11y::disabled(
                    grey("type-list-down", "↓(O)", can_down).on_click(cx.listener(
                        |this, _: &ClickEvent, window, cx| this.move_selected(false, window, cx),
                    )),
                    !can_down,
                ))
                .child(crate::a11y::disabled(
                    grey("type-list-add", "追加(A)", has_room).on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.add(window, cx)),
                    ),
                    !has_room,
                ))
                .child(crate::a11y::disabled(
                    grey("type-list-delete", "削除(D)", can_delete).on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.delete(window, cx)),
                    ),
                    !can_delete,
                ));
        let panel = crate::a11y::dialog(div().id("type-list"), "タイプ別設定一覧")
            .aria_description("下からタイプを選択してください")
            .key_context("TypeList")
            .track_focus(&self.focus)
            .keep_tab_inside(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .min_w(px(480.))
            .max_w(px(760.))
            .p_4()
            .flex()
            .flex_col()
            .bg(rgb(0xf0f0f0))
            .border_1()
            .border_color(rgb(0x888888))
            .text_color(rgb(TEXT))
            .text_size(px(14.))
            .child(div().mb_2().child("タイプ別設定一覧"))
            .child(div().mb_1().child("下からタイプを選択してください:"))
            .child(
                div()
                    .flex()
                    .child(
                        div()
                            .flex_1()
                            .bg(rgb(0xffffff))
                            .border_1()
                            .border_color(rgb(0xaaaaaa))
                            .min_h(px(120.))
                            .child(rows),
                    )
                    .child(side),
            );
        div()
            .id("type-list-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .flex()
            .bg(rgba(0x00000055))
            .items_center()
            .justify_center()
            .child(panel)
            .into_any_element()
    }
}

/// Shows the list over `view`. A list that is open is closed instead of opening a second one.
pub(crate) fn open(view: &mut EditorView, window: &mut Window, cx: &mut Context<EditorView>) {
    if view.doc_type.list.take().is_some() {
        window.focus(&view.focus_handle, cx);
        cx.notify();
        return;
    }
    let (labels, current) = view.type_labels(cx);
    let weak = cx.weak_entity();
    let list = cx.new(|cx| TypeList::new(weak, labels, current, cx));
    let focus = list.read(cx).focus.clone();
    window.focus(&focus, cx);
    view.doc_type.list = Some(list);
    cx.notify();
}

#[cfg(test)]
mod tests {
    use super::*;
    use matsu_config::settings::DocumentTypes;

    #[test]
    fn a_line_is_the_name_and_the_extensions() {
        let types = DocumentTypes::builtin();
        // 基本 has no extensions.
        assert_eq!(label(types.get(0).unwrap()), "基本");
        assert_eq!(
            label(types.get(1).unwrap()),
            "テキスト ( txt,log,1st,err,ps )"
        );
    }
}
