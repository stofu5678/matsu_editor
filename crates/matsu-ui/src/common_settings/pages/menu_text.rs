//! The text of a menu item that the file holds for a function that is not known, as the numbers of the
//! page's value (the main menu and the custom menus both keep it with the item).

/// Appends a text: its length in UTF-16 units (`-1` for none) and the units.
pub(super) fn push_text(out: &mut Vec<i32>, text: Option<&str>) {
    match text {
        None => out.push(-1),
        Some(text) => {
            let units: Vec<u16> = text.encode_utf16().collect();
            out.push(units.len() as i32);
            out.extend(units.into_iter().map(i32::from));
        }
    }
}

/// The text [`push_text`] made, read with `next` (which gives the numbers one by one).
pub(super) fn read_text(next: &mut dyn FnMut() -> Option<i32>) -> Option<String> {
    let length = usize::try_from(next()?).ok()?;
    let units: Vec<u16> = (0..length)
        .map_while(|_| next().and_then(|u| u16::try_from(u).ok()))
        .collect();
    Some(String::from_utf16_lossy(&units))
}
