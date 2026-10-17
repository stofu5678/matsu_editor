//! Typed settings for the first editor window: the font, and the settings of a document type (tab width,
//! wrap, line numbers, what is shown, kinsoku, the encoding and line ending of new files, and the rules of the
//! coloring: comments, strings, numbers, keywords). [`DocumentTypes`] is the list of types with Sakura's 17
//! built-in ones and the choice of a type by file name.
//!
//! Everything else in the file stays in the [`Profile`](crate::Profile) as raw entries.

pub mod builtin_types;
pub mod color;
pub mod custom_menu;
pub mod doc_types;
pub mod exec_history;
pub mod extra_types;
pub mod font;
pub mod keyword_data;
pub mod keywords;
pub mod search_keys;
pub mod type_rules;
pub mod types;

pub use builtin_types::builtin_types;
pub use color::{ColorAttributes, ColorEntry, ColorSet};
pub use custom_menu::{CustomMenu, CustomMenuItem, CustomMenus};
pub use doc_types::DocumentTypes;
pub use exec_history::ExecHistory;
pub use extra_types::ExtraSet;
pub use font::{FontSetting, LogFont, ViewFont};
pub use keywords::{KeywordSet, KeywordSets};
pub use search_keys::{KeyList, SearchKeys};
pub use type_rules::{BlockComment, LineComment, RegexKeyword};
pub use types::{NewFileKeys, TypeSettings, types_count};
