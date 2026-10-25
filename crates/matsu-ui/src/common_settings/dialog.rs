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

use crate::input_select::input_field;
use gpui::{
    AnyElement, AnyWindowHandle, App, AppContext as _, ClickEvent, Context, ElementId, Entity,
    FocusHandle, Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    PromptLevel, Render, Role, ScrollHandle, SharedString, StatefulInteractiveElement, Styled,
    Subscription, Window, div, prelude::FluentBuilder as _, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::radio::Radio;
use gpui_component::scroll::{Scrollbar, ScrollbarMode};
use gpui_component::{Disableable as _, Selectable as _, h_flex, v_flex};
use matsu_config::Profile;
use matsu_config::settings::FontSetting;

use super::model::{Control, Item, Kind, Value, Values, parse_label};
use super::semantics;
use super::table::{self, Op, Refused};
use super::{LastPage, Page, all_controls, apply};
use crate::a11y::{AriaDisabled as _, plain_name};
use crate::file_dialog::{self, Ask};
use crate::file_ops::windows::editors;
use crate::type_settings::{self, TypeSheet, TypeTarget};

/// The note after a control that MATSU Editor cannot use yet.
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
    /// The edit boxes under the editable lists by list id and column.
    table_inputs: HashMap<(&'static str, usize), Entity<InputState>>,
    /// The selected row of each editable list.
    table_selected: HashMap<&'static str, Option<usize>>,
    opener: Option<AnyWindowHandle>,
    focus: FocusHandle,
    focused: bool,
    /// The scroll position of the page body; the scroll bar draws from it.
    pub(crate) body_scroll: ScrollHandle,
    /// The scale factor of the window, as of the last paint (pictures in a page are made sharp
    /// for it).
    scale_factor: f32,
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
        let mut table_inputs = HashMap::new();
        let mut table_selected = HashMap::new();
        let mut subscriptions = Vec::new();
        for control in &controls {
            if let Kind::Table { spec } = control.kind {
                // `SetData`: the first row is selected, and the edit boxes show it.
                let rows = values.rows(control.id);
                table_selected.insert(control.id, (!rows.is_empty()).then_some(0));
                for column in 0..spec.columns.len() {
                    let initial = rows
                        .first()
                        .and_then(|row| row.get(column))
                        .cloned()
                        .unwrap_or_default();
                    let state = cx.new(|cx| input_field(window, cx));
                    state.update(cx, |state, cx| state.set_value(initial, window, cx));
                    table_inputs.insert((control.id, column), state);
                }
                continue;
            }
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
            let state = cx.new(|cx| input_field(window, cx));
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
            table_inputs,
            table_selected,
            opener,
            focus: cx.focus_handle(),
            focused: false,
            body_scroll: ScrollHandle::default(),
            scale_factor: 1.0,
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

    /// The scale factor of the window.
    pub(crate) fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    /// The values the controls hold now, for a custom widget to read.
    pub(crate) fn values(&self) -> &Values {
        &self.values
    }

    /// Changes several values at once (an import of a page). The controls show them afterwards.
    pub(crate) fn edit_values(&mut self, change: impl FnOnce(&mut Values), cx: &mut Context<Self>) {
        change(&mut self.values);
        cx.notify();
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

    /// The selected row of an editable list.
    pub(crate) fn selected_row(&self, id: &str) -> Option<usize> {
        self.table_selected.get(id).copied().flatten()
    }

    /// The text in the edit box of one column of an editable list.
    #[cfg(test)]
    pub(crate) fn table_input_text(&self, id: &str, column: usize, cx: &App) -> Option<String> {
        self.table_inputs
            .get(&(self.control(id)?.id, column))
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
        // The next page starts at its top.
        self.body_scroll.set_offset(gpui::point(px(0.), px(0.)));
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

    /// A font chosen with the font dialog ([`Kind::Font`]). The control's `changed` runs, so that a
    /// page can set the controls that go with it (the check box of the font of a type).
    pub(crate) fn set_font(&mut self, id: &'static str, font: FontSetting, cx: &mut Context<Self>) {
        if self.control(id).is_some() {
            self.values.set(id, Value::Font(font));
            if let Some(control) = self.control(id) {
                (control.changed)(&mut self.values, id);
            }
            cx.notify();
        }
    }

    /// Shows the font dialog starting at `initial` for the font control `id`: the font the user
    /// chooses becomes its value. The dialog may answer after this has returned.
    pub(crate) fn choose_font(
        &mut self,
        id: &'static str,
        initial: FontSetting,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let this = cx.entity().downgrade();
        let owner = crate::font_dialog::owner_of(window);
        crate::font_dialog::choose(
            cx,
            owner,
            initial,
            Box::new(move |app: &mut App, font| {
                this.update(app, |dialog, cx| dialog.set_font(id, font, cx))
                    .ok();
            }),
        );
    }

    /// A list of numbers that a custom widget edits ([`Kind::Numbers`]).
    pub(crate) fn set_numbers(&mut self, id: &'static str, list: Vec<i32>, cx: &mut Context<Self>) {
        if self.control(id).is_some_and(|c| self.is_usable(&c)) {
            self.values.set(id, Value::Numbers(list));
            cx.notify();
        }
    }

    /// The rows of an editable list that a page edits with a widget of its own (the keyword sets).
    pub(crate) fn set_rows(
        &mut self,
        id: &'static str,
        rows: Vec<table::Row>,
        cx: &mut Context<Self>,
    ) {
        if self.control(id).is_some_and(|c| self.is_usable(&c)) {
            self.values.set(id, Value::Rows(rows));
            cx.notify();
        }
    }

    /// Puts new rows into an editable list after an import (`SetData`): no row is selected and the
    /// edit boxes are empty.
    pub(crate) fn replace_rows(
        &mut self,
        id: &'static str,
        rows: Vec<table::Row>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.table_control(id).is_none() {
            return;
        }
        self.values.set(id, Value::Rows(rows));
        self.table_selected.insert(id, None);
        self.show_row(id, None, window, cx);
        cx.notify();
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

    fn table_control(&self, id: &str) -> Option<(Control, &'static table::TableSpec)> {
        let control = self.control(id).filter(|c| self.is_usable(c))?;
        match control.kind {
            Kind::Table { spec } => Some((control, spec)),
            _ => None,
        }
    }

    /// Puts the texts of row `row` of the list in its edit boxes, or empties them (`LVN_ITEMCHANGED`
    /// of `IDC_LIST_FNAME`).
    fn show_row(
        &mut self,
        id: &'static str,
        row: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let texts = row.and_then(|i| self.values.rows(id).get(i).cloned());
        for ((table, column), input) in &self.table_inputs {
            if *table != id {
                continue;
            }
            let text = texts
                .as_ref()
                .and_then(|r| r.get(*column).cloned())
                .unwrap_or_default();
            input.update(cx, |state, cx| state.set_value(text, window, cx));
        }
    }

    /// A click on a row of an editable list.
    pub(crate) fn select_row(
        &mut self,
        id: &'static str,
        row: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.table_control(id).is_none() || row >= self.values.rows(id).len() {
            return;
        }
        self.table_selected.insert(id, Some(row));
        self.show_row(id, Some(row), window, cx);
        cx.notify();
    }

    /// Replaces all the rows of an editable list (an import) and selects the first one, with its
    /// texts in the edit boxes (`SetDataKeywordList`: `ListView_SetItemState( 0, ... )`).
    pub(crate) fn replace_table_rows(
        &mut self,
        id: &'static str,
        rows: Vec<table::Row>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.table_control(id).is_none() {
            return;
        }
        let selected = (!rows.is_empty()).then_some(0);
        self.values.set(id, Value::Rows(rows));
        self.table_selected.insert(id, selected);
        self.show_row(id, selected, window, cx);
        cx.notify();
    }

    /// Types into the edit box of one column of an editable list.
    #[cfg(test)]
    pub(crate) fn set_table_input(
        &mut self,
        id: &'static str,
        column: usize,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.table_control(id).is_some()
            && let Some(input) = self.table_inputs.get(&(id, column)).cloned()
        {
            let text = text.to_owned();
            input.update(cx, |state, cx| state.set_value(text, window, cx));
            cx.notify();
        }
    }

    /// A button of an editable list (`CPropFileName::DispatchEvent`, `BN_CLICKED`).
    pub(crate) fn table_op(
        &mut self,
        id: &'static str,
        op: Op,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((_, spec)) = self.table_control(id) else {
            return;
        };
        if !spec.ops.contains(&op) {
            return;
        }
        let fields: Vec<String> = (0..spec.columns.len())
            .map(|column| {
                self.table_inputs
                    .get(&(id, column))
                    .map(|input| input.read(cx).value().to_string())
                    .unwrap_or_default()
            })
            .collect();
        let mut rows = self.values.rows(id).to_vec();
        let mut selected = self.selected_row(id);
        let base = crate::doc_types::with_user(cx, |user| {
            crate::settings::history_file(user)
                .and_then(|path| path.parent().map(std::path::Path::to_owned))
        });
        match table::apply_in(spec, &mut rows, &mut selected, op, &fields, base.as_deref()) {
            Ok(true) => {
                self.values.set(id, Value::Rows(rows));
                self.table_selected.insert(id, selected);
                self.show_row(id, selected, window, cx);
                cx.notify();
            }
            Ok(false) => {}
            Err(Refused::Full) => {
                // Nobody waits for this answer.
                drop(window.prompt(PromptLevel::Info, spec.full_message, None, &["OK"], cx));
            }
            Err(Refused::Rejected(message)) => {
                drop(window.prompt(PromptLevel::Info, &message, None, &["OK"], cx));
            }
        }
    }

    /// Alt with the letter of a button or of an edit box under an editable list.
    fn use_table_key(&mut self, key: char, window: &mut Window, cx: &mut Context<Self>) -> bool {
        for control in self.page_controls() {
            let Some((control, spec)) = self.table_control(control.id) else {
                continue;
            };
            if let Some(op) = spec
                .ops
                .iter()
                .find(|op| parse_label(op.label()).1 == Some(key))
            {
                self.table_op(control.id, *op, window, cx);
                return true;
            }
            let column = spec
                .columns
                .iter()
                .position(|c| parse_label(c.edit_label).1 == Some(key));
            if let Some(input) = column.and_then(|c| self.table_inputs.get(&(control.id, c))) {
                input.update(cx, |state, cx| state.focus(window, cx));
                return true;
            }
        }
        false
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
        let mut usable = self
            .page_controls()
            .into_iter()
            .filter(|c| parse_label(c.label).1 == Some(key) && self.is_usable(c));
        let Some(control) = usable.next() else {
            return self.use_table_key(key, window, cx);
        };
        // A mnemonic that several controls share only moves the focus between them in a Windows
        // dialog (`IsDialogMessage`); it clicks none of them. These controls have no focus to
        // move, so nothing is done (the custom menu page has two controls with S).
        if usable.next().is_some() {
            return true;
        }
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
            Kind::Numbers { .. } | Kind::Table { .. } | Kind::Font { .. } => {}
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
        let name = control.accessible_name();
        let id = control.id;
        let note = (!control.live).then(Self::not_available_note);
        match control.kind {
            Kind::Check => h_flex()
                .gap_2()
                .items_center()
                .child(
                    Checkbox::new(Self::element_id(control))
                        .label(label)
                        .accessibility_label(name)
                        .checked(self.values.bool(id))
                        .disabled(!usable)
                        .aria_disabled(!usable)
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
                        .accessibility_label(name)
                        .checked(self.values.int(id) == value)
                        .disabled(!usable)
                        .aria_disabled(!usable)
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
                    semantics::list_option(
                        div().id(Self::element_id(&Control {
                            kind: Kind::Radio { value },
                            ..*control
                        })),
                        &plain_name(text),
                        current == value,
                    )
                    .px_2()
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .when(current == value, |d| {
                        d.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
                    })
                    .child(*text)
                    .on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            this.set_int(id, value, cx);
                        },
                    ))
                });
                v_flex()
                    .gap_1()
                    .when(!label.is_empty(), |this| this.child(label.clone()))
                    .child(
                        semantics::list_box(div().id(Self::element_id(control)), &name)
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
                    Some(picker) if usable => ColorPicker::new(picker)
                        .accessibility_label(name.clone())
                        .into_any_element(),
                    _ => div()
                        .w(px(25.))
                        .h(px(14.))
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb_of(self.values.int(id)))
                        .into_any_element(),
                };
                semantics::named_group(
                    h_flex().id(SharedString::from(format!("cs-{id}-color"))),
                    Role::Group,
                    name,
                )
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
                    // A choice is a radio button of the group (the chosen one is checked).
                    .role(Role::RadioButton)
                    .toggled(current == value)
                    .disabled(!usable)
                    .on_click(cx.listener(move |this, _, _, cx| this.set_int(id, value, cx)))
                });
                v_flex()
                    .gap_1()
                    .when(!label.is_empty(), |this| this.child(label.clone()))
                    .child(
                        semantics::named_group(
                            h_flex().id(SharedString::from(format!("cs-{id}-choices"))),
                            Role::RadioGroup,
                            name,
                        )
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
                                    .child(Input::new(input).aria_label(name).disabled(!usable)),
                            )
                            .children(note),
                    )
                    .into_any_element()
            }
            Kind::Table { spec } => self.table_element(control, spec, usable, cx),
            Kind::Button { run } => h_flex()
                .gap_2()
                .items_center()
                .child(
                    Button::new(Self::element_id(control))
                        .label(label)
                        .accessibility_label(name)
                        .disabled(!usable)
                        .on_click(cx.listener(move |this, _, window, cx| run(this, window, cx))),
                )
                .children(note)
                .into_any_element(),
            // A list of numbers is drawn by the `Item::Custom` that edits it.
            Kind::Numbers { .. } | Kind::Font { .. } => div().into_any_element(),
        }
    }

    /// An editable list: the rows with their headings, the edit boxes of the columns under them,
    /// and the buttons beside them (the moves above, the changes below, as `IDD_PROP_FNAME`).
    fn table_element(
        &self,
        control: &Control,
        spec: &'static table::TableSpec,
        usable: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let id = control.id;
        let (label, _) = parse_label(control.label);
        let name = control.accessible_name();
        let selected = self.selected_row(id);
        let heading =
            semantics::header_row(h_flex().id(SharedString::from(format!("cs-{id}-head"))))
                .bg(rgb(0xe6e6e6))
                .border_b_1()
                .border_color(rgb(BORDER))
                .children(spec.columns.iter().enumerate().map(|(column, c)| {
                    semantics::column_header(
                        div().id(SharedString::from(format!("cs-{id}-head-{column}"))),
                        c.title,
                        column,
                    )
                    .w(gpui::relative(c.width as f32 / 100.))
                    .px_2()
                    .child(c.title)
                }));
        let rows: Vec<AnyElement> =
            self.values
                .rows(id)
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    semantics::table_row(
                        h_flex().id(SharedString::from(format!("cs-{id}-row-{index}"))),
                        row,
                        index,
                        selected == Some(index),
                    )
                    .h(px(20.))
                    .items_center()
                    .cursor_pointer()
                    .when(selected == Some(index), |d| {
                        d.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
                    })
                    .children(spec.columns.iter().zip(row).enumerate().map(
                        |(column, (c, text))| {
                            semantics::cell(
                                div().id(SharedString::from(format!(
                                    "cs-{id}-row-{index}-{column}"
                                ))),
                                text,
                                c.title,
                                column,
                            )
                            .w(gpui::relative(c.width as f32 / 100.))
                            .px_2()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(text.clone())
                        },
                    ))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.select_row(id, index, window, cx);
                    }))
                    .into_any_element()
                })
                .collect();
        let edits = spec.columns.iter().enumerate().filter_map(|(column, c)| {
            let input = self.table_inputs.get(&(id, column))?;
            Some(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().w(px(90.)).child(parse_label(c.edit_label).0))
                    .child(
                        div().flex_1().child(
                            Input::new(input)
                                .aria_label(plain_name(c.edit_label))
                                .disabled(!usable),
                        ),
                    ),
            )
        });
        let button = |op: Op, cx: &mut Context<Self>| {
            Button::new(SharedString::from(format!("cs-{id}-{op:?}")))
                .label(parse_label(op.label()).0)
                .disabled(!usable)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.table_op(id, op, window, cx);
                }))
        };
        let (moves, changes): (Vec<Op>, Vec<Op>) = Op::ALL
            .into_iter()
            .filter(|op| spec.ops.contains(op))
            .partition(|op| matches!(op, Op::Top | Op::Up | Op::Down | Op::Last));
        let moves: Vec<Button> = moves.into_iter().map(|op| button(op, cx)).collect();
        let changes: Vec<Button> = changes.into_iter().map(|op| button(op, cx)).collect();
        v_flex()
            .gap_1()
            .when(!label.is_empty(), |this| this.child(label.clone()))
            .child(
                h_flex()
                    .gap_2()
                    .items_start()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_2()
                            .child(
                                semantics::table(
                                    v_flex().id(Self::element_id(control)),
                                    &name,
                                    self.values.rows(id).len(),
                                    spec.columns.len(),
                                )
                                .h(px(150.))
                                .bg(rgb(0xffffff))
                                .border_1()
                                .border_color(rgb(BORDER))
                                .child(heading)
                                .child(
                                    v_flex()
                                        .id(SharedString::from(format!("cs-{id}-rows")))
                                        .flex_1()
                                        .overflow_y_scroll()
                                        .children(rows),
                                ),
                            )
                            .children(edits),
                    )
                    .child(
                        v_flex()
                            .gap_3()
                            .child(v_flex().gap_1().children(moves))
                            .child(v_flex().gap_1().children(changes)),
                    ),
            )
            .children((!control.live).then(Self::not_available_note))
            .into_any_element()
    }

    fn item_element(&self, item: &Item, cx: &mut Context<Self>) -> AnyElement {
        match item {
            Item::Control(control) => self.control_element(control, cx),
            Item::Label(text) => div()
                .text_color(rgb(MUTED))
                .child(parse_label(text).0)
                .into_any_element(),
            Item::Custom(draw) => draw(self, cx),
            Item::Hidden(_) => div().into_any_element(),
            Item::Group(title, items) => {
                let children: Vec<AnyElement> = items
                    .iter()
                    .map(|item| self.item_element(item, cx))
                    .collect();
                semantics::group(
                    v_flex().id(SharedString::from(format!("cs-group-{:p}", items.as_ptr()))),
                    &plain_name(title),
                )
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
                    "「{}」のページは MATSU Editor ではまだ使えません。",
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
        self.scale_factor = window.scale_factor();
        let ui_font = gpui_component::Theme::global(cx).font_family.clone();
        let pages: Vec<AnyElement> = self
            .pages
            .iter()
            .enumerate()
            .map(|(index, page)| {
                let selected = index == self.page;
                semantics::page_tab(
                    div().id(SharedString::from(format!("cs-page-{index}"))),
                    page.title,
                    selected,
                    index,
                )
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
        let dialog_name = match self.target {
            Target::Common => "共通設定",
            Target::Type(_) => "タイプ別設定",
        };
        semantics::dialog(div().id("common-settings"), dialog_name)
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
                    // `h_flex` centers its children. The body takes the whole height instead, so
                    // that a page taller than the window scrolls from its top: centered, it was as
                    // tall as the page and cut at the top and the bottom, with nothing to scroll.
                    .items_stretch()
                    .child(
                        semantics::page_list(v_flex().id("cs-pages"), self.pages.len())
                            .w(px(150.))
                            .py_2()
                            .border_r_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(0xe6e6e6))
                            .children(pages),
                    )
                    .child(
                        // The scroll bar is a sibling of the scrolling body, not its child: inside
                        // it, the bar's overlay itself made a page that fits overflow by its padding.
                        div()
                            .relative()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .child(
                                semantics::page_panel(
                                    div().id("cs-body"),
                                    self.pages[self.page].title,
                                )
                                .debug_selector(|| "cs-body".to_owned())
                                .size_full()
                                .p_3()
                                .overflow_y_scroll()
                                .track_scroll(&self.body_scroll)
                                .child(body),
                            )
                            // A page taller than the window scrolls; the bar shows that more is
                            // below (Sakura's pages fit without scrolling). It is always drawn
                            // while the page overflows: the default fades out when idle.
                            .child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .debug_selector(|| "cs-scrollbar".to_owned())
                                    .child(
                                        Scrollbar::vertical(&self.body_scroll)
                                            .mode(ScrollbarMode::Always),
                                    ),
                            ),
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
