//! Files that were asked to open in view mode (the third argument of `FileOpen` in a macro,
//! `F_FILEOPEN2`). The file is read later, in this window or in a new one, so the request waits
//! here until the read ends and the document is made (`load::load_here`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gpui::{App, Global};

#[derive(Default)]
struct ViewModeRequests(HashSet<PathBuf>);

impl Global for ViewModeRequests {}

/// The next read of `path` makes a document in view mode.
pub(crate) fn request(cx: &mut App, path: &Path) {
    cx.default_global::<ViewModeRequests>()
        .0
        .insert(path.to_owned());
}

/// Whether `path` was asked for in view mode; the request is used up.
pub(crate) fn take(cx: &mut App, path: &Path) -> bool {
    cx.default_global::<ViewModeRequests>().0.remove(path)
}
