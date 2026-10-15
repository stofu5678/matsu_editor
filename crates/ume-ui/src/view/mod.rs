//! `Entity<EditorView>`: the view layer of ADR-0013 section 3.
//!
//! It owns what belongs to one view of a document: the layout (`ume-layout`), the cursor, the
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

mod element;
mod input;
mod mouse;

use std::ops::Range;

use gpui::{
    App, Context, CursorStyle, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Render, SharedString, Styled, Subscription, Window,
    actions, div, px,
};
use ume_commands::command::{Arg, CommandError, Registration};
use ume_commands::context::{Changes, CommandContext};
use ume_commands::keymap::{Key, KeyMap, Lookup};
use ume_commands::registry::Registry;
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::{Document, Pos};
use ume_core::cursor::Cursor;
use ume_layout::{DocumentLayout, LayoutConfig, RowPos};

use crate::document::{DocumentEvent, SharedDocument};
use crate::geometry::{Metrics, clamp_scroll_y, scroll_x_to_show, scroll_y_to_show_row};
use crate::keys::chord_of;
use crate::layout_adapter::{LAYOUT_BUDGET, RowLayout};
use crate::status::StatusInfo;
use crate::ui_command::{UiRegistration, UiRegistry};

use element::EditorElement;

actions!(
    ume,
    [
        /// Edit > Undo
        Undo,
        /// Edit > Redo
        Redo,
    ]
);

/// The font of the text, by OS: a Japanese monospaced font, so that full-width characters are
/// twice as wide as half-width ones. GPUI picks a font by family name and falls back to its
/// default when the font is missing (the PoC's list of fonts is in `poc/gpui-textview`).
#[cfg(any(windows, target_os = "macos"))]
pub const FONT_FAMILY: &str = "BIZ UDGothic";
#[cfg(not(any(windows, target_os = "macos")))]
pub const FONT_FAMILY: &str = "Noto Sans Mono CJK JP";

const FONT_SIZE: f32 = 14.0;

/// `F_INDENT_TAB`, the function of the Tab key (`sakura_core/func/CKeyBind.cpp`).
const F_INDENT_TAB: u16 = 30260;

/// The text of an IME composition: it is in the document, on one line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Marked {
    pub line: usize,
    /// Byte columns of the line.
    pub range: Range<usize>,
}

/// What the mouse is doing between button down and button up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Drag {
    None,
    /// Selecting from `anchor` to the pointer.
    Select {
        anchor: Pos,
    },
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
    pub(crate) cursor: Cursor,
    pub(crate) keymap: KeyMap,
    pub(crate) focus_handle: FocusHandle,
    pub(crate) scroll_x: f32,
    pub(crate) scroll_y: f32,
    pub(crate) metrics: Metrics,
    pub(crate) paint: Option<PaintInfo>,
    /// The widest end column among the rows of the last paint, for the horizontal scroll range.
    pub(crate) widest_cols: u32,
    pub(crate) marked: Option<Marked>,
    /// An undo block for the composition is open (it closes when the text is committed).
    pub(crate) composing: bool,
    pub(crate) drag: Drag,
    pub(crate) notice: Option<String>,
    font_family: SharedString,
    font_size: f32,
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
        });
        Self {
            document,
            layout,
            layout_version,
            cursor: Cursor::default(),
            keymap: KeyMap::default(),
            focus_handle: cx.focus_handle(),
            scroll_x: 0.0,
            scroll_y: 0.0,
            metrics: Metrics::default(),
            paint: None,
            widest_cols: 0,
            marked: None,
            composing: false,
            drag: Drag::None,
            notice: None,
            font_family: FONT_FAMILY.into(),
            font_size: FONT_SIZE,
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
        let layout = &mut self.layout;
        let cursor = &mut self.cursor;
        let (result, changes) = document.update(cx, |shared, cx| {
            let (model, path) = shared.parts_mut();
            let mut adapter = RowLayout(layout);
            let mut clipboard = crate::clipboard::GpuiClipboard(cx);
            let mut command_cx = CommandContext::new(model, cursor, &mut adapter, path)
                .with_clipboard(&mut clipboard);
            let result = f(&mut command_cx);
            (result, command_cx.finish())
        });
        self.after_command(&changes, cx);
        result
    }

    /// Runs the command `reg` with `args`. A failure goes to the status bar.
    pub(crate) fn run_registration(
        &mut self,
        reg: &'static Registration,
        args: &[Arg],
        cx: &mut Context<Self>,
    ) {
        self.interrupt_composition(cx);
        let result = self.run_with(cx, |command_cx| reg.run(command_cx, args));
        self.notice = result
            .err()
            .map(|e: CommandError| format!("{}: {e}", reg.id));
        cx.notify();
    }

    /// Runs the command with the ID `id` (menus, tests, macros).
    pub fn run_command(&mut self, id: &str, args: &[Arg], cx: &mut Context<Self>) {
        match Registry::global().get(id) {
            Some(reg) => self.run_registration(reg, args, cx),
            None => {
                self.notice = Some(format!("no command {id}"));
                cx.notify();
            }
        }
    }

    /// Runs Sakura's function `code` the way a key or a menu item does: the core command for it if
    /// there is one, else the UI command ([`crate::ui_command`]), else a notice that it is not
    /// there yet.
    pub fn run_function(
        &mut self,
        code: u16,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(reg) = Registry::global().get_by_code(code) {
            self.run_registration(reg, args, cx);
        } else if let Some(reg) = UiRegistry::global().get_by_code(code) {
            self.run_ui_registration(reg, args, window, cx);
        } else {
            let name = FUNCTIONS
                .iter()
                .find(|f| f.code == code)
                .map_or_else(|| code.to_string(), |f| f.name.to_owned());
            self.notice = Some(format!("{name} はまだ使えない"));
            cx.notify();
        }
    }

    /// Runs the UI command `reg` with `args`. A failure goes to the status bar.
    fn run_ui_registration(
        &mut self,
        reg: &'static UiRegistration,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.interrupt_composition(cx);
        let result = reg.run(self, args, window, cx);
        self.notice = result.err().map(|e| format!("{}: {e}", reg.id));
        cx.notify();
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
        }
        if changes.text_changed() || changes.caret_moved || changes.selection_changed {
            self.marked = None;
            self.notice = None;
        }
        self.ensure_caret_visible(cx);
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
        let _ = self.cursor.clamp(doc);
        self.layout_version = version;
        self.interrupt_composition(cx);
        self.clamp_scroll();
        cx.notify();
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
        self.scroll_y = clamp_scroll_y(&self.metrics, self.layout.row_count(), self.scroll_y);
        self.scroll_x = self.scroll_x.max(0.0);
    }

    /// Where the caret is on the screen: row and column.
    pub(crate) fn caret_row(&self, cx: &App) -> RowPos {
        self.layout
            .pos_to_row(self.doc(cx), self.cursor.caret())
            .unwrap_or(RowPos { row: 0, column: 0 })
    }

    /// Scrolls so that the caret's row is `offset` rows below the top of the view, as Sakura's
    /// `MoveViewTopLine` does (`sakura_core/cmd/CViewCommander_Cursor.cpp`): the top is never above
    /// the first row. The caret does not move.
    pub fn scroll_caret_row_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let top = self.caret_row(cx).row.saturating_sub(offset);
        self.scroll_y = top as f32 * self.metrics.line_height;
        self.clamp_scroll();
        cx.notify();
    }

    /// Scrolls so that the caret is in view.
    pub fn ensure_caret_visible(&mut self, cx: &App) {
        let at = self.caret_row(cx);
        let m = self.metrics;
        self.scroll_y = scroll_y_to_show_row(&m, self.scroll_y, at.row);
        let x = at.column as f32 * m.cell_width;
        self.scroll_x = scroll_x_to_show(&m, self.scroll_x, x, x);
        self.clamp_scroll();
    }

    // ---- what the window shows about the editor ----

    pub fn status(&self, cx: &App) -> StatusInfo {
        let shared = self.document.read(cx);
        StatusInfo {
            line: self.cursor.caret().line + 1,
            column: self.caret_row(cx).column + 1,
            lines: shared.doc().line_count().max(1),
            modified: shared.model().is_modified(),
            notice: self.notice.clone(),
        }
    }

    pub fn title(&self, cx: &App) -> String {
        let shared = self.document.read(cx);
        crate::status::window_title(shared.file_name().as_deref(), shared.model().is_modified())
    }

    // ---- keys ----

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.prefer_character_input {
            return; // AltGr and the like: the character, not a command
        }
        let Some(chord) = chord_of(&event.keystroke) else {
            return;
        };
        match self.keymap.lookup(chord) {
            Lookup::Command(reg) => {
                self.run_registration(reg, &[], cx);
                cx.stop_propagation();
            }
            Lookup::NotImplemented { code, function } => {
                // A function that needs the window is not a core command.
                if let Some(reg) = UiRegistry::global().get_by_code(code) {
                    self.run_ui_registration(reg, &[], window, cx);
                    cx.stop_propagation();
                    return;
                }
                // Sakura's Tab key is `F_INDENT_TAB`, which types a tab when nothing is selected.
                // The command comes later; until then the key types the tab.
                if code == F_INDENT_TAB
                    && chord.mods == ume_commands::keymap::Modifiers::NONE
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
        self.advance_layout(window, cx);
        div()
            .id("editor")
            .size_full()
            .overflow_hidden()
            .key_context("Editor")
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::IBeam)
            .font_family(self.font_family.clone())
            .text_size(px(self.font_size))
            .line_height(px((self.font_size * 1.5).round()))
            .capture_key_down(cx.listener(Self::on_key_down))
            .on_action(cx.listener(|this, _: &Undo, _, cx| this.run_command("undo", &[], cx)))
            .on_action(cx.listener(|this, _: &Redo, _, cx| this.run_command("redo", &[], cx)))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .child(EditorElement::new(cx.entity()))
    }
}

#[cfg(test)]
mod tests;
