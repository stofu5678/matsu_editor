//! The settings of the file in the view: what [`crate::settings`] says, and the changes the user
//! makes at run time (the font size, the wrap method, the status bar).
//!
//! The run-time changes are Sakura's temporary settings: they belong to this view and are not
//! written to the file (see the module documentation of [`crate::settings`]).

use gpui::{App, Context};
use ume_commands::keymap::UserRow;
use ume_layout::Wrap;

use super::EditorView;
use crate::layout_adapter::LAYOUT_BUDGET;
use crate::settings::UserSettings;
use crate::settings::view_settings::{
    ViewSettings, WrapMethod, points_to_px, window_wrap_columns, wrap_for,
};
use crate::settings::zoom::{FONT_ZOOM, zoomed_value};

/// `MAXLINEKETAS`: the widest a wrap column can be.
use ume_layout::MAX_LINE_COLUMNS;

/// What the view keeps of the settings: the values of the file, and the temporary changes.
#[derive(Clone, Debug)]
pub(crate) struct ViewPrefs {
    /// The values of the file for the document type of this view.
    pub settings: ViewSettings,
    /// The wrap method now (`m_nTextWrapMethodCur`).
    pub wrap_method: WrapMethod,
    /// The wrap column of [`WrapMethod::SettingWidth`] now. It is the type's column, unless
    /// `F_WRAPWINDOWWIDTH` changed it (`CLayoutMgr::GetMaxLineKetas`).
    pub wrap_columns: u32,
    /// The zoom of the font size, 1.0 for the size of the file (`m_nCurrentZoom`).
    pub zoom: f64,
    pub show_status_bar: bool,
    /// The layout was changed while some lines still have estimated row counts: bring the caret
    /// into view again as the counts become exact.
    pub reveal_caret: bool,
}

impl ViewPrefs {
    pub(crate) fn new(settings: ViewSettings) -> Self {
        Self {
            wrap_method: settings.wrap_method,
            wrap_columns: settings.wrap_columns,
            zoom: 1.0,
            show_status_bar: settings.show_status_bar,
            reveal_caret: false,
            settings,
        }
    }
}

impl EditorView {
    /// Applies the values of a document type to this view: the font, the layout, the colors. The
    /// temporary changes (zoom, wrap) are reset, as they are when Sakura changes the type.
    pub fn apply_settings(&mut self, settings: ViewSettings, cx: &mut Context<Self>) {
        let status_bar = self.prefs.show_status_bar;
        self.font_size = settings.font_px();
        if let Some(family) = &settings.font_family {
            self.font_family = family.clone().into();
        } else {
            self.font_family = super::FONT_FAMILY.into();
        }
        self.prefs = ViewPrefs::new(settings);
        // The status bar is a setting of the window, not of the type; a type change keeps it.
        self.prefs.show_status_bar = status_bar;
        self.relayout_in(cx);
        cx.notify();
    }

    /// Reads the settings file for this view: the values for the file it shows, and the user's
    /// key bindings. The first problem in the file goes to the status bar.
    pub fn apply_user_settings(&mut self, user: &UserSettings, cx: &mut Context<Self>) {
        let rows: Vec<UserRow> = user.key_rows();
        self.keymap.apply_user_rows(&rows);
        self.prefs.show_status_bar = true;
        self.apply_type_of_document(user, cx);
        // The status bar of the file is the start value of the window.
        self.prefs.show_status_bar = self.prefs.settings.show_status_bar;
        if let Some(first) = user.warnings.first() {
            self.notice = Some(format!(
                "ume.ini: {first}{}",
                if user.warnings.len() > 1 {
                    format!(" (ほか {} 件)", user.warnings.len() - 1)
                } else {
                    String::new()
                }
            ));
        }
        cx.notify();
    }

    /// Chooses the document type of the file this view shows and applies it (Sakura does this
    /// when a file is opened or saved under another name). Call it after the name changes.
    pub fn apply_type_of_document(&mut self, user: &UserSettings, cx: &mut Context<Self>) {
        let name = self.document.read(cx).file_name();
        self.apply_settings(user.view_settings(name.as_deref()), cx);
    }

    /// The settings of the file for this view (the type, the colors, what is shown).
    pub fn view_settings(&self) -> &ViewSettings {
        &self.prefs.settings
    }

    /// Whether the status bar is shown (`F_SHOWSTATUSBAR`).
    pub fn status_bar_shown(&self) -> bool {
        self.prefs.show_status_bar
    }

    /// The font size now, in GPUI pixels.
    pub fn font_px(&self) -> f32 {
        self.font_size
    }

    // ---- wrap ----

    /// How many columns the text area has, as of the last paint (`m_nViewColNum`).
    pub(crate) fn window_columns(&self) -> u32 {
        let m = &self.metrics;
        if m.cell_width > 0.0 {
            (m.view_width / m.cell_width).floor().max(0.0) as u32
        } else {
            0
        }
    }

    /// The wrap the layout should have now.
    pub(crate) fn wanted_wrap(&self) -> Wrap {
        wrap_for(
            self.prefs.wrap_method,
            self.prefs.wrap_columns,
            self.window_columns(),
            self.prefs.settings.wrap_overhang,
        )
    }

    /// Whether the layout is out of date for the width of the window.
    pub(crate) fn wrap_is_stale(&self) -> bool {
        self.prefs.wrap_method == WrapMethod::WindowWidth
            && self.layout.config().wrap != self.wanted_wrap()
    }

    /// Lays the document out again with the settings of the view, if they differ from the
    /// layout's. Returns whether anything changed. The caller repaints.
    pub(crate) fn relayout_in(&mut self, cx: &App) -> bool {
        let mut config = self.layout.config().clone();
        config.wrap = self.wanted_wrap();
        config.tab_width = self.prefs.settings.tab_width;
        config.word_wrap = self.prefs.settings.word_wrap;
        if config == *self.layout.config() {
            return false;
        }
        let doc = self.document.read(cx).doc();
        let _ = self.layout.set_config(doc, config);
        let _ = self.layout.layout_for(doc, LAYOUT_BUDGET);
        let _ = self.cursor.clamp(doc);
        if matches!(self.layout.config().wrap, Wrap::Columns(_)) {
            self.scroll_x = 0.0;
        }
        self.clamp_scroll();
        // The caret has moved to another row; keep it in view.
        self.ensure_caret_visible(cx);
        self.prefs.reveal_caret = !self.layout.is_complete();
        true
    }

    // ---- temporary settings (Sakura's "一時設定") ----

    /// `Command_TEXTWRAPMETHOD` (`F_TMPWRAPNOWRAP`, `F_TMPWRAPSETTING`, `F_TMPWRAPWINDOW`): wraps
    /// by `method` for this view only. Nothing happens when the view already wraps that way.
    pub fn set_wrap_method(&mut self, method: WrapMethod, cx: &mut Context<Self>) {
        if self.prefs.wrap_method == method {
            return;
        }
        if method == WrapMethod::SettingWidth {
            // The column of the type, not the one a former `F_WRAPWINDOWWIDTH` set.
            self.prefs.wrap_columns = self.prefs.settings.wrap_columns;
        }
        self.prefs.wrap_method = method;
        self.relayout_in(cx);
        cx.notify();
    }

    /// `Command_WRAPWINDOWWIDTH` (`F_WRAPWINDOWWIDTH`): wraps at the width of the window, then at
    /// the largest column, then at the column of the type, and round again; steps that would
    /// change nothing are skipped (`CEditView::GetWrapMode`,
    /// `sakura_core/view/CEditView_Command.cpp`). The column stays as it was set; it does not
    /// follow the window afterwards (the method becomes 指定桁).
    pub fn toggle_wrap_window_width(&mut self, cx: &mut Context<Self>) {
        let current = self.layout.config().max_columns();
        let type_columns = self.prefs.settings.wrap_columns;
        let overhang = self.prefs.settings.wrap_overhang;
        let view_columns = i64::from(self.window_columns());
        let window = window_wrap_columns(self.window_columns(), overhang);
        let columns = if current == window {
            Some(MAX_LINE_COLUMNS)
        } else if i64::from(ume_layout::MIN_LINE_COLUMNS) > view_columns - i64::from(overhang) {
            // The window is too narrow to wrap at.
            if current != MAX_LINE_COLUMNS {
                Some(MAX_LINE_COLUMNS)
            } else if type_columns == MAX_LINE_COLUMNS {
                None
            } else {
                Some(type_columns)
            }
        } else if current == MAX_LINE_COLUMNS && type_columns != MAX_LINE_COLUMNS {
            Some(type_columns)
        } else {
            Some(window)
        };
        self.prefs.wrap_method = WrapMethod::SettingWidth;
        if let Some(columns) = columns {
            self.prefs.wrap_columns = columns;
            self.relayout_in(cx);
        }
        cx.notify();
    }

    /// `Command_SETFONTSIZE` in mode 2 (`F_SETFONTSIZEUP` with `steps` = 1, `F_SETFONTSIZEDOWN`
    /// with -1): the font gets the next size of the zoom table, for this view only.
    pub fn zoom_font(&mut self, steps: i32, cx: &mut Context<Self>) {
        let base = f64::from(self.prefs.settings.point_size);
        let Some((tenths, zoom)) = zoomed_value(&FONT_ZOOM, base, self.prefs.zoom, steps) else {
            return;
        };
        self.prefs.zoom = zoom;
        let old = self.font_size;
        self.font_size = points_to_px(tenths as i32);
        // Keep the same text at the top left of the view: the scroll offsets are in pixels.
        // The line height is the font size times 1.5, rounded (see `Render for EditorView`).
        let line_height = |size: f32| (size * 1.5).round();
        self.scroll_y *= line_height(self.font_size) / line_height(old);
        self.scroll_x *= self.font_size / old;
        // The metrics follow at the next paint, which brings the caret into view if it left.
        cx.notify();
    }

    /// `F_SHOWSTATUSBAR`: shows the status bar, or hides it.
    pub fn toggle_status_bar(&mut self, cx: &mut Context<Self>) {
        self.prefs.show_status_bar = !self.prefs.show_status_bar;
        cx.notify();
    }

    /// Ctrl+wheel and the other wheel chords of the key map (Sakura's table has the wheel as a
    /// key: `VKEX_WHEEL_UP`, `sakura_core/func/CKeyBind.cpp`). Runs the function of the chord
    /// and returns true. Returns false for a chord without a function (the caller scrolls).
    pub(crate) fn run_wheel_function(
        &mut self,
        event: &gpui::ScrollWheelEvent,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> bool {
        use ume_commands::keymap::{Chord, Key, Modifiers, MouseAction, Resolved};
        // The functions that scroll; they are done here, in the direction the function names.
        const F_WHEELUP: u16 = 30385;
        const F_WHEELDOWN: u16 = 30386;
        const F_WHEELLEFT: u16 = 30387;
        const F_WHEELRIGHT: u16 = 30388;
        let delta = event.delta.pixel_delta(gpui::px(self.metrics.line_height));
        let (dx, dy) = (f32::from(delta.x), f32::from(delta.y));
        // The vertical wheel decides; a horizontal-only event uses the horizontal wheel keys.
        let action = if dy > 0.0 {
            MouseAction::WheelUp
        } else if dy < 0.0 {
            MouseAction::WheelDown
        } else if dx > 0.0 {
            MouseAction::WheelLeft
        } else if dx < 0.0 {
            MouseAction::WheelRight
        } else {
            return false;
        };
        let m = event.modifiers;
        let chord = Chord::new(
            Key::Mouse(action),
            Modifiers {
                shift: m.shift,
                ctrl: m.control,
                alt: m.alt,
                meta: m.platform,
            },
        );
        match self.keymap.resolve(chord) {
            // The function decides the direction, not the wheel: a user may bind wheel up to
            // F_WHEELDOWN.
            Resolved::Code(code @ (F_WHEELUP | F_WHEELDOWN | F_WHEELLEFT | F_WHEELRIGHT)) => {
                // The amount of the event, from whichever axis it has; the function gives the
                // direction.
                let n = if dy != 0.0 { dy.abs() } else { dx.abs() };
                let (dx, dy) = match code {
                    F_WHEELUP => (if dy != 0.0 { dx } else { 0.0 }, n),
                    F_WHEELDOWN => (if dy != 0.0 { dx } else { 0.0 }, -n),
                    F_WHEELLEFT => (n, 0.0),
                    _ => (-n, 0.0),
                };
                self.scroll_by(dx, dy);
                cx.notify();
                true
            }
            Resolved::Code(code) => {
                self.run_function(code, &[], window, cx);
                true
            }
            _ => false,
        }
    }

    /// While a large document is still being laid out after a wrap change, the caret's row moves
    /// as the lines before it get exact counts; keep it in view until the layout is complete.
    pub(crate) fn settle_caret(&mut self, cx: &App) {
        if self.prefs.reveal_caret {
            self.ensure_caret_visible(cx);
            self.prefs.reveal_caret = !self.layout.is_complete();
        }
    }

    /// For the wrap at the right edge: lays the text out again if the window changed its width.
    /// Called before each frame; the paint asks for another frame when it measured a new width.
    pub(crate) fn follow_window_width(&mut self, cx: &App) {
        if self.prefs.wrap_method == WrapMethod::WindowWidth {
            self.relayout_in(cx);
        }
    }
}
