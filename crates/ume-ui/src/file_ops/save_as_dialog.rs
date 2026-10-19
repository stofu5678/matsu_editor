//! The options of "名前を付けて保存": the encoding, the BOM and the line ending.
//!
//! Sakura adds these three controls to the save dialog (`sakura_core/dlg/CDlgOpenFile_*.cpp`:
//! `IDC_COMBO_CODE`, `IDC_CHECK_BOM`, `IDC_COMBO_EOL`). The native dialogs of the OS cannot take
//! extra fields, so this small dialog is drawn in the editor window right after the path has been
//! chosen. It is modal to the window: it covers the view, and the view takes no keys meanwhile.
//!
//! Behavior taken from Sakura:
//!
//! * The encoding list is the encoding combo box of the dialog, without "Auto"
//!   (`CDlgOpenFile_CommonFileDialog.cpp`, `OnInitDialog` for `IDC_COMBO_CODE`).
//! * The line ending list is "変換なし", "CR+LF", "LF (UNIX)", "CR (Mac)" (`pEolNameArr`).
//! * The BOM check box is enabled only for encodings that can have a BOM
//!   (`CCodeTypeName::UseBom`). When the encoding changes, it is unchecked and disabled for the
//!   ones that cannot; for the encoding the document has now it takes the document's own BOM;
//!   for any other it takes the default of that encoding (`IsBomDefOn`). This is the
//!   `CBN_SELCHANGE` code of `IDC_COMBO_CODE`.
//!
//! Keys: Enter is OK (or the focused Cancel button), Esc cancels, Tab and Shift+Tab move between
//! the fields, the arrow keys choose in the lists and Space toggles the BOM.

use std::cell::RefCell;
use std::future::{Future, poll_fn};
use std::rc::Rc;
use std::task::{Poll, Waker};

use gpui::{
    AppContext as _, AsyncWindowContext, ClickEvent, Context, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, ScrollHandle,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, prelude::FluentBuilder, px, rgb,
    rgba,
};
use ume_core::buffer::Eol;
use ume_core::encoding::Encoding;

use crate::view::EditorView;

/// The encodings the dialog offers, in the order of the list: Sakura's nine, then the Windows code
/// pages ([`Encoding::choices`]). This is the one place where the UI asks the core for the list;
/// the rest of the dialog works on whatever this returns.
pub(crate) fn encoding_choices() -> Vec<Encoding> {
    Encoding::choices()
}

/// The line ending choices (`pEolNameArr`). `None` is "変換なし": keep the endings as they are.
const EOL_CHOICES: [(&str, Option<Eol>); 4] = [
    ("変換なし", None),
    ("CR+LF", Some(Eol::CrLf)),
    ("LF (UNIX)", Some(Eol::Lf)),
    ("CR (Mac)", Some(Eol::Cr)),
];

/// What the user chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SaveChoice {
    pub encoding: Encoding,
    /// Already false for an encoding that cannot have a BOM.
    pub bom: bool,
    /// `None` is "変換なし".
    pub eol: Option<Eol>,
}

/// The state of the three fields, without the window.
#[derive(Clone, Debug)]
struct State {
    choices: Vec<Encoding>,
    selected: usize,
    bom: bool,
    /// The encoding and the BOM of the document now.
    current: (Encoding, bool),
    eol: usize,
}

impl State {
    fn new(mut choices: Vec<Encoding>, current: (Encoding, bool), eol: Option<Eol>) -> Self {
        if choices.is_empty() {
            choices.push(current.0);
        }
        Self {
            selected: choices.iter().position(|e| *e == current.0).unwrap_or(0),
            choices,
            bom: current.1 && current.0.can_have_bom(),
            current,
            eol: EOL_CHOICES.iter().position(|c| c.1 == eol).unwrap_or(0),
        }
    }

    fn encoding(&self) -> Encoding {
        self.choices[self.selected]
    }

    fn bom_enabled(&self) -> bool {
        self.encoding().can_have_bom()
    }

    /// Chooses the encoding at `index` (clamped) and sets the BOM as Sakura does.
    fn select(&mut self, index: usize) {
        let index = index.min(self.choices.len() - 1);
        if index == self.selected {
            return;
        }
        self.selected = index;
        let encoding = self.encoding();
        self.bom = if !encoding.can_have_bom() {
            false
        } else if encoding == self.current.0 {
            self.current.1
        } else {
            encoding.bom_default_on()
        };
    }

    fn move_encoding(&mut self, delta: isize) {
        self.select(self.selected.saturating_add_signed(delta));
    }

    fn toggle_bom(&mut self) {
        if self.bom_enabled() {
            self.bom = !self.bom;
        }
    }

    fn move_eol(&mut self, delta: isize) {
        self.eol = self
            .eol
            .saturating_add_signed(delta)
            .min(EOL_CHOICES.len() - 1);
    }

    fn choice(&self) -> SaveChoice {
        SaveChoice {
            encoding: self.encoding(),
            bom: self.bom && self.bom_enabled(),
            eol: EOL_CHOICES[self.eol].1,
        }
    }
}

// ---- a one-shot answer, without a new dependency ----

struct Slot {
    value: Option<Option<SaveChoice>>,
    waker: Option<Waker>,
}

/// Sends the answer once. Dropping it without an answer (the window closed) answers `None`.
struct AnswerSender(Rc<RefCell<Slot>>);

impl AnswerSender {
    fn send(&self, value: Option<SaveChoice>) {
        let mut slot = self.0.borrow_mut();
        if slot.value.is_none() {
            slot.value = Some(value);
        }
        if let Some(waker) = slot.waker.take() {
            waker.wake();
        }
    }
}

impl Drop for AnswerSender {
    fn drop(&mut self) {
        self.send(None);
    }
}

fn answer_channel() -> (AnswerSender, impl Future<Output = Option<SaveChoice>>) {
    let slot = Rc::new(RefCell::new(Slot {
        value: None,
        waker: None,
    }));
    let receiver = slot.clone();
    let wait = poll_fn(move |task| {
        let mut slot = receiver.borrow_mut();
        match slot.value.take() {
            Some(value) => Poll::Ready(value),
            None => {
                slot.waker = Some(task.waker().clone());
                Poll::Pending
            }
        }
    });
    (AnswerSender(slot), wait)
}

// ---- the dialog ----

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Encoding,
    Bom,
    Eol,
    Ok,
    Cancel,
}

const FIELDS: [Field; 5] = [
    Field::Encoding,
    Field::Bom,
    Field::Eol,
    Field::Ok,
    Field::Cancel,
];

pub(crate) struct SaveAsDialog {
    state: State,
    /// Whether the dialog has the line ending (the Save As); 文字コードセット指定 has not
    /// (`IDD_SETCHARSET` has the encoding and the BOM only).
    with_eol: bool,
    view: WeakEntity<EditorView>,
    answer: Option<AnswerSender>,
    encoding_focus: FocusHandle,
    bom_focus: FocusHandle,
    eol_focus: FocusHandle,
    ok_focus: FocusHandle,
    cancel_focus: FocusHandle,
    list_scroll: ScrollHandle,
}

impl SaveAsDialog {
    fn new(
        state: State,
        with_eol: bool,
        view: WeakEntity<EditorView>,
        answer: AnswerSender,
        cx: &mut Context<Self>,
    ) -> Self {
        let dialog = Self {
            state,
            with_eol,
            view,
            answer: Some(answer),
            encoding_focus: cx.focus_handle(),
            bom_focus: cx.focus_handle(),
            eol_focus: cx.focus_handle(),
            ok_focus: cx.focus_handle(),
            cancel_focus: cx.focus_handle(),
            list_scroll: ScrollHandle::new(),
        };
        dialog.list_scroll.scroll_to_item(dialog.state.selected);
        dialog
    }

    fn handle(&self, field: Field) -> &FocusHandle {
        match field {
            Field::Encoding => &self.encoding_focus,
            Field::Bom => &self.bom_focus,
            Field::Eol => &self.eol_focus,
            Field::Ok => &self.ok_focus,
            Field::Cancel => &self.cancel_focus,
        }
    }

    /// The fields Tab visits, in order. The BOM is skipped while it is disabled.
    fn tab_order(&self) -> Vec<Field> {
        FIELDS
            .into_iter()
            .filter(|f| *f != Field::Bom || self.state.bom_enabled())
            .filter(|f| *f != Field::Eol || self.with_eol)
            .collect()
    }

    fn focused(&self, window: &Window) -> Option<Field> {
        FIELDS
            .into_iter()
            .find(|f| self.handle(*f).is_focused(window))
    }

    fn move_focus(&self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let order = self.tab_order();
        let at = self
            .focused(window)
            .and_then(|f| order.iter().position(|o| *o == f));
        let next = match (at, forward) {
            (Some(i), true) => (i + 1) % order.len(),
            (Some(i), false) => (i + order.len() - 1) % order.len(),
            (None, true) => 0,
            (None, false) => order.len() - 1,
        };
        window.focus(self.handle(order[next]), cx);
    }

    /// Ends the dialog with `choice` (`None` cancels) and gives the focus back to the view.
    fn finish(&mut self, choice: Option<SaveChoice>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(answer) = self.answer.take() {
            answer.send(choice);
        }
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.save_as_dialog = None;
            // The window list (Save from its dialog) stays open and keeps the keys.
            match view.window_list.clone() {
                Some(list) => window.focus(&list.focus_handle(cx), cx),
                None => window.focus(&view.focus_handle, cx),
            }
            cx.notify();
        })
        .ok();
    }

    fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let choice = self.state.choice();
        self.finish(Some(choice), window, cx);
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish(None, window, cx);
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let focused = self.focused(window);
        match (key, focused) {
            ("escape", _) => self.cancel(window, cx),
            ("tab", _) => self.move_focus(!event.keystroke.modifiers.shift, window, cx),
            ("enter", Some(Field::Cancel)) => self.cancel(window, cx),
            ("enter", _) => self.ok(window, cx),
            ("space", Some(Field::Bom)) => self.state.toggle_bom(),
            ("space", Some(Field::Ok)) => self.ok(window, cx),
            ("space", Some(Field::Cancel)) => self.cancel(window, cx),
            ("up", Some(Field::Encoding)) => self.state.move_encoding(-1),
            ("down", Some(Field::Encoding)) => self.state.move_encoding(1),
            ("pageup", Some(Field::Encoding)) => self.state.move_encoding(-8),
            ("pagedown", Some(Field::Encoding)) => self.state.move_encoding(8),
            ("home", Some(Field::Encoding)) => self.state.select(0),
            ("end", Some(Field::Encoding)) => self.state.select(usize::MAX),
            ("up" | "left", Some(Field::Eol)) => self.state.move_eol(-1),
            ("down" | "right", Some(Field::Eol)) => self.state.move_eol(1),
            _ => return,
        }
        self.list_scroll.scroll_to_item(self.state.selected);
        cx.stop_propagation();
        cx.notify();
    }
}

impl Focusable for SaveAsDialog {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.encoding_focus.clone()
    }
}

const PANEL_TEXT: u32 = 0x222222;
const SELECTED_BG: u32 = 0x2f6fd0;
const DISABLED_TEXT: u32 = 0x999999;

/// One row of a list: highlighted when it is the chosen one.
fn row(selected: bool) -> gpui::Div {
    let row = div().px_2().h(px(24.)).flex().items_center();
    if selected {
        row.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
    } else {
        row
    }
}

impl Render for SaveAsDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focused(window);
        let ring = |field: Field| {
            if focused == Some(field) {
                SELECTED_BG
            } else {
                0xaaaaaa
            }
        };

        let rows: Vec<_> = self
            .state
            .choices
            .iter()
            .enumerate()
            .map(|(i, encoding)| {
                row(i == self.state.selected)
                    .id(("encoding", i))
                    .child(encoding.combo_name())
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.state.select(i);
                        window.focus(&this.encoding_focus, cx);
                        cx.notify();
                    }))
            })
            .collect();
        let list = div()
            .id("encoding-list")
            .track_focus(&self.encoding_focus)
            .h(px(200.))
            .w_full()
            .overflow_y_scroll()
            .track_scroll(&self.list_scroll)
            .border_1()
            .border_color(rgb(ring(Field::Encoding)))
            .bg(rgb(0xffffff))
            .children(rows);

        let bom_enabled = self.state.bom_enabled();
        let bom = div()
            .id("bom")
            .track_focus(&self.bom_focus)
            .flex()
            .items_center()
            .gap_2()
            .px_1()
            .border_1()
            .border_color(rgb(ring(Field::Bom)))
            .text_color(rgb(if bom_enabled {
                PANEL_TEXT
            } else {
                DISABLED_TEXT
            }))
            .child(
                div()
                    .w(px(16.))
                    .h(px(16.))
                    .border_1()
                    .border_color(rgb(0x666666))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if self.state.bom && bom_enabled {
                        "✓"
                    } else {
                        ""
                    }),
            )
            .child("BOM を付ける")
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.state.toggle_bom();
                if this.state.bom_enabled() {
                    window.focus(&this.bom_focus, cx);
                }
                cx.notify();
            }));

        let eol_items: Vec<_> = EOL_CHOICES
            .iter()
            .enumerate()
            .map(|(i, (name, _))| {
                row(i == self.state.eol)
                    .id(("eol", i))
                    .child(*name)
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.state.eol = i;
                        window.focus(&this.eol_focus, cx);
                        cx.notify();
                    }))
            })
            .collect();
        let eol = div()
            .id("eol-group")
            .track_focus(&self.eol_focus)
            .flex()
            .border_1()
            .border_color(rgb(ring(Field::Eol)))
            .bg(rgb(0xffffff))
            .children(eol_items);

        let button = |id: &'static str, label: &'static str, field: Field| {
            div()
                .id(id)
                .track_focus(self.handle(field))
                .px_4()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(ring(field)))
                .bg(rgb(0xf4f4f4))
                .child(label)
        };
        let ok = button("ok", "OK", Field::Ok)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.ok(window, cx)));
        let cancel = button("cancel", "キャンセル", Field::Cancel)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.cancel(window, cx)));

        let label = |text: &'static str| div().mt_2().mb_1().child(text);
        let panel = div()
            .id("save-as-options")
            .key_context("SaveAsDialog")
            .on_key_down(cx.listener(Self::on_key_down))
            .w(px(360.))
            .p_4()
            .flex()
            .flex_col()
            .bg(rgb(0xf0f0f0))
            .border_1()
            .border_color(rgb(0x888888))
            .text_color(rgb(PANEL_TEXT))
            .text_size(px(14.))
            .child(if self.with_eol {
                "名前を付けて保存"
            } else {
                // `IDD_SETCHARSET`
                "文字コードの指定"
            })
            .child(label("文字コードセット"))
            .child(list)
            .child(div().mt_2().child(bom))
            .when(self.with_eol, |this| {
                this.child(label("改行コード")).child(eol)
            })
            .child(
                div()
                    .mt_4()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(ok)
                    .child(cancel),
            );
        div()
            .id("save-as-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .bg(rgba(0x00000055))
            .flex()
            .items_center()
            .justify_center()
            .child(panel)
    }
}

/// Shows the dialog over `view` and waits for the answer. `None` if the user cancelled, the
/// window went away, or another such dialog is already open.
pub(crate) async fn ask_options(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    current: (Encoding, bool),
    eol: Option<Eol>,
) -> Option<SaveChoice> {
    ask(view, cx, State::new(encoding_choices(), current, eol), true).await
}

/// The dialog of 文字コードセット指定 (`CDlgSetCharSet`): the encoding and the BOM of `current`,
/// asked in the same dialog as the Save As, without the line ending. `None` if cancelled.
pub(crate) async fn ask_charset(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    current: (Encoding, bool),
) -> Option<(Encoding, bool)> {
    let state = State::new(encoding_choices(), current, None);
    let choice = ask(view, cx, state, false).await?;
    Some((choice.encoding, choice.bom))
}

async fn ask(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    state: State,
    with_eol: bool,
) -> Option<SaveChoice> {
    let (sender, wait) = answer_channel();
    let weak = view.clone();
    let opened = view
        .update_in(cx, move |view, window, cx| {
            if view.save_as_dialog.is_some() {
                return false;
            }
            let dialog = cx.new(|cx| SaveAsDialog::new(state, with_eol, weak, sender, cx));
            let first = dialog.read(cx).encoding_focus.clone();
            window.focus(&first, cx);
            view.save_as_dialog = Some(dialog);
            cx.notify();
            true
        })
        .ok()?;
    if !opened {
        return None;
    }
    let choice = wait.await;
    // The window may have gone away with the dialog open; then there is nothing to clear.
    view.update(cx, |view, cx| {
        if view.save_as_dialog.take().is_some() {
            cx.notify();
        }
    })
    .ok();
    choice
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(current: Encoding, bom: bool) -> State {
        State::new(encoding_choices(), (current, bom), None)
    }

    fn index_of(s: &State, encoding: Encoding) -> usize {
        s.choices.iter().position(|e| *e == encoding).unwrap()
    }

    #[test]
    fn the_list_has_sakuras_encodings_and_then_the_code_pages() {
        let choices = encoding_choices();
        assert_eq!(&choices[..Encoding::ALL.len()], &Encoding::ALL[..]);
        let gbk = Encoding::CodePage(ume_core::encoding::CodePage::new(936).unwrap());
        assert!(choices.contains(&gbk));
        // a code page has no BOM, so choosing one turns the check box off
        let mut s = state(Encoding::Utf8, true);
        s.select(index_of(&s, gbk));
        assert!(!s.choice().bom);
    }

    #[test]
    fn the_current_values_are_preselected() {
        let s = State::new(encoding_choices(), (Encoding::Utf8, true), Some(Eol::Lf));
        assert_eq!(
            s.choice(),
            SaveChoice {
                encoding: Encoding::Utf8,
                bom: true,
                eol: Some(Eol::Lf)
            }
        );
        assert_eq!(state(Encoding::ShiftJis, false).choice().eol, None);
    }

    #[test]
    fn a_stale_bom_is_dropped_for_an_encoding_without_one() {
        // CDlgOpenFile_CommonFileDialog.cpp: the box is disabled and unchecked for these.
        let s = state(Encoding::ShiftJis, true);
        assert!(!s.bom);
        assert!(!s.bom_enabled());
    }

    #[test]
    fn the_bom_follows_the_encoding_as_in_sakura() {
        let mut s = state(Encoding::Utf8, true);
        // UTF-16LE: default on.
        s.select(index_of(&s, Encoding::Utf16Le));
        assert!(s.bom && s.bom_enabled());
        // UTF-8 again (the document's encoding): the document's own BOM.
        s.select(index_of(&s, Encoding::Utf8));
        assert!(s.bom);
        // Shift_JIS: unchecked and disabled, and toggling does nothing.
        s.select(0);
        assert!(!s.bom && !s.bom_enabled());
        s.toggle_bom();
        assert!(!s.choice().bom);
        // CESU-8 is not the document's encoding: default off.
        s.select(index_of(&s, Encoding::Cesu8));
        assert!(!s.bom && s.bom_enabled());
        s.toggle_bom();
        assert!(s.choice().bom);
    }

    #[test]
    fn moving_stays_inside_the_lists() {
        let mut s = state(Encoding::ShiftJis, false);
        s.move_encoding(-1);
        assert_eq!(s.selected, 0);
        s.move_encoding(1000);
        assert_eq!(s.selected, s.choices.len() - 1);
        s.move_eol(-1);
        assert_eq!(s.choice().eol, None);
        s.move_eol(2);
        assert_eq!(s.choice().eol, Some(Eol::Lf));
        s.move_eol(10);
        assert_eq!(s.choice().eol, Some(Eol::Cr));
    }

    #[test]
    fn an_encoding_missing_from_the_list_does_not_break_the_state() {
        let s = State::new(vec![Encoding::Utf8], (Encoding::Latin1, false), None);
        assert_eq!(s.encoding(), Encoding::Utf8);
        let s = State::new(Vec::new(), (Encoding::Latin1, false), None);
        assert_eq!(s.encoding(), Encoding::Latin1);
    }
}
