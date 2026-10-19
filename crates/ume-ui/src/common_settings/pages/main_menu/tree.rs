//! The tree of the page and what the buttons do to it: the handlers of `CPropMainMenu::DispatchEvent` and
//! `Check_MainMenu` (`sakura_core/prop/CPropComMainMenu.cpp`).
//!
//! The tree is the flat list of rows of the menu in depth-first order, with `level` as the depth. A row
//! belongs to the closest row before it that is one level up.

use ume_config::settings::main_menu::{
    F_NODE, F_OPTION, F_SEPARATOR, F_SPECIAL_FIRST, F_SPECIAL_LAST, MAX_MAIN_MENU_NAME_LEN,
    MAX_MAINMENU, MAX_MAINMENU_TOP,
};
use ume_config::settings::{MenuRow, RowKind};

/// `STR_PROPCOMMAINMENU_EDIT`: the name of a new node.
pub(super) const NEW_NODE_NAME: &str = "編集してください";

/// A row of the tree with what only the page knows (`SMainMenuWork`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PageRow {
    pub row: MenuRow,
    /// `m_bDupErr`: the access key is the same as a sibling's (marked by 検査).
    pub duplicate: bool,
    /// The node is shown without its children.
    pub collapsed: bool,
}

/// Where a button puts a new row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Place {
    /// 「＋」 (`IDC_BUTTON_INSERT_NODE`).
    Node,
    /// 「---」 (`IDC_BUTTON_INSERTSEPARATOR`).
    Separator,
    /// 「上⇒」 (`IDC_BUTTON_INSERT`).
    Above,
    /// 「下⇒」 (`IDC_BUTTON_INSERT_A`).
    Below,
    /// 「>>」 (`IDC_BUTTON_ADD`).
    Add,
}

/// The rows with the selected row and the selected function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Tree {
    pub rows: Vec<PageRow>,
    /// The selected row (`None`: nothing is selected).
    pub sel: Option<usize>,
    /// The selected row of the function list.
    pub func_sel: Option<usize>,
    /// How many functions the function list has.
    pub func_count: usize,
}

/// `F_SPECIAL_FIRST..=F_SPECIAL_LAST`: a list that is filled in at run time.
pub(super) fn is_special(function: u16) -> bool {
    (F_SPECIAL_FIRST..=F_SPECIAL_LAST).contains(&function)
}

fn page_row(row: MenuRow) -> PageRow {
    PageRow {
        row,
        duplicate: false,
        collapsed: false,
    }
}

impl Tree {
    /// The index after the last row under row `i`.
    pub(super) fn end_of(&self, i: usize) -> usize {
        let level = self.rows[i].row.level;
        (i + 1..self.rows.len())
            .find(|&j| self.rows[j].row.level <= level)
            .unwrap_or(self.rows.len())
    }

    pub(super) fn parent(&self, i: usize) -> Option<usize> {
        let level = self.rows[i].row.level;
        if level == 0 {
            return None;
        }
        (0..i).rev().find(|&j| self.rows[j].row.level < level)
    }

    pub(super) fn prev_sibling(&self, i: usize) -> Option<usize> {
        let level = self.rows[i].row.level;
        (0..i)
            .rev()
            .map(|j| (j, self.rows[j].row.level))
            .take_while(|&(_, l)| l >= level)
            .find(|&(_, l)| l == level)
            .map(|(j, _)| j)
    }

    pub(super) fn next_sibling(&self, i: usize) -> Option<usize> {
        let end = self.end_of(i);
        (end < self.rows.len() && self.rows[end].row.level == self.rows[i].row.level).then_some(end)
    }

    pub(super) fn is_node(&self, i: usize) -> bool {
        self.rows[i].row.kind == RowKind::Node
    }

    pub(super) fn has_children(&self, i: usize) -> bool {
        self.end_of(i) > i + 1
    }

    /// Rows that show: those with no collapsed node above them.
    pub(super) fn visible(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut hidden_below: Option<usize> = None;
        for (i, row) in self.rows.iter().enumerate() {
            if let Some(level) = hidden_below {
                if row.row.level > level {
                    continue;
                }
                hidden_below = None;
            }
            out.push(i);
            if row.collapsed && row.row.kind == RowKind::Node {
                hidden_below = Some(row.row.level);
            }
        }
        out
    }

    /// Opens the nodes above row `i` (`TreeView_SelectItem` shows the item).
    fn reveal(&mut self, i: usize) {
        let mut at = i;
        while let Some(parent) = self.parent(at) {
            self.rows[parent].collapsed = false;
            at = parent;
        }
    }

    /// 「全開」 and 「全閉」.
    pub(super) fn expand_all(&mut self, open: bool) {
        for row in &mut self.rows {
            if row.row.kind == RowKind::Node {
                row.collapsed = !open;
            }
        }
    }

    /// The click on the button of a node.
    pub(super) fn toggle(&mut self, i: usize) {
        if let Some(row) = self.rows.get_mut(i).filter(|r| r.row.kind == RowKind::Node) {
            row.collapsed = !row.collapsed;
        }
    }

    /// Selects row `i` and shows it.
    pub(super) fn select(&mut self, i: usize) {
        if i < self.rows.len() {
            self.sel = Some(i);
            self.reveal(i);
        }
    }

    /// Where the new row goes for `place`: `(index, level, parent)`. The rules of the handlers:
    ///
    /// * Nothing selected: the end of the top level.
    /// * 「>>」: the end of the selected node; a row that is not a node cannot have children, so the end of
    ///   its parent (the top level).
    /// * 「＋」 and 「下⇒」: the start of the selected node; after a row that is not a node.
    /// * 「---」 and 「上⇒」: before the selected row, in its parent; a row of the top level has no parent, and
    ///   the new row goes to the very top (`TVI_FIRST`, whichever row is selected).
    fn place(&self, place: Place) -> (usize, usize, Option<usize>) {
        let Some(sel) = self.sel.filter(|&s| s < self.rows.len()) else {
            return (self.rows.len(), 0, None);
        };
        let level = self.rows[sel].row.level;
        let parent = self.parent(sel);
        match place {
            Place::Add if self.is_node(sel) => (self.end_of(sel), level + 1, Some(sel)),
            Place::Add => match parent {
                Some(p) => (self.end_of(p), level, Some(p)),
                None => (self.rows.len(), 0, None),
            },
            Place::Node | Place::Below if self.is_node(sel) => (sel + 1, level + 1, Some(sel)),
            Place::Node | Place::Below => (self.end_of(sel), level, parent),
            Place::Separator | Place::Above => match parent {
                Some(p) => (sel, level, Some(p)),
                None => (0, 0, None),
            },
        }
    }

    fn put(&mut self, place: Place, row: MenuRow) {
        let (at, level, parent) = self.place(place);
        self.rows.insert(at, page_row(MenuRow { level, ..row }));
        if let Some(parent) = parent {
            self.rows[parent].collapsed = false;
        }
        self.sel = Some(at);
    }

    /// `List_SetCurSel(hwndLIST_FUNC, nIdxFunc + 1)`: the function list moves on; past its end nothing is
    /// selected.
    fn next_function(&self, func: usize) -> Option<usize> {
        Some(func + 1).filter(|&f| f < self.func_count)
    }

    /// 「＋」: a new node with the name 「編集してください」, which becomes the selected row.
    pub(super) fn insert_node(&mut self) {
        self.put(
            Place::Node,
            MenuRow {
                level: 0,
                kind: RowKind::Node,
                function: F_NODE,
                key: None,
                name: NEW_NODE_NAME.to_owned(),
            },
        );
    }

    /// 「---」.
    pub(super) fn insert_separator(&mut self) {
        self.put(Place::Separator, MenuRow::separator(0));
    }

    /// 「上⇒」, 「下⇒」 and 「>>」: the selected function, with no access key. It becomes the selected row and
    /// the function list moves on.
    pub(super) fn insert_function(&mut self, place: Place, code: u16) {
        let Some(func) = self.func_sel else { return };
        let mut row = MenuRow::leaf(0, code);
        if is_special(code) {
            row.kind = RowKind::Special;
        }
        self.put(place, row);
        self.func_sel = self.next_function(func);
    }

    /// 「削除」: removes the selected row and the rows under it. The next row of its level is selected, else the
    /// one before it, else its parent.
    pub(super) fn delete(&mut self) {
        let Some(sel) = self.sel.filter(|&s| s < self.rows.len()) else {
            return;
        };
        let end = self.end_of(sel);
        let next = self.next_sibling(sel).map(|_| sel);
        let target = next
            .or_else(|| self.prev_sibling(sel))
            .or_else(|| self.parent(sel));
        self.rows.drain(sel..end);
        self.sel = target.filter(|&t| t < self.rows.len());
    }

    /// 「↑」: the selected row, with the rows under it, changes places with the row of its level before it.
    pub(super) fn up(&mut self) {
        let Some(sel) = self.sel.filter(|&s| s < self.rows.len()) else {
            return;
        };
        let Some(prev) = self.prev_sibling(sel) else {
            return;
        };
        let end = self.end_of(sel);
        self.rows[prev..end].rotate_left(sel - prev);
        self.sel = Some(prev);
    }

    /// 「↓」.
    pub(super) fn down(&mut self) {
        let Some(sel) = self.sel.filter(|&s| s < self.rows.len()) else {
            return;
        };
        let Some(next) = self.next_sibling(sel) else {
            return;
        };
        let next_end = self.end_of(next);
        let size = next_end - next;
        self.rows[sel..next_end].rotate_left(next - sel);
        self.sel = Some(sel + size);
    }

    /// 「→」: the selected row, with the rows under it, becomes the last row of the node before it. Nothing
    /// happens when the row before it is not a node.
    pub(super) fn right(&mut self) {
        let Some(sel) = self.sel.filter(|&s| s < self.rows.len()) else {
            return;
        };
        let Some(prev) = self.prev_sibling(sel).filter(|&p| self.is_node(p)) else {
            return;
        };
        let end = self.end_of(sel);
        for row in &mut self.rows[sel..end] {
            row.row.level += 1;
        }
        self.rows[prev].collapsed = false;
    }

    /// 「←」: the selected row, with the rows under it, becomes the row after its parent. The rows after it
    /// stay in the parent.
    pub(super) fn left(&mut self) {
        let Some(sel) = self.sel.filter(|&s| s < self.rows.len()) else {
            return;
        };
        let Some(parent) = self.parent(sel) else {
            return;
        };
        let end = self.end_of(sel);
        let block: Vec<PageRow> = self.rows.drain(sel..end).collect();
        let at = self.end_of(parent);
        for (offset, mut row) in block.into_iter().enumerate() {
            row.row.level -= 1;
            self.rows.insert(at + offset, row);
        }
        self.sel = Some(at);
    }

    /// The access key of the selected row (`WM_KEYDOWN` and `NM_DBLCLK` of the tree): not for a separator.
    /// `None` clears it. The mark of a duplicate key goes.
    pub(super) fn set_key(&mut self, key: Option<char>) {
        if let Some(row) = self
            .sel
            .and_then(|i| self.rows.get_mut(i))
            .filter(|r| r.row.function != F_SEPARATOR)
        {
            row.row.key = key;
            row.duplicate = false;
        }
    }

    /// `TVN_ENDLABELEDIT`: the name of the selected node. An empty name is 「編集してください」; the name is cut
    /// to [`MAX_MAIN_MENU_NAME_LEN`]. A name that is not the one shown (`shown`) makes the node's name its
    /// own: its function becomes `F_NODE`.
    pub(super) fn set_name(&mut self, name: &str, shown: &str) {
        let Some(row) = self
            .sel
            .and_then(|i| self.rows.get_mut(i))
            .filter(|r| r.row.kind == RowKind::Node)
        else {
            return;
        };
        let name: String = if name.is_empty() {
            NEW_NODE_NAME.to_owned()
        } else {
            let mut units = 0;
            name.chars()
                .take_while(|c| {
                    units += c.len_utf16();
                    units <= MAX_MAIN_MENU_NAME_LEN
                })
                .collect()
        };
        if name != shown {
            row.row.function = F_NODE;
        }
        if row.row.function == F_NODE {
            row.row.name = name;
        }
    }

    /// `Check_MainMenu`: looks for the problems of the menu. Marks the rows whose access key another row of
    /// the same menu has, and the other way round clears the mark of a row whose key is its own. Returns the
    /// message (empty when the menu is fine) and the row that shows the first problem.
    pub(super) fn check(&mut self) -> (String, Option<usize>) {
        let mut found = Found::default();
        self.check_level(0, self.rows.len(), &mut found);
        let mut message = String::new();
        if !found.option {
            message.push_str("『共通設定』が有りません。\n");
        }
        if found.tops > MAX_MAINMENU_TOP {
            message.push_str("トップレベルの項目数が多すぎます。\n");
        }
        if found.rows > MAX_MAINMENU {
            message.push_str("登録項目数が多すぎます。\n");
        }
        if found.duplicates > 0 {
            message.push_str("重複したアクセスキーがあります。\n");
        }
        if found.unset > 0 {
            message.push_str("未設定のアクセスキーがあります。\n");
        }
        (message, found.first_error)
    }

    /// `Check_MainMenu_Sub` for the rows `from..to`, which are siblings and their descendants.
    fn check_level(&mut self, from: usize, to: usize, found: &mut Found) {
        let mut keys: Vec<(char, usize)> = Vec::new();
        let mut i = from;
        while i < to {
            let end = self.end_of(i).min(to);
            found.rows += 1;
            if self.rows[i].row.level == 0 {
                found.tops += 1;
            }
            let row = &self.rows[i].row;
            let kind = match row.function {
                F_NODE => RowKind::Node,
                F_SEPARATOR => RowKind::Separator,
                f if is_special(f) => RowKind::Special,
                _ if row.kind == RowKind::Node => RowKind::Node,
                f => {
                    found.option |= f == F_OPTION;
                    RowKind::Leaf
                }
            };
            match row.key {
                None => {
                    if matches!(kind, RowKind::Node | RowKind::Leaf) {
                        if found.unset == 0 && found.first_error.is_none() {
                            found.first_error = Some(i);
                        }
                        found.unset += 1;
                    }
                }
                Some(key) => match keys
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|&(_, first)| first)
                {
                    None => {
                        keys.push((key, i));
                        self.rows[i].duplicate = false;
                    }
                    Some(first) => {
                        if found.duplicates == 0 && found.first_error.is_none() {
                            found.first_error = Some(first);
                        }
                        found.duplicates += 1;
                        self.rows[i].duplicate = true;
                        self.rows[first].duplicate = true;
                    }
                },
            }
            if self.rows[i].row.kind == RowKind::Node {
                self.check_level(i + 1, end, found);
            }
            i = end;
        }
    }
}

#[derive(Default)]
struct Found {
    option: bool,
    rows: usize,
    tops: usize,
    duplicates: usize,
    unset: usize,
    first_error: Option<usize>,
}
