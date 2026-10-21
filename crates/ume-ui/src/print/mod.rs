//! Printing in the window (ADR-0011): the print preview ([`preview`]) takes the place of the text,
//! and the OS prints the pages (`ume-platform`).
//!
//! The functions are Sakura's `F_PRINT_PREVIEW`, `F_PRINT` and `F_PRINT_PAGESETUP`, which are UI
//! commands (`ui_commands/file/`) that call the methods of [`EditorView`] here. As in Sakura
//! (`CViewCommander::Command_PRINT`), printing opens the preview first and then the print dialog
//! of the OS; printing closes the preview, and cancelling the dialog leaves it open.

pub mod preview;

use gpui::{App, AppContext as _, Context, Entity, Focusable as _, Subscription, Window};
use ume_core::buffer::Document;
use ume_platform::print::{Error, PrintJob};
use ume_print::{HeaderContext, LineSource, TypeOptions};

pub use preview::{PreviewEvent, PrintPreview};

use crate::view::EditorView;

/// The document as the lines that the print layout reads. A clone of the document is a pointer
/// (`Document` shares its text), so the snapshot costs nothing, however big the document is, and
/// editing goes on without changing the pages.
struct DocLines(Document);

impl LineSource for DocLines {
    fn line_count(&self) -> usize {
        self.0.line_count()
    }

    fn read_line(&self, index: usize, buf: &mut String) -> bool {
        buf.clear();
        match self.0.line(index) {
            Ok(Some(line)) => {
                buf.push_str(&line.text);
                line.eol.is_some()
            }
            // A line that cannot be read prints as an empty one.
            _ => false,
        }
    }
}

/// What an [`EditorView`] knows about printing.
#[derive(Default)]
pub(crate) struct PrintState {
    /// The preview, which takes the place of the text while it is open.
    pub(crate) preview: Option<Entity<PrintPreview>>,
    subscription: Option<Subscription>,
    /// The print dialog of the OS is open.
    printing: bool,
    /// The print setting dialog, while it is open (`crate::print_setting`).
    pub(crate) setup_dialog: Option<Entity<crate::print_setting::dialog::PrintSettingDialog>>,
}

impl EditorView {
    /// The document, the print setting and what the header shows, as they are now.
    pub(crate) fn print_job(&self, cx: &App) -> PrintJob {
        let shared = self.document.read(cx);
        let caret = self.cursor.caret();
        let path = shared.path().map(|path| path.display().to_string());
        let title = shared.file_name().unwrap_or_else(|| "(無題)".into());
        PrintJob {
            source: std::sync::Arc::new(DocLines(shared.doc().clone())),
            setting: self.current_print_setting(cx),
            options: TypeOptions::default(),
            header: HeaderContext {
                file_path: path,
                caret_x: caret.col + 1,
                caret_y: caret.line + 1,
                modified: shared.model().is_modified(),
                ..HeaderContext::default()
            },
            title,
        }
    }

    /// Whether the print dialog of the OS is open (a macro waits for it).
    pub(crate) fn is_printing(&self) -> bool {
        self.print.printing
    }

    pub fn is_print_preview_open(&self) -> bool {
        self.print.preview.is_some()
    }

    /// The preview that is open, if any.
    pub fn print_preview(&self) -> Option<&Entity<PrintPreview>> {
        self.print.preview.as_ref()
    }

    /// Shows the print preview in place of the text (Sakura's `F_PRINT_PREVIEW`).
    pub fn open_print_preview(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PrintPreview> {
        if let Some(preview) = &self.print.preview {
            return preview.clone();
        }
        let job = self.print_job(cx);
        let preview = cx.new(|cx| PrintPreview::new(job, cx));
        self.print.subscription =
            Some(
                cx.subscribe_in(&preview, window, |this, _, event, window, cx| match event {
                    PreviewEvent::Print => this.print_from_preview(window, cx),
                    PreviewEvent::PageSetup => this.print_page_setup(window, cx),
                    PreviewEvent::Close => this.close_print_preview(window, cx),
                }),
            );
        window.focus(&preview.focus_handle(cx), cx);
        self.print.preview = Some(preview.clone());
        cx.notify();
        preview
    }

    /// Goes back to the text.
    pub fn close_print_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.print.preview.take().is_some() {
            self.print.subscription = None;
            window.focus(&self.focus_handle, cx);
            cx.notify();
        }
    }

    /// Prints as Sakura does (`CViewCommander::Command_PRINT`): the preview opens, then the print
    /// dialog of the OS.
    pub fn print(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_print_preview(window, cx);
        self.print_from_preview(window, cx);
    }

    /// Shows the print dialog for the pages of the preview that is open. The preview closes after
    /// printing and stays when the dialog is cancelled.
    fn print_from_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(preview) = self.print.preview.clone() else {
            return;
        };
        if self.print.printing {
            return;
        }
        let job = preview.read(cx).printable();
        preview.update(cx, |preview, cx| {
            preview.notice = job.as_ref().err().cloned();
            cx.notify();
        });
        let Ok(job) = job else {
            return;
        };
        self.print.printing = true;
        let outcome = os_print(job, window, cx);
        cx.spawn_in(window, async move |this, cx| {
            let result = outcome.await;
            this.update_in(cx, |this, window, cx| this.finish_print(result, window, cx))
                .ok();
        })
        .detach();
    }

    /// What became of the print dialog: printed, cancelled, or failed.
    pub(crate) fn finish_print(
        &mut self,
        result: Result<bool, Error>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.print.printing = false;
        let notice = match result {
            Ok(true) => {
                self.close_print_preview(window, cx);
                self.notice = Some("印刷しました".into());
                cx.notify();
                return;
            }
            Ok(false) => "印刷を取り消しました".to_owned(),
            Err(err) => err.0,
        };
        match &self.print.preview {
            Some(preview) => preview.update(cx, |preview, cx| {
                preview.notice = Some(notice.into());
                cx.notify();
            }),
            // The preview was closed while the dialog was open.
            None => {
                self.notice = Some(notice);
                cx.notify();
            }
        }
    }
}

/// The window of the OS under a GPUI window, for the print dialog to belong to.
#[cfg(any(windows, target_os = "macos"))]
fn raw_handle(window: &Window) -> Option<raw_window_handle::RawWindowHandle> {
    use raw_window_handle::HasWindowHandle;
    HasWindowHandle::window_handle(window)
        .ok()
        .map(|handle| handle.as_raw())
}

/// Shows the print dialog of the OS and prints, off the UI thread where the OS blocks until the
/// dialog closes. Resolves to whether it printed: `false` when the user cancels.
#[cfg(windows)]
fn os_print(
    job: PrintJob,
    window: &Window,
    cx: &mut Context<EditorView>,
) -> impl Future<Output = Result<bool, Error>> + use<> {
    // A number rather than a handle, so that it can go to the thread that shows the dialog.
    let owner = raw_handle(window).map_or(0, ume_platform::print::windows::owner);
    cx.background_spawn(async move { ume_platform::print::windows::print(owner, job) })
}

/// The print portal shows the dialog (`org.freedesktop.portal.Print`).
#[cfg(not(any(windows, target_os = "macos")))]
fn os_print(
    job: PrintJob,
    _: &Window,
    cx: &mut Context<EditorView>,
) -> impl Future<Output = Result<bool, Error>> + use<> {
    cx.background_spawn(ume_platform::print::linux::print(job))
}

/// AppKit's print panel, as a sheet on the window. It starts here, outside GPUI's update of the
/// view: the sheet takes the key window from GPUI's, and GPUI handles that at once.
#[cfg(target_os = "macos")]
fn os_print(
    job: PrintJob,
    window: &Window,
    _: &mut Context<EditorView>,
) -> impl Future<Output = Result<bool, Error>> + use<> {
    let ns_window = raw_handle(window).and_then(ume_platform::print::mac::ns_window);
    async move {
        match ns_window {
            Some(ns_window) => Ok(ume_platform::print::mac::print(ns_window, job).await),
            None => Err("印刷するウィンドウを取得できません".into()),
        }
    }
}

#[cfg(test)]
mod tests;
