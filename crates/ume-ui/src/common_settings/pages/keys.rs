//! 「キー割り当て」: `IDD_PROP_KEYBIND` (`sakura_core/sakura_rc.rc`), `CPropKeybind`
//! (`sakura_core/prop/CPropComKeybind.cpp`).
//!
//! Not built yet: [`items`] has no controls, so the dialog shows the page as not available.

use super::super::Env;
use super::super::model::{Item, Values};

/// The controls of the page, or `None` while the page is not built.
pub(in crate::common_settings) fn items() -> Option<Vec<Item>> {
    None
}

/// What OK does to the windows.
pub(in crate::common_settings) fn apply(_env: &mut Env, _after: &Values, _before: Option<&Values>) {
}
