//! Print preview (ADR-0011): draws the pages of the shared layout with GPUI, as Sakura's print
//! preview does (sakura_core/print/CPrintPreview.cpp). It takes the place of the text in the
//! window, under a bar of buttons like Sakura's (IDD_PRINTPREVIEWBAR in sakura_rc.rc).

use gpui::{
    App, BorderStyle, Bounds, ContentMask, Context, Element, ElementId, Entity, FocusHandle,
    Focusable, Font, FontFeatures, GlobalElementId, InspectorElementId, IntoElement, KeyBinding,
    LayoutId, Pixels, Point, Render, ScrollWheelEvent, SharedString, Size, Style, TextRun, Window,
    actions, black, div, fill, font, outline, point, prelude::*, px, relative, rgb, size,
};
use gpui_component::button::Button;
use gpui_component::{Disableable as _, Selectable as _, Sizable as _, h_flex};

use super::layout::{DocInfo, Layout, NoPrintableArea, Paper, PlacedText, PrintSetting, font_runs};
use super::{PrintJob, default_setting, paper};
use crate::{ClosePreview, Editor, Print, label};

actions!(
    print_preview,
    [
        PrevPage,
        NextPage,
        ZoomIn,
        ZoomOut,
        ToggleLineNumber,
        ToggleColumns
    ]
);

const CONTEXT: &str = "PrintPreview";

/// Logical pixels in an inch at 100 %: points on a Mac, and Windows' 96 elsewhere.
const PIXELS_PER_INCH: f32 = if cfg!(target_os = "macos") { 72. } else { 96. };
/// Sakura's zoom: from 10 % to 400 %, in steps of 10 (CPrintPreview::OnPreviewZoom).
const ZOOM_MIN: i32 = 10;
const ZOOM_MAX: i32 = 400;
/// The space around the paper: 8 mm (m_nPreview_ViewMarginLeft and m_nPreview_ViewMarginTop).
const VIEW_MARGIN: i32 = 80;

/// Binds the preview's keys. Call it after the app's own keys, so that these win over them.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", PrevPage, Some(CONTEXT)),
        KeyBinding::new("pageup", PrevPage, Some(CONTEXT)),
        KeyBinding::new("right", NextPage, Some(CONTEXT)),
        KeyBinding::new("pagedown", NextPage, Some(CONTEXT)),
        KeyBinding::new("secondary-=", ZoomIn, Some(CONTEXT)),
        KeyBinding::new("secondary-+", ZoomIn, Some(CONTEXT)),
        KeyBinding::new("secondary--", ZoomOut, Some(CONTEXT)),
        KeyBinding::new("escape", ClosePreview, Some(CONTEXT)),
        // The access keys of Sakura's preview bar, which work there without Alt.
        KeyBinding::new("p", Print, Some(CONTEXT)),
        KeyBinding::new("v", PrevPage, Some(CONTEXT)),
        KeyBinding::new("n", NextPage, Some(CONTEXT)),
        KeyBinding::new("o", ZoomOut, Some(CONTEXT)),
        KeyBinding::new("i", ZoomIn, Some(CONTEXT)),
        KeyBinding::new("q", ClosePreview, Some(CONTEXT)),
    ]);
    // On a Mac, with Control as well as Cmd, as the font size keys.
    #[cfg(target_os = "macos")]
    cx.bind_keys([
        KeyBinding::new("ctrl-=", ZoomIn, Some(CONTEXT)),
        KeyBinding::new("ctrl-+", ZoomIn, Some(CONTEXT)),
        KeyBinding::new("ctrl--", ZoomOut, Some(CONTEXT)),
    ]);
}

pub struct PrintPreview {
    focus_handle: FocusHandle,
    editor: Entity<Editor>,
    setting: PrintSetting,
    paper: Paper,
    /// The paper as the bar shows it, such as "A4  縦".
    paper_name: String,
    layout: Result<Layout, NoPrintableArea>,
    page: usize,
    zoom: i32,
    /// How far the paper is scrolled.
    scroll: Point<Pixels>,
    /// The size of the view at the last paint, which limits the scrolling.
    view: Size<Pixels>,
    /// What became of the last print, for the bar.
    pub notice: Option<SharedString>,
}

impl PrintPreview {
    pub fn new(editor: Entity<Editor>, cx: &mut Context<Self>) -> Self {
        let (paper, paper_name) = paper();
        let mut this = Self {
            focus_handle: cx.focus_handle(),
            editor,
            setting: default_setting(),
            paper,
            paper_name,
            layout: Err(NoPrintableArea),
            page: 0,
            zoom: 100,
            scroll: Point::default(),
            view: Size::default(),
            notice: None,
        };
        this.relayout(cx);
        this
    }

    /// Lays the editor's text out again, as Sakura does when the setting changes
    /// (CPrintPreview::OnChangePrintSetting).
    fn relayout(&mut self, cx: &App) {
        self.layout = Layout::new(&self.editor.read(cx).lines, &self.setting, self.paper);
        self.page = self.page.min(self.page_count().saturating_sub(1));
    }

    pub fn page_count(&self) -> usize {
        self.layout.as_ref().map_or(0, Layout::page_count)
    }

    /// The page shown, the zoom and the setting, which the self-check looks at.
    #[cfg(feature = "self-check")]
    pub fn state(&self) -> (usize, i32, &PrintSetting) {
        (self.page, self.zoom, &self.setting)
    }

    /// What printing needs, or why there is nothing to print.
    pub fn job(&self, cx: &App) -> Result<PrintJob, SharedString> {
        match &self.layout {
            Err(err) => Err(err.to_string().into()),
            // Sakura's STR_ERR_DLGPRNPRVW7.
            Ok(layout) if layout.page_count() == 0 => Err("印刷するページがありません。".into()),
            Ok(_) => {
                let editor = self.editor.read(cx);
                Ok(PrintJob {
                    lines: editor.lines.clone(),
                    setting: self.setting.clone(),
                    doc: doc_info(editor),
                })
            }
        }
    }

    pub fn prev_page(&mut self, _: &PrevPage, _: &mut Window, cx: &mut Context<Self>) {
        if self.page > 0 {
            self.page -= 1;
            cx.notify();
        }
    }

    pub fn next_page(&mut self, _: &NextPage, _: &mut Window, cx: &mut Context<Self>) {
        if self.page + 1 < self.page_count() {
            self.page += 1;
            cx.notify();
        }
    }

    pub fn zoom_in(&mut self, _: &ZoomIn, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom = (self.zoom + 10).min(ZOOM_MAX);
        cx.notify();
    }

    pub fn zoom_out(&mut self, _: &ZoomOut, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom = (self.zoom - 10).max(ZOOM_MIN);
        // Sakura goes back to the top-left corner when it zooms out.
        self.scroll = Point::default();
        cx.notify();
    }

    pub fn toggle_line_number(
        &mut self,
        _: &ToggleLineNumber,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.setting.line_number = !self.setting.line_number;
        self.relayout(cx);
        cx.notify();
    }

    pub fn toggle_columns(&mut self, _: &ToggleColumns, _: &mut Window, cx: &mut Context<Self>) {
        self.setting.columns = if self.setting.columns == 1 { 2 } else { 1 };
        self.relayout(cx);
        cx.notify();
    }

    fn on_scroll(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(20.));
        self.scroll = point(self.scroll.x - delta.x, self.scroll.y - delta.y);
        self.clamp_scroll();
        cx.notify();
    }

    /// Pixels in 0.1 mm.
    fn scale(&self) -> f32 {
        self.zoom as f32 / 100. * PIXELS_PER_INCH / 254.
    }

    fn clamp_scroll(&mut self) {
        let scale = self.scale();
        let content = size(
            px((self.paper.width + VIEW_MARGIN * 2) as f32 * scale),
            px((self.paper.height + VIEW_MARGIN * 2) as f32 * scale),
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

/// What the header may show of the editor's document.
fn doc_info(editor: &Editor) -> DocInfo {
    DocInfo {
        file_name: editor
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned()),
        path: editor.path.as_ref().map(|path| path.display().to_string()),
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
                    .on_click(|_, window, cx| window.dispatch_action(Box::new(Print), cx)),
            )
            .child(
                button("prev", label("前", 'V'))
                    .disabled(self.page == 0)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.prev_page(&PrevPage, window, cx)),
                    ),
            )
            .child(
                button("next", label("次", 'N'))
                    .disabled(self.page + 1 >= pages)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.next_page(&NextPage, window, cx)),
                    ),
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
                    .on_click(
                        cx.listener(|this, _, window, cx| this.zoom_out(&ZoomOut, window, cx)),
                    ),
            )
            .child(
                button("zoom-in", label("拡大", 'I'))
                    .disabled(self.zoom >= ZOOM_MAX)
                    .on_click(cx.listener(|this, _, window, cx| this.zoom_in(&ZoomIn, window, cx))),
            )
            .child(div().min_w(px(44.)).child(format!("{} %", self.zoom)))
            // Settings that Sakura's 印刷ページ設定 has, to try the layout with.
            .child(
                button("line-number", "行番号".into())
                    .selected(self.setting.line_number)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.toggle_line_number(&ToggleLineNumber, window, cx)
                    })),
            )
            .child(
                button("columns", "2段組".into())
                    .selected(self.setting.columns == 2)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.toggle_columns(&ToggleColumns, window, cx)
                    })),
            )
            .child(
                button("close", label("戻る", 'Q'))
                    .on_click(|_, window, cx| window.dispatch_action(Box::new(ClosePreview), cx)),
            )
            .child(
                div()
                    .text_color(rgb(0x555555))
                    .child(self.paper_name.clone()),
            )
            .child(div().text_color(rgb(0xb00000)).child(status));
        div()
            .flex()
            .flex_col()
            .size_full()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::prev_page))
            .on_action(cx.listener(Self::next_page))
            .on_action(cx.listener(Self::zoom_in))
            .on_action(cx.listener(Self::zoom_out))
            .on_action(cx.listener(Self::toggle_line_number))
            .on_action(cx.listener(Self::toggle_columns))
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
        let paper = preview.layout.as_ref().map_or(preview.paper, Layout::paper);
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
        let editor = preview.editor.read(cx);
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
                    at(s.margin_left, s.margin_top),
                    at(paper.width - s.margin_right, paper.height - s.margin_bottom),
                ),
                rgb(0x808080),
                BorderStyle::Solid,
            ));
            let page = layout.page(preview.page, &editor.lines, &doc_info(editor));
            let fonts = [print_font(&s.font_han), print_font(&s.font_zen)];
            for text in &page.texts {
                paint_text(
                    text,
                    s.font_width,
                    to_px(s.font_height),
                    &fonts,
                    &at,
                    window,
                );
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

/// Paints text on its grid: GPUI shapes it, picking fallback fonts for characters that the font
/// lacks, and each glyph goes to the cell of its character.
fn paint_text(
    text: &PlacedText,
    cell: i32,
    font_size: Pixels,
    fonts: &[Font; 2],
    at: &impl Fn(i32, i32) -> Point<Pixels>,
    window: &mut Window,
) {
    let runs: Vec<TextRun> = font_runs(&text.text)
        .into_iter()
        .map(|(range, zen)| TextRun {
            len: range.len(),
            font: fonts[usize::from(zen)].clone(),
            color: black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        })
        .collect();
    let line = window
        .text_system()
        .layout_line(&text.text, font_size, &runs, None);
    let xs: Vec<(usize, i32)> = text.positions(cell).map(|(i, _, x)| (i, x)).collect();
    for run in &line.runs {
        for glyph in &run.glyphs {
            let x = match xs.binary_search_by_key(&glyph.index, |&(i, _)| i) {
                Ok(k) => xs[k].1,
                Err(k) => xs[k.saturating_sub(1)].1,
            };
            let origin = at(x, text.baseline);
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
