//! The tree of the dialog 「ファイルツリー設定」 and what its buttons do to it.
//!
//! Sakura keeps the items in a tree view (`IDC_TREE_FL`) and the buttons of `CDlgFileTree` move
//! its nodes around (`sakura_core/outline/CDlgFileTree.cpp`, `OnBnClicked`). Here the tree is a
//! plain structure, so that the rules can be tested without a window. A [`Path`] names a node by
//! the position in each list from the top (`[2, 0]` is the first child of the third item).
//!
//! Only a Folder item (`EFileTreeItemType_Folder`) takes children (`cChildren` is 1 for it alone).

use ume_config::settings::file_tree::{FileTreeItem, ItemType};

/// Where a node is: the position in the list of the top, then in the list of its children, ...
pub(crate) type Path = Vec<usize>;

/// `_MAX_PATH`: `SFilePath` holds this many UTF-16 units with the end of the text.
const MAX_PATH: usize = 260;

/// A node of the tree view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Node {
    pub item: FileTreeItem,
    pub children: Vec<Node>,
    /// Whether the children are shown (`TVIS_EXPANDED`).
    pub expanded: bool,
}

impl Node {
    fn new(item: FileTreeItem) -> Self {
        Self {
            item,
            children: Vec::new(),
            expanded: false,
        }
    }
}

/// The text of a node (`GetFileTreeLabel`): the label, else the path (the current folder has a
/// name of its own), and for a Folder item the label, else `Folder`.
pub(crate) fn node_label(item: &FileTreeItem) -> String {
    if item.item_type == ItemType::Folder {
        if item.label.is_empty() {
            "Folder".to_owned()
        } else {
            item.label.clone()
        }
    } else if !item.label.is_empty() {
        item.label.clone()
    } else if matches!(item.target_path.as_str(), "." | ".\\" | "./") {
        // `STR_FILETREE_CURDIR`
        "<カレントフォルダー>".to_owned()
    } else {
        item.target_path.clone()
    }
}

fn takes_children(item: &FileTreeItem) -> bool {
    item.item_type == ItemType::Folder
}

/// A row of the tree as drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Row {
    pub path: Path,
    pub depth: usize,
    pub label: String,
    /// The node can be opened (a Folder item with something in it).
    pub has_children: bool,
    pub expanded: bool,
}

/// The tree and the selected node.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tree {
    roots: Vec<Node>,
    selected: Option<Path>,
}

impl Tree {
    /// `CDlgFileTree::SetData`: the items in order, each below the Folder item that the depths
    /// say (an item deeper than the last Folder item goes under it all the same). Nothing is
    /// open, and the first item is selected.
    pub(crate) fn from_items(items: &[FileTreeItem]) -> Self {
        let mut roots: Vec<Node> = Vec::new();
        // the path of each Folder item that can take the next item, from the top
        let mut parents: Vec<Path> = Vec::new();
        for item in items {
            while usize::try_from(item.depth).unwrap_or(0) < parents.len() {
                parents.pop();
            }
            let list = list_mut(&mut roots, &parents.last().cloned().unwrap_or_default());
            let at = list.len();
            let is_folder = takes_children(item);
            list.push(Node::new(item.clone()));
            if is_folder {
                let mut path = parents.last().cloned().unwrap_or_default();
                path.push(at);
                parents.push(path);
            }
        }
        let selected = (!roots.is_empty()).then(|| vec![0]);
        Self { roots, selected }
    }

    /// `CDlgFileTree::GetDataTree`: the items depth first with their depth in the tree, at most
    /// `max` of them (`0` means all). The flag is false when some were left out.
    pub(crate) fn to_items(&self, max: usize) -> (Vec<FileTreeItem>, bool) {
        let mut out = Vec::new();
        let complete = collect(&self.roots, 0, max, &mut out);
        (out, complete)
    }

    /// How many items there are (`TreeView_GetCount`).
    pub(crate) fn count(&self) -> usize {
        fn count(list: &[Node]) -> usize {
            list.iter().map(|n| 1 + count(&n.children)).sum()
        }
        count(&self.roots)
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub(crate) fn selected(&self) -> Option<&Path> {
        self.selected.as_ref()
    }

    pub(crate) fn selected_item(&self) -> Option<&FileTreeItem> {
        self.node(self.selected.as_ref()?).map(|n| &n.item)
    }

    pub(crate) fn node(&self, path: &[usize]) -> Option<&Node> {
        let (first, rest) = path.split_first()?;
        let mut node = self.roots.get(*first)?;
        for index in rest {
            node = node.children.get(*index)?;
        }
        Some(node)
    }

    fn node_mut(&mut self, path: &[usize]) -> Option<&mut Node> {
        let (first, rest) = path.split_first()?;
        let mut node = self.roots.get_mut(*first)?;
        for index in rest {
            node = node.children.get_mut(*index)?;
        }
        Some(node)
    }

    /// Selects a node (`TreeView_SelectItem`); its parents are opened. A path that is not there
    /// changes nothing.
    pub(crate) fn select(&mut self, path: &[usize]) {
        if self.node(path).is_none() {
            return;
        }
        for len in 1..path.len() {
            if let Some(parent) = self.node_mut(&path[..len]) {
                parent.expanded = true;
            }
        }
        self.selected = Some(path.to_vec());
    }

    /// Opens or closes a node (the `+` button).
    pub(crate) fn toggle(&mut self, path: &[usize]) {
        if let Some(node) = self.node_mut(path)
            && !node.children.is_empty()
        {
            node.expanded = !node.expanded;
            // a selected node below a closed one is not shown: the closed node is selected
            if !node.expanded
                && let Some(selected) = &self.selected
                && selected.len() > path.len()
                && selected.starts_with(path)
            {
                self.selected = Some(path.to_vec());
            }
        }
    }

    /// The rows that are shown, from the top.
    pub(crate) fn rows(&self) -> Vec<Row> {
        fn walk(list: &[Node], prefix: &mut Path, out: &mut Vec<Row>) {
            for (index, node) in list.iter().enumerate() {
                prefix.push(index);
                out.push(Row {
                    path: prefix.clone(),
                    depth: prefix.len() - 1,
                    label: node_label(&node.item),
                    has_children: !node.children.is_empty(),
                    expanded: node.expanded,
                });
                if node.expanded {
                    walk(&node.children, prefix, out);
                }
                prefix.pop();
            }
        }
        let mut out = Vec::new();
        walk(&self.roots, &mut Vec::new(), &mut out);
        out
    }

    /// The selected node has something below it (`TreeView_GetChild`).
    pub(crate) fn selected_has_children(&self) -> bool {
        self.selected
            .as_ref()
            .and_then(|path| self.node(path))
            .is_some_and(|n| !n.children.is_empty())
    }

    /// `IDC_BUTTON_ADD` (`>>`): the item goes last into the selected Folder item; below another
    /// item it goes last into the same list. With nothing selected it goes last into the top list.
    pub(crate) fn add(&mut self, item: FileTreeItem) {
        let parent = match self.selected_item() {
            None => Vec::new(),
            Some(selected) if takes_children(selected) => self.selected.clone().unwrap_or_default(),
            Some(_) => parent_of(self.selected.as_deref().unwrap_or_default()),
        };
        let len = self.list(&parent).len();
        self.insert_at(&parent, len, item);
    }

    /// `IDC_BUTTON_INSERT_A` (`下⇒`): the item goes first into the selected Folder item, or right
    /// after another selected item. With nothing selected it goes last into the top list.
    pub(crate) fn insert_below(&mut self, item: FileTreeItem) {
        let (parent, at) = match (self.selected.clone(), self.selected_item()) {
            (Some(path), Some(selected)) if takes_children(selected) => (path, 0),
            (Some(path), Some(_)) => {
                let at = path.last().copied().unwrap_or(0) + 1;
                (parent_of(&path), at)
            }
            _ => (Vec::new(), self.roots.len()),
        };
        self.insert_at(&parent, at, item);
    }

    /// `IDC_BUTTON_INSERT` (`上⇒`): the item goes right before the selected item, in its list.
    /// With nothing selected it goes first into the top list.
    ///
    /// Sakura puts it first in the top list whenever the selected item is in the top list
    /// (`htiParent == nullptr` takes `TVI_FIRST` without looking at the previous sibling): the
    /// button then means something else for the items of the top list than for the others. Here
    /// it is always "right before the selected item".
    pub(crate) fn insert_above(&mut self, item: FileTreeItem) {
        let (parent, at) = match self.selected.clone() {
            Some(path) if self.node(&path).is_some() => {
                let at = path.last().copied().unwrap_or(0);
                (parent_of(&path), at)
            }
            _ => (Vec::new(), 0),
        };
        self.insert_at(&parent, at, item);
    }

    /// `IDC_BUTTON_FILEADD` (`ファイル挿入`): File items for the files, in order. Into a selected
    /// Folder item they go first (the first of them first); below another item they go last into
    /// its list; with nothing selected, last into the top list. The first of them is selected.
    pub(crate) fn add_files(&mut self, items: Vec<FileTreeItem>) {
        let (parent, mut at) = match (self.selected.clone(), self.selected_item()) {
            (Some(path), Some(selected)) if takes_children(selected) => (path, 0),
            (Some(path), Some(_)) => {
                let parent = parent_of(&path);
                let len = self.list(&parent).len();
                (parent, len)
            }
            _ => (Vec::new(), self.roots.len()),
        };
        let mut first = None;
        for item in items {
            let path = self.put(&parent, at, item);
            first.get_or_insert(path);
            at += 1;
        }
        if let Some(first) = first {
            self.select(&first);
        }
    }

    /// `IDC_BUTTON_UPDATE` (`更新`): the selected item gets the new fields; the children stay.
    /// Returns false when nothing is selected.
    pub(crate) fn update(&mut self, item: FileTreeItem) -> bool {
        let Some(path) = self.selected.clone() else {
            return false;
        };
        match self.node_mut(&path) {
            Some(node) => {
                node.item = item;
                true
            }
            None => false,
        }
    }

    /// `IDC_BUTTON_DELETE` (`削除`): the selected item goes with everything below it. The next item
    /// of the list is selected, else the one before it, else the parent. Returns false when
    /// nothing is selected.
    pub(crate) fn delete(&mut self) -> bool {
        let Some(path) = self.selected.clone() else {
            return false;
        };
        let parent = parent_of(&path);
        let index = path.last().copied().unwrap_or(0);
        let list = list_mut(&mut self.roots, &parent);
        if index >= list.len() {
            return false;
        }
        list.remove(index);
        let len = list.len();
        self.selected = if index < len {
            Some([parent.as_slice(), &[index]].concat())
        } else if index > 0 {
            Some([parent.as_slice(), &[index - 1]].concat())
        } else if parent.is_empty() {
            None
        } else {
            Some(parent)
        };
        true
    }

    /// `IDC_BUTTON_UP` (`↑`): the selected item changes places with the one before it.
    pub(crate) fn move_up(&mut self) -> bool {
        let Some(path) = self.selected.clone() else {
            return false;
        };
        let index = path.last().copied().unwrap_or(0);
        if index == 0 {
            return false;
        }
        let parent = parent_of(&path);
        list_mut(&mut self.roots, &parent).swap(index, index - 1);
        self.selected = Some([parent.as_slice(), &[index - 1]].concat());
        true
    }

    /// `IDC_BUTTON_DOWN` (`↓`): the selected item changes places with the one after it.
    pub(crate) fn move_down(&mut self) -> bool {
        let Some(path) = self.selected.clone() else {
            return false;
        };
        let index = path.last().copied().unwrap_or(0);
        let parent = parent_of(&path);
        let list = list_mut(&mut self.roots, &parent);
        if index + 1 >= list.len() {
            return false;
        }
        list.swap(index, index + 1);
        self.selected = Some([parent.as_slice(), &[index + 1]].concat());
        true
    }

    /// `IDC_BUTTON_RIGHT` (`→`): the selected item (with its children) goes last into the Folder
    /// item before it. Nothing happens when the one before it is not a Folder item.
    pub(crate) fn move_right(&mut self) -> bool {
        let Some(path) = self.selected.clone() else {
            return false;
        };
        let index = path.last().copied().unwrap_or(0);
        if index == 0 {
            return false;
        }
        let parent = parent_of(&path);
        let before = [parent.as_slice(), &[index - 1]].concat();
        if !self.node(&before).is_some_and(|n| takes_children(&n.item)) {
            return false;
        }
        let node = list_mut(&mut self.roots, &parent).remove(index);
        let Some(folder) = self.node_mut(&before) else {
            return false;
        };
        folder.children.push(node);
        folder.expanded = true;
        let new = [before.as_slice(), &[folder.children.len() - 1]].concat();
        self.selected = Some(new);
        true
    }

    /// `IDC_BUTTON_LEFT` (`←`): the selected item (with its children) goes right after its parent,
    /// in the list of the parent. Nothing happens in the top list.
    pub(crate) fn move_left(&mut self) -> bool {
        let Some(path) = self.selected.clone() else {
            return false;
        };
        if path.len() < 2 {
            return false;
        }
        let parent = parent_of(&path);
        let index = path.last().copied().unwrap_or(0);
        let node = list_mut(&mut self.roots, &parent).remove(index);
        let grandparent = parent_of(&parent);
        let at = parent.last().copied().unwrap_or(0) + 1;
        let new = self.put_node(&grandparent, at, node);
        self.selected = Some(new);
        true
    }

    /// `IDC_BUTTON_REPLACE` (`パスの一括置換`): every place of `from` in the path of every item
    /// becomes `to`. A path that would not fit in `_MAX_PATH` stays as it was. An empty `from`
    /// changes nothing (Sakura's `CNativeW::Replace` never ends with it).
    pub(crate) fn replace_paths(&mut self, from: &str, to: &str) {
        fn walk(list: &mut [Node], from: &str, to: &str) {
            for node in list {
                let replaced = node.item.target_path.replace(from, to);
                if replaced.encode_utf16().count() < MAX_PATH {
                    node.item.target_path = replaced;
                }
                walk(&mut node.children, from, to);
            }
        }
        if !from.is_empty() {
            walk(&mut self.roots, from, to);
        }
    }

    fn list(&self, parent: &[usize]) -> &[Node] {
        if parent.is_empty() {
            return &self.roots;
        }
        self.node(parent).map_or(&[], |n| n.children.as_slice())
    }

    /// Puts the item into the list of `parent` at `at`, opens the parent and selects the item.
    fn insert_at(&mut self, parent: &[usize], at: usize, item: FileTreeItem) {
        let path = self.put(parent, at, item);
        self.select(&path);
    }

    fn put(&mut self, parent: &[usize], at: usize, item: FileTreeItem) -> Path {
        self.put_node(parent, at, Node::new(item))
    }

    fn put_node(&mut self, parent: &[usize], at: usize, node: Node) -> Path {
        let list = list_mut(&mut self.roots, parent);
        let at = at.min(list.len());
        list.insert(at, node);
        [parent, &[at]].concat()
    }
}

/// `GetDataTree`.
fn collect(list: &[Node], depth: i32, max: usize, out: &mut Vec<FileTreeItem>) -> bool {
    for node in list {
        if max > 0 && out.len() >= max {
            return false;
        }
        let mut item = node.item.clone();
        item.depth = depth;
        out.push(item);
        if takes_children(&node.item) && !collect(&node.children, depth + 1, max, out) {
            return false;
        }
    }
    true
}

fn parent_of(path: &[usize]) -> Path {
    path.split_last().map_or_else(Vec::new, |(_, p)| p.to_vec())
}

/// The list of the children of `parent` (the top list for an empty path).
fn list_mut<'a>(roots: &'a mut Vec<Node>, parent: &[usize]) -> &'a mut Vec<Node> {
    let mut list = roots;
    for index in parent {
        // the path was made from the tree, so it leads to a node
        list = &mut list[*index].children;
    }
    list
}

#[cfg(test)]
mod tests;
