//! The file dialogs of macOS: `NSOpenPanel` and `NSSavePanel` with `directoryURL`, as a sheet on
//! the window when there is one. Not run by CI (CI only builds it); the user checks it on a Mac.
//!
//! AppKit's panels belong to the main thread: [`show`] is to be polled there (GPUI's foreground
//! executor does), and answers when the user closes the panel.

// The Objective-C calls are `unsafe`. This module and the others of `ume-platform` that talk to
// the OS are the only places of the program that need it (ADR-0013).
#![allow(unsafe_code)]

use std::cell::Cell;
use std::path::PathBuf;

use block2::RcBlock;
use futures_channel::oneshot;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSModalResponse, NSOpenPanel, NSSavePanel, NSWindow};
use objc2_foundation::{NSArray, NSString, NSURL};
use raw_window_handle::RawWindowHandle;

use super::{Error, Kind, Request};

/// `NSModalResponseOK`.
const OK: NSModalResponse = 1;

/// The AppKit window the panel is a sheet of, if the handle is one.
#[derive(Default)]
pub struct Owner(Option<Retained<NSWindow>>);

impl Owner {
    pub fn from_handle(handle: RawWindowHandle) -> Self {
        Self(crate::print::mac::ns_window(handle))
    }
}

pub(super) async fn show(owner: Owner, request: Request) -> Result<Option<Vec<PathBuf>>, Error> {
    let mtm = MainThreadMarker::new().ok_or_else(|| {
        Error::Unavailable("AppKit's panels belong to the main thread".to_owned())
    })?;
    let (done, answer) = oneshot::channel();
    let (save, open) = match request.kind {
        Kind::SaveFile => (NSSavePanel::savePanel(mtm), None),
        Kind::OpenFile { .. } | Kind::Folder => {
            let open = NSOpenPanel::openPanel(mtm);
            (Retained::into_super(open.clone()), Some(open))
        }
    };
    if let Some(open) = &open {
        let folder = request.kind == Kind::Folder;
        open.setCanChooseFiles(!folder);
        open.setCanChooseDirectories(folder);
        open.setAllowsMultipleSelection(matches!(request.kind, Kind::OpenFile { multiple: true }));
    }
    save.setMessage(Some(&NSString::from_str(&request.title)));
    if let Some(name) = &request.name {
        save.setNameFieldStringValue(&NSString::from_str(name));
    }
    if let Some(start) = request.start.as_deref().and_then(|p| p.to_str()) {
        let url = NSURL::fileURLWithPath_isDirectory(&NSString::from_str(start), true);
        save.setDirectoryURL(Some(&url));
    }
    if request.kind == Kind::SaveFile {
        restrict_to_file_types(&save, &request);
    }
    let done = Cell::new(Some(done));
    let panel = save.clone();
    let handler = RcBlock::new(move |response: NSModalResponse| {
        let chosen = (response == OK).then(|| match &open {
            Some(open) => open
                .URLs()
                .to_vec()
                .iter()
                .filter_map(|url| path_of(url))
                .collect(),
            None => panel
                .URL()
                .and_then(|url| path_of(&url))
                .into_iter()
                .collect(),
        });
        if let Some(done) = done.take() {
            done.send(chosen).ok();
        }
    });
    match &owner.0 {
        Some(window) => save.beginSheetModalForWindow_completionHandler(window, &handler),
        None => save.beginWithCompletionHandler(&handler),
    }
    let chosen: Option<Vec<PathBuf>> = answer
        .await
        .map_err(|_| Error::Failed("the panel was dropped".to_owned()))?;
    Ok(chosen.filter(|paths| !paths.is_empty()))
}

/// The panel has no list of file types. The selected type gives the extensions the name may have
/// (`allowedFileTypes`): the panel adds the first to a name typed without one, and other names are
/// still allowed, also when the type has `*.*` next to its extensions. A type that names no extension
/// (`*.*` alone) gives the default extension to add.
fn restrict_to_file_types(panel: &NSSavePanel, request: &Request) {
    let chosen = request.filters.get(request.selected_filter);
    let mut extensions: Vec<&str> = chosen.map_or_else(Vec::new, |filter| filter.extensions());
    if extensions.is_empty() {
        extensions.extend(request.default_extension.as_deref());
    }
    if extensions.is_empty() {
        return;
    }
    let types: Vec<Retained<NSString>> = extensions.into_iter().map(NSString::from_str).collect();
    let types = NSArray::from_retained_slice(&types);
    // `allowedContentTypes` needs UTType; the extensions are all this needs.
    #[allow(deprecated)]
    panel.setAllowedFileTypes(Some(&types));
    panel.setAllowsOtherFileTypes(true);
}

fn path_of(url: &NSURL) -> Option<PathBuf> {
    url.path().map(|path| PathBuf::from(path.to_string()))
}
