//! A [`Source`] that reads a file by ranges (ADR-0020, section 1, plan B).
//!
//! [`RangedFile`] keeps the file open and reads only the blocks a range touches, through a cache of
//! fixed-size blocks (LRU). Nothing reads the whole file, and nothing is memory-mapped: a file that
//! shrinks while it is open makes a read fail with an error ([`ChangedOutside`]) instead of
//! crashing the process with SIGBUS.
//!
//! # When the file changes outside UME
//!
//! The length and the modification time are remembered at open (ADR-0020, "ファイルが外から変わった
//! とき"). When a block is read from the disk:
//!
//! * a short read, a length smaller than at open, or the same length with another modification
//!   time fails with [`ChangedOutside`], and every later read fails the same way (the bytes the
//!   document refers to may be gone);
//! * a longer file is an append: the bytes up to the length at open stay readable.
//!
//! Blocks that are already cached are not checked again (no system call on a cache hit); the
//! caller's file watcher finds a changed modification time and can call [`RangedFile::check`].
//! The error is a plain [`io::Error`] (the `Source` trait returns one); [`is_changed_outside`]
//! tells it from other errors.

mod os;
#[cfg(test)]
mod tests;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::buffer::Source;

/// The size of a cached block, 256 KiB (ADR-0020, section 1).
pub const DEFAULT_BLOCK_SIZE: usize = 256 * 1024;
/// The number of cached blocks. With the default block size the cache is 64 MiB.
pub const DEFAULT_CACHE_BLOCKS: usize = 256;

/// Sizes of a [`RangedFile`]. Small values make tests cross block edges with small files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangedOptions {
    /// Bytes per block. Must not be 0.
    pub block_size: usize,
    /// Blocks the cache keeps. Must not be 0.
    pub cache_blocks: usize,
}

impl Default for RangedOptions {
    fn default() -> Self {
        Self {
            block_size: DEFAULT_BLOCK_SIZE,
            cache_blocks: DEFAULT_CACHE_BLOCKS,
        }
    }
}

/// The error of a read when the file was changed by another program after it was opened.
///
/// It is the payload of the [`io::Error`] that [`Source::read`] returns; use
/// [`is_changed_outside`] to look for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChangedOutside;

impl fmt::Display for ChangedOutside {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the file was changed outside UME")
    }
}

impl std::error::Error for ChangedOutside {}

/// Whether `error` is the [`ChangedOutside`] error of a [`RangedFile`].
pub fn is_changed_outside(error: &io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|inner| inner.is::<ChangedOutside>())
}

fn changed_outside() -> io::Error {
    io::Error::other(ChangedOutside)
}

/// How often the cache answered, for measuring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// Blocks found in the cache.
    pub hits: u64,
    /// Blocks read from the file.
    pub misses: u64,
}

#[derive(Debug)]
struct Cache {
    /// Block number -> (bytes, time of the last use).
    blocks: HashMap<u64, (Arc<Vec<u8>>, u64)>,
    tick: u64,
}

impl Cache {
    fn get(&mut self, number: u64) -> Option<Arc<Vec<u8>>> {
        self.tick += 1;
        let tick = self.tick;
        let (bytes, used) = self.blocks.get_mut(&number)?;
        *used = tick;
        Some(bytes.clone())
    }

    fn insert(&mut self, number: u64, bytes: Arc<Vec<u8>>, capacity: usize) {
        self.tick += 1;
        while self.blocks.len() >= capacity && !self.blocks.contains_key(&number) {
            // At most `cache_blocks` (256 by default) entries: a scan is cheap next to the read
            // that got us here.
            let Some(oldest) = self
                .blocks
                .iter()
                .min_by_key(|(_, (_, used))| *used)
                .map(|(number, _)| *number)
            else {
                break;
            };
            self.blocks.remove(&oldest);
        }
        self.blocks.insert(number, (bytes, self.tick));
    }
}

/// A file read by ranges through a block cache. See the module documentation.
#[derive(Debug)]
pub struct RangedFile {
    file: File,
    /// The length at open: the length of this source, whatever the file grows to.
    len: u64,
    modified: Option<SystemTime>,
    block_size: u64,
    capacity: usize,
    cache: Mutex<Cache>,
    changed: AtomicBool,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl RangedFile {
    /// Opens `path` with the default sizes.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::open_with(path, RangedOptions::default())
    }

    /// Opens `path`. Only the metadata is read.
    ///
    /// # Panics
    ///
    /// When a size in `options` is 0.
    pub fn open_with(path: impl AsRef<Path>, options: RangedOptions) -> io::Result<Self> {
        Self::from_file(File::open(path)?, options)
    }

    /// Wraps a file that is already open.
    ///
    /// # Panics
    ///
    /// When a size in `options` is 0.
    pub fn from_file(file: File, options: RangedOptions) -> io::Result<Self> {
        assert!(options.block_size > 0 && options.cache_blocks > 0);
        let meta = file.metadata()?;
        Ok(Self {
            len: meta.len(),
            modified: meta.modified().ok(),
            block_size: options.block_size as u64,
            capacity: options.cache_blocks,
            cache: Mutex::new(Cache {
                blocks: HashMap::new(),
                tick: 0,
            }),
            changed: AtomicBool::new(false),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            file,
        })
    }

    /// Looks at the file's metadata and fails with [`ChangedOutside`] when the file has become
    /// shorter, or has another modification time at the same length. A longer file is fine.
    ///
    /// Reads from the disk do this themselves; this is for a caller that wants to ask (for
    /// example when its file watcher saw a new modification time). Once it fails, it keeps
    /// failing, as the reads do.
    pub fn check(&self) -> io::Result<()> {
        if self.changed.load(Ordering::Acquire) {
            return Err(changed_outside());
        }
        let meta = self.file.metadata()?;
        let unchanged = meta.len() > self.len
            || (meta.len() == self.len && meta.modified().ok() == self.modified);
        if unchanged {
            Ok(())
        } else {
            self.changed.store(true, Ordering::Release);
            Err(changed_outside())
        }
    }

    /// The cache's hits and misses since open.
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
        }
    }

    /// The bytes the cache holds right now.
    pub fn cached_bytes(&self) -> usize {
        let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.blocks.values().map(|(bytes, _)| bytes.len()).sum()
    }

    /// Reads exactly `len` bytes at `start` from the file. A file that ends early has changed.
    fn read_from_disk(&self, start: u64, len: usize) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        buf.try_reserve_exact(len)
            .map_err(|_| io::Error::from(io::ErrorKind::OutOfMemory))?;
        buf.resize(len, 0);
        let mut filled = 0;
        while filled < len {
            match os::read_at(&self.file, &mut buf[filled..], start + filled as u64)? {
                0 => {
                    self.changed.store(true, Ordering::Release);
                    return Err(changed_outside());
                }
                n => filled += n,
            }
        }
        Ok(buf)
    }

    fn block(&self, number: u64) -> io::Result<Arc<Vec<u8>>> {
        {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(bytes) = cache.get(number) {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Ok(bytes);
            }
        }
        // Read outside the lock so that a slow disk does not block readers of cached blocks. Two
        // threads may read the same block; the second insert just replaces the first.
        self.misses.fetch_add(1, Ordering::Relaxed);
        self.check()?;
        let start = number * self.block_size;
        let size = self.block_size.min(self.len - start);
        let buf = self.read_from_disk(start, size as usize)?;
        let bytes = Arc::new(buf);
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        cache.insert(number, bytes.clone(), self.capacity);
        Ok(bytes)
    }
}

impl Source for RangedFile {
    fn len(&self) -> u64 {
        self.len
    }

    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
        if start > end || end > self.len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "range out of bounds",
            ));
        }
        if self.changed.load(Ordering::Acquire) {
            return Err(changed_outside());
        }
        if start == end {
            return Ok(Cow::Borrowed(&[]));
        }
        if end - start >= 2 * self.block_size {
            // A long range (a scan reading in big pieces) is read straight into its buffer: the
            // blocks would be copied twice and would push out the blocks that the view uses.
            self.check()?;
            return self
                .read_from_disk(start, (end - start) as usize)
                .map(Cow::Owned);
        }
        let mut out = Vec::new();
        out.try_reserve_exact((end - start) as usize)
            .map_err(|_| io::Error::from(io::ErrorKind::OutOfMemory))?;
        let mut pos = start;
        while pos < end {
            let number = pos / self.block_size;
            let block = self.block(number)?;
            let from = (pos - number * self.block_size) as usize;
            let to = block.len().min(from + (end - pos) as usize);
            out.extend_from_slice(&block[from..to]);
            pos += (to - from) as u64;
        }
        Ok(Cow::Owned(out))
    }
}
