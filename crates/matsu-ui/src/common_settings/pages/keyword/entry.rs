// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CKeyWordSetMgr.cpp, sakura_core/env/CKeyWordSetMgr.h (the rules for adding, editing and
//     deleting a set and a keyword, the room for keywords, and the cleaning of duplicates)
//   sakura_core/prop/CPropComKeyword.cpp (what the buttons of the page do)
//   sakura_core/typeprop/CImpExpManager.cpp (CImpExpKeyWord::Import)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2001, jepro, MIK
//   Copyright (C) 2001, genta, MIK
//   Copyright (C) 2002, YAZAKI, MIK, genta
//   Copyright (C) 2003, KEITA
//   Copyright (C) 2004, Moca, genta
//   Copyright (C) 2005, genta, Moca
//   Copyright (C) 2006, ryoji
//   Copyright (C) 2007, ryoji
//   Copyright (C) 2009, ryoji
//   Copyright (C) 2010, Uchi, Moca
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

//! The keyword sets as the page edits them, and the rules of `CKeyWordSetMgr`.
//!
//! The dialog keeps the sets as the rows of a hidden list ([`Entry::to_row`]); an action of the page turns the
//! rows into [`Entry`]s, changes them here and puts the rows back. Nothing here knows the window.
//!
//! The limits are Sakura's: 100 sets (`MAX_SETNUM`), a name of 32 UTF-16 units (`MAX_SETNAMELEN`), a keyword of
//! 63 (`MAX_KEYWORDLEN`), and 15000 keywords for all the sets together (`MAX_KEYWORDNUM`). The keywords of a
//! set take their room in blocks of 50 (`nKeyWordSetBlockSize`): a set that has 51 keywords has used up 100.

use matsu_config::settings::KeywordSets;
use matsu_config::settings::keyword_file::KeywordFile;
use matsu_config::settings::keywords::{
    MAX_KEYWORD_LEN, MAX_KEYWORD_SETS, MAX_SET_NAME_LEN, SetDraft, compare_words,
};

use crate::common_settings::table::Row;

/// `MAX_KEYWORDNUM`: how many keywords all the sets can hold together.
pub(super) const MAX_WORDS: usize = 15000;
/// `nKeyWordSetBlockSize`: the keywords of a set take their room in blocks of this many.
const BLOCK: usize = 50;
/// What the first column of a set that was added in the dialog says: it is in no file yet.
const NEW: &str = "+";

/// One keyword set of the dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Entry {
    /// The number the set had when the dialog opened, which is how the types that use it are found and
    /// changed when a set before it is deleted. `None` for a set that was added.
    pub origin: Option<usize>,
    pub name: String,
    pub case_sensitive: bool,
    /// The keywords in the order of the list (`m_szKeyWordArr`).
    pub words: Vec<String>,
}

/// Why a button did nothing that the user should be told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Refused {
    /// There are 100 sets (`STR_PROPCOMKEYWORD_SETMAX`).
    TooManySets,
    /// There is no room for another keyword (`STR_PROPCOMKEYWORD_KEYMAX`).
    NoRoom,
}

/// What 編集 did to a keyword.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Edited {
    Replaced,
    /// The box was empty: the keyword was deleted.
    Removed,
    /// The keyword was a duplicate of one in the set (or the position is not one), so nothing changed.
    Unchanged,
}

impl Entry {
    /// The row that stands for the set in the dialog's values: the origin, the name, the case rule and the
    /// keywords, which have no tab in them.
    pub(super) fn to_row(&self) -> Row {
        vec![
            self.origin
                .map_or_else(|| NEW.to_owned(), |o| o.to_string()),
            self.name.clone(),
            if self.case_sensitive { "1" } else { "0" }.to_owned(),
            self.words.join("\t"),
        ]
    }

    pub(super) fn from_row(row: &[String]) -> Self {
        let text = |i: usize| row.get(i).cloned().unwrap_or_default();
        let words = text(3);
        Self {
            origin: text(0).parse().ok(),
            name: text(1),
            case_sensitive: text(2) == "1",
            words: if words.is_empty() {
                Vec::new()
            } else {
                words.split('\t').map(str::to_owned).collect()
            },
        }
    }

    fn draft(&self) -> SetDraft {
        SetDraft {
            name: self.name.clone(),
            case_sensitive: self.case_sensitive,
            words: self.words.clone(),
        }
    }
}

/// The sets of the file as the dialog starts with them.
pub(super) fn entries_of(sets: &KeywordSets) -> Vec<Entry> {
    sets.drafts()
        .into_iter()
        .enumerate()
        .map(|(i, d)| Entry {
            origin: Some(i),
            name: d.name,
            case_sensitive: d.case_sensitive,
            words: d.words,
        })
        .collect()
}

/// A number that tells the sets of the file as they are (names, case rules and keywords, in order): the
/// dialog keeps the one of the file it started from.
pub(super) fn stamp_of(sets: &KeywordSets) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for draft in sets.drafts() {
        (draft.name, draft.case_sensitive, draft.words).hash(&mut hasher);
    }
    sets.len().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub(super) fn rows_of(entries: &[Entry]) -> Vec<Row> {
    entries.iter().map(Entry::to_row).collect()
}

pub(super) fn from_rows(rows: &[Row]) -> Vec<Entry> {
    rows.iter().map(|r| Entry::from_row(r)).collect()
}

/// The sets as `store_sets` writes them.
pub(super) fn drafts_of(entries: &[Entry]) -> Vec<SetDraft> {
    entries.iter().map(Entry::draft).collect()
}

/// The room a set of `count` keywords has taken (`GetAlignmentSize`).
fn alloc(count: usize) -> usize {
    count.div_ceil(BLOCK) * BLOCK
}

/// How many keywords the sets can still take beyond what they have room for (`GetFreeSize`).
fn free(entries: &[Entry]) -> usize {
    MAX_WORDS.saturating_sub(entries.iter().map(|e| alloc(e.words.len())).sum())
}

/// Can another keyword go into set `set` (`CanAddKeyWord`)? There is room left in the blocks it has, or a
/// whole block is free.
pub(super) fn can_add_word(entries: &[Entry], set: usize) -> bool {
    let Some(entry) = entries.get(set) else {
        return false;
    };
    let count = entry.words.len();
    count < alloc(count) || free(entries) >= BLOCK
}

/// The free room shown after 空き (`DispKeywordCount`): what is left of the set's blocks, and what is free
/// for all.
pub(super) fn space(entries: &[Entry], set: usize) -> usize {
    let count = entries.get(set).map_or(0, |e| e.words.len());
    alloc(count) - count + free(entries)
}

/// Cuts `text` to at most `max` UTF-16 units, at a character boundary (`wcsncpy` into a buffer of that
/// size, or the limit of the edit box).
fn clip(text: &str, max: usize) -> String {
    let mut units = 0;
    let mut out = String::new();
    for c in text.chars() {
        units += c.len_utf16();
        if units > max {
            break;
        }
        out.push(c);
    }
    out
}

/// A keyword as it is stored: cut to 63 units. A text with a tab or a line break cannot be one (the file
/// keeps the keywords of a set apart with tabs, and a line is one value), so it is nothing.
fn word_of(text: &str) -> Option<String> {
    if text.contains(['\t', '\r', '\n']) {
        return None;
    }
    let word = clip(text, MAX_KEYWORD_LEN);
    (!word.is_empty()).then_some(word)
}

/// A set name as it is stored: cut to 32 units, without line breaks.
fn name_of(text: &str) -> Option<String> {
    let name = clip(&text.replace(['\r', '\n'], ""), MAX_SET_NAME_LEN);
    (!name.is_empty()).then_some(name)
}

/// セット追加 (`IDC_BUTTON_ADDSET`): a new set with the case ignored, after the others. Returns its number;
/// `None` when the name is empty (nothing is added).
pub(super) fn add_set(entries: &mut Vec<Entry>, name: &str) -> Result<Option<usize>, Refused> {
    if entries.len() >= MAX_KEYWORD_SETS {
        return Err(Refused::TooManySets);
    }
    let Some(name) = name_of(name) else {
        return Ok(None);
    };
    // `AddKeyWordSet` takes the room of one block, and does not add the set when there is none.
    if free(entries) < BLOCK {
        return Err(Refused::NoRoom);
    }
    entries.push(Entry {
        origin: None,
        name,
        case_sensitive: false,
        words: Vec::new(),
    });
    Ok(Some(entries.len() - 1))
}

/// 変更 (`IDC_BUTTON_KEYSETRENAME`): renames set `at`. An empty name changes nothing.
pub(super) fn rename_set(entries: &mut [Entry], at: usize, name: &str) -> bool {
    match (name_of(name), entries.get_mut(at)) {
        (Some(name), Some(entry)) => {
            entry.name = name;
            true
        }
        _ => false,
    }
}

/// `DelKeyWordSet`: removes set `at`.
pub(super) fn remove_set(entries: &mut Vec<Entry>, at: usize) -> bool {
    if at >= entries.len() {
        return false;
    }
    entries.remove(at);
    true
}

/// `AddKeyWord`: adds a keyword at the end of set `set`. An empty keyword and a duplicate (`wcscmp`, whatever
/// the case rule says) are not added: `Ok(false)`. A keyword that is too long is cut. The page asks
/// [`can_add_word`] before it asks for the keyword, so the refusal is the first answer.
pub(super) fn add_word(entries: &mut [Entry], set: usize, text: &str) -> Result<bool, Refused> {
    if !can_add_word(entries, set) {
        return Err(Refused::NoRoom);
    }
    Ok(push_word(entries, set, text))
}

fn push_word(entries: &mut [Entry], set: usize, text: &str) -> bool {
    let (Some(word), Some(entry)) = (word_of(text), entries.get_mut(set)) else {
        return false;
    };
    if entry.words.contains(&word) {
        return false;
    }
    entry.words.push(word);
    true
}

/// 編集 (`Edit_List_KeyWord`, `UpdateKeyWord`, `DelKeyWord`): the keyword `at` of set `set` becomes `text`; an
/// empty text deletes it. A duplicate (the keyword itself too) changes nothing.
pub(super) fn edit_word(entries: &mut [Entry], set: usize, at: usize, text: &str) -> Edited {
    let Some(entry) = entries.get_mut(set).filter(|e| at < e.words.len()) else {
        return Edited::Unchanged;
    };
    if text.is_empty() {
        entry.words.remove(at);
        return Edited::Removed;
    }
    match word_of(text) {
        Some(word) if !entry.words.contains(&word) => {
            entry.words[at] = word;
            Edited::Replaced
        }
        _ => Edited::Unchanged,
    }
}

/// 削除 (`DelKeyWord`): removes keyword `at` of set `set`.
pub(super) fn remove_word(entries: &mut [Entry], set: usize, at: usize) -> bool {
    match entries.get_mut(set).filter(|e| at < e.words.len()) {
        Some(entry) => {
            entry.words.remove(at);
            true
        }
        None => false,
    }
}

/// 整理 (`CleanKeyWords`): sorts the keywords of set `set` the way the set is searched, and removes each one
/// that is the same as the one before it under the case rule. Returns how many were removed.
pub(super) fn clean(entries: &mut [Entry], set: usize) -> usize {
    let Some(entry) = entries.get_mut(set) else {
        return 0;
    };
    let case = entry.case_sensitive;
    let before = entry.words.len();
    entry.words.sort_by(|a, b| compare_words(a, b, case));
    entry
        .words
        .dedup_by(|b, a| compare_words(a, b, case).is_eq());
    before - entry.words.len()
}

/// インポート (`CImpExpKeyWord::Import`, with the file read): adds the keywords of the file to set `set` as
/// 追加 does, and sets the case rule to the file's (`false` when the file says nothing). When the room runs out
/// the rest is left out and the answer is `true` (`STR_IMPEXP_KEYWORD`).
pub(super) fn import(entries: &mut [Entry], set: usize, file: &KeywordFile) -> bool {
    let mut out_of_room = false;
    for line in &file.words {
        if !can_add_word(entries, set) {
            out_of_room = true;
            break;
        }
        push_word(entries, set, line);
    }
    if let Some(entry) = entries.get_mut(set) {
        entry.case_sensitive = file.case_sensitive.unwrap_or(false);
    }
    out_of_room
}

/// The numbers the types hold, in the numbers the sets have after the dialog's changes. `old_len` is how many
/// sets the file had. A set that is gone becomes -1 (`nIndex1 == index[j]`), one that was after a deleted set
/// moves up by one (`nIndex1 < index[j]`), and a number beyond the sets that were there moves up by the number
/// of the sets deleted, as Sakura's does one deletion at a time.
pub(super) fn renumber(entries: &[Entry], old_len: usize) -> impl Fn(i32) -> i32 + use<> {
    let mut new_of: Vec<Option<usize>> = vec![None; old_len];
    for (n, entry) in entries.iter().enumerate() {
        if let Some(slot) = entry.origin.and_then(|o| new_of.get_mut(o)) {
            *slot = Some(n);
        }
    }
    let deleted = new_of.iter().filter(|n| n.is_none()).count();
    move |index| match usize::try_from(index) {
        Err(_) => index,
        Ok(i) if i < new_of.len() => new_of[i].map_or(-1, |n| i32::try_from(n).unwrap_or(-1)),
        Ok(_) => index - i32::try_from(deleted).unwrap_or(0),
    }
}
