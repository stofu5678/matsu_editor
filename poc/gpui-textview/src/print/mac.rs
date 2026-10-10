//! Printing on macOS (ADR-0011): the print panel and the printer through AppKit's
//! NSPrintOperation, with the pages of the shared layout drawn by Core Text. A real app keeps
//! this in its platform crate (`ume-platform`, ADR-0013), the only place with `unsafe`.
//!
//! The pages are an NSView that paginates itself (knowsPageRange: and rectForPage:). It lays the
//! text out again for the paper of the print operation, so the print panel's own preview follows
//! the paper chosen in it.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::ptr::NonNull;

use futures_channel::oneshot;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSGraphicsContext, NSPaperOrientation, NSPrintInfo, NSPrintOperation, NSPrintPanelOptions,
    NSResponder, NSView, NSWindow,
};
use objc2_core_foundation::{
    CFAttributedString, CFDictionary, CFIndex, CFNumber, CFRange, CFRetained, CFString, CFType,
    CGAffineTransform, CGFloat, CGPoint,
};
use objc2_core_graphics::{CGContext, CGGlyph};
use objc2_core_text::{CTFont, CTLine, CTRun, kCTFontAttributeName, kCTLigatureAttributeName};
use objc2_foundation::{NSCopying, NSInteger, NSPoint, NSRange, NSRect, NSSize, NSString};

use super::PrintJob;
use super::layout::{Layout, Page, Paper, PlacedText, PrintSetting, font_runs};

/// 0.1 mm in points (1/72 inch), the unit of Core Graphics.
fn points(tenth_mm: i32) -> CGFloat {
    tenth_mm as CGFloat * 72.0 / 254.0
}

fn tenth_mm(points: CGFloat) -> i32 {
    (points * 254.0 / 72.0).round() as i32
}

fn paper_of(info: &NSPrintInfo) -> Paper {
    let size = info.paperSize();
    Paper {
        width: tenth_mm(size.width),
        height: tenth_mm(size.height),
    }
}

/// The paper of the app's print settings (NSPrintInfo's shared one), as the preview bar shows
/// it: "A4  縦", as Sakura does.
pub fn paper() -> (Paper, String) {
    let info = NSPrintInfo::sharedPrintInfo();
    let name = info
        .localizedPaperName()
        .map(|name| name.to_string())
        .unwrap_or_default();
    let orientation = if info.orientation() == NSPaperOrientation::Landscape {
        "横"
    } else {
        "縦"
    };
    (paper_of(&info), format!("{name}  {orientation}"))
}

/// The AppKit window of a GPUI window.
pub fn ns_window(window: &gpui::Window) -> Option<Retained<NSWindow>> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    // SAFETY: GPUI's AppKit handle points at the window's content view, which lives as long as
    // the window.
    let view: &NSView = unsafe { handle.ns_view.cast().as_ref() };
    view.window()
}

/// A copy of the app's print settings without margins: the layout has its own, and each page
/// is the whole paper.
fn print_info() -> Retained<NSPrintInfo> {
    let info = NSPrintInfo::sharedPrintInfo().copy();
    info.setLeftMargin(0.);
    info.setRightMargin(0.);
    info.setTopMargin(0.);
    info.setBottomMargin(0.);
    info.setHorizontallyCentered(false);
    info.setVerticallyCentered(false);
    info
}

/// Shows the print panel as a sheet on the window and prints if the user says so. Returns
/// whether it printed. Call it outside GPUI's updates: the sheet takes the key window from
/// GPUI's, and GPUI handles that at once.
pub async fn print(window: Retained<NSWindow>, job: PrintJob) -> bool {
    let mtm = MainThreadMarker::new().expect("printing starts on the main thread");
    // AppKit would not start a second sheet, nor say that it did not.
    if window.attachedSheet().is_some() {
        return false;
    }
    let info = print_info();
    let title = job.doc.file_name.clone().unwrap_or_else(|| "(無題)".into());
    let view = PrintView::new(mtm, job, paper_of(&info));
    let operation = NSPrintOperation::printOperationWithView_printInfo(&view, &info);
    operation.setJobTitle(Some(&NSString::from_str(&title)));
    let panel = operation.printPanel();
    panel.setOptions(
        panel.options()
            | NSPrintPanelOptions::ShowsPaperSize
            | NSPrintPanelOptions::ShowsOrientation
            | NSPrintPanelOptions::ShowsPreview,
    );
    let (done, printed) = oneshot::channel();
    let delegate = PrintDelegate::new(mtm, done);
    let delegate_object: &AnyObject = &delegate;
    // SAFETY: the delegate has the method of the selector, with the signature that AppKit
    // calls, and it lives until AppKit has called it (below).
    unsafe {
        operation.runOperationModalForWindow_delegate_didRunSelector_contextInfo(
            &window,
            Some(delegate_object),
            Some(sel!(printOperationDidRun:success:contextInfo:)),
            std::ptr::null_mut(),
        );
    }
    let printed = printed.await.unwrap_or(false);
    drop(delegate);
    printed
}

/// The state of a print view.
struct Pages {
    job: PrintJob,
    /// The layout for the paper of the print operation, made in knowsPageRange:.
    layout: RefCell<Option<Layout>>,
}

define_class!(
    // SAFETY: NSView has no requirements for subclasses that this breaks, and PrintView does not
    // implement Drop.
    #[unsafe(super(NSView, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "UmePocPrintView"]
    #[ivars = Pages]
    struct PrintView;

    impl PrintView {
        /// Pages go down from the top, like the layout's lengths.
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        /// Lays the text out for the paper of the print operation and tells AppKit the pages.
        #[unsafe(method(knowsPageRange:))]
        fn knows_page_range(&self, range: *mut NSRange) -> bool {
            let state = self.ivars();
            let paper = NSPrintOperation::currentOperation(self.mtm())
                .map(|operation| paper_of(&operation.printInfo()))
                .unwrap_or(Paper::A4);
            let layout = Layout::new(&state.job.lines, &state.job.setting, paper).ok();
            let pages = layout.as_ref().map_or(0, Layout::page_count);
            self.setFrameSize(NSSize::new(
                points(paper.width),
                points(paper.height) * pages.max(1) as CGFloat,
            ));
            *state.layout.borrow_mut() = layout;
            // SAFETY: AppKit passes a valid pointer to the range to fill.
            unsafe { range.write(NSRange::new(1, pages)) };
            true
        }

        /// The pages go down the view, one paper high each.
        #[unsafe(method(rectForPage:))]
        fn rect_for_page(&self, page: NSInteger) -> NSRect {
            let Some(paper) = self.ivars().layout.borrow().as_ref().map(Layout::paper) else {
                return NSRect::ZERO;
            };
            let (width, height) = (points(paper.width), points(paper.height));
            NSRect::new(
                NSPoint::new(0., height * (page - 1) as CGFloat),
                NSSize::new(width, height),
            )
        }

        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, dirty: NSRect) {
            let state = self.ivars();
            let layout = state.layout.borrow();
            let (Some(layout), Some(context)) = (layout.as_ref(), NSGraphicsContext::currentContext())
            else {
                return;
            };
            let context = context.CGContext();
            let height = points(layout.paper().height);
            let first = (dirty.origin.y / height).floor().max(0.) as usize;
            let last = ((dirty.origin.y + dirty.size.height) / height).ceil() as usize;
            let fonts = Fonts::new(layout.setting());
            for index in first..last.min(layout.page_count()) {
                let page = layout.page(index, &state.job.lines, &state.job.doc);
                CGContext::save_g_state(Some(&context));
                CGContext::translate_ctm(Some(&context), 0., height * index as CGFloat);
                draw_page(&context, &page, layout.setting(), &fonts);
                CGContext::restore_g_state(Some(&context));
            }
        }
    }
);

impl PrintView {
    fn new(mtm: MainThreadMarker, job: PrintJob, paper: Paper) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Pages {
            job,
            layout: RefCell::new(None),
        });
        let frame = NSRect::new(
            NSPoint::ZERO,
            NSSize::new(points(paper.width), points(paper.height)),
        );
        // SAFETY: NSView's designated initializer.
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }
}

define_class!(
    // SAFETY: NSObject has no requirements for subclasses, and PrintDelegate does not implement
    // Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "UmePocPrintDelegate"]
    #[ivars = Cell<Option<oneshot::Sender<bool>>>]
    struct PrintDelegate;

    impl PrintDelegate {
        /// The print operation ended: printed, or cancelled in the print panel.
        #[unsafe(method(printOperationDidRun:success:contextInfo:))]
        fn print_operation_did_run(
            &self,
            operation: &NSPrintOperation,
            success: bool,
            _context: *mut c_void,
        ) {
            if success {
                // Keep the paper chosen in the print panel for the next preview, as Sakura keeps
                // the printer's settings after printing (CPrintPreview::OnPrint).
                let chosen = operation.printInfo();
                let shared = NSPrintInfo::sharedPrintInfo();
                shared.setPaperName(chosen.paperName().as_deref());
                shared.setPaperSize(chosen.paperSize());
                shared.setOrientation(chosen.orientation());
            }
            if let Some(done) = self.ivars().take() {
                done.send(success).ok();
            }
        }
    }
);

impl PrintDelegate {
    fn new(mtm: MainThreadMarker, done: oneshot::Sender<bool>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Cell::new(Some(done)));
        // SAFETY: NSObject's designated initializer.
        unsafe { msg_send![super(this), init] }
    }
}

/// The fonts of a setting, in the size that it prints.
struct Fonts {
    han: CFRetained<CTFont>,
    zen: CFRetained<CTFont>,
}

impl Fonts {
    fn new(setting: &PrintSetting) -> Self {
        // Sakura gives the height as the font's height in GDI. For Japanese fonts it is about
        // the size of their em square, which Core Text takes.
        let size = points(setting.font_height);
        let font = |name: &str| {
            // SAFETY: no transform; Core Text finds the font by name, or a fallback.
            unsafe { CTFont::with_name(&CFString::from_str(name), size, std::ptr::null()) }
        };
        Self {
            han: font(&setting.font_han),
            zen: font(&setting.font_zen),
        }
    }
}

/// Draws a page from the top-left corner of the paper, in a flipped context.
fn draw_page(context: &CGContext, page: &Page, setting: &PrintSetting, fonts: &Fonts) {
    CGContext::set_rgb_fill_color(Some(context), 0., 0., 0., 1.);
    for text in &page.texts {
        draw_text(context, text, setting.font_width, fonts);
    }
    CGContext::set_rgb_stroke_color(Some(context), 0., 0., 0., 1.);
    CGContext::set_line_width(Some(context), 0.5);
    for rule in &page.rules {
        let line = [
            CGPoint::new(points(rule.from.0), points(rule.from.1)),
            CGPoint::new(points(rule.to.0), points(rule.to.1)),
        ];
        // SAFETY: two points, one segment.
        unsafe { CGContext::stroke_line_segments(Some(context), line.as_ptr(), line.len()) };
    }
}

/// Draws text on its grid: Core Text shapes it, picking fallback fonts for characters that the
/// font lacks, and each glyph goes to the cell of its character.
fn draw_text(context: &CGContext, text: &PlacedText, cell: i32, fonts: &Fonts) {
    let positions: Vec<(usize, char, i32)> = text.positions(cell).collect();
    for (range, zen) in font_runs(&text.text) {
        // The x of each UTF-16 unit of the run, which is how Core Text counts the characters.
        let mut xs = Vec::new();
        for &(_, ch, x) in positions.iter().filter(|(i, ..)| range.contains(i)) {
            xs.extend(std::iter::repeat_n(points(x), ch.len_utf16()));
        }
        let font = if zen { &fonts.zen } else { &fonts.han };
        let Some(line) = shape(&text.text[range], font) else {
            continue;
        };
        // SAFETY: the runs of a line are CTRuns.
        let runs = unsafe { CTLine::glyph_runs(&line) };
        let runs = unsafe { runs.cast_unchecked::<CTRun>() };
        for run in runs.iter() {
            let count = unsafe { run.glyph_count() } as usize;
            if count == 0 {
                continue;
            }
            let mut glyphs: Vec<CGGlyph> = vec![0; count];
            let mut indices: Vec<CFIndex> = vec![0; count];
            // SAFETY: the buffers hold `count` items, and an empty range means all of them.
            unsafe {
                run.glyphs(
                    CFRange::new(0, 0),
                    NonNull::new_unchecked(glyphs.as_mut_ptr()),
                );
                run.string_indices(
                    CFRange::new(0, 0),
                    NonNull::new_unchecked(indices.as_mut_ptr()),
                );
            }
            let mut origins: Vec<CGPoint> = indices
                .iter()
                .map(|&i| CGPoint::new(xs.get(i as usize).copied().unwrap_or_default(), 0.))
                .collect();
            let run_font = run_font(&run).unwrap_or_else(|| font.clone());
            // Text space has y up: flip back at the baseline.
            CGContext::save_g_state(Some(context));
            CGContext::translate_ctm(Some(context), 0., points(text.baseline));
            CGContext::scale_ctm(Some(context), 1., -1.);
            let identity = CGAffineTransform {
                a: 1.,
                b: 0.,
                c: 0.,
                d: 1.,
                tx: 0.,
                ty: 0.,
            };
            CGContext::set_text_matrix(Some(context), identity);
            // SAFETY: `glyphs` and `origins` hold `count` items each.
            unsafe {
                run_font.draw_glyphs(
                    NonNull::new_unchecked(glyphs.as_mut_ptr()),
                    NonNull::new_unchecked(origins.as_mut_ptr()),
                    count,
                    context,
                );
            }
            CGContext::restore_g_state(Some(context));
        }
    }
}

/// Shapes text in a font, without ligatures, so that each character keeps its own glyph.
fn shape(text: &str, font: &CTFont) -> Option<CFRetained<CTLine>> {
    let no_ligatures = CFNumber::new_i32(0);
    let keys: [&CFString; 2] = [unsafe { kCTFontAttributeName }, unsafe {
        kCTLigatureAttributeName
    }];
    let values: [&CFType; 2] = [font, &no_ligatures];
    let attributes = CFDictionary::<CFString, CFType>::from_slices(&keys, &values);
    // SAFETY: the attributes are Core Text's, with values of the types it takes.
    let string = unsafe {
        CFAttributedString::new(
            None,
            Some(&CFString::from_str(text)),
            Some(attributes.as_opaque()),
        )
    }?;
    // SAFETY: an attributed string with Core Text attributes.
    Some(unsafe { CTLine::with_attributed_string(&string) })
}

/// The font that Core Text chose for a run, which may be a fallback font.
fn run_font(run: &CTRun) -> Option<CFRetained<CTFont>> {
    // SAFETY: a run's attributes map Core Text's attribute names to their values.
    let attributes = unsafe { run.attributes() };
    let attributes = unsafe { attributes.cast_unchecked::<CFString, CFType>() };
    attributes
        .get(unsafe { kCTFontAttributeName })?
        .downcast::<CTFont>()
        .ok()
}

/// The print operations of the self-check, which print to a PDF file without panels.
#[cfg(feature = "self-check")]
pub mod check {
    use std::path::Path;

    use anyhow::{Context as _, Result, ensure};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSPrintInfo, NSPrintJobSavingURL, NSPrintOperation, NSPrintSaveJob, NSWindow,
    };
    use objc2_core_foundation::{CFURL, CGFloat, CGRect};
    use objc2_core_graphics::{
        CGBitmapContextCreate, CGBitmapContextGetData, CGColorSpace, CGContext, CGImageAlphaInfo,
        CGPDFBox, CGPDFDocument, CGPDFPage,
    };
    use objc2_foundation::{NSSize, NSURL};

    use super::{PrintJob, PrintView, paper_of, points, print_info};
    use crate::print::layout::Paper;

    /// Prints the job to a PDF file through the same print view as the print panel, on the
    /// paper given or the app's.
    pub fn save_pdf(job: PrintJob, paper: Option<Paper>, path: &Path) -> Result<()> {
        let mtm = MainThreadMarker::new().context("not on the main thread")?;
        let info = print_info();
        if let Some(paper) = paper {
            info.setPaperSize(NSSize::new(points(paper.width), points(paper.height)));
        }
        info.setJobDisposition(unsafe { NSPrintSaveJob });
        let url = NSURL::from_file_path(path).context("no URL for the PDF")?;
        // SAFETY: NSPrintJobSavingURL takes an NSURL.
        unsafe {
            info.dictionary().setObject_forKey(
                &url,
                objc2::runtime::ProtocolObject::from_ref(NSPrintJobSavingURL),
            );
        }
        let view = PrintView::new(mtm, job, paper_of(&info));
        let operation = NSPrintOperation::printOperationWithView_printInfo(&view, &info);
        operation.setShowsPrintPanel(false);
        operation.setShowsProgressPanel(false);
        ensure!(operation.runOperation(), "the print operation failed");
        Ok(())
    }

    fn open(path: &Path) -> Result<objc2_core_foundation::CFRetained<CGPDFDocument>> {
        let url = CFURL::from_file_path(path).context("no URL for the PDF")?;
        CGPDFDocument::with_url(Some(&url)).context("not a PDF file")
    }

    /// The number of pages in a PDF file and the size of the first one, in points.
    pub fn pdf_pages(path: &Path) -> Result<(usize, (CGFloat, CGFloat))> {
        let document = open(path)?;
        let count = CGPDFDocument::number_of_pages(Some(&document));
        let page = CGPDFDocument::page(Some(&document), 1).context("no first page")?;
        let size = CGPDFPage::box_rect(Some(&page), CGPDFBox::MediaBox).size;
        Ok((count, (size.width, size.height)))
    }

    /// Renders a page of a PDF file, counted from 1, at `scale` pixels per point.
    pub fn render_pdf_page(path: &Path, number: usize, scale: CGFloat) -> Result<image::RgbaImage> {
        let document = open(path)?;
        let page = CGPDFDocument::page(Some(&document), number).context("no such page")?;
        let size = CGPDFPage::box_rect(Some(&page), CGPDFBox::MediaBox).size;
        let (width, height) = (
            (size.width * scale).ceil() as usize,
            (size.height * scale).ceil() as usize,
        );
        let space = CGColorSpace::new_device_rgb().context("no RGB color space")?;
        // SAFETY: Core Graphics allocates the pixels, 4 bytes each.
        let context = unsafe {
            CGBitmapContextCreate(
                std::ptr::null_mut(),
                width,
                height,
                8,
                width * 4,
                Some(&space),
                CGImageAlphaInfo::PremultipliedLast.0,
            )
        }
        .context("no bitmap context")?;
        CGContext::set_rgb_fill_color(Some(&context), 1., 1., 1., 1.);
        CGContext::fill_rect(
            Some(&context),
            CGRect::new(
                Default::default(),
                objc2_core_foundation::CGSize::new(width as CGFloat, height as CGFloat),
            ),
        );
        CGContext::scale_ctm(Some(&context), scale, scale);
        CGContext::draw_pdf_page(Some(&context), Some(&page));
        let data = CGBitmapContextGetData(Some(&context));
        ensure!(!data.is_null(), "no pixels");
        // SAFETY: the context's pixels, which it keeps until it is released below.
        let pixels = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), width * height * 4) };
        image::RgbaImage::from_raw(width as u32, height as u32, pixels.to_vec())
            .context("the pixels do not make an image")
    }

    /// Whether a sheet, such as the print panel, is on the window.
    pub fn has_sheet(window: &NSWindow) -> bool {
        window.attachedSheet().is_some()
    }

    /// The paper of the app's print settings, which a print with the panel may change.
    pub fn shared_paper() -> Paper {
        paper_of(&NSPrintInfo::sharedPrintInfo())
    }
}
