//! The pages of the type settings, in Sakura's order (`TypePropSheetInfoList`,
//! `sakura_core/typeprop/CPropTypes.cpp`), one module each.

pub(super) mod color;
pub(super) mod screen;
pub(super) mod window;

use super::PAGE_TITLES;
use crate::common_settings::SheetPage;

/// The six pages. The first three have their controls; the others say that they are not
/// available yet.
pub(super) fn pages(_type_index: usize) -> Vec<SheetPage> {
    let items = [
        Some(screen::items()),
        Some(color::items()),
        Some(window::items()),
        None,
        None,
        None,
    ];
    PAGE_TITLES
        .into_iter()
        .zip(items)
        .map(|(title, items)| SheetPage { title, items })
        .collect()
}
