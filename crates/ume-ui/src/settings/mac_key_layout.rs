//! The macOS key layout: how Sakura's key table is read on a Mac (ADR-0008).
//!
//! Sakura has no such item, so it lives in the section `[UME]` that Sakura does not use
//! (ADR-0010): `nMacKeyLayout=0|1|2`. Other OSes read it and keep it in the file but ignore it.

use ume_commands::keymap::{KeyMapOptions, Platform};
use ume_config::Profile;

/// The section of the items that Sakura does not have.
pub const UME_SECTION: &str = "UME";
/// The key of the layout: `0` standard, `1` all Control, `2` Cmd as Ctrl.
pub const KEY: &str = "nMacKeyLayout";

/// The ways to read Sakura's key table on macOS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MacKeyLayout {
    /// 0 (the default): Control is Sakura's Ctrl, Option is Alt, and Cmd has the Mac shortcuts
    /// only (`CMD_KEYS`: Cmd+S, Cmd+C, ...).
    #[default]
    Standard,
    /// 1: all Control. Cmd does nothing, so the hands are as with Sakura on Windows.
    AllControl,
    /// 2: the older reading. Sakura's Ctrl is read as Cmd (Ctrl+S is Cmd+S).
    CmdAsCtrl,
}

impl MacKeyLayout {
    /// Reads `[UME]` `nMacKeyLayout`. A missing item, text that is not a number, and a number
    /// that is not 0, 1 or 2 are the standard layout.
    pub fn from_profile(profile: &Profile) -> Self {
        match profile.get_i32(UME_SECTION, KEY) {
            Some(1) => Self::AllControl,
            Some(2) => Self::CmdAsCtrl,
            _ => Self::Standard,
        }
    }

    /// The key map options for `platform`; the other OSes do not look at the layout.
    pub fn options(self, platform: Platform) -> KeyMapOptions {
        let mut options = KeyMapOptions::for_platform(platform);
        if platform == Platform::MacOs {
            match self {
                Self::Standard => {}
                Self::AllControl => options.cmd_keys = false,
                Self::CmdAsCtrl => options.ctrl_as_cmd = true,
            }
        }
        options
    }
}

#[cfg(test)]
mod tests {
    use ume_commands::keymap::{Chord, Key, KeyMap, Modifiers, Resolved};

    use super::*;

    fn layout_of(text: &str) -> MacKeyLayout {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        MacKeyLayout::from_profile(&Profile::from_bytes(&bytes))
    }

    #[test]
    fn the_item_is_read_as_0_1_or_2() {
        let read = |v: &str| layout_of(&format!("[UME]\r\nnMacKeyLayout={v}\r\n"));
        assert_eq!(read("0"), MacKeyLayout::Standard);
        assert_eq!(read("1"), MacKeyLayout::AllControl);
        assert_eq!(read("2"), MacKeyLayout::CmdAsCtrl);
    }

    #[test]
    fn a_missing_or_unknown_item_is_the_standard_layout() {
        assert_eq!(layout_of(""), MacKeyLayout::Standard);
        assert_eq!(layout_of("[UME]\r\n"), MacKeyLayout::Standard);
        for v in ["3", "-1", "abc", ""] {
            let text = format!("[UME]\r\nnMacKeyLayout={v}\r\n");
            assert_eq!(layout_of(&text), MacKeyLayout::Standard, "{v:?}");
        }
        // Numbers are read as Sakura reads them (`wcstol`): "1.5" is 1.
        assert_eq!(
            layout_of("[UME]\r\nnMacKeyLayout=1.5\r\n"),
            MacKeyLayout::AllControl
        );
        // Another section does not count.
        assert_eq!(
            layout_of("[Common]\r\nnMacKeyLayout=2\r\n"),
            MacKeyLayout::Standard
        );
    }

    fn resolve(layout: MacKeyLayout, mods: Modifiers) -> Resolved {
        let map = KeyMap::new(layout.options(Platform::MacOs));
        map.resolve(Chord::new(Key::letter('s').unwrap(), mods))
    }

    const F_FILESAVE: u16 = 30103;

    #[test]
    fn each_layout_reads_cmd_s_and_control_s_as_described() {
        use MacKeyLayout::*;
        // Standard: Control+S is Ctrl+S, and Cmd+S is the Mac shortcut for the same function.
        assert_eq!(
            resolve(Standard, Modifiers::CTRL),
            Resolved::Code(F_FILESAVE)
        );
        assert_eq!(
            resolve(Standard, Modifiers::META),
            Resolved::Code(F_FILESAVE)
        );
        // All Control: Cmd does nothing.
        assert_eq!(
            resolve(AllControl, Modifiers::CTRL),
            Resolved::Code(F_FILESAVE)
        );
        assert_eq!(resolve(AllControl, Modifiers::META), Resolved::Unbound);
        // Cmd as Ctrl: Control+S is no longer Ctrl+S.
        assert_eq!(
            resolve(CmdAsCtrl, Modifiers::META),
            Resolved::Code(F_FILESAVE)
        );
        assert_eq!(resolve(CmdAsCtrl, Modifiers::CTRL), Resolved::Unbound);
    }

    #[test]
    fn other_oses_ignore_the_layout() {
        for layout in [
            MacKeyLayout::Standard,
            MacKeyLayout::AllControl,
            MacKeyLayout::CmdAsCtrl,
        ] {
            assert_eq!(
                layout.options(Platform::Other),
                KeyMapOptions::for_platform(Platform::Other)
            );
        }
    }
}
