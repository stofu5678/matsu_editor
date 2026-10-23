// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/recent/CRecentImp.cpp, CRecentImp.h (the list: add to the front, favorites, the view count)
//
// Original copyright notices of those files:
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2026, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
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

//! A list of recent items, most recent first (`CRecentImp`).
//!
//! The list keeps up to `capacity` items (`MAX_MRU` and `MAX_OPENFOLDER`, 36). Only the first `view`
//! of them are shown (the settings "ファイルの履歴MAX" and "フォルダーの履歴MAX"); the rest are
//! kept, because a favorite is never dropped and the list may be shown with a larger number later.
//! An item can be a favorite (お気に入り): it is not removed to make room, and it is moved into the
//! shown part when it would fall out of it.

/// What a list needs to know about its items.
pub trait Recent {
    /// The text that identifies the item (the path).
    fn text(&self) -> &str;
    fn is_favorite(&self) -> bool;
    fn set_favorite_flag(&mut self, favorite: bool);
}

/// A list of recent items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentList<T> {
    items: Vec<T>,
    capacity: usize,
    view: usize,
    /// Whether two texts that differ only in case are the same item (`_wcsicmp`, as on Windows).
    case_insensitive: bool,
}

impl<T: Recent> RecentList<T> {
    /// An empty list. `view` is limited to `capacity`.
    pub fn new(capacity: usize, view: usize, case_insensitive: bool) -> Self {
        Self {
            items: Vec::new(),
            capacity,
            view: view.min(capacity),
            case_insensitive,
        }
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// How many items are shown (`GetViewCount`).
    pub fn view_count(&self) -> usize {
        self.view
    }

    /// The items that are shown: the first `view_count`.
    pub fn visible(&self) -> &[T] {
        &self.items[..self.view.min(self.items.len())]
    }

    pub fn case_insensitive(&self) -> bool {
        self.case_insensitive
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }

    /// Puts an item that was read from a file at the end (no favorite handling, no limit but the
    /// capacity). Used when the list is read.
    pub(crate) fn push_loaded(&mut self, item: T) {
        if self.items.len() < self.capacity {
            self.items.push(item);
        }
    }

    fn same(&self, a: &str, b: &str) -> bool {
        if self.case_insensitive {
            a == b || a.to_lowercase() == b.to_lowercase()
        } else {
            a == b
        }
    }

    /// The index of the item with this text (`FindItem`).
    pub fn find(&self, text: &str) -> Option<usize> {
        self.items.iter().position(|i| self.same(i.text(), text))
    }

    pub fn is_favorite(&self, index: usize) -> bool {
        self.items.get(index).is_some_and(Recent::is_favorite)
    }

    /// Makes an item a favorite or not (`SetFavorite`). `false` if there is no such item.
    pub fn set_favorite(&mut self, index: usize, favorite: bool) -> bool {
        match self.items.get_mut(index) {
            Some(item) => {
                item.set_favorite_flag(favorite);
                true
            }
            None => false,
        }
    }

    /// The last item at or before `from` that is (or is not) a favorite: the oldest one of those
    /// (`GetOldestItem`).
    fn oldest(&self, from: isize, favorite: bool) -> Option<usize> {
        let last = self.items.len() as isize - 1;
        let mut i = from.min(last);
        while i >= 0 {
            if self.items[i as usize].is_favorite() == favorite {
                return Some(i as usize);
            }
            i -= 1;
        }
        None
    }

    /// Moves an item to another place, the ones between shift (`MoveItem`).
    pub fn move_item(&mut self, from: usize, to: usize) -> bool {
        if from >= self.items.len() || to >= self.items.len() {
            return false;
        }
        let item = self.items.remove(from);
        self.items.insert(to, item);
        true
    }

    /// Changes how many items are shown (`ChangeViewCount`). A favorite that would be outside is
    /// moved into the shown part, in place of the oldest ordinary item before it. `false` if
    /// `view` is more than the capacity.
    pub fn change_view_count(&mut self, view: usize) -> bool {
        if view > self.capacity {
            return false;
        }
        self.view = view;
        if view >= self.items.len() {
            return true;
        }
        let Some(mut i) = self.oldest(self.items.len() as isize - 1, true) else {
            return true;
        };
        while i >= view {
            if self.items[i].is_favorite() {
                let Some(j) = self.oldest(i as isize - 1, false) else {
                    break;
                };
                self.move_item(j, i);
            }
            if i == 0 {
                break;
            }
            i -= 1;
        }
        true
    }

    /// Adds an item at the front (`AppendItem`). An item that is in the list already is replaced
    /// by the new data (it stays a favorite if it was) and moved to the front. When the list is
    /// full, the oldest ordinary item goes; if all items are favorites, nothing is added and the
    /// result is `false`.
    pub fn append(&mut self, mut item: T) -> bool {
        if let Some(index) = self.find(item.text()) {
            item.set_favorite_flag(self.items[index].is_favorite());
            self.items[index] = item;
            self.move_item(index, 0);
        } else {
            if self.capacity <= self.items.len() {
                match self.oldest(self.items.len() as isize - 1, false) {
                    Some(i) => {
                        self.items.remove(i);
                    }
                    None => return false,
                }
            }
            item.set_favorite_flag(false);
            self.items.insert(0, item);
        }
        let view = self.view;
        self.change_view_count(view);
        true
    }

    /// Removes one item (`DeleteItem`).
    pub fn delete(&mut self, index: usize) -> bool {
        if index >= self.items.len() {
            return false;
        }
        self.items.remove(index);
        true
    }

    /// Removes every item that is not a favorite (`DeleteItemsNoFavorite`). `true` if any went.
    pub fn delete_non_favorites(&mut self) -> bool {
        let before = self.items.len();
        self.items.retain(Recent::is_favorite);
        self.items.len() != before
    }

    /// Removes all items, favorites too (`DeleteAllItem`).
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Removes the items for which `gone` is true. `true` if any went.
    pub fn delete_where(&mut self, mut gone: impl FnMut(&T) -> bool) -> bool {
        let before = self.items.len();
        self.items.retain(|i| !gone(i));
        self.items.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Item(String, bool);

    impl Recent for Item {
        fn text(&self) -> &str {
            &self.0
        }
        fn is_favorite(&self) -> bool {
            self.1
        }
        fn set_favorite_flag(&mut self, favorite: bool) {
            self.1 = favorite;
        }
    }

    fn item(name: &str) -> Item {
        Item(name.to_owned(), false)
    }

    fn names(list: &RecentList<Item>) -> Vec<&str> {
        list.items().iter().map(|i| i.0.as_str()).collect()
    }

    fn list(capacity: usize, view: usize) -> RecentList<Item> {
        RecentList::new(capacity, view, true)
    }

    #[test]
    fn the_newest_item_is_first_and_a_repeat_moves_to_the_front() {
        let mut l = list(5, 5);
        for n in ["a", "b", "c"] {
            assert!(l.append(item(n)));
        }
        assert_eq!(names(&l), ["c", "b", "a"]);
        l.append(item("a"));
        assert_eq!(names(&l), ["a", "c", "b"]);
    }

    #[test]
    fn the_full_list_drops_the_oldest_ordinary_item() {
        let mut l = list(3, 3);
        for n in ["a", "b", "c"] {
            l.append(item(n));
        }
        l.set_favorite(2, true); // a
        l.append(item("d"));
        // `a` is a favorite and stays; the oldest ordinary one, `b`, goes.
        assert_eq!(names(&l), ["d", "c", "a"]);
    }

    #[test]
    fn a_list_of_favorites_takes_nothing_more() {
        let mut l = list(2, 2);
        l.append(item("a"));
        l.append(item("b"));
        l.set_favorite(0, true);
        l.set_favorite(1, true);
        assert!(!l.append(item("c")));
        assert_eq!(names(&l), ["b", "a"]);
        // An item that is there is still updated.
        assert!(l.append(item("a")));
        assert_eq!(names(&l), ["a", "b"]);
        assert!(l.is_favorite(0));
    }

    #[test]
    fn the_same_text_in_another_case_is_the_same_item_when_asked() {
        let mut l = list(5, 5);
        l.append(item("C:\\Memo.txt"));
        l.append(item("c:\\memo.TXT"));
        assert_eq!(names(&l), ["c:\\memo.TXT"]);
        let mut exact: RecentList<Item> = RecentList::new(5, 5, false);
        exact.append(item("a"));
        exact.append(item("A"));
        assert_eq!(names(&exact), ["A", "a"]);
    }

    #[test]
    fn a_favorite_outside_the_view_moves_in() {
        let mut l = list(10, 10);
        for n in ["a", "b", "c", "d", "e"] {
            l.append(item(n));
        }
        // e d c b a; `a` is a favorite and the oldest.
        l.set_favorite(4, true);
        assert!(l.change_view_count(3));
        // `a` takes the place of the oldest ordinary item before it (`b`) and `b` moves out.
        assert_eq!(names(&l), ["e", "d", "a", "c", "b"]);
        assert_eq!(l.visible().len(), 3);
        // The same happens when something is added.
        l.append(item("f"));
        assert_eq!(names(&l), ["f", "e", "a", "d", "c", "b"]);
        assert!(l.visible().iter().any(|i| i.0 == "a"));
    }

    #[test]
    fn the_view_cannot_exceed_the_capacity() {
        let mut l = list(4, 99);
        assert_eq!(l.view_count(), 4);
        assert!(!l.change_view_count(5));
        assert!(l.change_view_count(0));
        assert!(l.visible().is_empty());
    }

    #[test]
    fn deleting() {
        let mut l = list(5, 5);
        for n in ["a", "b", "c", "d"] {
            l.append(item(n));
        }
        l.set_favorite(1, true); // c
        assert!(l.delete_non_favorites());
        assert_eq!(names(&l), ["c"]);
        assert!(!l.delete_non_favorites());
        assert!(!l.delete(1));
        assert!(l.delete(0));
        assert!(l.is_empty());
        l.append(item("x"));
        l.set_favorite(0, true);
        l.clear();
        assert!(l.is_empty());
    }
}
