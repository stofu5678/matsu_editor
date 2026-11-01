//! 位置と大きさの設定: the dialog that the button of the 「ウィンドウ」 page opens
//! (`CDlgWinSize`, `sakura_core/dlg/CDlgWinSize.cpp`; the dialog is `IDD_WINPOSSIZE` in
//! `sakura_core/sakura_rc.rc`, the caller `CPropWin` in `sakura_core/prop/CPropComWin.cpp`).
//!
//! The dialog edits the seven values of `[Common]` that say where and how big a new window opens
//! (`bSaveWindowSize`, `nWinSizeType`, `nWinSizeCX`, `nWinSizeCY`, `nSaveWindowPos`, `nWinPosX`,
//! `nWinPosY`; [`crate::saved_state::window`] reads and keeps them). What it edits is a copy that
//! the page takes when the dialog is closed; the file is written when OK of the common settings
//! is pressed.
//!
//! * 起動時の位置: 指定しない, 継承する, 直接指定; the X and Y boxes can be used only with 直接指定.
//! * 大きさ: 指定しない, 継承する, 直接指定; the kind of window (普通, 最大化, (最小化)), the width
//!   and the height can be used only with 直接指定.
//! * The only way out is 閉じる (Enter, Esc and the close button of the window as well):
//!   `OnBnClicked` reads the controls (`GetData`) for `IDOK` and `IDCANCEL` alike, so there is no
//!   way to throw away what was typed. The ヘルプ button is not here.
//! * The numbers are what `GetDlgItemInt(.., TRUE)` makes of the text: a number, or 0. The spin
//!   buttons of Sakura limit the position to ±30000 and the size to 0..30000; the text is held to
//!   the same range when the dialog is closed.
//!
//! The dialog is a window of its own that is not modal (GPUI has no modal child window); one is
//! open at a time.

use crate::input_select::input_field;
use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, ClickEvent, Context, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, Role,
    StatefulInteractiveElement, Styled, Subscription, Window, WindowBounds, WindowOptions, div, px,
    rgb, size,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::radio::Radio;
use gpui_component::{Disableable as _, Root, Selectable as _};

use crate::common_settings::parse_signed_number;

const TEXT: u32 = 0x222222;
const BORDER: u32 = 0xaaaaaa;

/// `WINSIZEMODE_DEF`, `WINSIZEMODE_SAVE` and `WINSIZEMODE_SET`.
pub(crate) const MODE_DEFAULT: i32 = 0;
pub(crate) const MODE_SAVE: i32 = 1;
pub(crate) const MODE_SET: i32 = 2;

/// `SIZE_RESTORED`, `SIZE_MINIMIZED` and `SIZE_MAXIMIZED` of `WM_SIZE`: what `nWinSizeType` holds.
pub(crate) const SIZE_RESTORED: i32 = 0;
pub(crate) const SIZE_MINIMIZED: i32 = 1;
pub(crate) const SIZE_MAXIMIZED: i32 = 2;

/// The range of the spin buttons of the position (`IDC_SPIN_WX`, `IDC_SPIN_WY`) and of the size
/// (`IDC_SPIN_SX`, `IDC_SPIN_SY`).
const POSITION_RANGE: (i32, i32) = (-30000, 30000);
const SIZE_RANGE: (i32, i32) = (0, 30000);

/// The entries of the box 起動時の大きさ (`STR_DLGWINSZ_NORMAL`, `_MAXIMIZE`, `_MINIMIZE`), with
/// the `nWinSizeType` each makes (`GetData`).
const KINDS: [(&str, i32); 3] = [
    ("普通", SIZE_RESTORED),
    ("最大化", SIZE_MAXIMIZED),
    ("(最小化)", SIZE_MINIMIZED),
];

/// What the dialog edits: the seven values of the settings file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WinSize {
    /// `bSaveWindowSize`: a `MODE_*`.
    pub size_mode: i32,
    /// `nWinSizeType`: a `SIZE_*`.
    pub size_type: i32,
    /// `nWinSizeCX`.
    pub width: i32,
    /// `nWinSizeCY`.
    pub height: i32,
    /// `nSaveWindowPos`: a `MODE_*`.
    pub pos_mode: i32,
    /// `nWinPosX`.
    pub x: i32,
    /// `nWinPosY`.
    pub y: i32,
}

/// What the dialog does with the values when it is closed.
pub(crate) type Done = Box<dyn FnOnce(&mut App, WinSize)>;

/// A box of the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Field {
    X,
    Y,
    Width,
    Height,
}

pub(crate) struct WinSizeDialog {
    start: WinSize,
    size_mode: i32,
    pos_mode: i32,
    size_type: i32,
    x: Entity<InputState>,
    y: Entity<InputState>,
    width: Entity<InputState>,
    height: Entity<InputState>,
    on_done: Option<Done>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl WinSizeDialog {
    fn new(start: WinSize, on_done: Done, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut subscriptions = Vec::new();
        let mut field = |value: i32, window: &mut Window, cx: &mut Context<Self>| {
            let input = cx.new(|cx| input_field(window, cx));
            input.update(cx, |state, cx| {
                state.set_value(value.to_string(), window, cx);
            });
            subscriptions.push(cx.subscribe_in(
                &input,
                window,
                |this: &mut Self, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.close(window, cx);
                    }
                },
            ));
            input
        };
        let x = field(start.x, window, cx);
        let y = field(start.y, window, cx);
        let width = field(start.width, window, cx);
        let height = field(start.height, window, cx);
        Self {
            start,
            size_mode: start.size_mode,
            pos_mode: start.pos_mode,
            size_type: start.size_type,
            x,
            y,
            width,
            height,
            on_done: Some(on_done),
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    /// The radio buttons 大きさ (`SetData`: a value that is none of the three is 指定しない).
    pub(crate) fn set_size_mode(&mut self, mode: i32, cx: &mut Context<Self>) {
        self.size_mode = mode;
        cx.notify();
    }

    /// The radio buttons 起動時の位置.
    pub(crate) fn set_pos_mode(&mut self, mode: i32, cx: &mut Context<Self>) {
        self.pos_mode = mode;
        cx.notify();
    }

    /// The box of the kind of window (a `SIZE_*`).
    pub(crate) fn set_size_type(&mut self, kind: i32, cx: &mut Context<Self>) {
        self.size_type = kind;
        cx.notify();
    }

    fn input(&self, field: Field) -> &Entity<InputState> {
        match field {
            Field::X => &self.x,
            Field::Y => &self.y,
            Field::Width => &self.width,
            Field::Height => &self.height,
        }
    }

    /// Types `text` in a box, as the user would.
    #[cfg(test)]
    pub(crate) fn set_text(
        &mut self,
        field: Field,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.input(field)
            .update(cx, |state, cx| state.set_value(text.to_owned(), window, cx));
    }

    /// The numbers and choices as `GetData` reads them.
    pub(crate) fn values(&self, cx: &App) -> WinSize {
        let number = |field: Field, range: (i32, i32), start: i32| {
            let text = self.input(field).read(cx).value().to_string();
            // A box that still shows the number it started with keeps exactly that number: the
            // width of a window that was never stored is `CW_USEDEFAULT`, far outside the range
            // of the spin button, and pressing 閉じる must not change it.
            if text == start.to_string() {
                start
            } else {
                parse_signed_number(&text).clamp(range.0, range.1)
            }
        };
        // A box that cannot be used (the mode is not 直接指定) is read as well (`GetData`).
        WinSize {
            size_mode: self.size_mode,
            size_type: self.size_type,
            width: number(Field::Width, SIZE_RANGE, self.start.width),
            height: number(Field::Height, SIZE_RANGE, self.start.height),
            pos_mode: self.pos_mode,
            x: number(Field::X, POSITION_RANGE, self.start.x),
            y: number(Field::Y, POSITION_RANGE, self.start.y),
        }
    }

    /// 閉じる: gives the values to whoever asked and closes the window.
    pub(crate) fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish(cx);
        window.remove_window();
    }

    fn finish(&mut self, cx: &mut Context<Self>) {
        let values = self.values(cx);
        if let Some(done) = self.on_done.take() {
            done(cx, values);
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // Esc is `IDCANCEL`, which also reads the boxes.
        match event.keystroke.key.as_str() {
            "escape" | "enter" => self.close(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Focusable for WinSizeDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for WinSizeDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window)
            || [&self.x, &self.y, &self.width, &self.height]
                .iter()
                .any(|input| input.read(cx).focus_handle(cx).is_focused(window));
        if !focused {
            window.focus(&self.focus, cx);
        }
        let ui_font = gpui_component::Theme::global(cx).font_family.clone();
        let pos_set = self.pos_mode == MODE_SET;
        let size_set = self.size_mode == MODE_SET;

        let modes = |group: &'static str,
                     labels: [&'static str; 3],
                     current: i32,
                     cx: &mut Context<Self>| {
            let radios = labels
                .into_iter()
                .zip([MODE_DEFAULT, MODE_SAVE, MODE_SET])
                .map(|(label, mode)| {
                    Radio::new((group, mode as usize))
                        .label(label)
                        .checked(current == mode)
                        .on_click(cx.listener(move |this, _: &bool, _, cx| {
                            if group == "win-pos" {
                                this.set_pos_mode(mode, cx);
                            } else {
                                this.set_size_mode(mode, cx);
                            }
                        }))
                });
            div()
                .id(group)
                .role(Role::RadioGroup)
                .flex()
                .flex_col()
                .gap_1()
                .children(radios)
        };
        let number = |label: &'static str, input: &Entity<InputState>, usable: bool| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().w(px(56.)).child(label))
                .child(
                    div()
                        .w(px(110.))
                        .child(Input::new(input).aria_label(label).disabled(!usable)),
                )
        };
        let kinds = KINDS
            .into_iter()
            .map(|(label, kind)| {
                gpui_component::button::Button::new(("win-kind", kind as usize))
                    .label(label)
                    .selected(self.size_type == kind)
                    .role(Role::RadioButton)
                    .toggled(self.size_type == kind)
                    .disabled(!size_set)
                    .on_click(cx.listener(move |this, _, _, cx| this.set_size_type(kind, cx)))
            })
            .collect::<Vec<_>>();
        let group = |title: &'static str| {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_2()
                .border_1()
                .border_color(rgb(BORDER))
                .child(div().child(title))
        };

        crate::a11y::dialog(div().id("win-size-dialog"), "ウィンドウの位置と大きさ")
            .key_context("WinSizeDialog")
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
                group("起動時の位置").child(
                    div()
                        .flex()
                        .gap_4()
                        .child(modes(
                            "win-pos",
                            ["指定しない(&D)", "継承する(&I)", "直接指定(&S)"],
                            self.pos_mode,
                            cx,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(number("X座標", &self.x, pos_set))
                                .child(number("Y座標", &self.y, pos_set)),
                        ),
                ),
            )
            .child(
                group("大きさ").child(
                    div()
                        .flex()
                        .gap_4()
                        .child(modes(
                            "win-size",
                            ["指定しない(&E)", "継承する(&N)", "直接指定(&T)"],
                            self.size_mode,
                            cx,
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .id("win-kinds")
                                        .role(Role::RadioGroup)
                                        .aria_label("起動時の大きさ")
                                        .flex()
                                        .gap_1()
                                        .children(kinds),
                                )
                                .child(number("幅", &self.width, size_set))
                                .child(number("高さ", &self.height, size_set)),
                        ),
                ),
            )
            .child(
                div().flex().justify_end().child(
                    crate::a11y::button(div().id("win-size-close"), "閉じる(&C)")
                        .px_4()
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(0xf4f4f4))
                        .cursor_pointer()
                        .child("閉じる(C)")
                        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                            this.close(window, cx);
                        })),
                ),
            )
    }
}

/// The dialog that is open, if any.
struct OpenDialog {
    window: AnyWindowHandle,
    /// For the tests, which drive the dialog.
    #[cfg_attr(not(test), allow(dead_code))]
    dialog: gpui::WeakEntity<WinSizeDialog>,
}

impl gpui::Global for OpenDialog {}

/// The dialog that is open and its window.
#[cfg(test)]
pub(crate) fn current(cx: &App) -> Option<(Entity<WinSizeDialog>, AnyWindowHandle)> {
    let open = cx.try_global::<OpenDialog>()?;
    if !cx.windows().contains(&open.window) {
        return None;
    }
    Some((open.dialog.upgrade()?, open.window))
}

/// Shows the dialog starting at `start`; `on_done` gets the values when the dialog is closed
/// (however it is closed). One that is open comes to the front instead (and `on_done` is dropped).
pub(crate) fn open(cx: &mut App, start: WinSize, on_done: Done) -> Option<Entity<WinSizeDialog>> {
    if let Some(open) = cx.try_global::<OpenDialog>()
        && cx.windows().contains(&open.window)
    {
        let window = open.window;
        window
            .update(cx, |_, window, _| window.activate_window())
            .ok();
        return None;
    }
    let bounds = Bounds::centered(None, size(px(420.), px(330.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        icon: crate::app_icon::window_icon(),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("ウィンドウの位置と大きさ".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<WinSizeDialog>>>> = Default::default();
    let made = slot.clone();
    let opened = cx.open_window(options, move |window, cx| {
        let dialog = cx.new(|cx| WinSizeDialog::new(start, on_done, window, cx));
        *made.borrow_mut() = Some(dialog.clone());
        // The close button of the window is 閉じる as well.
        let closing = dialog.downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            if let Some(dialog) = closing.upgrade() {
                dialog.update(cx, |dialog, cx| dialog.finish(cx));
            }
            true
        });
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

#[cfg(test)]
mod tests;
