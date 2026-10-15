//! Printing on macOS (ADR-0011): the print panel and the printer through AppKit's
//! NSPrintOperation, with the pages of the shared layout drawn by Core Text. Ported from the PoC
//! of 2026-10-11 (`poc/gpui-textview/src/print/mac.rs`); not run by CI (CI only builds it), the
//! user checks it on a Mac.
//!
//! The pages are an NSView that paginates itself (knowsPageRange: and rectForPage:). It lays the
//! text out again for the paper of the print operation, so the print panel's own preview follows
//! the paper chosen in it.

// The Objective-C calls are `unsafe`. This module and `windows` are the only places of the
// program that need it (ADR-0013).
#![allow(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::ptr::NonNull;

use futures_channel::oneshot;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, Sel};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSGraphicsContext, NSPrintInfo, NSPrintOperation, NSPrintPanelOptions, NSResponder, NSView,
    NSWindow,
};
use objc2_core_foundation::{
    CFAttributedString, CFDictionary, CFIndex, CFNumber, CFRange, CFRetained, CFString, CFType,
    CGAffineTransform, CGFloat, CGPoint,
};
use objc2_core_graphics::{CGContext, CGGlyph};
use objc2_core_text::{CTFont, CTLine, CTRun, kCTFontAttributeName, kCTLigatureAttributeName};
use objc2_foundation::{NSCopying, NSInteger, NSPoint, NSRange, NSRect, NSSize, NSString};

use raw_window_handle::RawWindowHandle;
use ume_print::{FontKind, Layout, Page, Paper, PlacedText};

use super::PrintJob;
use super::draw::{cell_positions, font_spec, on_grid};

/// 0.1 mm in points (1/72 inch), the unit of Core Graphics.
fn points(tenth_mm: i32) -> CGFloat {
    CGFloat::from(super::draw::points(tenth_mm))
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

/// The AppKit window of a window handle (`None` if it is not an AppKit window).
pub fn ns_window(handle: RawWindowHandle) -> Option<Retained<NSWindow>> {
    let RawWindowHandle::AppKit(handle) = handle else {
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
    let title = job.title.clone();
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
    #[name = "UmePrintView"]
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
            let layout = state.job.layout(paper).ok();
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
            let (Some(layout), Some(context)) = (
                layout.as_ref(),
                NSGraphicsContext::currentContext().and_then(|context| cg_context(&context)),
            ) else {
                return;
            };
            let height = points(layout.paper().height);
            let first = (dirty.origin.y / height).floor().max(0.) as usize;
            let last = ((dirty.origin.y + dirty.size.height) / height).ceil() as usize;
            let fonts = Fonts::new(layout);
            for index in first..last.min(layout.page_count()) {
                let Some(page) = layout.page(index, &*state.job.source, &state.job.header) else {
                    continue;
                };
                CGContext::save_g_state(Some(&context));
                CGContext::translate_ctm(Some(&context), 0., height * index as CGFloat);
                draw_page(&context, &page, layout, &fonts);
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
    #[name = "UmePrintDelegate"]
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

/// The fonts of a setting, in the size that it prints: the print fonts and, when the setting has
/// them, the header's and the footer's.
struct Fonts {
    han: CFRetained<CTFont>,
    zen: CFRetained<CTFont>,
    header: Option<CFRetained<CTFont>>,
    footer: Option<CFRetained<CTFont>>,
}

impl Fonts {
    fn new(layout: &Layout) -> Self {
        let setting = layout.setting();
        let font = |kind| {
            let spec = font_spec(setting, kind);
            // Sakura gives the height as the font's height in GDI. For Japanese fonts it is about
            // the size of their em square, which Core Text takes.
            // SAFETY: no transform; Core Text finds the font by name, or a fallback.
            unsafe {
                CTFont::with_name(
                    &CFString::from_str(spec.face),
                    points(spec.height),
                    std::ptr::null(),
                )
            }
        };
        let own = |f: &Option<ume_print::HeaderFooterFont>| {
            f.as_ref().is_some_and(|f| !f.face.is_empty())
        };
        Self {
            han: font(FontKind::Han),
            zen: font(FontKind::Zen),
            header: own(&setting.header_font).then(|| font(FontKind::Header)),
            footer: own(&setting.footer_font).then(|| font(FontKind::Footer)),
        }
    }

    fn of(&self, kind: FontKind) -> &CFRetained<CTFont> {
        match kind {
            FontKind::Han => &self.han,
            FontKind::Zen => &self.zen,
            FontKind::Header => self.header.as_ref().unwrap_or(&self.han),
            FontKind::Footer => self.footer.as_ref().unwrap_or(&self.han),
        }
    }
}

/// The Core Graphics context of an AppKit one. The print panel draws its preview through a
/// proxy (NSPrintPreviewGraphicsContext, an NSProxy), which forwards `CGContext` to the context
/// behind it. objc2's checks in debug builds look for the method on the proxy's class, don't
/// find it and panic, so this sends the message as Objective-C does, without them.
fn cg_context(context: &NSGraphicsContext) -> Option<CFRetained<CGContext>> {
    type MsgSend = unsafe extern "C-unwind" fn(*const NSGraphicsContext, Sel) -> *mut CGContext;
    // SAFETY: `CGContext` takes no arguments and returns a CGContextRef, or NULL.
    let send = unsafe {
        std::mem::transmute::<unsafe extern "C-unwind" fn(), MsgSend>(objc2::ffi::objc_msgSend)
    };
    let cg_context = NonNull::new(unsafe { send(context, sel!(CGContext)) })?;
    // SAFETY: a valid CGContext, which the graphics context owns; this retains it.
    Some(unsafe { CFRetained::retain(cg_context) })
}

/// Draws a page from the top-left corner of the paper, in a flipped context.
fn draw_page(context: &CGContext, page: &Page, layout: &Layout, fonts: &Fonts) {
    CGContext::set_rgb_fill_color(Some(context), 0., 0., 0., 1.);
    for text in &page.texts {
        draw_text(context, text, layout, fonts);
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
///
/// A header or a footer in a font of its own is not on the grid: it goes from its `x` with the
/// advances of the font, always from the left (centering needs its width; a follow-up).
fn draw_text(context: &CGContext, text: &PlacedText, layout: &Layout, fonts: &Fonts) {
    // The x of each UTF-16 unit of the text, which is how Core Text counts the characters.
    let grid = on_grid(text.font);
    let mut xs = Vec::new();
    if grid {
        for (_, ch, x) in cell_positions(text, layout) {
            xs.extend(std::iter::repeat_n(points(x), ch.len_utf16()));
        }
    }
    {
        let font = fonts.of(text.font);
        let Some(line) = shape(&text.text, font) else {
            return;
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
            let mut origins: Vec<CGPoint> = if grid {
                indices
                    .iter()
                    .map(|&i| CGPoint::new(xs.get(i as usize).copied().unwrap_or_default(), 0.))
                    .collect()
            } else {
                let mut positions = vec![CGPoint::new(0., 0.); count];
                // SAFETY: the buffer holds `count` items, and an empty range means all of them.
                unsafe {
                    run.positions(
                        CFRange::new(0, 0),
                        NonNull::new_unchecked(positions.as_mut_ptr()),
                    );
                }
                let left = points(text.x);
                positions
                    .iter()
                    .map(|p| CGPoint::new(left + p.x, 0.))
                    .collect()
            };
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
