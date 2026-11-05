//! The check of the free space before a save (ADR-0020, chapter 4; ADR-0005).
//!
//! A save writes a whole new file next to the target and only then replaces it, so while it runs
//! the disk holds both. For a file of several GB that is the cost most likely to fail, and it
//! fails after the time of writing it. The check asks the OS for the free space of the folder of
//! the target before anything is created and refuses ([`FileError::NoSpace`]) when it is less than
//! an estimate of what will be written.
//!
//! Sakura has no such check: it writes into the file in place (`CWriteManager`) and a full disk
//! shows as a write error half way through. The check is ours because of ADR-0005.
//!
//! # Where the number comes from
//!
//! The core knows nothing of the OS. The caller gives a probe ([`FreeSpaceProbe`]) with the
//! options of the save ([`SaveOptions::free_space`](super::SaveOptions)), or once for the process
//! ([`set_default_free_space_probe`]); the app uses the one of `matsu_platform::free_space`. With no
//! probe, or a probe that fails, nothing is checked: the write fails by itself when the disk is
//! full, and removes its temporary file.

use std::fmt;
use std::io;
use std::path::Path;
use std::sync::RwLock;

use super::error::{FileError, Result};
use super::save::SaveOptions;
use crate::buffer::Document;
use crate::encoding::Encoding;

/// Extra room asked for on top of every estimate: the BOM, the buffers and what a file system keeps
/// for itself.
const MARGIN: u64 = 1024 * 1024;

/// Extra room asked for when the bytes are converted, where the size of the result is a guess.
const CONVERT_MARGIN: u64 = 16 * 1024 * 1024;

/// A function that tells how many bytes can still be written in the folder `path`, to the user
/// of the program (quotas count).
#[derive(Clone, Copy)]
pub struct FreeSpaceProbe(pub fn(&Path) -> io::Result<u64>);

impl fmt::Debug for FreeSpaceProbe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("FreeSpaceProbe")
    }
}

impl PartialEq for FreeSpaceProbe {
    fn eq(&self, other: &Self) -> bool {
        self.0 as usize == other.0 as usize
    }
}

impl Eq for FreeSpaceProbe {}

static DEFAULT_PROBE: RwLock<Option<FreeSpaceProbe>> = RwLock::new(None);

/// Sets the probe that saves use when their options have none. `None` takes it away.
pub fn set_default_free_space_probe(probe: Option<FreeSpaceProbe>) {
    *DEFAULT_PROBE
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = probe;
}

fn default_probe() -> Option<FreeSpaceProbe> {
    *DEFAULT_PROBE
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// How many bytes one byte of added text (UTF-8) can become in `encoding`: ASCII text doubles in
/// UTF-16 and quadruples in UTF-32; the encodings with escape sequences (ISO-2022-JP, UTF-7) can
/// double it; every other one writes at most as many bytes as UTF-8 has.
fn expansion(encoding: Encoding) -> u64 {
    match encoding {
        Encoding::Utf16Le | Encoding::Utf16Be | Encoding::Jis | Encoding::Utf7 => 2,
        Encoding::Utf32Le | Encoding::Utf32Be => 4,
        _ => 1,
    }
}

/// How many bytes a save of `doc` can write at most, for the check.
///
/// * When the original bytes are copied (the encoding and the line endings are not changed) the
///   file is the original bytes of the pieces plus the added text encoded: `raw_len` counts both, and
///   the whole add buffer (also what was edited away) is counted again at its worst expansion.
/// * Otherwise every line is decoded and encoded, and the size is not known before: it is taken as
///   half as much again as the original (a change of encoding rarely does more), plus the added
///   text, plus a margin ([ADR-0020] "保存の空き容量").
///
/// [ADR-0020]: https://github.com/stofu5678/matsu_editor/blob/main/docs/adr/0020-huge-files.md
pub(super) fn estimate(doc: &Document, copies_original: bool, options: &SaveOptions) -> u64 {
    let raw = doc.raw_len() as u64;
    let added = doc.added_bytes() as u64;
    let grow = expansion(options.encoding);
    if copies_original {
        raw.saturating_add(added.saturating_mul(grow.saturating_sub(1)))
            .saturating_add(MARGIN)
    } else {
        raw.saturating_add(raw / 2)
            .saturating_add(added.saturating_mul(grow))
            .saturating_add(MARGIN + CONVERT_MARGIN)
    }
}

/// Refuses with [`FileError::NoSpace`] when the folder `dir` has less free space than `needed`.
/// A probe that is not there or fails passes: the check is a courtesy, not a gate.
pub(super) fn ensure(dir: &Path, needed: u64, options: &SaveOptions) -> Result<()> {
    let Some(probe) = options.free_space.or_else(default_probe) else {
        return Ok(());
    };
    match (probe.0)(dir) {
        Ok(available) if available < needed => Err(FileError::NoSpace { needed, available }),
        _ => Ok(()),
    }
}
