//! `NSFileManager`'s `trashItemAtURL:resultingItemURL:error:`. Not run by CI; the user checks it.

use std::io;
use std::path::Path;

use objc2_foundation::{NSFileManager, NSString, NSURL};

pub(super) fn move_to_trash(path: &Path) -> io::Result<()> {
    let text = std::path::absolute(path)?.to_string_lossy().into_owned();
    let url = NSURL::fileURLWithPath(&NSString::from_str(&text));
    NSFileManager::defaultManager()
        .trashItemAtURL_resultingItemURL_error(&url, None)
        .map_err(|e| io::Error::other(e.localizedDescription().to_string()))
}
