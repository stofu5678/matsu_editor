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
//!   way to throw away what was typed.
//! * The button ヘルプ and F1 open the page of the 「ウィンドウ」 page of the common settings, which
//!   describes this dialog (`CDlgWinSize`: `MyWinHelp( .., HLP000286 )`, a topic and not a function).
//! * The numbers are what `GetDlgItemInt(.., TRUE)` makes of the text: a number, or 0. The spin
//!   buttons of Sakura limit the position to ±30000 and the size to 0..30000; the text is held to
//!   the same range when the dialog is closed.
//!
//! The dialog is a window of its own that is not modal (GPUI has no modal child window); one is
//! open at a time.

use std::rc::Rc;

mod kind_combo;

use crate::access_keys::{Entry, Found, OnAccessKey as _, pick};
use crate::drop_down::DropDown;
use crate::input_select::{focus_selecting, input_field};
use crate::radio_group::{Group, Ring};
use crate::support::help::OnHelpKey as _;
use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, ClickEvent, Context, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Render, Role,
    StatefulInteractiveElement, Styled, Subscription, Window, WindowBounds, WindowOptions, div, px,
    rgb, size,
};
use gpui_component::Root;
use gpui_component::input::{Input, InputEvent, InputState};

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

/// The labels that show an access key (`IDD_WINPOSSIZE`: `sakura_core/sakura_rc.rc`), with the
/// key as the settings dialogs write it (`(&D)`).
const POS_LABELS: [&str; 3] = ["指定しない(&D)", "継承する(&I)", "直接指定(&S)"];
const SIZE_LABELS: [&str; 3] = ["指定しない(&E)", "継承する(&N)", "直接指定(&T)"];
const X_LABEL: &str = "X座標(&X)";
const Y_LABEL: &str = "Y座標(&Y)";
const WIDTH_LABEL: &str = "幅(&W)";
const HEIGHT_LABEL: &str = "高さ(&G)";
const CLOSE_LABEL: &str = "閉じる(&C)";
const HELP_LABEL: &str = "ヘルプ(&H)";
/// The page that describes this dialog.
const HELP_PAGE: &str = "common-settings/page-window.md";

/// What an access key does in the dialog.
#[derive(Clone, Copy)]
enum Use {
    PosMode(i32),
    SizeMode(i32),
    /// The focus goes to the box.
    Field(Field),
    Close,
    /// ヘルプ(H): the page of the dialog.
    Help,
}

/// `IDC_BUTTON_HELP`. The dialog stays.
fn open_help(cx: &mut App) {
    crate::support::help::open_page(cx, HELP_PAGE, None);
}

/// The key of a label that marks it with `&` (`直接指定(&S)` is `s`).
fn mark_of(label: &str) -> Option<char> {
    let (_, key) = crate::common_settings::model::parse_label(label);
    key.map(|key| key.to_ascii_lowercase())
}

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
    /// The focus of the radio buttons of 起動時の位置 and 大きさ (two groups in `IDD_WINPOSSIZE`:
    /// the first of each has `WS_GROUP`).
    pos_ring: Ring,
    size_ring: Ring,
    /// The combo box of the kind of window: its focus, whether its list is open and the row the
    /// keys are on in it (`kind_combo`, `crate::drop_down`). `size_type` is the choice.
    kind: DropDown,
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
            pos_ring: Ring::default(),
            size_ring: Ring::default(),
            kind: DropDown::new(cx),
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    /// The radio buttons 大きさ (`SetData`: a value that is none of the three is 指定しない).
    pub(crate) fn set_size_mode(&mut self, mode: i32, cx: &mut Context<Self>) {
        self.size_mode = mode;
        if !self.kind_usable() {
            self.kind.close();
        }
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
            // what the box shows, as `GetData` reads the entry (`kind_row`)
            size_type: KINDS[self.kind_row()].1,
            width: number(Field::Width, SIZE_RANGE, self.start.width),
            height: number(Field::Height, SIZE_RANGE, self.start.height),
            pos_mode: self.pos_mode,
            x: number(Field::X, POSITION_RANGE, self.start.x),
            y: number(Field::Y, POSITION_RANGE, self.start.y),
        }
    }

    /// Alt with the access key of a label (`IsDialogMessage`): the radio buttons are chosen, the
    /// label of a box moves the focus to it (only while the box can be used), 閉じる closes.
    /// Returns whether the letter was one of the dialog's.
    pub(crate) fn use_access_key(
        &mut self,
        letter: char,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let (pos_set, size_set) = (self.pos_mode == MODE_SET, self.size_mode == MODE_SET);
        let mut entries = Vec::new();
        for (label, mode) in POS_LABELS.into_iter().zip(MODES) {
            entries.extend(mark_of(label).map(|key| Entry::keyed(key, true, Use::PosMode(mode))));
        }
        for (label, mode) in SIZE_LABELS.into_iter().zip(MODES) {
            entries.extend(mark_of(label).map(|key| Entry::keyed(key, true, Use::SizeMode(mode))));
        }
        for (label, field, usable) in [
            (X_LABEL, Field::X, pos_set),
            (Y_LABEL, Field::Y, pos_set),
            (WIDTH_LABEL, Field::Width, size_set),
            (HEIGHT_LABEL, Field::Height, size_set),
        ] {
            entries.extend(mark_of(label).map(|key| Entry::keyed(key, usable, Use::Field(field))));
        }
        entries.extend(mark_of(CLOSE_LABEL).map(|key| Entry::keyed(key, true, Use::Close)));
        entries.extend(mark_of(HELP_LABEL).map(|key| Entry::keyed(key, true, Use::Help)));
        match pick(letter, &entries) {
            Found::Nothing => return false,
            Found::Several => {}
            Found::One(Use::PosMode(mode)) => {
                self.set_pos_mode(mode, cx);
                if let Some(index) = mode_index(mode) {
                    self.pos_ring.focus(index, window, cx);
                }
            }
            Found::One(Use::SizeMode(mode)) => {
                self.set_size_mode(mode, cx);
                if let Some(index) = mode_index(mode) {
                    self.size_ring.focus(index, window, cx);
                }
            }
            Found::One(Use::Field(field)) => focus_selecting(self.input(field), window, cx),
            Found::One(Use::Close) => self.close(window, cx),
            Found::One(Use::Help) => open_help(cx),
        }
        true
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
        let this = cx.entity();
        // The focus is somewhere in the dialog: the panel, a box, or a button that Tab or an arrow
        // key moved it to (it used to go back to the panel at the next drawing, so the radio
        // buttons could not keep it)
        let focused = self.focus.contains_focused(window, cx)
            || [&self.x, &self.y, &self.width, &self.height]
                .iter()
                .any(|input| input.read(cx).focus_handle(cx).is_focused(window));
        if !focused {
            window.focus(&self.focus, cx);
        }
        let ui_font = gpui_component::Theme::global(cx).font_family.clone();
        let pos_set = self.pos_mode == MODE_SET;
        let size_set = self.size_mode == MODE_SET;
        // a list that is open while the focus has left the box is closed, as a combo box does
        // (the print settings' lists do the same): Shift+Tab and a click elsewhere come here
        self.kind.settle(window, cx);

        let modes = |group: &'static str,
                     labels: [&'static str; 3],
                     current: i32,
                     cx: &mut Context<Self>| {
            let ring = if group == "win-pos" {
                &self.pos_ring
            } else {
                &self.size_ring
            };
            let choose = Group::new(
                ring,
                labels.len(),
                mode_index(current),
                Rc::new(cx.listener(move |this, index: &usize, _, cx| {
                    let Some(mode) = mode_at(*index) else {
                        return;
                    };
                    if group == "win-pos" {
                        this.set_pos_mode(mode, cx);
                    } else {
                        this.set_size_mode(mode, cx);
                    }
                })),
            );
            let radios = labels.into_iter().zip(MODES).map(|(label, mode)| {
                let (shown, name) = radio_texts(label);
                let index = mode_index(mode).unwrap_or_default();
                choose.radio(index, (group, index), true, |radio| {
                    radio
                        .label(shown)
                        .accessibility_label(name)
                        .checked(current == mode)
                })
            });
            div()
                .id(group)
                .role(Role::RadioGroup)
                .flex()
                .flex_col()
                .gap_1()
                .children(radios)
        };
        let number = |label: &'static str,
                      input: &Entity<InputState>,
                      range: (i32, i32),
                      usable: bool,
                      window: &mut Window,
                      cx: &mut App| {
            let (shown, name) = radio_texts(label);
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().w(px(72.)).child(shown))
                // `IDC_SPIN_WX`, `_WY`, `_SX`, `_SY`
                .child(crate::spin::spin_box(
                    format!("win-size-spin-{label}"),
                    input,
                    &name,
                    range,
                    usable,
                    div()
                        .w(px(110.))
                        .child(Input::new(input).aria_label(name.clone()).disabled(!usable)),
                    window,
                    cx,
                ))
        };
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
            // F1: the page of the dialog
            .on_help_page_key(HELP_PAGE)
            .key_context("WinSizeDialog")
            .track_focus(&self.focus)
            .on_access_key(move |letter, window, cx| {
                this.update(cx, |dialog, cx| dialog.use_access_key(letter, window, cx))
            })
            .on_key_down(cx.listener(Self::on_key_down))
            .capture_key_down(cx.listener(Self::on_kind_capture_key))
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
                        .child(modes("win-pos", POS_LABELS, self.pos_mode, cx))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(number(
                                    X_LABEL,
                                    &self.x,
                                    POSITION_RANGE,
                                    pos_set,
                                    window,
                                    cx,
                                ))
                                .child(number(
                                    Y_LABEL,
                                    &self.y,
                                    POSITION_RANGE,
                                    pos_set,
                                    window,
                                    cx,
                                )),
                        ),
                ),
            )
            .child(
                group("大きさ").child(
                    div()
                        .flex()
                        .gap_4()
                        .child(modes("win-size", SIZE_LABELS, self.size_mode, cx))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(self.kind_combo(cx))
                                .child(number(
                                    WIDTH_LABEL,
                                    &self.width,
                                    SIZE_RANGE,
                                    size_set,
                                    window,
                                    cx,
                                ))
                                .child(number(
                                    HEIGHT_LABEL,
                                    &self.height,
                                    SIZE_RANGE,
                                    size_set,
                                    window,
                                    cx,
                                )),
                        ),
                ),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        crate::a11y::button(
                            div()
                                .id("win-size-close")
                                .debug_selector(|| "win-size-close".to_owned()),
                            CLOSE_LABEL,
                        )
                        .px_4()
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(0xf4f4f4))
                        .cursor_pointer()
                        .child("閉じる(C)")
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, window, cx| {
                                this.close(window, cx);
                            },
                        )),
                    )
                    .child(
                        crate::a11y::button(div().id("win-size-help"), HELP_LABEL)
                            .px_4()
                            .h(px(28.))
                            .flex()
                            .items_center()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(0xf4f4f4))
                            .cursor_pointer()
                            .child("ヘルプ(H)")
                            .on_click(|_: &ClickEvent, _, cx| open_help(cx)),
                    ),
            )
    }
}

/// The modes in the order of the buttons of both groups (the order of `POS_LABELS` and
/// `SIZE_LABELS`): the one place that maps a mode to a button and back, for the drawing, the
/// `select` closure and the access keys.
const MODES: [i32; 3] = [MODE_DEFAULT, MODE_SAVE, MODE_SET];

fn mode_index(mode: i32) -> Option<usize> {
    MODES.iter().position(|m| *m == mode)
}

fn mode_at(index: usize) -> Option<i32> {
    MODES.get(index).copied()
}

/// What a radio button shows and what a screen reader says for a label with an access key mark,
/// as the other controls of the settings do (`(&D)` is drawn `(D)` and spoken without the key).
fn radio_texts(label: &str) -> (String, String) {
    (
        crate::common_settings::model::parse_label(label).0,
        crate::a11y::plain_name(label),
    )
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

/// Closes the dialog that is open and returns what it holds, without telling whoever opened it
/// (the sheet that did is closing and takes the values itself).
pub(crate) fn take_open(cx: &mut App) -> Option<WinSize> {
    let open = cx.try_global::<OpenDialog>()?;
    let (window, dialog) = (open.window, open.dialog.upgrade()?);
    if !cx.windows().contains(&window) {
        return None;
    }
    let values = dialog.update(cx, |dialog, cx| {
        dialog.on_done = None;
        dialog.values(cx)
    });
    window
        .update(cx, |_, window, _| window.remove_window())
        .ok();
    Some(values)
}

/// Closes the dialog that is open and drops what it holds (the sheet that opened it was
/// cancelled).
pub(crate) fn discard_open(cx: &mut App) {
    take_open(cx);
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
mod kind_tests;
#[cfg(test)]
mod tests;
