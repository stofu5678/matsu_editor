//! Which document type a file has: by its name, from the extensions of each type.
//!
//! The rules (Sakura's `GetDocumentTypeOfPath` and `IsFileNameMatch`) and the types are in
//! `ume-config` ([`ume_config::settings::doc_types`]); this module keeps the old path.

pub use ume_config::settings::doc_types::{is_file_name_match, type_index_for};
