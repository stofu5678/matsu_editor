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
//! Not done: the spin buttons next to the numbers, the access keys of the labels, and the help
//! button.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Render, StatefulInteractiveElement, Styled,
    Subscription, WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::radio::Radio;
use gpui_component::{Disableable as _, h_flex, v_flex};
use ume_config::settings::print::{NAME_MAX, PrintSettings};
use ume_config::settings::{FontSetting, LogFont};
use ume_print::{HeaderFooterFont, Orientation, PAPER_INFO, PrintSetting, find_paper_info};

use super::model::{self, Number};
use crate::font_dialog::model::families;
use crate::input_dialog::InputRequest;
use crate::view::EditorView;

/// The resolution Sakura works the font height out at on the screen (96 dpi).
const DPI: i32 = 96;

/// What OK gives back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Answer {
    /// The eight settings, with the changes.
    pub settings: PrintSettings,
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
    /// The eight settings. The one shown (`current`) is stale until [`Self::keep_current`].
    pub(super) settings: PrintSettings,
    current: usize,
    /// What the controls other than the edit boxes hold, for the setting shown.
    pub(super) draft: PrintSetting,
    pub(super) numbers: [Entity<InputState>; 8],
    pub(super) header: [Entity<InputState>; 3],
    pub(super) footer: [Entity<InputState>; 3],
    /// The width of the line numbers in characters (`m_nLineNumberColumns`).
    pub(super) number_columns: u32,
    /// The fonts of the OS.
    installed: Vec<String>,
    list: Option<List>,
    result: Option<async_channel::Sender<Option<Answer>>>,
    focus: FocusHandle,
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
            |window: &mut Window, cx: &mut Context<Self>| cx.new(|cx| InputState::new(window, cx));
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
                    InputEvent::PressEnter { .. } => this.ok(window, cx),
                    _ => {}
                },
            ));
        }
        let current = current.min(settings.slots().len() - 1);
        let draft = settings.get(current).clone();
        let mut dialog = Self {
            view,
            settings,
            current,
            draft,
            numbers,
            header,
            footer,
            number_columns,
            installed: cx.text_system().all_font_names(),
            list: None,
            result: Some(result),
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        };
        dialog.show_setting(window, cx);
        dialog
    }

    pub(crate) fn focus_first(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.numbers[0].update(cx, |state, cx| state.focus(window, cx));
    }

    /// `OnChangeSettingType`, the part that sets the edit boxes: they show `draft`.
    fn show_setting(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let draft = self.draft.clone();
        for (number, input) in Number::ALL.into_iter().zip(&self.numbers) {
            let text = number.of(&draft).to_string();
            input.update(cx, |state, cx| state.set_value(text, window, cx));
        }
        for (form, input) in draft
            .header_form
            .iter()
            .chain(&draft.footer_form)
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
            &self.draft,
            &Self::texts(&self.numbers, cx),
            &Self::texts(&self.header, cx),
            &Self::texts(&self.footer, cx),
        )
    }

    /// The setting shown has its place among the eight again, with the values checked.
    fn keep_current(&mut self, cx: &gpui::App) {
        let setting = self.collect(cx);
        self.draft = setting.clone();
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
        self.draft = self.settings.get(index).clone();
        self.list = None;
        self.show_setting(window, cx);
    }

    /// The name of the setting `index` as the list shows it.
    pub(crate) fn name_of(&self, index: usize) -> String {
        if index == self.current {
            self.draft.name.clone()
        } else {
            self.settings.get(index).name.clone()
        }
    }

    #[cfg(test)]
    pub(crate) fn current(&self) -> usize {
        self.current
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
            default: self.draft.name.clone(),
            max_len: NAME_MAX,
        };
        let answer = view.update(cx, |view, cx| view.open_input_dialog(request, window, cx));
        cx.spawn_in(window, async move |this, cx| {
            if let Some(name) = answer.await {
                this.update_in(cx, |this, window, cx| this.set_name(&name, window, cx))
                    .ok();
            }
        })
        .detach();
    }

    /// An empty name is not taken (`if( szWork[0] != L'\0' )`).
    pub(crate) fn set_name(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !name.is_empty() {
            self.draft.name = name.chars().take(NAME_MAX).collect();
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// The font dialog for the header or the footer (`IDC_BUTTON_FONT_HEAD`). It starts at the
    /// font of the part, or at the font for half-width characters at the height of the print font.
    fn choose_font(&mut self, part: Part, window: &mut Window, cx: &mut Context<Self>) {
        let initial = self.font_dialog_start(part);
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

    fn font_dialog_start(&self, part: Part) -> FontSetting {
        let font = match part {
            Part::Header => self.draft.header_font.as_ref(),
            Part::Footer => self.draft.footer_font.as_ref(),
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
                    face_name: self.draft.print_font_face_han.clone(),
                    height: -(self.draft.print_font_height * DPI / 254),
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
            Part::Header => self.draft.header_font = chosen,
            Part::Footer => self.draft.footer_font = chosen,
        }
        cx.notify();
    }

    /// 使用: unchecking it drops the font (`IDC_CHECK_USE_FONT_HEAD`).
    pub(crate) fn drop_font(&mut self, part: Part, cx: &mut Context<Self>) {
        match part {
            Part::Header => self.draft.header_font = None,
            Part::Footer => self.draft.footer_font = None,
        }
        cx.notify();
    }

    #[cfg(test)]
    pub(crate) fn draft_mut(&mut self) -> &mut PrintSetting {
        &mut self.draft
    }

    fn toggle_list(&mut self, list: List, cx: &mut Context<Self>) {
        self.list = if self.list == Some(list) {
            None
        } else {
            Some(list)
        };
        cx.notify();
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

    /// The font names to list for a face: the installed fonts, and the face itself if this
    /// machine lacks it.
    fn font_names(&self, face: &str) -> Vec<String> {
        families(self.installed.clone(), face)
    }

    fn list_rows(&self, list: List) -> (Vec<String>, Option<usize>) {
        match list {
            List::Setting => (
                (0..self.settings.slots().len())
                    .map(|i| self.name_of(i))
                    .collect(),
                Some(self.current),
            ),
            List::FontHan => {
                let names = self.font_names(&self.draft.print_font_face_han);
                let at = names
                    .iter()
                    .position(|n| *n == self.draft.print_font_face_han);
                (names, at)
            }
            List::FontZen => {
                let names = self.font_names(&self.draft.print_font_face_zen);
                let at = names
                    .iter()
                    .position(|n| *n == self.draft.print_font_face_zen);
                (names, at)
            }
            List::Paper => (
                PAPER_INFO.iter().map(|p| p.name.to_owned()).collect(),
                PAPER_INFO
                    .iter()
                    .position(|p| p.id == self.draft.print_paper_size),
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
                if let Some(name) = self.font_names(&self.draft.print_font_face_han).get(row) {
                    self.draft.print_font_face_han = name.clone();
                }
            }
            List::FontZen => {
                if let Some(name) = self.font_names(&self.draft.print_font_face_zen).get(row) {
                    self.draft.print_font_face_zen = name.clone();
                }
            }
            List::Paper => {
                if let Some(info) = PAPER_INFO.get(row) {
                    self.draft.print_paper_size = info.id;
                }
            }
        }
        self.list = None;
        cx.notify();
    }

    fn list_view(&self, list: List, cx: &mut Context<Self>) -> impl IntoElement {
        let (rows, selected) = self.list_rows(list);
        let id = match list {
            List::Setting => "ps-list-setting",
            List::FontHan => "ps-list-han",
            List::FontZen => "ps-list-zen",
            List::Paper => "ps-list-paper",
        };
        let mut view = v_flex()
            .id(id)
            .max_h(px(160.))
            .overflow_y_scroll()
            .border_1()
            .border_color(rgb(0xaaaaaa))
            .bg(rgb(0xffffff));
        for (i, text) in rows.into_iter().enumerate() {
            view = view.child(
                div()
                    .id((id, i))
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
        text: String,
        list: List,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div().flex_1().child(
            Button::new(id)
                .label(format!("{text}  v"))
                .w_full()
                .on_click(cx.listener(move |this, _, _, cx| this.toggle_list(list, cx))),
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
            .child(div().w(px(64.)).child(Input::new(&self.numbers[index])))
            .child(unit)
    }

    fn form_row(&self, part: Part, cx: &mut Context<Self>) -> impl IntoElement {
        let (label, inputs, font, use_id, button_id) = match part {
            Part::Header => (
                "ヘッダー(U)",
                &self.header,
                self.draft.header_font.as_ref(),
                "ps-use-font-head",
                "ps-font-head",
            ),
            Part::Footer => (
                "フッター(D)",
                &self.footer,
                self.draft.footer_font.as_ref(),
                "ps-use-font-foot",
                "ps-font-foot",
            ),
        };
        let has_font = font.is_some_and(|f| !f.face.is_empty());
        v_flex()
            .gap_1()
            .child(label)
            .child(
                h_flex().gap_1().children(
                    inputs
                        .iter()
                        .map(|input| div().flex_1().child(Input::new(input))),
                ),
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
                set(&mut this.draft, *value);
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
        let now = self.collect(cx);
        let printable = model::printable(&now, self.number_columns);
        let ok_enabled = model::has_printable_area(&now, self.number_columns);
        let paper_name = find_paper_info(self.draft.print_paper_size)
            .map_or_else(String::new, |p| p.name.to_owned());
        let open = |list: List| self.list == Some(list);

        let top = h_flex()
            .gap_2()
            .items_center()
            .child(div().w(px(104.)).text_right().child("ページ設定(1)"))
            .child(self.combo("ps-setting", self.name_of(self.current), List::Setting, cx))
            .child(
                Button::new("ps-rename")
                    .label("設定名変更(2)...")
                    .on_click(cx.listener(|this, _, window, cx| this.rename(window, cx))),
            );

        let fonts = v_flex()
            .gap_1()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().w(px(104.)).text_right().child("半角フォント(F)"))
                    .child(self.combo(
                        "ps-font-han",
                        self.draft.print_font_face_han.clone(),
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
                    .child(div().w(px(104.)).text_right().child("全角フォント(K)"))
                    .child(self.combo(
                        "ps-font-zen",
                        self.draft.print_font_face_zen.clone(),
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
                    .child(self.number_row("フォント高(P)", Number::FontHeight, "(1/10mm)"))
                    .child(model::font_points_text(now.print_font_height)),
            )
            .child(self.number_row("行送り(S)", Number::LineSpacing, "(%)"))
            .child(self.number_row("段数(C)", Number::Dansuu, ""))
            .child(self.number_row("段の隙間(E)", Number::DanSpace, "(mm)"))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().w(px(104.)).text_right().child("用紙サイズ(Z)"))
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
                            .label("縦(I)")
                            .checked(self.draft.print_paper_orientation == Orientation::Portrait)
                            .on_click(cx.listener(|this, _: &bool, _, cx| {
                                this.draft.print_paper_orientation = Orientation::Portrait;
                                cx.notify();
                            })),
                    )
                    .child(
                        Radio::new("ps-landscape")
                            .label("横(A)")
                            .checked(self.draft.print_paper_orientation == Orientation::Landscape)
                            .on_click(cx.listener(|this, _: &bool, _, cx| {
                                this.draft.print_paper_orientation = Orientation::Landscape;
                                cx.notify();
                            })),
                    ),
            )
            .child(self.number_row("余白／上(T)", Number::MarginTop, "(mm)"))
            .child(self.number_row("下(B)", Number::MarginBottom, "(mm)"))
            .child(self.number_row("左(L)", Number::MarginLeft, "(mm)"))
            .child(self.number_row("右(R)", Number::MarginRight, "(mm)"));

        let (columns, lines) = printable.map_or(("-".to_owned(), "-".to_owned()), |(c, l)| {
            (c.to_string(), l.to_string())
        });
        let right = v_flex()
            .gap_1()
            .flex_1()
            .child(self.check(
                "ps-line-number",
                "行番号を印刷(N)",
                self.draft.print_line_number,
                cx,
                |s, v| {
                    s.print_line_number = v;
                },
            ))
            .child(self.check(
                "ps-word-wrap",
                "英文ワードラップ(W)",
                self.draft.print_word_wrap,
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
                        self.draft.print_kinsoku_head,
                        cx,
                        |s, v| {
                            s.print_kinsoku_head = v;
                        },
                    ))
                    .child(self.check(
                        "ps-kinsoku-ret",
                        "改行ぶら下げ",
                        self.draft.print_kinsoku_ret,
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
                        self.draft.print_kinsoku_tail,
                        cx,
                        |s, v| {
                            s.print_kinsoku_tail = v;
                        },
                    ))
                    .child(self.check(
                        "ps-kinsoku-kuto",
                        "句読点ぶら下げ",
                        self.draft.print_kinsoku_kuto,
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
                "カラー印刷(V)",
                self.draft.color_print,
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
                            .label("キャンセル(X)")
                            .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                    ),
            );
        let panel = v_flex()
            .id("print-setting-dialog")
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .track_focus(&self.focus)
            .key_context("PrintSettingDialog")
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.on_key_down(event, window, cx)
            }))
            .w(px(760.))
            .max_w_full()
            .max_h_full()
            .overflow_y_scroll()
            .p_3()
            .gap_2()
            .bg(rgb(0xf7f7f7))
            .text_color(rgb(0x222222))
            .text_size(px(13.))
            .border_1()
            .border_color(rgb(0x888888))
            .rounded_md()
            .shadow_md()
            .child(
                div()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("印刷ページ設定"),
            )
            .child(body);
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
