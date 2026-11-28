//! Counters and timers for the measurement of `docs/measurements/long-line-normal.md`.
//!
//! They exist only with the `measure` feature, which the workspace never turns on by itself
//! (`cargo test --workspace` does not use it). Without the feature every function here is empty
//! and a [`Span`] has no fields and no `Drop`, so the product code that calls them is not
//! slower. Nothing here changes what a caller gets.
//!
//! The counts are per thread: GPUI's test platform runs a test on one thread.

/// Whether the counters are compiled in.
pub const ENABLED: bool = cfg!(feature = "measure");

/// What was seen under one name since the last [`reset`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    pub name: &'static str,
    /// The place that called the function of the span (`span_at`), or "" for a span without one.
    pub file: &'static str,
    pub line: u32,
    /// How many times [`add`] was called or a [`Span`] ended.
    pub calls: u64,
    /// The sum of the bytes given to [`add`] or [`Span::bytes`].
    pub bytes: u64,
    /// The sum of the time of the spans, in nanoseconds (nested spans count inside each other).
    pub nanos: u64,
}

#[cfg(feature = "measure")]
mod on {
    use super::Entry;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::time::Instant;

    type Key = (&'static str, &'static str, u32);

    thread_local! {
        static ENTRIES: RefCell<BTreeMap<Key, Entry>> = const { RefCell::new(BTreeMap::new()) };
    }

    fn with(key: Key, f: impl FnOnce(&mut Entry)) {
        ENTRIES.with(|e| {
            let mut map = e.borrow_mut();
            f(map.entry(key).or_insert_with(|| Entry {
                name: key.0,
                file: key.1,
                line: key.2,
                ..Entry::default()
            }));
        });
    }

    pub fn add(name: &'static str, bytes: usize) {
        with((name, "", 0), |e| {
            e.calls += 1;
            e.bytes += bytes as u64;
        });
    }

    pub fn reset() {
        ENTRIES.with(|e| e.borrow_mut().clear());
    }

    pub fn take() -> Vec<Entry> {
        ENTRIES.with(|e| std::mem::take(&mut *e.borrow_mut()).into_values().collect())
    }

    #[must_use]
    pub struct Span {
        key: Key,
        bytes: u64,
        start: Instant,
    }

    pub fn span(name: &'static str) -> Span {
        Span {
            key: (name, "", 0),
            bytes: 0,
            start: Instant::now(),
        }
    }

    /// A span that remembers where the function was called from (the function must be
    /// `#[track_caller]` for that to be its caller and not the function itself).
    #[track_caller]
    pub fn span_at(name: &'static str) -> Span {
        let at = std::panic::Location::caller();
        Span {
            key: (name, at.file(), at.line()),
            bytes: 0,
            start: Instant::now(),
        }
    }

    impl Span {
        /// Counts `bytes` for this span.
        pub fn bytes(&mut self, bytes: usize) {
            self.bytes += bytes as u64;
        }
    }

    impl Drop for Span {
        fn drop(&mut self) {
            let nanos = self.start.elapsed().as_nanos() as u64;
            let bytes = self.bytes;
            with(self.key, |e| {
                e.calls += 1;
                e.bytes += bytes;
                e.nanos += nanos;
            });
        }
    }
}

#[cfg(not(feature = "measure"))]
mod on {
    use super::Entry;

    #[inline(always)]
    pub fn add(_name: &'static str, _bytes: usize) {}
    #[inline(always)]
    pub fn reset() {}
    #[inline(always)]
    pub fn take() -> Vec<Entry> {
        Vec::new()
    }
    #[must_use]
    pub struct Span;
    #[inline(always)]
    pub fn span(_name: &'static str) -> Span {
        Span
    }
    #[inline(always)]
    pub fn span_at(_name: &'static str) -> Span {
        Span
    }
    impl Span {
        #[inline(always)]
        pub fn bytes(&mut self, _bytes: usize) {}
    }
}

pub use on::{Span, add, reset, span, span_at, take};
