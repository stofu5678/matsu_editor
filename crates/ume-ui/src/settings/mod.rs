//! UME Editor's settings file (`ume.ini`, in the format of Sakura's `sakura.ini`) in the window (ADR-0010).
//!
//! [`UserSettings`] is the file read through `ume-config`; [`ViewSettings`] is what the editor
//! view takes from it for one file. Both are plain data, so they are tested without a window.
//!
//! * [`view_settings`]: font, tab width, wrap, what is shown, colors: the meaning of Sakura's keys.
//! * [`mac_key_layout`]: `[UME]` `nMacKeyLayout`, how the key table is read on macOS.
//! * [`doc_type`]: which document type a file name has.
//! * [`zoom`]: the steps of the font size (`F_SETFONTSIZEUP` / `F_SETFONTSIZEDOWN`).
//!
//! # Items that Sakura does not have
//!
//! They go in the section `[UME]`, which Sakura does not use, so that Sakura can still open the
//! file (ADR-0010). Now: `nMacKeyLayout` (0 standard, 1 all Control, 2 Cmd as Ctrl; macOS only).
//!
//! # Where the file is
//!
//! `ume_config::location` says where to look on each OS; the final place is still open (ADR-0010,
//! #31). A file that is not there is not an error: every value is Sakura's default.
//!
//! # Writing it back
//!
//! The window writes only the section `[MRU]` (the recent files and folders, `crate::history`),
//! at once and safely for several windows; the rest of the file stays as it was.
//!
//! The window does not write the other sections. Sakura writes it when the last window closes, but the values
//! this PR changes at run time (the font size by Ctrl+wheel, the wrap method, the status bar of
//! this window) are Sakura's *temporary* settings: `F_SETFONTSIZEUP` and `F_SETFONTSIZEDOWN` run
//! `Command_SETFONTSIZE` in mode 2 ("一時適用フォント"), the `F_TMPWRAP*` functions say
//! "(一時設定)", and `Command_WRAPWINDOWWIDTH` does not touch the common settings
//! (`sakura_core/cmd/CViewCommander_Settings.cpp`). Only `F_SHOWSTATUSBAR` changes a common
//! setting (`bDispSTATUSBAR`); it is kept for the session, and written together with the other
//! settings when the settings dialogs come (they need the lossless writer of `ume-config` and
//! an answer to "which window's state is the file's").

pub mod doc_type;
pub mod mac_key_layout;
pub mod new_document;
pub mod view_settings;
pub mod zoom;

use std::path::{Path, PathBuf};

use ume_commands::keymap::user::parse_entries;
use ume_commands::keymap::{KeyMap, Platform, UserRow};
use ume_config::Profile;
use ume_config::location::{Environment, Os, candidate_paths, find_existing, write_path};

pub use mac_key_layout::MacKeyLayout;
pub use new_document::NewDocumentDefaults;
pub use view_settings::{Colors, Marks, ViewSettings, WrapMethod};

/// The settings file, read once at startup.
#[derive(Clone, Debug, Default)]
pub struct UserSettings {
    profile: Option<Profile>,
    path: Option<PathBuf>,
    /// Things in the file that could not be used, for the status bar.
    pub warnings: Vec<String>,
}

impl gpui::Global for UserSettings {}

/// The defaults of a new document for a file called `file_name`, from the settings file the
/// application read at startup (Sakura's, with the line ending of the OS, without one).
pub fn new_document_defaults(cx: &gpui::App, file_name: Option<&str>) -> NewDocumentDefaults {
    cx.try_global::<UserSettings>()
        .map(|user| user.new_document_defaults(file_name))
        .unwrap_or_default()
}

impl UserSettings {
    /// No file: Sakura's defaults.
    pub fn none() -> Self {
        Self::default()
    }

    /// Reads the file from where `ume-config` looks for it on this OS.
    pub fn load() -> Self {
        Self::load_from(&candidate_paths(
            Os::current(),
            &Environment::from_process(),
        ))
    }

    /// Reads the first of `candidates` that is a file.
    pub fn load_from(candidates: &[PathBuf]) -> Self {
        let Some(path) = find_existing(candidates) else {
            return Self::none();
        };
        match Profile::load(path) {
            Ok(profile) => Self::from_profile(profile, Some(path.to_owned())),
            Err(e) => Self {
                warnings: vec![format!("{}: {e}", path.display())],
                path: Some(path.to_owned()),
                ..Self::none()
            },
        }
    }

    /// Settings from a profile that is already read.
    pub fn from_profile(profile: Profile, path: Option<PathBuf>) -> Self {
        let (_, problems) = parse_entries(profile.entries("KeyBind"));
        let warnings = problems.iter().map(ToString::to_string).collect();
        Self {
            profile: Some(profile),
            path,
            warnings,
        }
    }

    /// The file this was read from.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The file as read, for the settings the window itself looks at.
    pub fn profile(&self) -> Option<&Profile> {
        self.profile.as_ref()
    }

    /// The settings of the view for a file called `file_name` (no folder), or for a new document.
    pub fn view_settings(&self, file_name: Option<&str>) -> ViewSettings {
        match &self.profile {
            Some(profile) => ViewSettings::from_profile(profile, file_name, Os::current()),
            None => ViewSettings::default(),
        }
    }

    /// The key map for `platform`: Sakura's table read the way `[UME]` `nMacKeyLayout` says
    /// (macOS only), with the user's `[KeyBind]` rows over it.
    pub fn key_map(&self, platform: Platform) -> KeyMap {
        let layout = self
            .profile
            .as_ref()
            .map_or_else(MacKeyLayout::default, MacKeyLayout::from_profile);
        let mut map = KeyMap::new(layout.options(platform));
        map.apply_user_rows(&self.key_rows());
        map
    }

    /// The encoding, BOM and line ending of a new document, or of a file that has no line ending,
    /// for a file called `file_name` (`None` for an untitled document). See [`new_document`].
    pub fn new_document_defaults(&self, file_name: Option<&str>) -> NewDocumentDefaults {
        match &self.profile {
            Some(profile) => NewDocumentDefaults::from_profile(profile, file_name),
            None => NewDocumentDefaults::default(),
        }
    }

    /// The user's key bindings (`[KeyBind]`), to put over Sakura's table. Empty without a file.
    pub fn key_rows(&self) -> Vec<UserRow> {
        match &self.profile {
            Some(profile) => parse_entries(profile.entries("KeyBind")).0,
            None => Vec::new(),
        }
    }
}

/// The settings file the history is written to: the file that was read, or else the place where
/// `ume-config` writes a new one. `None` if the OS has no such place.
pub fn history_file(user: &UserSettings) -> Option<PathBuf> {
    user.path().map(Path::to_owned).or_else(|| {
        write_path(&candidate_paths(
            Os::current(),
            &Environment::from_process(),
        ))
        .map(Path::to_owned)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bom(text: &str) -> Vec<u8> {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        bytes
    }

    #[test]
    fn no_file_means_the_defaults() {
        let s = UserSettings::load_from(&[PathBuf::from("/no/such/dir/ume.ini")]);
        assert!(s.path().is_none());
        assert!(s.warnings.is_empty());
        assert_eq!(s.view_settings(Some("a.txt")), ViewSettings::default());
        assert!(s.key_rows().is_empty());
    }

    #[test]
    fn the_first_existing_candidate_is_read() {
        let dir = std::env::temp_dir().join(format!("ume-ui-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ume.ini");
        std::fs::write(
            &path,
            bom("[Types(0)]\r\nnInts=0,10240,0,2,-1,-1,0,1,1,0,0,0\r\n"),
        )
        .unwrap();
        let s = UserSettings::load_from(&[dir.join("missing.ini"), path.clone()]);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(s.path(), Some(path.as_path()));
        assert_eq!(s.view_settings(None).tab_width, 2);
    }

    #[test]
    fn key_bindings_come_from_the_key_bind_section() {
        // `KeyBind[NNN]=KKKK,F0..F7,Name`: the key code in hex (0x41 is A), then the function of
        // each modifier combination; Ctrl+A (index 2) is 30020 (F_SELECTALL) in Sakura's table,
        // here it is F_UNDO (30210).
        let text = "[KeyBind]\r\nKeyBind[000]=0041,0,0,30210,0,0,0,0,0,A\r\nKeyBind[001]=zz,0\r\n";
        let s = UserSettings::from_profile(Profile::from_bytes(&bom(text)), None);
        let rows = s.key_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].functions[2], 30210);
        // The broken row is reported, not fatal.
        assert_eq!(s.warnings.len(), 1);
        assert!(s.warnings[0].contains("KeyBind[001]"));
    }
}
