//! The print setting dialog (`IDD_PRINTSETTING`, `CDlgPrintSetting`).
//!
//! Sakura: `sakura_core/dlg/CDlgPrintSetting.cpp`; the layout is `IDD_PRINTSETTING` in
//! `sakura_core/sakura_rc.rc`. It is a modal panel over the view, like the other dialogs of this
//! program. The drop-down lists of Sakura are buttons that open the list below them.
//!
//! The dialog works on a copy of the eight settings. Choosing another setting in the first list
//! keeps what the controls hold for the one that was shown (`OnChangeSettingType( TRUE )`).
//! OK checks the values (`DataCheckAndCorrect`) and gives the eight back with the number of the
//! chosen one; OK cannot be pressed while the paper has no room for a character
//! (`CalcPrintableLineAndColumn`).
//!
//! The keys of a drop-down list (the arrows, Home, End, PageUp, PageDown, Enter, Escape) and the
//! access keys of the labels (Alt+letter, `access_keys`) work as in Sakura's dialog.
//!
//! Not done: the spin buttons next to the numbers, and the help button.

use crate::access_keys::{Entry, Found, OnAccessKey as _, pick};
use crate::dialog_focus::KeepTabInside as _;
use crate::input_select::{focus_selecting, input_field};
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Render, ScrollHandle, StatefulInteractiveElement,
    Styled, Subscription, WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::radio::Radio;
use gpui_component::scroll::{Scrollbar, ScrollbarMode};
use gpui_component::{Disableable as _, h_flex, v_flex};
use matsu_config::settings::print::{NAME_MAX, PrintSettings};
use matsu_config::settings::{FontSetting, LogFont};
use matsu_print::{HeaderFooterFont, Orientation, PAPER_INFO, PrintSetting, find_paper_info};
use std::borrow::Cow;

use super::model::{self, Number};
use crate::font_dialog::model::families;
use crate::input_dialog::InputRequest;
use crate::view::EditorView;

/// The labels that show an access key (`IDD_PRINTSETTING`: `sakura_core/sakura_rc.rc`).
mod labels {
    pub(super) const SETTING: &str = "ページ設定(1)";
    pub(super) const RENAME: &str = "設定名変更(2)...";
    pub(super) const FONT_HAN: &str = "半角フォント(F)";
    pub(super) const FONT_ZEN: &str = "全角フォント(K)";
    pub(super) const FONT_HEIGHT: &str = "フォント高(P)";
    pub(super) const LINE_SPACING: &str = "行送り(S)";
    pub(super) const DANSUU: &str = "段数(C)";
    pub(super) const DAN_SPACE: &str = "段の隙間(E)";
    pub(super) const PAPER: &str = "用紙サイズ(Z)";
    pub(super) const PORTRAIT: &str = "縦(I)";
    pub(super) const LANDSCAPE: &str = "横(A)";
    pub(super) const MARGIN_TOP: &str = "余白／上(T)";
    pub(super) const MARGIN_BOTTOM: &str = "下(B)";
    pub(super) const MARGIN_LEFT: &str = "左(L)";
    pub(super) const MARGIN_RIGHT: &str = "右(R)";
    pub(super) const LINE_NUMBER: &str = "行番号を印刷(N)";
    pub(super) const WORD_WRAP: &str = "英文ワードラップ(W)";
    pub(super) const COLOR: &str = "カラー印刷(V)";
    pub(super) const HEADER: &str = "ヘッダー(U)";
    pub(super) const FOOTER: &str = "フッター(D)";
    pub(super) const CANCEL: &str = "キャンセル(X)";
    /// `&OK` in the resource file: the label shows no key.
    pub(super) const OK_KEY: char = 'o';
}

/// What an access key does in the dialog.
#[derive(Clone, Copy)]
enum Use {
    /// The list of a drop-down list opens (Sakura moves the focus to the combo box).
    List(List),
    Rename,
    /// The focus goes to the box of the number.
    Number(Number),
    /// The focus goes to the first box of the header or the footer.
    Form(Part),
    Orientation(Orientation),
    LineNumber,
    WordWrap,
    Color,
    Ok,
    Cancel,
}

/// How many rows PageUp and PageDown move in an open list: what its box shows (`max_h(160)`).
const LIST_PAGE: usize = 8;

/// The resolution Sakura works the font height out at on the screen (96 dpi).
const DPI: i32 = 96;

/// The parts of the header and of the footer, in the order of the boxes (`m_szHeaderForm[0..3]`).
const PLACES: [&str; 3] = ["左", "中央", "右"];

/// What OK gives back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Answer {
    /// The eight settings, with the changes.
    pub settings: PrintSettings,
    /// The eight settings the dialog started with: only the ones that differ are kept.
    pub original: PrintSettings,
    /// The one that was chosen (`m_nCurrentPrintSetting`).
    pub current: usize,
}

/// The list that is open under its button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum List {
    Setting,
    FontHan,
    FontZen,
    Paper,
}

/// The header or the footer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Header,
    Footer,
}

pub(crate) struct PrintSettingDialog {
    view: WeakEntity<EditorView>,
    /// The eight settings being edited: the dialog's own copy (`m_PrintSettingArr`), the one
    /// owner of every value except the text of the edit boxes. In the setting shown (`current`)
    /// the numbers and the forms are stale until [`Self::keep_current`]: the edit boxes own
    /// that text while it is typed.
    pub(super) settings: PrintSettings,
    /// The eight settings as they were when the dialog opened: what Cancel leaves, and what
    /// OK compares with when writing only the changed ones.
    original: PrintSettings,
    current: usize,
    pub(super) numbers: [Entity<InputState>; 8],
    pub(super) header: [Entity<InputState>; 3],
    pub(super) footer: [Entity<InputState>; 3],
    /// The width of the line numbers in characters (`m_nLineNumberColumns`).
    pub(super) number_columns: u32,
    /// The fonts of the OS that the dialog lists, filtered, sorted and without duplicates (made
    /// once: the OS does not change them while the dialog is open).
    installed: Vec<String>,
    list: Option<List>,
    /// The row of the open list that the keys are on (`CB_SETCURSEL` of the list of a combo box).
    list_cursor: usize,
    list_scroll: ScrollHandle,
    result: Option<async_channel::Sender<Option<Answer>>>,
    focus: FocusHandle,
    /// The scroll position of the dialog, which the scroll bar draws from.
    pub(super) scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl PrintSettingDialog {
    pub(crate) fn new(
        settings: PrintSettings,
        current: usize,
        number_columns: u32,
        view: WeakEntity<EditorView>,
        result: async_channel::Sender<Option<Answer>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let make =
            |window: &mut Window, cx: &mut Context<Self>| cx.new(|cx| input_field(window, cx));
        let numbers = [(); 8].map(|()| make(window, cx));
        let header = [(); 3].map(|()| make(window, cx));
        let footer = [(); 3].map(|()| make(window, cx));
        let mut subscriptions = Vec::new();
        for input in numbers.iter().chain(&header).chain(&footer) {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                |this: &mut Self, _, event: &InputEvent, window, cx| match event {
                    // the two numbers of the printable size follow the edit boxes
                    InputEvent::Change => cx.notify(),
                    // Enter in an open list takes the row (`on_capture_key`), not OK
                    InputEvent::PressEnter { .. } if this.list.is_none() => this.ok(window, cx),
                    _ => {}
                },
            ));
        }
        let current = current.min(settings.slots().len() - 1);
        let mut dialog = Self {
            view,
            original: settings.clone(),
            settings,
            current,
            numbers,
            header,
            footer,
            number_columns,
            installed: families(cx.text_system().all_font_names(), ""),
            list: None,
            list_cursor: 0,
            list_scroll: ScrollHandle::default(),
            result: Some(result),
            focus: cx.focus_handle(),
            scroll: ScrollHandle::default(),
            _subscriptions: subscriptions,
        };
        dialog.show_setting(window, cx);
        dialog
    }

    pub(crate) fn focus_first(&self, window: &mut Window, cx: &mut Context<Self>) {
        focus_selecting(&self.numbers[0], window, cx);
    }

    /// `OnChangeSettingType`, the part that sets the edit boxes: they show the setting shown.
    fn show_setting(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let shown = self.settings.get(self.current);
        for (number, input) in Number::ALL.into_iter().zip(&self.numbers) {
            let text = number.of(shown).to_string();
            input.update(cx, |state, cx| state.set_value(text, window, cx));
        }
        for (form, input) in shown
            .header_form
            .iter()
            .chain(&shown.footer_form)
            .zip(self.header.iter().chain(&self.footer))
        {
            let text = form.clone();
            input.update(cx, |state, cx| state.set_value(text, window, cx));
        }
        cx.notify();
    }

    fn texts<const N: usize>(inputs: &[Entity<InputState>; N], cx: &gpui::App) -> [String; N] {
        std::array::from_fn(|i| inputs[i].read(cx).value().to_string())
    }

    /// `GetData`: the setting the controls make now, checked and corrected.
    pub(crate) fn collect(&self, cx: &gpui::App) -> PrintSetting {
        model::collect(
            self.shown(),
            &Self::texts(&self.numbers, cx),
            Self::texts(&self.header, cx),
            Self::texts(&self.footer, cx),
        )
    }

    /// The setting shown has its place among the eight again, with the values checked.
    fn keep_current(&mut self, cx: &gpui::App) {
        let setting = self.collect(cx);
        self.settings.set(self.current, setting);
    }

    /// The first list: another setting is shown (`CBN_SELCHANGE` of `IDC_COMBO_SETTINGNAME`).
    pub(crate) fn select_setting(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if index >= self.settings.slots().len() {
            return;
        }
        self.keep_current(cx);
        self.current = index;
        self.list = None;
        self.show_setting(window, cx);
    }

    /// The name of the setting `index` as the list shows it.
    fn name_ref(&self, index: usize) -> &str {
        &self.settings.get(index).name
    }

    #[cfg(test)]
    pub(crate) fn name_of(&self, index: usize) -> String {
        self.name_ref(index).to_owned()
    }

    #[cfg(test)]
    pub(crate) fn current(&self) -> usize {
        self.current
    }

    /// The list that is open and the row the keys are on (for the tests).
    #[cfg(test)]
    pub(crate) fn open_list_and_cursor(&self) -> Option<(List, usize)> {
        self.list.map(|list| (list, self.list_cursor))
    }

    /// OK can be pressed when there is room to print.
    pub(crate) fn can_ok(&self, cx: &gpui::App) -> bool {
        model::has_printable_area(&self.collect(cx), self.number_columns)
    }

    /// OK: the values are checked and the dialog gives the eight settings back.
    pub(crate) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_ok(cx) {
            return;
        }
        self.keep_current(cx);
        // the boxes show the corrected values (`SetDlgItemInt` in `GetData`)
        self.show_setting(window, cx);
        let answer = Answer {
            settings: self.settings.clone(),
            original: self.original.clone(),
            current: self.current,
        };
        if let Some(result) = self.result.take() {
            let _ = result.try_send(Some(answer));
        }
        self.close(window, cx);
    }

    /// キャンセル: nothing is kept.
    pub(crate) fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(result) = self.result.take() {
            let _ = result.try_send(None);
        }
        self.close(window, cx);
    }

    /// Takes the dialog away from the view and gives the focus back.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.print.setup_dialog = None;
            let target = match &view.print.preview {
                Some(preview) => preview.focus_handle(cx),
                None => view.focus_handle.clone(),
            };
            window.focus(&target, cx);
            cx.notify();
        })
        .ok();
    }

    /// 設定名変更(2)...: asks for the name with the one-line input dialog (`CDlgInput1`).
    pub(crate) fn rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let request = InputRequest {
            title: "設定名の変更".into(),
            message: "設定の名称を入力してください。".into(),
            default: self.shown().name.clone(),
            max_len: NAME_MAX,
        };
        let answer = view.update(cx, |view, cx| view.open_input_dialog(request, window, cx));
        cx.spawn_in(window, async move |this, cx| {
            // The input dialog gives the focus to the view when it ends, whatever the answer was.
            let name = answer.await;
            this.update_in(cx, |this, window, cx| {
                this.set_name(name.as_deref().unwrap_or_default(), window, cx);
            })
            .ok();
        })
        .detach();
    }

    /// An empty name is not taken (`if( szWork[0] != L'\0' )`).
    pub(crate) fn set_name(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !name.is_empty() {
            self.shown_mut().name = name.chars().take(NAME_MAX).collect();
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// The font dialog for the header or the footer (`IDC_BUTTON_FONT_HEAD`). It starts at the
    /// font of the part, or at the font for half-width characters at the height of the print font.
    fn choose_font(&mut self, part: Part, window: &mut Window, cx: &mut Context<Self>) {
        let initial = self.font_start(part, cx);
        let owner = crate::font_dialog::owner_of(window);
        let dialog = cx.weak_entity();
        crate::font_dialog::choose(
            cx,
            owner,
            initial,
            Box::new(move |app, font| {
                dialog
                    .update(app, |this, cx| this.set_font(part, &font, cx))
                    .ok();
            }),
        );
    }

    /// The start of the font dialog for what the controls show now: Sakura reads its fields on
    /// every change, here they are taken before the dialog opens.
    pub(crate) fn font_start(&mut self, part: Part, cx: &gpui::App) -> FontSetting {
        self.keep_current(cx);
        self.font_dialog_start(part)
    }

    fn font_dialog_start(&self, part: Part) -> FontSetting {
        let font = match part {
            Part::Header => self.shown().header_font.as_ref(),
            Part::Footer => self.shown().footer_font.as_ref(),
        };
        let mut start = FontSetting::default();
        match font.filter(|f| !f.face.is_empty()) {
            Some(f) => {
                start.log_font.face_name.clone_from(&f.face);
                start.point_size = f.point_size;
                start.log_font.height = start.height_for_dpi(DPI);
            }
            None => {
                // `lf.lfHeight = -( m_nPrintFontHeight * dpi / 254 )`
                start.log_font = LogFont {
                    face_name: self.shown().print_font_face_han.clone(),
                    height: -(self.shown().print_font_height * DPI / 254),
                    ..start.log_font
                };
                start.point_size = 0;
            }
        }
        start
    }

    /// The font the user chose for the header or the footer.
    pub(crate) fn set_font(&mut self, part: Part, font: &FontSetting, cx: &mut Context<Self>) {
        if font.log_font.face_name.is_empty() {
            return;
        }
        let chosen = Some(HeaderFooterFont {
            face: font.log_font.face_name.clone(),
            point_size: font.point_size_for_dpi(DPI),
        });
        match part {
            Part::Header => self.shown_mut().header_font = chosen,
            Part::Footer => self.shown_mut().footer_font = chosen,
        }
        cx.notify();
    }

    /// 使用: unchecking it drops the font (`IDC_CHECK_USE_FONT_HEAD`).
    pub(crate) fn drop_font(&mut self, part: Part, cx: &mut Context<Self>) {
        match part {
            Part::Header => self.shown_mut().header_font = None,
            Part::Footer => self.shown_mut().footer_font = None,
        }
        cx.notify();
    }

    /// The setting shown, to edit in place.
    pub(super) fn shown_mut(&mut self) -> &mut PrintSetting {
        self.settings.get_mut(self.current)
    }

    /// The setting shown.
    pub(super) fn shown(&self) -> &PrintSetting {
        self.settings.get(self.current)
    }

    fn toggle_list(&mut self, list: List, window: &mut Window, cx: &mut Context<Self>) {
        if self.list == Some(list) {
            self.list = None;
            cx.notify();
        } else {
            self.open_list(list, window, cx);
        }
    }

    /// Opens the list with the row of the value that is set under the keys.
    ///
    /// The focus goes to the dialog itself: a box keeps Home, End and the like for its text (they
    /// are its actions, which come before the keys of the dialog).
    fn open_list(&mut self, list: List, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        self.list = Some(list);
        self.list_cursor = self.list_selected(list).unwrap_or(0);
        self.list_scroll.scroll_to_item(self.list_cursor);
        cx.notify();
    }

    /// The keys of an open list, as those of the list of a combo box: the arrows, Home, End, PageUp
    /// and PageDown move, Enter takes the row (`CBN_SELCHANGE` of the first list shows another
    /// setting), Escape closes it (`on_key_down`). They are taken before the control that has the
    /// focus, a button or a box, sees them. Returns whether the key was one of the list's.
    fn list_key(&mut self, key: &str, cx: &mut Context<Self>) -> Option<usize> {
        let list = self.list?;
        let last = self.list_len(list).checked_sub(1)?;
        let cursor = self.list_cursor.min(last);
        let to = match key {
            "up" => cursor.saturating_sub(1),
            "down" => (cursor + 1).min(last),
            "home" => 0,
            "end" => last,
            "pageup" => cursor.saturating_sub(LIST_PAGE),
            "pagedown" => (cursor + LIST_PAGE).min(last),
            "enter" => return Some(cursor),
            _ => return None,
        };
        self.list_cursor = to;
        self.list_scroll.scroll_to_item(to);
        cx.notify();
        Some(to)
    }

    fn on_capture_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.keystroke.modifiers;
        if self.list.is_none() || modifiers.modified() {
            return;
        }
        let key = event.keystroke.key.as_str();
        let list = self.list;
        if let Some(row) = self.list_key(key, cx) {
            if key == "enter"
                && let Some(list) = list
            {
                // after the key has been through the control that has the focus (a box says
                // PressEnter, a button is clicked), which see the list still open
                let this = cx.entity();
                window.defer(cx, move |window, cx| {
                    this.update(cx, |dialog, cx| dialog.pick(list, row, window, cx));
                });
            }
            cx.stop_propagation();
        }
    }

    /// Alt with the access key of a label (`IsDialogMessage`). The label of a drop-down list
    /// opens the list (Sakura moves the focus to the combo box, which a button of ours cannot
    /// take from outside), so that the keys can choose from it. Returns whether the letter was
    /// one of the dialog's.
    pub(crate) fn use_access_key(
        &mut self,
        letter: char,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let ok = self.can_ok(cx);
        let entries = [
            Entry::labelled(labels::SETTING, true, Use::List(List::Setting)),
            Entry::labelled(labels::RENAME, true, Use::Rename),
            Entry::labelled(labels::FONT_HAN, true, Use::List(List::FontHan)),
            Entry::labelled(labels::FONT_ZEN, true, Use::List(List::FontZen)),
            Entry::labelled(labels::FONT_HEIGHT, true, Use::Number(Number::FontHeight)),
            Entry::labelled(labels::LINE_SPACING, true, Use::Number(Number::LineSpacing)),
            Entry::labelled(labels::DANSUU, true, Use::Number(Number::Dansuu)),
            Entry::labelled(labels::DAN_SPACE, true, Use::Number(Number::DanSpace)),
            Entry::labelled(labels::PAPER, true, Use::List(List::Paper)),
            Entry::labelled(
                labels::PORTRAIT,
                true,
                Use::Orientation(Orientation::Portrait),
            ),
            Entry::labelled(
                labels::LANDSCAPE,
                true,
                Use::Orientation(Orientation::Landscape),
            ),
            Entry::labelled(labels::MARGIN_TOP, true, Use::Number(Number::MarginTop)),
            Entry::labelled(
                labels::MARGIN_BOTTOM,
                true,
                Use::Number(Number::MarginBottom),
            ),
            Entry::labelled(labels::MARGIN_LEFT, true, Use::Number(Number::MarginLeft)),
            Entry::labelled(labels::MARGIN_RIGHT, true, Use::Number(Number::MarginRight)),
            Entry::labelled(labels::LINE_NUMBER, true, Use::LineNumber),
            Entry::labelled(labels::WORD_WRAP, true, Use::WordWrap),
            Entry::labelled(labels::COLOR, true, Use::Color),
            Entry::labelled(labels::HEADER, true, Use::Form(Part::Header)),
            Entry::labelled(labels::FOOTER, true, Use::Form(Part::Footer)),
            // OK is greyed out while the paper has no room (`can_ok`)
            Entry::keyed(labels::OK_KEY, ok, Use::Ok),
            Entry::labelled(labels::CANCEL, true, Use::Cancel),
        ];
        let what = match pick(letter, &entries) {
            Found::Nothing => return false,
            Found::Several => return true,
            Found::One(what) => what,
        };
        // a key in the dialog closes the list that is open, as a click elsewhere does
        if !matches!(what, Use::List(_)) {
            self.list = None;
        }
        match what {
            Use::List(list) => self.open_list(list, window, cx),
            Use::Rename => self.rename(window, cx),
            Use::Number(number) => {
                let at = Number::ALL.iter().position(|n| *n == number).unwrap_or(0);
                focus_selecting(&self.numbers[at], window, cx);
            }
            Use::Form(Part::Header) => focus_selecting(&self.header[0], window, cx),
            Use::Form(Part::Footer) => focus_selecting(&self.footer[0], window, cx),
            Use::Orientation(orientation) => self.shown_mut().print_paper_orientation = orientation,
            Use::LineNumber => {
                let on = &mut self.shown_mut().print_line_number;
                *on = !*on;
            }
            Use::WordWrap => {
                let on = &mut self.shown_mut().print_word_wrap;
                *on = !*on;
            }
            Use::Color => {
                let on = &mut self.shown_mut().color_print;
                *on = !*on;
            }
            Use::Ok => self.ok(window, cx),
            Use::Cancel => self.cancel(window, cx),
        }
        cx.notify();
        true
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" {
            if self.list.take().is_some() {
                cx.notify();
            } else {
                self.cancel(window, cx);
            }
            cx.stop_propagation();
        }
    }

    /// How many rows the list has, without making them (the keys ask on every press).
    fn list_len(&self, list: List) -> usize {
        match list {
            List::Setting => self.settings.slots().len(),
            List::FontHan => font_count(&self.installed, &self.shown().print_font_face_han),
            List::FontZen => font_count(&self.installed, &self.shown().print_font_face_zen),
            List::Paper => PAPER_INFO.len(),
        }
    }

    /// The row of the value that is set, without making the rows.
    fn list_selected(&self, list: List) -> Option<usize> {
        match list {
            List::Setting => Some(self.current),
            List::FontHan => font_index(&self.installed, &self.shown().print_font_face_han),
            List::FontZen => font_index(&self.installed, &self.shown().print_font_face_zen),
            List::Paper => PAPER_INFO
                .iter()
                .position(|p| p.id == self.shown().print_paper_size),
        }
    }

    /// The rows of the list, for drawing it (once per frame, while it is open).
    fn list_rows(&self, list: List) -> (Vec<String>, Option<usize>) {
        match list {
            List::Setting => (
                (0..self.settings.slots().len())
                    .map(|i| self.name_ref(i).to_owned())
                    .collect(),
                Some(self.current),
            ),
            List::FontHan => {
                let names =
                    font_names(&self.installed, &self.shown().print_font_face_han).into_owned();
                let at = names
                    .iter()
                    .position(|n| *n == self.shown().print_font_face_han);
                (names, at)
            }
            List::FontZen => {
                let names =
                    font_names(&self.installed, &self.shown().print_font_face_zen).into_owned();
                let at = names
                    .iter()
                    .position(|n| *n == self.shown().print_font_face_zen);
                (names, at)
            }
            List::Paper => (
                PAPER_INFO.iter().map(|p| p.name.to_owned()).collect(),
                PAPER_INFO
                    .iter()
                    .position(|p| p.id == self.shown().print_paper_size),
            ),
        }
    }

    /// A row of an open list was chosen.
    pub(crate) fn pick(
        &mut self,
        list: List,
        row: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match list {
            List::Setting => {
                self.select_setting(row, window, cx);
                return;
            }
            List::FontHan => {
                let name = font_names(&self.installed, &self.shown().print_font_face_han)
                    .get(row)
                    .cloned();
                if let Some(name) = name {
                    self.shown_mut().print_font_face_han = name;
                }
            }
            List::FontZen => {
                let name = font_names(&self.installed, &self.shown().print_font_face_zen)
                    .get(row)
                    .cloned();
                if let Some(name) = name {
                    self.shown_mut().print_font_face_zen = name;
                }
            }
            List::Paper => {
                if let Some(info) = PAPER_INFO.get(row) {
                    self.shown_mut().print_paper_size = info.id;
                }
            }
        }
        self.list = None;
        cx.notify();
    }

    fn list_view(&self, list: List, cx: &mut Context<Self>) -> impl IntoElement {
        let (rows, _) = self.list_rows(list);
        let selected = Some(self.list_cursor);
        let (id, name) = match list {
            List::Setting => (
                "ps-list-setting",
                crate::a11y::without_key_hint(labels::SETTING),
            ),
            List::FontHan => (
                "ps-list-han",
                crate::a11y::without_key_hint(labels::FONT_HAN),
            ),
            List::FontZen => (
                "ps-list-zen",
                crate::a11y::without_key_hint(labels::FONT_ZEN),
            ),
            List::Paper => (
                "ps-list-paper",
                crate::a11y::without_key_hint(labels::PAPER),
            ),
        };
        let mut view = v_flex()
            .id(id)
            .role(gpui::Role::ListBox)
            .aria_label(name)
            .max_h(px(160.))
            .overflow_y_scroll()
            .track_scroll(&self.list_scroll)
            .border_1()
            .border_color(rgb(0xaaaaaa))
            .bg(rgb(0xffffff));
        for (i, text) in rows.into_iter().enumerate() {
            view = view.child(
                crate::a11y::list_row(div().id((id, i)), &text, selected == Some(i))
                    .px_1()
                    .cursor_pointer()
                    .when(selected == Some(i), |row| row.bg(rgb(0xc8dcf5)))
                    .hover(|style| style.bg(rgb(0xdde8f5)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.pick(list, i, window, cx);
                    }))
                    .child(text),
            );
        }
        view
    }

    fn combo(
        &self,
        id: &'static str,
        text: &str,
        list: List,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().flex_1().child(
            Button::new(id)
                .label(format!("{text}  v"))
                .w_full()
                .on_click(
                    cx.listener(move |this, _, window, cx| this.toggle_list(list, window, cx)),
                ),
        )
    }

    /// A number box with its label and the unit after it.
    fn number_row(
        &self,
        label: &'static str,
        number: Number,
        unit: &'static str,
    ) -> impl IntoElement {
        let index = Number::ALL.iter().position(|n| *n == number).unwrap_or(0);
        h_flex()
            .gap_2()
            .items_center()
            .child(div().w(px(104.)).text_right().child(label))
            .child(
                div()
                    .w(px(64.))
                    .child(Input::new(&self.numbers[index]).aria_label(number.name())),
            )
            .child(unit)
    }

    fn form_row(&self, part: Part, cx: &mut Context<Self>) -> impl IntoElement {
        let (label, name, inputs, font, use_id, button_id) = match part {
            Part::Header => (
                labels::HEADER,
                "ヘッダー",
                &self.header,
                self.shown().header_font.as_ref(),
                "ps-use-font-head",
                "ps-font-head",
            ),
            Part::Footer => (
                labels::FOOTER,
                "フッター",
                &self.footer,
                self.shown().footer_font.as_ref(),
                "ps-use-font-foot",
                "ps-font-foot",
            ),
        };
        let has_font = font.is_some_and(|f| !f.face.is_empty());
        v_flex()
            .gap_1()
            .child(label)
            .child(
                h_flex()
                    .gap_1()
                    .children(inputs.iter().zip(PLACES).map(|(input, place)| {
                        div()
                            .flex_1()
                            .child(Input::new(input).aria_label(format!("{name}{place}")))
                    })),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Checkbox::new(use_id)
                            .label("使用")
                            .checked(has_font)
                            .disabled(!has_font)
                            .on_click(cx.listener(move |this, _: &bool, _, cx| {
                                this.drop_font(part, cx);
                            })),
                    )
                    .child(
                        Button::new(button_id)
                            .label("フォント")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.choose_font(part, window, cx);
                            })),
                    )
                    .child(
                        div()
                            .text_color(rgb(0x444444))
                            .child(model::header_font_label(font)),
                    ),
            )
    }

    fn check(
        &self,
        id: &'static str,
        label: &'static str,
        checked: bool,
        cx: &mut Context<Self>,
        set: impl Fn(&mut PrintSetting, bool) + 'static,
    ) -> impl IntoElement {
        Checkbox::new(id)
            .label(label)
            .checked(checked)
            .on_click(cx.listener(move |this, value: &bool, _, cx| {
                set(this.shown_mut(), *value);
                cx.notify();
            }))
    }
}

impl Focusable for PrintSettingDialog {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for PrintSettingDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let now = self.collect(cx);
        let printable = model::printable(&now, self.number_columns);
        let ok_enabled = model::has_printable_area(&now, self.number_columns);
        let paper_name = find_paper_info(self.shown().print_paper_size).map_or("", |p| p.name);
        let open = |list: List| self.list == Some(list);

        let top = h_flex()
            .gap_2()
            .items_center()
            .child(div().w(px(104.)).text_right().child(labels::SETTING))
            .child(self.combo("ps-setting", self.name_ref(self.current), List::Setting, cx))
            .child(
                Button::new("ps-rename")
                    .label(labels::RENAME)
                    .on_click(cx.listener(|this, _, window, cx| this.rename(window, cx))),
            );

        let fonts = v_flex()
            .gap_1()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().w(px(104.)).text_right().child(labels::FONT_HAN))
                    .child(self.combo(
                        "ps-font-han",
                        &self.shown().print_font_face_han,
                        List::FontHan,
                        cx,
                    )),
            )
            .when(open(List::FontHan), |this| {
                this.child(self.list_view(List::FontHan, cx))
            })
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().w(px(104.)).text_right().child(labels::FONT_ZEN))
                    .child(self.combo(
                        "ps-font-zen",
                        &self.shown().print_font_face_zen,
                        List::FontZen,
                        cx,
                    )),
            )
            .when(open(List::FontZen), |this| {
                this.child(self.list_view(List::FontZen, cx))
            });

        let left = v_flex()
            .gap_1()
            .flex_1()
            .child(fonts)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(self.number_row(labels::FONT_HEIGHT, Number::FontHeight, "(1/10mm)"))
                    .child(model::font_points_text(now.print_font_height)),
            )
            .child(self.number_row(labels::LINE_SPACING, Number::LineSpacing, "(%)"))
            .child(self.number_row(labels::DANSUU, Number::Dansuu, ""))
            .child(self.number_row(labels::DAN_SPACE, Number::DanSpace, "(mm)"))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().w(px(104.)).text_right().child(labels::PAPER))
                    .child(self.combo("ps-paper", paper_name, List::Paper, cx)),
            )
            .when(open(List::Paper), |this| {
                this.child(self.list_view(List::Paper, cx))
            })
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(div().w(px(104.)).text_right().child("用紙の向き"))
                    .child(
                        Radio::new("ps-portrait")
                            .label(labels::PORTRAIT)
                            .checked(self.shown().print_paper_orientation == Orientation::Portrait)
                            .on_click(cx.listener(|this, _: &bool, _, cx| {
                                this.shown_mut().print_paper_orientation = Orientation::Portrait;
                                cx.notify();
                            })),
                    )
                    .child(
                        Radio::new("ps-landscape")
                            .label(labels::LANDSCAPE)
                            .checked(self.shown().print_paper_orientation == Orientation::Landscape)
                            .on_click(cx.listener(|this, _: &bool, _, cx| {
                                this.shown_mut().print_paper_orientation = Orientation::Landscape;
                                cx.notify();
                            })),
                    ),
            )
            .child(self.number_row(labels::MARGIN_TOP, Number::MarginTop, "(mm)"))
            .child(self.number_row(labels::MARGIN_BOTTOM, Number::MarginBottom, "(mm)"))
            .child(self.number_row(labels::MARGIN_LEFT, Number::MarginLeft, "(mm)"))
            .child(self.number_row(labels::MARGIN_RIGHT, Number::MarginRight, "(mm)"));

        let (columns, lines) = printable.map_or(("-".to_owned(), "-".to_owned()), |(c, l)| {
            (c.to_string(), l.to_string())
        });
        let right = v_flex()
            .gap_1()
            .flex_1()
            .child(self.check(
                "ps-line-number",
                labels::LINE_NUMBER,
                self.shown().print_line_number,
                cx,
                |s, v| {
                    s.print_line_number = v;
                },
            ))
            .child(self.check(
                "ps-word-wrap",
                labels::WORD_WRAP,
                self.shown().print_word_wrap,
                cx,
                |s, v| {
                    s.print_word_wrap = v;
                },
            ))
            .child(
                h_flex()
                    .gap_3()
                    .child(self.check(
                        "ps-kinsoku-head",
                        "行頭禁則",
                        self.shown().print_kinsoku_head,
                        cx,
                        |s, v| {
                            s.print_kinsoku_head = v;
                        },
                    ))
                    .child(self.check(
                        "ps-kinsoku-ret",
                        "改行ぶら下げ",
                        self.shown().print_kinsoku_ret,
                        cx,
                        |s, v| {
                            s.print_kinsoku_ret = v;
                        },
                    )),
            )
            .child(
                h_flex()
                    .gap_3()
                    .child(self.check(
                        "ps-kinsoku-tail",
                        "行末禁則",
                        self.shown().print_kinsoku_tail,
                        cx,
                        |s, v| {
                            s.print_kinsoku_tail = v;
                        },
                    ))
                    .child(self.check(
                        "ps-kinsoku-kuto",
                        "句読点ぶら下げ",
                        self.shown().print_kinsoku_kuto,
                        cx,
                        |s, v| {
                            s.print_kinsoku_kuto = v;
                        },
                    )),
            )
            .child(
                v_flex()
                    .gap_1()
                    .p_1()
                    .border_1()
                    .border_color(rgb(0xcccccc))
                    .child(
                        h_flex()
                            .gap_2()
                            .child("行あたりの文字数：")
                            .child(div().id("ps-columns").child(columns)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child("縦方向の行数：")
                            .child(div().id("ps-lines").child(lines)),
                    ),
            )
            .child(self.check(
                "ps-color",
                labels::COLOR,
                self.shown().color_print,
                cx,
                |s, v| {
                    s.color_print = v;
                },
            ))
            .child(self.form_row(Part::Header, cx))
            .child(self.form_row(Part::Footer, cx));

        let body = v_flex()
            .gap_2()
            .child(top)
            .when(open(List::Setting), |this| {
                this.child(self.list_view(List::Setting, cx))
            })
            .child(
                v_flex()
                    .gap_1()
                    .p_2()
                    .border_1()
                    .border_color(rgb(0xcccccc))
                    .child("設定")
                    .child(h_flex().gap_4().items_start().child(left).child(right)),
            )
            .child(
                h_flex()
                    .gap_2()
                    .justify_center()
                    .child(
                        Button::new("ps-ok")
                            .label("OK")
                            .disabled(!ok_enabled)
                            .on_click(cx.listener(|this, _, window, cx| this.ok(window, cx))),
                    )
                    .child(
                        Button::new("ps-cancel")
                            .label(labels::CANCEL)
                            .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                    ),
            );
        // The scroll bar is a sibling of the scrolling part, not its child: inside it, the bar's
        // overlay itself made a dialog that fits overflow by its padding.
        let scrolling = v_flex()
            .id("print-setting-dialog")
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .track_focus(&self.focus)
            .keep_tab_inside(&self.focus)
            .key_context("PrintSettingDialog")
            .capture_key_down(cx.listener(Self::on_capture_key))
            // Alt+letter works from the boxes too, and does not type the letter
            .on_access_key(move |letter, window, cx| {
                this.update(cx, |dialog, cx| dialog.use_access_key(letter, window, cx))
            })
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.on_key_down(event, window, cx)
            }))
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .p_3()
            .gap_2()
            .text_color(rgb(0x222222))
            .text_size(px(13.))
            .child(
                div()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("印刷ページ設定"),
            )
            .child(body);
        let panel = v_flex()
            .relative()
            .w(px(760.))
            .max_w_full()
            .max_h_full()
            .bg(rgb(0xf7f7f7))
            .border_1()
            .border_color(rgb(0x888888))
            .rounded_md()
            .shadow_md()
            .child(scrolling)
            // The dialog scrolls when the window is short; the bar shows that more is below. It
            // is always drawn while the dialog overflows: the default fades out when idle.
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .child(Scrollbar::vertical(&self.scroll).mode(ScrollbarMode::Always)),
            );
        div()
            .id("print-setting-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .child(panel)
    }
}

/// The font names to list for a face: the listed fonts, and the face itself if this machine
/// lacks it. `installed` is the list that `families` made (with no face), so that only the
/// face of the setting can be missing from it.
fn font_names<'a>(installed: &'a [String], face: &str) -> Cow<'a, [String]> {
    if face.is_empty() || installed.iter().any(|f| f == face) {
        Cow::Borrowed(installed)
    } else {
        let mut list = Vec::with_capacity(installed.len() + 1);
        list.push(face.to_owned());
        list.extend_from_slice(installed);
        Cow::Owned(list)
    }
}

/// `font_names(..).len()` without making the list.
fn font_count(installed: &[String], face: &str) -> usize {
    installed.len() + usize::from(!face.is_empty() && !installed.iter().any(|f| f == face))
}

/// The row of `face` in `font_names(..)` without making the list: the first one when the machine
/// lacks the face (it is put first), none for no face.
fn font_index(installed: &[String], face: &str) -> Option<usize> {
    if face.is_empty() {
        return None;
    }
    Some(installed.iter().position(|f| f == face).unwrap_or(0))
}

impl EditorView {
    /// Opens the dialog over the view and focuses it.
    pub(crate) fn open_print_setting_dialog(
        &mut self,
        settings: PrintSettings,
        current: usize,
        number_columns: u32,
        result: async_channel::Sender<Option<Answer>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().downgrade();
        let dialog = cx.new(|cx| {
            PrintSettingDialog::new(settings, current, number_columns, view, result, window, cx)
        });
        dialog.update(cx, |dialog, cx| dialog.focus_first(window, cx));
        self.print.setup_dialog = Some(dialog);
        cx.notify();
    }
}

#[cfg(test)]
mod font_name_tests {
    use super::font_names;
    use std::borrow::Cow;

    fn fonts() -> Vec<String> {
        ["Arial", "Consolas"].map(str::to_owned).to_vec()
    }

    #[test]
    fn a_listed_face_or_no_face_borrows_the_list() {
        let list = fonts();
        assert!(matches!(font_names(&list, "Consolas"), Cow::Borrowed(_)));
        assert!(matches!(font_names(&list, ""), Cow::Borrowed(_)));
    }

    #[test]
    fn the_count_and_the_row_agree_with_the_list() {
        use super::{font_count, font_index};
        let list = fonts();
        for face in ["Consolas", "Arial", "", "ＭＳ ゴシック"] {
            let names = font_names(&list, face);
            assert_eq!(font_count(&list, face), names.len(), "{face}");
            assert_eq!(
                font_index(&list, face),
                names.iter().position(|n| n == face),
                "{face}"
            );
        }
    }

    #[test]
    fn a_face_the_machine_lacks_goes_first() {
        let list = fonts();
        // a font of the settings file that this machine lacks (ＭＳ ゴシック on macOS)
        let names = font_names(&list, "ＭＳ ゴシック");
        assert_eq!(names.as_ref(), ["ＭＳ ゴシック", "Arial", "Consolas"]);
        assert_eq!(list, fonts());
    }
}
