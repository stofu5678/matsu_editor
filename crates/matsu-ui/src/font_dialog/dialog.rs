//! MATSU Editor's own font dialog, for macOS and Linux (Windows has `ChooseFont`): the family, the
//! style and the size, each in a list, and a sample of the text in the chosen font. It returns a
//! [`FontSetting`] (see [`super::model`] for what the fields hold).
//!
//! The dialog is a window of its own that is not modal: the settings window it was opened from
//! can still be used while it is open (GPUI has no modal child window). One dialog is open at a
//! time; asking for a second brings the first to the front.

use crate::input_select::input_field;
use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, ClickEvent, Context, Entity, FocusHandle,
    Focusable, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render,
    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
    WindowBounds, WindowOptions, div, prelude::FluentBuilder as _, px, rgb, size,
};
use gpui_component::Root;
use gpui_component::input::{Input, InputEvent, InputState};
use matsu_config::settings::FontSetting;

use super::model::{self, Choice, SIZES, Style};
use crate::settings::view_settings::points_to_px;

const TEXT: u32 = 0x222222;
const BORDER: u32 = 0xaaaaaa;
const SELECTED_BG: u32 = 0x2f6fd0;

/// The text of the sample: Latin letters and digits, then Japanese (hiragana, katakana, kanji).
const SAMPLE: &str = "AaBbYyZz 0123 あいうえお アイウエオ 漢字";

/// What the dialog does with the font the user chose.
pub(crate) type Done = Box<dyn FnOnce(&mut App, FontSetting)>;

pub(crate) struct FontDialog {
    /// The font the dialog started with: the fields it cannot change are taken from it.
    base: FontSetting,
    families: Vec<String>,
    choice: Choice,
    size_input: Entity<InputState>,
    family_scroll: ScrollHandle,
    on_done: Option<Done>,
    focus: FocusHandle,
    _subscription: Subscription,
}

impl FontDialog {
    pub(crate) fn new(
        base: FontSetting,
        families: Vec<String>,
        on_done: Done,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let choice = Choice::of(&base);
        let size_input = cx.new(|cx| input_field(window, cx));
        let text = model::size_text(choice.size);
        size_input.update(cx, |state, cx| state.set_value(text, window, cx));
        let subscription = cx.subscribe_in(
            &size_input,
            window,
            |this: &mut Self, input, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    let text = input.read(cx).value().to_string();
                    this.typed_size(&text, cx);
                }
                InputEvent::PressEnter { .. } => this.ok(window, cx),
                _ => {}
            },
        );
        let family_scroll = ScrollHandle::new();
        if let Some(at) = families.iter().position(|f| *f == choice.family) {
            family_scroll.scroll_to_item(at);
        }
        Self {
            base,
            families,
            choice,
            size_input,
            family_scroll,
            on_done: Some(on_done),
            focus: cx.focus_handle(),
            _subscription: subscription,
        }
    }

    /// What the user has chosen so far.
    #[cfg(test)]
    pub(crate) fn choice(&self) -> &Choice {
        &self.choice
    }

    /// The family list.
    #[cfg(test)]
    pub(crate) fn families(&self) -> &[String] {
        &self.families
    }

    pub(crate) fn select_family(&mut self, family: &str, cx: &mut Context<Self>) {
        self.choice.family = family.to_owned();
        cx.notify();
    }

    pub(crate) fn select_style(&mut self, style: Style, cx: &mut Context<Self>) {
        self.choice.style = style;
        cx.notify();
    }

    /// A size of the list: the size box shows it.
    pub(crate) fn select_size(&mut self, tenths: i32, window: &mut Window, cx: &mut Context<Self>) {
        self.choice.size = tenths;
        let text = model::size_text(tenths);
        self.size_input
            .update(cx, |state, cx| state.set_value(text, window, cx));
        cx.notify();
    }

    /// The text of the size box changed: a number is the size; text that is not one leaves the
    /// size as it was (OK uses the last good size).
    pub(crate) fn typed_size(&mut self, text: &str, cx: &mut Context<Self>) {
        if let Some(size) = model::parse_size(text) {
            self.choice.size = size;
            cx.notify();
        }
    }

    /// The font the choice makes.
    pub(crate) fn font(&self) -> FontSetting {
        self.choice.setting(&self.base)
    }

    /// OK: gives the font to whoever asked and closes the window.
    pub(crate) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let font = self.font();
        if let Some(done) = self.on_done.take() {
            done(cx, font);
        }
        window.remove_window();
    }

    /// キャンセル: nothing changes.
    pub(crate) fn cancel(&mut self, window: &mut Window) {
        self.on_done = None;
        window.remove_window();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => self.cancel(window),
            "enter" => self.ok(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Focusable for FontDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// One row of a list: selected rows are drawn like the selection of a list box.
fn row(
    id: impl Into<gpui::ElementId>,
    text: impl Into<SharedString>,
    selected: bool,
) -> gpui::Stateful<gpui::Div> {
    let text: SharedString = text.into();
    crate::a11y::list_row(div().id(id), &text, selected)
        .px_2()
        .h(px(22.))
        .flex()
        .items_center()
        .cursor_pointer()
        .when(selected, |row| {
            row.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
        })
        .child(text)
}

impl Render for FontDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.focus.is_focused(window)
            && !self.size_input.read(cx).focus_handle(cx).is_focused(window)
        {
            window.focus(&self.focus, cx);
        }
        let ui_font = gpui_component::Theme::global(cx).font_family.clone();
        let list = |id: &'static str, name: &'static str| {
            div()
                .id(id)
                .role(gpui::Role::ListBox)
                .aria_label(name)
                .overflow_y_scroll()
                .bg(rgb(0xffffff))
                .border_1()
                .border_color(rgb(BORDER))
        };

        let families = self.families.iter().enumerate().map(|(i, family)| {
            let name = family.clone();
            row(("family", i), family.clone(), *family == self.choice.family).on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| this.select_family(&name, cx)),
            )
        });
        let styles = Style::ALL.into_iter().enumerate().map(|(i, style)| {
            row(("style", i), style.label(), style == self.choice.style).on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| this.select_style(style, cx)),
            )
        });
        let sizes = SIZES.into_iter().enumerate().map(|(i, tenths)| {
            row(
                ("size", i),
                model::size_text(tenths),
                tenths == self.choice.size,
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.select_size(tenths, window, cx);
            }))
        });

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
        let heading = |text: &'static str| div().h(px(20.)).child(text);

        let sample = crate::a11y::text(div().id("font-sample"), SAMPLE)
            .h(px(72.))
            .px_2()
            .flex()
            .items_center()
            .overflow_hidden()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(BORDER))
            .font_family(SharedString::from(self.choice.family.clone()))
            .text_size(px(points_to_px(self.choice.size)))
            .when(self.choice.style.bold(), |d| {
                d.font_weight(FontWeight::BOLD)
            })
            .when(self.choice.style.italic(), |d| d.italic())
            .child(SAMPLE);

        crate::a11y::dialog(div().id("font-dialog"), "フォント")
            .key_context("FontDialog")
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
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.))
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_h(px(0.))
                            .child(heading("フォント名:"))
                            .child(
                                list("font-families", "フォント名")
                                    .flex_1()
                                    .track_scroll(&self.family_scroll)
                                    .children(families),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .w(px(110.))
                            .child(heading("スタイル:"))
                            .child(list("font-styles", "スタイル").children(styles)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .w(px(90.))
                            .min_h(px(0.))
                            .gap_1()
                            .child(heading("サイズ:"))
                            .child(Input::new(&self.size_input).aria_label("サイズ"))
                            .child(list("font-sizes", "サイズ").flex_1().children(sizes)),
                    ),
            )
            .child(heading("サンプル"))
            .child(sample)
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(button("font-ok", "OK").on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.ok(window, cx)),
                    ))
                    .child(button("font-cancel", "キャンセル").on_click(
                        cx.listener(|this, _: &ClickEvent, window, _| this.cancel(window)),
                    )),
            )
    }
}

/// The dialog that is open, if any.
struct OpenDialog {
    window: AnyWindowHandle,
    /// For the tests, which press the buttons of the dialog.
    #[cfg_attr(not(test), allow(dead_code))]
    dialog: gpui::WeakEntity<FontDialog>,
}

impl gpui::Global for OpenDialog {}

/// The dialog that is open and its window.
#[cfg(test)]
pub(crate) fn current(cx: &App) -> Option<(Entity<FontDialog>, AnyWindowHandle)> {
    let open = cx.try_global::<OpenDialog>()?;
    if !cx.windows().contains(&open.window) {
        return None;
    }
    Some((open.dialog.upgrade()?, open.window))
}

/// Shows the dialog starting at `base`; `on_done` gets the font when the user presses OK. One that
/// is open comes to the front instead (and `on_done` is dropped: it counts as キャンセル).
pub(crate) fn open(cx: &mut App, base: FontSetting, on_done: Done) -> Option<Entity<FontDialog>> {
    if let Some(open) = cx.try_global::<OpenDialog>()
        && cx.windows().contains(&open.window)
    {
        let window = open.window;
        window
            .update(cx, |_, window, _| window.activate_window())
            .ok();
        return None;
    }
    let installed = cx.text_system().all_font_names();
    let families = model::families(installed, &base.log_font.face_name);
    let bounds = Bounds::centered(None, size(px(560.), px(480.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        icon: crate::app_icon::window_icon(),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("フォント".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<FontDialog>>>> = Default::default();
    let made = slot.clone();
    let opened = cx.open_window(options, move |window, cx| {
        let dialog = cx.new(|cx| FontDialog::new(base, families, on_done, window, cx));
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
