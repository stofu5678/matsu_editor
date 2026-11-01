//! The small dialogs that the color page opens (`CDlgSameColor`, `CDlgKeywordSelect`): one window
//! each, shown over the sheet. Sakura runs them modally (`DoModal`); GPUI has no modal window, so
//! the sheet stays usable and a second request brings the open dialog to the front instead.

use std::rc::Rc;
use std::time::Duration;

use gpui::{
    AnyElement, AnyWindowHandle, App, AppContext as _, Bounds, Context, ElementId, Entity,
    FocusHandle, Global, IntoElement as _, Render, SharedString, Window, WindowBounds,
    WindowOptions, px, size,
};
use gpui_component::Root;
use gpui_component::button::Button;
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};

use crate::common_settings::CommonSettingsDialog;

/// How often the dialog looks whether the sheet it belongs to is still there.
const REFRESH: Duration = Duration::from_millis(500);

/// What a drop-down list does with the set that was chosen.
type Pick = Rc<dyn Fn(i32, &mut Window, &mut App)>;

/// The dialog that is open.
struct OpenChild(AnyWindowHandle);

impl Global for OpenChild {}

/// Shows a window of `width` x `height` pixels with the view `build` makes. A dialog of this kind that is
/// already open comes to the front and `None` is returned. The window is the one handed back.
pub(super) fn open<V: Render + 'static>(
    cx: &mut App,
    title: &'static str,
    width: f32,
    height: f32,
    build: impl FnOnce(&mut Window, &mut Context<V>) -> V + 'static,
) -> Option<Entity<V>> {
    if let Some(open) = cx.try_global::<OpenChild>().map(|open| open.0)
        && cx.windows().contains(&open)
    {
        open.update(cx, |_, window, _| window.activate_window())
            .ok();
        return None;
    }
    let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        icon: crate::app_icon::window_icon(),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some(title.into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let slot: Rc<std::cell::RefCell<Option<Entity<V>>>> = Default::default();
    let made = slot.clone();
    let mut build = Some(build);
    let handle = cx
        .open_window(options, move |window, cx| {
            let build = build.take().expect("the window is made once");
            let view = cx.new(|cx| build(window, cx));
            *made.borrow_mut() = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx))
        })
        .ok()?;
    cx.set_global(OpenChild(handle.into()));
    slot.borrow_mut().take()
}

/// The window of the dialog that is open, for the tests.
#[cfg(test)]
pub(in crate::type_settings) fn open_window(cx: &App) -> Option<AnyWindowHandle> {
    cx.try_global::<OpenChild>().map(|open| open.0)
}

/// Closes the dialog when the sheet it belongs to is gone.
pub(super) fn close_with_sheet<V: 'static>(
    sheet: gpui::WeakEntity<CommonSettingsDialog>,
    window: &mut Window,
    cx: &mut Context<V>,
) {
    cx.spawn_in(window, async move |this, cx| {
        loop {
            cx.background_executor().timer(REFRESH).await;
            if sheet.upgrade().is_some() {
                continue;
            }
            this.update_in(cx, |_, window, _| window.remove_window())
                .ok();
            break;
        }
    })
    .detach();
}

/// Focuses `handle` once, when the window first draws (the keys Enter and Esc are heard there).
pub(super) fn focus_once(
    focused: &mut bool,
    handle: &FocusHandle,
    window: &mut Window,
    cx: &mut App,
) {
    if !*focused {
        *focused = true;
        window.focus(handle, cx);
    }
}

/// A drop-down list of the keyword sets: the button shows the chosen set, the menu has a blank
/// line (no set, `-1`) and a line for each set (`Combo_AddString`, the first line is blank).
/// `pick` gets the number of the set that was chosen.
pub(super) fn set_dropdown(
    id: impl Into<ElementId>,
    chosen: i32,
    names: Vec<String>,
    pick: impl Fn(i32, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let shown = usize::try_from(chosen)
        .ok()
        .and_then(|at| names.get(at).cloned())
        .unwrap_or_default();
    // A blank label has no width: the button would collapse.
    let label: SharedString = if shown.is_empty() {
        " ".into()
    } else {
        shown.into()
    };
    let pick: Pick = Rc::new(pick);
    Button::new(id)
        .label(label)
        .dropdown_menu(move |menu, _, _| {
            let blank = {
                let pick = pick.clone();
                PopupMenuItem::new(" ").on_click(move |_, window, cx| pick(-1, window, cx))
            };
            names
                .iter()
                .enumerate()
                .fold(menu.item(blank), |menu, (at, name)| {
                    let pick = pick.clone();
                    let number = i32::try_from(at).unwrap_or(-1);
                    menu.item(
                        PopupMenuItem::new(name.clone())
                            .on_click(move |_, window, cx| pick(number, window, cx)),
                    )
                })
        })
        .into_any_element()
}
