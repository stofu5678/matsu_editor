// Derived from Sakura Editor (zlib License):
//   sakura_core/env/CFileNameManager.cpp (GetTransformFileNameFast, TransformFileName_MakeCache,
//   GetFilePathFormat)
//
// Copyright (C) 2008, kobake
// Copyright (C) 2018-2026, Sakura Editor Organization
//
// Modified: rewritten in Rust on strings. The meta folders come from `ume_config::dialog_dir`
// and `ume_platform::folders`; the width of a path is counted in columns (see `short_path`).
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! How a path is shown: Sakura's 「ファイル名表示」 (`CFileNameManager`).
//!
//! The common settings (`[Nickname]`) have a list of replacements (`From00` and `To00` ...) and the
//! short form of a long path (`bShortPath`, `nShortPathMaxWidth`). [`FileNames::transform`] does
//! what `GetTransformFileNameFast` does: it replaces the parts of the path, in the order of the
//! list and without regard to case, and then shortens it. Sakura uses the result in
//!
//! * the title of the window, `$N` (and `$E`, the folder) of the title format ([`title`]),
//! * the tooltip of a tab (`CTabWnd::GetTabName`) and the tab list when it shows the full paths,
//! * the window list and the list of the DIFF dialogs (`GetMenuFullLabel_WinList`),
//! * the recent files and the recent folders of the File menu (`GetMenuFullLabel_MRU`).
//!
//! The text of a tab is the tab caption format (`szTabWndCaption`), whose `$f` is the file name.
//!
//! # The meta folders
//!
//! The `From` side may hold meta folders, `%DeskTop%\` for example, which stand for a folder of
//! the user (`ExpandMetaToFolder`, [`ume_config::dialog_dir::expand_meta`]). Sakura reads their
//! paths from the Windows registry. Here:
//!
//! * Windows: the same folders, from the shell (`ume_platform::folders`): `%DeskTop%`,
//!   `%Personal%` (Documents), `%AppData%`, `%Cache%`, `%Common DeskTop%`, `%Common Documents%` and
//!   the others of the key `Shell Folders`; `%TEMP%` is the environment variable.
//! * macOS and Linux: `%DeskTop%` is `~/Desktop` (on Linux the folder of `user-dirs.dirs`),
//!   `%Personal%` is `~/Documents`, `%My Pictures%`, `%My Music%` and `%Video%` are `~/Pictures`,
//!   `~/Music` and `~/Movies` (`~/Videos`); `%TEMP%` and `%TMP%` are `$TMPDIR` (else `/tmp`).
//!   The folders of Windows (`%AppData%`, `%Cache%`, `%Common ...%`) do not exist: a meta
//!   folder with no folder is left as typed and replaces nothing. `\` in the list is `/`.
//! * `%SAKURA%` is the folder of the program and `%SAKURADATA%` the folder of the settings file.

mod short_path;
pub(crate) mod title;

#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::Arc;

use gpui::{App, Global};
use ume_config::dialog_dir::{MetaSource, expand_meta};

use crate::common_settings::table::Row;

/// `STR_NO_TITLE1`: the name of a document that has no file.
pub(crate) const NO_TITLE: &str = "(無題)";

/// The settings of 「ファイル名表示」, ready to use: the replacements have their meta folders
/// expanded (`TransformFileName_MakeCache`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FileNames {
    short_path: bool,
    /// The most columns of a path (`nShortPathMaxWidth`).
    max_width: usize,
    /// The expanded text to look for and its replacement, in order.
    replacements: Arc<Vec<(Vec<char>, String)>>,
}

impl Global for FileNames {}

impl FileNames {
    /// Settings that show the paths as they are.
    pub(crate) fn plain() -> Self {
        Self::default()
    }

    /// `rows` are the list of the page: the text to look for (with its meta folders) and the
    /// replacement. A row whose meta folders cannot be expanded is left out, and so is an empty
    /// one. `native_separators` makes `\` a `/` in the list (the OSes that are not Windows).
    pub(crate) fn new(
        short_path: bool,
        max_width: usize,
        rows: &[Row],
        meta: &dyn MetaSource,
        native_separators: bool,
    ) -> Self {
        let separators = |text: &str| {
            if native_separators {
                text.replace('\\', "/")
            } else {
                text.to_owned()
            }
        };
        let replacements = rows
            .iter()
            .filter_map(|row| {
                let from = row.first().filter(|from| !from.is_empty())?;
                let expanded = expand_meta(from, meta);
                if !expanded.complete || expanded.text.is_empty() {
                    return None;
                }
                let to = row.get(1).map(String::as_str).unwrap_or_default();
                Some((separators(&expanded.text).chars().collect(), separators(to)))
            })
            .collect();
        Self {
            short_path,
            max_width,
            replacements: Arc::new(replacements),
        }
    }

    /// `GetTransformFileNameFast`: the path with the replacements done, then, if the setting
    /// says so, shortened to the width of the setting. With `fit` a result that is still too
    /// long is cut (the title, the menus); the tooltip of a tab does not.
    pub(crate) fn transform(&self, path: &str, fit: bool) -> String {
        let mut out = path.to_owned();
        for (from, to) in self.replacements.iter() {
            out = replace_ignoring_case(&out, from, to);
        }
        if self.short_path {
            short_path::short_path(&out, self.max_width, fit)
        } else {
            out
        }
    }

    /// The name of a document in the lists and the tooltips: the path as `transform` shows it,
    /// or 「(無題)」 for a document with no file.
    pub(crate) fn name_of(&self, path: Option<&Path>, fit: bool) -> String {
        path.map_or_else(
            || NO_TITLE.to_owned(),
            |path| self.transform(&path.to_string_lossy(), fit),
        )
    }
}

/// `GetFilePathFormat`: every `from` in `src` becomes `to`. The characters are compared without
/// regard to case (`_wcsnicmp`).
fn replace_ignoring_case(src: &str, from: &[char], to: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < chars.len() {
        let rest = &chars[i..];
        if rest.len() >= from.len()
            && !from.is_empty()
            && rest.iter().zip(from).all(|(a, b)| same_letter(*a, *b))
        {
            out.push_str(to);
            i += from.len();
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn same_letter(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

/// The meta folders of this machine for the list: those of the file dialogs, and on the OSes
/// that are not Windows the temporary folder for `%TEMP%` and `%TMP%`.
pub(crate) struct Folders(crate::file_dialog::Meta);

impl Folders {
    pub(crate) fn new(cx: &App) -> Self {
        Self(crate::file_dialog::Meta::new(cx))
    }
}

impl MetaSource for Folders {
    fn program_dir(&self) -> Option<String> {
        self.0.program_dir()
    }

    fn settings_dir(&self) -> Option<String> {
        self.0.settings_dir()
    }

    fn shell_folder(&self, name: &str) -> Option<String> {
        self.0.shell_folder(name)
    }

    fn env_var(&self, name: &str) -> Option<String> {
        self.0.env_var(name).or_else(|| {
            let temporary = name.eq_ignore_ascii_case("TEMP") || name.eq_ignore_ascii_case("TMP");
            (temporary && !cfg!(windows)).then(|| {
                std::env::var("TMPDIR")
                    .ok()
                    .map(|dir| dir.trim_end_matches('/').to_owned())
                    .filter(|dir| !dir.is_empty())
                    .unwrap_or_else(|| "/tmp".to_owned())
            })
        })
    }
}

/// The settings the windows use now.
pub(crate) fn current(cx: &App) -> FileNames {
    cx.try_global::<FileNames>().cloned().unwrap_or_default()
}

/// Meta folders nobody has, for the tests of the places that show paths.
#[cfg(test)]
pub(crate) struct NoFolders;

#[cfg(test)]
impl MetaSource for NoFolders {
    fn program_dir(&self) -> Option<String> {
        None
    }

    fn settings_dir(&self) -> Option<String> {
        None
    }

    fn shell_folder(&self, _: &str) -> Option<String> {
        None
    }

    fn env_var(&self, _: &str) -> Option<String> {
        None
    }
}
