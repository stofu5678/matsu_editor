// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/plugin/CPluginManager.cpp (`InstZipPluginSub`), sakura_core/io/CZipFile.cpp
//   (`ChkPluginDef`, `Unzip`)
//
// Original copyright notices of those files:
//   Copyright (C) 2009, syat
//   Copyright (C) 2011, syat
//   Copyright (C) 2018-2026, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! Plugins in a ZIP file (ADR-0026, PR 11): `ZIPプラグインを導入`.
//!
//! Sakura (`CPluginManager::InstZipPluginSub`) hands the file to the shell's ZIP folder
//! (`CZipFile`): `ChkPluginDef` looks for a folder in the archive that has a `plugin.def`, and
//! `Unzip` copies everything into `plugins\`. [`ZipPlugin::inspect`] makes the same check and
//! [`ZipPlugin::extract`] the same copy, with the rules that the shell does not need to state but
//! a program that unpacks a file from anywhere does:
//!
//! * The archive has exactly one folder at its top, every entry is in it, and it has a
//!   `plugin.def` (Sakura takes the first folder that has one and copies the rest as it is).
//! * The `Plugin.Id` of the `plugin.def` is checked ([`check_id`], `InstallPlugin`) before
//!   anything is written.
//! * An entry cannot leave the folder: no `..`, no absolute path, no drive, no stream (`:`), no
//!   character or name that Windows refuses, no link or device. A name that cannot be written on
//!   Windows is refused on every system, so that an archive is good everywhere or nowhere.
//! * Limits ([`MAX_ARCHIVE_BYTES`] and the others) keep a small file from filling the disk; the
//!   sizes that the archive declares are checked while the files are inflated, not trusted.
//! * Encrypted entries, ZIP64 and archives in several parts are refused. Only stored and
//!   deflated entries are read (what the shell's ZIP folder writes, and every tool's default).
//!
//! The files are written to a temporary folder in `plugins\` first and moved in when all of
//! them are good, so a broken archive leaves nothing behind. Files that are in the plugin's folder
//! already are replaced and the others stay (the copy of Sakura does the same).

use std::io::Read as _;
use std::path::{Path, PathBuf};

use super::{ID_MAX_CHARS, InstallError, check_id, same_name};
use crate::profile::Profile;

/// The largest ZIP file that is read, in bytes. The file is read into memory.
pub const MAX_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;
/// The most entries (files and folders) an archive can have.
pub const MAX_ENTRIES: usize = 2000;
/// The largest file in the archive when unpacked, in bytes.
pub const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
/// The largest size of all the files when unpacked, in bytes.
pub const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
/// The deepest path inside the plugin folder (the folder itself counts as 1).
pub const MAX_DEPTH: usize = 16;
/// The longest name of one part of a path, in characters.
pub const MAX_NAME_CHARS: usize = 100;

/// Why a ZIP file was not installed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZipError {
    /// `STR_PLGMGR_INST_ZIP_ACCESS`: the file cannot be read.
    Unreadable(String),
    /// Not a ZIP file, or cut short.
    NotZip,
    /// A kind of ZIP file that is not read (the text says which).
    Unsupported(&'static str),
    /// The ZIP file is larger than [`MAX_ARCHIVE_BYTES`].
    ArchiveTooLarge,
    /// More than [`MAX_ENTRIES`] entries.
    TooManyEntries,
    /// A file larger than [`MAX_FILE_BYTES`], or all together more than [`MAX_TOTAL_BYTES`].
    TooLarge,
    /// `STR_PLGMGR_INST_ZIP_DEF`: no folder has a `plugin.def`.
    NoDefinition,
    /// The archive has more than one folder at its top, or a file outside the folder.
    NotOneFolder,
    /// The name of the folder is longer than [`ID_MAX_CHARS`] or starts with a dot.
    BadFolderName,
    /// An entry that would be written outside the folder, or with a name that cannot be used.
    UnsafePath(String),
    /// An entry that is a link or a device.
    SpecialFile(String),
    /// Two entries have the same path.
    Duplicate(String),
    /// The data of an entry is broken (does not inflate, or its size or checksum is not the
    /// declared one).
    Corrupt(String),
    /// `STR_PLGMGR_INST_ZIP_UNZIP`: a file could not be written.
    Write(String),
    /// `InstallPlugin` would refuse the ID.
    Id(InstallError),
}

impl ZipError {
    /// The reason, in Japanese for the message `STR_PLGMGR_INST_ZIP_ERR`.
    pub fn text(&self) -> String {
        match self {
            ZipError::Unreadable(why) => format!("ZIPファイルにアクセス出来ません（{why}）"),
            ZipError::NotZip => "ZIPファイルとして読めません".into(),
            ZipError::Unsupported(what) => format!("{what}のZIPファイルは扱えません"),
            ZipError::ArchiveTooLarge => format!(
                "ZIPファイルが大きすぎます（{} MiB まで）",
                MAX_ARCHIVE_BYTES / 1024 / 1024
            ),
            ZipError::TooManyEntries => {
                format!("ZIPファイルの中のファイルが多すぎます（{MAX_ENTRIES} 個まで）")
            }
            ZipError::TooLarge => format!(
                "ZIPファイルの中身が大きすぎます（1 ファイル {} MiB、全体で {} MiB まで）",
                MAX_FILE_BYTES / 1024 / 1024,
                MAX_TOTAL_BYTES / 1024 / 1024
            ),
            ZipError::NoDefinition => "プラグイン定義ファイル（plugin.def）がありません".into(),
            ZipError::NotOneFolder => {
                "ZIPファイルの一番上には、プラグインのフォルダー 1 つだけを置いてください".into()
            }
            ZipError::BadFolderName => {
                format!("プラグインのフォルダー名は{ID_MAX_CHARS}文字までで、. で始められません")
            }
            ZipError::UnsafePath(name) => format!("使えないパスがあります：{name}"),
            ZipError::SpecialFile(name) => {
                format!("リンクやデバイスのファイルは含められません：{name}")
            }
            ZipError::Duplicate(name) => format!("同じパスが 2 つあります：{name}"),
            ZipError::Corrupt(name) => format!("ZIPファイルの中身が壊れています：{name}"),
            ZipError::Write(why) => format!("解凍に失敗しました（{why}）"),
            ZipError::Id(error) => match error {
                InstallError::NoDefinition => {
                    "プラグイン定義ファイル（plugin.def）がありません".into()
                }
                InstallError::NoId => "Plugin.IDがありません".into(),
                InstallError::ReservedChar(_) => "Plugin.IDに使用できない文字があります".into(),
                InstallError::IdStartsWithDigit => "Plugin.IDの先頭に数字は使用できません".into(),
                InstallError::IdTooLong => format!("Plugin.IDは{ID_MAX_CHARS}文字までです"),
                other => format!("{other:?}"),
            },
        }
    }
}

/// An entry of the archive that is checked and will be written.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    /// The parts of the path, the first one is the folder of the plugin (spelled as the first entry
    /// of the archive spells it).
    parts: Vec<String>,
    is_dir: bool,
    /// 0: stored, 8: deflated.
    method: u16,
    crc: u32,
    compressed: u64,
    size: u64,
    /// Where the local header starts.
    offset: u64,
}

/// A ZIP file that has been checked: it is one plugin.
#[derive(Debug)]
pub struct ZipPlugin {
    bytes: Vec<u8>,
    entries: Vec<Entry>,
    /// The name of the folder that the plugin will have in `plugins\`.
    pub folder: String,
    /// `Plugin.Id` of its `plugin.def`.
    pub id: String,
}

fn le16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const EOCD_SIZE: usize = 22;

/// The characters Windows does not allow in a name (and `:`, which starts a stream there).
const BAD_NAME_CHARS: &str = "<>:\"|?*";

/// Names that Windows takes for devices, with or without an extension.
fn is_device_name(part: &str) -> bool {
    let stem = part
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end()
        .to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ((stem.starts_with("COM") || stem.starts_with("LPT"))
        && stem.len() == 4
        && stem.as_bytes()[3].is_ascii_digit())
}

/// Checks one part of a path.
fn part_is_good(part: &str) -> bool {
    !part.is_empty()
        && part != "."
        && part != ".."
        && part.chars().count() <= MAX_NAME_CHARS
        && !part.ends_with(['.', ' '])
        && !part
            .chars()
            .any(|c| c.is_control() || BAD_NAME_CHARS.contains(c))
        && !is_device_name(part)
}

/// The name of an entry as text: UTF-8 (flag bit 11 or valid UTF-8), else Shift_JIS, which Japanese
/// Windows tools write.
fn decode_name(raw: &[u8], utf8_flag: bool) -> Option<String> {
    if let Ok(text) = std::str::from_utf8(raw) {
        return Some(text.to_owned());
    }
    if utf8_flag {
        return None;
    }
    let (text, _, errors) = encoding_rs::SHIFT_JIS.decode(raw);
    (!errors).then(|| text.into_owned())
}

/// The parts of the path of an entry, or the reason it is refused. `dir` is whether the name ends
/// with a slash.
fn split_path(name: &str) -> Result<(Vec<String>, bool), ZipError> {
    let unsafe_path = || ZipError::UnsafePath(name.to_owned());
    // Windows tools write `\` in the old ones; both separate here.
    let text = name.replace('\\', "/");
    if text.starts_with('/') {
        return Err(unsafe_path());
    }
    let is_dir = text.ends_with('/');
    let body = text.trim_end_matches('/');
    if body.is_empty() {
        return Err(unsafe_path());
    }
    let mut parts = Vec::new();
    for part in body.split('/') {
        // `./a` is what some tools write for `a`.
        if part == "." {
            continue;
        }
        if !part_is_good(part) {
            return Err(unsafe_path());
        }
        parts.push(part.to_owned());
    }
    if parts.is_empty() || parts.len() > MAX_DEPTH {
        return Err(unsafe_path());
    }
    Ok((parts, is_dir))
}

/// A link, device or other special file by the attributes the archive keeps.
fn is_special(made_by_host: u8, external_attributes: u32) -> bool {
    match made_by_host {
        // Unix: the mode is in the high 16 bits; only files and folders are good.
        3 => {
            let kind = (external_attributes >> 16) & 0o170000;
            kind != 0 && kind != 0o100000 && kind != 0o040000
        }
        // MS-DOS and Windows: `FILE_ATTRIBUTE_REPARSE_POINT` (0x400).
        0 | 10 => external_attributes & 0x400 != 0,
        _ => false,
    }
}

impl ZipPlugin {
    /// Reads the ZIP file at `path` and checks it (`ChkPluginDef`, and `InstallPlugin`'s checks of the
    /// ID). Nothing is written.
    pub fn inspect(path: &Path) -> Result<Self, ZipError> {
        let meta = std::fs::metadata(path).map_err(|e| ZipError::Unreadable(e.to_string()))?;
        if meta.len() > MAX_ARCHIVE_BYTES {
            return Err(ZipError::ArchiveTooLarge);
        }
        let bytes = std::fs::read(path).map_err(|e| ZipError::Unreadable(e.to_string()))?;
        Self::from_bytes(bytes)
    }

    /// [`inspect`](Self::inspect) for the bytes of a ZIP file.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, ZipError> {
        if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
            return Err(ZipError::ArchiveTooLarge);
        }
        let entries = Self::read_entries(&bytes)?;
        let folder = entries
            .iter()
            .map(|e| e.parts[0].clone())
            .next()
            .ok_or(ZipError::NoDefinition)?;
        if folder.chars().count() > ID_MAX_CHARS || folder.starts_with('.') {
            return Err(ZipError::BadFolderName);
        }
        let mut seen = std::collections::HashSet::new();
        let mut entries = entries;
        for entry in &mut entries {
            if !same_name(&entry.parts[0], &folder) || (entry.parts.len() == 1 && !entry.is_dir) {
                return Err(ZipError::NotOneFolder);
            }
            entry.parts[0] = folder.clone();
            // A folder entry and a file with the same path are two things of the same name too.
            if !seen.insert(entry.parts.join("/").to_lowercase()) {
                return Err(ZipError::Duplicate(entry.parts.join("/")));
            }
        }
        // A path that is a file cannot have children.
        let files: std::collections::HashSet<String> = entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.parts.join("/").to_lowercase())
            .collect();
        for entry in &entries {
            for end in 1..entry.parts.len() {
                let parent = entry.parts[..end].join("/");
                if files.contains(&parent.to_lowercase()) {
                    return Err(ZipError::Duplicate(parent));
                }
            }
        }
        let def_index = entries
            .iter()
            .position(|e| {
                !e.is_dir
                    && e.parts.len() == 2
                    && e.parts[1].eq_ignore_ascii_case(super::DEF_FILE_NAME)
            })
            .ok_or(ZipError::NoDefinition)?;
        let mut plugin = ZipPlugin {
            bytes,
            entries,
            folder,
            id: String::new(),
        };
        let mut def = Vec::new();
        plugin.inflate(&plugin.entries[def_index], &mut def)?;
        let id = Profile::from_bytes(&def)
            .get("Plugin", "Id")
            .filter(|id| !id.is_empty())
            .ok_or(ZipError::Id(InstallError::NoId))?
            .to_owned();
        check_id(&id).map_err(ZipError::Id)?;
        plugin.id = id;
        Ok(plugin)
    }

    /// The central directory: the entries, checked one by one.
    fn read_entries(bytes: &[u8]) -> Result<Vec<Entry>, ZipError> {
        // The end record is the last 22 bytes, or before a comment of up to 65535 bytes.
        let earliest = bytes
            .len()
            .saturating_sub(EOCD_SIZE + usize::from(u16::MAX));
        let eocd = (earliest..=bytes.len().saturating_sub(EOCD_SIZE))
            .rev()
            .find(|&at| le32(bytes, at) == Some(EOCD_SIGNATURE))
            .ok_or(ZipError::NotZip)?;
        let disk = le16(bytes, eocd + 4).ok_or(ZipError::NotZip)?;
        let cd_disk = le16(bytes, eocd + 6).ok_or(ZipError::NotZip)?;
        let on_disk = le16(bytes, eocd + 8).ok_or(ZipError::NotZip)?;
        let total = le16(bytes, eocd + 10).ok_or(ZipError::NotZip)?;
        let cd_size = le32(bytes, eocd + 12).ok_or(ZipError::NotZip)?;
        let cd_offset = le32(bytes, eocd + 16).ok_or(ZipError::NotZip)?;
        if total == u16::MAX || cd_size == u32::MAX || cd_offset == u32::MAX {
            return Err(ZipError::Unsupported("ZIP64"));
        }
        if disk != 0 || cd_disk != 0 || on_disk != total {
            return Err(ZipError::Unsupported("複数に分かれた"));
        }
        let total = usize::from(total);
        if total > MAX_ENTRIES {
            return Err(ZipError::TooManyEntries);
        }
        let mut at = cd_offset as usize;
        let mut entries = Vec::with_capacity(total);
        let mut sum: u64 = 0;
        for _ in 0..total {
            if le32(bytes, at) != Some(CENTRAL_SIGNATURE) {
                return Err(ZipError::NotZip);
            }
            let made_by = le16(bytes, at + 4).ok_or(ZipError::NotZip)?;
            let flags = le16(bytes, at + 8).ok_or(ZipError::NotZip)?;
            let method = le16(bytes, at + 10).ok_or(ZipError::NotZip)?;
            let crc = le32(bytes, at + 16).ok_or(ZipError::NotZip)?;
            let compressed = le32(bytes, at + 20).ok_or(ZipError::NotZip)?;
            let size = le32(bytes, at + 24).ok_or(ZipError::NotZip)?;
            let name_len = usize::from(le16(bytes, at + 28).ok_or(ZipError::NotZip)?);
            let extra_len = usize::from(le16(bytes, at + 30).ok_or(ZipError::NotZip)?);
            let comment_len = usize::from(le16(bytes, at + 32).ok_or(ZipError::NotZip)?);
            let attributes = le32(bytes, at + 38).ok_or(ZipError::NotZip)?;
            let offset = le32(bytes, at + 42).ok_or(ZipError::NotZip)?;
            let raw = bytes
                .get(at + 46..at + 46 + name_len)
                .ok_or(ZipError::NotZip)?;
            at += 46 + name_len + extra_len + comment_len;
            if at > bytes.len() {
                return Err(ZipError::NotZip);
            }
            if compressed == u32::MAX || size == u32::MAX || offset == u32::MAX {
                return Err(ZipError::Unsupported("ZIP64"));
            }
            let name = decode_name(raw, flags & 0x800 != 0)
                .ok_or_else(|| ZipError::UnsafePath(String::from_utf8_lossy(raw).into_owned()))?;
            let (parts, is_dir) = split_path(&name)?;
            if is_special((made_by >> 8) as u8, attributes) {
                return Err(ZipError::SpecialFile(name));
            }
            if flags & 1 != 0 || flags & 0x40 != 0 {
                return Err(ZipError::Unsupported("暗号化された"));
            }
            if !is_dir && !matches!(method, 0 | 8) {
                return Err(ZipError::Unsupported("圧縮方式に対応していない"));
            }
            if u64::from(size) > MAX_FILE_BYTES {
                return Err(ZipError::TooLarge);
            }
            sum += u64::from(size);
            if sum > MAX_TOTAL_BYTES {
                return Err(ZipError::TooLarge);
            }
            entries.push(Entry {
                parts,
                is_dir,
                method,
                crc,
                compressed: u64::from(compressed),
                size: u64::from(size),
                offset: u64::from(offset),
            });
        }
        Ok(entries)
    }

    /// The compressed bytes of an entry (after its local header).
    fn raw(&self, entry: &Entry) -> Result<&[u8], ZipError> {
        let corrupt = || ZipError::Corrupt(entry.parts.join("/"));
        let at = usize::try_from(entry.offset).map_err(|_| corrupt())?;
        if le32(&self.bytes, at) != Some(LOCAL_SIGNATURE) {
            return Err(corrupt());
        }
        let name_len = usize::from(le16(&self.bytes, at + 26).ok_or_else(corrupt)?);
        let extra_len = usize::from(le16(&self.bytes, at + 28).ok_or_else(corrupt)?);
        let start = at + 30 + name_len + extra_len;
        let end = start
            .checked_add(usize::try_from(entry.compressed).map_err(|_| corrupt())?)
            .ok_or_else(corrupt)?;
        self.bytes.get(start..end).ok_or_else(corrupt)
    }

    /// Inflates a file into `out`, checking the size and the CRC that the archive declares. At most
    /// the declared size plus one byte is read, so a stream that lies cannot grow.
    fn inflate(&self, entry: &Entry, out: &mut impl std::io::Write) -> Result<(), ZipError> {
        let corrupt = || ZipError::Corrupt(entry.parts.join("/"));
        let raw = self.raw(entry)?;
        let mut reader: Box<dyn std::io::Read + '_> = match entry.method {
            0 => Box::new(raw),
            _ => Box::new(flate2::read::DeflateDecoder::new(raw)),
        };
        let mut limited = (&mut reader).take(entry.size + 1);
        let mut crc = Crc32::new();
        let mut written: u64 = 0;
        let mut buffer = [0u8; 16 * 1024];
        loop {
            let n = limited.read(&mut buffer).map_err(|_| corrupt())?;
            if n == 0 {
                break;
            }
            crc.update(&buffer[..n]);
            written += n as u64;
            out.write_all(&buffer[..n])
                .map_err(|e| ZipError::Write(e.to_string()))?;
        }
        if written != entry.size || crc.finish() != entry.crc {
            return Err(corrupt());
        }
        Ok(())
    }

    /// Unpacks the plugin into `plugins_dir` (`Unzip`): `plugins_dir/<folder>/...`. The folder is
    /// made if it is not there. Files that exist are replaced; files the archive does not have are
    /// left. All files are written to a temporary folder first; when one fails, `plugins_dir` is as
    /// it was.
    pub fn extract(&self, plugins_dir: &Path) -> Result<(), ZipError> {
        let write = |e: std::io::Error| ZipError::Write(e.to_string());
        std::fs::create_dir_all(plugins_dir).map_err(write)?;
        let temp = unused_temp_dir(plugins_dir)?;
        let result = self.extract_to(&temp).and_then(|()| {
            let source = temp.join(&self.folder);
            let target = plugins_dir.join(&self.folder);
            move_tree(&source, &target)
        });
        let _ = std::fs::remove_dir_all(&temp);
        result
    }

    fn extract_to(&self, temp: &Path) -> Result<(), ZipError> {
        let write = |e: std::io::Error| ZipError::Write(e.to_string());
        for entry in &self.entries {
            let path = entry
                .parts
                .iter()
                .fold(temp.to_path_buf(), |p, part| p.join(part));
            if entry.is_dir {
                std::fs::create_dir_all(&path).map_err(write)?;
                continue;
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(write)?;
            }
            // `create_new`: never writes through a file or link that is there already.
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(write)?;
            self.inflate(entry, &mut file)?;
        }
        // The folder exists also when the archive has files only.
        std::fs::create_dir_all(temp.join(&self.folder)).map_err(write)
    }
}

/// A new, empty folder next to the plugins, for unpacking.
fn unused_temp_dir(plugins_dir: &Path) -> Result<PathBuf, ZipError> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    for n in 0..100u32 {
        let path = plugins_dir.join(format!(".matsu-unzip-{}-{stamp}-{n}", std::process::id()));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(ZipError::Write(e.to_string())),
        }
    }
    Err(ZipError::Write("temporary folder".into()))
}

/// Moves the files of `source` into `target`, replacing files that are there. A file where a folder
/// should be (or the reverse) is an error, and so is a link: nothing is written through one.
fn move_tree(source: &Path, target: &Path) -> Result<(), ZipError> {
    let write = |e: std::io::Error| ZipError::Write(e.to_string());
    match std::fs::symlink_metadata(target) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            return Err(ZipError::Write(format!(
                "{} is not a folder",
                target.display()
            )));
        }
        Err(_) => std::fs::create_dir_all(target).map_err(write)?,
    }
    for entry in std::fs::read_dir(source).map_err(write)? {
        let entry = entry.map_err(write)?;
        let to = target.join(entry.file_name());
        if entry.file_type().map_err(write)?.is_dir() {
            move_tree(&entry.path(), &to)?;
        } else {
            if let Ok(meta) = std::fs::symlink_metadata(&to)
                && meta.is_dir()
            {
                return Err(ZipError::Write(format!("{} is a folder", to.display())));
            }
            std::fs::rename(entry.path(), &to).map_err(write)?;
        }
    }
    Ok(())
}

/// CRC-32 (the one the ZIP format uses).
struct Crc32(u32);

const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

impl Crc32 {
    fn new() -> Self {
        Crc32(!0)
    }
    fn update(&mut self, data: &[u8]) {
        for &b in data {
            self.0 = CRC_TABLE[usize::from((self.0 as u8) ^ b)] ^ (self.0 >> 8);
        }
    }
    fn finish(&self) -> u32 {
        !self.0
    }
}

#[cfg(test)]
mod tests;
