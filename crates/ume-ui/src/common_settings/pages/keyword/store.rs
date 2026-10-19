//! The sets in the settings file: how the hidden list of the page reads them and OK writes them.
//!
//! OK writes `[KeyWords]` (`ShareData_IO_KeyWords`) and, because the types refer to the sets by number, the
//! numbers of the types that a deleted set moves (`CPropCommon::ApplyData` writes `m_Types_nKeyWordSetIdx`).
//! Both happen under the lock of the settings file, in one write, so that the file never has sets and types
//! that disagree.

use ume_config::settings::DocumentTypes;
use ume_config::settings::builtin_types::BUILTIN_TYPE_COUNT;
use ume_config::settings::doc_types::is_unused_slot;
use ume_config::settings::keywords::{KeywordSets, MAX_KEYWORD_SETS, store_sets};
use ume_config::{Profile, ProfileError};

use super::entry::{drafts_of, entries_of, from_rows, renumber, rows_of};
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

/// The sets of the file (the built-in ones when it has none) as the rows of the list.
fn load(profile: &Profile) -> Option<Vec<Row>> {
    Some(rows_of(&entries_of(&KeywordSets::from_profile(profile))))
}

/// Writes the sets and the numbers of the types. Nothing is written when the sets are as the file has them.
/// Returns whether the file changed.
fn store(rows: &[Row], profile: &mut Profile) -> Result<bool, ProfileError> {
    let old = entries_of(&KeywordSets::from_profile(profile));
    let entries = from_rows(rows);
    if entries == old {
        return Ok(false);
    }
    let mut changed = false;
    // The types as the file has them now: they are read before the sets are written, because their numbers
    // are the numbers of the old sets.
    let renumber = renumber(&entries, old.len());
    let types = DocumentTypes::from_profile(profile);
    for (index, original) in types.iter().enumerate() {
        // A type of UME Editor's own that the file does not have yet is made again, with its sets found by
        // name, every time the file is read (ADR-0017).
        let number = i32::try_from(index).unwrap_or(i32::MAX);
        if index >= BUILTIN_TYPE_COUNT && is_unused_slot(profile, number) {
            continue;
        }
        let mut moved = original.clone();
        moved.keyword_set_idx = original.keyword_set_idx.map(&renumber);
        if moved.keyword_set_idx != original.keyword_set_idx {
            moved.write(profile, number)?;
            changed = true;
        }
    }
    changed |= store_sets(profile, &drafts_of(&entries))?;
    Ok(changed)
}
