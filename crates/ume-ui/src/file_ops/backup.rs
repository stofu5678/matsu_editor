//! The backup before a save (`CBackupAgent`, `sakura_core/agent/CBackupAgent.cpp`).
//!
//! Before the file is written, the file that is there is copied to a backup, if 「保存時に
//! バックアップを作成する」 is on (`OnPreBeforeSave`). The name of the backup, the folder, the
//! generations and the questions are Sakura's:
//!
//! * [`backup_path`] is `FormatBackUpPath`: the name by the kind (`nBackUpType`: 1 `.bak`, 2 the
//!   date and time, 3 the generations `.b00`, 4 the date and time of the last save, 5 and 6 as 1 and
//!   3 but the extension stays), or by the detailed format (`szBackUpPathAdvanced`).
//! * [`make_backup`] is the rest of `MakeBackUp`: moves the older generations, copies the file,
//!   and moves the copy to the recycle bin if asked.
//! * [`before_save`] asks the questions (`OnPreBeforeSave`, `MakeBackUp`) and calls them.
//!
//! The functions of the first two are plain, so that the tests give them the time.
//!
//! Differences from Sakura: the file name is cut at its last dot only if the dot is not the first
//! character (`.bashrc` has no extension; `_wsplitpath_s` would make it the extension); the
//! characters `*` and `?` of the detailed format are replaced in the part that the format made, not
//! in the folder before it; on macOS and Linux the backslash of the settings is a `/`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use chrono::{Datelike, Local, NaiveDateTime, Timelike};
use gpui::{App, AsyncWindowContext, Global, PromptLevel};

use super::ask::tell;

/// `BKUP_YEAR` .. `BKUP_SEC` (`EBackupOptionFlag`), the bits of `bBackUpType2_Opt1`.
pub(crate) const YEAR: i32 = 32;
pub(crate) const MONTH: i32 = 16;
pub(crate) const DAY: i32 = 8;
pub(crate) const HOUR: i32 = 4;
pub(crate) const MIN: i32 = 2;
pub(crate) const SEC: i32 = 1;

/// `_MAX_PATH`: Windows paths of Sakura's buffers (`SFilePath`) are not longer.
const MAX_PATH: usize = 260;

/// The settings of the page バックアップ (`CommonSetting_Backup`). The default is Sakura's
/// (`CShareData::InitShareData`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BackupPrefs {
    /// `bBackUp`: 保存時にバックアップを作成する.
    pub enabled: bool,
    /// `bBackUpDialog`: 作成前に確認する.
    pub confirm: bool,
    /// `bBackUpFolder`: 指定フォルダーに作成する.
    pub use_folder: bool,
    /// `bBackUpFolderRM`: only for files on removable media.
    pub folder_removable_only: bool,
    /// `szBackUpFolder`, with its meta folders (`%MYDOC%`) not expanded.
    pub folder: String,
    /// `nBackUpType`.
    pub kind: i32,
    /// `bBackUpType2_Opt1`: the bits of the date and time.
    pub date_bits: i32,
    /// The generations (`GetBackupCount`, the low 16 bits of `bBackUpType2_Opt2`).
    pub generations: i32,
    /// The first letter of the extension of the generations (`GetBackupExtChar`).
    pub ext_char: char,
    /// `bBackUpType2_Opt4` (`GetBackupTypeAdv`): 2 the time of the save, 4 the time of the file.
    pub advanced_time: i32,
    /// `bBackUpDustBox`: ごみ箱に放り込む.
    pub dust_box: bool,
    /// `bBackUpPathAdvanced`: 詳細設定する.
    pub advanced: bool,
    /// `szBackUpPathAdvanced`.
    pub advanced_format: String,
}

impl Default for BackupPrefs {
    fn default() -> Self {
        Self {
            enabled: false,
            confirm: true,
            use_folder: false,
            folder_removable_only: false,
            folder: String::new(),
            kind: 2,
            date_bits: YEAR | MONTH | DAY,
            generations: 10,
            ext_char: 'b',
            advanced_time: 0,
            dust_box: false,
            advanced: false,
            advanced_format: String::new(),
        }
    }
}

impl Global for BackupPrefs {}

impl BackupPrefs {
    /// `GetBackupOpt`.
    pub fn has_date_part(&self, flag: i32) -> bool {
        self.date_bits & flag == flag
    }
}

/// The settings now.
pub(crate) fn prefs(cx: &App) -> BackupPrefs {
    cx.try_global::<BackupPrefs>().cloned().unwrap_or_default()
}

/// The times that a name can have: now, and the last write time of the file.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Times {
    pub now: NaiveDateTime,
    pub last_write: NaiveDateTime,
}

/// Cuts a file name into the name and the extension with its dot (`_wsplitpath_s`).
fn split_name(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 => name.split_at(i),
        _ => (name, ""),
    }
}

/// `GetDateTimeFormat` (`sakura_core/util/format.cpp`): `%Y` the year, `%y` its last two digits,
/// `%m` `%d` `%H` `%M` `%S`; `%x` for another letter is the letter; `%%` is `%`.
pub(crate) fn format_date_time(format: &str, time: &NaiveDateTime) -> String {
    let mut out = String::new();
    let mut in_specifier = false;
    for c in format.chars() {
        if in_specifier {
            in_specifier = false;
            match c {
                'Y' => out.push_str(&time.year().to_string()),
                'y' => out.push_str(&format!("{:02}", time.year().rem_euclid(100))),
                'm' => out.push_str(&format!("{:02}", time.month())),
                'd' => out.push_str(&format!("{:02}", time.day())),
                'H' => out.push_str(&format!("{:02}", time.hour())),
                'M' => out.push_str(&format!("{:02}", time.minute())),
                'S' => out.push_str(&format!("{:02}", time.second())),
                other => out.push(other),
            }
        } else if c == '%' {
            in_specifier = true;
        } else {
            out.push(c);
        }
    }
    out
}

/// The folder names above the file for `$1` .. `$9` of the detailed format; `$0` is the name
/// without the extension (the array `folders` of `FormatBackUpPath`).
fn folder_names(target: &Path, stem: &str) -> Vec<String> {
    let mut names = vec![stem.to_owned()];
    if let Some(parent) = target.parent() {
        let mut up: Vec<String> = parent
            .components()
            .filter_map(|c| match c {
                std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();
        up.reverse();
        names.extend(up.into_iter().take(9));
    }
    names
}

/// Replaces `$0` .. `$9` of the detailed format. A `$` with another character after it, or at the
/// end, stays; a number above the folders there are is nothing.
fn expand_folder_keys(format: &str, folders: &[String]) -> String {
    let mut out = String::new();
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$'
            && let Some(digit) = chars.peek().and_then(|d| d.to_digit(10))
        {
            chars.next();
            if let Some(name) = folders.get(digit as usize) {
                out.push_str(name);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// The folder of a path as a string that ends with a separator.
fn with_separator(dir: &Path) -> String {
    let mut text = dir.to_string_lossy().into_owned();
    if !text.is_empty() && !text.ends_with(['/', '\\']) {
        text.push(std::path::MAIN_SEPARATOR);
    }
    text
}

/// Sakura's settings hold backslashes; on the other OSes they are separators too.
fn native(text: &str) -> String {
    if cfg!(windows) {
        text.to_owned()
    } else {
        text.replace('\\', "/")
    }
}

/// `FormatBackUpPath`: where the backup of `target` goes, or `None` when the path cannot be made
/// (too long on Windows).
///
/// `folder` is the folder of 「指定フォルダー」 with its meta folders expanded, when it is to be
/// used (`bBackUpFolder`, and the removable-media check, are the caller's); else the backup is
/// next to the file.
pub(crate) fn backup_path(
    prefs: &BackupPrefs,
    target: &Path,
    folder: Option<&Path>,
    times: Times,
) -> Option<PathBuf> {
    let file_name = target.file_name()?.to_string_lossy().into_owned();
    let (stem, ext) = split_name(&file_name);
    let dir = match folder {
        Some(folder) => std::path::absolute(folder).unwrap_or_else(|_| folder.to_owned()),
        None => target.parent().map(Path::to_owned).unwrap_or_default(),
    };

    let path = if !prefs.advanced {
        let name = match prefs.kind {
            1 => format!("{stem}.bak"),
            5 => format!("{stem}{ext}.bak"),
            2 | 4 => {
                let time = if prefs.kind == 2 {
                    times.now
                } else {
                    times.last_write
                };
                let mut format = String::new();
                for (flag, spec) in [
                    (YEAR, "%Y"),
                    (MONTH, "%m"),
                    (DAY, "%d"),
                    (HOUR, "%H"),
                    (MIN, "%M"),
                    (SEC, "%S"),
                ] {
                    if prefs.has_date_part(flag) {
                        format.push_str(spec);
                    }
                }
                format!("{stem}_{}{ext}", format_date_time(&format, &time))
            }
            3 | 6 => {
                // The number is the last two characters (`MakeBackUp` counts on it).
                let kept = if prefs.kind == 3 { "" } else { ext };
                format!("{stem}{kept}.{}00", prefs.ext_char)
            }
            // Sakura adds nothing: the "backup" would be the folder itself.
            _ => String::new(),
        };
        dir.join(name)
    } else {
        let time = if prefs.advanced_time == 4 {
            times.last_write
        } else {
            times.now
        };
        let formatted = format_date_time(&native(&prefs.advanced_format), &time);
        let formatted = expand_folder_keys(&formatted, &folder_names(target, stem));
        let extension = ext.strip_prefix('.').unwrap_or(ext);
        let formatted = formatted.replace('*', extension).replace('?', "_");
        PathBuf::from(format!("{}{formatted}", with_separator(&dir)))
    };
    // `SFilePath` holds `_MAX_PATH` characters.
    if cfg!(windows) && path.to_string_lossy().chars().count() >= MAX_PATH {
        return None;
    }
    Some(path)
}

/// Why a backup was not made, for the questions of [`before_save`].
#[derive(Debug)]
pub(crate) enum Failure {
    /// The oldest generation could not be deleted (`STR_BACKUP_ERR_DELETE`). The save goes on.
    Delete(PathBuf),
    /// A generation could not be moved (`STR_BACKUP_ERR_MOVE`). The save goes on.
    Move(PathBuf),
    /// The copy failed (`MakeBackUp` returns 3): ask whether to save anyway.
    Copy(io::Error),
}

/// `dest` with its last two characters replaced by `number`.
fn numbered(dest: &Path, number: usize) -> PathBuf {
    let text = dest.to_string_lossy();
    let cut = text.char_indices().rev().nth(1).map_or(0, |(i, _)| i);
    PathBuf::from(format!("{}{number:02}", &text[..cut]))
}

/// The rest of `MakeBackUp` after the questions: the generations, the copy, the recycle bin.
///
/// `to_trash`: the copy is moved to the recycle bin (`bBackUpDustBox` on a local drive).
pub(crate) fn make_backup(
    prefs: &BackupPrefs,
    target: &Path,
    dest: &Path,
    to_trash: bool,
) -> Result<(), Failure> {
    // The file must not be one of the files that the backup deletes, moves or overwrites: the name
    // itself (`a.bak` saved with the kind 1), or a generation (`a.b01` saved with the kind 3, which
    // the rotation would move away). The files are compared as the OS sees them, so a hard link to
    // the file is the file.
    // The destination itself is always one of them: with the detailed format it need not end in
    // the number of a generation.
    // Sakura rotates for the kinds 3 and 6 even with the detailed format ("@todo Advanced modeでの
    // 世代管理"). Here a detailed name rotates only if it ends in two digits (`$0.b00`): a hidden
    // choice of the simple controls must not delete the files of `$0.bak`'s neighbours.
    let generations = (prefs.kind == 3 || prefs.kind == 6)
        && (!prefs.advanced
            || dest
                .to_string_lossy()
                .chars()
                .rev()
                .take(2)
                .all(|c| c.is_ascii_digit()));
    let mut touched: Vec<PathBuf> = vec![dest.to_owned()];
    if generations {
        touched.extend((0..=100).map(|n| numbered(dest, n)));
    }
    if touched
        .iter()
        .any(|path| ume_platform::file_id::same_file(target, path))
    {
        return Err(Failure::Copy(io::Error::other(
            "the backup would replace the file itself",
        )));
    }
    // `CreateDirectory` of the folder of the backup, one level.
    if let Some(parent) = dest.parent().filter(|p| !p.as_os_str().is_empty())
        && !parent.exists()
    {
        let _ = fs::create_dir(parent);
    }
    // The copy is made first, into a temporary file, so that a copy that fails changes nothing
    // (no generation has been deleted or moved yet).
    let temp = copy_to_temporary(target, dest).map_err(Failure::Copy)?;
    if generations && let Err(failure) = rotate_generations(prefs, dest) {
        let _ = fs::remove_file(&temp);
        return Err(failure);
    }
    // The name is replaced, not the contents: a file that shares the data of the old backup (a hard
    // link) is not written to.
    if let Err(error) = fs::rename(&temp, dest) {
        let _ = fs::remove_file(&temp);
        return Err(Failure::Copy(error));
    }
    if to_trash {
        // The result is not looked at (`SHFileOperation` in Sakura).
        let _ = ume_platform::trash::move_to_trash(dest);
    }
    Ok(())
}

/// Copies `target` into a new temporary file in the folder of `dest` and returns its path. The
/// file is made exclusively under a short unique name (it never follows or overwrites what is
/// there, a symbolic link included), and gets the permissions of `target` as `fs::copy` gives them.
/// On a failure nothing is left.
fn copy_to_temporary(target: &Path, dest: &Path) -> io::Result<PathBuf> {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let mut source = fs::File::open(target)?;
    let permissions = source.metadata()?.permissions();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    for _ in 0..100 {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp = dest.with_file_name(format!(
            ".ume-backup-{:x}-{n:x}-{nanos:x}.tmp",
            std::process::id()
        ));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        // Private while it is written; the permissions of `target` are set at the end.
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let mut file = match options.open(&temp) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let copied = io::copy(&mut source, &mut file)
            .and_then(|_| ume_core::file::sync_file(&file))
            .and_then(|()| fs::set_permissions(&temp, permissions.clone()));
        return match copied {
            Ok(()) => Ok(temp),
            Err(e) => {
                let _ = fs::remove_file(&temp);
                Err(e)
            }
        };
    }
    Err(io::Error::other("no unique name for the temporary file"))
}

/// Step 1 to 3 of `MakeBackUp` for the kinds 3 and 6: `.b00` becomes `.b01` and so on, and the
/// numbers from the count on are deleted.
fn rotate_generations(prefs: &BackupPrefs, dest: &Path) -> Result<(), Failure> {
    let exists = |number: usize| fs::symlink_metadata(numbered(dest, number)).is_ok();
    // 1. The numbers that are there (up to 99, whatever the count is).
    let mut i: isize = 0;
    while i <= 99 && exists(i as usize) {
        i += 1;
    }
    i -= 1;
    // 2. From the highest down to the count - 1: delete.
    let boundary = (prefs.generations.max(1) - 1) as isize;
    while i >= boundary {
        let old = numbered(dest, i as usize);
        if fs::remove_file(&old).is_err() {
            return Err(Failure::Delete(old));
        }
        i -= 1;
    }
    // 3. The rest, from the highest down to 0: one number up.
    while i >= 0 {
        let from = numbered(dest, i as usize);
        if fs::rename(&from, numbered(dest, i as usize + 1)).is_err() {
            return Err(Failure::Move(from));
        }
        i -= 1;
    }
    Ok(())
}

/// What `OnPreBeforeSave` and `MakeBackUp` decide before they ask anything.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Prepared {
    /// No backup, and the save goes on.
    Skip,
    /// The path could not be made (`STR_BACKUP_ERR_PATH_CRETE`).
    BadPath,
    /// Copy the file to `dest`; `to_trash` if the copy goes to the recycle bin.
    Make { dest: PathBuf, to_trash: bool },
}

/// The decision, from the settings and the file. `folder` is the expanded 「指定フォルダー」;
/// `is_local` says whether a path is on a local disk.
pub(crate) fn prepare(
    prefs: &BackupPrefs,
    target: &Path,
    folder: &Path,
    is_local: &dyn Fn(&Path) -> bool,
    now: NaiveDateTime,
) -> Prepared {
    // A file that is not there is a new file (`OnPreBeforeSave`); one that cannot be written is
    // left alone (`MakeBackUp`: `_waccess( target_file, 2 )`).
    let Ok(metadata) = fs::metadata(target) else {
        return Prepared::Skip;
    };
    if !prefs.enabled || metadata.permissions().readonly() {
        return Prepared::Skip;
    }
    let last_write = metadata
        .modified()
        .map(|t| chrono::DateTime::<Local>::from(t).naive_local())
        .unwrap_or(now);

    // `bBackUpFolderRM`: only for files on removable media (not local).
    let folder = (prefs.use_folder
        && (!prefs.folder_removable_only || !is_local(target))
        && !folder.as_os_str().is_empty())
    .then_some(folder);
    let Some(dest) = backup_path(prefs, target, folder, Times { now, last_write }) else {
        return Prepared::BadPath;
    };
    // A network or removable drive is not given to the recycle bin.
    let to_trash = prefs.dust_box && is_local(&dest);
    Prepared::Make { dest, to_trash }
}

/// The buttons of the questions.
const YES: &str = "はい(Y)";
const NO: &str = "いいえ(N)";
const CANCEL: &str = "キャンセル";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Reply {
    Yes,
    No,
    Cancel,
}

async fn question(
    cx: &mut AsyncWindowContext,
    message: &str,
    detail: &str,
    with_cancel: bool,
) -> Reply {
    let labels: &[&str] = if with_cancel {
        &[YES, NO, CANCEL]
    } else {
        &[YES, NO]
    };
    match cx
        .prompt(PromptLevel::Info, message, Some(detail), labels)
        .await
    {
        Ok(0) => Reply::Yes,
        Ok(1) => Reply::No,
        _ => Reply::Cancel,
    }
}

/// `OnPreBeforeSave`: makes the backup of the file at `target` before it is overwritten. Returns
/// whether the save goes on.
pub(crate) async fn before_save(cx: &mut AsyncWindowContext, target: &Path) -> bool {
    let Ok(settings) = cx.update(|_, app| prefs(app)) else {
        return true;
    };
    if !settings.enabled || !target.exists() {
        return true;
    }
    let folder = cx
        .update(|_, app| {
            PathBuf::from(native(&crate::file_dialog::expanded_text(
                app,
                &settings.folder,
            )))
        })
        .unwrap_or_default();
    let prepared = prepare(
        &settings,
        target,
        &folder,
        &|path| ume_platform::drive::is_local(path),
        Local::now().naive_local(),
    );
    let (dest, to_trash) = match prepared {
        Prepared::Skip => return true,
        Prepared::BadPath => {
            // `STR_BACKUP_ERR_PATH_CRETE`: Yes saves without the backup, No stops.
            let reply = question(
                cx,
                "バックアップ先のパス作成中にエラーになりました。\nパスが長すぎるか不正な書式です。\nバックアップを作成せずに上書き保存してよろしいですか？",
                "",
                false,
            )
            .await;
            return reply == Reply::Yes;
        }
        Prepared::Make { dest, to_trash } => (dest, to_trash),
    };

    if settings.confirm {
        // `STR_BACKUP_CONFORM_MSG1` (with the recycle bin, Yes and No) and `_MSG2` (with Cancel).
        let message = "変更される前に、バックアップファイルを作成します。\nよろしいですか？  [いいえ(N)] を選ぶと作成せずに上書き（または名前を付けて）保存になります。";
        let mut detail = format!("{}\n    ↓\n{}\n", target.display(), dest.display());
        if to_trash {
            detail.push_str("\n作成したバックアップファイルをごみ箱に放り込みます。\n");
        }
        match question(cx, message, &detail, !to_trash).await {
            Reply::Yes => {}
            Reply::No => return true,
            Reply::Cancel => return false,
        }
    }

    match make_backup(&settings, target, &dest, to_trash) {
        Ok(()) => true,
        Err(Failure::Delete(path)) => {
            tell(
                cx,
                PromptLevel::Warning,
                "削除失敗",
                Some(&path.display().to_string()),
            )
            .await;
            true
        }
        Err(Failure::Move(path)) => {
            tell(
                cx,
                PromptLevel::Warning,
                "移動失敗",
                Some(&path.display().to_string()),
            )
            .await;
            true
        }
        Err(Failure::Copy(error)) => {
            // `STR_BACKUP_ERR_MSG`: Yes overwrites without the backup, No stops.
            let reply = question(
                cx,
                "バックアップの作成に失敗しました．元ファイルへの上書きを継続して行いますか．",
                &format!("{}\n{error}", dest.display()),
                false,
            )
            .await;
            reply == Reply::Yes
        }
    }
}

#[cfg(test)]
mod tests;
