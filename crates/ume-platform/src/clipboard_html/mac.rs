//! The type `public.html` of the general pasteboard on macOS. Not run by CI; the user checks it.

// `addTypes:owner:` is `unsafe` in objc2. See the list of the modules that need it in `lib.rs`.
#![allow(unsafe_code)]

use objc2_app_kit::NSPasteboard;
use objc2_foundation::{NSArray, NSString};

/// Adds the HTML to the general pasteboard, whose other types stay (GPUI cleared it and wrote
/// the text before).
pub(super) fn add_html(fragment: &str) -> bool {
    let pasteboard = NSPasteboard::generalPasteboard();
    // `NSPasteboardTypeHTML`
    let kind = NSString::from_str("public.html");
    // The type is declared first, without an owner, because the data is given right after: a
    // type that the pasteboard does not list is refused by `setString:forType:`.
    // SAFETY: an array of one type, and no owner (`nil` is allowed when the data is set now).
    unsafe { pasteboard.addTypes_owner(&NSArray::from_slice(&[&*kind]), None) };
    pasteboard.setString_forType(&NSString::from_str(fragment), &kind)
}
