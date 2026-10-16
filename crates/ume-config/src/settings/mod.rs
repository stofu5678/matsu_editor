//! Typed settings for the first editor window: the font, and the settings of a document type (tab width,
//! wrap, line numbers, what is shown, kinsoku, the encoding and line ending of new files, and the rules of the
//! coloring: comments, strings, numbers, keywords). [`DocumentTypes`] is the list of types with Sakura's 17
//! built-in ones and the choice of a type by file name.
//!
//! Everything else in the file stays in the [`Profile`](crate::Profile) as raw entries.

pub mod builtin_types;
pub mod color;
pub mod doc_types;
pub mod font;
pub mod keyword_data;
pub mod keywords;
pub mod search_keys;
pub mod type_rules;
pub mod types;

pub use builtin_types::builtin_types;
pub use color::{ColorEntry, ColorSet};
pub use doc_types::DocumentTypes;
pub use font::{FontSetting, LogFont, ViewFont};
pub use keywords::{KeywordSet, KeywordSets};
pub use search_keys::{KeyList, SearchKeys};
pub use type_rules::{BlockComment, LineComment, RegexKeyword};
pub use types::{NewFileKeys, TypeSettings, types_count};
