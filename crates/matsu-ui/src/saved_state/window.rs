//! The position and the size that a new window starts with.
//!
//! Sakura: `CommonSetting::m_sWindow` (`m_eSaveWindowSize`, `m_nWinSizeType`, `m_nWinSizeCX`,
//! `m_nWinSizeCY`, `m_eSaveWindowPos`, `m_nWinPosX`, `m_nWinPosY`; `sakura_core/env/CommonSetting.h`)
//! with the keys `bSaveWindowSize`, `nWinSizeType`, `nWinSizeCX`, `nWinSizeCY`, `nSaveWindowPos`,
//! `nWinPosX` and `nWinPosY` of `[Common]` (`ShareData_IO_Common`).
//!
//! * `CEditWnd::_GetWindowRectForInit` reads them for a new window: the size when the mode is not
//!   "指定なし", the position when its mode is not "指定なし"; the command line (`-SX=` ..) wins over
//!   them. `CEditWnd::Create` then shows the window maximized when the mode is not "指定なし" and
//!   the type is maximized.
//! * `CEditWnd::OnSize` keeps the size (and `OnMove` the position) in the shared data each time they
//!   change when the mode is "継承(保存)": a maximized window keeps only the type, a minimized one
//!   nothing.
//!
//! Sakura's defaults (`CShareData::InitShareData`): the size is inherited, the position is not.
//! The sizes are GPUI's logical pixels here, where Sakura's are device pixels. "直接指定" with a
//! minimized type opens the window normal, because GPUI cannot open a window minimized.

use std::time::Duration;

use gpui::{App, Global, WindowBounds, point, px, size};
use matsu_config::Profile;

const KEY_SIZE_MODE: &str = "bSaveWindowSize";
const KEY_SIZE_TYPE: &str = "nWinSizeType";
const KEY_SIZE_CX: &str = "nWinSizeCX";
const KEY_SIZE_CY: &str = "nWinSizeCY";
const KEY_POS_MODE: &str = "nSaveWindowPos";
const KEY_POS_X: &str = "nWinPosX";
const KEY_POS_Y: &str = "nWinPosY";

/// `CW_USEDEFAULT`: nothing is stored.
const USE_DEFAULT: i32 = i32::MIN;
/// `SIZE_RESTORED` and `SIZE_MAXIMIZED` of `WM_SIZE`.
const SIZE_RESTORED: i32 = 0;
const SIZE_MAXIMIZED: i32 = 2;

/// How the window gets its size or position (`EWinSizeMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SaveMode {
    /// `WINSIZEMODE_DEF` (0): 指定なし.
    Default,
    /// `WINSIZEMODE_SAVE` (1): 継承(保存).
    Save,
    /// `WINSIZEMODE_SET` (2): 直接指定.
    Set,
}

impl SaveMode {
    fn from_code(code: i32, default: Self) -> Self {
        match code {
            0 => Self::Default,
            1 => Self::Save,
            2 => Self::Set,
            _ => default,
        }
    }
}

/// What a new window takes, and what the windows keep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WindowState {
    pub size_mode: SaveMode,
    pub size_type: i32,
    pub width: i32,
    pub height: i32,
    pub pos_mode: SaveMode,
    pub x: i32,
    pub y: i32,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            size_mode: SaveMode::Save,
            size_type: SIZE_RESTORED,
            width: USE_DEFAULT,
            height: 0,
            pos_mode: SaveMode::Default,
            x: USE_DEFAULT,
            y: 0,
        }
    }
}

impl Global for WindowState {}

impl WindowState {
    pub(crate) fn from_profile(profile: &Profile) -> Self {
        let d = Self::default();
        let int = |key: &str, default: i32| profile.get_i32("Common", key).unwrap_or(default);
        Self {
            size_mode: SaveMode::from_code(int(KEY_SIZE_MODE, 1), d.size_mode),
            size_type: int(KEY_SIZE_TYPE, d.size_type),
            width: int(KEY_SIZE_CX, d.width),
            height: int(KEY_SIZE_CY, d.height),
            pos_mode: SaveMode::from_code(int(KEY_POS_MODE, 0), d.pos_mode),
            x: int(KEY_POS_X, d.x),
            y: int(KEY_POS_Y, d.y),
        }
    }

    /// The size a new window has, if the settings say one (`_GetWindowRectForInit`).
    pub(crate) fn size(&self) -> Option<gpui::Size<gpui::Pixels>> {
        (self.size_mode != SaveMode::Default && self.width > 0 && self.height > 0)
            .then(|| size(px(self.width as f32), px(self.height as f32)))
    }

    /// The top left corner a new window has, if the settings say one.
    pub(crate) fn origin(&self) -> Option<gpui::Point<gpui::Pixels>> {
        (self.pos_mode != SaveMode::Default && self.x != USE_DEFAULT)
            .then(|| point(px(self.x as f32), px(self.y as f32)))
    }

    /// Whether a new window opens maximized (`CEditWnd::Create`).
    pub(crate) fn maximized(&self) -> bool {
        self.size_mode != SaveMode::Default && self.size_type == SIZE_MAXIMIZED
    }

    /// The window changed its bounds (`OnSize`, `OnMove`): takes what the modes say to keep.
    /// Returns whether something changed.
    pub(crate) fn record(&mut self, bounds: WindowBounds) -> bool {
        let before = *self;
        match bounds {
            // A maximized window keeps only that it is one: its size and place are the screen's.
            WindowBounds::Maximized(_) => {
                if self.size_mode == SaveMode::Save {
                    self.size_type = SIZE_MAXIMIZED;
                }
            }
            WindowBounds::Windowed(b) => {
                if self.size_mode == SaveMode::Save {
                    self.size_type = SIZE_RESTORED;
                    self.width = f32::from(b.size.width).round() as i32;
                    self.height = f32::from(b.size.height).round() as i32;
                }
                if self.pos_mode == SaveMode::Save {
                    self.x = f32::from(b.origin.x).round() as i32;
                    self.y = f32::from(b.origin.y).round() as i32;
                }
            }
            WindowBounds::Fullscreen(_) => {}
        }
        *self != before
    }
}

/// The state now.
pub(crate) fn state(cx: &App) -> WindowState {
    cx.try_global::<WindowState>().copied().unwrap_or_default()
}

/// What is waiting to be written: the newest change, so that only the last of a burst of changes
/// (a window being dragged) starts a write, and whether the file lacks something.
#[derive(Default)]
struct Pending {
    generation: u64,
    dirty: bool,
}

impl Global for Pending {}

/// How long a window that is being dragged or resized waits before the file is written.
const WRITE_DELAY: Duration = Duration::from_millis(600);

/// A window changed its bounds: keeps what the modes say, and writes the file soon.
pub(crate) fn bounds_changed(cx: &mut App, bounds: WindowBounds) {
    let mut now = state(cx);
    if !now.record(bounds) {
        return;
    }
    cx.set_global(now);
    let pending = cx.default_global::<Pending>();
    pending.dirty = true;
    pending.generation += 1;
    let generation = pending.generation;
    cx.spawn(async move |cx| {
        cx.background_executor().timer(WRITE_DELAY).await;
        cx.update(|cx| {
            if cx.default_global::<Pending>().generation == generation {
                flush(cx);
            }
        });
    })
    .detach();
}

/// Writes the state to the settings file if it changed since the last write. Also when the
/// application ends (Sakura writes the file then).
pub(crate) fn flush(cx: &mut App) {
    let pending = cx.default_global::<Pending>();
    if !std::mem::take(&mut pending.dirty) {
        return;
    }
    let now = state(cx);
    crate::history::write_settings(cx, |profile| {
        let mut changed = false;
        for (key, value) in [
            (KEY_SIZE_TYPE, now.size_type),
            (KEY_SIZE_CX, now.width),
            (KEY_SIZE_CY, now.height),
            (KEY_POS_X, now.x),
            (KEY_POS_Y, now.y),
        ] {
            let text = value.to_string();
            if profile.get("Common", key) != Some(text.as_str()) {
                profile.set("Common", key, &text)?;
                changed = true;
            }
        }
        Ok(changed)
    });
}

#[cfg(test)]
mod tests {
    use gpui::Bounds;

    use super::*;

    fn windowed(x: f32, y: f32, w: f32, h: f32) -> WindowBounds {
        WindowBounds::Windowed(Bounds::new(point(px(x), px(y)), size(px(w), px(h))))
    }

    fn profile(text: &str) -> Profile {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        Profile::from_bytes(&bytes)
    }

    #[test]
    fn the_defaults_are_sakuras() {
        // the size is inherited and nothing is stored yet; the position is not kept
        let d = WindowState::from_profile(&Profile::new());
        assert_eq!(d, WindowState::default());
        assert_eq!(
            (d.size_mode, d.pos_mode),
            (SaveMode::Save, SaveMode::Default)
        );
        assert_eq!((d.size(), d.origin(), d.maximized()), (None, None, false));
    }

    #[test]
    fn the_keys_of_the_file_are_read() {
        let s = WindowState::from_profile(&profile(
            "[Common]\r\nbSaveWindowSize=2\r\nnWinSizeType=2\r\nnWinSizeCX=800\r\nnWinSizeCY=600\r\n\
             nSaveWindowPos=1\r\nnWinPosX=-30\r\nnWinPosY=40\r\n",
        ));
        assert_eq!(s.size_mode, SaveMode::Set);
        assert_eq!(s.pos_mode, SaveMode::Save);
        assert_eq!(s.size(), Some(size(px(800.), px(600.))));
        assert_eq!(s.origin(), Some(point(px(-30.), px(40.))));
        assert!(s.maximized());
        // a mode that is none of the three is the default one
        let s = WindowState::from_profile(&profile("[Common]\r\nbSaveWindowSize=9\r\n"));
        assert_eq!(s.size_mode, SaveMode::Save);
    }

    #[test]
    fn nothing_is_taken_in_the_mode_without_a_setting() {
        let mut s = WindowState::from_profile(&profile(
            "[Common]\r\nbSaveWindowSize=0\r\nnWinSizeType=2\r\nnWinSizeCX=800\r\nnWinSizeCY=600\r\n\
             nSaveWindowPos=0\r\nnWinPosX=5\r\nnWinPosY=6\r\n",
        ));
        assert_eq!((s.size(), s.origin(), s.maximized()), (None, None, false));
        // and nothing is kept
        assert!(!s.record(windowed(1., 2., 300., 200.)));
    }

    #[test]
    fn a_window_keeps_its_size_and_its_position_when_the_modes_say_so() {
        let mut s = WindowState {
            pos_mode: SaveMode::Save,
            ..Default::default()
        };
        assert!(s.record(windowed(10., 20., 640., 480.)));
        assert_eq!((s.width, s.height, s.x, s.y), (640, 480, 10, 20));
        assert_eq!(s.size_type, SIZE_RESTORED);
        // the same bounds again change nothing
        assert!(!s.record(windowed(10., 20., 640., 480.)));
        // a maximized window keeps the type only
        assert!(s.record(WindowBounds::Maximized(Bounds::new(
            point(px(0.), px(0.)),
            size(px(1920.), px(1080.))
        ))));
        assert_eq!(
            (s.size_type, s.width, s.height, s.x),
            (SIZE_MAXIMIZED, 640, 480, 10)
        );
        // and a fixed size is not overwritten
        let mut set = WindowState {
            size_mode: SaveMode::Set,
            width: 800,
            height: 600,
            ..Default::default()
        };
        assert!(!set.record(windowed(0., 0., 300., 300.)));
        assert_eq!((set.width, set.height), (800, 600));
    }

    #[test]
    fn a_new_window_takes_the_size_and_the_position() {
        let s = WindowState {
            size_mode: SaveMode::Save,
            width: 640,
            height: 480,
            pos_mode: SaveMode::Set,
            x: -100,
            y: 50,
            ..Default::default()
        };
        assert_eq!(s.size(), Some(size(px(640.), px(480.))));
        assert_eq!(s.origin(), Some(point(px(-100.), px(50.))));
        // a size that was never stored (CW_USEDEFAULT) leaves the default one
        assert_eq!(WindowState::default().size(), None);
    }
}
