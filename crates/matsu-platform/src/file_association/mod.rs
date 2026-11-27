//! The association of file types with MATSU Editor in the registry of Windows: the two check
//! boxes of the type list (`F_TYPE_LIST`), 右クリックメニューに追加 and ダブルクリックで開く.
//!
//! Sakura: `RegistExt`, `UnregistExt` and `CheckExt` in `sakura_core/typeprop/CDlgTypeList.cpp`
//! (lines 831-1106), called from `CDlgTypeList::DispatchEvent` (`LBN_SELCHANGE`,
//! `IDC_CHECK_EXT_RMENU`, `IDC_CHECK_EXT_DBLCLICK`, lines 195-300).
//!
//! What Sakura does, and what is the same here:
//!
//! * It writes only below `HKEY_CURRENT_USER\Software\Classes` (so no administrator rights); it
//!   reads `HKEY_LOCAL_MACHINE\Software\Classes` to copy the ProgID the extension had there.
//! * For an extension `.ext` (lower case) it makes the ProgID `SakuraEditor_ext`, sets it as the
//!   default value of `.ext`, and keeps the previous default in the value `SakuraEditorBackup`.
//! * Under the ProgID, `shell\SakuraEditor` (default value `Sakura &Editor`) with `command` =
//!   `"<exe>" "%1"` is the menu item. 右クリック is the existence of that key; ダブルクリック is
//!   `shell` (the default verb) having the value `SakuraEditor`, and the value it replaced is kept
//!   in `shell\SakuraEditor\ShellBackup`.
//! * Checking 右クリック registers with the double click on too (`RegistExt(ext, true)`); unchecking
//!   it removes the association (`UnregistExt`). Checking and unchecking ダブルクリック is
//!   `RegistExt(ext, checked)`. A token of the type's extension list with a wildcard is skipped, and
//!   the check box shows the AND of the extensions.
//! * Turning ダブルクリックで開く off gives `.ext` back to its previous default (or removes the
//!   value MATSU wrote) instead of leaving it on MATSU's ProgID as Sakura does
//!   (`keep_only_the_menu_item`).
//! * Sakura does nothing about the choice of the user (`UserChoice`). Neither does this (it only
//!   tells the user, see below). Sakura sets no icon and does not call `SHChangeNotify`; MATSU
//!   does both (the user decided on 2026-11-27): `DefaultIcon` of MATSU's own ProgID, in the form
//!   the installer uses (`<exe>,0`: `packaging/windows/matsu.iss`, `MATSU.Editor.txt\DefaultIcon`;
//!   `matsu.exe` carries one icon), and one `SHChangeNotify(SHCNE_ASSOCCHANGED)` after a change.
//!
//! What is different (ADR-0018 type B/F, see the PR):
//!
//! * Explorer takes the right-click verbs from the ProgID of the program the user chose as the
//!   default (`UserChoice`), so Sakura's item under its own ProgID does not show then. The item is
//!   also written under `SystemFileAssociations\.ext\shell\MatsuEditor` of the current user, which
//!   shows whatever the default is; 右クリック is that key. Removing deletes it too.
//! * The names are MATSU's: ProgID `MatsuEditor_ext`, the verb `MatsuEditor`, the label
//!   `MATSU &Editor`, `MatsuEditorBackup`.
//! * Removing deletes only what MATSU wrote: the ProgID `MatsuEditor_ext` of that extension (never
//!   another program's ProgID, which Sakura's `else` branch edits), the backup value, and the
//!   key `.ext` only when nothing else is left in it (Sakura deletes it with all it holds, such as
//!   `OpenWithProgids` of Explorer). The previous default of `.ext` comes back only while `.ext`
//!   still names MATSU's ProgID. Sakura also puts the backup of the default verb back without
//!   looking; here it comes back only while the default verb is still MATSU's.
//! * Windows keeps the choice of the user for the double click (`UserChoice`, protected by a hash:
//!   a program cannot set it). [`TypeState::overridden`] names the extensions whose double click
//!   still opens another program, so the dialog can say so. MATSU only reads it.
//!
//! The keys and values are made by plain functions ([`Ext`] and the names below), and the flows
//! ([`register`], [`unregister`], [`check`]) work on a [`Registry`], which the tests fake. Only
//! Windows has the real one (`windows`); on macOS and Linux [`supported`] is false and nothing
//! is read or written (the user decides those OSes later).

#[cfg(windows)]
mod windows;

#[cfg(test)]
mod tests;

use std::io;

/// The verb under `shell` (`ACTION_NAME`).
pub const ACTION_NAME: &str = "MatsuEditor";
/// The text of the menu item (`Sakura &Editor`).
pub const ACTION_LABEL: &str = "MATSU &Editor";
/// The value of `.ext` that keeps the previous default (`PROGID_BACKUP_NAME`).
pub const PROGID_BACKUP_NAME: &str = "MatsuEditorBackup";
/// The key under the verb that keeps the previous default verb (`ACTION_BACKUP_PATH`).
pub const SHELL_BACKUP_KEY: &str = "ShellBackup";
/// The start of a ProgID of MATSU (`SakuraEditor_`).
const PROGID_PREFIX: &str = "MatsuEditor_";

/// A file extension of a type, in lower case and without the dot (`CharLower` of the token).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ext(String);

impl Ext {
    /// The extension of one token of a type's extension list, or `None` when the token has a
    /// wildcard (`m_typeExtWildcards`; Sakura skips it) or cannot be the name of a registry key
    /// (`\`, `/` and the characters of a file name that Windows forbids).
    pub fn from_token(token: &str) -> Option<Self> {
        if token.is_empty()
            || token
                .chars()
                .any(|c| c.is_control() || "*?\\/:\"<>|".contains(c))
        {
            return None;
        }
        Some(Self(token.chars().flat_map(char::to_lowercase).collect()))
    }

    /// The extension without the dot.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The key of the extension, `.ext`.
    pub fn dot_key(&self) -> String {
        format!(".{}", self.0)
    }

    /// The ProgID of MATSU for this extension, `MatsuEditor_ext`.
    pub fn prog_id(&self) -> String {
        format!("{PROGID_PREFIX}{}", self.0)
    }

    fn shell_key(&self) -> String {
        format!("{}\\shell", self.prog_id())
    }

    fn action_key(&self) -> String {
        format!("{}\\{ACTION_NAME}", self.shell_key())
    }

    fn icon_key(&self) -> String {
        format!("{}\\DefaultIcon", self.prog_id())
    }

    fn command_key(&self) -> String {
        format!("{}\\command", self.action_key())
    }

    /// `SystemFileAssociations\.ext\shell`: the verbs for every file with this extension, whatever
    /// the default program is.
    fn system_shell_key(&self) -> String {
        format!("SystemFileAssociations\\{}\\shell", self.dot_key())
    }

    fn system_action_key(&self) -> String {
        format!("{}\\{ACTION_NAME}", self.system_shell_key())
    }

    fn system_command_key(&self) -> String {
        format!("{}\\command", self.system_action_key())
    }

    fn shell_backup_key(&self) -> String {
        format!("{}\\{SHELL_BACKUP_KEY}", self.action_key())
    }
}

/// The extensions of a type's extension list (`szTypeExts`, separated by space, `;` and `,`) that
/// can be associated, each once.
pub fn extensions_of(type_exts: &str) -> Vec<Ext> {
    let mut found: Vec<Ext> = Vec::new();
    for ext in type_exts.split([' ', ';', ',']).filter_map(Ext::from_token) {
        if !found.contains(&ext) {
            found.push(ext);
        }
    }
    found
}

/// The command of the menu item: the path of the program and `"%1"`, both quoted
/// (`"\"" + sExePath + "\" \"%1\""`).
pub fn command_line(exe: &str) -> String {
    format!("\"{exe}\" \"%1\"")
}

/// The value of `DefaultIcon`: the program and the index of its icon, as the installer writes it
/// for `MATSU.Editor.txt` (`"{app}\matsu.exe,0"` in `packaging/windows/matsu.iss`). `matsu.exe`
/// carries one icon (`crates/matsu/matsu.rc`), the program's; there is no separate document icon.
pub fn icon_value(exe: &str) -> String {
    format!("{exe},0")
}

/// Does the choice of the user (`UserChoice`) open MATSU? This is the one place that lists what
/// counts as MATSU's own ProgID for `ext` (case does not matter, as in the registry):
///
/// * `MatsuEditor_<ext>`, the one [`register`] writes;
/// * `MATSU.Editor.<ext>`, the one the installer registers (`packaging/windows/matsu.iss`, task
///   `associatetxt`: `MATSU.Editor.txt`); keep this in step with that file;
/// * `Applications\<exe>`, which "Open with" makes for the program itself (`matsu.iss`, task
///   `openwith`: `Applications\matsu.exe`), where `exe_name` is the file name of the running
///   program.
pub fn user_choice_is_ours(choice: &str, ext: &Ext, exe_name: &str) -> bool {
    let installer = format!("MATSU.Editor.{}", ext.as_str());
    if choice.eq_ignore_ascii_case(&ext.prog_id()) || choice.eq_ignore_ascii_case(&installer) {
        return true;
    }
    choice.split_once('\\').is_some_and(|(head, name)| {
        head.eq_ignore_ascii_case("Applications") && name.eq_ignore_ascii_case(exe_name)
    })
}

/// The part of the registry the flows use. Paths are relative to
/// `HKEY_CURRENT_USER\Software\Classes` and separated by `\`; `""` as a name is the default value.
pub trait Registry {
    /// Does the key exist (in `HKEY_CURRENT_USER`)?
    fn key_exists(&self, path: &str) -> bool;
    /// The string value (in `HKEY_CURRENT_USER`), or `None` when the key or the value is missing.
    fn value(&self, path: &str, name: &str) -> Option<String>;
    /// Sets a string value and creates the key if needed (in `HKEY_CURRENT_USER`).
    fn set_value(&mut self, path: &str, name: &str, data: &str) -> io::Result<()>;
    /// Deletes a value. A missing value or key is not an error.
    fn delete_value(&mut self, path: &str, name: &str) -> io::Result<()>;
    /// Deletes a key with everything below it. A missing key is not an error.
    fn delete_tree(&mut self, path: &str) -> io::Result<()>;
    /// Deletes a key that has no value and no subkey. Another key is left as it is.
    fn delete_key_if_empty(&mut self, path: &str) -> io::Result<()>;
    /// The default value of a key of `HKEY_LOCAL_MACHINE\Software\Classes` (read only).
    fn machine_default(&self, path: &str) -> Option<String>;
    /// Copies the key `from` of `HKEY_LOCAL_MACHINE\Software\Classes` with everything below it to
    /// the key `to` of `HKEY_CURRENT_USER` (`CopyRegistry`).
    fn copy_machine_tree(&mut self, from: &str, to: &str) -> io::Result<()>;
    /// The ProgID of the choice of the user for `.ext` (`UserChoice` of `FileExts`), read only.
    fn user_choice(&self, ext: &Ext) -> Option<String>;
    /// Tells the shell that associations changed (`SHChangeNotify(SHCNE_ASSOCCHANGED)`).
    fn notify_changed(&mut self);
}

/// What is registered for one extension (`CheckExt`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Status {
    /// 右クリックメニューに追加: the menu item exists.
    pub menu: bool,
    /// ダブルクリックで開く: the item is the default verb.
    pub default: bool,
}

/// Do two ProgIDs or verb names name the same thing? The registry does not care about case, so
/// every check of what is ours (and what to back up and restore) must not either.
fn same_name(a: &str, b: &str) -> bool {
    a == b || a.to_lowercase() == b.to_lowercase()
}

fn nonempty(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.is_empty())
}

/// `CheckExt`: what MATSU has registered for `ext`. The menu item is the one under
/// `SystemFileAssociations` (the one Explorer shows whatever the default program is); the default
/// verb is the one `CheckExt` reads in the ProgID the extension names now.
pub fn check(registry: &impl Registry, ext: &Ext) -> Status {
    let menu = registry.key_exists(&ext.system_action_key());
    let Some(prog_id) = nonempty(registry.value(&ext.dot_key(), "")) else {
        return Status {
            menu,
            default: false,
        };
    };
    let shell = format!("{prog_id}\\shell");
    let default = registry.key_exists(&format!("{shell}\\{ACTION_NAME}"))
        && registry
            .value(&shell, "")
            .is_some_and(|v| same_name(&v, ACTION_NAME));
    Status { menu, default }
}

/// `RegistExt`: associates `ext` with the program `exe` (the path, unquoted). With `make_default`
/// the item is also the default verb (the double click) and `.ext` names MATSU's ProgID; without
/// it only the right-click item stays (see `keep_only_the_menu_item`).
pub fn register(
    registry: &mut impl Registry,
    ext: &Ext,
    exe: &str,
    make_default: bool,
) -> io::Result<()> {
    if !make_default {
        return keep_only_the_menu_item(registry, ext, exe);
    }
    let dot = ext.dot_key();
    let prog_id = ext.prog_id();

    // The ProgID the extension has on the machine (`keyExt_HKLM`).
    let machine_prog_id = nonempty(registry.machine_default(&dot));

    // The verbs of the program the machine had for `.ext` stay available in our ProgID.
    if !registry.key_exists(&prog_id)
        && let Some(machine) = machine_prog_id
        && let Err(error) = registry.copy_machine_tree(&machine, &prog_id)
    {
        // A half-built ProgID would stand for "the copy is done" next time (the key exists).
        return Err(match registry.delete_tree(&prog_id) {
            Ok(()) => error,
            Err(cleanup) => io::Error::new(
                error.kind(),
                format!("{error}; removing the half-built {prog_id} failed too: {cleanup}"),
            ),
        });
    }

    registry.set_value(&ext.command_key(), "", &command_line(exe))?;
    registry.set_value(&ext.action_key(), "", ACTION_LABEL)?;
    // The icon of the files, only in our own ProgID (it replaces the one copied from the machine).
    registry.set_value(&ext.icon_key(), "", &icon_value(exe))?;

    // Explorer takes the verbs of a file from the ProgID of the program the user chose as the
    // default (`UserChoice`), so the item under our ProgID does not show when another program is
    // the default. The same item under `SystemFileAssociations` shows whatever the default is.
    registry.set_value(&ext.system_command_key(), "", &command_line(exe))?;
    registry.set_value(&ext.system_action_key(), "", ACTION_LABEL)?;

    let shell = ext.shell_key();
    let shell_value = registry.value(&shell, "").unwrap_or_default();
    if !same_name(&shell_value, ACTION_NAME) {
        if !shell_value.is_empty() {
            registry.set_value(&ext.shell_backup_key(), "", &shell_value)?;
        }
        registry.set_value(&shell, "", ACTION_NAME)?;
    }

    // Last: `.ext` names our ProgID only when the ProgID is complete. A failure above leaves
    // `.ext` as it was, not pointing at a handler that has no command.
    // `.ext` names our ProgID; the previous default is kept.
    let current = registry.value(&dot, "").unwrap_or_default();
    // `.ext` may already name our ProgID with other letter case: that is ours, not a previous
    // default to back up (a backup of it would be restored as a self-reference later).
    if !same_name(&current, &prog_id) {
        if !current.is_empty() {
            registry.set_value(&dot, PROGID_BACKUP_NAME, &current)?;
        }
        registry.set_value(&dot, "", &prog_id)?;
    } else if current != prog_id {
        registry.set_value(&dot, "", &prog_id)?;
    }
    Ok(())
}

/// ダブルクリックで開く off, 右クリックメニューに追加 on (ADR-0018 type B).
///
/// Sakura (`RegistExt(ext, false)`) only drops its own `shell` default and leaves `.ext` naming its
/// ProgID. That does not work when there is no other verb to fall back on: Windows then takes the
/// only verb left in the ProgID, which is MATSU's, and a double click still starts MATSU while the
/// check box is off. The item under `SystemFileAssociations` does not need `.ext` to name our
/// ProgID, so here `.ext` gets its previous default back (the backup), or loses MATSU's value when
/// there was none, while it still names MATSU's ProgID. MATSU's ProgID stays for the next time
/// the double click is turned on ([`register`] with `make_default`).
fn keep_only_the_menu_item(registry: &mut impl Registry, ext: &Ext, exe: &str) -> io::Result<()> {
    let dot = ext.dot_key();
    registry.set_value(&ext.system_command_key(), "", &command_line(exe))?;
    registry.set_value(&ext.system_action_key(), "", ACTION_LABEL)?;
    let prog_id = ext.prog_id();
    if registry
        .value(&dot, "")
        .is_some_and(|v| same_name(&v, &prog_id))
    {
        match nonempty(registry.value(&dot, PROGID_BACKUP_NAME)) {
            Some(previous) => registry.set_value(&dot, "", &previous)?,
            None => registry.delete_value(&dot, "")?,
        }
        registry.delete_value(&dot, PROGID_BACKUP_NAME)?;
        registry.delete_key_if_empty(&dot)?;
    }
    Ok(())
}

/// `UnregistExt`: removes what [`register`] wrote for `ext` (also the item under
/// `SystemFileAssociations`), and nothing else; MATSU's ProgID stays while `UserChoice` names it. `.ext` gets its
/// previous default back while it still names MATSU's ProgID; the key `.ext` goes only when it is
/// empty then.
pub fn unregister(registry: &mut impl Registry, ext: &Ext) -> io::Result<()> {
    let dot = ext.dot_key();
    let prog_id = ext.prog_id();
    if registry
        .value(&dot, "")
        .is_some_and(|v| same_name(&v, &prog_id))
    {
        match nonempty(registry.value(&dot, PROGID_BACKUP_NAME)) {
            Some(previous) => registry.set_value(&dot, "", &previous)?,
            None => registry.delete_value(&dot, "")?,
        }
    }
    registry.delete_value(&dot, PROGID_BACKUP_NAME)?;
    // The protected choice of the user (`UserChoice`) may name our ProgID: it cannot be repaired
    // from here, so the ProgID stays with its command and the choice keeps working. Only the
    // right-click item and `.ext` go; the dialog then says that Windows' own setting decides.
    let chosen = registry
        .user_choice(ext)
        .is_some_and(|choice| same_name(&choice, &prog_id));
    if !chosen {
        registry.delete_tree(&prog_id)?;
    }
    registry.delete_tree(&ext.system_action_key())?;
    // The keys above it are removed only when MATSU's item was all they held.
    registry.delete_key_if_empty(&ext.system_shell_key())?;
    registry.delete_key_if_empty(&format!("SystemFileAssociations\\{dot}"))?;
    registry.delete_key_if_empty("SystemFileAssociations")?;
    registry.delete_key_if_empty(&dot)
}

/// What the check boxes of the type list show for one type (the AND over its extensions, as
/// `m_bExtRMenu` and `m_bExtDblClick` of Sakura).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TypeState {
    /// The type has an extension that can be associated; the check boxes are grey without one.
    pub available: bool,
    /// 右クリックメニューに追加, for every extension.
    pub menu: bool,
    /// ダブルクリックで開く, for every extension.
    pub default: bool,
    /// The extensions where the double click is MATSU's but the user's own choice of Windows
    /// names another program, so Windows opens that one.
    pub overridden: Vec<String>,
    /// The extensions where the double click is off but the user's own choice of Windows names
    /// MATSU (this one's ProgID, the installer's, or the program), so Windows still opens MATSU:
    /// turning the check box off cannot change that.
    pub chosen: Vec<String>,
}

/// What one click on a check box asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// 右クリックメニューに追加 was set to this.
    Menu(bool),
    /// ダブルクリックで開く was set to this.
    Default(bool),
}

/// The state of a type with the extension list `type_exts`.
pub fn type_state(registry: &impl Registry, type_exts: &str, exe_name: &str) -> TypeState {
    let exts = extensions_of(type_exts);
    if exts.is_empty() {
        return TypeState::default();
    }
    let mut state = TypeState {
        available: true,
        menu: true,
        default: true,
        overridden: Vec::new(),
        chosen: Vec::new(),
    };
    for ext in &exts {
        let status = check(registry, ext);
        state.menu &= status.menu;
        state.default &= status.default;
        let Some(choice) = registry.user_choice(ext) else {
            continue;
        };
        let ours = user_choice_is_ours(&choice, ext, exe_name);
        if status.default && !ours {
            state.overridden.push(ext.as_str().to_owned());
        } else if !status.default && ours && (status.menu || same_name(&choice, &ext.prog_id())) {
            // Our own ProgID named by the choice counts without the menu item: removing the item
            // leaves the ProgID, and Windows still opens MATSU for the double click.
            state.chosen.push(ext.as_str().to_owned());
        }
    }
    state
}

/// The click on a check box (`IDC_CHECK_EXT_RMENU` and `IDC_CHECK_EXT_DBLCLICK`). It stops at the
/// first extension that fails; the extensions before it stay as they are changed.
pub fn apply(
    registry: &mut impl Registry,
    type_exts: &str,
    exe: &str,
    change: Change,
) -> io::Result<()> {
    let exts = extensions_of(type_exts);
    let result = apply_each(registry, &exts, exe, change);
    // Once, also after a failure half way: what was written is already in the registry.
    if !exts.is_empty() {
        registry.notify_changed();
    }
    result
}

fn apply_each(
    registry: &mut impl Registry,
    exts: &[Ext],
    exe: &str,
    change: Change,
) -> io::Result<()> {
    for ext in exts {
        match change {
            Change::Menu(true) => register(registry, ext, exe, true)?,
            Change::Menu(false) => unregister(registry, ext)?,
            Change::Default(on) => register(registry, ext, exe, on)?,
        }
    }
    Ok(())
}

/// Can this OS associate file types from the type list? Only Windows can for now; macOS and Linux
/// wait for the user's decision.
pub const fn supported() -> bool {
    cfg!(windows)
}

/// The path of the running program and its file name, for the real registry.
#[cfg(windows)]
fn this_program() -> io::Result<(String, String)> {
    let exe = std::env::current_exe()?;
    let name = exe
        .file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "the program's name"))?;
    let path = exe.to_str().map(str::to_owned).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "the program's path is not Unicode",
        )
    })?;
    Ok((path, name))
}

/// [`type_state`] of the real registry. It fails where there is no registry, and when the path of
/// the running program cannot be found (nothing is then known, which is not "not associated").
pub fn current_type_state(type_exts: &str) -> io::Result<TypeState> {
    #[cfg(windows)]
    {
        let (_, name) = this_program()?;
        let registry = windows::Real::current_user();
        Ok(type_state(&registry, type_exts, &name))
    }
    #[cfg(not(windows))]
    {
        let _ = type_exts;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "file associations are read on Windows only",
        ))
    }
}

/// [`apply`] to the real registry of Windows.
pub fn apply_to_system(type_exts: &str, change: Change) -> io::Result<()> {
    #[cfg(windows)]
    {
        let (exe, _) = this_program()?;
        let mut registry = windows::Real::current_user();
        apply(&mut registry, type_exts, &exe, change)
    }
    #[cfg(not(windows))]
    {
        let _ = (type_exts, change);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "file associations are written on Windows only",
        ))
    }
}
