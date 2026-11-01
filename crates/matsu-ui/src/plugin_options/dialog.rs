//! The window of the plugin options: `IDD_PLUGIN_OPTION` (`sakura_core/sakura_rc.rc`),
//! `CDlgPluginOption`. A list of the options (項目 and 値), and under it the part that edits the
//! one that is selected: a text box for a string, a text box and a button for a folder, a number
//! box with a spin for an int, a check box for a bool and the choices for a selection.
//!
//! The window is not modal (GPUI has no modal child window): the settings window it was opened
//! from can still be used. One dialog is open at a time; asking for a second brings the first to
//! the front. OK writes the options file; キャンセル writes nothing.

use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, ClickEvent, Context, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, PathPromptOptions,
    Render, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
    WindowBounds, WindowOptions, div, prelude::FluentBuilder as _, px, rgb, size,
};
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{Root, h_flex, v_flex};
use matsu_config::plugins::PluginDef;
use matsu_config::plugins::def::OptionType;

use super::model::{self, Form};
use crate::a11y::Labelled as _;
use crate::input_select::input_field;

const TEXT: u32 = 0x222222;
const BORDER: u32 = 0xaaaaaa;
const SELECTED_BG: u32 = 0x2f6fd0;
const HEADING_BG: u32 = 0xe6e6e6;
const ROW_HEIGHT: f32 = 22.0;

/// `STR_DLGPLUGINOPT_OPTION`: the dialog of a plugin that has no options.
const NO_OPTIONS: &str = "指定できるオプションがありません";

pub(crate) struct OptionDialog {
    form: Form,
    /// The row that is selected (`m_Line`).
    selected: Option<usize>,
    /// The box for a string, a folder and an int; it shows the value of the selected row.
    input: Entity<InputState>,
    /// Why OK did not close the window.
    message: Option<String>,
    /// The scroll position of the list: the selected row is kept in view.
    scroll: ScrollHandle,
    focused_once: bool,
    focus: FocusHandle,
    _subscription: Subscription,
}

impl OptionDialog {
    pub(crate) fn new(form: Form, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| input_field(window, cx));
        let subscription = cx.subscribe_in(
            &input,
            window,
            |this: &mut Self, input, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    let text = input.read(cx).value().to_string();
                    this.typed(&text, cx);
                }
                InputEvent::PressEnter { .. } => this.ok(window, cx),
                _ => {}
            },
        );
        Self {
            form,
            selected: None,
            input,
            message: None,
            scroll: ScrollHandle::new(),
            focused_once: false,
            focus: cx.focus_handle().tab_stop(true),
            _subscription: subscription,
        }
    }

    #[cfg(test)]
    pub(crate) fn form(&self) -> &Form {
        &self.form
    }

    #[cfg(test)]
    pub(crate) fn input_text(&self, cx: &App) -> String {
        self.input.read(cx).value().to_string()
    }

    /// The keys of the dialog, for the tests.
    #[cfg(test)]
    pub(crate) fn key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.on_key_down(
            &KeyDownEvent {
                keystroke: gpui::Keystroke::parse(key).unwrap(),
                is_held: false,
                prefer_character_input: false,
            },
            window,
            cx,
        );
    }

    /// Whether the selected row is inside the list, for the tests.
    #[cfg(test)]
    pub(crate) fn selected_row_in_view(&self) -> Option<bool> {
        let bounds = self.scroll.bounds();
        let row = self.scroll.bounds_for_item(self.selected?)?;
        Some(row.top() >= bounds.top() && row.bottom() <= bounds.bottom())
    }

    #[cfg(test)]
    pub(crate) fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Selects a row (`ChangeListPosition`): the edit part shows its value.
    pub(crate) fn select(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.form.entries.get(index) else {
            return;
        };
        self.selected = Some(index);
        self.scroll.scroll_to_item(index);
        if matches!(
            entry.def.kind,
            OptionType::Str | OptionType::Dir | OptionType::Int
        ) {
            let text = entry.value.clone();
            self.input
                .update(cx, |state, cx| state.set_value(text, window, cx));
        }
        cx.notify();
    }

    /// The text of the box changed (`OnEnChange`): the row takes it.
    pub(crate) fn typed(&mut self, text: &str, cx: &mut Context<Self>) {
        let Some(index) = self.selected else {
            return;
        };
        let Some(entry) = self.form.entries.get(index) else {
            return;
        };
        if !matches!(
            entry.def.kind,
            OptionType::Str | OptionType::Dir | OptionType::Int
        ) {
            return;
        }
        self.form.set(index, text);
        cx.notify();
    }

    /// The check box (`IDC_CHECK_PLUGIN_OPTION`).
    pub(crate) fn set_bool(&mut self, on: bool, cx: &mut Context<Self>) {
        if let Some(index) = self.selected {
            self.form.set(index, if on { "1" } else { "0" });
            cx.notify();
        }
    }

    /// An item of the selection (`IDC_COMBO_PLUGIN_OPTION`).
    pub(crate) fn choose(&mut self, value: &str, cx: &mut Context<Self>) {
        if let Some(index) = self.selected {
            self.form.set(index, value);
            cx.notify();
        }
    }

    /// The spin (`IDC_SPIN_PLUGIN_OPTION`): the number moves by `step` and stops at the ends of
    /// `int`.
    pub(crate) fn spin(&mut self, step: i32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.selected else {
            return;
        };
        let number = model::atoi(&self.form.entries[index].value).saturating_add(step);
        let text = number.to_string();
        self.form.set(index, &text);
        self.input
            .update(cx, |state, cx| state.set_value(text, window, cx));
        cx.notify();
    }

    /// A folder was chosen (`SelectDirectory`): it goes in the box with the separator at its end.
    pub(crate) fn set_directory(&mut self, dir: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.selected else {
            return;
        };
        let text = model::with_trailing_separator(dir);
        self.form.set(index, &text);
        self.input
            .update(cx, |state, cx| state.set_value(text, window, cx));
        cx.notify();
    }

    /// 「…」 (`IDC_BUTTON_PLUGIN_OPTION_DIR`): asks for a folder.
    fn browse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.selected.and_then(|i| self.form.entries.get(i)) else {
            return;
        };
        // `STR_DLGPLUGINOPT_SELECT`
        let title = format!("{}の選択", entry.def.label);
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(SharedString::from(title)),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = chosen.await
                && let Some(path) = paths.into_iter().next()
            {
                this.update_in(cx, |this, window, cx| {
                    this.set_directory(&path.display().to_string(), window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// OK: writes the options file and closes the window. A file that cannot be written leaves
    /// the window open with the reason.
    pub(crate) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.form.entries.is_empty() {
            // `EnableWindow( IDOK, FALSE )`
            return;
        }
        match self.form.save() {
            Ok(()) => window.remove_window(),
            Err(error) => {
                self.message = Some(format!("オプションを保存できませんでした\n理由：{error}"));
                cx.notify();
            }
        }
    }

    /// キャンセル: nothing is written.
    pub(crate) fn cancel(&mut self, window: &mut Window) {
        window.remove_window();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => self.cancel(window),
            "enter" => self.ok(window, cx),
            // The keys of the list, the check box and the choices, while the focus is not in a
            // text box (the text box has the arrow keys for itself).
            "up" | "down" if self.focus.is_focused(window) => {
                let last = self.form.entries.len().saturating_sub(1);
                let next = match (self.selected, event.keystroke.key.as_str()) {
                    (None, _) => 0,
                    (Some(i), "up") => i.saturating_sub(1),
                    (Some(i), _) => (i + 1).min(last),
                };
                self.select(next, window, cx);
            }
            "left" | "right" if self.focus.is_focused(window) => {
                self.step_choice(if event.keystroke.key == "left" { -1 } else { 1 }, cx)
            }
            "space" if self.focus.is_focused(window) => self.toggle(cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    /// The previous or next item of a selection.
    fn step_choice(&mut self, step: isize, cx: &mut Context<Self>) {
        let Some(entry) = self
            .selected
            .and_then(|i| self.form.entries.get(i))
            .filter(|e| e.def.kind == OptionType::Sel)
        else {
            return;
        };
        let values: Vec<String> = entry.def.selects().into_iter().map(|(_, v)| v).collect();
        let at = values.iter().position(|v| *v == entry.value);
        let next = match at {
            Some(i) => i
                .saturating_add_signed(step)
                .min(values.len().saturating_sub(1)),
            None => 0,
        };
        if let Some(value) = values.get(next) {
            let value = value.clone();
            self.choose(&value, cx);
        }
    }

    /// The check box, from the keyboard.
    fn toggle(&mut self, cx: &mut Context<Self>) {
        let Some(entry) = self
            .selected
            .and_then(|i| self.form.entries.get(i))
            .filter(|e| e.def.kind == OptionType::Bool)
        else {
            return;
        };
        let on = entry.value != "1";
        self.set_bool(on, cx);
    }

    /// The part under the list that edits the selected row.
    fn editor(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(index) = self.selected else {
            return div().h(px(ROW_HEIGHT)).into_any_element();
        };
        let entry = &self.form.entries[index];
        // The glyph is what is drawn; the name is what a screen reader says.
        let button = |id: &'static str, text: &'static str, name: &'static str| {
            crate::a11y::button(div().id(id), name)
                .w(px(26.))
                .h(px(ROW_HEIGHT))
                .flex()
                .items_center()
                .justify_center()
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(0xf4f4f4))
                .cursor_pointer()
                .child(text)
        };
        match entry.def.kind {
            OptionType::Bool => {
                let on = entry.value != "0" && !entry.value.is_empty();
                Checkbox::new("plugin-option-bool")
                    .labelled(entry.def.label.clone())
                    .checked(on)
                    .on_click(cx.listener(|this, value: &bool, _, cx| this.set_bool(*value, cx)))
                    .into_any_element()
            }
            OptionType::Sel => {
                let items =
                    entry
                        .def
                        .selects()
                        .into_iter()
                        .enumerate()
                        .map(|(i, (shown, value))| {
                            let selected = value == entry.value;
                            crate::a11y::list_row(
                                div().id(("plugin-option-item", i)),
                                &shown,
                                selected,
                            )
                            .px_2()
                            .h(px(ROW_HEIGHT))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .when(selected, |row| {
                                row.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
                            })
                            .child(shown)
                            .on_click(cx.listener(
                                move |this, _: &ClickEvent, _, cx| {
                                    this.choose(&value, cx);
                                },
                            ))
                        });
                v_flex()
                    .id("plugin-option-items")
                    .max_h(px(110.))
                    .overflow_y_scroll()
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .children(items)
                    .into_any_element()
            }
            OptionType::Dir => h_flex()
                .gap_1()
                .child(
                    div()
                        .flex_1()
                        .child(Input::new(&self.input).aria_label(entry.def.label.clone())),
                )
                .child(
                    button("plugin-option-browse", "…", "フォルダーの選択").on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.browse(window, cx)),
                    ),
                )
                .into_any_element(),
            OptionType::Int => h_flex()
                .gap_1()
                .child(
                    div()
                        .w(px(150.))
                        .child(Input::new(&self.input).aria_label(entry.def.label.clone())),
                )
                .child(button("plugin-option-up", "▲", "増やす").on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.spin(1, window, cx)),
                ))
                .child(button("plugin-option-down", "▼", "減らす").on_click(
                    cx.listener(|this, _: &ClickEvent, window, cx| this.spin(-1, window, cx)),
                ))
                .into_any_element(),
            OptionType::Str => Input::new(&self.input)
                .aria_label(entry.def.label.clone())
                .into_any_element(),
        }
    }
}

impl Focusable for OptionDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for OptionDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The dialog takes the focus once, when it opens; after that the user moves it (Tab).
        if !self.focused_once {
            self.focused_once = true;
            window.focus(&self.focus, cx);
        }
        let ui_font = gpui_component::Theme::global(cx).font_family.clone();
        let button = |id: &'static str, text: &'static str| {
            crate::a11y::button(div().id(id), text)
                .px_4()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(BORDER))
                .bg(rgb(0xf4f4f4))
                .cursor_pointer()
                .child(text)
        };
        let has_options = !self.form.entries.is_empty();
        let rows = self.form.entries.iter().enumerate().map(|(i, entry)| {
            let selected = self.selected == Some(i);
            let shown = entry.shown();
            row_element(i, &entry.def.label, &shown, selected)
                .h(px(ROW_HEIGHT))
                .items_center()
                .cursor_pointer()
                .when(selected, |row| {
                    row.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
                })
                .child(
                    div()
                        .w(gpui::relative(0.4))
                        .px_1()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(entry.def.label.clone()),
                )
                .child(
                    div()
                        .w(gpui::relative(0.55))
                        .px_1()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(shown),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    // The list has the keys again after the focus was in an editor.
                    window.focus(&this.focus, cx);
                    this.select(i, window, cx);
                }))
        });
        let heading = h_flex()
            .bg(rgb(HEADING_BG))
            .border_b_1()
            .border_color(rgb(BORDER))
            .child(div().w(gpui::relative(0.4)).px_1().child("項目"))
            .child(div().w(gpui::relative(0.55)).px_1().child("値"));
        let list = v_flex()
            .id("plugin-option-list")
            .role(gpui::Role::ListBox)
            .aria_label("オプション")
            .flex_1()
            .min_h(px(0.))
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(BORDER))
            .child(heading)
            .child(
                v_flex()
                    .id("plugin-option-rows")
                    .track_scroll(&self.scroll)
                    .flex_1()
                    .overflow_y_scroll()
                    .children(rows),
            );
        let title = self.form.title();
        crate::a11y::dialog(div().id("plugin-option-dialog"), &title)
            .key_context("PluginOptionDialog")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .size_full()
            .p_3()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0xf0f0f0))
            .text_color(rgb(TEXT))
            .text_size(px(14.))
            .font_family(ui_font)
            // `IDC_STATIC_MSG`
            .child(div().child(if has_options {
                "プラグインのオプションを設定してください"
            } else {
                NO_OPTIONS
            }))
            .child(list)
            .when(self.selected.is_some(), |d| {
                d.child(
                    h_flex()
                        .gap_2()
                        .items_start()
                        .child(div().h(px(ROW_HEIGHT)).flex().items_center().child("値："))
                        .child(div().flex_1().child(self.editor(cx))),
                )
            })
            .when_some(self.message.clone(), |d, message| {
                d.child(
                    crate::a11y::text(div().id("plugin-option-message"), &message).child(message),
                )
            })
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        button("plugin-option-ok", "OK")
                            .when(!has_options, |b| b.opacity(0.5))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.ok(window, cx);
                            })),
                    )
                    .child(button("plugin-option-cancel", "キャンセル").on_click(
                        cx.listener(|this, _: &ClickEvent, window, _| this.cancel(window)),
                    )),
            )
    }
}

/// The name of a row for a screen reader: the label, then the value that is shown.
fn row_name(label: &str, shown: &str) -> String {
    format!("{label}、{shown}")
}

/// A row of the list with its role, name (label and value) and selected state.
pub(super) fn row_element(
    index: usize,
    label: &str,
    shown: &str,
    selected: bool,
) -> gpui::Stateful<gpui::Div> {
    crate::a11y::list_row(
        h_flex().id(("plugin-option-row", index)),
        &row_name(label, shown),
        selected,
    )
}

/// The dialog that is open, if any.
struct OpenDialog {
    window: AnyWindowHandle,
    #[cfg_attr(not(test), allow(dead_code))]
    dialog: gpui::WeakEntity<OptionDialog>,
}

impl gpui::Global for OpenDialog {}

/// The dialog that is open and its window.
#[cfg(test)]
pub(crate) fn current(cx: &App) -> Option<(Entity<OptionDialog>, AnyWindowHandle)> {
    let open = cx.try_global::<OpenDialog>()?;
    if !cx.windows().contains(&open.window) {
        return None;
    }
    Some((open.dialog.upgrade()?, open.window))
}

/// Shows the options of the plugin `def`, whose values are in `option_file`. One that is open
/// comes to the front instead.
pub(crate) fn open(
    cx: &mut App,
    def: &PluginDef,
    option_file: &std::path::Path,
) -> Option<Entity<OptionDialog>> {
    if let Some(open) = cx.try_global::<OpenDialog>()
        && cx.windows().contains(&open.window)
    {
        let window = open.window;
        window
            .update(cx, |_, window, _| window.activate_window())
            .ok();
        return None;
    }
    let form = Form::load(def, option_file);
    let title = form.title();
    let bounds = Bounds::centered(None, size(px(480.), px(420.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        icon: crate::app_icon::window_icon(),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some(title.into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<OptionDialog>>>> = Default::default();
    let made = slot.clone();
    let opened = cx.open_window(options, move |window, cx| {
        let dialog = cx.new(|cx| OptionDialog::new(form, window, cx));
        *made.borrow_mut() = Some(dialog.clone());
        cx.new(|cx| Root::new(dialog, window, cx))
    });
    let handle = opened.ok()?;
    let dialog = slot.borrow_mut().take()?;
    cx.set_global(OpenDialog {
        window: handle.into(),
        dialog: dialog.downgrade(),
    });
    Some(dialog)
}
