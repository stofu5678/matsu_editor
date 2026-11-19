//! Editing a history list by hand, the way the history management dialog does (`CDlgFavorite`).
//!
//! Sakura: `CRecentImp::AppendItemText`, `EditItemText`, `DeleteItem` and `DeleteAllItem`
//! (`sakura_core/recent/CRecentImp.cpp`). The lists of the search, replace, Grep and command
//! histories all work on text, so the dialog treats them through [`EditableList`].

/// A history of text, newest first, that the dialog adds to, edits and deletes from.
pub trait EditableList {
    /// The strings, newest first.
    fn texts(&self) -> &[String];

    /// The number of UTF-16 units a string must stay under (`GetTextMaxLength`, the size of its
    /// buffer): a string of this many units or more is refused.
    fn max_units(&self) -> usize;

    /// `AppendItemText`: puts a string in front. Refused (`false`) when it is too long or when
    /// the list has it already (by the list's own comparison): the old one does not move. A full
    /// list drops its oldest string.
    fn append_text(&mut self, text: &str) -> bool;

    /// `EditItemText`: replaces the string at `index` in place (it keeps its place). Refused when
    /// the string is too long, or another string of the list is the same; a change of case of
    /// the string itself is fine.
    fn edit_text(&mut self, index: usize, text: &str) -> bool;

    /// `DeleteItem`: removes the string at `index` and closes the gap.
    fn delete_at(&mut self, index: usize) -> bool;

    /// `DeleteAllItem`.
    fn clear_all(&mut self);
}
