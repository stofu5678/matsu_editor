//! The store for documents that are not saved (ADR-0022): the files, not the editor's side.
//!
//! The editor keeps a copy of each unsaved document in the folder `unsaved/` next to `matsu.ini`, so
//! that a crash or a power loss does not lose it, and brings it back at the next start. Nothing here
//! touches the real files of the user's documents.
//!
//! ```text
//! unsaved/
//!   session.ini        the windows, their tabs and the ids of the documents ([`Session`])
//!   <id>.doc           one document: the information ([`DocInfo`]) and the text, in one file
//!   lock               held by the process that uses the folder ([`Store::open`])
//!   rejected/          what could not be restored, kept for 30 days
//! ```
//!
//! # The order that keeps a crash harmless
//!
//! Every file is written to `<name>.tmp` in the same folder, flushed to the disk, renamed over the
//! target, and then the folder is flushed ([`Store::write_doc`], [`Store::write_session`]).
//!
//! * A new document: its `.doc` is written first, `session.ini` after. A crash between the two leaves a
//!   `.doc` that `session.ini` does not list ([`Store::scan`] reports it as an orphan; the editor
//!   restores it).
//! * A removed document ([`Store::remove_doc`]): the `.doc` is deleted first, `session.ini` is
//!   rewritten after. A crash between the two leaves an entry whose file is gone (a "dangling" entry,
//!   skipped when restoring). So an orphan is always an unfinished addition, never a document the user
//!   threw away.
//!
//! The text is passed as a writer callback and read as a stream, so the module never asks for a whole
//! document as one string.
//!
//! This crate has no UI. The editor's writer (a queue with one thread and a generation for each
//! document) is in `matsu-ui`.

mod file;
mod info;
mod lock;
mod session;
mod store;
#[cfg(test)]
mod tests;

pub use file::{DocFile, FormatError};
pub use info::DocInfo;
pub use session::{Session, SessionWindow, is_valid_id};
pub use store::{REJECTED_KEEP, Scan, Store, StoreError, unsaved_dir};
