//! MATSU Editor's settings file (`matsu.ini`, in the format of Sakura's `sakura.ini`) in the window (ADR-0010).
//!
//! [`UserSettings`] is the file read through `matsu-config`; [`ViewSettings`] is what the editor
//! view takes from it for one file. Both are plain data, so they are tested without a window.
//!
//! * [`view_settings`]: font, tab width, wrap, what is shown, colors: the meaning of Sakura's keys.
//! * [`mac_key_layout`]: `[MATSU]` `nMacKeyLayout`, how the key table is read on macOS.
//! * [`doc_type`]: which document type a file name has.
//! * [`zoom`]: the steps of the font size (`F_SETFONTSIZEUP` / `F_SETFONTSIZEDOWN`).
//!
//! # Items that Sakura does not have
//!
//! They go in the section `[MATSU]`, which Sakura does not use, so that Sakura can still open the
//! file (ADR-0010). Now: `nMacKeyLayout` (0 standard, 1 all Control, 2 Cmd as Ctrl; macOS only).
//!
//! # Where the file is
//!
//! `matsu_config::location` says where to look on each OS; the final place is still open (ADR-0010,
//! #31). A file that is not there is not an error: every value is Sakura's default.
//!
//! # Writing it back
//!
//! The window writes only the section `[MRU]` (the recent files and folders, `crate::history`) and
//! the tab keys of `[Common]` (`bDispTabWnd`, `bDispTabWndMultiWin`, `crate::tabs::settings`), at
//! once and safely for several windows; the rest of the file stays as it was.
//!
//! The window does not write the other sections. Sakura writes it when the last window closes, but the values
//! this PR changes at run time (the font size by Ctrl+wheel, the wrap method, the status bar of
//! this window) are Sakura's *temporary* settings: `F_SETFONTSIZEUP` and `F_SETFONTSIZEDOWN` run
//! `Command_SETFONTSIZE` in mode 2 ("一時適用フォント"), the `F_TMPWRAP*` functions say
//! "(一時設定)", and `Command_WRAPWINDOWWIDTH` does not touch the common settings
//! (`sakura_core/cmd/CViewCommander_Settings.cpp`). Only `F_SHOWSTATUSBAR` changes a common
//! setting (`bDispSTATUSBAR`); it is kept for the session, and written together with the other
//! settings when the settings dialogs come (they need the lossless writer of `matsu-config` and
//! an answer to "which window's state is the file's").

pub mod doc_type;
pub mod mac_key_layout;
pub mod new_document;
pub mod view_settings;
pub mod zoom;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use matsu_commands::keymap::user::parse_entries;
use matsu_commands::keymap::{KeyMap, Platform, UserRow};
use matsu_config::Profile;
use matsu_config::location::{Environment, Os, candidate_paths, find_existing, write_path};
use matsu_config::settings::DocumentTypes;

pub use mac_key_layout::MacKeyLayout;
pub use new_document::NewDocumentDefaults;
pub use view_settings::{Colors, DiffColor, Marks, ViewSettings, WrapMethod};

/// The settings file, read once at startup.
#[derive(Clone, Debug, Default)]
pub struct UserSettings {
    profile: Option<Profile>,
    /// The document types: Sakura's built-in ones with what the file overrides and adds.
    types: Arc<DocumentTypes>,
    path: Option<PathBuf>,
    /// Things in the file that could not be used, for the status bar.
    pub warnings: Vec<String>,
}

impl gpui::Global for UserSettings {}

/// The defaults of a new document for a file called `file_name`, from the settings file the
/// application read at startup (Sakura's, with the line ending of the OS, without one).
pub fn new_document_defaults(cx: &gpui::App, file_name: Option<&str>) -> NewDocumentDefaults {
    let mut defaults = cx
        .try_global::<UserSettings>()
        .map(|user| user.new_document_defaults(file_name))
        .unwrap_or_default();
    // 「MIMEエンコードされたヘッダをデコードする」 is a common setting, not a type's
    defaults.mime_decode = crate::file_ops::prefs::prefs(cx).auto_mime_decode;
    defaults
}

/// The message of the migration from the old settings, with the time it was made.
///
/// Kept for the thread that starts the application, which is the one that draws the windows; so the tests,
/// each on a thread of its own, do not see the message of another.
mod startup_slot {
    use std::cell::RefCell;
    use std::time::Instant;

    thread_local! {
        pub(super) static SLOT: RefCell<Option<(String, Instant)>> = const { RefCell::new(None) };
    }
}

/// How long the message of the migration stays in the status bar.
const STARTUP_NOTICE_TIME: std::time::Duration = std::time::Duration::from_secs(20);

/// Copies the settings of the old name to the places of `matsu.ini` when they are missing there
/// (`matsu_config::migrate_old_name`; ADR-0021). Call it once at the start, after `-PROF=`, before the
/// settings file is read. TEMPORARY: delete at version 1.0.
pub(crate) fn migrate_old_settings() {
    let outcome =
        matsu_config::migrate_old_name::migrate(Os::current(), &Environment::from_process());
    if let Some(notice) = outcome.notice() {
        eprintln!("matsu: {notice}");
        set_startup_notice(&notice);
    }
}

/// Keeps the message of the migration for [`startup_notice`].
pub(crate) fn set_startup_notice(text: &str) {
    startup_slot::SLOT.with(|slot| {
        *slot.borrow_mut() = Some((text.to_owned(), std::time::Instant::now()));
    });
}

/// The message of the migration, for a while after the start: the status bar of every window shows it where
/// the notice goes, while there is no other notice. (It is not kept in a window: the windows of a first start
/// load a file or reset their notice, and a window can be replaced.)
pub(crate) fn startup_notice() -> Option<String> {
    startup_slot::SLOT.with(|slot| {
        slot.borrow()
            .as_ref()
            .filter(|(_, since)| since.elapsed() < STARTUP_NOTICE_TIME)
            .map(|(text, _)| text.clone())
    })
}

impl UserSettings {
    /// No file: Sakura's defaults.
    pub fn none() -> Self {
        Self::default()
    }

    /// Reads the file from where `matsu-config` looks for it on this OS.
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
            types: Arc::new(DocumentTypes::from_profile(&profile)),
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

    /// Where the file dialogs start (「ファイルダイアログの初期位置」, `eOpenDialogDir`).
    pub fn dialog_dir(&self) -> matsu_config::dialog_dir::DialogDirSetting {
        self.profile
            .as_ref()
            .map(matsu_config::dialog_dir::DialogDirSetting::from_profile)
            .unwrap_or_default()
    }

    /// `[Folders]` `szMACROFOLDER`, the folder of the macros (マクロ用フォルダー), as written. `None`
    /// when the file does not have it.
    pub fn macro_folder(&self) -> Option<String> {
        matsu_config::settings::macros::macro_folder(self.profile.as_ref()?)
    }

    /// The registered macros (登録マクロ, `[Macro]`) and the three automatic ones.
    pub fn registered_macros(&self) -> matsu_config::settings::macros::RegisteredMacros {
        self.profile
            .as_ref()
            .map(matsu_config::settings::macros::RegisteredMacros::from_profile)
            .unwrap_or_default()
    }

    /// The document types (タイプ別設定).
    pub fn document_types(&self) -> &DocumentTypes {
        &self.types
    }

    /// The document types, to keep beyond the borrow of the settings.
    pub fn shared_types(&self) -> Arc<DocumentTypes> {
        Arc::clone(&self.types)
    }

    /// The index of the document type of a file called `file_name` (no folder), or of a new
    /// document (the basic type).
    pub fn type_index_for(&self, file_name: Option<&str>) -> usize {
        file_name.map_or(0, |name| self.types.type_index_for_file_name(name))
    }

    /// The settings of the view for a file called `file_name` (no folder), or for a new document.
    pub fn view_settings(&self, file_name: Option<&str>) -> ViewSettings {
        self.view_settings_of_type(self.type_index_for(file_name))
    }

    /// The settings of the view for document type `index`.
    pub fn view_settings_of_type(&self, index: usize) -> ViewSettings {
        let empty;
        let profile = match &self.profile {
            Some(profile) => profile,
            None => {
                empty = Profile::new();
                &empty
            }
        };
        ViewSettings::from_types(&self.types, index, profile, Os::current())
    }

    /// The key map for `platform`: Sakura's table read the way `[MATSU]` `nMacKeyLayout` says
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
            Some(profile) => NewDocumentDefaults::from_types(profile, &self.types, file_name),
            None => NewDocumentDefaults::default(),
        }
    }

    /// The encoding, BOM and line ending a document of type `index` gets.
    pub fn new_document_defaults_of_type(&self, index: usize) -> NewDocumentDefaults {
        match &self.profile {
            Some(profile) => NewDocumentDefaults::for_type(profile, index),
            None => NewDocumentDefaults::default(),
        }
    }

    /// The custom menus and the right-click menu (`[CustMenu]`): Sakura's defaults under what the file sets.
    pub fn custom_menus(&self) -> matsu_config::settings::CustomMenus {
        match &self.profile {
            Some(profile) => matsu_config::settings::CustomMenus::from_profile(profile, &|name| {
                matsu_commands::macro_names::by_name(name).map(|f| f.code)
            }),
            None => matsu_config::settings::CustomMenus::default(),
        }
    }

    /// The main menu (`[MainMenu]`): Sakura's default (`MainMenu.ini`) when the file has no menu, with the
    /// items that newer versions added to a menu an older version wrote.
    pub fn main_menu(&self) -> matsu_config::settings::MainMenu {
        match &self.profile {
            Some(profile) => matsu_config::settings::MainMenu::from_profile(
                profile,
                &|name| matsu_commands::macro_names::by_name(name).map(|f| f.code),
                crate::menu::default_main_menu(),
            ),
            None => crate::menu::default_main_menu().clone(),
        }
    }

    /// 「アクセスキーを必ず( )付で表示」 (`bKeyParentheses`): the setting `MakeMenuLabel` reads for every
    /// menu Sakura builds with access keys, not for the main menu only.
    pub fn key_parentheses(&self) -> bool {
        self.main_menu().key_parentheses
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
/// `matsu-config` writes a new one. `None` if the OS has no such place.
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
        let s = UserSettings::load_from(&[PathBuf::from("/no/such/dir/matsu.ini")]);
        assert!(s.path().is_none());
        assert!(s.warnings.is_empty());
        // A file of an unknown type, and a new document, are the basic type.
        assert_eq!(s.view_settings(Some("a.xyz")), ViewSettings::default());
        assert_eq!(s.view_settings(None), ViewSettings::default());
        // The built-in types are there without a file: .txt is テキスト.
        assert_eq!(s.view_settings(Some("a.txt")).type_index, 1);
        // Sakura's 17, MATSU Editor's own 10 (ADR-0017), and 3 copies of the basic type.
        assert_eq!(s.document_types().len(), 30);
        assert!(s.key_rows().is_empty());
    }

    #[test]
    fn the_first_existing_candidate_is_read() {
        let dir = std::env::temp_dir().join(format!("matsu-ui-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("matsu.ini");
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
