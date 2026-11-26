//! Where the profile manager keeps things on the disk: `matsu_prof.ini` and the folders of the
//! profiles (`matsu_config::profile_manager`).
//!
//! The store is the one owner of what is on the disk. The dialog does not keep a list that it writes
//! back: every action reads the file, changes the one thing, writes, and shows the result (the dialog
//! can be open in several processes, and the picker of the start-up is a process of its own).

use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use matsu_platform::file_lock::{FileLock, ShareMode};

use matsu_config::location::{Environment, Os};
use matsu_config::profile_manager::{self, ProfileSettings};

/// Why a profile folder cannot be renamed (checked before anything is written).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MoveError {
    /// A folder of the new name is there already: `MoveFile` does not replace it
    /// (`STR_DLGPROFILE_ERR_RENAME`).
    Exists,
    /// The old name was a file, not a folder, and the new name is a file too
    /// (`STR_DLGPROFILE_ERR_FILE`).
    FileInTheWay,
}

/// A folder to move, decided by [`Store::plan_rename`].
#[derive(Debug)]
pub(crate) struct Move {
    from: PathBuf,
    to: PathBuf,
    /// The two names are the same folder on a file system that does not tell cases apart
    /// (`work` and `Work`): the move goes through a name of its own.
    same_folder: bool,
}

/// Where the folder of a profile is, by the name of the profile.
type DirOf = Box<dyn Fn(&str) -> Option<PathBuf>>;

/// The manager's file and the places of the profiles' folders.
pub(crate) struct Store {
    ini: PathBuf,
    /// The folder of the settings of a profile (`matsu_config::profile_manager::profile_dir`).
    dir_of: DirOf,
    /// How long an action waits for the lock of the file.
    wait: Duration,
}

/// The lock of `matsu_prof.ini` for one action: let go when dropped.
#[derive(Debug)]
pub(crate) struct ListLock(#[allow(dead_code)] FileLock);

/// The usual wait for the lock: an action holds it for a read, a write and a folder move.
const LOCK_WAIT: Duration = Duration::from_secs(5);

impl Store {
    /// The places of this process: the file next to the settings file it would use without `-PROF`,
    /// and the folder of a profile where a process with that `-PROF` keeps its settings file.
    /// `None` when the OS has no place for the settings file.
    pub(crate) fn for_process() -> Option<Self> {
        let (os, env) = (Os::current(), Environment::from_process());
        let ini = profile_manager::settings_path(os, &env)?;
        Some(Self {
            ini,
            dir_of: Box::new(move |name| profile_manager::profile_dir(os, &env, name)),
            wait: LOCK_WAIT,
        })
    }

    /// A store in one folder: the file and the profiles' folders are in it.
    #[cfg(test)]
    pub(crate) fn in_folder(base: &Path) -> Self {
        let base = base.to_path_buf();
        Self {
            ini: base.join(profile_manager::FILE_NAME),
            dir_of: Box::new(move |name| Some(base.join(name))),
            wait: LOCK_WAIT,
        }
    }

    /// A store whose profiles are looked up by `dir_of` (several roots).
    #[cfg(test)]
    pub(crate) fn with_dirs(
        base: &Path,
        dir_of: impl Fn(&str) -> Option<PathBuf> + 'static,
    ) -> Self {
        Self {
            ini: base.join(profile_manager::FILE_NAME),
            dir_of: Box::new(dir_of),
            wait: LOCK_WAIT,
        }
    }

    /// The same, with another wait for the lock.
    #[cfg(test)]
    pub(crate) fn waiting(mut self, wait: Duration) -> Self {
        self.wait = wait;
        self
    }

    /// Locks `matsu_prof.ini.lock` next to the file against every other action of every window and
    /// process, for the whole of a read-change-write. Waits for the holder (a short time, the others
    /// hold it for one action); an error when it does not come free.
    pub(crate) fn lock(&self) -> io::Result<ListLock> {
        let mut path = self.ini.clone().into_os_string();
        path.push(".lock");
        let path = PathBuf::from(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let start = Instant::now();
        loop {
            // Both steps are in the loop: on Windows the holder shares nothing, so the open that makes
            // sure the file is there is refused too while another process holds it. The handle that makes
            // the file is closed before the lock opens it (a closure parameter `_` would keep it until the
            // closure ends, and Windows refuses the lock then).
            let locked = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .and_then(|file| {
                    drop(file);
                    FileLock::acquire(&path, ShareMode::DenyReadWrite)
                });
            match locked {
                Ok(lock) => return Ok(ListLock(lock)),
                Err(error) if start.elapsed() >= self.wait => return Err(error),
                Err(_) => std::thread::sleep(Duration::from_millis(25)),
            }
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
    pub(crate) fn dir(&self, name: &str) -> Option<PathBuf> {
        (self.dir_of)(name)
    }

    /// Whether a file (not a folder) has the name of the profile, so that it cannot be made
    /// (`CreateProf`: `IsFileExists(dir, true)`).
    pub(crate) fn file_in_the_way(&self, name: &str) -> bool {
        self.dir(name).is_some_and(|dir| is_file(&dir))
    }

    /// `RenameProf`, before anything is written: what has to be moved, if anything. A profile that has
    /// no folder (never used) is only renamed in the list.
    pub(crate) fn plan_rename(&self, old: &str, new: &str) -> Result<Option<Move>, MoveError> {
        // The folder stays where it is: the new name goes beside the old one. Looking the new name up like a
        // profile that was never used would give another root (on Windows with a portable `matsu.ini`,
        // the user's own folder), and the move would cross it or fail.
        let Some(from) = self.dir(old) else {
            return Ok(None);
        };
        let Some(to) = from.parent().map(|parent| parent.join(new)) else {
            return Ok(None);
        };
        if !from.exists() {
            return Ok(None);
        }
        if from.is_dir() {
            let same_folder = same_folder(&from, &to);
            if to.exists() && !same_folder {
                return Err(MoveError::Exists);
            }
            return Ok(Some(Move {
                from,
                to,
                same_folder,
            }));
        }
        // the old name was a file: the profile is made new, which needs a name that is free
        if is_file(&to) {
            return Err(MoveError::FileInTheWay);
        }
        Ok(None)
    }

    /// Moves the folder. The error is the real one of the OS. A case-only rename goes through a
    /// temporary name, and is undone if its second step fails.
    pub(crate) fn move_dir(&self, plan: &Move) -> io::Result<()> {
        if !plan.same_folder {
            return std::fs::rename(&plan.from, &plan.to);
        }
        let mut temporary = plan.from.clone().into_os_string();
        temporary.push(format!(".matsu-rename-{}", std::process::id()));
        let temporary = PathBuf::from(temporary);
        std::fs::rename(&plan.from, &temporary)?;
        if let Err(error) = std::fs::rename(&temporary, &plan.to) {
            let _ = std::fs::rename(&temporary, &plan.from);
            return Err(error);
        }
        Ok(())
    }
}

fn is_file(path: &Path) -> bool {
    path.exists() && !path.is_dir()
}

/// Whether the two paths are one folder (`work` and `Work` on a file system that ignores case).
fn same_folder(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}
