use std::fs;
use std::io::Write as _;

use super::*;

const DEF: &str = "[Plugin]\r\nId=jp.example.zipped\r\nName=Zipped\r\n";

/// One entry of a test archive.
struct Item<'a> {
    name: &'a [u8],
    data: &'a [u8],
    deflate: bool,
    /// The upper 16 bits of the external attributes with the host 3 (Unix), 0: none.
    unix_mode: u32,
    flags: u16,
    /// Overrides the CRC that is written.
    crc: Option<u32>,
    /// Overrides the size that is written.
    size: Option<u32>,
}

fn item<'a>(name: &'a str, data: &'a [u8]) -> Item<'a> {
    Item {
        name: name.as_bytes(),
        data,
        deflate: false,
        unix_mode: 0,
        flags: 0,
        crc: None,
        size: None,
    }
}

fn crc_of(data: &[u8]) -> u32 {
    let mut crc = Crc32::new();
    crc.update(data);
    crc.finish()
}

fn zip(items: &[Item]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for item in items {
        let stored: Vec<u8> = if item.deflate {
            let mut enc =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(item.data).unwrap();
            enc.finish().unwrap()
        } else {
            item.data.to_vec()
        };
        let method: u16 = if item.deflate { 8 } else { 0 };
        let crc = item.crc.unwrap_or_else(|| crc_of(item.data));
        let size = item.size.unwrap_or(item.data.len() as u32);
        let offset = out.len() as u32;
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&item.flags.to_le_bytes());
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(stored.len() as u32).to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&(item.name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(item.name);
        out.extend_from_slice(&stored);

        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        let host: u16 = if item.unix_mode != 0 { 3 } else { 0 };
        central.extend_from_slice(&(host << 8 | 20).to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&item.flags.to_le_bytes());
        central.extend_from_slice(&method.to_le_bytes());
        central.extend_from_slice(&[0; 4]);
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(stored.len() as u32).to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&(item.name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0; 2 + 2 + 2 + 2]);
        central.extend_from_slice(&(item.unix_mode << 16).to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(item.name);
    }
    let cd_offset = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(items.len() as u16).to_le_bytes());
    out.extend_from_slice(&(items.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn good() -> Vec<u8> {
    let script = "Editor.InsText('x');\n".repeat(50);
    zip(&[
        item("Zipped/", b""),
        item("Zipped/plugin.def", DEF.as_bytes()),
        Item {
            deflate: true,
            ..item("Zipped/cmd.js", script.as_bytes())
        },
        item("Zipped/local/", b""),
        item("Zipped/local/plugin_ja_JP.def", b"[Plugin]\r\n"),
    ])
}

fn one(name: &str) -> Result<ZipPlugin, ZipError> {
    ZipPlugin::from_bytes(zip(&[
        item("Zipped/plugin.def", DEF.as_bytes()),
        item(name, b"x"),
    ]))
}

#[test]
fn a_good_archive_is_one_plugin_and_unpacks() {
    let plugin = ZipPlugin::from_bytes(good()).unwrap();
    assert_eq!(plugin.folder, "Zipped");
    assert_eq!(plugin.id, "jp.example.zipped");
    let dir = tempfile::tempdir().unwrap();
    let plugins = dir.path().join("plugins");
    plugin.extract(&plugins).unwrap();
    let root = plugins.join("Zipped");
    assert_eq!(fs::read_to_string(root.join("plugin.def")).unwrap(), DEF);
    assert_eq!(
        fs::read_to_string(root.join("cmd.js")).unwrap(),
        "Editor.InsText('x');\n".repeat(50)
    );
    assert!(root.join("local/plugin_ja_JP.def").is_file());
    // The temporary folder is gone.
    let names: Vec<_> = fs::read_dir(&plugins).unwrap().flatten().collect();
    assert_eq!(names.len(), 1);
}

#[test]
fn the_file_can_be_read_from_disk() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("p.zip");
    fs::write(&file, good()).unwrap();
    assert_eq!(ZipPlugin::inspect(&file).unwrap().folder, "Zipped");
    assert!(matches!(
        ZipPlugin::inspect(&dir.path().join("none.zip")),
        Err(ZipError::Unreadable(_))
    ));
}

#[test]
fn the_folder_may_have_no_entry_of_its_own_and_backslashes_separate() {
    let plugin = ZipPlugin::from_bytes(zip(&[
        item("Zipped\\plugin.def", DEF.as_bytes()),
        item("./Zipped/a.js", b"1"),
    ]))
    .unwrap();
    assert_eq!(plugin.folder, "Zipped");
}

#[test]
fn the_definition_is_found_whatever_its_case() {
    let plugin = ZipPlugin::from_bytes(zip(&[item("P/PLUGIN.DEF", DEF.as_bytes())])).unwrap();
    assert_eq!(plugin.folder, "P");
}

#[test]
fn japanese_names_in_utf8_and_shift_jis_are_read() {
    let utf8 = ZipPlugin::from_bytes(zip(&[Item {
        flags: 0x800,
        ..item("プラグイン/plugin.def", DEF.as_bytes())
    }]))
    .unwrap();
    assert_eq!(utf8.folder, "プラグイン");
    let (sjis, _, _) = encoding_rs::SHIFT_JIS.encode("プラグイン/plugin.def");
    let sjis = ZipPlugin::from_bytes(zip(&[Item {
        name: &sjis,
        ..item("", DEF.as_bytes())
    }]))
    .unwrap();
    assert_eq!(sjis.folder, "プラグイン");
}

#[test]
fn an_archive_without_plugin_def_is_refused() {
    let bytes = zip(&[
        item("Zipped/a.js", b"1"),
        item("Zipped/sub/plugin.def", DEF.as_bytes()),
    ]);
    assert_eq!(
        ZipPlugin::from_bytes(bytes).unwrap_err(),
        ZipError::NoDefinition
    );
    assert_eq!(
        ZipPlugin::from_bytes(zip(&[])).unwrap_err(),
        ZipError::NoDefinition
    );
    // A folder named plugin.def is not the file.
    assert_eq!(
        ZipPlugin::from_bytes(zip(&[item("Zipped/plugin.def/", b"")])).unwrap_err(),
        ZipError::NoDefinition
    );
}

#[test]
fn the_archive_must_have_one_folder_and_nothing_outside_it() {
    let two = zip(&[
        item("A/plugin.def", DEF.as_bytes()),
        item("B/plugin.def", DEF.as_bytes()),
    ]);
    assert_eq!(
        ZipPlugin::from_bytes(two).unwrap_err(),
        ZipError::NotOneFolder
    );
    let loose = zip(&[
        item("A/plugin.def", DEF.as_bytes()),
        item("readme.txt", b"x"),
    ]);
    assert_eq!(
        ZipPlugin::from_bytes(loose).unwrap_err(),
        ZipError::NotOneFolder
    );
    let flat = zip(&[item("plugin.def", DEF.as_bytes())]);
    assert_eq!(
        ZipPlugin::from_bytes(flat).unwrap_err(),
        ZipError::NotOneFolder
    );
}

#[test]
fn paths_that_leave_the_folder_are_refused() {
    for name in [
        "Zipped/../evil.js",
        "../evil.js",
        "Zipped/a/../../evil.js",
        "/etc/passwd",
        "\\evil.js",
        "C:/evil.js",
        "Zipped/C:evil.js",
        "Zipped/plugin.def:stream",
        "Zipped//a.js",
        "Zipped/a.js.",
        "Zipped/con.txt",
        "Zipped/a\u{1}b.js",
        "Zipped/a?.js",
        "Zipped/..\\evil.js",
    ] {
        assert!(
            matches!(one(name), Err(ZipError::UnsafePath(_))),
            "{name:?} was accepted: {:?}",
            one(name)
        );
    }
    // A name that starts with two dots is not a way up.
    assert!(one("Zipped/..a.js").is_ok());
}

#[test]
fn links_and_devices_are_refused() {
    let link = Item {
        unix_mode: 0o120777,
        ..item("Zipped/link", b"/etc/passwd")
    };
    let bytes = zip(&[item("Zipped/plugin.def", DEF.as_bytes()), link]);
    assert!(matches!(
        ZipPlugin::from_bytes(bytes),
        Err(ZipError::SpecialFile(_))
    ));
    let fifo = Item {
        unix_mode: 0o010644,
        ..item("Zipped/fifo", b"")
    };
    assert!(matches!(
        ZipPlugin::from_bytes(zip(&[item("Zipped/plugin.def", DEF.as_bytes()), fifo])),
        Err(ZipError::SpecialFile(_))
    ));
    let file = Item {
        unix_mode: 0o100644,
        ..item("Zipped/ok.js", b"1")
    };
    assert!(ZipPlugin::from_bytes(zip(&[item("Zipped/plugin.def", DEF.as_bytes()), file])).is_ok());
}

#[test]
fn the_same_path_twice_is_refused_also_in_another_case() {
    let bytes = zip(&[
        item("Zipped/plugin.def", DEF.as_bytes()),
        item("Zipped/a.js", b"1"),
        item("Zipped/A.JS", b"2"),
    ]);
    assert!(matches!(
        ZipPlugin::from_bytes(bytes),
        Err(ZipError::Duplicate(_))
    ));
    let bytes = zip(&[
        item("Zipped/plugin.def", DEF.as_bytes()),
        item("Zipped/a", b"1"),
        item("Zipped/a/b.js", b"2"),
    ]);
    assert!(matches!(
        ZipPlugin::from_bytes(bytes),
        Err(ZipError::Duplicate(_))
    ));
}

#[test]
fn the_id_is_checked_before_anything_is_written() {
    let no_id = zip(&[item("Z/plugin.def", b"[Plugin]\r\nName=x\r\n")]);
    assert_eq!(
        ZipPlugin::from_bytes(no_id).unwrap_err(),
        ZipError::Id(InstallError::NoId)
    );
    let digit = zip(&[item("Z/plugin.def", b"[Plugin]\r\nId=1abc\r\n")]);
    assert_eq!(
        ZipPlugin::from_bytes(digit).unwrap_err(),
        ZipError::Id(InstallError::IdStartsWithDigit)
    );
    let reserved = zip(&[item("Z/plugin.def", b"[Plugin]\r\nId=a/b\r\n")]);
    assert_eq!(
        ZipPlugin::from_bytes(reserved).unwrap_err(),
        ZipError::Id(InstallError::ReservedChar('/'))
    );
}

#[test]
fn the_name_of_the_folder_has_a_limit() {
    let long = "x".repeat(ID_MAX_CHARS + 1);
    let bytes = zip(&[item(&format!("{long}/plugin.def"), DEF.as_bytes())]);
    assert_eq!(
        ZipPlugin::from_bytes(bytes).unwrap_err(),
        ZipError::BadFolderName
    );
    let hidden = zip(&[item(".git/plugin.def", DEF.as_bytes())]);
    assert_eq!(
        ZipPlugin::from_bytes(hidden).unwrap_err(),
        ZipError::BadFolderName
    );
}

#[test]
fn big_archives_are_refused() {
    // Too many entries.
    let names: Vec<String> = (0..=MAX_ENTRIES).map(|i| format!("Z/f{i}.txt")).collect();
    let items: Vec<Item> = names.iter().map(|n| item(n, b"")).collect();
    assert_eq!(
        ZipPlugin::from_bytes(zip(&items)).unwrap_err(),
        ZipError::TooManyEntries
    );
    // A file whose declared size is over the limit (nothing is allocated for it).
    let big = Item {
        size: Some(MAX_FILE_BYTES as u32 + 1),
        ..item("Z/big.bin", b"x")
    };
    assert_eq!(
        ZipPlugin::from_bytes(zip(&[item("Z/plugin.def", DEF.as_bytes()), big])).unwrap_err(),
        ZipError::TooLarge
    );
    // Many files that are each under the limit but together over it.
    let each = Some(MAX_FILE_BYTES as u32);
    let names: Vec<String> = (0..5).map(|i| format!("Z/big{i}.bin")).collect();
    let items: Vec<Item> = names
        .iter()
        .map(|n| Item {
            size: each,
            ..item(n, b"x")
        })
        .collect();
    assert_eq!(
        ZipPlugin::from_bytes(zip(&items)).unwrap_err(),
        ZipError::TooLarge
    );
    // The file on disk.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("big.zip");
    let f = fs::File::create(&file).unwrap();
    f.set_len(MAX_ARCHIVE_BYTES + 1).unwrap();
    assert_eq!(
        ZipPlugin::inspect(&file).unwrap_err(),
        ZipError::ArchiveTooLarge
    );
}

#[test]
fn a_size_that_lies_is_caught_and_nothing_is_left() {
    // The archive says 1 byte; the data inflate to more (a bomb in small).
    let data = vec![b'a'; 100_000];
    let bomb = Item {
        deflate: true,
        size: Some(1),
        crc: Some(crc_of(b"a")),
        ..item("Z/bomb.txt", &data)
    };
    let plugin = ZipPlugin::from_bytes(zip(&[item("Z/plugin.def", DEF.as_bytes()), bomb])).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let plugins = dir.path().join("plugins");
    assert!(matches!(
        plugin.extract(&plugins),
        Err(ZipError::Corrupt(_))
    ));
    assert_eq!(
        fs::read_dir(&plugins).unwrap().count(),
        0,
        "nothing left behind"
    );
}

#[test]
fn a_wrong_checksum_is_caught() {
    let bad = Item {
        crc: Some(1),
        ..item("Z/a.js", b"1")
    };
    let plugin = ZipPlugin::from_bytes(zip(&[item("Z/plugin.def", DEF.as_bytes()), bad])).unwrap();
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        plugin.extract(&dir.path().join("plugins")),
        Err(ZipError::Corrupt(_))
    ));
    assert!(!dir.path().join("plugins").join("Z").exists());
}

#[test]
fn encrypted_zip64_and_other_methods_are_refused() {
    let encrypted = Item {
        flags: 1,
        ..item("Z/a.js", b"1")
    };
    assert!(matches!(
        ZipPlugin::from_bytes(zip(&[item("Z/plugin.def", DEF.as_bytes()), encrypted])),
        Err(ZipError::Unsupported(_))
    ));
    let mut bytes = good();
    // The method of the first entry in the central directory: 12 (bzip2).
    let cd = le32(&bytes, bytes.len() - 6).unwrap() as usize;
    bytes[cd + 10] = 12;
    // The first entry is a folder, which does not need a method: use the second one.
    assert!(ZipPlugin::from_bytes(bytes.clone()).is_ok());
    let first = 46 + "Zipped/".len();
    bytes[cd + first + 10] = 12;
    assert!(matches!(
        ZipPlugin::from_bytes(bytes),
        Err(ZipError::Unsupported(_))
    ));
    // The end record that says ZIP64.
    let mut bytes = good();
    let eocd = bytes.len() - EOCD_SIZE;
    bytes[eocd + 16..eocd + 20].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        ZipPlugin::from_bytes(bytes).unwrap_err(),
        ZipError::Unsupported("ZIP64")
    );
}

#[test]
fn things_that_are_not_zip_files_are_refused() {
    for bytes in [
        vec![],
        b"hello".to_vec(),
        vec![0x50, 0x4b, 5, 6, 0, 0, 0, 0],
    ] {
        assert_eq!(ZipPlugin::from_bytes(bytes).unwrap_err(), ZipError::NotZip);
    }
    let mut bytes = good();
    bytes.truncate(bytes.len() - 30);
    assert!(ZipPlugin::from_bytes(bytes).is_err());
    // An offset that points outside the file.
    let mut bytes = good();
    let eocd = bytes.len() - EOCD_SIZE;
    bytes[eocd + 16..eocd + 20].copy_from_slice(&0x00ff_0000u32.to_le_bytes());
    assert_eq!(ZipPlugin::from_bytes(bytes).unwrap_err(), ZipError::NotZip);
}

#[test]
fn overwriting_replaces_the_files_and_keeps_the_others() {
    let dir = tempfile::tempdir().unwrap();
    let plugins = dir.path().join("plugins");
    let root = plugins.join("Zipped");
    fs::create_dir_all(root.join("local")).unwrap();
    fs::write(root.join("plugin.def"), "[Plugin]\r\nId=old\r\n").unwrap();
    fs::write(root.join("mine.txt"), "mine").unwrap();
    ZipPlugin::from_bytes(good())
        .unwrap()
        .extract(&plugins)
        .unwrap();
    assert_eq!(fs::read_to_string(root.join("plugin.def")).unwrap(), DEF);
    assert_eq!(fs::read_to_string(root.join("mine.txt")).unwrap(), "mine");
    assert!(root.join("local/plugin_ja_JP.def").is_file());
}

#[test]
fn a_file_where_a_folder_goes_stops_the_unpacking() {
    let dir = tempfile::tempdir().unwrap();
    let plugins = dir.path().join("plugins");
    fs::create_dir_all(&plugins).unwrap();
    fs::write(plugins.join("Zipped"), "a file").unwrap();
    assert!(matches!(
        ZipPlugin::from_bytes(good()).unwrap().extract(&plugins),
        Err(ZipError::Write(_))
    ));
    assert_eq!(
        fs::read_to_string(plugins.join("Zipped")).unwrap(),
        "a file"
    );
    assert_eq!(
        fs::read_dir(&plugins).unwrap().count(),
        1,
        "no temporary folder"
    );
}

#[cfg(unix)]
#[test]
fn nothing_is_written_through_a_link_that_is_there() {
    let dir = tempfile::tempdir().unwrap();
    let plugins = dir.path().join("plugins");
    let outside = dir.path().join("outside.txt");
    fs::write(&outside, "keep").unwrap();
    fs::create_dir_all(plugins.join("Zipped")).unwrap();
    std::os::unix::fs::symlink(&outside, plugins.join("Zipped/cmd.js")).unwrap();
    ZipPlugin::from_bytes(good())
        .unwrap()
        .extract(&plugins)
        .unwrap();
    // The link was replaced by the file; the target of the link is untouched.
    assert_eq!(fs::read_to_string(&outside).unwrap(), "keep");
    assert!(!plugins.join("Zipped/cmd.js").is_symlink());
}

#[test]
fn the_crc_is_the_one_of_zip() {
    assert_eq!(crc_of(b"123456789"), 0xCBF4_3926);
    assert_eq!(crc_of(b""), 0);
}
