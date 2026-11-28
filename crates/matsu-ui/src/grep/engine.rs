// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/agent/CGrepAgent.cpp (DoGrep, DoGrepTree, DoGrepFile, CreateFolders, ChopYen,
//   AllFoldersExist)
//
// sakura_core/agent/CGrepAgent.cpp:
//   Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the file above and was modified: the C++ code (the Win32 windows and
// messages, the cancel dialog, `CFileLoad`, `CBregexp` and the UTF-16 buffers) was rewritten in
// Rust on `matsu_core::file`, `matsu_core::search::Matcher` and `String`, the search does not touch the
// window (it writes to a sink and reads a flag to stop), and the tests are new.
//
// zlib License
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

//! Grep without a window: goes through the folders and the files, reads each file line by line,
//! and writes the result to a [`Sink`] in Sakura's form (the `format` module).
//!
//! Sakura: `CGrepAgent::DoGrep` (`sakura_core/agent/CGrepAgent.cpp`). The search runs off the UI
//! thread; it reads the flag `cancel` between files and every 32 lines, which is the stop button
//! of Sakura's progress dialog.
//!
//! Each file is read with [`matsu_core::file::open_file_with`], one file at a time (the bytes of
//! one file are in memory, the lines are decoded when they are looked at). A file of
//! [`GrepInfo::huge_file_len`] bytes or more is not read whole: its lines are read in order, a
//! chunk at a time, without an index (ADR-0020, chapter 5, [`matsu_core::file::LineReader`]).
//! The line number is counted as the lines are read. The matching is the
//! matcher of the search panel (`build_matcher` in `crate::search::state`), also for
//! the regular expression and the words. Sakura does not skip binary files, nor do we.
//!
//! Grep 置換 is the same search with [`GrepInfo::replace`] set: the files are rewritten by the
//! `replace` module, through the save of `matsu_core::file`.
//!
//! 「この編集中のテキストから検索する」 (`:HWND:`) searches the text of a window instead of the files
//! ([`GrepInfo::window`]): `GetHwndTitle` and the `hWndTarget` branch of `DoGrep`, and
//! `CFileLoadOrWnd`, which reads the lines of the window. The window is a snapshot of its
//! document taken when the search starts, read by lines like a file, so the unsaved changes and
//! a document with no file are searched. A document of a huge file reads each line from its
//! block index, not the whole text; while its scan is still running the snapshot has only the
//! scanned start, so the file behind it is read in order instead ([`WindowSource::scanning`]).
//!
//! Differences from Sakura: the output to the standard output (`-GOPT=U`) is in [`super::stdout`];
//! a Grep 置換 of the text of a window replaces in the document (Sakura only searches, and then
//! says that it replaced: `DoGrep` calls `DoGrepFile`, never `DoGrepReplaceFile`, for
//! `hWndTarget`; ADR-0018 type A), see `replace_window`. Sakura reads the window line by line while
//! it searches and never checks whether the text changed; here the edits are planned on the
//! snapshot, so a document that changed meanwhile gets none of them. A window that is gone is told in the
//! result, as Sakura does (`GetHwndTitle` is -1 in `DoGrep`: "HWND handle error." follows the
//! conditions of the heading and nothing else is written).

use std::borrow::Cow;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use matsu_config::Profile;
use matsu_config::settings::DocumentTypes;
use matsu_core::buffer::{BlockIndex, Document};
use matsu_core::encoding::{DETECT_SAMPLE_LEN, DetectOptions, Encoding, detect};
use matsu_core::file::{
    DEFAULT_STREAM_CHUNK, FileError, Limited, LineReader, LineStream, OpenOptions, TextFile,
    open_file_with, open_line_stream,
};
use matsu_core::model::eol_text;
use matsu_core::search::ReplaceTarget;
use matsu_core::search::{AppliedEdit, Match, Matcher};
use matsu_regex::{
    PatternError, RegexMatcher, RegexReplacer, RegexSearchOptions, ReplacementError,
};

use super::enumerate::{Walker, base_prefix_len, join_path};
use super::format::{
    self, Body, EOL, FilePaths, HeaderConditions, Layout, LineType, OutputStyle, PathInfoState,
};
use super::keys::{GrepKeys, KeysError, split_folders};
use crate::search::state::{SearchOptions, build_matcher};

mod replace;

/// The version text that the heading shows for the regular expression (`GetVersionW` of
/// bregonig in Sakura).
pub const REGEX_VERSION: &str = "Onigmo";

/// The output is sent on when it has this many bytes, or ten hits and 50 ms have passed
/// (`ADDTAIL_INTERVAL_MILLISEC`).
const FLUSH_BYTES: usize = 32 * 1024;
const FLUSH_INTERVAL: Duration = Duration::from_millis(50);
/// The flag is read every this many lines.
const CANCEL_CHECK_LINES: usize = 32;

/// What one Grep needs (`GrepInfo` in `sakura_core/basis/GrepInfo.h`, without the parts of the
/// standard output).
#[derive(Clone, Debug)]
pub struct GrepInfo {
    /// The pattern (`cmGrepKey`). Empty searches the names of the files only ("ファイル検索").
    pub key: String,
    /// The file patterns with the excluded files and folders packed in, `file;#folder;!file`
    /// (`cmGrepFile`, see [`GrepSettings::packed_files`](super::settings::GrepSettings::packed_files)).
    pub file: String,
    /// The folders, separated by `;` (`cmGrepFolder`).
    pub folder: String,
    /// 英大文字と小文字を区別 / 単語単位 / 正規表現. The other fields of the options are not
    /// used by Grep.
    pub options: SearchOptions,
    /// サブフォルダーからも検索 (`bGrepSubFolder`).
    pub sub_folder: bool,
    /// The encoding of the files, or `None` to detect it (`nGrepCharSet`).
    pub charset: Option<Encoding>,
    pub layout: Layout,
    /// ファイル毎最初のみ検索 (`bGrepOutputFileOnly`).
    pub file_only: bool,
    /// 除外ファイルは正規表現 (`bGrepExceptFileRegexp`).
    pub except_file_regex: bool,
    /// Grep 置換: what to replace the hits with (`bGrepReplace`).
    pub replace: Option<ReplaceInfo>,
    /// Decode the MIME encoded words of ISO-2022-JP files (`bAutoMIMEdecode`: Sakura gives the
    /// common setting to `CFileLoad::FileOpen` in `CGrepAgent::DoGrep`). Not in the dialog: the
    /// window sets it from the common settings when the search starts.
    pub mime_decode: bool,
    /// A file of this many bytes or more is read in order, a chunk at a time, instead of whole
    /// (the size from which the editor opens a file by ranges, `nHugeFileSizeMB`; ADR-0020,
    /// chapter 5). `None` reads every file whole. Not in the dialog: the window sets it from the
    /// common settings when the search starts.
    pub huge_file_len: Option<u64>,
    /// The text of the window to search instead of the files (`:HWND:` in `cmGrepFile`). The
    /// window, which only the UI can find, puts it here when the search starts.
    pub window: Option<WindowSource>,
    /// The file box names a window that is not there (`GetHwndTitle` returned -1): the result
    /// says so and nothing is searched.
    pub window_gone: bool,
    /// The document types, to detect the encoding of each file with the settings of the type that
    /// its name chooses (`CGrepAgent::DoGrepFile`: `CCodeMediator( type->m_encoding )`). Not in
    /// the dialog: the window sets it from the settings when the search starts. `None` detects
    /// with the defaults (UTF-8, CESU-8 not preferred).
    pub type_detect: Option<TypeDetect>,
}

/// The document types and the settings that decide how the encoding of a file is detected: the
/// type of a file is found by its name (`GetDocumentTypeOfPath`), and the type gives the default
/// encoding and whether CESU-8 is preferred (`m_eDefaultCodetype`, `m_bPriorCesu8`).
#[derive(Clone, Debug)]
pub struct TypeDetect {
    types: Arc<DocumentTypes>,
    profile: Arc<Profile>,
}

impl TypeDetect {
    pub fn new(types: Arc<DocumentTypes>, profile: Profile) -> Self {
        Self {
            types,
            profile: Arc::new(profile),
        }
    }

    /// The detection settings for a file called `file_name` (the name only, no folder).
    fn options_for(&self, file_name: &str) -> DetectOptions {
        let index = self.types.type_index_for_file_name(file_name);
        crate::settings::NewDocumentDefaults::for_type(&self.profile, index).detect_options()
    }
}

/// The text of a window that a search of 「この編集中のテキスト」 reads (`CFileLoadOrWnd`).
#[derive(Clone, Debug)]
pub struct WindowSource {
    /// The id of the document (the number in `:HWND:[id]`): the window that owns it makes the
    /// edits of a Grep 置換.
    pub id: u64,
    /// The version of the document at the snapshot: edits are made only while it is the same.
    pub version: u64,
    /// The document as it was when the search started (cloning is cheap: it shares the text).
    pub document: Document,
    /// The encoding of the document, for the name in front of a hit.
    pub encoding: Encoding,
    /// The folder of its file, or empty for a document that has none (`szWindowPath`).
    pub folder: String,
    /// The name of its file, or `:HWND:[id]name` when it has none, which is what a tag jump
    /// from the result finds the window by (`szWindowName`).
    pub name: String,
    /// What the heading shows in `Window:[...]`.
    pub title: String,
    /// Set while the document still shows only the scanned start of its huge file (the scan of
    /// the line endings is running): the index of the scan. The search reads the file behind it
    /// in order, through the same open file as the document ([`BlockIndex::source`]), instead of
    /// the snapshot, which would miss the rest. The text is the document's even if another
    /// program changes the file meanwhile (that is an error of the read).
    pub scanning: Option<BlockIndex>,
}

/// The parts of a Grep 置換 that a Grep does not have (`cmGrepRep`, `bGrepPaste`, `bGrepBackup`).
#[derive(Clone, Debug)]
pub struct ReplaceInfo {
    /// The new text. For a regular expression it is bregonig's replacement (`$1`, `\n`...); with
    /// `paste` it is the text of the clipboard, which is used as it is.
    pub text: String,
    /// クリップボードから貼り付け: `text` came from the clipboard and is not read as a
    /// replacement of a regular expression.
    pub paste: bool,
    /// バックアップ作成: the old file is kept as `name.skrold`.
    pub backup: bool,
}

impl GrepInfo {
    /// `GrepInfo::Normalized` (`sakura_core/basis/GrepInfo.cpp`): "一致しなかった行を出力" does
    /// not hold for a replacement, and shows the lines of the hits.
    fn normalized(&self) -> Cow<'_, GrepInfo> {
        if self.replace.is_some() && self.layout.line_type == LineType::NoHitLine {
            let mut info = self.clone();
            info.layout.line_type = LineType::Line;
            Cow::Owned(info)
        } else {
            Cow::Borrowed(self)
        }
    }
}

/// Why a Grep did not start. Sakura says it in a message box and writes nothing to the result.
#[derive(Debug)]
pub enum GrepError {
    /// The pattern is not a regular expression (`CheckRegexpSyntax`).
    Pattern(PatternError),
    /// The file patterns are wrong (`STR_GREP_ERR_ENUMKEYS1`, `STR_GREP_ERR_ENUMKEYS2`).
    Keys(KeysError),
    /// An excluded file expression does not compile (`STR_GREP_ERR_EXCLUDE_REGEXP`).
    ExceptRegex { pattern: String, reason: String },
    /// A folder is not there (`STR_DLGGREP5`).
    FolderMissing,
    /// A replacement without a pattern (`STR_DLGREPLC_REPSTR`).
    NoPattern,
    /// The replacement of a regular expression is wrong (`SetPattern` of the replacement).
    Replacement(ReplacementError),
    /// A Grep 置換 of the text of a window whose huge file is still being scanned: the snapshot
    /// has only the start of the file.
    WindowScanning,
}

impl GrepError {
    /// The text of the message box.
    pub fn message(&self) -> String {
        match self {
            GrepError::Pattern(e) => crate::search::state::regex_error_message(e),
            GrepError::Keys(e) => e.message().to_owned(),
            GrepError::ExceptRegex { pattern, reason } => {
                format!("除外ファイルの正規表現が正しくありません。\n{pattern}\n{reason}")
            }
            GrepError::FolderMissing => "検索対象フォルダーが正しくありません。".to_owned(),
            GrepError::NoPattern => "置換条件を指定してください。".to_owned(),
            GrepError::Replacement(e) => crate::search::state::regex_error_message(e),
            GrepError::WindowScanning => {
                "巨大なファイルを読み込み中の編集中のテキストは、まだGrep置換できません。"
                    .to_owned()
            }
        }
    }
}

/// How a Grep ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// The number of hits (of the files, for a search of the names).
    pub hits: usize,
    /// It was stopped.
    pub stopped: bool,
}

/// What one match did to the text of a document, in bytes of the text of its lines and their
/// breaks: the `old_len` bytes at `at` became `new_len`. A caret is moved through them by
/// arithmetic, and then put back into the document that the edits made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextShift {
    pub at: usize,
    pub old_len: usize,
    pub new_len: usize,
}

/// Where the result goes, and what shows the progress.
pub trait Sink {
    /// A piece of the result, to be appended to the text.
    fn write(&mut self, text: &str);
    /// The file that is being searched and the hits so far (the cancel dialog of Sakura).
    fn progress(&mut self, _file: &str, _hits: usize) {}
    /// The edits that a Grep 置換 of the text of a window makes, for the window that owns the
    /// document to make (the engine only plans them, on a snapshot). It does not wait for the
    /// answer: the window tells at the count (see [`Sink::count`]).
    fn replaced(&mut self, _edits: Vec<AppliedEdit>, _shifts: Vec<TextShift>) {}
    /// The last line: the number of hits, or of replacements. A sink whose window may have
    /// refused the edits of [`Sink::replaced`] says what was really made.
    fn count(&mut self, hits: usize, replace: bool) {
        self.write(&if replace {
            format::replace_count_line(hits)
        } else {
            format::count_line(hits)
        });
    }
}

/// `ChopYen`: the last backslash of a folder is taken off (`C:\a\` is `C:\a`). A slash is taken
/// off too, except for the root.
pub fn chop_yen(folder: &str) -> String {
    if let Some(rest) = folder.strip_suffix('\\') {
        return rest.to_owned();
    }
    if folder.len() > 1
        && let Some(rest) = folder.strip_suffix('/')
    {
        return rest.to_owned();
    }
    folder.to_owned()
}

/// [`chop_yen`] for a folder that `GetLongFileName` made absolute: a drive root stays `C:\`.
/// `C:` alone is the current folder of that drive, so the walk would canonicalize it to another
/// folder (searching `..` from `C:\work` would skip the real `C:\work` as a cycle). A UNC root
/// (`\\server\share\`) is the same path without its last separator, so it is chopped as usual.
pub(super) fn chop_yen_keeping_root(folder: &str) -> String {
    let chopped = chop_yen(folder);
    let mut chars = chopped.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(drive), Some(':'), None) if drive.is_ascii_alphabetic() => format!("{chopped}\\"),
        _ => chopped,
    }
}

/// Stopped by the user.
struct Stopped;

/// Why the next line could not be had from a file or a document.
enum ReadStop {
    /// It could not be read.
    Failed,
    /// The user stopped the search while the line was being read.
    Cancelled,
}

/// The longest line, in bytes of the file, that a search of a file read by ranges looks at
/// (ADR-0020, chapter 5; type C). Matching needs the whole line as one string, and the hit writes it
/// (cut at `nMaxOutStr`, [`format`]), so a line is held about four times: as it was read, decoded,
/// with its ending, and in the text of the result. A longer line is not read: the result says
/// so, and the search goes on with the next line.
pub(super) const LONG_LINE_LIMIT: usize = 64 * 1024 * 1024;

/// The bytes one read of a big copy of a file asks for (the backup, the start of the new file):
/// the stop is looked at between the reads.
const COPY_CHUNK: usize = 1024 * 1024;

fn copy_chunk() -> usize {
    #[cfg(test)]
    return stream_probe::copy_chunk();
    #[cfg(not(test))]
    COPY_CHUNK
}

/// Called after each chunk of a big copy (the tests stop the search from it).
fn copied_chunk() {
    #[cfg(test)]
    stream_probe::copied_chunk();
}

fn line_limit() -> usize {
    #[cfg(test)]
    return stream_probe::line_limit();
    #[cfg(not(test))]
    LONG_LINE_LIMIT
}

/// The bytes one read of a file that is read in order asks for.
fn stream_chunk() -> usize {
    #[cfg(test)]
    return stream_probe::chunk();
    #[cfg(not(test))]
    DEFAULT_STREAM_CHUNK
}

/// What the tests see of the files that were read in order, on this thread: the size of a read,
/// and what the readers held and read.
#[cfg(test)]
pub(super) mod stream_probe {
    use std::cell::{Cell, RefCell};

    use super::{COPY_CHUNK, DEFAULT_STREAM_CHUNK, LONG_LINE_LIMIT};

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Seen {
        /// The files read in order.
        pub files: usize,
        /// The most bytes one reader held at once.
        pub peak: usize,
        /// The bytes read from the files in all.
        pub bytes: u64,
    }

    thread_local! {
        static CHUNK: Cell<usize> = const { Cell::new(DEFAULT_STREAM_CHUNK) };
        static LINE_LIMIT: Cell<usize> = const { Cell::new(LONG_LINE_LIMIT) };
        static COPY: Cell<usize> = const { Cell::new(COPY_CHUNK) };
        #[allow(clippy::type_complexity)]
        static COPY_HOOK: RefCell<Option<Box<dyn FnMut()>>> = const { RefCell::new(None) };
        static SEEN: Cell<Seen> = const { Cell::new(Seen { files: 0, peak: 0, bytes: 0 }) };
    }

    /// The files that were read whole, by every thread (the search of the app runs on another
    /// one). The paths are the real ones (`canonicalize`): a folder has more than one spelling
    /// (a Windows `TEMP` can be an 8.3 short path, and the search is given the long one), so
    /// text compared as it is would never match.
    static WHOLE: std::sync::Mutex<Vec<(std::path::PathBuf, u64)>> =
        std::sync::Mutex::new(Vec::new());

    pub fn whole_read(path: &str) {
        let len = std::fs::metadata(path).map_or(0, |m| m.len());
        let real = std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
        WHOLE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((real, len));
    }

    /// The files of `dir` that were read whole, and their sizes. `dir` may be spelled in any
    /// way that names the folder.
    pub fn whole_reads_in(dir: &std::path::Path) -> Vec<(String, u64)> {
        let dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
        WHOLE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|(p, _)| p.starts_with(&dir))
            .map(|(p, len)| (p.to_string_lossy().into_owned(), *len))
            .collect()
    }

    pub fn chunk() -> usize {
        CHUNK.with(Cell::get)
    }

    pub fn line_limit() -> usize {
        LINE_LIMIT.with(Cell::get)
    }

    pub fn copy_chunk() -> usize {
        COPY.with(Cell::get)
    }

    pub fn copied_chunk() {
        // taken out while it runs, so that a hook cannot start itself
        if let Some(mut hook) = COPY_HOOK.with(|h| h.borrow_mut().take()) {
            hook();
            COPY_HOOK.with(|h| *h.borrow_mut() = Some(hook));
        }
    }

    /// Runs `f` with big copies of `chunk` bytes, calling `hook` after each chunk.
    pub fn with_copy_hook<R>(
        chunk: usize,
        hook: impl FnMut() + 'static,
        f: impl FnOnce() -> R,
    ) -> R {
        struct Restore;
        impl Drop for Restore {
            fn drop(&mut self) {
                COPY.with(|c| c.set(COPY_CHUNK));
                COPY_HOOK.with(|h| *h.borrow_mut() = None);
            }
        }
        let _restore = Restore;
        COPY.with(|c| c.set(chunk));
        COPY_HOOK.with(|h| *h.borrow_mut() = Some(Box::new(hook)));
        f()
    }

    /// Runs `f` with `limit` as the longest line that is looked at.
    pub fn with_line_limit<R>(limit: usize, f: impl FnOnce() -> R) -> R {
        // The old limit comes back also when `f` panics.
        struct Restore(usize);
        impl Drop for Restore {
            fn drop(&mut self) {
                LINE_LIMIT.with(|l| l.set(self.0));
            }
        }
        let _restore = Restore(LINE_LIMIT.with(|l| l.replace(limit)));
        f()
    }

    /// Runs `f` with reads of `chunk` bytes, and tells what the streams saw.
    pub fn with_chunk<R>(chunk: usize, f: impl FnOnce() -> R) -> (R, Seen) {
        let before = CHUNK.with(|c| c.replace(chunk));
        SEEN.with(|s| s.set(Seen::default()));
        let result = f();
        CHUNK.with(|c| c.set(before));
        (result, SEEN.with(Cell::get))
    }

    pub fn record(reader: &matsu_core::file::LineReader) {
        SEEN.with(|s| {
            let mut seen = s.get();
            seen.files += 1;
            seen.peak = seen.peak.max(reader.peak_buffered());
            seen.bytes += reader.bytes_read();
            s.set(seen);
        });
    }
}

struct Runner<'a> {
    info: &'a GrepInfo,
    matcher: Option<Arc<dyn Matcher>>,
    /// The replacement of a regular expression (not for a paste).
    replacer: Option<RegexReplacer>,
    walker: Walker,
    cancel: &'a AtomicBool,
    sink: &'a mut dyn Sink,
    hits: usize,
    /// The text not yet sent to the sink.
    buffer: String,
    last_flush: Instant,
    hits_at_flush: usize,
    /// The real paths of the folders the search is in, so that a link to a folder above is not
    /// followed for ever. (Sakura has no such check; Windows has few links like that.)
    visiting: Vec<std::path::PathBuf>,
    /// The backups that a Grep 置換 made in this search. They are not searched: their text would
    /// be replaced a second time. The paths are in lower case, as a file system may not tell the
    /// cases apart.
    kept: std::collections::HashSet<String>,
}

/// What a search needs that can be wrong: the pattern, the file patterns and the folders.
struct Prepared {
    matcher: Option<Arc<dyn Matcher>>,
    replacer: Option<RegexReplacer>,
    keys: GrepKeys,
    regexes: Vec<Arc<dyn Matcher>>,
    /// The folders as the heading shows them (`ChopYen`).
    folders: Vec<String>,
}

/// Checks the conditions in the order of `DoGrep` and makes what the search needs.
fn prepare(info: &GrepInfo) -> Result<Prepared, GrepError> {
    let built = build_matcher(&info.key, &info.options).map_err(GrepError::Pattern)?;
    if info.replace.is_some() && built.is_none() {
        return Err(GrepError::NoPattern);
    }
    if info.replace.is_some() && info.window.as_ref().is_some_and(|w| w.scanning.is_some()) {
        return Err(GrepError::WindowScanning);
    }
    // A regular expression is replaced as bregonig does, with `$1` and the like; a paste, and
    // the other searches, put the text in as it is.
    let replacer = match (&info.replace, built.as_ref().and_then(|b| b.regex.clone())) {
        (Some(replace), Some(regex)) if !replace.paste => Some(
            RegexReplacer::new(regex, &replace.text, ReplaceTarget::Match)
                .map_err(GrepError::Replacement)?,
        ),
        _ => None,
    };
    let keys = GrepKeys::parse(&info.file, info.except_file_regex).map_err(GrepError::Keys)?;
    let regexes = compile_excluded(&keys)?;
    // The folder box is not used for a window (`m_szFolder` is cleared by the dialog).
    let folders = if info.window.is_some() || info.window_gone {
        Vec::new()
    } else {
        // `CreateFolders`: each folder in its long form (`GetLongFileName`; only Windows has one).
        split_folders(&info.folder)
            .into_iter()
            .map(
                |folder| match matsu_platform::long_path::long_path(&folder) {
                    Some(long) => (long, true),
                    None => (folder, false),
                },
            )
            .collect::<Vec<_>>()
    };
    if !folders.iter().all(|(folder, _)| Path::new(folder).is_dir()) {
        return Err(GrepError::FolderMissing);
    }
    Ok(Prepared {
        matcher: built.map(|b| b.matcher),
        replacer,
        keys,
        regexes,
        folders: folders
            .iter()
            .map(|(folder, expanded)| {
                if *expanded {
                    chop_yen_keeping_root(folder)
                } else {
                    chop_yen(folder)
                }
            })
            .collect(),
    })
}

/// Whether a search with these conditions can start: the error is what `run` would return. A
/// caller that opens a window for the result checks first, so that a mistake is told in the
/// window the user is in.
pub fn check(info: &GrepInfo) -> Result<(), GrepError> {
    prepare(info).map(|_| ())
}

/// Searches. The heading is written first, then the hits as they are found, and at the end the
/// count (`DoGrep`). Returns without writing anything when the conditions are wrong.
pub fn run(
    info: &GrepInfo,
    cancel: &AtomicBool,
    sink: &mut dyn Sink,
) -> Result<Outcome, GrepError> {
    let normalized = info.normalized();
    let info = &*normalized;
    let Prepared {
        matcher,
        replacer,
        keys,
        regexes,
        folders: shown_folders,
    } = prepare(info)?;

    // The exclusions are compared with the paths of the walk, which are in the long form.
    let mut walk_keys = keys.clone();
    walk_keys.expand_absolute(&matsu_platform::long_path::long_path);
    let mut runner = Runner {
        info,
        matcher,
        replacer,
        walker: Walker::new(walk_keys, regexes),
        cancel,
        sink,
        hits: 0,
        buffer: String::new(),
        last_flush: Instant::now(),
        hits_at_flush: 0,
        visiting: Vec::new(),
        kept: std::collections::HashSet::new(),
    };
    let conditions = HeaderConditions {
        key: &info.key,
        match_case: info.options.match_case,
        whole_word: info.options.whole_word,
        regex: info.options.regex,
        folders: &shown_folders,
        sub_folder: info.sub_folder,
        except_file_regex: info.except_file_regex,
        charset: info.charset,
        layout: info.layout,
        file_only: info.file_only,
        regex_version: REGEX_VERSION,
        replace: info.replace.as_ref().map(|r| format::ReplaceHeader {
            text: &r.text,
            paste: r.paste,
        }),
        window: info.window.as_ref().map(|w| w.title.as_str()),
        window_gone: info.window_gone,
    };
    runner.sink.write(&format::header(&conditions, &keys));
    if info.window_gone {
        // `DoGrep`: after "HWND handle error." Sakura returns; there is no count.
        return Ok(Outcome {
            hits: 0,
            stopped: false,
        });
    }

    let mut stopped = false;
    if let Some(window) = &info.window {
        stopped = runner.window(window).is_err();
    } else {
        for folder in &shown_folders {
            let mut base_written = false;
            if runner.tree(folder, folder, &mut base_written).is_err() {
                stopped = true;
                break;
            }
        }
    }
    runner.flush();
    if stopped {
        runner.sink.write(format::SUSPENDED);
    }
    runner.sink.count(runner.hits, info.replace.is_some());
    Ok(Outcome {
        hits: runner.hits,
        stopped,
    })
}

/// The excluded file expressions, compiled (`CGrepExceptFileRegexps::Attach`). The case of the
/// path does not matter, as in Sakura.
fn compile_excluded(keys: &GrepKeys) -> Result<Vec<Arc<dyn Matcher>>, GrepError> {
    keys.except_file_regexes
        .iter()
        .map(|pattern| {
            RegexMatcher::new(pattern, RegexSearchOptions { match_case: false })
                .map(|m| Arc::new(m) as Arc<dyn Matcher>)
                .map_err(|e| GrepError::ExceptRegex {
                    pattern: pattern.clone(),
                    reason: e.to_string(),
                })
        })
        .collect()
}

impl Runner<'_> {
    fn check_cancel(&self) -> Result<(), Stopped> {
        if self.cancel.load(Ordering::Relaxed) {
            Err(Stopped)
        } else {
            Ok(())
        }
    }

    fn flush(&mut self) {
        if !self.buffer.is_empty() {
            self.sink.write(&self.buffer);
            self.buffer.clear();
        }
        self.last_flush = Instant::now();
        self.hits_at_flush = self.hits;
    }

    /// Sends the text on when it is time (`AddTail` after ten hits and 50 ms).
    fn flush_when_due(&mut self) {
        if self.buffer.len() >= FLUSH_BYTES
            || (!self.buffer.is_empty()
                && self.hits - self.hits_at_flush >= 10
                && self.last_flush.elapsed() >= FLUSH_INTERVAL)
        {
            self.flush();
        }
    }

    /// `DoGrepTree`: the files of the folder `path`, then its subfolders.
    fn tree(&mut self, path: &str, base: &str, base_written: &mut bool) -> Result<(), Stopped> {
        let real = std::fs::canonicalize(path).ok();
        if let Some(real) = &real {
            if self.visiting.contains(real) {
                return Ok(());
            }
            self.visiting.push(real.clone());
        }
        let result = self.tree_in(path, base, base_written);
        if real.is_some() {
            self.visiting.pop();
        }
        result
    }

    /// The body of [`tree`](Self::tree).
    fn tree_in(&mut self, path: &str, base: &str, base_written: &mut bool) -> Result<(), Stopped> {
        let mut folder_written = false;
        for name in self.walker.files_in(path) {
            self.check_cancel()?;
            let full = join_path(path, &name);
            if !self.kept.is_empty() && self.kept.contains(&full.to_lowercase()) {
                continue;
            }
            // `nBasePathLen2`: the folder below the base, without the separator after the base.
            let below_base = path
                .get(base_prefix_len(base)..)
                .or_else(|| path.get(base.len()..))
                .unwrap_or("");
            let layout = self.info.layout;
            let folder = if layout.separate_folder && layout.base_folder {
                below_base
            } else {
                path
            };
            let relative = if layout.separate_folder {
                name.as_str()
            } else {
                full.get(base_prefix_len(base)..).unwrap_or(&full)
            };
            let paths = FilePaths {
                full: &full,
                base_folder: base,
                folder,
                relative,
            };
            self.sink.progress(&full, self.hits);
            let mut state = PathInfoState {
                base_folder_written: *base_written,
                folder_written,
                file_written: false,
            };
            let result = self.file(&paths, &mut state);
            *base_written = state.base_folder_written;
            folder_written = state.folder_written;
            self.flush_when_due();
            result?;
        }
        if self.info.sub_folder {
            for name in self.walker.folders_in(path) {
                self.check_cancel()?;
                self.tree(&join_path(path, &name), base, base_written)?;
            }
        }
        Ok(())
    }

    /// `DoGrepFile`.
    fn file(&mut self, paths: &FilePaths<'_>, state: &mut PathInfoState) -> Result<(), Stopped> {
        if self.info.key.is_empty() {
            self.file_name_only(paths, state, None);
            return Ok(());
        }
        if self.info.replace.is_some() {
            return self.replace_file(paths, state);
        }
        // A huge file is read in order, a chunk at a time (ADR-0020, chapter 5). One that cannot
        // be is not read whole instead (that could use all the memory): it is told as a file
        // that cannot be read.
        if let Some(from) = self.info.huge_file_len
            && std::fs::metadata(paths.full).is_ok_and(|m| m.len() >= from)
        {
            let stream = match self.open_stream(paths.full, from) {
                Ok(stream) => stream,
                Err(reason) => {
                    self.buffer.push_str(&format::file_open_error(paths.full));
                    self.buffer.push_str(&format!("  ({reason}){EOL}"));
                    return Ok(());
                }
            };
            let code_name = self.code_name(stream.encoding);
            return self.search_stream(stream.into_reader(), paths, &code_name, state);
        }
        let Some(file) = self.open(paths.full)? else {
            return Ok(());
        };
        let code_name = self.code_name(file.encoding);
        self.search_document(&file.document, paths, &code_name, state)
    }

    /// The window of a search of 「この編集中のテキスト」 (the `hWndTarget` branch of `DoGrep`): the
    /// one document, shown as the file it has, or by its `:HWND:` name when it has none.
    fn window(&mut self, window: &WindowSource) -> Result<(), Stopped> {
        let full = if window.folder.is_empty() {
            window.name.clone()
        } else {
            join_path(&window.folder, &window.name)
        };
        let layout = self.info.layout;
        // `SGrepFilePath`: the folder is the document's own, but empty below the base folder when
        // both options are on; the relative path is the name.
        let folder = if layout.separate_folder && layout.base_folder {
            ""
        } else {
            window.folder.as_str()
        };
        let paths = FilePaths {
            full: &full,
            base_folder: &window.folder,
            folder,
            relative: &window.name,
        };
        self.check_cancel()?;
        self.sink.progress(&full, self.hits);
        let mut state = PathInfoState {
            base_folder_written: false,
            folder_written: false,
            file_written: false,
        };
        let code_name = self.code_name(window.encoding);
        let result = if self.info.key.is_empty() {
            let name = if self.info.charset.is_none() {
                window.encoding.bracket_name()
            } else {
                String::new()
            };
            self.file_name_only(&paths, &mut state, Some(name));
            Ok(())
        } else if let Some(index) = &window.scanning {
            // The snapshot has only the scanned start of the file: read the file itself, through
            // the open file of the document.
            let reader = LineReader::new(
                index.source().clone(),
                index.codec().clone(),
                index.ext_eol(),
                stream_chunk(),
            );
            self.search_stream(reader, &paths, &code_name, &mut state)
        } else if self.info.replace.is_some() {
            self.replace_window(window, &paths, &mut state, &code_name)
        } else {
            self.search_document(&window.document, &paths, &code_name, &mut state)
        };
        self.flush_when_due();
        result
    }

    /// The lines of a document, one after another (`DoGrepFile`'s loop over `ReadLine`). A
    /// document of a huge file finds each line in its block index; the text is not read whole,
    /// and a line over [`LONG_LINE_LIMIT`] is not read at all.
    fn search_document(
        &mut self,
        doc: &Document,
        paths: &FilePaths<'_>,
        code_name: &str,
        state: &mut PathInfoState,
    ) -> Result<(), Stopped> {
        let count = doc.line_count();
        let limit = doc.is_ranged().then(line_limit);
        let mut index = 0;
        self.search_lines(paths, code_name, state, &mut || {
            if index >= count {
                return Ok(None);
            }
            index += 1;
            let too_long = limit
                .and_then(|max| {
                    let bytes = doc.line_raw_len(index - 1).ok().flatten()?;
                    (bytes > max).then_some(bytes as u64)
                })
                .map(|bytes| Limited::TooLong {
                    bytes,
                    eol: doc.line_eol(index - 1).ok().flatten(),
                });
            if too_long.is_some() {
                return Ok(too_long);
            }
            doc.line(index - 1)
                .map(|line| line.map(Limited::Line))
                .map_err(|_| ReadStop::Failed)
        })
    }

    /// The lines of a file that is read in order. The stop is looked at before each read, so that
    /// a line of any length can be stopped inside, and a line over [`LONG_LINE_LIMIT`] is
    /// dropped as it is read, not held.
    fn search_stream(
        &mut self,
        mut reader: LineReader,
        paths: &FilePaths<'_>,
        code_name: &str,
        state: &mut PathInfoState,
    ) -> Result<(), Stopped> {
        let cancel = self.cancel;
        let limit = line_limit();
        let result = self.search_lines(paths, code_name, state, &mut || {
            reader
                .next_line_limited(limit, &mut || cancel.load(Ordering::Relaxed))
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::Interrupted {
                        ReadStop::Cancelled
                    } else {
                        ReadStop::Failed
                    }
                })
        });
        #[cfg(test)]
        stream_probe::record(&reader);
        result
    }

    /// The loop over the lines that `next` gives. The number of a line is counted here, so that
    /// a file that is read in order needs no index.
    fn search_lines(
        &mut self,
        paths: &FilePaths<'_>,
        code_name: &str,
        state: &mut PathInfoState,
        next: &mut dyn FnMut() -> Result<Option<Limited>, ReadStop>,
    ) -> Result<(), Stopped> {
        let mut file_hits = 0usize;
        let mut index = 0usize;
        loop {
            if index.is_multiple_of(CANCEL_CHECK_LINES) {
                self.check_cancel()?;
            }
            let line = match next() {
                Ok(Some(Limited::Line(line))) => line,
                Ok(Some(Limited::TooLong { bytes, .. })) => {
                    // Not searched, and not a hit: the result tells it, as it tells a file that
                    // cannot be read.
                    self.buffer.push_str(&format::line_too_long(
                        paths.full,
                        index as u64 + 1,
                        bytes,
                        line_limit() as u64,
                    ));
                    index += 1;
                    continue;
                }
                Ok(None) => break,
                Err(ReadStop::Cancelled) => return Err(Stopped),
                Err(ReadStop::Failed) => {
                    self.buffer.push_str(&format::file_read_error(paths.full));
                    break;
                }
            };
            let eol = line.eol.map_or("", eol_text);
            file_hits += self.line(paths, code_name, state, index as u64 + 1, &line.text, eol)?;
            index += 1;
            self.flush_when_due();
            // ファイル毎最初のみ検索: the file is done at its first hit.
            if self.info.file_only && file_hits >= 1 {
                break;
            }
        }
        Ok(())
    }

    /// The detection settings for the file at `path`: those of the document type that its name
    /// chooses, or the defaults without the types.
    fn detect_options(&self, path: &str) -> DetectOptions {
        detect_options_of(self.info, path)
    }

    /// Opens a file of `from` bytes or more to be read in order. The error says why it cannot
    /// be: it cannot be opened, or its encoding keeps a state between characters (ISO-2022-JP,
    /// UTF-7).
    fn open_stream(&self, path: &str, from: u64) -> Result<LineStream, String> {
        let options = OpenOptions {
            encoding: self.info.charset,
            ext_eol: matsu_commands::edit_options::get().ext_eol,
            huge_file_len: Some(from),
            detect: self.detect_options(path),
            ..OpenOptions::default()
        };
        match open_line_stream(path, &options, stream_chunk()) {
            Ok(Some(stream)) => Ok(stream),
            Ok(None) => Err("このファイルは範囲ごとに読めません".to_owned()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Opens a file for the search, in the encoding of the dialog or the detected one. A file that
    /// cannot be opened is told in the result and gives `None`.
    fn open(&mut self, path: &str) -> Result<Option<TextFile>, Stopped> {
        #[cfg(test)]
        stream_probe::whole_read(path);
        let mut options = OpenOptions {
            encoding: self.info.charset,
            mime_decode: self.info.mime_decode,
            // The setting now, as for a file that is opened (`bEnableExtEol`).
            ext_eol: matsu_commands::edit_options::get().ext_eol,
            ..OpenOptions::default()
        };
        options.detect = self.detect_options(path);
        let cancel = self.cancel;
        match open_file_with(path, &options, &mut |_, _| {
            if cancel.load(Ordering::Relaxed) {
                std::ops::ControlFlow::Break(())
            } else {
                std::ops::ControlFlow::Continue(())
            }
        }) {
            Ok(file) => Ok(Some(file)),
            Err(FileError::Cancelled) => Err(Stopped),
            Err(_) => {
                self.buffer.push_str(&format::file_open_error(path));
                Ok(None)
            }
        }
    }

    /// The encoding shown in front of a hit, as `  [UTF-8]`; none when the dialog chose one.
    fn code_name(&self, encoding: Encoding) -> String {
        match self.info.charset {
            None => encoding.bracket_name(),
            Some(_) => String::new(),
        }
    }

    /// The hits of one line. Sakura matches on the line with its line break, and the break is
    /// not part of the text that is written for a line. Returns the number of hits.
    fn line(
        &mut self,
        paths: &FilePaths<'_>,
        code_name: &str,
        state: &mut PathInfoState,
        number: u64,
        text: &str,
        eol: &str,
    ) -> Result<usize, Stopped> {
        let layout = self.info.layout;
        let Some(matcher) = self.matcher.clone() else {
            return Ok(0);
        };
        let subject: String;
        let whole: &str = if eol.is_empty() {
            text
        } else {
            subject = format!("{text}{eol}");
            &subject
        };
        let shown = shown_path(paths, layout);
        let mut at = 0;
        if layout.line_type == LineType::NoHitLine {
            if next_hit(matcher.as_ref(), whole, &mut at).is_some() {
                return Ok(0);
            }
            self.hits += 1;
            self.push_hit(paths, code_name, state, shown, number, 1, Body::Line(text));
            return Ok(1);
        }
        // One hit is enough for a line, unless the matched parts are written.
        let all = layout.line_type == LineType::MatchPart && !self.info.file_only;
        // The column of a hit is counted from the one before it, so that a line with many hits is
        // not read again for each.
        let (mut counted_to, mut units) = (0, 0);
        let mut hits = 0;
        while let Some(m) = next_hit(matcher.as_ref(), whole, &mut at) {
            if m.start >= counted_to {
                units += format::utf16_len(&whole[counted_to..m.start]);
            } else {
                units = format::utf16_len(&whole[..m.start]);
            }
            counted_to = m.start;
            let body = if layout.line_type == LineType::Line {
                Body::Line(text)
            } else {
                Body::Part(&whole[m.start..m.end])
            };
            hits += 1;
            self.hits += 1;
            self.push_hit(paths, code_name, state, shown, number, units + 1, body);
            // A line can have millions of hits: the stop and the output are looked at inside it.
            if hits % CANCEL_CHECK_LINES == 0 {
                self.check_cancel()?;
            }
            self.flush_when_due();
            if !all {
                break;
            }
        }
        Ok(hits)
    }

    #[allow(clippy::too_many_arguments)]
    fn push_hit(
        &mut self,
        paths: &FilePaths<'_>,
        code_name: &str,
        state: &mut PathInfoState,
        shown: &str,
        number: u64,
        column: usize,
        body: Body<'_>,
    ) {
        let layout = self.info.layout;
        format::push_path_info(&mut self.buffer, &layout, paths, code_name, state);
        format::push_result(
            &mut self.buffer,
            &layout,
            shown,
            code_name,
            number,
            column,
            body,
        );
    }

    /// A search of the names only (the pattern is empty): each file is a hit, with its encoding
    /// when that is detected (`0 == nKeyLen` in `DoGrepFile`).
    fn file_name_only(
        &mut self,
        paths: &FilePaths<'_>,
        state: &mut PathInfoState,
        known_code_name: Option<String>,
    ) {
        let layout = self.info.layout;
        let code_name = known_code_name.unwrap_or_else(|| {
            if self.info.charset.is_none() {
                detect_name(Path::new(paths.full), &self.detect_options(paths.full))
            } else {
                String::new()
            }
        });
        let shown = shown_path(paths, layout);
        let out = &mut self.buffer;
        let (full_form, base2, file_form, file_form2): (Form, &str, Form, Form) = match layout.style
        {
            OutputStyle::Normal => (Form::Plain, "■", Form::Dot, Form::Dot),
            OutputStyle::PerFile => (Form::Square, "◎", Form::Diamond, Form::Square),
            OutputStyle::ResultOnly => (Form::Plain, "■", Form::Bare, Form::Bare),
        };
        if layout.base_folder || layout.separate_folder {
            if !state.base_folder_written && layout.base_folder {
                let mark = if layout.separate_folder { "◎" } else { base2 };
                out.push_str(&format!("{mark}\"{}\"{EOL}", paths.base_folder));
                state.base_folder_written = true;
            }
            if !state.folder_written && layout.separate_folder {
                if paths.folder.is_empty() {
                    out.push_str(&format!("■{EOL}"));
                } else {
                    out.push_str(&format!("■\"{}\"{EOL}", paths.folder));
                }
                state.folder_written = true;
            }
            let form = if layout.separate_folder {
                file_form
            } else {
                file_form2
            };
            form.push(out, shown, &code_name);
        } else {
            full_form.push(out, paths.full, &code_name);
        }
        self.hits += 1;
    }
}

/// How a line of the search of the names is written (the `pszFormat...` of `DoGrepFile`).
#[derive(Clone, Copy)]
enum Form {
    /// `path  [code]`.
    Plain,
    /// `・"path"  [code]`.
    Dot,
    /// `■"path"  [code]`.
    Square,
    /// `◆"path"  [code]`.
    Diamond,
    /// `path`, without the code.
    Bare,
}

impl Form {
    fn push(self, out: &mut String, path: &str, code_name: &str) {
        match self {
            Form::Plain => out.push_str(&format!("{path}{code_name}{EOL}")),
            Form::Dot => out.push_str(&format!("・\"{path}\"{code_name}{EOL}")),
            Form::Square => out.push_str(&format!("■\"{path}\"{code_name}{EOL}")),
            Form::Diamond => out.push_str(&format!("◆\"{path}\"{code_name}{EOL}")),
            Form::Bare => out.push_str(&format!("{path}{EOL}")),
        }
    }
}

/// The name written in front of a hit (`pszDispFilePath`).
fn shown_path<'a>(paths: &FilePaths<'a>, layout: Layout) -> &'a str {
    if layout.separate_folder || layout.base_folder {
        paths.relative
    } else {
        paths.full
    }
}

/// The detection settings for the file at `path` (`CDocTypeManager::GetDocumentTypeOfPath` takes
/// the type of the name without its folder).
fn detect_options_of(info: &GrepInfo, path: &str) -> DetectOptions {
    let name = Path::new(path)
        .file_name()
        .map_or(path, |n| n.to_str().unwrap_or(path));
    match &info.type_detect {
        Some(types) => types.options_for(name),
        None => DetectOptions::default(),
    }
}

/// The encoding of a file for the search of the names, as `  [UTF-8]` (the detection of
/// `CCodeMediator::CheckKanjiCodeOfFile`).
fn detect_name(path: &Path, options: &DetectOptions) -> String {
    let mut bytes = Vec::new();
    let read = std::fs::File::open(path)
        .and_then(|file| file.take(DETECT_SAMPLE_LEN as u64).read_to_end(&mut bytes));
    match read {
        Ok(_) => detect(&bytes, options).encoding.bracket_name(),
        Err(_) => "  [(DetectError)]".to_owned(),
    }
}

/// The next match of the scan of a line, and where the scan goes on: at its end, and after an
/// empty match one character on, as `DoGrepFile` does for a regular expression. `at` is past the
/// line when the scan is over.
fn next_hit(matcher: &dyn Matcher, line: &str, at: &mut usize) -> Option<Match> {
    // `CBregexp::Match` finds nothing when the search starts at the end of the line or later
    // (`nStart >= len`), so the empty match at the very end is not a hit.
    if *at >= line.len() {
        return None;
    }
    let Ok(Some(m)) = matcher.find_at(line, *at) else {
        *at = usize::MAX;
        return None;
    };
    let next = if m.end > m.start {
        m.end
    } else {
        match line[m.start..].chars().next() {
            Some(c) => m.start + c.len_utf8(),
            None => line.len() + 1,
        }
    };
    // never go back, so that the scan ends
    *at = if next > *at { next } else { usize::MAX };
    Some(m)
}
