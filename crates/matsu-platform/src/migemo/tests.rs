use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::*;

/// An absolute path that does not depend on the OS: a drive on Windows, the root elsewhere.
fn abs(path: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("C:\\{}", path.replace('/', "\\")))
    } else {
        PathBuf::from(format!("/{path}"))
    }
}

fn files(list: &[PathBuf]) -> impl Fn(&Path) -> bool + use<> {
    let set: HashSet<PathBuf> = list.iter().cloned().collect();
    move |p| set.contains(p)
}

#[test]
fn an_absolute_path_is_used_as_it_is() {
    let dll = abs("tools/migemo/custom.dll");
    let got = library_path(
        &dll.to_string_lossy(),
        Some(&abs("ini")),
        Some(&abs("exe")),
        files(std::slice::from_ref(&dll)),
    );
    assert_eq!(got, Some(dll));
}

#[test]
fn a_relative_name_is_looked_up_under_the_settings_then_the_program() {
    let (ini, exe) = (abs("ini"), abs("exe"));
    let in_exe = exe.join("lib").join("m.dll");
    let got = library_path(
        "lib/m.dll",
        Some(&ini),
        Some(&exe),
        files(std::slice::from_ref(&in_exe)),
    );
    assert_eq!(got, Some(in_exe.clone()));
    let in_ini = ini.join("lib").join("m.dll");
    let both = files(&[in_ini.clone(), in_exe]);
    let got = library_path("lib/m.dll", Some(&ini), Some(&exe), both);
    assert_eq!(got, Some(in_ini));
}

#[test]
fn a_relative_name_never_reaches_the_os_as_it_is() {
    // nothing exists: the answer is still a full path
    let ini = abs("ini");
    for configured in ["", "migemo.dll", "lib/m.dll", "..", "./x"] {
        let got = library_path(configured, Some(&ini), Some(&abs("exe")), |_| false).unwrap();
        assert!(got.is_absolute(), "{configured}: {got:?}");
    }
    // with no folder at all there is no path
    assert_eq!(library_path("migemo.dll", None, None, |_| false), None);
    // a folder that is itself relative is not a place to look in
    assert_eq!(
        library_path(
            "m.dll",
            Some(Path::new("rel")),
            Some(Path::new("rel2")),
            |_| true
        ),
        None
    );
}

#[test]
fn the_default_name_is_the_one_of_the_program_folder_then_the_settings_folder() {
    let (ini, exe) = (abs("ini"), abs("exe"));
    let name = library_file_name();
    let both = files(&[ini.join(name), exe.join(name)]);
    assert_eq!(
        library_path("", Some(&ini), Some(&exe), both),
        Some(exe.join(name))
    );
    let only_ini = files(&[ini.join(name)]);
    assert_eq!(
        library_path("", Some(&ini), Some(&exe), only_ini),
        Some(ini.join(name))
    );
    // a configured file that is not there falls back to the default name, as Sakura does
    let only_exe = files(&[exe.join(name)]);
    assert_eq!(
        library_path("gone.dll", Some(&ini), Some(&exe), only_exe),
        Some(exe.join(name))
    );
}

#[test]
fn the_dictionary_folder_is_under_the_settings_and_has_the_utf8_dictionary() {
    let ini = abs("ini");
    let dict = ini.join("dict").join("utf-8");
    let here = files(&[dict.join("migemo-dict")]);
    assert_eq!(dictionary_dir("", Some(&ini), &here), Some(dict.clone()));
    assert_eq!(dictionary_dir("dict", Some(&ini), &here), Some(dict));
    // an absolute folder
    let other = abs("data/dict");
    let there = files(&[other.join("utf-8").join("migemo-dict")]);
    let given = other.to_string_lossy().into_owned();
    assert_eq!(
        dictionary_dir(&given, None, there),
        Some(other.join("utf-8"))
    );
    // the code page dictionary alone is not used; no settings folder is nothing
    let cp932 = files(&[ini.join("dict").join("cp932").join("migemo-dict")]);
    assert_eq!(dictionary_dir("", Some(&ini), cp932), None);
    assert_eq!(dictionary_dir("", None, |_| true), None);
}

#[test]
fn a_char_is_read_as_utf8() {
    assert_eq!(decode_char("a".as_bytes()), Some((0x61, 1)));
    assert_eq!(decode_char("é".as_bytes()), Some((0xE9, 2)));
    assert_eq!(decode_char("あい".as_bytes()), Some((0x3042, 3)));
    assert_eq!(decode_char("😀x".as_bytes()), Some((0x1F600, 4)));
    assert_eq!(decode_char(b""), None);
}

#[test]
fn a_broken_sequence_stands_for_its_first_byte() {
    // a lead byte without its tail, a stray tail byte, 0xFF
    assert_eq!(decode_char(&[0xE3, 0x81]), Some((0xE3, 1)));
    assert_eq!(decode_char(&[0xE3, 0x41, 0x41]), Some((0xE3, 1)));
    assert_eq!(decode_char(&[0x80]), Some((0x80, 1)));
    assert_eq!(decode_char(&[0xFF, 0x80]), Some((0xFF, 1)));
}

#[test]
fn a_char_is_written_as_utf8_and_syntax_is_escaped() {
    let put = |code: u32| {
        let mut out = [0u8; 8];
        let n = encode_char(code, &mut out);
        out[..n].to_vec()
    };
    assert_eq!(put(u32::from('a')), b"a");
    assert_eq!(put(0xE9), "é".as_bytes());
    assert_eq!(put(0x3042), "あ".as_bytes());
    assert_eq!(put(0x1F600), "😀".as_bytes());
    for c in "\\.*^$/[]|()+?{}:-&".chars() {
        assert_eq!(put(u32::from(c)), format!("\\{c}").into_bytes(), "{c}");
    }
    // not syntax
    for c in [
        '_', ' ', '#', '~', '!', '"', '%', ',', ';', '<', '=', '>', '@',
    ] {
        assert_eq!(put(u32::from(c)), c.to_string().into_bytes(), "{c}");
    }
    // a char goes through the two functions and comes back
    for c in ['あ', '漢', 'ｱ', '😀', 'é'] {
        let bytes = put(u32::from(c));
        assert_eq!(decode_char(&bytes), Some((u32::from(c), bytes.len())));
    }
}

#[test]
fn the_library_is_never_loaded_from_a_relative_path() {
    for name in ["migemo.dll", "./migemo.dll", "lib/libmigemo.so", ""] {
        match Migemo::open(Path::new(name)) {
            Err(OpenError::NotAbsolute) => {}
            Err(other) => panic!("{name}: {other:?}"),
            Ok(_) => panic!("{name} was loaded"),
        }
    }
}

#[test]
fn a_library_that_is_not_there_is_an_error_and_not_a_panic() {
    let missing = abs("matsu-test-no-such-dir/migemo-missing");
    assert!(matches!(Migemo::open(&missing), Err(OpenError::Load(_))));
}

#[test]
fn a_file_that_is_not_a_library_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join(library_file_name());
    std::fs::write(&fake, b"not a library").unwrap();
    assert!(matches!(Migemo::open(&fake), Err(OpenError::Load(_))));
}
