//! The settings of the file in the view: what [`crate::settings`] says, and the changes the user
//! makes at run time (the font size, the wrap method, the status bar).
//!
//! The run-time changes are Sakura's temporary settings: they belong to this view and are not
//! written to the file (see the module documentation of [`crate::settings`]).

use gpui::{App, Context};
use matsu_layout::Wrap;

use super::EditorView;
use crate::layout_adapter::LAYOUT_BUDGET;
use crate::settings::UserSettings;
use crate::settings::view_settings::{
    ViewSettings, WrapMethod, points_to_px, window_wrap_columns, wrap_for,
};
use crate::settings::zoom::{FONT_ZOOM, zoomed_value};

/// `MAXLINEKETAS`: the widest a wrap column can be.
use matsu_layout::MAX_LINE_COLUMNS;

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
    /// How the selection is counted in the status bar (`m_nSelectCountMode`).
    pub select_count: crate::status_options::SelectCountMode,
    /// The layout was changed while some lines still have estimated row counts: bring the caret
    /// into view again as the counts become exact.
    pub reveal_caret: bool,
}

/// The temporary settings a change of settings keeps.
#[derive(Default)]
struct Temporary {
    /// The wrap method and the column.
    wrap: Option<(WrapMethod, u32)>,
    /// The font size in pixels, and the zoom.
    font: Option<(f32, f64)>,
}

impl ViewPrefs {
    pub(crate) fn new(settings: ViewSettings) -> Self {
        Self {
            wrap_method: settings.wrap_method,
            wrap_columns: settings.wrap_columns,
            zoom: 1.0,
            show_status_bar: settings.show_status_bar,
            select_count: Default::default(),
            reveal_caret: false,
            settings,
        }
    }
}

impl EditorView {
    /// Applies the values of a document type to this view: the font, the layout, the colors. The
    /// temporary changes (zoom, wrap) are reset, as they are when Sakura changes the type.
    pub fn apply_settings(&mut self, settings: ViewSettings, cx: &mut Context<Self>) {
        self.apply_settings_keeping(settings, Temporary::default(), cx);
    }

    /// The type of the view is the same and its settings changed (the type settings dialog, OK):
    /// takes them, and keeps the temporary wrap and the temporary font size unless they end
    /// (`CEditDoc::OnChangeSetting`, `sakura_core/doc/CEditDoc.cpp`):
    ///
    /// * a temporary wrap ends when it now equals the type's (for 指定桁, with the same column);
    /// * a temporary font size ends when `bUseTypeFont` or the base point size changed, and follows
    ///   a new face otherwise (the same rules as [`Self::refresh_font`]).
    pub(crate) fn apply_changed_settings(
        &mut self,
        settings: ViewSettings,
        cx: &mut Context<Self>,
    ) {
        let old = &self.prefs.settings;
        let prefs = &self.prefs;
        let temporary_wrap = prefs.wrap_method != old.wrap_method
            || (prefs.wrap_method == WrapMethod::SettingWidth
                && prefs.wrap_columns != old.wrap_columns);
        let wrap_ends = prefs.wrap_method == settings.wrap_method
            && (prefs.wrap_method != WrapMethod::SettingWidth
                || prefs.wrap_columns == settings.wrap_columns);
        let wrap =
            (temporary_wrap && !wrap_ends).then_some((prefs.wrap_method, prefs.wrap_columns));
        let temporary_font = self.font_size != old.font_px();
        let font = (temporary_font
            && settings.point_size == old.point_size
            && settings.use_type_font == old.use_type_font)
            .then_some((self.font_size, prefs.zoom));
        self.apply_settings_keeping(settings, Temporary { wrap, font }, cx);
    }

    fn apply_settings_keeping(
        &mut self,
        settings: ViewSettings,
        keep: Temporary,
        cx: &mut Context<Self>,
    ) {
        let status_bar = self.prefs.show_status_bar;
        let select_count = self.prefs.select_count;
        self.font_size = settings.font_px();
        if let Some(family) = &settings.font_family {
            self.font_family = family.clone().into();
        } else {
            self.font_family = super::FONT_FAMILY.into();
        }
        self.prefs = ViewPrefs::new(settings);
        if let Some((method, columns)) = keep.wrap {
            self.prefs.wrap_method = method;
            self.prefs.wrap_columns = columns;
        }
        if let Some((size, zoom)) = keep.font {
            self.font_size = size;
            self.prefs.zoom = zoom;
        }
        // Other rules of coloring: the states of the lines are made again.
        self.color_states.reset();
        self.line_color_cache.clear();
        // The status bar is a setting of the window, not of the type; a type change keeps it.
        self.prefs.show_status_bar = status_bar;
        self.prefs.select_count = select_count;
        self.relayout_in(cx);
        cx.notify();
    }

    /// Reads the settings file for this view: the values for the file it shows, and the user's
    /// key bindings. The first problem in the file goes to the status bar.
    pub fn apply_user_settings(&mut self, user: &UserSettings, cx: &mut Context<Self>) {
        self.keymap = user.key_map(matsu_commands::keymap::Platform::current());
        self.prefs.show_status_bar = true;
        self.apply_type_of_document(user, cx);
        // The status bar of the file is the start value of the window.
        self.prefs.show_status_bar = self.prefs.settings.show_status_bar;
        if let Some(first) = user.warnings.first() {
            self.notice = Some(format!(
                "matsu.ini: {first}{}",
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
        // A file that is loaded takes the type of its name: the user's choice (`F_CHANGETYPE`) ends.
        let index = user.type_index_for(name.as_deref());
        self.document.update(cx, |shared, cx| {
            shared.set_type_locked(false);
            shared.announce_type(index, cx);
        });
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
        // The columns of the corners of a box are in the old layout and mean nothing in the new.
        if self.cursor.is_box() {
            self.cursor.clear_selection();
        }
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
        } else if i64::from(matsu_layout::MIN_LINE_COLUMNS) > view_columns - i64::from(overhang) {
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
        self.set_temporary_font_size(0, steps, cx);
    }

    /// The size `Command_SETFONTSIZE` (`sakura_core/cmd/CViewCommander_Settings.cpp`) gives the
    /// font, in 1/10 point, and the zoom that goes with it; `None` when nothing changes.
    ///
    /// `size` is a size to set (brought into 1 to 72 points; the zoom is 1.0 again), else `shift`
    /// steps of the zoom table from the size of the font of the file (from the zoom now, when
    /// `temporary`). Only TrueType fonts are changed (`truetype` says the font is one).
    pub(crate) fn plan_font_size(
        &self,
        size: i32,
        shift: i32,
        truetype: bool,
        temporary: bool,
    ) -> Option<(i32, f64)> {
        if !truetype {
            return None;
        }
        let zoom = if temporary { self.prefs.zoom } else { 1.0 };
        if size != 0 {
            let tenths = size.clamp(FONT_ZOOM.value_min as i32, FONT_ZOOM.value_max as i32);
            return Some((tenths, 1.0));
        }
        if shift == 0 {
            return None;
        }
        let base = f64::from(self.prefs.settings.point_size);
        // The table has 72 steps; a shift from a script can be any number.
        let shift = shift.clamp(-100, 100);
        let (tenths, zoom) = zoomed_value(&FONT_ZOOM, base, zoom, shift)?;
        Some((tenths as i32, zoom))
    }

    /// `Command_SETFONTSIZE` in mode 2 ("一時適用フォント"): the size for this view only.
    pub fn set_temporary_font_size(&mut self, size: i32, shift: i32, cx: &mut Context<Self>) {
        let truetype = self.prefs.settings.font_truetype;
        let Some((tenths, zoom)) = self.plan_font_size(size, shift, truetype, true) else {
            return;
        };
        self.prefs.zoom = zoom;
        self.resize_font(points_to_px(tenths), cx);
    }

    /// Draws the text at `px` pixels, keeping the same text at the top left of the view: the scroll
    /// offsets are in pixels.
    fn resize_font(&mut self, px: f32, cx: &mut Context<Self>) {
        let old = self.font_size;
        self.font_size = px;
        // The line height is the font size times 1.5, rounded (see `Render for EditorView`).
        let line_height = |size: f32| (size * 1.5).round();
        self.scroll_y *= line_height(self.font_size) / line_height(old);
        self.scroll_x *= self.font_size / old;
        // The metrics follow at the next paint, which brings the caret into view if it left.
        cx.notify();
    }

    /// The common font or the font of a type changed (`PM_CHANGESETTING_FONT`,
    /// `PM_CHANGESETTING_FONTSIZE`): this view reads the font of its type again.
    ///
    /// A size set for this view only (`F_SETFONTSIZEUP`, ...) stays if the size of the font of the
    /// file is the same as before (and follows a new face); otherwise it ends
    /// (`CEditDoc::OnChangeSetting`, `sakura_core/doc/CEditDoc.cpp`: "フォント設定が変更された。
    /// 元に戻す"). A view whose type has a font of its own does not notice a change of the common font.
    pub(crate) fn refresh_font(&mut self, user: &UserSettings, cx: &mut Context<Self>) {
        let new = user.view_settings_of_type(self.prefs.settings.type_index);
        let old = &self.prefs.settings;
        if new.font_family == old.font_family
            && new.point_size == old.point_size
            && new.font_weight == old.font_weight
            && new.font_italic == old.font_italic
            && new.font_truetype == old.font_truetype
            && new.use_type_font == old.use_type_font
        {
            return;
        }
        let temporary = self.font_size != old.font_px();
        // `bFontTypeOld != ref.m_bUseTypeFont` ends it too.
        let keep_size =
            temporary && new.point_size == old.point_size && new.use_type_font == old.use_type_font;
        self.font_family = match &new.font_family {
            Some(family) => family.clone().into(),
            None => super::FONT_FAMILY.into(),
        };
        let px = new.font_px();
        let settings = &mut self.prefs.settings;
        settings.font_family = new.font_family;
        settings.point_size = new.point_size;
        settings.font_weight = new.font_weight;
        settings.font_italic = new.font_italic;
        settings.font_truetype = new.font_truetype;
        settings.use_type_font = new.use_type_font;
        if keep_size {
            cx.notify();
        } else {
            self.prefs.zoom = 1.0;
            self.resize_font(px, cx);
        }
    }

    /// `F_SHOWSTATUSBAR`: shows the status bar, or hides it.
    pub fn toggle_status_bar(&mut self, cx: &mut Context<Self>) {
        self.prefs.show_status_bar = !self.prefs.show_status_bar;
        cx.notify();
    }

    /// Shows or hides the status bar (the common settings, `bDispSTATUSBAR`).
    pub(crate) fn set_status_bar(&mut self, shown: bool, cx: &mut Context<Self>) {
        if self.prefs.show_status_bar != shown {
            self.prefs.show_status_bar = shown;
            cx.notify();
        }
    }

    // The wheel chords of the key map (Ctrl+wheel = font size, remapped wheels) are run by
    // `EditorView::handle_wheel` in `paging.rs`.

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
