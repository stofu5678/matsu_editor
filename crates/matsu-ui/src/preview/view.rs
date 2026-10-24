//! The view of the preview tab: a header line and the rendered text of the document.

use std::path::PathBuf;
use std::time::Duration;

use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, Styled as _, Subscription, WeakEntity, Window, div,
    px, rgb,
};
use gpui_component::text::TextViewState;

use super::text::{self, Format, MAX_BYTES};
use super::{AsHtml, AsMarkdown, Close, Refresh, ToSource};
use crate::app::Workspace;
use crate::document::SharedDocument;

/// How long after the last change the view reads the text again.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// The preview of one document.
pub(crate) struct PreviewView {
    document: Entity<SharedDocument>,
    state: Entity<TextViewState>,
    format: Format,
    truncated: bool,
    too_large: bool,
    /// How many bytes of text the view was given last, and how many times it was given text.
    shown_len: usize,
    reads: u64,
    /// The version of the document the view shows.
    shown_version: u64,
    /// Counts the changes seen, so that only the last one of a burst reads the text.
    generation: u64,
    focus: FocusHandle,
    workspace: WeakEntity<Workspace>,
    _observe: Subscription,
}

impl PreviewView {
    pub(crate) fn new(
        document: Entity<SharedDocument>,
        workspace: WeakEntity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        let format = text::format_of(document.read(cx).path());
        let read = text::extract(document.read(cx).doc(), MAX_BYTES);
        let state = new_state(format, &read.text, cx);
        let observe = cx.observe(&document, |this, _, cx| this.schedule_refresh(cx));
        Self {
            shown_version: document.read(cx).version(),
            document,
            state,
            format,
            truncated: read.truncated,
            too_large: read.too_large,
            shown_len: read.text.len(),
            reads: 1,
            generation: 0,
            focus: cx.focus_handle(),
            workspace,
            _observe: observe,
        }
    }

    pub(crate) fn document(&self) -> &Entity<SharedDocument> {
        &self.document
    }

    #[cfg(test)]
    pub(crate) fn format(&self) -> Format {
        self.format
    }

    #[cfg(test)]
    pub(crate) fn truncated(&self) -> bool {
        self.truncated
    }

    #[cfg(test)]
    pub(crate) fn too_large(&self) -> bool {
        self.too_large
    }

    /// The name of the document as the preview titles it.
    pub(crate) fn document_name(&self, cx: &App) -> String {
        let shared = self.document.read(cx);
        crate::file_ops::ask::document_title(shared.path(), shared.untitled_number())
    }

    /// The bytes of text the view was given last, and how many times it was given text (the tests
    /// check that a burst of changes reads once).
    #[cfg(test)]
    pub(crate) fn shown(&self) -> (usize, u64) {
        (self.shown_len, self.reads)
    }

    /// Reads the text again after the document stops changing for [`DEBOUNCE`].
    fn schedule_refresh(&mut self, cx: &mut Context<Self>) {
        if self.document.read(cx).version() == self.shown_version {
            return;
        }
        self.generation += 1;
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(DEBOUNCE).await;
            this.update(cx, |this, cx| {
                if this.generation == generation {
                    this.refresh(cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// Reads the text of the document again and shows it.
    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        let shared = self.document.read(cx);
        self.shown_version = shared.version();
        let read = text::extract(shared.doc(), MAX_BYTES);
        self.truncated = read.truncated;
        self.too_large = read.too_large;
        self.shown_len = read.text.len();
        self.reads += 1;
        self.state
            .update(cx, |state, cx| state.set_text(&read.text, cx));
        cx.notify();
    }

    /// Shows the document as `format` (Markdown or HTML).
    pub(crate) fn set_format(&mut self, format: Format, cx: &mut Context<Self>) {
        if format == self.format {
            return;
        }
        self.format = format;
        let shared = self.document.read(cx);
        self.shown_version = shared.version();
        let read = text::extract(shared.doc(), MAX_BYTES);
        self.truncated = read.truncated;
        self.too_large = read.too_large;
        self.shown_len = read.text.len();
        self.reads += 1;
        self.state = new_state(format, &read.text, cx);
        cx.notify();
    }

    /// The directory the images of the document are looked up in: its own.
    fn base_dir(&self, cx: &App) -> Option<PathBuf> {
        self.document
            .read(cx)
            .path()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
    }

    /// The line above the text: what is shown and what was left out.
    fn header(&self, cx: &App) -> String {
        let mut header = format!(
            "{} のプレビュー: {}",
            self.format.name(),
            self.document_name(cx)
        );
        if self.too_large {
            header.push_str(&format!(
                "（大きいので、プレビューしません。{} MiB までのファイルを表示します）",
                text::MAX_SOURCE_BYTES >> 20
            ));
        } else if self.truncated {
            header.push_str(&format!(
                "（大きいので、先頭の {} KiB だけを表示しています）",
                MAX_BYTES / 1024
            ));
        }
        header
    }
}

fn new_state(format: Format, text: &str, cx: &mut Context<PreviewView>) -> Entity<TextViewState> {
    cx.new(|cx| match format {
        Format::Markdown => TextViewState::markdown(text, cx),
        Format::Html => TextViewState::html(text, cx),
    })
}

impl Focusable for PreviewView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for PreviewView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let header = self.header(cx);
        let base = self.base_dir(cx);
        let body = gpui_base::TextView::new(&self.state)
            .scrollable(true)
            .selectable(true)
            // A preview must not fetch from the network (a page can carry a tracking image), so
            // only the files next to the document are loaded.
            .image_source(
                move |uri| match text::local_image(uri.as_ref(), base.as_deref()) {
                    Some(path) => gpui::ImageSource::Resource(gpui::Resource::Path(path.into())),
                    None => gpui::ImageSource::Custom(std::sync::Arc::new(|_, _| None)),
                },
            )
            // Only web links open (in the browser); a link to a file or to a program does not.
            .on_link_click(|url, _, _, cx| {
                let lower = url.to_ascii_lowercase();
                if lower.starts_with("http://")
                    || lower.starts_with("https://")
                    || lower.starts_with("mailto:")
                {
                    cx.open_url(url);
                }
            });
        let name = header.clone();
        crate::a11y::preview_region(div().id("preview"), &name)
            .key_context("Preview")
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &Refresh, _, cx| this.refresh(cx)))
            .on_action(
                cx.listener(|this, _: &AsMarkdown, _, cx| this.set_format(Format::Markdown, cx)),
            )
            .on_action(cx.listener(|this, _: &AsHtml, _, cx| this.set_format(Format::Html, cx)))
            .on_action(cx.listener(|this, _: &ToSource, window, cx| {
                this.in_workspace(window, cx, |ws, window, cx| {
                    ws.preview_to_source(window, cx)
                });
            }))
            .on_action(cx.listener(|this, _: &Close, window, cx| {
                this.in_workspace(window, cx, |ws, window, cx| {
                    ws.close_active_preview(window, cx)
                });
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .text_color(rgb(0x1e1e1e))
            .child(
                div()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(rgb(0xd0d0d0))
                    .bg(rgb(0xf3f3f3))
                    .text_size(px(12.))
                    .child(header),
            )
            .child(div().flex_1().min_h_0().p_2().child(body))
    }
}

impl PreviewView {
    /// Runs `f` on the window after the action that is running ends: the window changes the tabs,
    /// and this view is one of them.
    fn in_workspace(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) + 'static,
    ) {
        let host = self.workspace.clone();
        window.defer(cx, move |window, cx| {
            host.update(cx, |workspace, cx| f(workspace, window, cx))
                .ok();
        });
    }
}
