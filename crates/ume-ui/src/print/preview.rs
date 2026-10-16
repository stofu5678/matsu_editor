//! The print preview (ADR-0011): draws the pages of the shared layout (`ume-print`) with GPUI, as
//! Sakura's print preview does (`sakura_core/print/CPrintPreview.cpp`). It takes the place of the
//! text in the window, under a bar of buttons like Sakura's (`IDD_PRINTPREVIEWBAR` in
//! `sakura_core/sakura_rc.rc`). Ported from the PoC of 2026-10-11
//! (`poc/gpui-textview/src/print/preview.rs`).
//!
//! The preview owns a snapshot of the document ([`PrintJob`]), so the pages do not change under
//! it. It does not print or close itself: it tells whoever shows it ([`PreviewEvent`]).

use gpui::{
    App, BorderStyle, Bounds, ContentMask, Context, Element, ElementId, Entity, EventEmitter,
    FocusHandle, Focusable, Font, FontFeatures, GlobalElementId, InspectorElementId, IntoElement,
    KeyDownEvent, LayoutId, Pixels, Point, Render, ScrollWheelEvent, SharedString, Size, Style,
    TextRun, Window, black, div, fill, font, outline, point, prelude::*, px, relative, rgb, size,
};
use gpui_component::button::Button;
use gpui_component::{Disableable as _, Selectable as _, Sizable as _, h_flex};
use ume_platform::print::PrintJob;
use ume_platform::print::draw::{cell_positions, font_spec, on_grid};
use ume_print::{Layout, Orientation, Paper, PlacedText, PrintError, find_paper_info};

/// What the preview asks of whoever shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewEvent {
    /// The user wants to print (the button, `P`, Ctrl+P).
    Print,
    /// The user leaves the preview (the button, `Q`, Esc).
    Close,
}

/// Logical pixels in an inch at 100 %: points on a Mac, and Windows' 96 elsewhere.
const PIXELS_PER_INCH: f32 = if cfg!(target_os = "macos") { 72. } else { 96. };
/// Sakura's zoom: from 10 % to 400 %, in steps of 10 (`CPrintPreview::OnPreviewZoom`).
pub const ZOOM_MIN: i32 = 10;
pub const ZOOM_MAX: i32 = 400;
/// The space around the paper: 8 mm (`m_nPreview_ViewMarginLeft` and `m_nPreview_ViewMarginTop`).
const VIEW_MARGIN: i32 = 80;

/// A label with its access key, as Windows shows it: "印刷(P)". macOS has none.
fn label(name: &str, key: char) -> String {
    if cfg!(target_os = "macos") {
        name.to_owned()
    } else {
        format!("{name}({key})")
    }
}

pub struct PrintPreview {
    focus_handle: FocusHandle,
    job: PrintJob,
    layout: Result<Layout, PrintError>,
    page: usize,
    zoom: i32,
    /// How far the paper is scrolled.
    scroll: Point<Pixels>,
    /// The size of the view at the last paint, which limits the scrolling.
    view: Size<Pixels>,
    /// What became of the last print, for the bar.
    pub notice: Option<SharedString>,
}

impl EventEmitter<PreviewEvent> for PrintPreview {}

impl PrintPreview {
    pub fn new(job: PrintJob, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            focus_handle: cx.focus_handle(),
            job,
            layout: Err(PrintError::NoPrintableArea),
            page: 0,
            zoom: 100,
            scroll: Point::default(),
            view: Size::default(),
            notice: None,
        };
        this.relayout();
        this
    }

    /// Lays the document out again, as Sakura does when the setting changes
    /// (`CPrintPreview::OnChangePrintSetting`).
    fn relayout(&mut self) {
        self.layout = self.job.layout(self.job.setting_paper());
        self.page = self.page.min(self.page_count().saturating_sub(1));
    }

    pub fn page_count(&self) -> usize {
        self.layout.as_ref().map_or(0, Layout::page_count)
    }

    /// The page shown (from 0) and the zoom in percent.
    pub fn state(&self) -> (usize, i32) {
        (self.page, self.zoom)
    }

    pub fn job(&self) -> &PrintJob {
        &self.job
    }

    /// What printing needs, or why there is nothing to print.
    pub fn printable(&self) -> Result<PrintJob, SharedString> {
        match &self.layout {
            Err(err) => Err(err.to_string().into()),
            // Sakura's STR_ERR_DLGPRNPRVW7.
            Ok(layout) if layout.page_count() == 0 => Err("印刷するページがありません。".into()),
            Ok(_) => Ok(self.job.clone()),
        }
    }

    /// The paper as the bar shows it, such as "A4  縦" (Sakura's `STR_ERR_DLGPRNPRVW1` style).
    pub fn paper_name(&self) -> String {
        let setting = &self.job.setting;
        let name = find_paper_info(setting.print_paper_size).map_or("", |info| info.name);
        let orientation = match setting.print_paper_orientation {
            Orientation::Portrait => "縦",
            Orientation::Landscape => "横",
        };
        format!("{name}  {orientation}")
    }

    pub fn prev_page(&mut self, cx: &mut Context<Self>) {
        if self.page > 0 {
            self.page -= 1;
            cx.notify();
        }
    }

    pub fn next_page(&mut self, cx: &mut Context<Self>) {
        if self.page + 1 < self.page_count() {
            self.page += 1;
            cx.notify();
        }
    }

    pub fn zoom_in(&mut self, cx: &mut Context<Self>) {
        self.zoom = (self.zoom + 10).min(ZOOM_MAX);
        cx.notify();
    }

    pub fn zoom_out(&mut self, cx: &mut Context<Self>) {
        self.zoom = (self.zoom - 10).max(ZOOM_MIN);
        // Sakura goes back to the top-left corner when it zooms out.
        self.scroll = Point::default();
        cx.notify();
    }

    /// Prints or not the line numbers. Sakura changes this in the print setting dialog; the
    /// preview has the button to try the layout until that dialog is made.
    pub fn toggle_line_number(&mut self, cx: &mut Context<Self>) {
        self.job.setting.print_line_number = !self.job.setting.print_line_number;
        self.relayout();
        cx.notify();
    }

    /// One column or two (段組), likewise.
    pub fn toggle_columns(&mut self, cx: &mut Context<Self>) {
        let setting = &mut self.job.setting;
        setting.print_dansuu = if setting.print_dansuu == 1 { 2 } else { 1 };
        self.relayout();
        cx.notify();
    }

    fn on_scroll(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(20.));
        self.scroll = point(self.scroll.x - delta.x, self.scroll.y - delta.y);
        self.clamp_scroll();
        cx.notify();
    }

    /// The keys of the preview: the arrows and page keys, zoom, Esc, and the access keys of
    /// Sakura's bar (P, V, N, O, I, Q), which work without Alt.
    fn on_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let k = &event.keystroke;
        let command = k.modifiers.secondary();
        let plain = !k.modifiers.control && !k.modifiers.alt && !k.modifiers.platform;
        let handled = match k.key.as_str() {
            "left" | "pageup" => {
                self.prev_page(cx);
                true
            }
            "right" | "pagedown" => {
                self.next_page(cx);
                true
            }
            "escape" => {
                cx.emit(PreviewEvent::Close);
                true
            }
            // Cmd on a Mac, Ctrl elsewhere; on a Mac Ctrl too, as the font size keys.
            "=" | "+" if command || k.modifiers.control => {
                self.zoom_in(cx);
                true
            }
            "-" if command || k.modifiers.control => {
                self.zoom_out(cx);
                true
            }
            "p" if command => {
                cx.emit(PreviewEvent::Print);
                true
            }
            "p" if plain => {
                cx.emit(PreviewEvent::Print);
                true
            }
            "v" if plain => {
                self.prev_page(cx);
                true
            }
            "n" if plain => {
                self.next_page(cx);
                true
            }
            "o" if plain => {
                self.zoom_out(cx);
                true
            }
            "i" if plain => {
                self.zoom_in(cx);
                true
            }
            "q" if plain => {
                cx.emit(PreviewEvent::Close);
                true
            }
            _ => false,
        };
        if handled {
            cx.stop_propagation();
        }
    }

    /// Pixels in 0.1 mm.
    fn scale(&self) -> f32 {
        self.zoom as f32 / 100. * PIXELS_PER_INCH / 254.
    }

    fn paper(&self) -> Paper {
        self.layout
            .as_ref()
            .map_or_else(|_| self.job.setting_paper(), Layout::paper)
    }

    fn clamp_scroll(&mut self) {
        let scale = self.scale();
        let paper = self.paper();
        let content = size(
            px((paper.width + VIEW_MARGIN * 2) as f32 * scale),
            px((paper.height + VIEW_MARGIN * 2) as f32 * scale),
        );
        let max = point(
            (content.width - self.view.width).max(px(0.)),
            (content.height - self.view.height).max(px(0.)),
        );
        self.scroll = point(
            self.scroll.x.clamp(px(0.), max.x),
            self.scroll.y.clamp(px(0.), max.y),
        );
    }
}

impl Focusable for PrintPreview {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PrintPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pages = self.page_count();
        let status: SharedString = match &self.layout {
            Err(err) => err.to_string().into(),
            Ok(_) => self.notice.clone().unwrap_or_default(),
        };
        let button = |id: &'static str, text: String| {
            Button::new(id)
                .small()
                .compact()
                .tab_stop(false)
                .label(text)
        };
        let bar = h_flex()
            .gap_1()
            .px_2()
            .py_1()
            .bg(rgb(0xf0f0f0))
            .border_b_1()
            .border_color(rgb(0xd0d0d0))
            .text_size(px(12.))
            .child(
                button("print", format!("{}...", label("印刷", 'P')))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(PreviewEvent::Print))),
            )
            .child(
                button("prev", label("前", 'V'))
                    .disabled(self.page == 0)
                    .on_click(cx.listener(|this, _, _, cx| this.prev_page(cx))),
            )
            .child(
                button("next", label("次", 'N'))
                    .disabled(self.page + 1 >= pages)
                    .on_click(cx.listener(|this, _, _, cx| this.next_page(cx))),
            )
            // Sakura's STR_ERR_DLGPRNPRVW6.
            .child(
                div()
                    .min_w(px(56.))
                    .child(format!("{}/{}頁", self.page + 1, pages)),
            )
            .child(
                button("zoom-out", label("縮小", 'O'))
                    .disabled(self.zoom <= ZOOM_MIN)
                    .on_click(cx.listener(|this, _, _, cx| this.zoom_out(cx))),
            )
            .child(
                button("zoom-in", label("拡大", 'I'))
                    .disabled(self.zoom >= ZOOM_MAX)
                    .on_click(cx.listener(|this, _, _, cx| this.zoom_in(cx))),
            )
            .child(div().min_w(px(44.)).child(format!("{} %", self.zoom)))
            // Settings that Sakura's 印刷ページ設定 has, to try the layout with.
            .child(
                button("line-number", "行番号".into())
                    .selected(self.job.setting.print_line_number)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_line_number(cx))),
            )
            .child(
                button("columns", "2段組".into())
                    .selected(self.job.setting.print_dansuu == 2)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_columns(cx))),
            )
            .child(
                button("close", label("戻る", 'Q'))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(PreviewEvent::Close))),
            )
            .child(div().text_color(rgb(0x555555)).child(self.paper_name()))
            .child(div().text_color(rgb(0xb00000)).child(status));
        div()
            .id("print-preview")
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xffffff))
            .key_context("PrintPreview")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
            .child(bar)
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .on_scroll_wheel(cx.listener(Self::on_scroll))
                    .child(PageView {
                        preview: cx.entity(),
                    }),
            )
    }
}

/// The paper of the current page, on gray as in Sakura.
struct PageView {
    preview: Entity<PrintPreview>,
}

impl IntoElement for PageView {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for PageView {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.preview
            .update(cx, |preview, _| preview.view = bounds.size);
        // Sakura's GRAY_BRUSH.
        window.paint_quad(fill(bounds, rgb(0x808080)));
        let preview = self.preview.read(cx);
        let scale = preview.scale();
        let to_px = |v: i32| px(v as f32 * scale);
        let paper = preview.paper();
        let paper_size = size(to_px(paper.width), to_px(paper.height));
        let margin = to_px(VIEW_MARGIN);
        // The paper is centered while it fits across, and scrolls when it does not.
        let left = if paper_size.width + margin * 2. <= bounds.size.width {
            bounds.left() + (bounds.size.width - paper_size.width) / 2.
        } else {
            bounds.left() + margin - preview.scroll.x
        };
        let origin = point(left, bounds.top() + margin - preview.scroll.y);
        let at = |x: i32, y: i32| point(origin.x + to_px(x), origin.y + to_px(y));
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            let sheet = Bounds::new(origin, paper_size);
            window.paint_quad(fill(sheet, rgb(0xffffff)));
            window.paint_quad(outline(sheet, rgb(0x000000), BorderStyle::Solid));
            let Ok(layout) = &preview.layout else {
                return;
            };
            let s = layout.setting();
            // The margins, in gray as Sakura draws them.
            window.paint_quad(outline(
                Bounds::from_corners(
                    at(s.print_margin_lx, s.print_margin_ty),
                    at(
                        paper.width - s.print_margin_rx,
                        paper.height - s.print_margin_by,
                    ),
                ),
                rgb(0x808080),
                BorderStyle::Solid,
            ));
            let Some(page) = layout.page(preview.page, &*preview.job.source, &preview.job.header)
            else {
                return;
            };
            for text in &page.texts {
                paint_text(text, layout, scale, &at, window);
            }
            for rule in &page.rules {
                let (from, to) = (at(rule.from.0, rule.from.1), at(rule.to.0, rule.to.1));
                window.paint_quad(fill(
                    Bounds::from_corners(from, point(to.x + px(1.), to.y + px(1.))),
                    rgb(0x000000),
                ));
            }
        });
    }
}

/// A font for printing, without ligatures, so that each character keeps its own glyph.
fn print_font(family: &str) -> Font {
    Font {
        features: FontFeatures::disable_ligatures(),
        ..font(family.to_owned())
    }
}

/// Paints text: GPUI shapes it, picking fallback fonts for characters that the font lacks. On
/// the grid, each glyph goes to the cell of its character; a header or a footer in a font of its
/// own goes from its `x` with the advances of the font, from the left (centering needs its
/// width, a follow-up).
fn paint_text(
    text: &PlacedText,
    layout: &Layout,
    scale: f32,
    at: &impl Fn(i32, i32) -> Point<Pixels>,
    window: &mut Window,
) {
    let spec = font_spec(layout.setting(), text.font);
    let font_size = px(spec.height as f32 * scale);
    let runs = [TextRun {
        len: text.text.len(),
        font: print_font(spec.face),
        color: black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    }];
    let line = window
        .text_system()
        .layout_line(&text.text, font_size, &runs, None);
    let grid = on_grid(text.font);
    let xs: Vec<(usize, i32)> = if grid {
        cell_positions(text, layout)
            .map(|(i, _, x)| (i, x))
            .collect()
    } else {
        Vec::new()
    };
    for run in &line.runs {
        for glyph in &run.glyphs {
            let origin = if grid {
                let x = match xs.binary_search_by_key(&glyph.index, |&(i, _)| i) {
                    Ok(k) => xs[k].1,
                    Err(k) => xs[k.saturating_sub(1)].1,
                };
                at(x, text.baseline)
            } else {
                at(text.x, text.baseline) + point(glyph.position.x, px(0.))
            };
            // The vertical offset of shaping (combining marks), as GPUI's text painter adds it.
            let origin = origin + point(px(0.), glyph.position.y);
            if glyph.is_emoji {
                window
                    .paint_emoji(origin, run.font_id, glyph.id, font_size)
                    .ok();
            } else {
                window
                    .paint_glyph(origin, run.font_id, glyph.id, font_size, black())
                    .ok();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use gpui::TestAppContext;
    use ume_platform::print::default_setting;
    use ume_print::{HeaderContext, TypeOptions};

    use super::*;

    fn job(lines: usize) -> PrintJob {
        let mut text: Vec<String> = (0..lines).map(|i| format!("line {i}")).collect();
        text.push(String::new());
        PrintJob {
            source: Arc::new(text),
            setting: default_setting(),
            options: TypeOptions::default(),
            header: HeaderContext::default(),
            title: "t".into(),
        }
    }

    #[gpui::test]
    fn the_pages_turn_and_stop_at_the_ends(cx: &mut TestAppContext) {
        let preview = cx.new(|cx| PrintPreview::new(job(200), cx));
        let pages = preview.read_with(cx, |p, _| p.page_count());
        assert!(pages >= 3, "200 lines are {pages} pages");
        preview.update(cx, |p, cx| {
            p.prev_page(cx);
            assert_eq!(p.state().0, 0);
            p.next_page(cx);
            p.next_page(cx);
            assert_eq!(p.state().0, 2);
            for _ in 0..pages {
                p.next_page(cx);
            }
            assert_eq!(p.state().0, pages - 1);
        });
    }

    #[gpui::test]
    fn the_zoom_stays_between_sakuras_limits(cx: &mut TestAppContext) {
        let preview = cx.new(|cx| PrintPreview::new(job(3), cx));
        preview.update(cx, |p, cx| {
            for _ in 0..100 {
                p.zoom_in(cx);
            }
            assert_eq!(p.state().1, ZOOM_MAX);
            for _ in 0..100 {
                p.zoom_out(cx);
            }
            assert_eq!(p.state().1, ZOOM_MIN);
        });
    }

    #[gpui::test]
    fn the_settings_of_the_bar_lay_the_pages_out_again(cx: &mut TestAppContext) {
        let preview = cx.new(|cx| PrintPreview::new(job(120), cx));
        let before = preview.read_with(cx, |p, _| p.page_count());
        preview.update(cx, |p, cx| {
            p.next_page(cx);
            p.toggle_columns(cx);
        });
        let (columns, page) = preview.read_with(cx, |p, _| (p.page_count(), p.state().0));
        // Two columns hold twice the rows, so there are fewer pages, and the page shown is kept
        // in range.
        assert!(
            columns < before,
            "{columns} pages in two columns, {before} in one"
        );
        assert!(page < columns);
        preview.update(cx, |p, cx| p.toggle_columns(cx));
        assert_eq!(preview.read_with(cx, |p, _| p.page_count()), before);
    }

    #[gpui::test]
    fn an_empty_document_cannot_be_printed(cx: &mut TestAppContext) {
        let preview = cx.new(|cx| PrintPreview::new(job(0), cx));
        // Sakura's STR_ERR_DLGPRNPRVW7.
        let err = preview.read_with(cx, |p, _| p.printable().unwrap_err());
        assert_eq!(err.as_ref(), "印刷するページがありません。");
    }

    #[gpui::test]
    fn the_paper_is_named_like_the_bar_of_sakura(cx: &mut TestAppContext) {
        let preview = cx.new(|cx| PrintPreview::new(job(1), cx));
        let name = preview.read_with(cx, |p, _| p.paper_name());
        assert!(name.starts_with("A4"), "{name}");
        assert!(name.ends_with("縦"), "{name}");
    }
}
