//! What a window takes from the type of its document: the IME at the start (起動時のIME,
//! `nImeState`) and the icon of the window (「文書アイコンを使う」, `bUseDocumentIcon`).
//!
//! Sakura sets the IME when a document has been loaded (`CDocEditor::OnAfterLoad`) and when the
//! window is made (`CEditWnd::Create`), and the icon whenever the file name or the settings of the
//! type change (`CEditDoc::SetFilePathAndIcon`, `CDocType::SetDocumentIcon`). Each of Sakura's
//! windows has one document; here a window has tabs, so the IME is set once for each document the
//! window shows, while the window is active (the first time it is on view, and again when a file is
//! loaded into it: opened into an untitled tab, closed and opened, read again), not at every change
//! of the tab. The icon follows the document that is on view.
//!
//! The OS calls are in `matsu_platform::ime_mode` and `matsu_platform::doc_icon`; macOS and Linux
//! have neither (the settings are greyed out there).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use gpui::{App, Window, WindowId};
use raw_window_handle::HasWindowHandle;

use crate::view::EditorView;

/// What a window has been given already.
#[derive(Default)]
struct Seen {
    /// The documents (and their load, and their type) whose IME state has been set.
    ime: HashSet<(u64, u64, usize)>,
    /// The file the icon of the window is that of, `None` for the icon of the application.
    icon: Option<PathBuf>,
}

impl Seen {
    /// Whether the IME of this document is still to be set; it is not asked twice.
    fn ime_is_due(&mut self, document: u64, generation: u64, type_index: usize) -> bool {
        self.ime.insert((document, generation, type_index))
    }

    /// The icon to give the window when it is not the one it has: `Some(None)` for the icon of
    /// the application.
    fn icon_to_set(&mut self, want: Option<&Path>) -> Option<Option<PathBuf>> {
        if self.icon.as_deref() == want {
            return None;
        }
        self.icon = want.map(Path::to_path_buf);
        Some(self.icon.clone())
    }
}

/// The path whose icon the window wears: the file of the document when the type says so.
fn icon_file(use_document_icon: bool, path: Option<&Path>) -> Option<&Path> {
    path.filter(|_| use_document_icon)
}

thread_local! {
    static SEEN: RefCell<HashMap<WindowId, Seen>> = RefCell::new(HashMap::new());
}

/// Applies the IME state and the icon of the type of `editor`'s document to its window. Called
/// when the window is drawn; it does the OS calls only when something changed.
pub(crate) fn sync(window: &Window, editor: &EditorView, cx: &App) {
    // Where the OS has neither, there is nothing to ask of it.
    let ime = matsu_platform::ime_mode::unsupported().is_none();
    let icon = matsu_platform::doc_icon::unsupported().is_none();
    if !ime && !icon {
        return;
    }
    let settings = &editor.prefs.settings;
    let document = editor.document();
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let raw = handle.as_raw();
    let id = window.window_handle().window_id();
    SEEN.with(|seen| {
        let mut seen = seen.borrow_mut();
        let seen = seen.entry(id).or_default();
        // The default input context belongs to the thread, not to the window: a document that
        // loads in a window the user has left waits until the window is active again.
        if ime
            && window.is_window_active()
            && seen.ime_is_due(
                document.entity_id().as_u64(),
                document.read(cx).generation(),
                settings.type_index,
            )
        {
            // A failure leaves the IME as the user's; there is no one to tell.
            let _ = matsu_platform::ime_mode::apply(raw, settings.ime_state);
        }
        if icon {
            let path = document.read(cx).path();
            let want = icon_file(settings.use_document_icon, path);
            if let Some(file) = seen.icon_to_set(want) {
                let _ = matsu_platform::doc_icon::set(raw, file.as_deref());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ime_of_a_document_is_set_once() {
        let mut seen = Seen::default();
        let (a, b) = (1, 2);
        assert!(seen.ime_is_due(a, 0, 0));
        assert!(
            !seen.ime_is_due(a, 0, 0),
            "not again when the tab is shown again"
        );
        assert!(seen.ime_is_due(b, 0, 0), "another document");
        assert!(seen.ime_is_due(a, 0, 3), "the type of the document changed");
        assert!(
            seen.ime_is_due(a, 1, 0),
            "a file opened into the tab, or the file read again"
        );
    }

    #[test]
    fn the_icon_is_given_when_it_changes() {
        let mut seen = Seen::default();
        // a window starts with the icon of the application: nothing to do for it
        assert_eq!(seen.icon_to_set(None), None);
        let a = Path::new("a.txt");
        assert_eq!(seen.icon_to_set(Some(a)), Some(Some(a.to_path_buf())));
        assert_eq!(seen.icon_to_set(Some(a)), None);
        assert_eq!(
            seen.icon_to_set(None),
            Some(None),
            "back to the icon of the application"
        );
    }

    #[test]
    fn the_icon_follows_the_file_only_when_the_type_says_so() {
        let path = Path::new("a.txt");
        assert_eq!(icon_file(true, Some(path)), Some(path));
        assert_eq!(icon_file(false, Some(path)), None);
        assert_eq!(
            icon_file(true, None),
            None,
            "a document with no file has the icon of the application"
        );
    }
}
