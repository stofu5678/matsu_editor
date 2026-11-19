//! Printing in the window (ADR-0011): the print preview ([`preview`]) takes the place of the text,
//! and the OS prints the pages (`matsu-platform`).
//!
//! The functions are Sakura's `F_PRINT_PREVIEW`, `F_PRINT` and `F_PRINT_PAGESETUP`, which are UI
//! commands (`ui_commands/file/`) that call the methods of [`EditorView`] here. As in Sakura
//! (`CViewCommander::Command_PRINT`), printing opens the preview first and then the print dialog
//! of the OS; printing closes the preview, and cancelling the dialog leaves it open.

pub mod preview;
mod styler;

use gpui::{App, AppContext as _, Context, Entity, Focusable as _, Subscription, Window};
use matsu_core::buffer::Document;
use matsu_platform::print::{Error, Outcome, PrintJob};
use matsu_print::{Layout, LineSource, TypeOptions};

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

/// Where the print of a window is: one state, so that the print setting it started with belongs
/// to the print that runs (`docs/ownership.md`, rule 1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Phase {
    /// No print is running.
    #[default]
    Idle,
    /// The user asked to print while the pages of a big document were being laid out: the print
    /// dialog comes when they are there ([`PreviewEvent::Laid`]). `slot` is the print setting
    /// the window printed with when it was asked.
    WaitingForLayout { slot: usize },
    /// The print dialog of the OS is open. `slot` is the print setting (of the eight) the print
    /// started with: the printer that comes back goes to it, whatever another window selects
    /// meanwhile.
    Printing { slot: usize },
    /// The printer dialog of the preview bar is open (Windows). `slot` is the print setting it
    /// edits: the answer goes to it, whatever another window selects meanwhile.
    #[cfg_attr(not(any(windows, test)), allow(dead_code))]
    SelectingPrinter { slot: usize },
}

/// What an [`EditorView`] knows about printing.
#[derive(Default)]
pub(crate) struct PrintState {
    /// The preview, which takes the place of the text while it is open.
    pub(crate) preview: Option<Entity<PrintPreview>>,
    subscription: Option<Subscription>,
    /// The window title and the tab caption show the page (`$p`, `$P`), so the view redraws them
    /// when the preview changes.
    observation: Option<Subscription>,
    /// Whether a print runs, and the setting it started with.
    pub(crate) phase: Phase,
    /// The wait for the printer dialog of the bar (Windows); replaced by the next one.
    #[cfg(windows)]
    selecting: Option<gpui::Task<()>>,
    /// The print setting dialog, while it is open (`crate::print_setting`).
    pub(crate) setup_dialog: Option<Entity<crate::print_setting::dialog::PrintSettingDialog>>,
}

impl EditorView {
    /// The document, the print setting and what the header shows, as they are now.
    pub(crate) fn print_job(&self, cx: &App) -> PrintJob {
        let shared = self.document.read(cx);
        let title = shared.file_name().unwrap_or_else(|| "(無題)".into());
        PrintJob {
            source: std::sync::Arc::new(DocLines(shared.doc().clone())),
            setting: self.current_print_setting(cx),
            options: self.print_type_options(),
            // `$R`, `$N` and the rest as the title of the window has them.
            header: self.header_context(cx),
            title,
            // The colors of the type, for a setting that prints in color. The preview can turn
            // `color_print` on after the job was made, so it is always there.
            styler: Some(std::sync::Arc::new(styler::DocStyler::new(
                shared.doc().clone(),
                self.prefs.settings.coloring.clone(),
            ))),
        }
    }

    /// What the print layout takes from the type of the document: the printing makes its layout
    /// from a copy of the type (`CPrintPreview::OnChangePrintSetting`, `ref = ...GetDocumentAttribute()`),
    /// so the tab width, the indent of wrapped rows and the kinsoku characters are the type's.
    pub(crate) fn print_type_options(&self) -> TypeOptions {
        let settings = &self.prefs.settings;
        TypeOptions {
            tab_width: settings.tab_width,
            indent: settings.wrap_indent.clone(),
            kinsoku_head: settings.kinsoku.head_chars.clone(),
            kinsoku_tail: settings.kinsoku.tail_chars.clone(),
            kinsoku_kuto: settings.kinsoku.kuto_chars.clone(),
            tsv_mode: settings.tsv_mode,
            ..TypeOptions::default()
        }
    }

    /// Whether the print dialog of the OS is open, or a print is queued until the pages of a big
    /// document are laid out (a macro waits for it).
    pub(crate) fn is_printing(&self) -> bool {
        self.print.phase != Phase::Idle
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
                    PreviewEvent::SelectPrinter => this.select_printer(window, cx),
                    PreviewEvent::Close => this.close_print_preview(window, cx),
                    PreviewEvent::Laid => {
                        if let Phase::WaitingForLayout { slot } = this.print.phase {
                            this.print.phase = Phase::Idle;
                            this.start_print(slot, window, cx);
                        }
                    }
                }),
            );
        self.print.observation = Some(cx.observe(&preview, |_, _, cx| cx.notify()));
        window.focus(&preview.focus_handle(cx), cx);
        self.print.preview = Some(preview.clone());
        cx.notify();
        preview
    }

    /// Goes back to the text.
    pub fn close_print_preview(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.print.preview.take().is_some() {
            if matches!(self.print.phase, Phase::WaitingForLayout { .. }) {
                self.print.phase = Phase::Idle;
            }
            self.print.subscription = None;
            self.print.observation = None;
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
        if self.print.phase != Phase::Idle {
            return;
        }
        let slot = self.print_slot(cx);
        if preview.read(cx).is_laying_out() {
            self.print.phase = Phase::WaitingForLayout { slot };
            return;
        }
        self.start_print(slot, window, cx);
    }

    /// Shows the print dialog of the OS for the preview, with the printer of the setting `slot`.
    fn start_print(&mut self, slot: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(preview) = self.print.preview.clone() else {
            return;
        };
        let job = preview.read(cx).printable();
        preview.update(cx, |preview, cx| {
            preview.notice = job.as_ref().err().cloned();
            cx.notify();
        });
        let Ok(mut job) = job else {
            return;
        };
        // The dialog starts on the printer the preferences have now, not on the copy the preview
        // took (another window may have printed on another one since).
        job.setting.printer = Self::remembered_printer(slot, cx);
        self.print.phase = Phase::Printing { slot };
        // Only the macOS panel takes the layout (see `os_print`).
        let laid = if cfg!(target_os = "macos") {
            preview.read(cx).layout_snapshot()
        } else {
            None
        };
        let outcome = os_print(job, laid, window, cx);
        cx.spawn_in(window, async move |this, cx| {
            let result = outcome.await;
            this.update_in(cx, |this, window, cx| this.finish_print(result, window, cx))
                .ok();
        })
        .detach();
    }

    /// The printer button of the preview bar (`IDC_BUTTON_PRINTERSELECT`,
    /// `CPrintPreview::DispatchEvent_PPB`): shows the print setup dialog of the OS on the
    /// printer, the paper and the direction of the setting, and writes the answer into the
    /// setting. Windows only; the bar has no such button elsewhere
    /// ([`PrintPreview::has_printer_button`]).
    pub(crate) fn select_printer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        #[cfg(windows)]
        {
            if self.print.preview.is_none() || self.print.phase != Phase::Idle {
                return;
            }
            let slot = self.print_slot(cx);
            let setting = Self::print_setting_of(slot, cx);
            self.print.phase = Phase::SelectingPrinter { slot };
            let owner = raw_handle(window).map_or(0, matsu_platform::print::windows::owner);
            let choice = cx.background_spawn(async move {
                matsu_platform::print::windows::select_printer(owner, &setting)
            });
            // The view owns the wait: when the view goes, the wait ends. The native dialog itself
            // is modal on its own thread and lives until the user closes it (it cannot be
            // cancelled from here); its answer is then dropped (`docs/ownership.md`, section 6).
            self.print.selecting = Some(cx.spawn_in(window, async move |this, cx| {
                let result = choice.await;
                this.update_in(cx, |this, window, cx| {
                    this.finish_select_printer(result, window, cx)
                })
                .ok();
            }));
        }
        #[cfg(not(windows))]
        {
            let _ = (window, cx);
        }
    }

    /// What became of the printer dialog: the printer, the paper and the direction go to the
    /// setting it started with and every window's preview lays the document out again
    /// (`PM_PRINTSETTING`); cancelling changes nothing; a failure is shown on the bar.
    #[cfg(any(windows, test))]
    pub(crate) fn finish_select_printer(
        &mut self,
        result: Result<Option<matsu_platform::print::devmode::PrinterChoice>, Error>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let phase = std::mem::take(&mut self.print.phase);
        match (result, phase) {
            (Ok(Some(choice)), Phase::SelectingPrinter { slot }) => {
                Self::remember_choice(slot, choice, cx);
                self.print_setting_changed_in_all_windows(cx);
            }
            (Err(err), _) => {
                if let Some(preview) = &self.print.preview {
                    preview.update(cx, |preview, cx| {
                        preview.notice = Some(err.0.into());
                        cx.notify();
                    });
                }
            }
            _ => {}
        }
    }

    /// What became of the print dialog: printed, cancelled, or failed.
    pub(crate) fn finish_print(
        &mut self,
        result: Result<Outcome, Error>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let phase = std::mem::take(&mut self.print.phase);
        let notice = match result {
            Ok(Outcome::Printed(printer)) => {
                // The printer goes to the setting the print started with.
                if let (Phase::Printing { slot }, Some(printer)) = (phase, printer) {
                    Self::remember_printer(slot, printer, cx);
                }
                self.close_print_preview(window, cx);
                self.notice = Some("印刷しました".into());
                cx.notify();
                return;
            }
            Ok(Outcome::Cancelled) => "印刷を取り消しました".to_owned(),
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
/// dialog closes. Resolves to what became of it ([`Outcome`]).
///
/// `laid` is the layout the preview made for the paper of the setting, which only the macOS
/// print panel takes (it lays the document out for the paper it is asked about).
#[cfg(windows)]
fn os_print(
    job: PrintJob,
    _laid: Option<Layout>,
    window: &Window,
    cx: &mut Context<EditorView>,
) -> impl Future<Output = Result<Outcome, Error>> + use<> {
    // A number rather than a handle, so that it can go to the thread that shows the dialog.
    let owner = raw_handle(window).map_or(0, matsu_platform::print::windows::owner);
    cx.background_spawn(async move { matsu_platform::print::windows::print(owner, &job) })
}

/// The print portal shows the dialog (`org.freedesktop.portal.Print`).
#[cfg(not(any(windows, target_os = "macos")))]
fn os_print(
    job: PrintJob,
    _laid: Option<Layout>,
    _: &Window,
    cx: &mut Context<EditorView>,
) -> impl Future<Output = Result<Outcome, Error>> + use<> {
    let printed = cx.background_spawn(matsu_platform::print::linux::print(job));
    async move { printed.await.map(Outcome::from_printed) }
}

/// AppKit's print panel, as a sheet on the window. It starts here, outside GPUI's update of the
/// view: the sheet takes the key window from GPUI's, and GPUI handles that at once.
#[cfg(target_os = "macos")]
fn os_print(
    job: PrintJob,
    laid: Option<Layout>,
    window: &Window,
    _: &mut Context<EditorView>,
) -> impl Future<Output = Result<Outcome, Error>> + use<> {
    let ns_window = raw_handle(window).and_then(matsu_platform::print::mac::ns_window);
    async move {
        match ns_window {
            Some(ns_window) => Ok(Outcome::from_printed(
                matsu_platform::print::mac::print(ns_window, job, laid).await,
            )),
            None => Err("印刷するウィンドウを取得できません".into()),
        }
    }
}

#[cfg(test)]
mod tests;
