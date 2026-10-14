//! The bytes of an opened file as a buffer [`Source`], without the BOM.

use std::borrow::Cow;
use std::io;
use std::sync::Arc;

use crate::buffer::Source;

/// The bytes of a file held in memory. The first `start` bytes (the BOM) are not part of the text.
#[derive(Debug)]
pub(crate) struct FileBytes {
    data: Arc<Vec<u8>>,
    start: usize,
}

impl FileBytes {
    pub fn new(data: Arc<Vec<u8>>, start: usize) -> Self {
        Self {
            start: start.min(data.len()),
            data,
        }
    }
}

impl Source for FileBytes {
    fn len(&self) -> u64 {
        (self.data.len() - self.start) as u64
    }

    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
        let range = usize::try_from(start)
            .ok()
            .zip(usize::try_from(end).ok())
            .filter(|(s, e)| s <= e)
            .and_then(|(s, e)| self.data.get(self.start + s..self.start.checked_add(e)?));
        range
            .map(Cow::Borrowed)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "range out of bounds"))
    }
}
