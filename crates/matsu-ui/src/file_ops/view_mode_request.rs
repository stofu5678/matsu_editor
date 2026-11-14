//! Files that were asked to open in view mode (the third argument of `FileOpen` in a macro,
//! `F_FILEOPEN2`, and the ReadMe of a plugin). The file is read later, in this window or in a new
//! one, so the request waits here until the read ends and the document is made
//! (`load::load_here`).
//!
//! Requests are counted per path: two requests for the same file that are made before the first
//! read ends (the ReadMe button pressed twice) are two requests, and each read uses up one.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui::{App, Global};

#[derive(Default)]
struct ViewModeRequests(HashMap<PathBuf, usize>);

impl Global for ViewModeRequests {}

/// The next read of `path` makes a document in view mode.
pub(crate) fn request(cx: &mut App, path: &Path) {
    *cx.default_global::<ViewModeRequests>()
        .0
        .entry(path.to_owned())
        .or_default() += 1;
}

/// Whether `path` was asked for in view mode; one request is used up.
pub(crate) fn take(cx: &mut App, path: &Path) -> bool {
    let requests = &mut cx.default_global::<ViewModeRequests>().0;
    match requests.get_mut(path) {
        Some(count) if *count > 1 => {
            *count -= 1;
            true
        }
        Some(_) => {
            requests.remove(path);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn two_requests_for_a_path_are_used_up_one_by_one(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let path = Path::new("a.txt");
            assert!(!take(cx, path));
            request(cx, path);
            request(cx, path);
            assert!(take(cx, path));
            assert!(take(cx, path));
            assert!(!take(cx, path));
        });
    }
}
