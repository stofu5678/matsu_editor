// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for UME Editor.
//
// Original files: sakura_core/outline/CDlgFuncList.cpp (CDlgFuncList::SetTreeFileSub)
//
// Original copyright notices of that file:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2010, the authors named in the original file (genta, Stonee, JEPRO, hor,
//     MIK, aroka, YAZAKI, Moca, frozen, zenryaku, naoh, little YOSHI, novice, ryoji and others)
//   Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the file above and was modified: the C++ code (tree view items) was
// rewritten in Rust on `Walker`, which is the port of the Grep enumerators, and the tests are new.
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

//! Listing one folder of a Grep item: the subfolders and the files the patterns of the item pick.
//!
//! The patterns are the ファイル box of Grep (`CGrepEnumKeys::SetFileKeys`: `*.rs;*.toml`, `!*.bak`
//! for files left out, `#target` for folders left out); the three options leave out hidden,
//! read-only and system entries (`CGrepEnumOptions`).

use std::path::Path;

use crate::grep::enumerate::{EnumOptions, Walker};
use crate::grep::keys::GrepKeys;
use ume_config::settings::file_tree::FileTreeItem;

/// What a folder holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Listing {
    /// The names of the subfolders and of the files, each in the order of their names.
    Entries {
        folders: Vec<String>,
        files: Vec<String>,
    },
    /// The patterns cannot be used (a wild card in the folder part, a full path to search).
    WildCardError,
}

/// `SetTreeFileSub`: lists `dir` with the patterns and the options of `item`.
pub(crate) fn list(dir: &Path, item: &FileTreeItem) -> Listing {
    let Ok(keys) = GrepKeys::parse(&item.target_file, false) else {
        return Listing::WildCardError;
    };
    let options = EnumOptions {
        ignore_hidden: item.ignore_hidden,
        ignore_read_only: item.ignore_read_only,
        ignore_system: item.ignore_system,
    };
    let mut walker = Walker::with_options(keys, Vec::new(), options);
    let dir = dir.to_string_lossy();
    // the subfolders first, then the files
    let folders = walker.folders_in(&dir);
    let files = walker.files_in(&dir);
    Listing::Entries { folders, files }
}
