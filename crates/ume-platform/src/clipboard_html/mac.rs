//! The type `public.html` of the general pasteboard on macOS. Not run by CI; the user checks it.

use objc2_app_kit::NSPasteboard;
use objc2_foundation::NSString;

/// Adds the HTML to the general pasteboard, whose other types stay (GPUI cleared it and wrote
/// the text before).
pub(super) fn add_html(fragment: &str) -> bool {
    let pasteboard = NSPasteboard::generalPasteboard();
    // `NSPasteboardTypeHTML`
    let kind = NSString::from_str("public.html");
    pasteboard.setString_forType(&NSString::from_str(fragment), &kind)
}
