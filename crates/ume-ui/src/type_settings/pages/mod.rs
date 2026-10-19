//! The pages of the type settings, in Sakura's order (`TypePropSheetInfoList`,
//! `sakura_core/typeprop/CPropTypes.cpp`), one module each.

pub(super) mod color;
pub(super) mod keyhelp;
pub(super) mod regex;
mod regex_file;
pub(super) mod screen;
pub(super) mod support;
pub(super) mod window;

use super::PAGE_TITLES;
use crate::common_settings::SheetPage;

/// The six pages.
pub(super) fn pages(_type_index: usize) -> Vec<SheetPage> {
    let items = [
        screen::items(),
        color::items(),
        window::items(),
        support::items(),
        regex::items(),
        keyhelp::items(),
    ];
    PAGE_TITLES
        .into_iter()
        .zip(items)
        .map(|(title, items)| SheetPage {
            title,
            items: Some(items),
        })
        .collect()
}
