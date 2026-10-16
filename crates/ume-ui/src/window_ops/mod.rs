//! Sakura's window functions (ウィンドウ系) on the editor windows of this program: arranging
//! them, going to the next one, the list of them, closing the others.
//!
//! This program opens one window per file, like Sakura without the tab mode; tabs and groups are
//! not decided yet, so every window is in one group.
//!
//! * [`Screen`] is what the arranging needs from the OS: the work area, the place of a window, and
//!   raising it. [`Os`] is the real one (GPUI and `ume_platform::window`); the tests use a fake
//!   that remembers the rectangles.
//! * The rest are the flows of the functions. The arithmetic (where a window goes) is in
//!   `ume_platform::window::arrange`.
//! * [`list`] is the window list: the popup of `F_WINLIST` and the dialog of `F_DLGWINLIST`.

pub(crate) mod list;

use gpui::{AnyWindowHandle, App, AsyncWindowContext, Entity, Window};
use ume_commands::command::CommandError;
use ume_platform::window::arrange::{
    Direction, Rect, cascade, cascade_order, maximize_h, maximize_v, neighbor, tile_h, tile_order,
    tile_v,
};
use ume_platform::window::{self, Error, RawWindowHandle};

use crate::file_ops::close::confirm_and_close_window;
use crate::file_ops::windows::{activate, editors};
use crate::view::EditorView;

/// What the OS does with a window. All the windows are told apart by their GPUI handle.
pub(crate) trait Screen {
    /// The usable area of the screen of the window.
    fn work_area(&mut self, window: AnyWindowHandle) -> Result<Rect, Error>;
    /// The rectangle of the window with its frame.
    fn frame(&mut self, window: AnyWindowHandle) -> Result<Rect, Error>;
    fn is_minimized(&mut self, window: AnyWindowHandle) -> Result<bool, Error>;
    /// Restores the window and puts it at `rect`.
    fn place(&mut self, window: AnyWindowHandle, rect: Rect) -> Result<(), Error>;
    /// The shift of a cascade.
    fn cascade_step(&self) -> (i32, i32);
    /// Minimizes the window.
    fn minimize(&mut self, window: AnyWindowHandle);
    /// Brings the window to the front.
    fn raise(&mut self, window: AnyWindowHandle);
    fn is_topmost(&mut self, window: AnyWindowHandle) -> Result<bool, Error>;
    fn set_topmost(&mut self, window: AnyWindowHandle, topmost: bool) -> Result<(), Error>;
}

/// The OS handle of a GPUI window (`None` when it has none, as in the headless tests).
fn raw_handle(window: &Window) -> Option<RawWindowHandle> {
    use raw_window_handle::HasWindowHandle;
    HasWindowHandle::window_handle(window)
        .ok()
        .map(|handle| handle.as_raw())
}

/// The real screen. The window that runs the command is borrowed by GPUI while the command runs,
/// so it is reached through `window`; the others through their handles.
pub(crate) struct Os<'a> {
    window: &'a mut Window,
    cx: &'a mut App,
    current: AnyWindowHandle,
}

impl<'a> Os<'a> {
    pub(crate) fn new(window: &'a mut Window, cx: &'a mut App) -> Self {
        let current = window.window_handle();
        Self {
            window,
            cx,
            current,
        }
    }

    fn raw(&mut self, handle: AnyWindowHandle) -> Result<RawWindowHandle, Error> {
        let raw = if handle == self.current {
            raw_handle(self.window)
        } else {
            handle
                .update(self.cx, |_, window, _| raw_handle(window))
                .ok()
                .flatten()
        };
        raw.ok_or(Error::Unsupported("このウィンドウのハンドルが取れない"))
    }
}

impl Screen for Os<'_> {
    fn work_area(&mut self, window: AnyWindowHandle) -> Result<Rect, Error> {
        window::work_area(self.raw(window)?)
    }

    fn frame(&mut self, window: AnyWindowHandle) -> Result<Rect, Error> {
        window::frame(self.raw(window)?)
    }

    fn is_minimized(&mut self, window: AnyWindowHandle) -> Result<bool, Error> {
        window::is_minimized(self.raw(window)?)
    }

    fn place(&mut self, window: AnyWindowHandle, rect: Rect) -> Result<(), Error> {
        window::place(self.raw(window)?, rect)
    }

    fn cascade_step(&self) -> (i32, i32) {
        window::cascade_step()
    }

    fn minimize(&mut self, window: AnyWindowHandle) {
        if window == self.current {
            self.window.minimize_window();
        } else {
            window
                .update(self.cx, |_, other, _| other.minimize_window())
                .ok();
        }
    }

    fn raise(&mut self, window: AnyWindowHandle) {
        if window == self.current {
            self.window.activate_window();
        } else {
            activate(self.cx, window);
        }
    }

    fn is_topmost(&mut self, window: AnyWindowHandle) -> Result<bool, Error> {
        window::is_topmost(self.raw(window)?)
    }

    fn set_topmost(&mut self, window: AnyWindowHandle, topmost: bool) -> Result<(), Error> {
        window::set_topmost(self.raw(window)?, topmost)
    }
}

/// Runs `f` with the real screen, the editor windows (oldest first) and the window that runs the
/// command.
pub(crate) fn with_screen<R>(
    window: &mut Window,
    cx: &mut App,
    f: impl FnOnce(&mut dyn Screen, &[AnyWindowHandle], AnyWindowHandle) -> Result<R, Error>,
) -> Result<R, Error> {
    let all = all_handles(cx);
    let mut screen = Os::new(window, cx);
    let current = screen.current;
    f(&mut screen, &all, current)
}

/// The error of a command for a window the OS cannot place.
pub(crate) fn unavailable(error: Error) -> CommandError {
    match error {
        Error::Unsupported(what) | Error::Failed(what) => CommandError::Unavailable(what),
    }
}

/// The editor windows that can be arranged, in the order they were opened: not the minimized ones
/// (`IsIconic`). `current` is the position of the window that runs the command.
fn arrangeable(
    screen: &mut dyn Screen,
    all: &[AnyWindowHandle],
    current: AnyWindowHandle,
) -> Result<(Vec<AnyWindowHandle>, usize), Error> {
    let mut windows = Vec::new();
    let mut at = usize::MAX;
    for handle in all {
        if screen.is_minimized(*handle)? {
            continue;
        }
        if *handle == current {
            at = windows.len();
        }
        windows.push(*handle);
    }
    Ok((windows, at))
}

/// `F_CASCADE`: the windows overlap, each a little lower and to the right of the one before, the
/// current window last and on top (`Command_CASCADE`).
pub(crate) fn cascade_windows(
    screen: &mut dyn Screen,
    all: &[AnyWindowHandle],
    current: AnyWindowHandle,
) -> Result<(), Error> {
    let (windows, at) = arrangeable(screen, all, current)?;
    if windows.is_empty() {
        return Ok(());
    }
    let work = screen.work_area(current)?;
    let windows = cascade_order(windows, at);
    let (positions, (width, height)) = cascade(work, windows.len(), screen.cascade_step());
    for (handle, (x, y)) in windows.iter().zip(positions) {
        screen.place(*handle, Rect::at(x, y, width, height))?;
    }
    // Sakura stacks them with the first of the list at the bottom and the current window at the
    // top (`SetWindowPos` with the window before as `hWndInsertAfter`); raising them in the order
    // of the list gives the same.
    for handle in &windows {
        screen.raise(*handle);
    }
    Ok(())
}

/// `F_TILE_V` and `F_TILE_H`: the windows side by side or one above the other, the current
/// window first, which also gets the focus (`Command_TILE_V`, `Command_TILE_H`).
pub(crate) fn tile_windows(
    screen: &mut dyn Screen,
    all: &[AnyWindowHandle],
    current: AnyWindowHandle,
    vertical: bool,
) -> Result<(), Error> {
    let (windows, at) = arrangeable(screen, all, current)?;
    if windows.is_empty() {
        return Ok(());
    }
    let work = screen.work_area(current)?;
    let windows = tile_order(windows, at);
    let rects = if vertical {
        tile_v(work, windows.len())
    } else {
        tile_h(work, windows.len())
    };
    for (handle, rect) in windows.iter().zip(rects) {
        screen.place(*handle, rect)?;
    }
    screen.raise(windows[0]);
    Ok(())
}

/// `F_MINIMIZE_ALL`: minimizes every editor window (`Command_MINIMIZE_ALL`).
pub(crate) fn minimize_windows(screen: &mut dyn Screen, all: &[AnyWindowHandle]) {
    for handle in all {
        screen.minimize(*handle);
    }
}

/// `F_MAXIMIZE_V` and `F_MAXIMIZE_H`: the window as high (or as wide) as the work area, and
/// not larger in the other direction (`Command_MAXIMIZE_V`, `Command_MAXIMIZE_H`).
pub(crate) fn maximize_window(
    screen: &mut dyn Screen,
    current: AnyWindowHandle,
    vertical: bool,
) -> Result<(), Error> {
    let frame = screen.frame(current)?;
    let work = screen.work_area(current)?;
    let to = if vertical {
        maximize_v(frame, work)
    } else {
        maximize_h(frame, work)
    };
    screen.place(current, to)
}

/// `F_TOPMOST`: `how` is 0 to switch, 1 to keep the window on top, 2 to stop doing that
/// (`CEditWnd::WindowTopMost`). Any other number does nothing.
pub(crate) fn topmost(
    screen: &mut dyn Screen,
    current: AnyWindowHandle,
    how: i64,
) -> Result<(), Error> {
    let on = match how {
        0 => !screen.is_topmost(current)?,
        1 => true,
        2 => false,
        _ => return Ok(()),
    };
    screen.set_topmost(current, on)
}

/// The window that `F_NEXTWINDOW` / `F_PREVWINDOW` goes to from `current`, among the editor
/// windows in the order they were opened.
pub(crate) fn window_after(
    all: &[AnyWindowHandle],
    current: AnyWindowHandle,
    direction: Direction,
) -> Option<AnyWindowHandle> {
    let at = all.iter().position(|h| *h == current)?;
    Some(all[neighbor(all.len(), at, direction)])
}

/// All editor windows, oldest first.
pub(crate) fn all_handles(cx: &mut App) -> Vec<AnyWindowHandle> {
    editors(cx).into_iter().map(|(handle, _)| handle).collect()
}

/// `F_WIN_CLOSEALL` and `F_TAB_CLOSEOTHER`: asks about each window with unsaved changes and
/// closes it, one by one. A "cancel" stops it: the windows closed before stay closed
/// (`CAppNodeGroupHandle::RequestCloseEditor` sends the close request to one window after the
/// other and returns when one refuses). `keep` leaves the window alone (`F_TAB_CLOSEOTHER`).
///
/// The window that runs the flow goes last: it would take the flow with it when it closes. When
/// the last window is closed the program ends, as the editor processes of Sakura end.
/// Returns whether every window was closed.
pub(crate) async fn close_windows(
    cx: &mut AsyncWindowContext,
    current: AnyWindowHandle,
    keep_current: bool,
) -> bool {
    let editors = cx.update(|_, app| editors(app)).unwrap_or_default();
    let (mine, others): (Vec<_>, Vec<_>) = editors.into_iter().partition(|(h, _)| *h == current);
    let mut closing = others;
    if !keep_current {
        closing.extend(mine);
    }
    for (handle, view) in closing {
        let weak = view.downgrade();
        let Ok(mut window_cx) = handle.update(cx, |_, window, app| window.to_async(app)) else {
            continue;
        };
        if !confirm_and_close_window(&weak, &mut window_cx).await {
            return false;
        }
    }
    true
}

/// Runs Sakura's function `code` in the window `handle`, as if it was typed there. Used by the
/// buttons of the window list, from a task (not from inside an update of that window).
pub(crate) fn run_in_window(
    cx: &mut AsyncWindowContext,
    handle: AnyWindowHandle,
    view: &Entity<EditorView>,
    code: u16,
) -> bool {
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &[], window, cx))
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests;
