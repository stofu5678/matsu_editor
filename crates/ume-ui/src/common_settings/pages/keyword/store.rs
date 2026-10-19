//! The sets in the settings file: how the hidden list of the page reads them and OK writes them.
//!
//! OK writes `[KeyWords]` (`ShareData_IO_KeyWords`) and, because the types refer to the sets by number, the
//! numbers of the types that a deleted set moves (`CPropCommon::ApplyData` writes `m_Types_nKeyWordSetIdx`).
//! Both happen under the lock of the settings file, in one write, so that the file never has sets and types
//! that disagree.

use ume_config::settings::DocumentTypes;
use ume_config::settings::keywords::{KeywordSets, MAX_KEYWORD_SETS, store_sets};
use ume_config::{Profile, ProfileError};

use super::entry::{drafts_of, entries_of, from_rows, renumber, rows_of, stamp_of};
use crate::common_settings::table::{Column, Row, Storage, TableSpec};

/// The columns of the hidden list: where the set was, its name, the case rule, and its keywords. The list is
/// never drawn: the page draws the sets itself.
const COLUMNS: &[Column] = &[Column {
    title: "",
    edit_label: "",
    key: "",
    width: 0,
    max_chars: 0,
}; 4];

pub(super) static SETS: TableSpec = TableSpec {
    columns: COLUMNS,
    max_rows: MAX_KEYWORD_SETS,
    ops: &[],
    defaults: &[],
    storage: Storage::Custom { load, store },
    full_message: "",
    prepare: None,
};

thread_local! {
    /// Set by [`STAMP`] when the file has other sets than the dialog started from; [`SETS`] and the page's `apply` read it.
    static STALE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The one-row list that keeps what the sets of the file were when the dialog started, apart from the sets
/// the user edits (they may all be deleted). It is stored before [`SETS`] (the order of the page's controls).
pub(super) static STAMP: TableSpec = TableSpec {
    columns: &[Column {
        title: "",
        edit_label: "",
        key: "",
        width: 0,
        max_chars: 0,
    }],
    max_rows: 1,
    ops: &[],
    defaults: &[],
    storage: Storage::Custom {
        load: load_stamp,
        store: store_stamp,
    },
    full_message: "",
    prepare: None,
};

fn load_stamp(profile: &Profile) -> Option<Vec<Row>> {
    Some(vec![vec![stamp_of(&KeywordSets::from_profile(profile))]])
}

/// Did the last OK find that the file has other sets than the dialog started from (so [`SETS`] wrote nothing)?
pub(super) fn stale() -> bool {
    STALE.get()
}

/// Does not write; remembers whether the file still has the sets the dialog started from.
fn store_stamp(rows: &[Row], profile: &mut Profile) -> Result<bool, ProfileError> {
    let now = stamp_of(&KeywordSets::from_profile(profile));
    STALE.set(rows.first().is_some_and(|row| row[0] != now));
    Ok(false)
}

/// The sets of the file (the built-in ones when it has none) as the rows of the list.
fn load(profile: &Profile) -> Option<Vec<Row>> {
    Some(rows_of(&entries_of(&KeywordSets::from_profile(profile))))
}

/// Writes the sets and the numbers of the types. Nothing is written when the sets are as the file has them.
/// Returns whether the file changed.
fn store(rows: &[Row], profile: &mut Profile) -> Result<bool, ProfileError> {
    let old = entries_of(&KeywordSets::from_profile(profile));
    let entries = from_rows(rows);
    // The dialog started from other sets than the file has now (it was changed meanwhile): the numbers of the
    // dialog do not fit it, so nothing is written (the page tells the user, `apply`).
    if stale() {
        return Ok(false);
    }
    if entries == old {
        return Ok(false);
    }
    // The types as the file has them now, with the numbers of the old sets, and the numbers they must have.
    let renumber = renumber(&entries, old.len());
    let wanted: Vec<_> = DocumentTypes::from_profile(profile)
        .iter()
        .map(|t| t.keyword_set_idx.map(&renumber))
        .collect();
    let mut changed = store_sets(profile, &drafts_of(&entries))?;
    // Read the types again from the file as it is now: a type that comes out with other numbers than it must
    // have is written with the right ones. A type of the file has the old numbers; a type that the file does
    // not have (a built-in one, or one of UME Editor's own: ADR-0017) finds its set by the name, which is not
    // there when the user renamed or deleted the set.
    for (index, (now, wanted)) in DocumentTypes::from_profile(profile)
        .iter()
        .zip(wanted)
        .enumerate()
    {
        if now.keyword_set_idx != wanted {
            let mut moved = now.clone();
            moved.keyword_set_idx = wanted;
            moved.write(profile, i32::try_from(index).unwrap_or(i32::MAX))?;
            changed = true;
        }
    }
    Ok(changed)
}
