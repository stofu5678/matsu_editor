//! The registry of Windows under [`Registry`](super::Registry), through `windows-sys`
//! (`Win32_System_Registry`). Not run by CI except for the test at the end, which Windows runs
//! under a key of its own; the user checks the rest.

// The registry functions are `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::io;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_ITEMS, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS,
};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE,
    KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegDeleteKeyW,
    RegDeleteTreeW, RegDeleteValueW, RegEnumKeyExW, RegEnumValueW, RegOpenKeyExW, RegQueryInfoKeyW,
    RegQueryValueExW, RegSetValueExW,
};
use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};

use super::{Ext, Registry};

/// Where the keys are. [`Real::current_user`] is the real place; the test uses a key of its own.
pub(super) struct Real {
    /// `Software\Classes` of `HKEY_CURRENT_USER`: the keys the flows read and write.
    user_classes: String,
    /// `Software\Classes` of the machine, read only.
    machine: (HKEY, String),
    /// `Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts` of `HKEY_CURRENT_USER`.
    file_exts: String,
}

impl Real {
    pub(super) fn current_user() -> Self {
        Self {
            user_classes: r"Software\Classes".to_owned(),
            machine: (HKEY_LOCAL_MACHINE, r"Software\Classes".to_owned()),
            file_exts: r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts".to_owned(),
        }
    }

    fn user(&self, path: &str) -> String {
        if path.is_empty() {
            return self.user_classes.clone();
        }
        format!("{}\\{path}", self.user_classes)
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// The result of a registry call, with the call and the key it was for in the message (the
/// functions return the error code itself): `RegSetValueExW Software\Classes\.txt: Access is denied.`
fn check(code: u32, call: &str, path: impl FnOnce() -> String) -> io::Result<()> {
    if code == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(with_context(code, call, &path()))
    }
}

/// A key or value that is not there (the only failure a delete may ignore).
fn is_missing(code: u32) -> bool {
    code == ERROR_FILE_NOT_FOUND || code == ERROR_PATH_NOT_FOUND
}

fn with_context(code: u32, call: &str, path: &str) -> io::Error {
    let os = io::Error::from_raw_os_error(code as i32);
    io::Error::new(os.kind(), format!("{call} {path}: {os}"))
}

/// An open key, closed at the end.
struct Key(HKEY);

impl Key {
    fn open(hive: HKEY, path: &str, access: u32) -> Result<Self, u32> {
        let path = wide(path);
        let mut key: HKEY = null_mut();
        // SAFETY: `path` is NUL-terminated and lives through the call; `key` is a valid place for
        // the handle.
        let code = unsafe { RegOpenKeyExW(hive, path.as_ptr(), 0, access, &mut key) };
        if code == ERROR_SUCCESS {
            Ok(Self(key))
        } else {
            Err(code)
        }
    }

    /// The key, or `None` when it is not there. Any other failure (access denied ...) is an error:
    /// a key that exists but cannot be opened must not be taken for one that needs no work.
    fn open_if_present(hive: HKEY, path: &str, access: u32) -> io::Result<Option<Self>> {
        match Self::open(hive, path, access) {
            Ok(key) => Ok(Some(key)),
            Err(code) if is_missing(code) => Ok(None),
            Err(code) => Err(with_context(code, "RegOpenKeyExW", path)),
        }
    }

    fn create(hive: HKEY, path: &str) -> io::Result<Self> {
        let name = path;
        let path = wide(path);
        let mut key: HKEY = null_mut();
        // SAFETY: as in `open`; the class, the attributes and the disposition are not used.
        let code = unsafe {
            RegCreateKeyExW(
                hive,
                path.as_ptr(),
                0,
                null(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                null(),
                &mut key,
                null_mut(),
            )
        };
        check(code, "RegCreateKeyExW", || name.to_owned())?;
        Ok(Self(key))
    }

    /// The names of the subkeys.
    fn subkeys(&self) -> io::Result<Vec<String>> {
        let mut names = Vec::new();
        for index in 0.. {
            // A key name has at most 255 characters.
            let mut buffer = [0u16; 256];
            let mut length = buffer.len() as u32;
            // SAFETY: `buffer` has `length` characters of room; the other outputs are not used.
            let code = unsafe {
                RegEnumKeyExW(
                    self.0,
                    index,
                    buffer.as_mut_ptr(),
                    &mut length,
                    null(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                )
            };
            if code == ERROR_NO_MORE_ITEMS {
                break;
            }
            check(code, "RegEnumKeyExW", String::new)?;
            names.push(String::from_utf16_lossy(&buffer[..length as usize]));
        }
        Ok(names)
    }

    /// Copies every value of this key to `target`, as they are (name, type and bytes). The two
    /// buffers are sized once from the longest name and the longest data of the key.
    fn copy_values_to(&self, target: &Key) -> io::Result<()> {
        let (mut longest_name, mut longest_data) = (0u32, 0u32);
        // SAFETY: only the two lengths are asked for; the other outputs are null.
        let code = unsafe {
            RegQueryInfoKeyW(
                self.0,
                null_mut(),
                null_mut(),
                null(),
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
                &mut longest_name,
                &mut longest_data,
                null_mut(),
                null_mut(),
            )
        };
        check(code, "RegQueryInfoKeyW", || "(the key to copy)".to_owned())?;
        // The name length does not count the NUL that the call writes.
        let mut name = vec![0u16; longest_name as usize + 1];
        let mut data = vec![0u8; longest_data as usize];
        for index in 0.. {
            let mut name_length = name.len() as u32;
            let mut size = data.len() as u32;
            let mut kind = 0u32;
            // SAFETY: `name` has `name_length` characters of room and `data` has `size` bytes.
            let code = unsafe {
                RegEnumValueW(
                    self.0,
                    index,
                    name.as_mut_ptr(),
                    &mut name_length,
                    null(),
                    &mut kind,
                    data.as_mut_ptr(),
                    &mut size,
                )
            };
            if code == ERROR_NO_MORE_ITEMS {
                break;
            }
            check(code, "RegEnumValueW", || format!("(value {index})"))?;
            // SAFETY: the call NUL-terminated `name`, and `data` has `size` valid bytes.
            let code =
                unsafe { RegSetValueExW(target.0, name.as_ptr(), 0, kind, data.as_ptr(), size) };
            check(code, "RegSetValueExW", || {
                format!(
                    "(a copied value, {})",
                    String::from_utf16_lossy(&name[..name_length as usize])
                )
            })?;
        }
        Ok(())
    }

    /// The string value `name` (`""` is the default value).
    fn string(&self, name: &str) -> Option<String> {
        let name = wide(name);
        let mut kind = 0;
        let mut size = 0u32;
        // SAFETY: the first call only asks for the size; `name` is NUL-terminated.
        let code = unsafe {
            RegQueryValueExW(
                self.0,
                name.as_ptr(),
                null(),
                &mut kind,
                null_mut(),
                &mut size,
            )
        };
        if code != ERROR_SUCCESS || (kind != REG_SZ && kind != 2) {
            return None;
        }
        // Room for the characters and a NUL that the stored data may lack.
        let mut buffer = vec![0u16; size as usize / 2 + 1];
        let mut size = u32::try_from(buffer.len() * 2 - 2).ok()?;
        // SAFETY: `buffer` has `size` bytes of room, as `size` says.
        let code = unsafe {
            RegQueryValueExW(
                self.0,
                name.as_ptr(),
                null(),
                &mut kind,
                buffer.as_mut_ptr().cast(),
                &mut size,
            )
        };
        if code != ERROR_SUCCESS {
            return None;
        }
        buffer.truncate(size as usize / 2);
        let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..end]))
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: the handle came from a successful open or create and is closed once.
        unsafe { RegCloseKey(self.0) };
    }
}

/// Copies the values and the subkeys of `source` into the key `to` of `HKEY_CURRENT_USER`, which
/// is made. Done by hand with the plain calls and the rights of `KEY_READ` and `KEY_WRITE`, not with
/// `RegCopyTreeW`: that is the one call of the flow that the real-registry test reached and the
/// Sandbox run did not (it needs a machine ProgID to copy), and the suspect for its "Access is
/// denied".
fn copy_tree(source: &Key, to: &str) -> io::Result<()> {
    let target = Key::create(HKEY_CURRENT_USER, to)?;
    source.copy_values_to(&target)?;
    for name in source.subkeys()? {
        let child = Key::open(source.0, &name, KEY_READ)
            .map_err(|code| with_context(code, "RegOpenKeyExW", &name))?;
        copy_tree(&child, &format!("{to}\\{name}"))?;
    }
    Ok(())
}

impl Registry for Real {
    fn key_exists(&self, path: &str) -> bool {
        Key::open(HKEY_CURRENT_USER, &self.user(path), KEY_READ).is_ok()
    }

    fn value(&self, path: &str, name: &str) -> Option<String> {
        Key::open(HKEY_CURRENT_USER, &self.user(path), KEY_QUERY_VALUE)
            .ok()?
            .string(name)
    }

    fn set_value(&mut self, path: &str, name: &str, data: &str) -> io::Result<()> {
        let key = Key::create(HKEY_CURRENT_USER, &self.user(path))?;
        let value_name = name;
        let name = wide(name);
        let data = wide(data);
        let bytes = u32::try_from(data.len() * 2)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "the value is too long"))?;
        // SAFETY: `name` and `data` are NUL-terminated, and `bytes` is the size of `data`.
        let code =
            unsafe { RegSetValueExW(key.0, name.as_ptr(), 0, REG_SZ, data.as_ptr().cast(), bytes) };
        check(code, "RegSetValueExW", || {
            format!("{} [{value_name}]", self.user(path))
        })
    }

    fn delete_value(&mut self, path: &str, name: &str) -> io::Result<()> {
        let Some(key) = Key::open_if_present(HKEY_CURRENT_USER, &self.user(path), KEY_SET_VALUE)?
        else {
            return Ok(());
        };
        let value_name = name;
        let name = wide(name);
        // SAFETY: `name` is NUL-terminated.
        let code = unsafe { RegDeleteValueW(key.0, name.as_ptr()) };
        if is_missing(code) {
            Ok(())
        } else {
            check(code, "RegDeleteValueW", || {
                format!("{} [{value_name}]", self.user(path))
            })
        }
    }

    fn delete_tree(&mut self, path: &str) -> io::Result<()> {
        let full = self.user(path);
        let wide_path = wide(&full);
        // SAFETY: `wide_path` is NUL-terminated.
        let code = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, wide_path.as_ptr()) };
        if is_missing(code) {
            Ok(())
        } else {
            check(code, "RegDeleteTreeW", || full)
        }
    }

    fn delete_key_if_empty(&mut self, path: &str) -> io::Result<()> {
        let full = self.user(path);
        let Some(key) = Key::open_if_present(HKEY_CURRENT_USER, &full, KEY_QUERY_VALUE)? else {
            return Ok(());
        };
        let (mut subkeys, mut values) = (0u32, 0u32);
        // SAFETY: only the two counts are asked for; the other outputs are null.
        let code = unsafe {
            RegQueryInfoKeyW(
                key.0,
                null_mut(),
                null_mut(),
                null(),
                &mut subkeys,
                null_mut(),
                null_mut(),
                &mut values,
                null_mut(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        };
        check(code, "RegQueryInfoKeyW", || full.clone())?;
        drop(key);
        if subkeys != 0 || values != 0 {
            return Ok(());
        }
        let wide_path = wide(&full);
        // SAFETY: `wide_path` is NUL-terminated. `RegDeleteKeyW` fails for a key with subkeys,
        // which it does not have.
        let code = unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, wide_path.as_ptr()) };
        if is_missing(code) {
            Ok(())
        } else {
            check(code, "RegDeleteKeyW", || full)
        }
    }

    fn machine_default(&self, path: &str) -> Option<String> {
        let path = format!("{}\\{path}", self.machine.1);
        Key::open(self.machine.0, &path, KEY_QUERY_VALUE)
            .ok()?
            .string("")
    }

    fn copy_machine_tree(&mut self, from: &str, to: &str) -> io::Result<()> {
        let from = format!("{}\\{from}", self.machine.1);
        let source = Key::open(self.machine.0, &from, KEY_READ)
            .map_err(|code| with_context(code, "RegOpenKeyExW", &from))?;
        copy_tree(&source, &self.user(to))
    }

    fn notify_changed(&mut self) {
        // The test's own roots are not the shell's; only the real place tells the shell.
        if self.user_classes != r"Software\Classes" {
            return;
        }
        // SAFETY: `SHCNE_ASSOCCHANGED` takes no items: both pointers are null.
        unsafe {
            SHChangeNotify(SHCNE_ASSOCCHANGED as i32, SHCNF_IDLIST, null(), null());
        }
    }

    fn subkeys(&self, path: &str) -> io::Result<Vec<String>> {
        match Key::open_if_present(HKEY_CURRENT_USER, &self.user(path), KEY_READ)? {
            Some(key) => key.subkeys(),
            None => Ok(Vec::new()),
        }
    }

    fn user_choice(&self, ext: &Ext) -> Option<String> {
        let path = format!("{}\\{}\\UserChoice", self.file_exts, ext.dot_key());
        Key::open(HKEY_CURRENT_USER, &path, KEY_QUERY_VALUE)
            .ok()?
            .string("ProgId")
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Change, apply, type_state};
    use super::*;

    /// A key of this test below `HKEY_CURRENT_USER\Software`, with the process id in its name,
    /// deleted at the end even when the test fails. Nothing is written under `Classes`.
    struct TestRoot(String);

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let path = wide(&self.0);
            // SAFETY: `path` is NUL-terminated.
            unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, path.as_ptr()) };
        }
    }

    /// Writes the real registry, in a key of the test. Run by CI on Windows.
    #[test]
    fn the_real_registry_associates_and_removes() {
        let root = TestRoot(format!("Software\\MatsuEditorTest-{}", std::process::id()));
        let mut registry = Real {
            user_classes: format!("{}\\Classes", root.0),
            machine: (HKEY_CURRENT_USER, format!("{}\\Machine", root.0)),
            file_exts: format!("{}\\FileExts", root.0),
        };
        // A program of the machine owns `.mtest`; its ProgID has an `open` verb.
        let machine = format!("{}\\Machine", root.0);
        let mut machine_side = Real {
            user_classes: machine,
            machine: (HKEY_CURRENT_USER, String::new()),
            file_exts: String::new(),
        };
        machine_side.set_value(".mtest", "", "othertype").unwrap();
        machine_side
            .set_value(r"othertype\shell\open\command", "", "other.exe %1")
            .unwrap();

        let exe = r"C:\Program Files\MATSU Editor\matsu.exe";
        apply(&mut registry, "mtest", exe, Change::Menu(true)).unwrap();
        let state = type_state(&registry, "mtest", "matsu.exe");
        assert!(state.menu && state.default, "{state:?}");
        assert_eq!(
            registry.value(".mtest", "").as_deref(),
            Some("MatsuEditor_mtest")
        );
        assert_eq!(
            registry
                .value(r"MatsuEditor_mtest\shell\MatsuEditor\command", "")
                .as_deref(),
            Some(r#""C:\Program Files\MATSU Editor\matsu.exe" "%1""#)
        );
        // The verbs of the machine's ProgID were copied.
        assert!(registry.key_exists(r"MatsuEditor_mtest\shell\open\command"));
        assert!(registry.key_exists(r"SystemFileAssociations\.mtest\shell\MatsuEditor\command"));
        assert_eq!(
            registry
                .value(r"MatsuEditor_mtest\DefaultIcon", "")
                .as_deref(),
            Some(r"C:\Program Files\MATSU Editor\matsu.exe,0")
        );

        apply(&mut registry, "mtest", exe, Change::Menu(false)).unwrap();
        assert!(!registry.key_exists(".mtest"));
        assert!(!registry.key_exists("MatsuEditor_mtest"));
        assert!(!registry.key_exists("SystemFileAssociations"));
        assert!(!type_state(&registry, "mtest", "matsu.exe").menu);
    }
}
