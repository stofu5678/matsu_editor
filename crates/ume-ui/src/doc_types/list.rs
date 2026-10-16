//! The list of document types: the dialog of `F_TYPE_LIST` (タイプ別設定一覧).
//!
//! Sakura: `Command_TYPE_LIST` (`sakura_core/cmd/CViewCommander_Settings.cpp`) shows
//! `CDlgTypeList` (`sakura_core/typeprop/CDlgTypeList.cpp`): a list of the types, each as
//! `名前 ( 拡張子 )` (just the name when the type has no extensions), the current type selected.
//! 一時適用 (or a double click) applies the selected type to this document with `F_CHANGETYPE`.
//!
//! Drawn over the editor view, like the window list: the dialog of Sakura is a Windows dialog.
//! Only 一時適用 and キャンセル are here; the buttons that change the types (設定変更, インポート,
//! エクスポート, 初期化, 複製, 上, 下, 追加, 削除, and the file association) need the full type
//! settings dialog (`F_OPTION_TYPE`), which comes later.

use gpui::{
    App, AppContext as _, ClickEvent, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, ScrollHandle, StatefulInteractiveElement,
    Styled, WeakEntity, Window, div, px, rgb, rgba,
};
use ume_config::settings::TypeSettings;

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
    view: WeakEntity<EditorView>,
    labels: Vec<String>,
    selected: usize,
    focus: FocusHandle,
    scroll: ScrollHandle,
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
        }
    }

    /// Closes the list and gives the focus back to the editor.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.doc_type.list = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    /// 一時適用: `F_CHANGETYPE` with the selected type (the argument is the number, 1 origin).
    fn apply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let number = i64::try_from(self.selected + 1).unwrap_or(0);
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.doc_type.list = None;
            window.focus(&view.focus_handle, cx);
            view.change_type(number, cx);
            cx.notify();
        })
        .ok();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let last = self.labels.len().saturating_sub(1);
        match event.keystroke.key.as_str() {
            "escape" => self.close(window, cx),
            "enter" => self.apply(window, cx),
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

const TEXT: u32 = 0x222222;
const SELECTED_BG: u32 = 0x2f6fd0;

impl Render for TypeList {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items: Vec<_> = self
            .labels
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let mut item = div()
                    .id(("type", i))
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
        let max_rows = (window.viewport_size().height - px(170.)).max(px(66.));
        let rows = div()
            .id("type-list-rows")
            .max_h(max_rows)
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .children(items);
        let button = |id: &'static str, text: &'static str| {
            div()
                .id(id)
                .px_4()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(0xaaaaaa))
                .bg(rgb(0xf4f4f4))
                .cursor_pointer()
                .child(text)
        };
        let panel = div()
            .id("type-list")
            .key_context("TypeList")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .min_w(px(320.))
            .max_w(px(640.))
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
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(rgb(0xaaaaaa))
                    .min_h(px(120.))
                    .child(rows),
            )
            .child(
                div()
                    .mt_4()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(button("type-list-apply", "一時適用(R)").on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.apply(window, cx)),
                    ))
                    .child(button("type-list-cancel", "キャンセル(X)").on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.close(window, cx)),
                    )),
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
    use ume_config::settings::DocumentTypes;

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
