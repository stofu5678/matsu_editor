// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/outline/CDlgFuncList.cpp (CDlgFuncList::SetTreeFile,
// CDlgFuncList::SetTreeFileSub, CDlgFuncList::GetTreeFileFullName)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2010, the authors named in the original files (genta, Stonee, JEPRO, hor,
//     MIK, aroka, YAZAKI, Moca, frozen, zenryaku, naoh, little YOSHI, novice, ryoji and others)
//   Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (a Win32 tree view
// whose items carry codes in `lParam`, and `m_pcFuncInfoArr` for the paths) was rewritten in Rust
// as a tree of nodes that holds the paths itself, folders are listed in the background, and the
// tests are new.
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

//! The tree of the file tree: the nodes the items make, and the folders listed below them.
//!
//! Sakura builds the top of the tree from the items (`SetTreeFile`): a Folder item is a node that
//! the items below it (deeper `nDepth`) hang on, a File item is a leaf that opens the file, and a
//! Grep item is a folder whose files and subfolders are listed (`SetTreeFileSub`): at once for the
//! item itself, and for a subfolder when it is opened. A listing shows the subfolders first, then
//! the files, each in the order the file system gives; a folder with nothing in it has no `+`.
//!
//! The tree view of Sakura keeps the codes of the nodes in `lParam` (-1 a file of a listing, -2 a
//! subfolder, -(n*10+3) the Grep item n, -4 a Folder item or a message, n >= 0 a File item). Here
//! a node has its [`Kind`], and the path of every node that stands for a file or a folder.

use std::path::{Component, Path, PathBuf};

use matsu_config::dialog_dir::{MetaSource, expand_meta};
use matsu_config::settings::file_tree::{FileTreeItem, ItemType};

use super::list::{Listing, list};
use super::source::Loaded;

/// The label of a Folder item that has none (`L"Folder"` in `SetTreeFile`).
pub(crate) const DEFAULT_FOLDER_LABEL: &str = "Folder";
/// The message under a Grep item whose patterns cannot be used (`<Wild Card Error>`).
pub(crate) const WILD_CARD_ERROR: &str = "<Wild Card Error>";

/// What a node stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A Folder item: only groups the nodes below it.
    Group,
    /// A File item: opens the file.
    FileItem,
    /// A Grep item: a folder that is listed with the patterns of the item.
    Root,
    /// A subfolder in a listing.
    Dir,
    /// A file in a listing: opens the file.
    File,
    /// `<Wild Card Error>`.
    Message,
}

/// Whether the folder of a node has been listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Load {
    /// Nothing to list (or listed).
    Done,
    /// To be listed when it is opened (or at once, for a Grep item).
    Pending,
    /// Being listed.
    Loading,
}

#[derive(Clone, Debug)]
pub(crate) struct Node {
    pub label: String,
    pub kind: Kind,
    /// The file or folder the node stands for.
    pub path: Option<PathBuf>,
    pub children: Vec<usize>,
    /// The Grep item the node is listed with (`Root` and what is below it).
    pub item: Option<usize>,
    pub load: Load,
}

impl Node {
    /// Whether the node can be opened: it has children, or a listing may give some.
    pub(crate) fn has_children(&self) -> bool {
        !self.children.is_empty() || matches!(self.load, Load::Pending | Load::Loading)
    }

    /// The file a double click or Enter opens.
    pub(crate) fn file(&self) -> Option<&Path> {
        match self.kind {
            Kind::File | Kind::FileItem => self.path.as_deref(),
            _ => None,
        }
    }
}

/// A folder to list, handed to the thread that lists it.
pub(crate) struct LoadJob {
    pub dir: PathBuf,
    pub item: FileTreeItem,
}

impl LoadJob {
    pub(crate) fn run(&self) -> Listing {
        list(&self.dir, &self.item)
    }
}

/// The tree and the items it was made from.
#[derive(Debug, Default)]
pub(crate) struct FileTreeModel {
    pub nodes: Vec<Node>,
    pub roots: Vec<usize>,
    pub items: Vec<FileTreeItem>,
}

/// A path as the tree shows it and opens it: made absolute with `base`, without the `.` parts.
fn absolute(base: &Path, text: &str) -> PathBuf {
    let path = Path::new(text);
    let full = if path.is_absolute() {
        path.to_owned()
    } else if path.has_root() {
        // Windows: `\shared` has a root but no drive. Sakura's current folder is the folder of the
        // document, so the drive (or the UNC share) of `base` goes in front of it. (A path with a
        // drive but no root, `D:foo`, is left as it is.)
        let mut full = PathBuf::new();
        if let Some(Component::Prefix(prefix)) = base.components().next() {
            full.push(prefix.as_os_str());
        }
        full.push(path);
        full
    } else {
        base.join(path)
    };
    full.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect()
}

impl FileTreeModel {
    /// `SetTreeFile`: the nodes of the items. `base` is the current folder; the Grep items are
    /// to be listed ([`Load::Pending`]), see [`Self::pending_roots`].
    pub(crate) fn build(loaded: &Loaded, base: &Path, meta: &dyn MetaSource) -> Self {
        let mut model = Self {
            items: loaded.items.clone(),
            ..Self::default()
        };
        // the nodes the next items hang on: the top of the tree, then each Folder item
        let mut parents: Vec<Option<usize>> = vec![None];
        for (index, item) in loaded.items.iter().enumerate() {
            // ExpandMetaToFolder, then `<iniroot>` (only when a project file gave the items)
            let mut text = expand_meta(&item.target_path, meta).text;
            if loaded.project_ini.is_some() {
                let root = loaded
                    .ini_root
                    .as_deref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                text = text.replace("<iniroot>", &root);
            }
            // the settings are written with Windows paths; a backslash separates folders on every
            // OS (as it does in the patterns of Grep)
            if !cfg!(windows) {
                text = text.replace('\\', "/");
            }
            // `while( item.m_nDepth < (int)hParentTree.size() - 1 )`: the top is never left
            while parents.len() > 1
                && i64::from(item.depth) < i64::try_from(parents.len()).unwrap_or(i64::MAX) - 1
            {
                parents.pop();
            }
            let parent = parents.last().copied().flatten();
            // `GetLongFileName` makes the path absolute; the label is that path when the item has
            // none. A path that cannot be made absolute (an empty one) is left as it is.
            let path = (!text.is_empty()).then(|| absolute(base, &text));
            let label = if item.label.is_empty() {
                path.as_ref()
                    .map_or_else(String::new, |p| p.to_string_lossy().into_owned())
            } else {
                item.label.clone()
            };
            match item.item_type {
                ItemType::Grep => {
                    // a path that names nothing lists nothing
                    let load = if path.is_some() {
                        Load::Pending
                    } else {
                        Load::Done
                    };
                    model.push(parent, label, Kind::Root, path, Some(index), load);
                }
                ItemType::File => {
                    model.push(parent, label, Kind::FileItem, path, None, Load::Done);
                }
                ItemType::Folder => {
                    let label = if item.label.is_empty() {
                        DEFAULT_FOLDER_LABEL.to_owned()
                    } else {
                        item.label.clone()
                    };
                    let node = model.push(parent, label, Kind::Group, None, None, Load::Done);
                    parents.push(Some(node));
                }
                ItemType::Other(_) => {}
            }
        }
        model
    }

    fn push(
        &mut self,
        parent: Option<usize>,
        label: String,
        kind: Kind,
        path: Option<PathBuf>,
        item: Option<usize>,
        load: Load,
    ) -> usize {
        let id = self.nodes.len();
        self.nodes.push(Node {
            label,
            kind,
            path,
            children: Vec::new(),
            item,
            load,
        });
        match parent {
            Some(parent) => self.nodes[parent].children.push(id),
            None => self.roots.push(id),
        }
        id
    }

    /// The Grep items, which are listed as soon as the tree is made.
    pub(crate) fn pending_roots(&self) -> Vec<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == Kind::Root && n.load == Load::Pending)
            .map(|(id, _)| id)
            .collect()
    }

    /// Starts listing the folder of `node` if that is still to do. `None` for a node that has
    /// been listed, is being listed, or has no folder.
    pub(crate) fn begin_load(&mut self, node: usize) -> Option<LoadJob> {
        let n = self.nodes.get_mut(node)?;
        if n.load != Load::Pending {
            return None;
        }
        let (dir, item) = (n.path.clone()?, self.items.get(n.item?)?.clone());
        n.load = Load::Loading;
        Some(LoadJob { dir, item })
    }

    /// Puts the listing of the folder of `node` below it (`SetTreeFileSub`).
    pub(crate) fn finish_load(&mut self, node: usize, listing: Listing) {
        let Some(n) = self.nodes.get_mut(node) else {
            return;
        };
        n.load = Load::Done;
        let (dir, item) = (n.path.clone(), n.item);
        match listing {
            Listing::WildCardError => {
                self.push(
                    Some(node),
                    WILD_CARD_ERROR.to_owned(),
                    Kind::Message,
                    None,
                    None,
                    Load::Done,
                );
            }
            Listing::Entries { folders, files } => {
                let Some(dir) = dir else {
                    return;
                };
                for name in folders {
                    let path = dir.join(&name);
                    self.push(Some(node), name, Kind::Dir, Some(path), item, Load::Pending);
                }
                for name in files {
                    let path = dir.join(&name);
                    self.push(Some(node), name, Kind::File, Some(path), item, Load::Done);
                }
            }
        }
    }
}
