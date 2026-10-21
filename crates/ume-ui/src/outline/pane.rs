// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for UME Editor.
//
// Original files: sakura_core/outline/CDlgFuncList.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2014, the authors named in the original files (genta, zenryaku, Moca,
//     aroka, hor, YAZAKI, little YOSHI, ryoji, syat and others)
//   Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (a UTF-16 buffer
// walked with `CDocLineMgr`, the results put in `CFuncInfoArr`, a Windows dialog) was rewritten in
// Rust on the lines of `LineSource`, the positions in the layout are left to the window, and the
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

//! The outline pane: Sakura's アウトライン解析 dialog (`IDD_FUNCLIST`) as a pane docked at the right
//! of the window.
//!
//! The analysis runs in the background on a clone of the document; the pane then shows the list
//! (C, Perl, Visual Basic, the bookmarks) or the tree (the others). A click on an item moves the
//! caret there without taking the focus (`OnJump( false, false )` of the docked dialog), Enter and a
//! double click move it and give the focus back to the text (`MYWM_SETACTIVEPANE`), the space key
//! moves it and keeps the focus. After an edit the result is kept but no longer follows the caret
//! (`NotifyDocModification`); 更新, `F_OUTLINE` and a switch to another tab analyze again.
//!
//! Sakura: `sakura_core/outline/CDlgFuncList.cpp`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use gpui::{
    App, ClickEvent, ClipboardItem, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, ScrollStrategy, StatefulInteractiveElement,
    Styled, Subscription, UniformListScrollHandle, WeakEntity, Window, div, px, rgb, uniform_list,
};
use ume_commands::keymap::Resolved;
use ume_core::buffer::Pos;
use ume_outline::tree::{
    build_java_tree, build_tree, clip_text_java, clip_text_list, clip_text_tree, clip_text_vb,
    item_at, list_order, list_remark, prepare_append_texts, sort_tree, vb_remark,
};
use ume_outline::{
    Analysis, ListColumn, ListSort, OutlineKind, Request, SortType, TreeNode, ViewType, analyze,
};

use super::file_tree::model::FileTreeModel;
use super::file_tree::source::{Inputs, load_setting};
use super::{OutlineOptions, Requested, Show};
use crate::document::{DocumentEvent, SharedDocument};
use crate::keys::chord_of;
use crate::view::EditorView;

/// The fixed part of the width of the docked pane when the window is narrow.
const MIN_WIDTH: f32 = 160.0;

const TEXT: u32 = 0x1e1e1e;
const SELECTED_BG: u32 = 0x2f6fd0;
const STALE_TEXT: u32 = 0x777777;

/// One line of what is shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Row {
    /// The number of the node (`lParam`) in a tree, the index of the item in a list.
    pub key: usize,
    /// The item of the analysis, if this is one.
    pub item: Option<usize>,
    pub depth: usize,
    pub label: String,
    pub has_children: bool,
    pub expanded: bool,
    /// The columns of a list: line, column, remark.
    pub columns: Option<[String; 3]>,
    /// The file that a double click or Enter opens (the file tree).
    pub path: Option<PathBuf>,
}

/// What an analysis gave, ready to show.
struct Shown {
    analysis: Analysis,
    title: &'static str,
    view: ViewType,
    tree: Vec<TreeNode>,
    /// The line numbers as the list shows them (the rows of the wrapped view, or the lines).
    list_lines: Vec<usize>,
    clip_text: String,
    unsupported: bool,
}

pub(crate) struct OutlinePane {
    host: WeakEntity<crate::app::Workspace>,
    view: WeakEntity<EditorView>,
    view_id: gpui::EntityId,
    document: Entity<SharedDocument>,
    /// What was asked for: the kind of the type, or the bookmarks (`m_nOutlineType`).
    requested: Requested,
    /// The kind after the file name settled C/C++ (what `CheckListType` compares).
    outline_type: OutlineKind,
    shown: Option<Shown>,
    /// The file tree, when that is what the pane shows (`OUTLINE_FILETREE`).
    file_tree: Option<FileTreeModel>,
    analyzing: bool,
    generation: u64,
    /// The text changed after the analysis (`m_bFuncInfoArrIsUpToDate` is false).
    stale: bool,
    /// The file the result was made for.
    analyzed_path: Option<PathBuf>,
    list_sort: ListSort,
    sort_type: SortType,
    expanded: HashSet<usize>,
    rows: Vec<Row>,
    selected: Option<usize>,
    last_caret: Option<Pos>,
    focus: FocusHandle,
    scroll: UniformListScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl OutlinePane {
    pub(crate) fn new(
        host: WeakEntity<crate::app::Workspace>,
        view: &Entity<EditorView>,
        cx: &mut Context<Self>,
    ) -> Self {
        let document = view.read(cx).document().clone();
        let mut pane = Self {
            host,
            view: view.downgrade(),
            view_id: view.entity_id(),
            document,
            requested: Requested::Default,
            outline_type: OutlineKind::Text,
            shown: None,
            file_tree: None,
            analyzing: false,
            generation: 0,
            stale: false,
            analyzed_path: None,
            list_sort: ListSort::default(),
            sort_type: SortType::default(),
            expanded: HashSet::new(),
            rows: Vec::new(),
            selected: None,
            last_caret: None,
            focus: cx.focus_handle(),
            scroll: UniformListScrollHandle::new(),
            _subscriptions: Vec::new(),
        };
        pane.listen(view, cx);
        pane
    }

    /// Listens to the view (the caret) and its document (edits, a new type).
    fn listen(&mut self, view: &Entity<EditorView>, cx: &mut Context<Self>) {
        let document = view.read(cx).document().clone();
        self.document = document.clone();
        self.view = view.downgrade();
        self.view_id = view.entity_id();
        self.last_caret = None;
        let observe = cx.observe(view, |this, view, cx| this.caret_moved(&view, cx));
        let events = cx.subscribe(&document, |this, _, event, cx| match *event {
            // NotifyDocModification: the result stays, but it no longer follows the caret
            DocumentEvent::Changed { .. } => {
                // a file loaded into this tab replaces the document (`Refresh` after a load)
                let path = this.document.read(cx).path().map(Path::to_path_buf);
                if path != this.analyzed_path {
                    this.analyze(this.requested, cx);
                } else if !this.stale && this.requested != Requested::FileTree {
                    // (the file tree does not depend on the text)
                    this.stale = true;
                    cx.notify();
                }
            }
            // a new type: analyze again when its kind of outline differs (ChangeType)
            DocumentEvent::TypeChanged { .. } => this.type_changed(cx),
        });
        self._subscriptions = vec![observe, events];
    }

    pub(crate) fn outline_type(&self) -> OutlineKind {
        self.outline_type
    }

    pub(crate) fn view_id(&self) -> gpui::EntityId {
        self.view_id
    }

    #[cfg(test)]
    pub(crate) fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub(crate) fn selected_row(&self) -> Option<usize> {
        let key = self.selected?;
        self.rows.iter().position(|r| r.key == key)
    }

    pub(crate) fn title(&self) -> &'static str {
        if self.outline_type == OutlineKind::FileTree {
            return OutlineKind::FileTree.title();
        }
        self.shown.as_ref().map_or("", |s| s.title)
    }

    #[cfg(test)]
    pub(crate) fn is_stale(&self) -> bool {
        self.stale
    }

    #[cfg(test)]
    pub(crate) fn is_analyzing(&self) -> bool {
        self.analyzing
    }

    pub(crate) fn clip_text(&self) -> &str {
        self.shown.as_ref().map_or("", |s| s.clip_text.as_str())
    }

    pub(crate) fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }

    /// Points the pane at another view. A view of the same document keeps the result; another
    /// document is analyzed (`Refresh` when the file changes).
    pub(crate) fn follow(&mut self, view: &Entity<EditorView>, cx: &mut Context<Self>) {
        if view.entity_id() == self.view_id {
            return;
        }
        let same_document = view.read(cx).document().entity_id() == self.document.entity_id();
        self.listen(view, cx);
        if same_document {
            self.caret_moved(view, cx);
        } else {
            // the result belongs to the other document: nothing of it stays to be clicked or
            // copied while the new one is made
            self.shown = None;
            self.rows.clear();
            self.selected = None;
            self.analyze(self.requested, cx);
        }
    }

    fn type_changed(&mut self, cx: &mut Context<Self>) {
        // the file tree may be set for each type (`nOutlineDockSet`)
        if self.requested == Requested::FileTree {
            self.analyze(Requested::FileTree, cx);
            return;
        }
        if self.requested != Requested::Default {
            return;
        }
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let kind = super::resolve_kind(Requested::Default, &view, cx);
        if kind != self.outline_type {
            self.analyze(Requested::Default, cx);
        }
    }

    /// `Command_FUNCLIST`: analyzes the document of the view in the background and shows the
    /// result when it is there.
    pub(crate) fn analyze(&mut self, requested: Requested, cx: &mut Context<Self>) {
        let Some(view) = self.view.upgrade() else {
            return;
        };
        self.requested = requested;
        self.outline_type = super::resolve_kind(requested, &view, cx);
        self.generation += 1;
        let generation = self.generation;
        self.analyzed_path = self.document.read(cx).path().map(Path::to_path_buf);
        let version = self.document.read(cx).version();
        self.analyzing = true;

        // `Command_FUNCLIST` analyzes nothing for the file tree; the pane makes the tree
        if self.outline_type == OutlineKind::FileTree {
            self.analyze_file_tree(generation, cx);
            return;
        }
        self.file_tree = None;

        let kind = self.outline_type;
        let options = cx
            .try_global::<OutlineOptions>()
            .copied()
            .unwrap_or_default();
        let heading_marks = cx
            .try_global::<super::HeadingMarks>()
            .map(|marks| marks.0.clone());
        let (doc, file_path, bookmarks, new_line_len) = {
            let shared = self.document.read(cx);
            let bookmarks: Vec<usize> = if kind == OutlineKind::Bookmark {
                shared.model().bookmarks().iter().collect()
            } else {
                Vec::new()
            };
            (
                shared.doc().clone(),
                shared.path().map(|p| p.to_string_lossy().into_owned()),
                bookmarks,
                shared.new_line().as_str().len(),
            )
        };
        let source = DocumentSource { doc, new_line_len };
        let path_for_clip = file_path.clone().unwrap_or_default();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let analysis = cx
                .background_executor()
                .spawn(async move {
                    let mut request = Request::new(kind);
                    request.file_path = file_path.as_deref();
                    request.bookmarks = &bookmarks;
                    request.ignore_blank_bookmarks = options.ignore_blank_bookmarks;
                    if let Some(marks) = &heading_marks {
                        request.heading_marks = marks;
                    }
                    analyze(&source, &request)
                })
                .await;
            this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                // the text was edited while the analysis ran: its line numbers are old, so it
                // is made again
                if this.document.read(cx).version() != version {
                    this.analyze(this.requested, cx);
                    return;
                }
                this.finish(analysis, path_for_clip, cx);
            })
            .ok();
        })
        .detach();
    }

    /// `Redraw` and `SetData`: the result is shown.
    fn finish(&mut self, mut analysis: Analysis, file_path: String, cx: &mut Context<Self>) {
        self.analyzing = false;
        self.stale = false;
        let kind = analysis.kind;
        let line_num_is_crlf = self
            .view
            .upgrade()
            .map(|v| v.read(cx).view_settings().outline.line_num_is_crlf)
            .unwrap_or(true);
        // SetDocLineFuncList: the lines of the functions get the marks F_FUNCLIST_NEXT and _PREV
        // move between (not for the bookmarks)
        if kind != OutlineKind::Bookmark
            && let Some(view) = self.view.upgrade()
        {
            let lines: Vec<usize> = analysis
                .items
                .items()
                .iter()
                .filter(|i| i.line > 0)
                .map(|i| i.line - 1)
                .collect();
            view.update(cx, |view, cx| {
                view.run_with(cx, |c| c.set_funclist_marks(lines));
            });
        }
        let list_lines = match self.view.upgrade() {
            Some(view) if !line_num_is_crlf => {
                let view = view.read(cx);
                analysis
                    .items
                    .items()
                    .iter()
                    .map(|i| view.layout().first_row_of_line(i.line.saturating_sub(1)) + 1)
                    .collect()
            }
            _ => analysis.items.items().iter().map(|i| i.line).collect(),
        };
        let view_type = kind.view();
        let java_tree = matches!(
            kind,
            OutlineKind::CCpp | OutlineKind::Cpp | OutlineKind::Java
        );
        let mut tree = Vec::new();
        let clip_text;
        if java_tree {
            prepare_append_texts(&mut analysis.items);
            tree = build_java_tree(&analysis.items);
            clip_text = clip_text_java(&analysis.items, &file_path);
        } else if view_type == ViewType::Tree {
            tree = build_tree(&analysis.items);
            // Python writes lines the tag jump can follow (SetTree with tagjump)
            clip_text = clip_text_tree(&analysis.items, &file_path, kind == OutlineKind::Python);
        } else if kind == OutlineKind::VisualBasic {
            clip_text = clip_text_vb(&analysis.items, &file_path);
        } else {
            clip_text = clip_text_list(&analysis.items, &file_path);
        }
        sort_tree(&mut tree, self.sort_type);
        // the nodes at the top of the tree of C++ and Java are open (TreeView_Expand)
        self.expanded.clear();
        if java_tree {
            self.expanded.extend(tree.iter().map(|n| n.order));
        }
        self.shown = Some(Shown {
            title: kind.title(),
            view: view_type,
            tree,
            list_lines,
            clip_text,
            unsupported: !kind.is_supported(),
            analysis,
        });
        self.rebuild_rows();
        // the item for the caret is selected, and opened (SetItemSelection with bAllowExpand)
        self.select_for_caret(true, cx);
        cx.notify();
    }

    /// The rows from the list order or the open nodes of the tree.
    fn rebuild_rows(&mut self) {
        if self.outline_type == OutlineKind::FileTree {
            self.rows = self.file_tree_rows();
            return;
        }
        let Some(shown) = &self.shown else {
            self.rows.clear();
            return;
        };
        let items = shown.analysis.items.items();
        let mut rows = Vec::new();
        match shown.view {
            ViewType::List => {
                let kind = shown.analysis.kind;
                for index in list_order(&shown.analysis.items, self.list_sort) {
                    let item = &items[index];
                    let remark = if kind == OutlineKind::VisualBasic {
                        vb_remark(item.info)
                    } else {
                        list_remark(item.info_value()).to_owned()
                    };
                    rows.push(Row {
                        key: index,
                        item: Some(index),
                        depth: 0,
                        label: item.name.clone(),
                        has_children: false,
                        expanded: false,
                        columns: Some([
                            shown
                                .list_lines
                                .get(index)
                                .copied()
                                .unwrap_or(item.line)
                                .to_string(),
                            item.col.to_string(),
                            remark,
                        ]),
                        path: None,
                    });
                }
            }
            ViewType::Tree => {
                // depth first, an explicit stack so that a deep tree does not use the stack
                let mut stack: Vec<(&TreeNode, usize)> =
                    shown.tree.iter().rev().map(|n| (n, 0)).collect();
                while let Some((node, depth)) = stack.pop() {
                    let open = self.expanded.contains(&node.order);
                    rows.push(Row {
                        key: node.order,
                        item: node.item,
                        depth,
                        label: node.label.clone(),
                        has_children: !node.children.is_empty(),
                        expanded: open && !node.children.is_empty(),
                        columns: None,
                        path: None,
                    });
                    if open {
                        stack.extend(node.children.iter().rev().map(|n| (n, depth + 1)));
                    }
                }
            }
        }
        self.rows = rows;
    }

    /// The path of keys from the top of the tree to the node of an item.
    fn path_to_item(nodes: &[TreeNode], item: usize) -> Option<Vec<usize>> {
        for node in nodes {
            if node.item == Some(item) {
                return Some(vec![node.order]);
            }
            if let Some(mut rest) = Self::path_to_item(&node.children, item) {
                rest.insert(0, node.order);
                return Some(rest);
            }
        }
        None
    }

    /// `SetItemSelection`: selects the item of the analysis for the caret. With `allow_expand` the
    /// nodes above it open; else the closed node that holds it is selected.
    fn select_for_caret(&mut self, allow_expand: bool, cx: &mut Context<Self>) {
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let caret = view.read(cx).cursor().caret();
        self.last_caret = Some(caret);
        let Some(shown) = &self.shown else {
            return;
        };
        let mut opened = false;
        let Some(index) = item_at(&shown.analysis.items, caret.line + 1, caret.col + 1) else {
            self.selected = None;
            return;
        };
        let key = match shown.view {
            ViewType::List => index,
            ViewType::Tree => {
                let Some(path) = Self::path_to_item(&shown.tree, index) else {
                    return;
                };
                if allow_expand {
                    let (_, parents) = path.split_last().expect("a path has a node");
                    let before = self.expanded.len();
                    self.expanded.extend(parents.iter().copied());
                    opened = self.expanded.len() != before;
                    *path.last().expect("a path has a node")
                } else {
                    // a node that is closed hides what is below: select the highest one
                    let mut chosen = path[path.len() - 1];
                    for &parent in &path[..path.len() - 1] {
                        if !self.expanded.contains(&parent) {
                            chosen = parent;
                            break;
                        }
                    }
                    chosen
                }
            }
        };
        self.selected = Some(key);
        // the rows change only when a node opened (a move of the caret in a large list is cheap)
        if opened {
            self.rebuild_rows();
        }
        if let Some(row) = self.selected_row() {
            self.scroll.scroll_to_item(row, ScrollStrategy::Center);
        }
    }

    /// `NotifyCaretMovement`: the selection follows the caret while the result is up to date.
    fn caret_moved(&mut self, view: &Entity<EditorView>, cx: &mut Context<Self>) {
        if self.stale || self.shown.is_none() {
            return;
        }
        let caret = view.read(cx).cursor().caret();
        if self.last_caret == Some(caret) {
            return;
        }
        self.select_for_caret(false, cx);
        cx.notify();
    }

    // ---- actions ----

    /// `OnJump`: moves the caret to the item of the row. `focus_text` gives the focus to the
    /// text afterwards (the docked pane does that for Enter and a double click).
    pub(crate) fn jump(
        &mut self,
        row: usize,
        focus_text: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.outline_type == OutlineKind::FileTree {
            self.open_file_row(row, focus_text, window, cx);
            return;
        }
        let Some(item) = self.rows.get(row).and_then(|r| r.item) else {
            return;
        };
        let Some(line) = self
            .shown
            .as_ref()
            .and_then(|s| s.analysis.items.get(item))
            .map(|i| i.line)
        else {
            return;
        };
        self.selected = self.rows.get(row).map(|r| r.key);
        let Some(view) = self.view.upgrade() else {
            return;
        };
        // MYWM_SETCARETPOS with PM_SETCARETPOS_KEEPSELECT: the caret goes to the start of the
        // line (the column of every item is 1) and a locked selection is extended
        let target = Pos::new(line.saturating_sub(1), 0);
        view.update(cx, |view, cx| {
            view.run_with(cx, |c| {
                let extend = c.extends(false);
                c.move_caret(target, extend)
            })
            .ok();
        });
        self.last_caret = Some(target);
        if focus_text {
            view.update(cx, |view, cx| window.focus(&view.focus_handle, cx));
        }
        cx.notify();
    }

    pub(crate) fn toggle_node(&mut self, row: usize, cx: &mut Context<Self>) {
        let Some(r) = self.rows.get(row) else {
            return;
        };
        if !r.has_children {
            return;
        }
        let key = r.key;
        if !self.expanded.remove(&key) {
            self.expanded.insert(key);
            // `TVN_ITEMEXPANDING`: a folder of the file tree is listed when it is opened
            self.start_load(key, cx);
        }
        self.rebuild_rows();
        cx.notify();
    }

    fn set_node(&mut self, row: usize, open: bool, cx: &mut Context<Self>) {
        let Some(r) = self.rows.get(row) else {
            return;
        };
        if r.has_children && r.expanded != open {
            self.toggle_node(row, cx);
        }
    }

    /// The copy button: the text of the result to the clipboard.
    pub(crate) fn copy(&mut self, cx: &mut Context<Self>) {
        let text = self.clip_text().to_owned();
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    /// 更新: analyzes again.
    pub(crate) fn reload(&mut self, cx: &mut Context<Self>) {
        self.analyze(self.requested, cx);
    }

    pub(crate) fn set_sort_type(&mut self, sort_type: SortType, cx: &mut Context<Self>) {
        if self.sort_type == sort_type {
            return;
        }
        self.sort_type = sort_type;
        if let Some(shown) = &mut self.shown {
            sort_tree(&mut shown.tree, sort_type);
        }
        self.rebuild_rows();
        cx.notify();
    }

    /// `LVN_COLUMNCLICK`.
    pub(crate) fn click_column(&mut self, column: ListColumn, cx: &mut Context<Self>) {
        self.list_sort = self.list_sort.click(column);
        self.rebuild_rows();
        cx.notify();
    }

    fn set_ignore_blank(&mut self, on: bool, cx: &mut Context<Self>) {
        cx.set_global(OutlineOptions {
            ignore_blank_bookmarks: on,
        });
        if self.outline_type == OutlineKind::Bookmark {
            self.reload(cx);
        }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let host = self.host.clone();
        window.defer(cx, move |window, cx| {
            host.update(cx, |workspace, cx| workspace.close_outline(window, cx))
                .ok();
        });
    }

    /// Key2Command: a key the pane does not use itself runs the function the key map binds it to,
    /// for the outline functions and the copy.
    fn function_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(chord) = chord_of(&event.keystroke) else {
            return false;
        };
        let Some(view) = self.view.upgrade() else {
            return false;
        };
        let Resolved::Code(code) = view.read(cx).keymap.resolve(chord) else {
            return false;
        };
        let name = ume_commands::sakura::FUNCTIONS
            .iter()
            .find(|f| f.code == code)
            .map(|f| f.name);
        match name {
            // Key2Command: these run the function again with the action SHOW_RELOAD, except the
            // toggle, which closes the pane (`HandleCommand` gives it SHOW_TOGGLE); F_REDRAW
            // makes the same kind again
            Some(
                name @ ("F_OUTLINE" | "F_OUTLINE_TOGGLE" | "F_BOOKMARK_VIEW" | "F_FILETREE"
                | "F_REDRAW"),
            ) => {
                let (requested, show) = match name {
                    "F_OUTLINE" => (Requested::Default, Show::Reload),
                    "F_OUTLINE_TOGGLE" => (Requested::Default, Show::Toggle),
                    "F_BOOKMARK_VIEW" => (Requested::Bookmark, Show::Reload),
                    "F_FILETREE" => (Requested::FileTree, Show::Reload),
                    _ => (self.requested, Show::Reload),
                };
                let host = self.host.clone();
                window.defer(cx, move |window, cx| {
                    host.update(cx, |workspace, cx| {
                        workspace.outline_command(requested, show, window, cx)
                    })
                    .ok();
                });
                true
            }
            Some("F_BOOKMARK_SET") => {
                if let Some(row) = self.selected_row() {
                    self.jump(row, false, window, cx);
                }
                view.update(cx, |view, cx| {
                    view.run_function(code, &[], window, cx);
                });
                true
            }
            Some("F_COPY" | "F_CUT") => {
                self.copy(cx);
                true
            }
            _ => false,
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let last = self.rows.len().saturating_sub(1);
        let current = self.selected_row();
        let tree = self
            .shown
            .as_ref()
            .is_some_and(|s| s.view == ViewType::Tree);
        let mods = event.keystroke.modifiers;
        let plain = !mods.control && !mods.alt && !mods.platform && !mods.shift;
        let move_to = |this: &mut Self, row: usize, cx: &mut Context<Self>| {
            if let Some(r) = this.rows.get(row) {
                this.selected = Some(r.key);
                this.scroll.scroll_to_item(row, ScrollStrategy::Nearest);
                cx.notify();
            }
        };
        match event.keystroke.key.as_str() {
            "up" if plain => move_to(self, current.map_or(0, |c| c.saturating_sub(1)), cx),
            "down" if plain => move_to(self, current.map_or(0, |c| (c + 1).min(last)), cx),
            "pageup" if plain => move_to(self, current.map_or(0, |c| c.saturating_sub(10)), cx),
            "pagedown" if plain => move_to(self, current.map_or(0, |c| (c + 10).min(last)), cx),
            "home" if plain => move_to(self, 0, cx),
            "end" if plain => move_to(self, last, cx),
            "right" if plain && tree => {
                if let Some(row) = current {
                    self.set_node(row, true, cx);
                }
            }
            "left" if plain && tree => {
                if let Some(row) = current {
                    self.set_node(row, false, cx);
                }
            }
            // OK: the caret goes there and the text has the focus
            "enter" if plain => {
                if let Some(row) = current {
                    self.jump(row, true, window, cx);
                }
            }
            // OnJump( false ): the caret goes there, the focus stays
            "space" if plain => {
                if let Some(row) = current {
                    self.jump(row, false, window, cx);
                }
            }
            // IDCANCEL of a docked dialog: the focus goes back to the text
            "escape" if plain => {
                if let Some(view) = self.view.upgrade() {
                    view.update(cx, |view, cx| window.focus(&view.focus_handle, cx));
                }
            }
            _ => {
                if !self.function_key(event, window, cx) {
                    return;
                }
            }
        }
        cx.stop_propagation();
    }
}

/// The file tree (`OUTLINE_FILETREE`): see [`super::file_tree`].
impl OutlinePane {
    /// `SetTreeFile`: reads the settings and builds the tree in the background, then shows it.
    fn analyze_file_tree(&mut self, generation: u64, cx: &mut Context<Self>) {
        self.shown = None;
        let type_index = self
            .view
            .upgrade()
            .map_or(0, |view| view.read(cx).view_settings().type_index);
        // Sakura's current folder is the folder of the document it opened
        let base_dir = self
            .document
            .read(cx)
            .path()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .filter(|dir| !dir.as_os_str().is_empty())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        // the file as it is now (another window may have written it), else what was read at start
        let settings_file = crate::history::settings_path(cx);
        let fallback =
            crate::doc_types::with_user(cx, |user| user.profile().cloned()).unwrap_or_default();
        let meta = crate::file_dialog::Meta::new(cx);
        cx.notify();
        cx.spawn(async move |this, cx| {
            let model = cx
                .background_executor()
                .spawn(async move {
                    let profile = settings_file
                        .and_then(|path| ume_config::Profile::load(&path).ok())
                        .unwrap_or(fallback);
                    let inputs = Inputs {
                        profile,
                        type_index,
                        base_dir,
                        meta: Box::new(meta),
                    };
                    let loaded = load_setting(&inputs);
                    FileTreeModel::build(&loaded, &inputs.base_dir, inputs.meta.as_ref())
                })
                .await;
            this.update(cx, |this, cx| {
                if this.generation == generation {
                    this.finish_file_tree(model, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    fn finish_file_tree(&mut self, model: FileTreeModel, cx: &mut Context<Self>) {
        self.analyzing = false;
        self.stale = false;
        self.expanded.clear();
        self.selected = None;
        let roots = model.pending_roots();
        self.file_tree = Some(model);
        self.rebuild_rows();
        // `SetTreeFileSub` for each Grep item, at once
        for root in roots {
            self.start_load(root, cx);
        }
        cx.notify();
    }

    /// Lists the folder of a node in the background, if it is still to be listed, and puts the
    /// entries below the node when they are there.
    fn start_load(&mut self, node: usize, cx: &mut Context<Self>) {
        let Some(job) = self
            .file_tree
            .as_mut()
            .and_then(|tree| tree.begin_load(node))
        else {
            return;
        };
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            let listing = cx
                .background_executor()
                .spawn(async move { job.run() })
                .await;
            this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                if let Some(tree) = &mut this.file_tree {
                    tree.finish_load(node, listing);
                }
                this.rebuild_rows();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// The rows of the open nodes of the tree, depth first.
    fn file_tree_rows(&self) -> Vec<Row> {
        let Some(tree) = &self.file_tree else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        let mut stack: Vec<(usize, usize)> = tree.roots.iter().rev().map(|&id| (id, 0)).collect();
        while let Some((id, depth)) = stack.pop() {
            let node = &tree.nodes[id];
            let open = self.expanded.contains(&id);
            rows.push(Row {
                key: id,
                item: None,
                depth,
                label: node.label.clone(),
                has_children: node.has_children(),
                expanded: open && !node.children.is_empty(),
                columns: None,
                path: node.file().map(Path::to_path_buf),
            });
            if open {
                stack.extend(node.children.iter().rev().map(|&child| (child, depth + 1)));
            }
        }
        rows
    }

    /// `OnJump` for the file tree: a file is opened the way the open command does it
    /// (`TagJumpTimer`): in this window if it takes a file, else in the window that has the file
    /// or in a new one. Folders and messages do nothing. With `focus_text` the text gets the
    /// focus when the file went into this window (`MYWM_SETACTIVEPANE`).
    fn open_file_row(
        &mut self,
        row: usize,
        focus_text: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((key, Some(path))) = self.rows.get(row).map(|r| (r.key, r.path.clone())) else {
            return;
        };
        self.selected = Some(key);
        let Some(view) = self.view.upgrade() else {
            return;
        };
        let here = view.read(cx).accepts_load(cx);
        view.update(cx, |view, cx| {
            crate::file_ops::start(window, cx, async move |view, cx| {
                crate::file_ops::load::open_paths(&view, cx, vec![path], None).await;
            });
            if focus_text && here {
                window.focus(&view.focus_handle, cx);
            }
        });
        cx.notify();
    }
}

/// A clone of the document, read by the analyzers.
struct DocumentSource {
    doc: ume_core::buffer::Document,
    new_line_len: usize,
}

impl ume_outline::LineSource for DocumentSource {
    fn line_count(&self) -> usize {
        ume_outline::LineSource::line_count(&self.doc)
    }

    fn line(&self, index: usize) -> Option<Vec<char>> {
        ume_outline::LineSource::line(&self.doc, index)
    }

    fn new_line_len(&self) -> usize {
        self.new_line_len
    }
}

impl Focusable for OutlinePane {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for OutlinePane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = (window.viewport_size().width / 3.0).max(px(MIN_WIDTH));
        let button = |id: &'static str, text: &'static str| {
            div()
                .id(id)
                .px_2()
                .h(px(24.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(0xaaaaaa))
                .bg(rgb(0xf4f4f4))
                .cursor_pointer()
                .child(text)
        };
        let is_list = self
            .shown
            .as_ref()
            .is_some_and(|s| s.view == ViewType::List);
        let is_bookmark = self.outline_type == OutlineKind::Bookmark;

        let title = if self.analyzing && self.shown.is_none() {
            "解析中...".to_owned()
        } else {
            self.title().to_owned()
        };
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .child(div().font_weight(gpui::FontWeight::BOLD).child(title))
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        button("outline-update", "更新")
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.reload(cx))),
                    )
                    .child(button("outline-close", "閉じる").on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.close(window, cx)),
                    )),
            );

        // 順序 (the trees only)
        let sort_row = (!is_list && self.shown.is_some()).then(|| {
            let mut row = div().flex().flex_wrap().gap_1().px_2().pb_1().child("順序");
            for sort in SortType::ALL {
                let mut b = button(
                    match sort {
                        SortType::Default => "outline-sort-0",
                        SortType::DefaultDesc => "outline-sort-1",
                        SortType::AtoZ => "outline-sort-2",
                        SortType::ZtoA => "outline-sort-3",
                    },
                    sort.label(),
                );
                if sort == self.sort_type {
                    b = b.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff));
                }
                row =
                    row.child(b.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.set_sort_type(sort, cx)
                    })));
            }
            row
        });

        // the headers of the columns of a list
        let column_row = is_list.then(|| {
            let sorted = |c: ListColumn| {
                if self.list_sort.column == c { " *" } else { "" }
            };
            let name = if is_bookmark {
                "テキスト"
            } else {
                "関数名"
            };
            let head = |id: &'static str, text: String, column: ListColumn| {
                div().id(id).px_1().cursor_pointer().child(text).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| this.click_column(column, cx)),
                )
            };
            div()
                .flex()
                .bg(rgb(0xe8e8e8))
                .child(
                    head(
                        "outline-col-row",
                        format!("行{}", sorted(ListColumn::Row)),
                        ListColumn::Row,
                    )
                    .w(px(52.)),
                )
                .child(
                    head(
                        "outline-col-col",
                        format!("桁{}", sorted(ListColumn::Col)),
                        ListColumn::Col,
                    )
                    .w(px(40.)),
                )
                .child(
                    head(
                        "outline-col-name",
                        format!("{name}{}", sorted(ListColumn::Name)),
                        ListColumn::Name,
                    )
                    .flex_1(),
                )
                .child(
                    head(
                        "outline-col-remark",
                        sorted(ListColumn::Remark).trim().to_owned(),
                        ListColumn::Remark,
                    )
                    .w(px(120.)),
                )
        });

        let count = self.rows.len();
        let selected = self.selected;
        let stale = self.stale;
        let body = uniform_list(
            "outline-rows",
            count,
            cx.processor(
                move |this: &mut Self, range: std::ops::Range<usize>, _window, cx| {
                    range
                        .filter_map(|i| {
                            let row = this.rows.get(i)?.clone();
                            Some(this.render_row(i, &row, selected == Some(row.key), stale, cx))
                        })
                        .collect::<Vec<_>>()
                },
            ),
        )
        .track_scroll(&self.scroll)
        .flex_1();

        let footer = div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .child(
                button("outline-copy", "コピー(C)")
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.copy(cx))),
            )
            .when_bookmark(is_bookmark, |this| {
                let on = cx
                    .try_global::<OutlineOptions>()
                    .is_some_and(|o| o.ignore_blank_bookmarks);
                this.child(
                    div()
                        .id("outline-ignore-blank")
                        .cursor_pointer()
                        .child(format!("{} 空行を無視する(B)", if on { "☑" } else { "☐" }))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.set_ignore_blank(!on, cx)
                        })),
                )
            });

        let message = self.shown.as_ref().and_then(|s| {
            s.unsupported
                .then_some("このタイプのアウトライン解析にはまだ対応していません")
        });

        div()
            .id("outline-pane")
            .key_context("Outline")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .w(width)
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(rgb(0xf6f6f6))
            .text_color(rgb(TEXT))
            .text_size(px(13.))
            .border_l_1()
            .border_color(rgb(0xb0b0b0))
            .child(header)
            .children(sort_row)
            .children(column_row)
            .children(message.map(|m| div().px_2().text_color(rgb(STALE_TEXT)).child(m)))
            .child(body)
            .child(footer)
    }
}

/// `when` for a plain flag, without GPUI's `FluentBuilder` trait in scope.
trait WhenBookmark: Sized {
    fn when_bookmark(self, on: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if on { f(self) } else { self }
    }
}

impl WhenBookmark for gpui::Div {}

impl OutlinePane {
    fn render_row(
        &self,
        index: usize,
        row: &Row,
        selected: bool,
        stale: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let mut line = div()
            .id(("outline-row", index))
            .h(px(20.))
            .flex()
            .items_center()
            .cursor_pointer();
        if selected {
            line = line.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff));
        } else if stale {
            line = line.text_color(rgb(STALE_TEXT));
        }
        let line = match &row.columns {
            Some([line_no, col, remark]) => line
                .child(div().w(px(52.)).px_1().child(line_no.clone()))
                .child(div().w(px(40.)).px_1().child(col.clone()))
                .child(
                    div()
                        .flex_1()
                        .px_1()
                        .overflow_hidden()
                        .child(row.label.clone()),
                )
                .child(
                    div()
                        .w(px(120.))
                        .px_1()
                        .overflow_hidden()
                        .child(remark.clone()),
                ),
            None => {
                let arrow = if !row.has_children {
                    " "
                } else if row.expanded {
                    "▾"
                } else {
                    "▸"
                };
                line.pl(px(4. + 14. * row.depth as f32))
                    .child(
                        div()
                            .id(("outline-arrow", index))
                            .w(px(14.))
                            .child(arrow)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.toggle_node(index, cx);
                                cx.stop_propagation();
                            })),
                    )
                    .child(div().flex_1().overflow_hidden().child(row.label.clone()))
            }
        };
        line.on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            // a click moves the caret and keeps the focus (OnJump( false, false )); a double
            // click gives the focus to the text (OnJump())
            this.selected = this.rows.get(index).map(|r| r.key);
            let double = event.click_count() >= 2;
            // a click on the file tree only selects (`OnJump( false, false )`: no file jump);
            // the double click opens the file, and does not open or close the folder
            // (`NM_DBLCLK` sets `DWLP_MSGRESULT` to TRUE)
            if this.outline_type == OutlineKind::FileTree && !double {
                this.focus(window, cx);
                cx.notify();
                return;
            }
            this.jump(index, double, window, cx);
            if !double {
                this.focus(window, cx);
            }
        }))
    }
}
