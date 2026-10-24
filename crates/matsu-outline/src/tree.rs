// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
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

//! What the outline pane shows of a result: the tree (nested by `depth`, or by the `::` and `\` of
//! the names), the order of the list, the remarks, the text that the copy button puts on the
//! clipboard, and the item that matches the position of the caret.
//!
//! Sakura: `CDlgFuncList::SetTree`, `SetTreeJava`, `SetListVB`, `SetData`, `SortListView`,
//! `CompareFunc_Asc`, `SortTree`, `GetFuncInfoIndex` in `sakura_core/outline/CDlgFuncList.cpp`.

use std::cmp::Ordering;

use crate::func_obj;
use crate::{FuncInfoArr, NO_CLIP_TEXT};

/// A node of the tree (`HTREEITEM` with its `lParam`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNode {
    pub label: String,
    /// The number the node got when it was made (`lParam`): the order the items came in, which
    /// the default sort follows.
    pub order: usize,
    /// The index in the [`FuncInfoArr`] when the node is an item of the analysis; `None` for the
    /// nodes that were made for a class, a namespace or "グローバル".
    pub item: Option<usize>,
    pub children: Vec<TreeNode>,
}

/// The nodes of a tree under construction. A parent comes before its children.
#[derive(Default)]
struct Arena {
    nodes: Vec<ArenaNode>,
    roots: Vec<usize>,
}

struct ArenaNode {
    label: String,
    order: usize,
    item: Option<usize>,
    children: Vec<usize>,
}

impl Arena {
    fn add(
        &mut self,
        parent: Option<usize>,
        label: String,
        order: usize,
        item: Option<usize>,
    ) -> usize {
        let id = self.nodes.len();
        self.nodes.push(ArenaNode {
            label,
            order,
            item,
            children: Vec::new(),
        });
        match parent {
            Some(p) => self.nodes[p].children.push(id),
            None => self.roots.push(id),
        }
        id
    }

    fn children_of(&self, parent: Option<usize>) -> &[usize] {
        match parent {
            Some(p) => &self.nodes[p].children,
            None => &self.roots,
        }
    }

    fn finish(self) -> Vec<TreeNode> {
        let Arena { nodes, roots } = self;
        let mut built: Vec<Option<TreeNode>> = (0..nodes.len()).map(|_| None).collect();
        // children come after their parent, so going backwards builds them first
        for (id, node) in nodes.into_iter().enumerate().rev() {
            let children = node
                .children
                .iter()
                .filter_map(|&c| built[c].take())
                .collect();
            built[id] = Some(TreeNode {
                label: node.label,
                order: node.order,
                item: node.item,
                children,
            });
        }
        roots.into_iter().filter_map(|r| built[r].take()).collect()
    }
}

/// The deepest nesting of the tree. Sakura has no limit, but a text with levels can have a line of
/// thousands of dots, and the traversals of the tree recurse; deeper items go at this depth.
pub const MAX_TREE_DEPTH: usize = 256;

/// `SetTree`: the items go under the last item of the depth above. A depth that jumps by more
/// than one (Sakura does not allow for it) goes under the deepest item there is.
pub fn build_tree(arr: &FuncInfoArr) -> Vec<TreeNode> {
    let mut arena = Arena::default();
    // `phParentStack`: the item at each depth
    let mut parents: Vec<usize> = Vec::new();
    for (i, item) in arr.items().iter().enumerate() {
        let depth = item.depth.min(parents.len()).min(MAX_TREE_DEPTH);
        parents.truncate(depth);
        let parent = parents.last().copied();
        let id = arena.add(parent, item.name.clone(), i, Some(i));
        parents.push(id);
    }
    arena.finish()
}

/// `STR_DLGFNCLST_APND_*`: what the tree of C++ and Java writes after a name.
pub fn append_text(info: u32) -> &'static str {
    match info {
        func_obj::DECLARE => "(宣言)",
        func_obj::CLASS => " クラス",
        func_obj::STRUCT => " 構造体",
        func_obj::ENUM => " 列挙体",
        func_obj::UNION => " 共用体",
        func_obj::NAMESPACE => " 名前空間",
        func_obj::INTERFACE => " インターフェース",
        func_obj::GLOBAL => "グローバル",
        _ => "",
    }
}

/// `MAX_JAVA_TREE_NEST`.
const MAX_JAVA_TREE_NEST: usize = 32;

/// Splits the name of an item at the `::` and `\` that separate the classes. Returns the classes
/// and the name that is left (the part after the last separator).
fn split_classes(name: &[char]) -> (Vec<String>, usize) {
    let mut classes = Vec::new();
    let mut m = 0usize;
    let operator: Vec<char> = "operator ".chars().collect();
    let has_scope = name.windows(2).any(|w| w == [':', ':']);
    if !has_scope || name.starts_with(&operator) {
        return (classes, 0);
    }
    let len = name.len();
    let mut template_nest = 0;
    let mut k = 0usize;
    while k < len {
        // Sakura stops at a nesting that is too deep (2009.9.21 syat)
        if classes.len() == MAX_JAVA_TREE_NEST {
            break;
        }
        let c = name[k];
        if template_nest == 0 && c == ':' {
            // C++ is joined with Java: "::" as well as "\" separates the classes (2001.01.04 genta)
            if k + 1 < len && name[k + 1] == ':' {
                classes.push(name[m..k].iter().collect());
                m = k + 2;
                k += 1;
                // Klass::operator std::string
                if name[m..].starts_with(&operator) {
                    break;
                }
            } else {
                break;
            }
        } else if c == '\\' {
            classes.push(name[m..k].iter().collect());
            m = k + 1;
        } else if c == '<' {
            // namesp::function<std::string>
            template_nest += 1;
        } else if c == '>' && template_nest > 0 {
            template_nest -= 1;
        }
        k += 1;
    }
    (classes, m)
}

/// `SetTreeJava` with the classes added: the C++ and Java trees. An item whose name has `::` or
/// `\` goes under a node for each class; the classes of the items are made when they are met,
/// and the items that are not in a class go under "グローバル" (or at the top, for a class,
/// structure, enumeration, union, namespace and interface).
///
/// With `add_class` false (`bAddClass` is FALSE, the tree of COBOL) a node made for a class has the
/// bare name, with no text such as " クラス" after it, and an existing node is the one whose label
/// is exactly the name.
pub fn build_java_tree(arr: &FuncInfoArr, add_class: bool) -> Vec<TreeNode> {
    let mut arena = Arena::default();
    // the counter of `lParam`
    let mut counter = 0usize;
    let mut global: Option<usize> = None;
    for (i, item) in arr.items().iter().enumerate() {
        let name: Vec<char> = item.name.chars().collect();
        let (classes, m) = split_classes(&name);
        let parent: Option<usize> = if !classes.is_empty() {
            // the nodes of the classes: an existing node is the one that starts with the name of
            // the class and a space ("Name クラス")
            let mut node: Option<usize> = None;
            for class in &classes {
                let found = arena.children_of(node).iter().copied().find(|&child| {
                    let label = &arena.nodes[child].label;
                    if add_class {
                        label.starts_with(class.as_str()) && label[class.len()..].starts_with(' ')
                    } else {
                        label == class
                    }
                });
                let id = match found {
                    Some(id) => id,
                    None => {
                        let mut label = class.clone();
                        if add_class {
                            label.push_str(if item.info == func_obj::NAMESPACE {
                                arr.append_text(func_obj::NAMESPACE)
                            } else {
                                arr.append_text(func_obj::CLASS)
                            });
                        }
                        let id = arena.add(node, label, counter, None);
                        counter += 1;
                        id
                    }
                };
                node = Some(id);
            }
            node
        } else if (func_obj::CLASS..=func_obj::ELEMENT_MAX).contains(&item.info) {
            None
        } else {
            // 2001.01.04 genta: the global space (C++)
            if global.is_none() {
                let id = arena.add(
                    None,
                    arr.append_text(func_obj::GLOBAL).to_owned(),
                    counter,
                    None,
                );
                counter += 1;
                global = Some(id);
            }
            global
        };
        let mut label: String = name[m..].iter().collect();
        match item.info {
            // the place of a definition needs no text; the namespace and the global space were
            // dealt with above
            func_obj::DEFINITION | func_obj::NAMESPACE | func_obj::GLOBAL => {}
            info => label.push_str(arr.append_text(info)),
        }
        arena.add(parent, label, counter, Some(i));
        counter += 1;
    }
    arena.finish()
}

/// Makes a [`FuncInfoArr`] ready for the tree of C++ and Java: sets the texts that follow the
/// names (`SetAppendText` with `overwrite` false).
pub fn prepare_append_texts(arr: &mut FuncInfoArr) {
    for info in [
        func_obj::DECLARE,
        func_obj::CLASS,
        func_obj::STRUCT,
        func_obj::ENUM,
        func_obj::UNION,
        func_obj::NAMESPACE,
        func_obj::INTERFACE,
        func_obj::GLOBAL,
    ] {
        arr.set_append_text(info, append_text(info), false);
    }
}

/// 順序: how the tree is sorted (`SORTTYPE_*`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortType {
    /// デフォルト: as they came.
    #[default]
    Default,
    /// デフォルト(降順).
    DefaultDesc,
    /// アルファベット順.
    AtoZ,
    /// アルファベット(降順).
    ZtoA,
}

impl SortType {
    /// The index in the choice (`nOutlineSortType`).
    pub fn from_index(i: i32) -> Self {
        match i {
            1 => Self::DefaultDesc,
            2 => Self::AtoZ,
            3 => Self::ZtoA,
            _ => Self::Default,
        }
    }

    pub fn index(self) -> i32 {
        match self {
            Self::Default => 0,
            Self::DefaultDesc => 1,
            Self::AtoZ => 2,
            Self::ZtoA => 3,
        }
    }

    /// The labels of the choice (`STR_DLGFNCLST_SORTTYPE*`).
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "デフォルト",
            Self::DefaultDesc => "デフォルト(降順)",
            Self::AtoZ => "アルファベット順",
            Self::ZtoA => "アルファベット(降順)",
        }
    }

    pub const ALL: [SortType; 4] = [Self::Default, Self::DefaultDesc, Self::AtoZ, Self::ZtoA];
}

/// `lstrcmpi`: compares ignoring the case.
fn compare_text(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}

/// `SortTree`: sorts the children of every node.
pub fn sort_tree(nodes: &mut [TreeNode], sort: SortType) {
    nodes.sort_by(|a, b| match sort {
        SortType::Default => a.order.cmp(&b.order),
        SortType::DefaultDesc => b.order.cmp(&a.order),
        // the same name keeps the order they came in (`Compare_by_ItemText`)
        SortType::AtoZ => compare_text(&a.label, &b.label).then(a.order.cmp(&b.order)),
        SortType::ZtoA => compare_text(&b.label, &a.label).then(b.order.cmp(&a.order)),
    });
    for node in nodes {
        sort_tree(&mut node.children, sort);
    }
}

/// The columns of the list (`EFuncListCol`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListColumn {
    /// 行 (and the column of the same line).
    #[default]
    Row,
    /// 桁.
    Col,
    /// 関数名 (テキスト in the bookmark list).
    Name,
    /// The remark.
    Remark,
}

/// The sort of the list: a column, and the direction (`m_nSortCol`, `m_bSortDesc`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListSort {
    pub column: ListColumn,
    pub descending: bool,
}

impl ListSort {
    /// What a click on the header of `column` does (`LVN_COLUMNCLICK`): the same column turns the
    /// direction, another column starts ascending... as Sakura keeps the direction when the column
    /// changes, it only turns when the same column is clicked.
    pub fn click(self, column: ListColumn) -> Self {
        Self {
            column,
            descending: if column == self.column {
                !self.descending
            } else {
                self.descending
            },
        }
    }
}

/// `STR_DLGFNCLST_REMARK*`: the remark of the list for the `info` of an item.
pub fn list_remark(info: u32) -> &'static str {
    match info {
        1 => "宣言",
        10 => "関数宣言",
        20 => "プロシージャ宣言",
        11 => "関数",
        21 => "プロシージャ",
        31 => "■パッケージ仕様部",
        41 => "■パッケージ本体部",
        50 => "PROC",
        51 => "ラベル",
        52 => "ENDP",
        _ => "",
    }
}

/// The remark of an item of Visual Basic (`SetListVB`): the type, and the access and `Static` in
/// full-width brackets.
pub fn vb_remark(info: u32) -> String {
    let mut option = String::new();
    if (info >> 8) & 0x01 != 0 {
        option.push_str("静的 ");
    }
    option.push_str(match (info >> 4) & 0x0f {
        2 => "プライベート",
        3 => "フレンド",
        // not found: public
        _ => "パブリック",
    });
    let type_name = match info & 0x0f {
        1 => "関数",
        2 => "プロシージャ",
        3 => "プロパティ 取得",
        4 => "プロパティ 設定",
        5 => "プロパティ 参照",
        6 => "定数",
        7 => "列挙型",
        8 => "ユーザ定義型",
        9 => "イベント",
        // not defined
        _ => return String::new(),
    };
    let mut type_option = type_name.to_owned();
    if (info >> 8) & 0x02 != 0 {
        type_option.push_str("宣言");
    }
    if !option.is_empty() {
        type_option.push_str(&format!("（{option}）"));
    }
    type_option
}

/// The order of the rows of the list (`ListView_SortItems` with `CompareFunc_Asc`): indexes into
/// the items.
pub fn list_order(arr: &FuncInfoArr, sort: ListSort) -> Vec<usize> {
    let items = arr.items();
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&a, &b| {
        let (x, y) = (&items[a], &items[b]);
        let ascending = match sort.column {
            ListColumn::Name => compare_text(&x.name, &y.name),
            ListColumn::Row => x.line.cmp(&y.line).then(x.col.cmp(&y.col)),
            ListColumn::Col => x.col.cmp(&y.col),
            ListColumn::Remark => x.info.cmp(&y.info),
        };
        if sort.descending {
            ascending.reverse()
        } else {
            ascending
        }
    });
    order
}

/// `GetFuncInfoIndex`: the item for the caret at `line` and `col` (1 origin): the last item that
/// is not after the caret, among the items that follow one another in the order of the text. The
/// first item when the caret is before all of them.
pub fn item_at(arr: &FuncInfoArr, line: usize, col: usize) -> Option<usize> {
    let mut found: Option<usize> = None;
    // `nFuncLineOld`, `nFuncColOld`: -1 at the start, and not set by the first item
    let mut old = (0usize, 0usize);
    for (i, item) in arr.items().iter().enumerate() {
        let pos = (item.line, item.col);
        if found.is_none() {
            found = Some(i);
        } else if old <= pos && pos <= (line, col) {
            old = pos;
            found = Some(i);
        }
    }
    found
}

/// The text of the copy button for a list (`SetData`): `file(line,col): name(remark)` lines.
pub fn clip_text_list(arr: &FuncInfoArr, file_path: &str) -> String {
    let mut text = String::new();
    for item in arr.items() {
        let remark = list_remark(item.info_value());
        text.push_str(&format!(
            "{file_path}({},{}): {}",
            item.line, item.col, item.name
        ));
        if !remark.is_empty() {
            text.push_str(&format!("({remark})"));
        }
        text.push_str("\r\n");
    }
    text
}

/// The text of the copy button for the list of Visual Basic (`SetListVB`).
pub fn clip_text_vb(arr: &FuncInfoArr, file_path: &str) -> String {
    let mut text = String::new();
    for item in arr.items() {
        let remark = vb_remark(item.info);
        text.push_str(&format!(
            "{file_path}({},{}): {}",
            item.line, item.col, item.name
        ));
        if !remark.is_empty() {
            text.push_str(&format!("({remark})"));
        }
        text.push_str("\r\n");
    }
    text
}

/// The text of the copy button for a tree (`SetTree`): the names indented by their depth. With
/// `tag_jump` each line starts with `file(line,col): ` so that the tag jump works on it (2007.02.25
/// genta; Python does it).
pub fn clip_text_tree(arr: &FuncInfoArr, file_path: &str, tag_jump: bool) -> String {
    let mut text = String::new();
    for item in arr.items() {
        if item.info & NO_CLIP_TEXT != 0 {
            // 2003.06.22 Moca: a dummy item is in the tree, not in the tag list
            continue;
        }
        if tag_jump {
            text.push_str(file_path);
            if item.line > 0 {
                text.push_str(&format!("({},{}): ", item.line, item.col));
            }
        }
        for _ in 0..item.depth {
            text.push_str("  ");
        }
        text.push(' ');
        text.push_str(&item.name);
        text.push_str("\r\n");
    }
    text
}

/// The text of the copy button for the tree of C++ and Java (`SetTreeJava`).
pub fn clip_text_java(arr: &FuncInfoArr, file_path: &str) -> String {
    let mut text = String::new();
    for item in arr.items() {
        text.push_str(&format!(
            "{file_path}({},{}): {}",
            item.line, item.col, item.name
        ));
        if item.info == func_obj::DECLARE {
            text.push_str(arr.append_text(func_obj::DECLARE));
        }
        text.push_str("\r\n");
    }
    text
}
