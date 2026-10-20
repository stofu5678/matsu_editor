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
//! `unsafe` is in `windows` and `mac` only.

#[cfg(target_os = "macos")]
mod mac;
#[cfg(windows)]
mod windows;

/// The page around the fragment, as `CClipboard::SetHtmlText` writes it.
const PREFIX: &str = "<html><body>\r\n<!--StartFragment -->\r\n";
const SUFFIX: &str = "\r\n<!--EndFragment-->\r\n</body></html>\r\n";

/// The digits of each number of the header. Sakura writes 8 (`%08Id`), which is too few for a
/// fragment of 100 MB or more: the number gets longer, the header with it, and every offset after
/// it is wrong. 10 digits hold any size up to 9,999,999,999 bytes, and the readers of the format
/// read a number up to the end of its line, so the header is as long as ever.
const DIGITS: usize = 10;

/// The header of the format for a fragment of `fragment_len` bytes, or `None` when a number of
/// it would not fit in [`DIGITS`] digits (then the HTML is left out and only the text is copied).
fn header(fragment_len: usize) -> Option<String> {
    let head = |start_html: usize, end_html: usize, start_fragment: usize, end_fragment: usize| {
        format!(
            "Version:0.9\r\nStartHTML:{start_html:0DIGITS$}\r\nEndHTML:{end_html:0DIGITS$}\r\n\
             StartFragment:{start_fragment:0DIGITS$}\r\nEndFragment:{end_fragment:0DIGITS$}\r\n"
        )
    };
    // The length of the header does not depend on the numbers.
    let header_len = head(0, 0, 0, 0).len();
    let start_fragment = header_len + PREFIX.len();
    let end_fragment = start_fragment + fragment_len;
    let end_html = end_fragment + SUFFIX.len();
    (end_html < 10usize.pow(DIGITS as u32))
        .then(|| head(header_len, end_html, start_fragment, end_fragment))
}

/// The bytes of the clipboard format `HTML Format` for the HTML fragment `fragment`: the header
/// of byte offsets (`Version`, `StartHTML`, `EndHTML`, `StartFragment`, `EndFragment`), the page
/// around the fragment, and the fragment as UTF-8. Sakura's `SetHtmlText` writes the same
/// header with 8 digits; its `EndHTML` is 2 bytes short of the end of the data, and here it is
/// the length of the data, as the format describes it. `None` for a fragment too long for the
/// header (about 10 GB).
pub fn cf_html(fragment: &str) -> Option<Vec<u8>> {
    let mut out = header(fragment.len())?;
    out.reserve(PREFIX.len() + fragment.len() + SUFFIX.len());
    out.push_str(PREFIX);
    out.push_str(fragment);
    out.push_str(SUFFIX);
    Some(out.into_bytes())
}

/// Adds the HTML fragment `fragment` to the clipboard, in the format the OS has for HTML.
/// Returns whether it was added; it is not on Linux, and a clipboard that another program holds
/// open refuses it. The text that was put on the clipboard stays either way.
pub fn add_html(fragment: &str) -> bool {
    #[cfg(windows)]
    {
        cf_html(fragment).is_some_and(|data| windows::add_html(&data))
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
