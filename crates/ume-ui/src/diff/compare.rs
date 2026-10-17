//! ファイル内容比較 (`F_COMPARE`): the dialog that chooses the window to compare with, and the
//! comparison.
//!
//! Sakura: `Command_COMPARE` (`sakura_core/cmd/CViewCommander_Diff.cpp`), the dialog
//! `CDlgCompare` (`sakura_core/dlg/CDlgCompare.cpp`, `IDD_COMPARE`). The two windows are compared
//! from their carets, char by char, up to the first place where they differ
//! (`ume_core::diff::first_difference`); both carets go there, and the window compared with says
//! whether it found a difference. The dialog is a panel in the editor window. Sakura can also
//! tile the two windows side by side afterwards (比較後、左右に並べて表示).
//!
//! Left out: the button ヘルプ, and the disabling of the tiling for Sakura's one-window tab mode,
//! which this program does not have.

use gpui::{
    App, AppContext as _, ClickEvent, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, StatefulInteractiveElement, Styled,
    WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::{h_flex, v_flex};
use ume_core::diff::first_difference;

use super::panel::own_label;
use super::{
    Candidate, DocRef, Owner, Remembered, best_candidate, candidates, find_document, remember,
    remembered,
};
use crate::view::EditorView;
use crate::window_ops::{tile_windows, with_screen};

/// `STR_ERR_CEDITVIEW_CMD22`: nothing differs.
pub(crate) const NO_DIFFERENCE: &str = "異なる箇所は見つかりませんでした。";
/// `STR_ERR_CEDITVIEW_CMD23`: the carets are on the first difference.
pub(crate) const FOUND_DIFFERENCE: &str = "異なる箇所が見つかりました。";

pub struct ComparePanel {
    editor: WeakEntity<EditorView>,
    owner: Owner,
    own_label: String,
    selected: Option<DocRef>,
    tile: bool,
    focus: FocusHandle,
}

impl ComparePanel {
    fn new(
        editor: WeakEntity<EditorView>,
        owner: Owner,
        own_label: String,
        selected: Option<DocRef>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            editor,
            owner,
            own_label,
            selected,
            tile: remembered(cx).tile,
            focus: cx.focus_handle(),
        }
    }

    pub(crate) fn select_window(&mut self, doc: DocRef, cx: &mut Context<Self>) {
        self.selected = Some(doc);
        cx.notify();
    }

    pub(crate) fn set_tile(&mut self, tile: bool, cx: &mut Context<Self>) {
        self.tile = tile;
        cx.notify();
    }

    /// OK: compares. Without a selected window the dialog just closes
    /// (`GetData` is false and `Command_COMPARE` returns).
    pub(crate) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (selected, tile) = (self.selected, self.tile);
        if selected.is_some() {
            let previous = remembered(cx);
            remember(cx, Remembered { tile, ..previous });
        }
        let Some(editor) = self.editor.upgrade() else {
            return;
        };
        editor.update(cx, |editor, cx| {
            editor.close_compare_panel(window, cx);
            if let Some(doc) = selected {
                editor.compare_with(doc, tile, window, cx);
            }
        });
    }

    pub(crate) fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| editor.close_compare_panel(window, cx))
            .ok();
    }
}

impl Focusable for ComparePanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ComparePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<Candidate> = candidates(cx, self.owner);
        let list = rows.iter().enumerate().map(|(i, row)| {
            let doc = row.doc;
            let mut item = div()
                .id(("compare-window", i))
                .px_2()
                .h(px(22.))
                .flex()
                .items_center()
                .cursor_pointer()
                .child(row.label.clone())
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.select_window(doc, cx);
                }));
            if self.selected == Some(doc) {
                item = item.bg(rgb(0x2f6fd0)).text_color(rgb(0xffffff));
            }
            item
        });
        let buttons = h_flex()
            .gap_2()
            .child(
                Button::new("compare-ok")
                    .label("OK")
                    .on_click(cx.listener(|this, _, window, cx| this.ok(window, cx))),
            )
            .child(
                Button::new("compare-cancel")
                    .label("キャンセル(X)")
                    .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
            );
        v_flex()
            .id("compare-panel")
            .absolute()
            .occlude()
            .top(px(8.))
            .right(px(24.))
            .w(px(460.))
            .p_2()
            .gap_2()
            .bg(rgb(0xf6f6f6))
            .text_color(rgb(0x1e1e1e))
            .text_size(px(13.))
            .border_1()
            .border_color(rgb(0xb0b0b0))
            .rounded_md()
            .shadow_md()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => this.cancel(window, cx),
                    "enter" => this.ok(window, cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .child(
                div()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("ファイル内容比較"),
            )
            .child(format!("ソースファイル: {}", self.own_label))
            .child("相手のファイル")
            .child(
                div()
                    .id("compare-windows")
                    .min_h(px(88.))
                    .max_h(px(200.))
                    .overflow_y_scroll()
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(rgb(0xb0b0b0))
                    .children(list),
            )
            .child(
                Checkbox::new("compare-tile")
                    .label("左右に並べて表示(T)")
                    .checked(self.tile)
                    .on_click(cx.listener(|this, value: &bool, _, cx| this.set_tile(*value, cx))),
            )
            .child(buttons)
    }
}

impl EditorView {
    /// `F_COMPARE`: opens the dialog. The window whose file name fits this window's best is
    /// selected (`FileMatchScoreSepExt`).
    pub fn open_compare_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_diff_panel(window, cx);
        let owner = Owner {
            view: cx.entity_id(),
            document: self.document.entity_id(),
        };
        let (path, modified, encoding) = {
            let shared = self.document.read(cx);
            (
                shared.path().map(std::path::Path::to_path_buf),
                shared.model().is_modified(),
                shared.file().encoding,
            )
        };
        let others = candidates(cx, owner);
        // the list of Sakura selects the first window when none matches
        let selected =
            best_candidate(path.as_deref(), encoding, &others, false).map(|i| others[i].doc);
        let label = own_label(path.as_deref(), modified, encoding);
        let editor = cx.weak_entity();
        match &self.diff.compare {
            Some(panel) => {
                let focus = panel.read(cx).focus.clone();
                window.focus(&focus, cx);
            }
            None => {
                let panel = cx.new(|cx| ComparePanel::new(editor, owner, label, selected, cx));
                let focus = panel.read(cx).focus.clone();
                window.focus(&focus, cx);
                self.diff.compare = Some(panel);
            }
        }
        cx.notify();
    }

    /// Closes the dialog and gives the focus back to the text.
    pub fn close_compare_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.diff.compare.take().is_some() {
            window.focus(&self.focus_handle, cx);
            cx.notify();
        }
    }

    /// Whether the dialog is open.
    pub fn compare_panel_open(&self) -> bool {
        self.diff.compare.is_some()
    }

    /// The body of `Command_COMPARE`: compares this window with `handle` from the carets of both,
    /// tiles the two windows if asked to, moves both carets to the first difference, and says
    /// so in a box on the other window.
    pub(crate) fn compare_with(
        &mut self,
        doc: DocRef,
        tile: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let this = cx.entity_id();
        let Some((handle, other)) = find_document(cx, this, doc) else {
            return;
        };
        let own_caret = self.cursor.caret();
        let other_caret = other.read(cx).cursor().caret();
        let found = first_difference(
            self.document.read(cx).doc(),
            own_caret,
            other.read(cx).document().read(cx).doc(),
            other_caret,
        );
        let found = match found {
            Ok(found) => found,
            Err(e) => {
                self.warning_box(&format!("比較できなかった: {e}"), window, cx);
                return;
            }
        };
        // Another tab of this window has no window to place or to ask for a box.
        let same_window = handle == window.window_handle();
        if tile && !same_window {
            // The OS may not be able to place windows (Linux); the comparison goes on.
            let _ = with_screen(window, cx, |screen, _, current| {
                tile_windows(screen, &[current, handle], current, false)
            });
        }
        match found {
            None => {
                if same_window {
                    other.update(cx, |view, cx| view.info_box(NO_DIFFERENCE, window, cx));
                } else {
                    handle
                        .update(cx, |_, other_window, app| {
                            other.update(app, |view, cx| {
                                view.info_box(NO_DIFFERENCE, other_window, cx);
                            });
                        })
                        .ok();
                }
            }
            Some((own, theirs)) => {
                // the carets move before the message ("位置を変更してからメッセージ")
                self.run_with(cx, |c| c.move_caret(own, false)).ok();
                if same_window {
                    other.update(cx, |view, cx| {
                        view.run_with(cx, |c| c.move_caret(theirs, false)).ok();
                        view.warning_box(FOUND_DIFFERENCE, window, cx);
                    });
                } else {
                    handle
                        .update(cx, |_, other_window, app| {
                            other.update(app, |view, cx| {
                                view.run_with(cx, |c| c.move_caret(theirs, false)).ok();
                                view.warning_box(FOUND_DIFFERENCE, other_window, cx);
                            });
                        })
                        .ok();
                }
            }
        }
        // "開いているウィンドウをアクティブにする"
        window.activate_window();
    }
}
