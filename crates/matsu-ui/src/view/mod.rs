//! `Entity<EditorView>`: the view layer of ADR-0013 section 3.
//!
//! It owns what belongs to one view of a document: the layout (`matsu-layout`), the cursor, the
//! scroll position, the key map, and the IME composition. The document itself is in an
//! `Entity<SharedDocument>` that other views can share.
//!
//! # Running a command
//!
//! Everything that changes the text or the cursor goes through `EditorView::run_with`: keys,
//! typed text, the IME, the mouse and the menu. It opens a [`CommandContext`] over the document,
//! the cursor and the layout, runs the closure, and then does what ADR-0013 section 4 asks: the
//! layout matches the text, the caret is in view, and the window repaints.
//!
//! The context tells the layout about each edit as it happens (`RowLayout`), so a command that
//! edits and then moves vertically sees the new rows.

mod a11y_text;
pub(crate) mod autoscroll;
mod bracket_marks;
mod caret_shape;
mod colors;
mod diff_mark;
mod element;
mod held_buttons;
mod input;
mod lines;
pub(crate) mod minimap;
mod mouse;
pub(crate) mod paging;
pub(crate) mod pane;
mod ruler;
mod scan;
mod settings;
#[cfg(test)]
mod settings_tests;
mod status_info;
mod syntax;

use std::ops::Range;

use gpui::{
    App, Context, CursorStyle, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled, Subscription, Window, actions, div, px,
};
use matsu_commands::command::{Arg, Registration};
use matsu_commands::context::{Changes, CommandContext};
use matsu_commands::keymap::{Key, KeyMap, Lookup};
use matsu_commands::registry::Registry;
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::{Document, Pos};
use matsu_core::cursor::Cursor;
use matsu_layout::{DocumentLayout, LayoutConfig, RowPos};

use crate::document::{DocumentEvent, SharedDocument};
use crate::geometry::{Metrics, clamp_scroll_y, scroll_x_to_show, scroll_y_to_show_row};
use crate::keys::chord_of;
use crate::layout_adapter::{LAYOUT_BUDGET, RowLayout};
use crate::macros::MacroState;
use crate::search::{ISearch, SearchPanel, SearchState};
use crate::status::StatusInfo;
use crate::ui_command::{UiRegistration, UiRegistry};

use element::EditorElement;
use held_buttons::HeldButtons;
pub(crate) use settings::ViewPrefs;

actions!(
    matsu,
    [
        /// Edit > Undo
        Undo,
        /// Edit > Redo
        Redo,
    ]
);

/// A menu item: run the Sakura function with this code on the editor (`EditorView::run_function`).
#[derive(Clone, PartialEq, Debug, gpui::Action)]
#[action(namespace = matsu, no_json)]
pub struct RunFunction {
    pub code: u16,
}

/// A menu item of the recent files: open this file. The path is the one the menu showed, so a history
/// that another window changed since cannot make the item open another file.
#[derive(Clone, PartialEq, Debug, gpui::Action)]
#[action(namespace = matsu, no_json)]
pub struct OpenRecentFile {
    pub path: String,
}

/// A menu item of the recent folders: show the open dialog in this folder, and open the files
/// chosen. The path is the one the menu showed.
#[derive(Clone, PartialEq, Debug, gpui::Action)]
#[action(namespace = matsu, no_json)]
pub struct OpenRecentFolder {
    pub path: String,
}

/// The font of the text, by OS: a Japanese monospaced font, so that full-width characters are
/// twice as wide as half-width ones. GPUI picks a font by family name and falls back to its
/// default when the font is missing (the PoC's list of fonts is in `poc/gpui-textview`).
#[cfg(any(windows, target_os = "macos"))]
pub const FONT_FAMILY: &str = "BIZ UDGothic";
#[cfg(not(any(windows, target_os = "macos")))]
pub const FONT_FAMILY: &str = "Noto Sans Mono CJK JP";

/// `F_INDENT_TAB`, the function of the Tab key (`sakura_core/func/CKeyBind.cpp`).
const F_INDENT_TAB: u16 = 30260;

/// The text of an IME composition: it is in the document, on one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Marked {
    pub line: usize,
    /// Byte columns of the line.
    pub range: Range<usize>,
    /// The spaces right before `range` that the composition put there to fill the gap of the
    /// free cursor mode. They go when the composition is cancelled.
    pub pad: usize,
}

/// What the mouse is doing between button down and button up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Drag {
    None,
    /// Selecting from the range `bgn` (the empty range at the click, or the word, the URL that the
    /// press selected) to the pointer; with `words` both ends snap to word boundaries
    /// (`m_sSelectBgn`, `m_bBeginWordSelect`).
    Select {
        bgn: (Pos, Pos),
        words: bool,
    },
    /// Selecting rows (the line number column) or logical lines (a triple click) from `bgn`.
    Lines {
        bgn: (Pos, Pos),
        logical: bool,
    },
    /// An Alt drag: the corner of a box selection follows the pointer.
    Box,
    /// The button went down in the selection (with drag and drop on): a click if the pointer stays,
    /// a drag of the text once it moves ([`Drag::Drop`]).
    Press {
        down: gpui::Point<gpui::Pixels>,
    },
    /// Dragging the selected text; [`EditorView::drop_caret`] shows where it would go.
    Drop,
    /// Dragging the scroll bar thumb.
    Thumb {
        start_y: f32,
        start_scroll: f32,
    },
}

/// Where the last paint put the text, in window coordinates.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PaintInfo {
    /// The top left of the text area (right of the line numbers).
    pub origin_x: f32,
    pub origin_y: f32,
    /// The left edge of the scroll bar.
    pub scrollbar_left: f32,
}

pub struct EditorView {
    pub(crate) document: Entity<SharedDocument>,
    pub(crate) layout: DocumentLayout,
    /// The version of the document that `layout` matches.
    pub(crate) layout_version: u64,
    /// The color state at the start of each line, for the syntax coloring (`view/colors.rs`), and
    /// the version of the document it matches.
    pub(crate) color_states: matsu_coloring::ColorStates,
    pub(crate) color_version: u64,
    /// The colors of the last long line painted (`view/colors.rs`).
    pub(crate) line_color_cache: colors::LineColorCache,
    /// The rows on the screen, left by the last paint for the screen readers (`a11y_text.rs`).
    a11y_text: a11y_text::Shared,
    pub(crate) cursor: Cursor,
    pub(crate) keymap: KeyMap,
    pub(crate) focus_handle: FocusHandle,
    pub(crate) scroll_x: f32,
    pub(crate) scroll_y: f32,
    /// The top row may be past the end of the scroll range (Sakura's `MoveViewTopLine`, see
    /// `paging.rs`); any other scrolling clears this.
    pub(crate) scroll_past_end: bool,
    /// The part of a wheel notch that has not made a whole row yet, vertical and horizontal
    /// (`paging.rs`).
    pub(crate) wheel_remainder: [f32; 2],
    /// The mouse buttons that are down now, for the wheel combinations (`paging.rs`).
    pub(crate) held_buttons: HeldButtons,
    /// The buttons that were the key of a wheel combination while the wheel turned, so letting
    /// the button go does nothing (`paging.rs`).
    pub(crate) wheel_used_buttons: HeldButtons,
    pub(crate) metrics: Metrics,
    pub(crate) paint: Option<PaintInfo>,
    /// The widest end column among the rows of the last paint, for the horizontal scroll range.
    pub(crate) widest_cols: u32,
    pub(crate) marked: Option<Marked>,
    /// An undo block for the composition is open (it closes when the text is committed).
    pub(crate) composing: bool,
    /// The composition replaced a selection when it started: its commit overwrites nothing more.
    pub(crate) composing_over_selection: bool,
    pub(crate) drag: Drag,
    /// Where a drag of the selected text would drop it: the caret is drawn there meanwhile
    /// (Sakura moves the caret with the pointer, `OnMOUSEMOVE` while `m_bDragMode`).
    pub(crate) drop_caret: Option<Pos>,
    /// Which click of a series the last press was: 1 to 4 (single, double, triple, quadruple).
    /// Sakura keeps it as `m_dwTripleClickCheck` (`mouse.rs`).
    pub(crate) click_phase: u8,
    /// The auto scroll of the middle button (`view/autoscroll.rs`).
    pub(crate) autoscroll: autoscroll::AutoScroll,
    pub(crate) notice: Option<String>,
    /// Message boxes that commands asked for (`Changes::message_box`), shown by the next paint,
    /// which has the window.
    pending_message_boxes: Vec<String>,
    /// A jump to a place the scan of the file has not reached, held back until it is done
    /// (`view/scan.rs`).
    pub(crate) pending_scan_jump: Option<scan::PendingScanJump>,
    /// The last command was refused because the document is read-only (`view/scan.rs`).
    refused_edit: bool,
    /// The jump that waited for the scan runs now: it was recorded already.
    replaying_scan_jump: bool,
    /// The caret and the history of a file that was opened while it is scanned, to restore when
    /// the scan is done (`view/scan.rs`).
    pub(crate) pending_scan_restore: Option<scan::ScanRestore>,
    /// The settings of the file and the temporary changes (`view/settings.rs`).
    pub(crate) prefs: ViewPrefs,
    /// The size of the selection counted last (`status_info.rs`).
    selection_count: status_info::SelectionCountCache,
    /// The print preview and the print dialog (`crate::print`).
    pub(crate) print: crate::print::PrintState,
    /// Find and replace: the pattern, the options and the marks (`crate::search`).
    pub(crate) search: SearchState,
    /// The incremental search, while its mode is on (`crate::search::isearch`).
    pub(crate) isearch: Option<ISearch>,
    /// The find, replace or go-to-line panel, while it is open.
    pub(crate) panel: Option<Entity<SearchPanel>>,
    /// Grep: the dialog, and whether this window shows a result (`crate::grep`).
    pub(crate) grep: crate::grep::window::GrepWindow,
    /// Key macro recording and the macro that runs (`crate::macros`).
    pub(crate) macros: MacroState,
    /// The Save As options dialog that covers the view, while one is open.
    pub(crate) save_as_dialog: Option<Entity<crate::file_ops::save_as_dialog::SaveAsDialog>>,
    /// The text input dialog (`InputBox`), while it is open.
    pub(crate) input_dialog: Option<Entity<crate::input_dialog::InputDialog>>,
    /// The list that chooses a connection of `databases.ini` (ADR-0023).
    pub(crate) connection_dialog: Option<Entity<crate::database::pick::ConnectionDialog>>,
    /// The completion list, the command list and the version dialog (`crate::support`).
    pub(crate) support: crate::support::Support,
    /// The window list (popup or dialog) over the view, while one is open.
    pub(crate) window_list: Option<Entity<crate::window_ops::list::WindowList>>,
    /// The tab list (`CTabWnd::TabListMenu`) over the view, while one is open.
    pub(crate) tab_list: Option<Entity<crate::tabs::list::TabList>>,
    /// The DIFF and compare dialogs, while one is open (`crate::diff`).
    pub(crate) diff: crate::diff::DiffState,
    /// Whether the user chose the document type, and the list of types (`crate::doc_types`).
    pub(crate) doc_type: crate::doc_types::DocTypeState,
    /// The popup of a custom menu or the right-click menu, while one is open (`crate::custom_menu`).
    pub(crate) custom_menu: crate::custom_menu::CustomMenuState,
    /// The window that holds this view among its panes (`crate::panes`).
    pub(crate) split_host: Option<gpui::WeakEntity<crate::app::Workspace>>,
    font_family: SharedString,
    font_size: f32,
    /// This view's own id, to tell it from the other views of its document.
    pub(crate) entity_id: gpui::EntityId,
    _subscription: Subscription,
}

impl EditorView {
    pub fn new(document: Entity<SharedDocument>, cx: &mut Context<Self>) -> Self {
        let mut layout = DocumentLayout::new(document.read(cx).doc(), LayoutConfig::default())
            .unwrap_or_else(|_| empty_layout());
        let _ = layout.layout_for(document.read(cx).doc(), LAYOUT_BUDGET);
        let layout_version = document.read(cx).version();
        let subscription = cx.subscribe(&document, |this, _, event, cx| match *event {
            DocumentEvent::Changed { version } => this.document_changed(version, cx),
            DocumentEvent::TypeChanged { index } => this.follow_document_type(index, cx),
        });
        // The typing mode changes only notify the document (no `Changed` event, no new layout):
        // repaint for the caret and the status bar.
        cx.observe(&document, |_, _, cx| cx.notify()).detach();
        let weak = cx.weak_entity();
        document.update(cx, |shared, _| shared.add_view(weak));
        Self {
            entity_id: cx.entity_id(),
            document,
            layout,
            layout_version,
            color_states: Default::default(),
            color_version: layout_version,
            line_color_cache: Default::default(),
            a11y_text: Default::default(),
            cursor: Cursor::default(),
            keymap: KeyMap::default(),
            focus_handle: cx.focus_handle(),
            scroll_x: 0.0,
            scroll_y: 0.0,
            scroll_past_end: false,
            wheel_remainder: [0.0; 2],
            held_buttons: HeldButtons::default(),
            wheel_used_buttons: HeldButtons::default(),
            metrics: Metrics::default(),
            paint: None,
            widest_cols: 0,
            marked: None,
            composing: false,
            composing_over_selection: false,
            drag: Drag::None,
            drop_caret: None,
            click_phase: 1,
            autoscroll: Default::default(),
            notice: None,
            pending_message_boxes: Vec::new(),
            pending_scan_jump: None,
            refused_edit: false,
            replaying_scan_jump: false,
            pending_scan_restore: None,
            prefs: ViewPrefs::new(Default::default()),
            selection_count: Default::default(),
            print: Default::default(),
            search: SearchState::default(),
            isearch: None,
            panel: None,
            grep: Default::default(),
            macros: MacroState::default(),
            save_as_dialog: None,
            input_dialog: None,
            connection_dialog: None,
            support: Default::default(),
            window_list: None,
            tab_list: None,
            diff: Default::default(),
            doc_type: Default::default(),
            custom_menu: Default::default(),
            split_host: None,
            font_family: FONT_FAMILY.into(),
            font_size: crate::settings::ViewSettings::default().font_px(),
            _subscription: subscription,
        }
    }

    pub fn document(&self) -> &Entity<SharedDocument> {
        &self.document
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    pub fn layout(&self) -> &DocumentLayout {
        &self.layout
    }

    pub fn scroll_y(&self) -> f32 {
        self.scroll_y
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    /// Sets the size of the text area, as the paint does. For tests, which have no paint.
    pub fn set_metrics(&mut self, metrics: Metrics) {
        self.metrics = metrics;
    }

    /// The font family and size of the text, for what is drawn over it.
    pub(crate) fn font(&self) -> (SharedString, f32) {
        (self.font_family.clone(), self.font_size)
    }

    /// The document of a view (the text only).
    fn doc<'a>(&self, cx: &'a App) -> &'a Document {
        self.document.read(cx).doc()
    }

    // ---- commands ----

    /// Runs `f` on a [`CommandContext`] over the document, the cursor and the layout, then brings
    /// the view up to date with what the context recorded.
    pub(crate) fn run_with<R>(
        &mut self,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut CommandContext<'_>) -> R,
    ) -> R {
        let document = self.document.clone();
        // The macro that runs before a save is told the file it will be written to.
        let save_path = self.macros.save_path().map(std::path::Path::to_owned);
        // `bInsSpace` of the view, which `ChangeInsertSpace` and the type settings change.
        let ins_space = self.prefs.settings.ins_space;
        let layout = &mut self.layout;
        let cursor = &mut self.cursor;
        let (result, changes) = document.update(cx, |shared, cx| {
            let overwrite = shared.overwrite();
            // View mode, or a file that cannot be written (`CEditDoc::IsEditable`).
            let uneditable = crate::file_ops::prefs::prefs(cx).uneditable_if_unwritable;
            // A document that shows only the scanned start of its file is read-only too.
            let view_mode = shared.view_mode()
                || shared.locked_by_unwritable(uneditable)
                || shared.is_partial();
            let (model, path) = shared.parts_mut();
            let mut adapter = RowLayout(layout);
            let quote = cx
                .try_global::<crate::clipboard::QuoteString>()
                .map(|quote| quote.0.clone());
            let mut clipboard = crate::clipboard::GpuiClipboard(cx);
            let mut command_cx = CommandContext::new(model, cursor, &mut adapter, path)
                .with_clipboard(&mut clipboard)
                .with_overwrite(overwrite)
                .with_ins_space(ins_space)
                .with_save_path(save_path.as_deref())
                .with_view_mode(view_mode);
            if let Some(quote) = quote {
                command_cx = command_cx.with_quote_string(quote);
            }
            let result = f(&mut command_cx);
            let changes = command_cx.finish();
            if let Some(quote) = changes.quote_string.clone() {
                cx.set_global(crate::clipboard::QuoteString(quote));
            }
            shared.set_overwrite(changes.overwrite, cx);
            if changes.bookmarks_changed {
                // the other views of the document draw the marks too
                cx.notify();
            }
            (result, changes)
        });
        self.after_command(&changes, cx);
        result
    }

    /// Runs the command `reg` with `args`. A failure goes to the status bar; the result says
    /// whether the command succeeded.
    pub(crate) fn run_registration(
        &mut self,
        reg: &'static Registration,
        args: &[Arg],
        cx: &mut Context<Self>,
    ) -> bool {
        // Any function but F_AUTOSCROLL ends the auto scroll (`CViewCommander::HandleCommand`).
        self.autoscroll_exit();
        self.interrupt_composition(cx);
        // Before the recording, as in `CViewCommander::HandleCommand`: the typed chars of the
        // incremental search are not edits, so a macro does not replay them as such.
        if self.isearch_intercept(reg.sakura, args, cx) {
            return true;
        }
        // While the file is scanned, a jump to a place it has not reached waits.
        if self.defer_for_scan(reg, args, cx) {
            return true;
        }
        // A jump that waited for the scan was recorded when it was asked for.
        if !std::mem::take(&mut self.replaying_scan_jump) {
            self.record_call(reg.sakura, args);
        }
        self.completion_before_command(reg.sakura);
        // `MoveCursorLayout(row, ..)` names a row of the whole text, and `MoveCursor` with a box
        // needs the row of its target: a document that is still being laid out in the background
        // has estimates for the rest.
        if matches!(reg.sakura, "F_MOVECURSORLAYOUT" | "F_MOVECURSOR") && !self.layout.is_complete()
        {
            let _ = self.layout.layout_all(self.document.read(cx).doc());
        }
        // Cleared before the command, which may leave a notice of its own (`Changes::status_message`).
        self.notice = None;
        let result = self.run_with(cx, |command_cx| reg.run(command_cx, args));
        let ok = result.is_ok();
        if let Err(e) = result {
            self.notice = Some(format!("{}: {e}", reg.id));
        }
        self.say_why_refused(cx);
        cx.notify();
        ok
    }

    /// Runs the command with the ID `id` (menus, tests, macros).
    pub fn run_command(&mut self, id: &str, args: &[Arg], cx: &mut Context<Self>) {
        match Registry::global().get(id) {
            Some(reg) => {
                self.run_registration(reg, args, cx);
            }
            None => {
                self.notice = Some(format!("no command {id}"));
                cx.notify();
            }
        }
    }

    /// Runs Sakura's function `code` the way a key or a menu item does: the core command for it if
    /// there is one, else the UI command ([`crate::ui_command`]), else a notice that it is not
    /// there yet. Returns whether a command ran and succeeded (a key macro stops at `false`).
    pub fn run_function(
        &mut self,
        code: u16,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        // Whatever runs the function (a menu, a macro), the tip goes and the wait starts again.
        self.key_help_restart(window, cx);

        if let Some(reg) = Registry::global().get_by_code(code) {
            self.run_registration(reg, args, cx)
        } else if let Some(reg) = UiRegistry::global().get_by_code(code) {
            self.run_ui_registration(reg, args, window, cx)
        } else if let Some(index) = matsu_config::settings::macros::index_of_function(code) {
            // A registered macro (`F_USERMACRO_0 + n`) is not in Sakura's table of functions.
            self.run_registered_macro(index, true, window, cx)
        } else if matsu_commands::plugin_commands::is_plugin_code(code) {
            // A command of a plugin (`F_PLUGCOMMAND_FIRST` and on) is not in the table either.
            self.run_plugin_command(code, cx)
        } else if let Some(ok) = self.run_function_statement(code, args, window, cx) {
            ok
        } else {
            let name = FUNCTIONS
                .iter()
                .find(|f| f.code == code)
                .map_or_else(|| code.to_string(), |f| f.name.to_owned());
            // A function without a command still ends the incremental search (or is taken by
            // it).
            if self.isearch_intercept(&name, args, cx) {
                return true;
            }
            self.notice = Some(format!("{name} はまだ使えない"));
            cx.notify();
            false
        }
    }

    /// Runs the UI command `reg` with `args`. A failure goes to the status bar; the result says
    /// whether the command succeeded.
    fn run_ui_registration(
        &mut self,
        reg: &'static UiRegistration,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if reg.sakura != "F_AUTOSCROLL" {
            self.autoscroll_exit();
        }
        self.interrupt_composition(cx);
        // Before the recording, as in `CViewCommander::HandleCommand`: the typed chars of the
        // incremental search are not edits, so a macro does not replay them as such.
        if self.isearch_intercept(reg.sakura, args, cx) {
            return true;
        }
        self.record_call(reg.sakura, args);
        self.completion_before_command(reg.sakura);
        // The command may leave a message of its own (a search that wrapped around), so only a
        // failure replaces it.
        self.notice = None;
        let result = reg.run(self, args, window, cx);
        let ok = result.is_ok();
        if let Err(e) = result {
            self.notice = Some(format!("{}: {e}", reg.id));
        }
        self.say_why_refused(cx);
        cx.notify();
        ok
    }

    /// Shows the message boxes that commands asked for: the caption, the text and an OK button.
    /// A box does not make a macro wait, as for `StatusMsg`.
    fn show_pending_message_boxes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending_message_boxes.is_empty() {
            return;
        }
        let texts = std::mem::take(&mut self.pending_message_boxes);
        // not while painting: a box opened there would be asked for in the middle of the frame
        cx.defer_in(window, move |_, window, cx| {
            for text in &texts {
                crate::boxes::tell(
                    window,
                    gpui::PromptLevel::Info,
                    crate::boxes::CAPTION,
                    Some(text),
                    cx,
                );
            }
        });
    }

    /// Types one character the way the keyboard does (Enter is `'\r'`).
    pub(crate) fn type_char(&mut self, c: char, cx: &mut Context<Self>) {
        self.run_command("input_char", &[Arg::Char(c)], cx);
    }

    /// What ADR-0013 section 4 wants after a command: the layout matches the text, the caret is
    /// in view, and the window repaints.
    fn after_command(&mut self, changes: &Changes, cx: &mut Context<Self>) {
        if changes.text_changed() {
            // The context has already told the layout about every edit (`RowLayout`).
            self.layout_version = self.document.update(cx, |shared, cx| shared.changed(cx));
            self.colors_after_edit(changes, self.layout_version, cx);
        }
        if changes.text_changed() || changes.caret_moved || changes.selection_changed {
            self.marked = None;
            self.notice = None;
        }
        self.refused_edit = changes.refused_in_view_mode;
        // After the clearing above: what the command says is for this very state of the text.
        if let Some(text) = &changes.status_message {
            self.notice = Some(text.clone());
        }
        if let Some(text) = changes.message_box.clone() {
            self.pending_message_boxes.push(text);
        }
        self.completion_after_command(changes, cx);
        // While a macro runs, a command that moved nothing leaves the view where it is:
        // `SetViewTop` scrolls away from the caret, and the calls after it must not scroll back
        // (Sakura scrolls to the caret when the caret moves). Outside a macro every command
        // brings the caret into view, as before.
        if !self.macros.is_suspended()
            || changes.text_changed()
            || changes.caret_moved
            || changes.selection_changed
        {
            self.ensure_caret_visible(cx);
        }
        cx.notify();
    }

    /// Another view (or code that is not a command) changed the text.
    fn document_changed(&mut self, version: u64, cx: &mut Context<Self>) {
        if version == self.layout_version {
            return; // our own change, already laid out
        }
        let doc = self.document.read(cx).doc();
        let _ = self.layout.resync(doc);
        let _ = self.layout.layout_for(doc, LAYOUT_BUDGET);
        let _ = self.cursor.clamp_if_outside(doc);
        // The text of the line may have grown under a caret that stood right of its end.
        self.cursor.set_free_col(None);
        self.layout_version = version;
        self.drop_composition(cx);
        // the candidates were made for the old text
        self.support.completion = None;
        self.clamp_scroll();
        cx.notify();
        self.run_pending_scan_jump(cx);
        self.apply_scan_restore(cx);
    }

    /// Lays out some more of a large document each frame, until it is all done.
    fn advance_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.layout.is_complete() {
            return;
        }
        let doc = self.document.read(cx).doc();
        let _ = self.layout.layout_for(doc, LAYOUT_BUDGET);
        if !self.layout.is_complete() {
            window.request_animation_frame();
        }
    }

    // ---- scrolling ----

    pub(crate) fn clamp_scroll(&mut self) {
        self.scroll_y = if self.scroll_past_end {
            self.scroll_y.max(0.0)
        } else {
            clamp_scroll_y(&self.metrics, self.layout.row_count(), self.scroll_y)
        };
        self.scroll_x = self.scroll_x.max(0.0);
    }

    /// Where the caret is on the screen: row and column.
    pub(crate) fn caret_row(&self, cx: &App) -> RowPos {
        self.layout
            .pos_to_row(self.doc(cx), self.cursor.caret())
            .unwrap_or(RowPos { row: 0, column: 0 })
    }

    /// The display column of the caret on its row. In a box selection the caret can be to the right
    /// of the end of the text (see `matsu_core::cursor::Cursor::box_caret_col`), and so can the caret of
    /// the free cursor mode (`Cursor::free_col`).
    pub(crate) fn caret_column(&self, cx: &App) -> u32 {
        let column = self.caret_row(cx).column;
        // The free cursor mode: the caret to the right of the end of the line.
        let free = self
            .cursor
            .free_col()
            .filter(|_| matsu_commands::edit_options::get().free_cursor);
        self.cursor
            .box_caret_col()
            .or(free)
            .map_or(column, |c| u32::try_from(c).unwrap_or(u32::MAX).max(column))
    }

    /// Scrolls so that the caret's row is `offset` rows below the top of the view, as Sakura's
    /// `MoveViewTopLine` does (`sakura_core/cmd/CViewCommander_Cursor.cpp`): the top is never above
    /// the first row, and it may be past the end of the scroll range. The caret does not move.
    pub fn scroll_caret_row_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let top = self.caret_row(cx).row as isize - offset as isize;
        self.move_view_top_line(top, cx);
    }

    /// Scrolls so that the caret is in view.
    pub fn ensure_caret_visible(&mut self, cx: &App) {
        let at = self.caret_row(cx);
        let m = self.metrics;
        let before = self.scroll_y;
        self.scroll_y = scroll_y_to_show_row(&m, self.scroll_y, at.row);
        if self.scroll_y != before {
            self.scroll_past_end = false;
        }
        let x = self.caret_column(cx) as f32 * m.cell_width;
        self.scroll_x = scroll_x_to_show(&m, self.scroll_x, x, x);
        self.clamp_scroll();
    }

    // ---- what the window shows about the editor ----

    pub fn status(&self, cx: &App) -> StatusInfo {
        let shared = self.document.read(cx);
        let options = crate::status_options::current(cx);
        let (line, column) = self.status_position(&options, cx);
        StatusInfo {
            line,
            column,
            char_code: self.caret_char_code(&options, cx),
            selection: self.selection_message(&options, cx),
            lines: shared.doc().line_count().max(1),
            modified: shared.model().is_modified(),
            overwrite: shared.overwrite(),
            charset: crate::status::charset_label(shared.file().encoding, shared.file().bom),
            eol: crate::status::eol_name(shared.new_line()).to_owned(),
            recording: self.is_recording_key_macro(),
            zoom: self.prefs.zoom,
            // the incremental search shows its pattern where the notice goes
            notice: self
                .isearch
                .as_ref()
                .map(|a| a.message().to_owned())
                .or_else(|| self.notice.clone())
                .or_else(|| shared.scan_text())
                .or_else(crate::settings::startup_notice),
        }
    }

    /// The title of the window while it is active (`szWinCaptionActive`).
    pub fn title(&self, cx: &App) -> String {
        self.title_for(true, cx)
    }

    /// The title of the window (`CEditWnd::UpdateCaption`): the format of the active or the
    /// inactive window, expanded for this document (`file_names::title`).
    pub fn title_for(&self, active: bool, cx: &App) -> String {
        self.expand_format(&crate::file_names::title::format(cx, active), cx)
    }

    /// `ExpandParameter` of a title or caption `format` for this document: the title of the
    /// window and the caption of the tab (`file_names::title`).
    pub(crate) fn expand_format(&self, format: &str, cx: &App) -> String {
        matsu_print::expand_parameter(format, &self.header_context(cx), None)
    }

    /// What `ExpandParameter` reads from this window: the file, the marks, the caret and the
    /// number of an untitled document. The title, the caption of the tab and the headers of the
    /// print take it.
    pub(crate) fn header_context(&self, cx: &App) -> matsu_print::HeaderContext {
        let shared = self.document.read(cx);
        let path = shared.path().map(|p| p.to_string_lossy().into_owned());
        // The key of a Grep window; its title is the key, and the tab shows `【Grep】key`.
        let grep_key = self.grep_key();
        let subject = crate::file_names::title::Subject {
            path: path.as_deref(),
            modified: shared.model().is_modified(),
            view_mode: shared.view_mode(),
            unwritable: shared.disk().unwritable,
            recording_macro: self.is_recording_key_macro(),
            grep_key: grep_key.as_deref(),
            caret: (self.cursor.caret().line, self.cursor.caret().col),
            untitled_number: shared.untitled_number(),
        };
        crate::file_names::title::context(&subject, &crate::file_names::current(cx))
    }

    // ---- keys ----

    /// Takes Tab and Shift+Tab out of the key bindings while the editor has the focus.
    /// gpui-component's `Root` binds them to "move the focus to the next control", and GPUI runs
    /// the key bindings before the key listeners, so without this the editor never sees the keys:
    /// Tab (`F_INDENT_TAB`) typed nothing and the focus went to the menu bar. Call it after
    /// `gpui_component::init`: the later binding wins, and the "Editor" context is deeper than
    /// "Root".
    pub(crate) fn bind_keys(cx: &mut App) {
        cx.bind_keys([
            gpui::KeyBinding::new("tab", gpui::NoAction, Some("Editor")),
            gpui::KeyBinding::new("shift-tab", gpui::NoAction, Some("Editor")),
        ]);
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.autoscroll_key_pressed(&event.keystroke);
        self.key_help_restart(window, cx);
        // The completion list takes its keys first (Sakura: `CHokanMgr::KeyProc`).
        if self.completion_key(event, cx) {
            cx.stop_propagation();
            return;
        }
        if event.prefer_character_input {
            return; // AltGr and the like: the character, not a command
        }
        let Some(chord) = chord_of(&event.keystroke) else {
            return;
        };
        match self.keymap.lookup(chord) {
            Lookup::Command(reg) => {
                // A held key repeats the command (`bRepeat` of `Command_UP`/`Command_DOWN`).
                for _ in 0..self.key_repeat_count(reg.sakura, event.is_held, cx) {
                    self.run_registration(reg, &[], cx);
                }
                cx.stop_propagation();
            }
            Lookup::NotImplemented { code, function } => {
                // A function that is not there yet still ends the incremental search (Esc is
                // `F_CANCEL_MODE`), and Tab and Space are chars in it.
                if let Some(f) = function
                    && self.isearch.is_some()
                    && UiRegistry::global().get_by_code(code).is_none()
                {
                    self.interrupt_composition(cx);
                    if self.isearch_intercept(f.name, &[], cx) {
                        cx.stop_propagation();
                        return;
                    }
                }
                // A function that needs the window is not a core command.
                if let Some(reg) = UiRegistry::global().get_by_code(code) {
                    self.run_ui_registration(reg, &[], window, cx);
                    cx.stop_propagation();
                    return;
                }
                // A command of a plugin runs as from a menu (`run_function`).
                if matsu_commands::plugin_commands::is_plugin_code(code) {
                    self.run_plugin_command(code, cx);
                    cx.stop_propagation();
                    return;
                }
                // A registered macro (`F_USERMACRO_0 + n`) is not in Sakura's table of functions;
                // it runs as from a menu (`run_function`).
                if let Some(index) = matsu_config::settings::macros::index_of_function(code) {
                    self.run_registered_macro(index, true, window, cx);
                    cx.stop_propagation();
                    return;
                }
                // Sakura's Tab key is `F_INDENT_TAB`, which types a tab when nothing is selected.
                // The command comes later; until then the key types the tab.
                if code == F_INDENT_TAB
                    && chord.mods == matsu_commands::keymap::Modifiers::NONE
                    && !self.cursor.has_selection()
                {
                    self.type_char('\t', cx);
                    cx.stop_propagation();
                    return;
                }
                let name = function.map_or_else(|| code.to_string(), |f| f.name.to_owned());
                self.notice = Some(format!("{name} はまだ使えない"));
                cx.notify();
                cx.stop_propagation();
            }
            Lookup::PassThrough => {}
            Lookup::Unbound => {
                // Enter has no binding in Sakura's table; the window types it (`F_WCHAR` with '\r').
                let plain = !chord.mods.ctrl && !chord.mods.alt && !chord.mods.meta;
                let typed = match chord.key {
                    Key::Enter if plain => Some('\r'),
                    _ => None,
                };
                if let Some(c) = typed {
                    self.type_char(c, cx);
                    cx.stop_propagation();
                }
            }
        }
    }
}

fn empty_layout() -> DocumentLayout {
    DocumentLayout::new(&Document::empty(), LayoutConfig::default())
        .expect("the layout of an empty document")
}

impl Focusable for EditorView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EditorView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // A tip does not outlive the focus of the view (another pane, a panel).
        if self.support.key_help.tip.is_some() && !self.focus_handle.is_focused(window) {
            self.support.key_help.tip = None;
        }
        self.show_pending_message_boxes(window, cx);
        self.follow_window_width(cx);
        self.advance_layout(window, cx);
        self.advance_colors(window, cx);
        self.settle_caret(cx);
        // macOS: whatever changed the view (a key, the mouse, a function) repaints it; the marks of
        // the native menu follow, if they differ from what it shows.
        crate::app::defer_refresh_menu_states(cx, false);
        // The print preview takes the place of the text, and the keys of the text stay out of it.
        if let Some(preview) = &self.print.preview {
            return crate::a11y::group(div().id("editor-print"), "印刷プレビュー")
                .relative()
                .size_full()
                .overflow_hidden()
                .child(preview.clone())
                .children(self.print.setup_dialog.clone())
                .children(self.input_dialog.clone())
                .children(self.connection_dialog.clone())
                .into_any_element();
        }
        let a11y_text = self.a11y_text.clone();
        let mut editor = crate::a11y::focusable_editor(div().id("editor"), &self.focus_handle)
            // The text runs, the caret and the selection, written when a screen reader is connected.
            .a11y_synthetic_children(move |builder| a11y_text::write(&a11y_text, builder))
            .size_full()
            .overflow_hidden()
            .key_context("Editor")
            .cursor(CursorStyle::IBeam)
            .font_family(self.font_family.clone())
            // `lfWeight` and `lfItalic` of the font (`ViewSettings`)
            .font_weight(gpui::FontWeight(self.prefs.settings.gpui_font_weight()))
            .text_size(px(self.font_size))
            .line_height(px((self.font_size * 1.5).round()))
            .capture_key_down(cx.listener(Self::on_key_down))
            .on_action(cx.listener(|this, _: &Undo, _, cx| this.run_command("undo", &[], cx)))
            .on_action(cx.listener(|this, _: &Redo, _, cx| this.run_command("redo", &[], cx)))
            .on_action(cx.listener(|this, a: &RunFunction, window, cx| {
                this.run_function(a.code, &[], window, cx);
            }))
            .on_action(
                cx.listener(|this, _: &crate::key_help::CopyKeyHelp, _, cx| {
                    this.copy_key_help(cx);
                }),
            )
            .on_action(
                cx.listener(|this, _: &crate::key_help::OpenKeyHelpDict, window, cx| {
                    this.open_key_help_dict(window, cx);
                }),
            )
            .on_action(cx.listener(|this, a: &OpenRecentFile, window, cx| {
                this.open_recent_file(a.path.clone(), window, cx);
            }))
            .on_action(cx.listener(|this, a: &OpenRecentFolder, window, cx| {
                this.open_recent_folder(a.path.clone(), window, cx);
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up_outside))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::on_middle_mouse_down))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::on_middle_mouse_up))
            .on_mouse_up_out(
                MouseButton::Middle,
                cx.listener(Self::on_middle_mouse_up_outside),
            )
            .on_mouse_down(
                MouseButton::Navigate(gpui::NavigationDirection::Back),
                cx.listener(Self::on_side_mouse_down),
            )
            .on_mouse_up(
                MouseButton::Navigate(gpui::NavigationDirection::Back),
                cx.listener(Self::on_side_mouse_up),
            )
            .on_mouse_down(
                MouseButton::Navigate(gpui::NavigationDirection::Forward),
                cx.listener(Self::on_side_mouse_down),
            )
            .on_mouse_up(
                MouseButton::Navigate(gpui::NavigationDirection::Forward),
                cx.listener(Self::on_side_mouse_up),
            )
            .on_mouse_down(MouseButton::Right, cx.listener(Self::on_right_mouse_down))
            .on_mouse_up(MouseButton::Right, cx.listener(Self::on_right_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .child(EditorElement::new(cx.entity()));
        if self.prefs.settings.font_italic {
            editor = editor.italic();
        }
        let editor = held_buttons::track(editor, cx);
        // The panel is a sibling, so that keys typed in its inputs do not reach the editor's.
        // The Save As options cover the view (and take the keys) while they are open.
        div()
            .size_full()
            .relative()
            .child(editor)
            .children(self.panel.clone())
            .children(self.grep.panel.clone())
            .children(self.grep_progress_bar(cx))
            .children(self.support_overlays(cx))
            .children(self.window_list.clone())
            .children(self.tab_list.clone())
            .children(self.diff.panel.clone())
            .children(self.diff.compare.clone())
            .children(self.doc_type.list.clone())
            .children(self.custom_menu_popup(window, cx))
            // Above the window list: Save from the list can ask for the options.
            .children(self.save_as_dialog.clone())
            .children(self.print.setup_dialog.clone())
            .children(self.input_dialog.clone())
            .children(self.connection_dialog.clone())
            .into_any_element()
    }
}

#[cfg(test)]
mod colors_tests;
#[cfg(test)]
mod mouse_tests;
#[cfg(test)]
mod paging_tests;
#[cfg(test)]
mod ruler_tests;
#[cfg(test)]
mod status_info_tests;
#[cfg(test)]
mod tests;
