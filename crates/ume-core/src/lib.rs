//! UI-independent core of UME Editor.
//!
//! The buffer, the cursor, undo, encodings, search, and editing commands live here. This crate
//! must not depend on any UI crate (see `CLAUDE.md`).

pub mod buffer;
pub mod cursor;
pub mod encoding;
pub mod model;
pub mod search;
pub mod undo;

/// Returns the crate version, so that the workspace has something to build
/// and test until the real modules land in Phase 1.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_manifest() {
        assert_eq!(version(), "0.0.0");
    }
}
