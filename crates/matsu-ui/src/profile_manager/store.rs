//! Where the profile manager keeps things on the disk: `matsu_prof.ini` and the folders of the
//! profiles (`matsu_config::profile_manager`).
//!
//! The dialog does not know paths; it asks the store, and the tests give it a folder of their own.

use std::io;
use std::path::{Path, PathBuf};

use matsu_config::location::{Environment, Os};
use matsu_config::profile_manager::{self, ProfileSettings};

/// Why a profile folder could not be renamed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MoveError {
    /// The folder cannot be moved: the new name is taken, or the profile is in use
    /// (`STR_DLGPROFILE_ERR_RENAME`).
    Failed,
    /// The old name was a file, not a folder, and the new name is a file too
    /// (`STR_DLGPROFILE_ERR_FILE`).
    FileInTheWay,
}

/// The manager's file and the folder the profiles' folders are in.
#[derive(Debug, Clone)]
pub(crate) struct Store {
    ini: PathBuf,
    base: PathBuf,
}

impl Store {
    /// The places of this process: next to the settings file it would use without `-PROF`.
    /// `None` when the OS has no place for the settings file.
    pub(crate) fn for_process() -> Option<Self> {
        let (os, env) = (Os::current(), Environment::from_process());
        Some(Self {
            ini: profile_manager::settings_path(os, &env)?,
            base: profile_manager::base_dir(os, &env)?,
        })
    }

    /// A store in one folder: the file and the profiles' folders are in it.
    #[cfg(test)]
    pub(crate) fn in_folder(base: &Path) -> Self {
        Self {
            ini: base.join(profile_manager::FILE_NAME),
            base: base.to_path_buf(),
        }
    }

    /// `ReadProfSettings`: `None` when the file is not there.
    pub(crate) fn read(&self) -> Option<ProfileSettings> {
        profile_manager::read(&self.ini)
    }

    /// `WriteProfSettings`.
    pub(crate) fn write(&self, settings: &ProfileSettings) -> io::Result<()> {
        profile_manager::write(&self.ini, settings)
    }

    /// `GetProfileDirectory`: where the settings of the profile are.
    fn dir(&self, name: &str) -> PathBuf {
        self.base.join(name)
    }

    /// Whether a file (not a folder) has the name of the profile, so that it cannot be made
    /// (`CreateProf`: `IsFileExists(dir, true)`).
    pub(crate) fn file_in_the_way(&self, name: &str) -> bool {
        is_file(&self.dir(name))
    }

    /// `RenameProf`: moves the folder of the profile. A profile that has no folder (never used) is
    /// only renamed in the list. Sakura's `MoveFile` does not replace a folder that is there.
    pub(crate) fn rename_dir(&self, old: &str, new: &str) -> Result<(), MoveError> {
        let (from, to) = (self.dir(old), self.dir(new));
        if !from.exists() {
            return Ok(());
        }
        if from.is_dir() {
            if to.exists() {
                return Err(MoveError::Failed);
            }
            return std::fs::rename(&from, &to).map_err(|_| MoveError::Failed);
        }
        // the old name was a file: the profile is made new, which needs a name that is free
        if is_file(&to) {
            return Err(MoveError::FileInTheWay);
        }
        Ok(())
    }
}

fn is_file(path: &Path) -> bool {
    path.exists() && !path.is_dir()
}
