//! The settings of the copies of unsaved documents (ADR-0022, section 8).
//!
//! Sakura has no such items, so they live in the section `[MATSU]` that Sakura does not use (ADR-0010):
//! `KeepUnsaved`, `KeepUnsavedNoAsk` and `KeepUnsavedMaxMB`. The first two are checkboxes on the page ファイル of
//! the common settings; the last one is read from the file only.

use gpui::{App, Global};
use matsu_config::Profile;

/// The section of the items that Sakura does not have.
pub const SECTION: &str = "MATSU";
/// Keep unsaved documents and bring them back at the next start. On by default.
pub const KEY_KEEP: &str = "KeepUnsaved";
/// Do not ask 「保存しますか」 when the editor quits (hot exit). Off by default. Read by the quit flow (a later
/// change); the setting is kept here so that the page has it.
pub const KEY_NO_ASK: &str = "KeepUnsavedNoAsk";
/// A document larger than this many MiB is not kept; 0 means no limit. 64 by default.
pub const KEY_MAX_MB: &str = "KeepUnsavedMaxMB";

/// The settings now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeepPrefs {
    /// `KeepUnsaved`.
    pub keep: bool,
    /// `KeepUnsavedNoAsk`.
    pub no_ask: bool,
    /// `KeepUnsavedMaxMB`.
    pub max_mb: u32,
}

impl Default for KeepPrefs {
    fn default() -> Self {
        Self {
            keep: true,
            no_ask: false,
            max_mb: 64,
        }
    }
}

impl Global for KeepPrefs {}

impl KeepPrefs {
    /// The size limit in bytes, `None` when there is none.
    pub fn max_bytes(&self) -> Option<u64> {
        (self.max_mb != 0).then(|| u64::from(self.max_mb) * 1024 * 1024)
    }

    /// Reads the three items; a missing or unreadable one has its default.
    pub fn from_profile(profile: &Profile) -> Self {
        let default = Self::default();
        Self {
            keep: profile.get_bool(SECTION, KEY_KEEP).unwrap_or(default.keep),
            no_ask: profile
                .get_bool(SECTION, KEY_NO_ASK)
                .unwrap_or(default.no_ask),
            // A negative number is not a size (`get_u32` would wrap it around).
            max_mb: profile
                .get_i32(SECTION, KEY_MAX_MB)
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or(default.max_mb),
        }
    }
}

/// The settings now.
pub fn prefs(cx: &App) -> KeepPrefs {
    cx.try_global::<KeepPrefs>().copied().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> KeepPrefs {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        KeepPrefs::from_profile(&Profile::from_bytes(&bytes))
    }

    #[test]
    fn the_defaults_are_on_ask_and_64_mib() {
        let p = read("");
        assert_eq!(
            p,
            KeepPrefs {
                keep: true,
                no_ask: false,
                max_mb: 64
            }
        );
        assert_eq!(p.max_bytes(), Some(64 * 1024 * 1024));
    }

    #[test]
    fn the_items_are_read() {
        let p = read("[MATSU]\r\nKeepUnsaved=0\r\nKeepUnsavedNoAsk=1\r\nKeepUnsavedMaxMB=0\r\n");
        assert_eq!(
            p,
            KeepPrefs {
                keep: false,
                no_ask: true,
                max_mb: 0
            }
        );
        assert_eq!(p.max_bytes(), None);
    }

    #[test]
    fn unreadable_items_have_their_defaults() {
        let p = read("[MATSU]\r\nKeepUnsaved=abc\r\nKeepUnsavedMaxMB=-5\r\n");
        assert_eq!(p.keep, KeepPrefs::default().keep);
        assert_eq!(p.max_mb, 64);
    }
}
