//! Outline analysis of MATSU Editor (Sakura's アウトライン解析): the analyzers that list the
//! functions, classes, headings or bookmarks of a document, and the tree that the outline pane
//! shows.
//!
//! The crate knows nothing of a window. It reads a document line by line through a
//! [`LineSource`] (never as one string, so a very large file is read only as far as the analysis
//! goes) and gives a [`FuncInfoArr`], Sakura's `CFuncInfoArr`. `matsu-ui` runs the analysis in the
//! background and draws the result.
//!
//! # Where it lives
//!
//! A crate of its own, next to `matsu-coloring`: the analyzers need the lines of a document and the
//! outline kinds of the document types (`matsu-config`) and nothing else, so tests run without a
//! window and an analysis can run on a clone of the document in a background task. `matsu-core`
//! stays free of per-language rules, and `matsu-coloring` is about the colors of a line.
//!
//! # Analyzers
//!
//! One module per kind, each following the Sakura function it is named after:
//!
//! | module | Sakura | kind |
//! |---|---|---|
//! | [`cpp`] | `CDocOutline::MakeFuncList_C` | C, C++, C/C++ |
//! | [`java`] | `MakeFuncList_Java` | Java |
//! | [`python`] | `MakeFuncList_python` | Python |
//! | [`perl`] | `MakeFuncList_Perl` | Perl |
//! | [`vb`] | `MakeFuncList_VisualBasic` | Visual Basic |
//! | [`text`] | `MakeTopicList_txt`, `MakeTopicList_wztxt` | text, text with levels |
//! | [`html`] | `MakeTopicList_html` | HTML, XML |
//! | [`bookmark`] | `MakeFuncList_BookMark` | the bookmark list |
//!
//! # Where this differs from Sakura
//!
//! * Positions are bytes of UTF-8 in the buffer; the analyzers walk the characters of a line and
//!   report the line (1 origin, as `m_nFuncLineCRLF`). The column of every item is 1, as in
//!   Sakura's analyzers. The line number in the layout (`m_nFuncLineLAYOUT`) is worked out by the
//!   window from the line, when it shows the numbers of wrapped rows.
//! * A character outside the BMP counts as one character, not two code units; where Sakura tests
//!   for a two-unit character (`GetSizeOfChar`) the analyzers test for such a character.
//! * `m_bEnableExtEol` (NEL, LS and PS as line endings) is off, as it is by default in Sakura.
//! * Not ported yet: PL/SQL, COBOL, assembler, TeX, Erlang and the rule file.

pub mod bookmark;
mod chars;
pub mod cpp;
pub mod html;
mod info;
pub mod java;
mod kind;
pub mod perl;
pub mod python;
mod source;
pub mod text;
pub mod tree;
pub mod vb;

#[cfg(test)]
mod tests;

pub use info::{FuncInfo, FuncInfoArr, NO_CLIP_TEXT, func_obj};
pub use kind::{Analysis, OutlineKind, Request, ViewType, analyze};
pub use source::{LineSource, VecSource};
pub use tree::{ListColumn, ListSort, SortType, TreeNode};
