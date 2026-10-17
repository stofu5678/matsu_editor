//! The window of the common settings: the list of pages, the page, OK and キャンセル.
//!
//! The pages are lists of controls ([`super::model`]); this file draws them, keeps the values
//! while the window is open and does what Sakura's property sheet does (`CPropCommon`): OK takes
//! every page (`PSN_KILLACTIVE` for each, then the apply), キャンセル changes nothing.
//!
//! The keys are those of a property sheet: Tab moves between the controls, Ctrl+Tab and
//! Ctrl+PageDown go to the next page, Ctrl+Shift+Tab and Ctrl+PageUp to the previous one, Enter is
//! OK, Esc is キャンセル, and Alt with the letter of a label (`(&F)`) uses the control.
//!
//! The window is a *sheet*: a list of pages, their controls, the values, and what OK does with
//! them. The common settings are one sheet ([`Target::Common`]); the type settings
//! (`crate::type_settings`) are another ([`Target::Type`]), which brings its own pages and
//! values and writes to a `Types(N)` section.

use std::collections::HashMap;
use std::time::Duration;

use gpui::{
    AnyElement, AnyWindowHandle, App, AppContext as _, ClickEvent, Context, ElementId, Entity,
    FocusHandle, Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    PromptLevel, Render, SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
    div, prelude::FluentBuilder as _, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::radio::Radio;
use gpui_component::{Disableable as _, Selectable as _, h_flex, v_flex};
use ume_config::Profile;

use super::model::{Control, Item, Kind, Value, Values, parse_label};
use super::{LastPage, Page, all_controls, apply};
use crate::file_dialog::{self, Ask};
use crate::file_ops::windows::editors;
use crate::type_settings::{self, TypeSheet, TypeTarget};

/// The note after a control that UME Editor cannot use yet.
pub(crate) const NOT_AVAILABLE: &str = "（未対応）";

/// How often the window looks whether the editor windows are still there.
const REFRESH: Duration = Duration::from_millis(500);

const TEXT: u32 = 0x222222;
const MUTED: u32 = 0x888888;
const BORDER: u32 = 0xaaaaaa;
const SELECTED_BG: u32 = 0x2f6fd0;

/// A page of a sheet: the name on the tab and its items; `None` for a page that is not available
/// yet.
pub(crate) struct SheetPage {
    pub title: &'static str,
    pub items: Option<Vec<Item>>,
}

/// What OK writes.
pub(crate) enum Target {
    /// The common settings: `[Common]`, and every window takes them.
    Common,
    /// One document type: `Types(N)`, and the windows of that type take it.
    Type(TypeTarget),
}

pub(crate) struct CommonSettingsDialog {
    /// The index of the page that is shown.
    page: usize,
    pages: Vec<SheetPage>,
    target: Target,
    /// Every control of every available page.
    controls: Vec<Control>,
    values: Values,
    /// The values when the window opened: what OK compares with.
    original: Values,
    /// The edit boxes (numbers and texts) by control id.
    inputs: HashMap<&'static str, Entity<InputState>>,
    /// The color pickers by control id.
    pickers: HashMap<&'static str, Entity<ColorPickerState>>,
    opener: Option<AnyWindowHandle>,
    focus: FocusHandle,
    focused: bool,
    _subscriptions: Vec<Subscription>,
}

/// What `GetDlgItemInt(..., FALSE)` makes of the text of a number: the number, or 0 when it is not
/// one (a sign, a letter, a number too large).
pub(crate) fn parse_number(text: &str) -> i32 {
    text.trim()
        .parse::<u32>()
        .ok()
        .and_then(|n| i32::try_from(n).ok())
        .unwrap_or(0)
}

/// What `GetDlgItemInt(..., TRUE)` makes of the text of a signed number: the number, or 0 when it
/// is not one.
pub(crate) fn parse_signed_number(text: &str) -> i32 {
    text.trim().parse::<i32>().unwrap_or(0)
}

/// The red, green and blue (0 to 255) of a `COLORREF` (`0x00BBGGRR`).
fn channels(color: i32) -> (u32, u32, u32) {
    let c = u32::from_ne_bytes(color.to_ne_bytes());
    (c & 0xff, (c >> 8) & 0xff, (c >> 16) & 0xff)
}

/// The color a `COLORREF` is, as gpui draws it.
fn rgb_of(color: i32) -> gpui::Rgba {
    let (r, g, b) = channels(color);
    gpui::rgb((r << 16) | (g << 8) | b)
}

/// A `COLORREF` as the color picker holds it.
fn hsla_of(color: i32) -> gpui::Hsla {
    rgb_of(color).into()
}

/// The `COLORREF` of a color the picker holds.
fn colorref_of(color: gpui::Hsla) -> i32 {
    let rgba = gpui::Rgba::from(color);
    // A channel is in 0..=1, so the rounded value is in 0..=255.
    let channel = |v: f32| (v.clamp(0., 1.) * 255.).round() as u32;
    let packed = channel(rgba.r) | (channel(rgba.g) << 8) | (channel(rgba.b) << 16);
    i32::from_ne_bytes(packed.to_ne_bytes())
}

impl CommonSettingsDialog {
    pub(crate) fn new(
        page: Page,
        opener: Option<AnyWindowHandle>,
        profile: &Profile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let pages = Page::ALL
            .iter()
            .map(|p| SheetPage {
                title: p.title(),
                items: p.items(),
            })
            .collect();
        let controls = all_controls();
        let values = Values::load(&controls, profile);
        Self::build(
            page.index(),
            pages,
            controls,
            values,
            Target::Common,
            opener,
            window,
            cx,
        )
    }

    /// The type settings of one document type (`crate::type_settings`).
    pub(crate) fn for_type(
        page: usize,
        sheet: TypeSheet,
        opener: Option<AnyWindowHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let TypeSheet {
            pages,
            controls,
            values,
            target,
        } = sheet;
        let page = page.min(pages.len().saturating_sub(1));
        Self::build(
            page,
            pages,
            controls,
            values,
            Target::Type(target),
            opener,
            window,
            cx,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build(
        page: usize,
        pages: Vec<SheetPage>,
        controls: Vec<Control>,
        values: Values,
        target: Target,
        opener: Option<AnyWindowHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut inputs = HashMap::new();
        let mut pickers = HashMap::new();
        let mut subscriptions = Vec::new();
        for control in &controls {
            if matches!(control.kind, Kind::Color) {
                let picker = cx.new(|cx| ColorPickerState::new(window, cx));
                picker.update(cx, |state, cx| {
                    state.set_value(hsla_of(values.int(control.id)), window, cx);
                });
                let id = control.id;
                subscriptions.push(cx.subscribe_in(
                    &picker,
                    window,
                    move |this: &mut Self, _, event: &ColorPickerEvent, _, cx| {
                        let ColorPickerEvent::Change(Some(color)) = event else {
                            return;
                        };
                        this.set_color(id, colorref_of(*color), cx);
                    },
                ));
                pickers.insert(control.id, picker);
            }
            if !matches!(control.kind, Kind::Number { .. } | Kind::Text) {
                continue;
            }
            let initial = match values.get(control.id) {
                Some(Value::Int(n)) => n.to_string(),
                Some(Value::Text(t)) => t.clone(),
                _ => String::new(),
            };
            let state = cx.new(|cx| InputState::new(window, cx));
            state.update(cx, |state, cx| state.set_value(initial, window, cx));
            subscriptions.push(cx.subscribe_in(
                &state,
                window,
                |this: &mut Self, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.ok(window, cx);
                    }
                },
            ));
            inputs.insert(control.id, state);
        }
        // The dialog belongs to the editor: it goes when the last window does.
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(REFRESH).await;
                let alive = this
                    .update_in(cx, |_, window, cx| {
                        if editors(cx).is_empty() {
                            window.remove_window();
                        }
                    })
                    .is_ok();
                if !alive {
                    break;
                }
            }
        })
        .detach();
        Self {
            page,
            pages,
            target,
            controls,
            original: values.clone(),
            values,
            inputs,
            pickers,
            opener,
            focus: cx.focus_handle(),
            focused: false,
            _subscriptions: subscriptions,
        }
    }

    // ---- state, for the window and for the tests ----

    #[cfg(test)]
    pub(crate) fn page(&self) -> Page {
        Page::ALL.get(self.page).copied().unwrap_or(Page::General)
    }

    /// The index of the page that is shown.
    #[cfg(test)]
    pub(crate) fn page_index(&self) -> usize {
        self.page
    }

    #[cfg(test)]
    pub(crate) fn values(&self) -> &Values {
        &self.values
    }

    /// The controls of the page that is shown, in the order they are drawn.
    pub(crate) fn page_controls(&self) -> Vec<Control> {
        self.pages[self.page]
            .items
            .as_deref()
            .map(super::model::controls)
            .unwrap_or_default()
    }

    /// Whether the control can be used now: its feature is there and the other controls allow it.
    pub(crate) fn is_usable(&self, control: &Control) -> bool {
        control.live && (control.enabled)(&self.values)
    }

    /// The color the picker of a color control shows.
    #[cfg(test)]
    pub(crate) fn picker_color(&self, id: &str, cx: &App) -> Option<i32> {
        self.pickers
            .get(id)
            .and_then(|picker| picker.read(cx).value())
            .map(colorref_of)
    }

    /// The text in the edit box of a number or a text control.
    pub(crate) fn input_text(&self, id: &str, cx: &App) -> Option<String> {
        self.inputs
            .get(id)
            .map(|input| input.read(cx).value().to_string())
    }

    // ---- changes ----

    /// Shows a page (a click on its name, Ctrl+Tab, `F_OPTION_*`). The page is remembered for the
    /// next `F_OPTION` (`m_nPageNum`).
    pub(crate) fn select_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.select_index(page.index(), cx);
    }

    /// Shows the page at `index` of the sheet and remembers it for the next time.
    pub(crate) fn select_index(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.pages.len() {
            return;
        }
        self.page = index;
        match &self.target {
            Target::Common => {
                if let Some(page) = Page::ALL.get(index) {
                    cx.set_global(LastPage(*page));
                }
            }
            Target::Type(_) => type_settings::remember_page(cx, index),
        }
        cx.notify();
    }

    fn step_page(&mut self, forward: bool, cx: &mut Context<Self>) {
        let count = self.pages.len();
        let at = self.page;
        let next = if forward {
            (at + 1) % count
        } else {
            (at + count - 1) % count
        };
        self.select_index(next, cx);
    }

    fn control(&self, id: &str) -> Option<Control> {
        self.controls.iter().find(|c| c.id == id).copied()
    }

    /// A check box.
    pub(crate) fn set_bool(&mut self, id: &'static str, on: bool, cx: &mut Context<Self>) {
        if let Some(control) = self.control(id).filter(|c| self.is_usable(c)) {
            self.values.set(id, Value::Bool(on));
            (control.changed)(&mut self.values, id);
            cx.notify();
        }
    }

    /// A radio group, a drop-down list or a list box.
    pub(crate) fn set_int(&mut self, id: &'static str, value: i32, cx: &mut Context<Self>) {
        if let Some(control) = self.control(id).filter(|c| self.is_usable(c)) {
            self.values.set(id, Value::Int(value));
            (control.changed)(&mut self.values, id);
            cx.notify();
        }
    }

    /// A color (`COLORREF`).
    pub(crate) fn set_color(&mut self, id: &'static str, color: i32, cx: &mut Context<Self>) {
        self.set_int(id, color, cx);
    }

    /// Types into the edit box of a number or a text control.
    pub(crate) fn set_input(
        &mut self,
        id: &'static str,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(input) = self.inputs.get(id).cloned()
            && self.control(id).is_some_and(|c| self.is_usable(&c))
        {
            let text = text.to_owned();
            input.update(cx, |state, cx| state.set_value(text, window, cx));
            cx.notify();
        }
    }

    /// `PSN_KILLACTIVE` of every page (`GetData`): the edit boxes become values, numbers into their
    /// range.
    fn commit_inputs(&mut self, cx: &App) {
        for control in &self.controls {
            if !control.live {
                continue;
            }
            let Some(input) = self.inputs.get(control.id) else {
                continue;
            };
            let text = input.read(cx).value().to_string();
            match control.kind {
                Kind::Number { min, .. } if min < 0 => self
                    .values
                    .set(control.id, Value::Int(parse_signed_number(&text))),
                Kind::Number { .. } => self.values.set(control.id, Value::Int(parse_number(&text))),
                Kind::Text => self.values.set(control.id, Value::Text(text)),
                _ => {}
            }
        }
        self.values.clamp_numbers(&self.controls);
    }

    /// OK: the values are written and every window takes them; the window closes.
    pub(crate) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_inputs(cx);
        match &self.target {
            Target::Common => {
                if let Some(message) = super::pages::tab::blocked(cx, &self.values, &self.original)
                {
                    // Nothing is written; the dialog stays so that the user can choose again.
                    drop(window.prompt(PromptLevel::Info, message, None, &["OK"], cx));
                    return;
                }
                apply::commit(
                    cx,
                    &self.controls,
                    &self.original,
                    &self.values,
                    self.opener,
                );
            }
            Target::Type(target) => type_settings::commit(cx, target, &self.values),
        }
        window.remove_window();
    }

    /// キャンセル: nothing is changed.
    pub(crate) fn cancel(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        window.remove_window();
    }

    /// Asks (OK / キャンセル), and when it is OK runs `action` and says `done`. For the buttons
    /// that act at once, such as 履歴をクリア.
    pub(crate) fn confirm_then(
        &mut self,
        question: &str,
        done: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
        action: impl FnOnce(&mut App) + 'static,
    ) {
        let answer = window.prompt(PromptLevel::Info, question, None, &["OK", "キャンセル"], cx);
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(0) = answer.await {
                this.update_in(cx, |_, window, cx| {
                    action(cx);
                    // Nobody waits for this answer.
                    drop(window.prompt(PromptLevel::Info, done, None, &["OK"], cx));
                })
                .ok();
            }
        })
        .detach();
    }

    /// The button beside a folder box: chooses a folder, starting where the box says (its meta
    /// folders expanded), and puts it in the box with `%` doubled (`IDC_BUTTON_FILEOPENDIR`).
    pub(crate) fn choose_folder(
        &mut self,
        id: &'static str,
        title: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = self.input_text(id, cx).unwrap_or_default();
        let start = file_dialog::expanded_folder(cx, &text);
        let receiver =
            file_dialog::ask(window, cx, Ask::folder().start(start).titled(title, "選択"));
        cx.spawn_in(window, async move |this, cx| {
            let chosen = receiver.await.ok().flatten();
            if let Some(path) = chosen.and_then(|paths| paths.into_iter().next()) {
                let text = path.to_string_lossy().replace('%', "%%");
                this.update_in(cx, |this, window, cx| this.set_input(id, &text, window, cx))
                    .ok();
            }
        })
        .detach();
    }

    /// Alt with the letter of a label: uses that control of the page. A check box is switched, a
    /// radio button chosen, an edit box gets the focus, a drop-down list goes to its next choice, a
    /// button is pressed. Returns whether a control was used.
    pub(crate) fn use_access_key(
        &mut self,
        key: char,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = key.to_lowercase().next().unwrap_or(key);
        // The buttons of the window itself.
        match key {
            'o' if self
                .page_controls()
                .iter()
                .all(|c| parse_label(c.label).1 != Some('o')) =>
            {
                self.ok(window, cx);
                return true;
            }
            'x' if self
                .page_controls()
                .iter()
                .all(|c| parse_label(c.label).1 != Some('x')) =>
            {
                self.cancel(window, cx);
                return true;
            }
            _ => {}
        }
        let Some(control) = self
            .page_controls()
            .into_iter()
            .find(|c| parse_label(c.label).1 == Some(key) && self.is_usable(c))
        else {
            return false;
        };
        match control.kind {
            Kind::Check => {
                let on = self.values.bool(control.id);
                self.set_bool(control.id, !on, cx);
            }
            Kind::Radio { value } => self.set_int(control.id, value, cx),
            Kind::Choice { options } | Kind::List { options } => {
                let now = self.values.int(control.id);
                let at = options.iter().position(|(_, v)| *v == now).unwrap_or(0);
                let next = options[(at + 1) % options.len()].1;
                self.set_int(control.id, next, cx);
            }
            Kind::Number { .. } | Kind::Text => {
                if let Some(input) = self.inputs.get(control.id).cloned() {
                    input.update(cx, |state, cx| state.focus(window, cx));
                }
            }
            Kind::Color => {
                if let Some(picker) = self.pickers.get(control.id).cloned() {
                    picker.update(cx, |state, cx| state.set_open(true, cx));
                }
            }
            Kind::Button { run } => run(self, window, cx),
        }
        true
    }

    /// Up and Down in a page with a list box move its choice (the arrow keys of a list box), when
    /// no edit box is being typed in. Returns whether a choice was moved.
    fn step_list(&mut self, forward: bool, window: &Window, cx: &mut Context<Self>) -> bool {
        let typing = self
            .inputs
            .values()
            .any(|input| input.read(cx).focus_handle(cx).is_focused(window));
        if typing {
            return false;
        }
        let Some((control, options)) =
            self.page_controls().into_iter().find_map(|c| match c.kind {
                Kind::List { options } if self.is_usable(&c) => Some((c, options)),
                _ => None,
            })
        else {
            return false;
        };
        let now = self.values.int(control.id);
        let at = options.iter().position(|(_, v)| *v == now).unwrap_or(0);
        let next = if forward {
            (at + 1).min(options.len().saturating_sub(1))
        } else {
            at.saturating_sub(1)
        };
        self.set_int(control.id, options[next].1, cx);
        true
    }

    /// Makes the color pickers show the colors of the values (a change of another control may have
    /// set them, such as the choice of a color item).
    fn sync_pickers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for (id, picker) in &self.pickers {
            let wanted = self.values.int(id);
            let shown = picker.read(cx).value().map(colorref_of);
            if shown != Some(wanted) {
                picker.update(cx, |state, cx| state.set_value(hsla_of(wanted), window, cx));
            }
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        let handled = match key {
            "escape" => {
                self.cancel(window, cx);
                true
            }
            "enter" if !modifiers.alt => {
                self.ok(window, cx);
                true
            }
            "tab" if modifiers.control => {
                self.step_page(!modifiers.shift, cx);
                true
            }
            "pagedown" if modifiers.control => {
                self.step_page(true, cx);
                true
            }
            "pageup" if modifiers.control => {
                self.step_page(false, cx);
                true
            }
            "up" | "down" if !modifiers.modified() => self.step_list(key == "down", window, cx),
            k if modifiers.alt && !modifiers.control && k.chars().count() == 1 => {
                let letter = k.chars().next().unwrap_or(' ');
                self.use_access_key(letter, window, cx)
            }
            _ => false,
        };
        if handled {
            cx.stop_propagation();
        }
    }

    // ---- drawing ----

    fn element_id(control: &Control) -> ElementId {
        let (id, value) = control.element_id();
        ElementId::from(SharedString::from(format!("cs-{id}-{value}")))
    }

    fn not_available_note() -> impl IntoElement {
        div()
            .text_size(px(11.))
            .text_color(rgb(MUTED))
            .child(NOT_AVAILABLE)
    }

    fn control_element(&self, control: &Control, cx: &mut Context<Self>) -> AnyElement {
        let usable = self.is_usable(control);
        let (label, _) = parse_label(control.label);
        let id = control.id;
        let note = (!control.live).then(Self::not_available_note);
        match control.kind {
            Kind::Check => h_flex()
                .gap_2()
                .items_center()
                .child(
                    Checkbox::new(Self::element_id(control))
                        .label(label)
                        .checked(self.values.bool(id))
                        .disabled(!usable)
                        .on_click(cx.listener(move |this, on: &bool, _, cx| {
                            this.set_bool(id, *on, cx);
                        })),
                )
                .children(note)
                .into_any_element(),
            Kind::Radio { value } => h_flex()
                .gap_2()
                .items_center()
                .child(
                    Radio::new(Self::element_id(control))
                        .label(label)
                        .checked(self.values.int(id) == value)
                        .disabled(!usable)
                        .on_click(cx.listener(move |this, _: &bool, _, cx| {
                            this.set_int(id, value, cx);
                        })),
                )
                .children(note)
                .into_any_element(),
            Kind::List { options } => {
                let current = self.values.int(id);
                let rows = options.iter().map(|(text, value)| {
                    let value = *value;
                    div()
                        .id(Self::element_id(&Control {
                            kind: Kind::Radio { value },
                            ..*control
                        }))
                        .px_2()
                        .h(px(20.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .when(current == value, |d| {
                            d.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
                        })
                        .child(*text)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.set_int(id, value, cx);
                        }))
                });
                v_flex()
                    .gap_1()
                    .when(!label.is_empty(), |this| this.child(label.clone()))
                    .child(
                        div()
                            .id(Self::element_id(control))
                            .h(px(150.))
                            .overflow_y_scroll()
                            .bg(rgb(0xffffff))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .children(rows),
                    )
                    .into_any_element()
            }
            Kind::Color => {
                let chooser = match self.pickers.get(id) {
                    Some(picker) if usable => ColorPicker::new(picker).into_any_element(),
                    _ => div()
                        .w(px(25.))
                        .h(px(14.))
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb_of(self.values.int(id)))
                        .into_any_element(),
                };
                h_flex()
                    .gap_2()
                    .items_center()
                    .when(!label.is_empty(), |this| this.child(label.clone()))
                    .child(chooser)
                    .children(note)
                    .into_any_element()
            }
            Kind::Choice { options } => {
                let current = self.values.int(id);
                let choices = options.iter().map(|(text, value)| {
                    let value = *value;
                    Button::new(Self::element_id(&Control {
                        kind: Kind::Radio { value },
                        ..*control
                    }))
                    .label(*text)
                    .selected(current == value)
                    .disabled(!usable)
                    .on_click(cx.listener(move |this, _, _, cx| this.set_int(id, value, cx)))
                });
                v_flex()
                    .gap_1()
                    .when(!label.is_empty(), |this| this.child(label.clone()))
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .flex_wrap()
                            .children(choices)
                            .children(note),
                    )
                    .into_any_element()
            }
            Kind::Number { .. } | Kind::Text => {
                let Some(input) = self.inputs.get(id) else {
                    return div().into_any_element();
                };
                let wide = matches!(control.kind, Kind::Text);
                v_flex()
                    .gap_1()
                    .when(!label.is_empty(), |this| this.child(label.clone()))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .when(wide, |d| d.flex_1().min_w(px(160.)))
                                    .when(!wide, |d| d.w(px(80.)))
                                    .child(Input::new(input).disabled(!usable)),
                            )
                            .children(note),
                    )
                    .into_any_element()
            }
            Kind::Button { run } => h_flex()
                .gap_2()
                .items_center()
                .child(
                    Button::new(Self::element_id(control))
                        .label(label)
                        .disabled(!usable)
                        .on_click(cx.listener(move |this, _, window, cx| run(this, window, cx))),
                )
                .children(note)
                .into_any_element(),
        }
    }

    fn item_element(&self, item: &Item, cx: &mut Context<Self>) -> AnyElement {
        match item {
            Item::Control(control) => self.control_element(control, cx),
            Item::Label(text) => div()
                .text_color(rgb(MUTED))
                .child(parse_label(text).0)
                .into_any_element(),
            Item::Group(title, items) => {
                let children: Vec<AnyElement> = items
                    .iter()
                    .map(|item| self.item_element(item, cx))
                    .collect();
                v_flex()
                    .gap_1()
                    .p_2()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_md()
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(parse_label(title).0),
                    )
                    .children(children)
                    .into_any_element()
            }
            Item::Columns(columns) => {
                let columns: Vec<AnyElement> = columns
                    .iter()
                    .map(|column| {
                        let children: Vec<AnyElement> = column
                            .iter()
                            .map(|item| self.item_element(item, cx))
                            .collect();
                        v_flex()
                            .flex_1()
                            .gap_2()
                            .children(children)
                            .into_any_element()
                    })
                    .collect();
                h_flex()
                    .gap_3()
                    .items_start()
                    .children(columns)
                    .into_any_element()
            }
            Item::Row(items) => {
                let children: Vec<AnyElement> = items
                    .iter()
                    .map(|item| self.item_element(item, cx))
                    .collect();
                h_flex()
                    .gap_3()
                    .items_center()
                    .flex_wrap()
                    .children(children)
                    .into_any_element()
            }
        }
    }

    fn page_element(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.pages[self.page].items.as_deref() {
            Some(items) => {
                let children: Vec<AnyElement> = items
                    .iter()
                    .map(|item| self.item_element(item, cx))
                    .collect();
                v_flex().gap_2().children(children).into_any_element()
            }
            None => div()
                .text_color(rgb(MUTED))
                .child(format!(
                    "「{}」のページは UME Editor ではまだ使えません。",
                    self.pages[self.page].title
                ))
                .into_any_element(),
        }
    }
}

impl Focusable for CommonSettingsDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for CommonSettingsDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.focused {
            self.focused = true;
            window.focus(&self.focus, cx);
        }
        self.sync_pickers(window, cx);
        let ui_font = gpui_component::Theme::global(cx).font_family.clone();
        let pages: Vec<AnyElement> = self
            .pages
            .iter()
            .enumerate()
            .map(|(index, page)| {
                let selected = index == self.page;
                div()
                    .id(SharedString::from(format!("cs-page-{index}")))
                    .px_3()
                    .py_1()
                    .cursor_pointer()
                    .when(selected, |d| {
                        d.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
                    })
                    .child(page.title)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.select_index(index, cx);
                    }))
                    .into_any_element()
            })
            .collect();
        let body = self.page_element(cx);
        div()
            .id("common-settings")
            .key_context("CommonSettings")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0xf0f0f0))
            .text_color(rgb(TEXT))
            .text_size(px(13.))
            .font_family(ui_font)
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        v_flex()
                            .w(px(150.))
                            .py_2()
                            .border_r_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(0xe6e6e6))
                            .children(pages),
                    )
                    .child(
                        div()
                            .id("cs-body")
                            .flex_1()
                            .p_3()
                            .overflow_y_scroll()
                            .child(body),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .p_3()
                    .justify_end()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .child(
                        Button::new("cs-ok")
                            .label("OK")
                            .on_click(cx.listener(|this, _, window, cx| this.ok(window, cx))),
                    )
                    .child(
                        Button::new("cs-cancel")
                            .label("キャンセル(X)")
                            .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(Button::new("cs-help").label("ヘルプ(H)").disabled(true))
                            .child(Self::not_available_note()),
                    ),
            )
    }
}
