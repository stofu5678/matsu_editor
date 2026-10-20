//! HTML on the clipboard, next to the text that GPUI puts there (the colored HTML copy,
//! `F_COPY_COLOR_HTML`).
//!
//! Sakura: `CClipboard::SetHtmlText` in `sakura_core/_os/CClipboard.cpp` registers the clipboard
//! format `HTML Format` and puts the HTML in it behind the header of offsets that format needs
//! ([`cf_html`]). The caller writes the text first (GPUI empties the clipboard when it writes),
//! then [`add_html`] adds the HTML to what is there.
//!
//! * Windows: the format `HTML Format` with the header of [`cf_html`], as Sakura does.
//! * macOS: the type `public.html` of the general pasteboard (it has no header of offsets).
//! * Linux: nothing. GPUI's clipboard there offers plain text only and this crate has no
//!   clipboard of its own to add a second type to, so the HTML source goes as text only.
//!
//! `unsafe` is in `windows` only.

#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod windows;

/// The page around the fragment, as `CClipboard::SetHtmlText` writes it.
const PREFIX: &str = "<html><body>\r\n<!--StartFragment -->\r\n";
const SUFFIX: &str = "\r\n<!--EndFragment-->\r\n</body></html>\r\n";

/// The bytes of the clipboard format `HTML Format` for the HTML fragment `fragment`: the header
/// of byte offsets (`Version`, `StartHTML`, `EndHTML`, `StartFragment`, `EndFragment`, each
/// number with 8 digits so that the header has a fixed length), the page around the fragment,
/// and the fragment as UTF-8. Sakura's `SetHtmlText` writes the same header; its `EndHTML` is 2
/// bytes short of the end of the data, and here it is the length of the data, as the format
/// describes it.
pub fn cf_html(fragment: &str) -> Vec<u8> {
    // The header has the same length whatever the numbers are: 97 bytes.
    const HEADER_LEN: usize = "Version:0.9\r\n".len()
        + "StartHTML:00000000\r\n".len()
        + "EndHTML:00000000\r\n".len()
        + "StartFragment:00000000\r\n".len()
        + "EndFragment:00000000\r\n".len();
    let start_fragment = HEADER_LEN + PREFIX.len();
    let end_fragment = start_fragment + fragment.len();
    let end_html = end_fragment + SUFFIX.len();
    let mut out = String::with_capacity(end_html);
    out.push_str("Version:0.9\r\n");
    out.push_str(&format!("StartHTML:{HEADER_LEN:08}\r\n"));
    out.push_str(&format!("EndHTML:{end_html:08}\r\n"));
    out.push_str(&format!("StartFragment:{start_fragment:08}\r\n"));
    out.push_str(&format!("EndFragment:{end_fragment:08}\r\n"));
    out.push_str(PREFIX);
    out.push_str(fragment);
    out.push_str(SUFFIX);
    out.into_bytes()
}

/// Adds the HTML fragment `fragment` to the clipboard, in the format the OS has for HTML.
/// Returns whether it was added; it is not on Linux, and a clipboard that another program holds
/// open refuses it. The text that was put on the clipboard stays either way.
pub fn add_html(fragment: &str) -> bool {
    #[cfg(windows)]
    {
        windows::add_html(&cf_html(fragment))
    }
    #[cfg(target_os = "macos")]
    {
        mac::add_html(fragment)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = fragment;
        false
    }
}

#[cfg(test)]
mod tests;
