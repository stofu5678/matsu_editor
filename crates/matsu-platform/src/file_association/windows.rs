//! The registry of Windows under [`Registry`](super::Registry), through `windows-sys`
//! (`Win32_System_Registry`). Not run by CI except for the test at the end, which Windows runs
//! under a key of its own; the user checks the rest.

// The registry functions are `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::io;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE,
    KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCopyTreeW, RegCreateKeyExW,
    RegDeleteKeyW, RegDeleteTreeW, RegDeleteValueW, RegOpenKeyExW, RegQueryInfoKeyW,
    RegQueryValueExW, RegSetValueExW,
};

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
        format!("{}\\{path}", self.user_classes)
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn check(code: u32) -> io::Result<()> {
    if code == ERROR_SUCCESS {
        Ok(())
    } else {
        // The registry functions return the error code itself.
        Err(io::Error::from_raw_os_error(code as i32))
    }
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

    fn create(hive: HKEY, path: &str) -> io::Result<Self> {
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
        check(code)?;
        Ok(Self(key))
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
        let name = wide(name);
        let data = wide(data);
        let bytes = u32::try_from(data.len() * 2)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "the value is too long"))?;
        // SAFETY: `name` and `data` are NUL-terminated, and `bytes` is the size of `data`.
        let code =
            unsafe { RegSetValueExW(key.0, name.as_ptr(), 0, REG_SZ, data.as_ptr().cast(), bytes) };
        check(code)
    }

    fn delete_value(&mut self, path: &str, name: &str) -> io::Result<()> {
        let Ok(key) = Key::open(HKEY_CURRENT_USER, &self.user(path), KEY_SET_VALUE) else {
            return Ok(());
        };
        let name = wide(name);
        // SAFETY: `name` is NUL-terminated.
        let code = unsafe { RegDeleteValueW(key.0, name.as_ptr()) };
        if code == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            check(code)
        }
    }

    fn delete_tree(&mut self, path: &str) -> io::Result<()> {
        let path = wide(&self.user(path));
        // SAFETY: `path` is NUL-terminated.
        let code = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, path.as_ptr()) };
        if code == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            check(code)
        }
    }

    fn delete_key_if_empty(&mut self, path: &str) -> io::Result<()> {
        let path = self.user(path);
        let Ok(key) = Key::open(HKEY_CURRENT_USER, &path, KEY_QUERY_VALUE) else {
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
        check(code)?;
        drop(key);
        if subkeys != 0 || values != 0 {
            return Ok(());
        }
        let path = wide(&path);
        // SAFETY: `path` is NUL-terminated. `RegDeleteKeyW` fails for a key with subkeys, which it
        // does not have.
        let code = unsafe { RegDeleteKeyW(HKEY_CURRENT_USER, path.as_ptr()) };
        if code == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            check(code)
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
            .map_err(|code| io::Error::from_raw_os_error(code as i32))?;
        let target = Key::create(HKEY_CURRENT_USER, &self.user(to))?;
        // SAFETY: both handles are open; no sub key name means the key itself.
        let code = unsafe { RegCopyTreeW(source.0, null(), target.0) };
        check(code)
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

        apply(&mut registry, "mtest", exe, Change::Menu(false)).unwrap();
        assert!(!registry.key_exists(".mtest"));
        assert!(!registry.key_exists("MatsuEditor_mtest"));
        assert!(!type_state(&registry, "mtest", "matsu.exe").menu);
    }
}
