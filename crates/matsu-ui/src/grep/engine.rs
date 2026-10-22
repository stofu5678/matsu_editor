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
//! one file are in memory, the lines are decoded when they are looked at). The matching is the
//! matcher of the search panel (`build_matcher` in `crate::search::state`), also for
//! the regular expression and the words. Sakura does not skip binary files, nor do we.
//!
//! Grep 置換 is the same search with [`GrepInfo::replace`] set: the files are rewritten by the
//! `replace` module, through the save of `matsu_core::file`.
//!
//! Differences from Sakura: the option "この編集中のテキストから検索する" (`:HWND:`, searching the text
//! of a window) is not here; `GetLongFileName` is not applied to the folders; the
//! output to the standard output (`-GOPT=S`) is not here.

use std::borrow::Cow;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use matsu_core::encoding::{DETECT_SAMPLE_LEN, DetectOptions, Encoding, detect};
use matsu_core::file::{FileError, OpenOptions, TextFile, open_file_with};
use matsu_core::model::eol_text;
use matsu_core::search::ReplaceTarget;
use matsu_core::search::{Match, Matcher};
use matsu_regex::{
    PatternError, RegexMatcher, RegexReplacer, RegexSearchOptions, ReplacementError,
};

use super::enumerate::{Walker, join_path};
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

/// Where the result goes, and what shows the progress.
pub trait Sink {
    /// A piece of the result, to be appended to the text.
    fn write(&mut self, text: &str);
    /// The file that is being searched and the hits so far (the cancel dialog of Sakura).
    fn progress(&mut self, _file: &str, _hits: usize) {}
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

/// Stopped by the user.
struct Stopped;

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
    let folders = split_folders(&info.folder);
    if !folders.iter().all(|folder| Path::new(folder).is_dir()) {
        return Err(GrepError::FolderMissing);
    }
    Ok(Prepared {
        matcher: built.map(|b| b.matcher),
        replacer,
        keys,
        regexes,
        folders: folders.iter().map(|f| chop_yen(f)).collect(),
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

    let mut runner = Runner {
        info,
        matcher,
        replacer,
        walker: Walker::new(keys.clone(), regexes),
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
    };
    runner.sink.write(&format::header(&conditions, &keys));

    let mut stopped = false;
    for folder in &shown_folders {
        let mut base_written = false;
        if runner.tree(folder, folder, &mut base_written).is_err() {
            stopped = true;
            break;
        }
    }
    runner.flush();
    if stopped {
        runner.sink.write(format::SUSPENDED);
    }
    runner.sink.write(&if info.replace.is_some() {
        format::replace_count_line(runner.hits)
    } else {
        format::count_line(runner.hits)
    });
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
                .get(base.len() + 1..)
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
                full.get(base.len() + 1..).unwrap_or(&full)
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
            self.file_name_only(paths, state);
            return Ok(());
        }
        if self.info.replace.is_some() {
            return self.replace_file(paths, state);
        }
        let Some(file) = self.open(paths.full)? else {
            return Ok(());
        };
        let code_name = self.code_name(&file);
        let mut file_hits = 0usize;
        let doc = &file.document;
        for index in 0..doc.line_count() {
            if index % CANCEL_CHECK_LINES == 0 {
                self.check_cancel()?;
            }
            let line = match doc.line(index) {
                Ok(Some(line)) => line,
                Ok(None) => break,
                Err(_) => {
                    self.buffer.push_str(&format::file_read_error(paths.full));
                    break;
                }
            };
            let eol = line.eol.map_or("", eol_text);
            file_hits += self.line(paths, &code_name, state, index as u64 + 1, &line.text, eol)?;
            self.flush_when_due();
            // ファイル毎最初のみ検索: the file is done at its first hit.
            if self.info.file_only && file_hits >= 1 {
                break;
            }
        }
        Ok(())
    }

    /// Opens a file for the search, in the encoding of the dialog or the detected one. A file that
    /// cannot be opened is told in the result and gives `None`.
    fn open(&mut self, path: &str) -> Result<Option<TextFile>, Stopped> {
        let mut options = OpenOptions {
            encoding: self.info.charset,
            mime_decode: self.info.mime_decode,
            // The setting now, as for a file that is opened (`bEnableExtEol`).
            ext_eol: matsu_commands::edit_options::get().ext_eol,
            ..OpenOptions::default()
        };
        options.detect = DetectOptions::default();
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
    fn code_name(&self, file: &TextFile) -> String {
        match self.info.charset {
            None => file.encoding.bracket_name(),
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
        let shown = shown_path(paths, &layout);
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
    fn file_name_only(&mut self, paths: &FilePaths<'_>, state: &mut PathInfoState) {
        let layout = self.info.layout;
        let code_name = if self.info.charset.is_none() {
            detect_name(Path::new(paths.full))
        } else {
            String::new()
        };
        let shown = shown_path(paths, &layout);
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
fn shown_path<'a>(paths: &FilePaths<'a>, layout: &Layout) -> &'a str {
    if layout.separate_folder || layout.base_folder {
        paths.relative
    } else {
        paths.full
    }
}

/// The encoding of a file for the search of the names, as `  [UTF-8]` (the detection of
/// `CCodeMediator::CheckKanjiCodeOfFile`).
fn detect_name(path: &Path) -> String {
    let mut bytes = Vec::new();
    let read = std::fs::File::open(path)
        .and_then(|file| file.take(DETECT_SAMPLE_LEN as u64).read_to_end(&mut bytes));
    match read {
        Ok(_) => detect(&bytes, &DetectOptions::default())
            .encoding
            .bracket_name(),
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
