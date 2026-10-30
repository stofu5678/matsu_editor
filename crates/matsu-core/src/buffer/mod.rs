//! The text buffer: a persistent piece tree (ADR-0005).
//!
//! # Design
//!
//! * **Two storages.** The original data stays in the file's own encoding behind a [`Source`]
//!   and is decoded through a [`Codec`] only where it is shown or searched. Text added by editing
//!   goes to an append-only UTF-8 add buffer. A piece is a byte range of one of them.
//! * **Persistent tree.** The pieces sit in a copy-on-write AVL tree (`Arc` nodes). An edit builds
//!   a new path from the root; cloning a [`Document`] copies one pointer. Undo can keep old
//!   versions and a background thread can read a clone while the UI keeps editing.
//! * **Lines.** Each node knows the byte length and the number of line breaks of its subtree, so
//!   the `k`-th line break is found in O(log n). Inside a piece, a per-storage index of line
//!   endings (5 bytes per line ending) gives the position in O(log n). A CRLF may be cut in two
//!   pieces; the node aggregates also record a leading LF and a trailing CR, which makes the two
//!   halves count as one break.
//! * **Columns.** The tree only knows raw bytes of the storages. A column is the decoded UTF-8
//!   length inside one line, so it is converted by decoding that line only.

mod block;
mod codec;
mod document;
mod error;
mod position;
mod storage;
mod tree;

#[cfg(test)]
mod tests;

pub use block::{
    BlockIndex, BlockScanner, DEFAULT_INDEX_BLOCK_SIZE, DEFAULT_LONG_LINE, ScanError, ScanFailure,
    ScanJob, ScanOptions, ScanProgress, ScanState, scan_blocks,
};
pub(crate) use codec::find_eol;
pub use codec::{Codec, EncodingCodec, Eol, INVALID_BYTE_BASE, Unmappable};
#[cfg(test)]
pub(crate) use document::with_chunk_len;
pub use document::{
    CHUNK_LEN, Chunk, Chunks, Document, Edit, Line, LineSlice, Lines, largest_decode,
    reset_largest_decode,
};
pub use error::{BufferError, Result};
pub use position::{Pos, col_from_utf16, col_to_utf16, utf16_len};
pub use storage::{MemorySource, Source};
