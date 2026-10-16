//! What the Grep dialog holds, without any window: the values and their defaults, how the boxes
//! become one `GrepInfo`, the checks of the OK button, and the arguments of the macro `Grep`.
//!
//! Sakura: `CDlgGrep` (`sakura_core/dlg/CDlgGrep.cpp`, `CDlgGrep.h`: `GetData`, `OnInitDialog`,
//! `MakeGrepInfo`), the defaults in `sakura_core/env/CShareData.cpp` (`sSearch`), and the macro in
//! `CMacro::AddStringParam`/`CMacro::MakeGrepInfo` (`sakura_core/macro/CMacro.cpp`). The dialog is
//! described by its behavior here; its code is not copied.

use std::path::Path;

use ume_commands::command::{Arg, CommandError};
use ume_core::encoding::{AUTODETECT_CODE, Encoding};

use super::engine::{GrepInfo, ReplaceInfo};
use super::format::{Layout, LineType, OutputStyle};
use super::keys::{GrepKeys, has_unclosed_quote, split_folders, split_pattern_keep_quotes};
use crate::search::state::{SearchOptions, build_matcher, regex_error_message};

/// The first value of the box 除外ファイル (`DEFAULT_EXCLUDE_FILE_PATTERN` in
/// `sakura_core/dlg/CDlgGrep.h`).
pub const DEFAULT_EXCLUDE_FILE: &str = "*.msi;*.exe;*.obj;*.pdb;*.ilk;*.res;*.pch;*.iobj;*.ipdb";
/// The first value of the box 除外フォルダー (`DEFAULT_EXCLUDE_FOLDER_PATTERN`).
pub const DEFAULT_EXCLUDE_FOLDER: &str = ".git;.svn;.vs";

/// A mistake in the dialog: the text of the box that Sakura shows, with the field it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogError {
    pub message: String,
    /// Whether it is a regular expression error (the box has the title 正規表現エラー).
    pub regex: bool,
}

impl DialogError {
    fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            regex: false,
        }
    }
}

/// The values of the Grep dialog (the members of `CDlgGrep` and of `sSearch`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrepSettings {
    /// 条件 (`m_strText`).
    pub key: String,
    /// ファイル (`m_szFile`).
    pub file: String,
    /// フォルダー (`m_szFolder`).
    pub folder: String,
    /// 除外ファイル (`m_szExcludeFile`).
    pub exclude_file: String,
    /// 除外フォルダー (`m_szExcludeFolder`).
    pub exclude_folder: String,
    /// サブフォルダーからも検索 (`m_bGrepSubFolder`, on by default).
    pub sub_folder: bool,
    /// 英大文字と小文字を区別 / 単語単位 / 正規表現 (the other fields are not used).
    pub options: SearchOptions,
    /// 文字コードセット; `None` is 自動選択 (`m_nGrepCharSet`, `CODE_AUTODETECT` by default).
    pub charset: Option<Encoding>,
    /// 結果出力: 該当行 by default (`m_nGrepOutputLineType`).
    pub line_type: LineType,
    /// 出力形式: ノーマル by default (`m_nGrepOutputStyle`).
    pub style: OutputStyle,
    /// ファイル毎最初のみ検索.
    pub file_only: bool,
    /// ベースフォルダー表示.
    pub base_folder: bool,
    /// フォルダー毎に表示.
    pub separate_folder: bool,
    /// 除外ファイルを正規表現で指定する.
    pub except_file_regex: bool,
    /// フォルダーの初期値をカレントフォルダーにする (`m_bGrepDefaultFolder`).
    pub default_folder: bool,
    /// 置換後 of Grep 置換 (`CDlgGrepReplace::m_strText2`).
    pub replacement: String,
    /// クリップボードの内容で置換 (`m_bPaste`); off whenever the dialog opens.
    pub paste: bool,
    /// バックアップ作成 (`m_bGrepBackup`, on by default).
    pub backup: bool,
}

impl Default for GrepSettings {
    fn default() -> Self {
        // CShareData.cpp: sSearch.m_bGrepSubFolder = TRUE, m_nGrepOutputLineType = 1,
        // m_nGrepOutputStyle = 1, m_nGrepCharSet = CODE_AUTODETECT, the others FALSE; the boxes
        // start with `*.*` (`m_aGrepFiles`) and the default exclusions.
        Self {
            key: String::new(),
            file: "*.*".to_owned(),
            folder: String::new(),
            exclude_file: DEFAULT_EXCLUDE_FILE.to_owned(),
            exclude_folder: DEFAULT_EXCLUDE_FOLDER.to_owned(),
            sub_folder: true,
            options: SearchOptions::default(),
            charset: None,
            line_type: LineType::Line,
            style: OutputStyle::Normal,
            file_only: false,
            base_folder: false,
            separate_folder: false,
            except_file_regex: false,
            default_folder: false,
            replacement: String::new(),
            paste: false,
            // CShareData.cpp: sSearch.m_bGrepBackup = true
            backup: true,
        }
    }
}

impl GrepSettings {
    /// The three patterns boxes as one string, `file;#folder;!file` (`GetPackedGFileString`).
    /// The excluded patterns keep their quotation marks, so that one with a comma or a space in
    /// it stays one element.
    pub fn packed_files(&self) -> String {
        let mut packed = self.file.clone();
        for pattern in split_pattern_keep_quotes(&self.exclude_folder) {
            packed.push_str(";#");
            packed.push_str(pattern);
        }
        for pattern in split_pattern_keep_quotes(&self.exclude_file) {
            packed.push_str(";!");
            packed.push_str(pattern);
        }
        packed
    }

    /// The folder box starts with the folder of the file in the window when the box is empty or
    /// the option 初期値をカレントフォルダーにする is on (`OnInitDialog`). The window has no
    /// folder to offer without a file.
    pub fn offer_folder(&mut self, file_in_window: Option<&Path>) {
        if let Some(folder) = file_in_window.and_then(Path::parent)
            && (self.folder.is_empty() || self.default_folder)
        {
            self.folder = quote_folder(&folder.to_string_lossy());
        }
    }

    /// The OK button (`CDlgGrep::GetData`): checks the boxes in Sakura's order and makes the
    /// `GrepInfo`. The folders are made absolute and written back to the box.
    pub fn check(&mut self) -> Result<GrepInfo, DialogError> {
        self.check_boxes(None)
    }

    /// The OK button of Grep 置換 (`CDlgGrepReplace::GetData`): the condition must not be empty,
    /// and the rest is the check of a Grep. `clipboard` is the text for クリップボードの内容で置換.
    pub fn check_replace(&mut self, clipboard: Option<String>) -> Result<GrepInfo, DialogError> {
        if self.key.is_empty() {
            // STR_DLGREPLC_REPSTR
            return Err(DialogError::warning("置換条件を指定してください。"));
        }
        self.check_boxes(Some(clipboard))
    }

    /// The checks of `CDlgGrep::GetData`; `replace` is the clipboard of a Grep 置換.
    fn check_boxes(&mut self, replace: Option<Option<String>>) -> Result<GrepInfo, DialogError> {
        if has_unclosed_quote(&self.file)
            || has_unclosed_quote(&self.exclude_file)
            || has_unclosed_quote(&self.exclude_folder)
        {
            return Err(DialogError::warning(
                "対象ファイル・除外ファイル・除外フォルダーの指定は、引用符で正しく囲んでください。",
            ));
        }
        if !self.file.is_empty()
            && let Err(e) = GrepKeys::parse(&self.file, self.except_file_regex)
        {
            return Err(DialogError::warning(e.message()));
        }
        if self.file.is_empty() {
            self.file = "*.*".to_owned();
        }
        if self.folder.is_empty() {
            return Err(DialogError::warning(
                "検索対象フォルダーを指定してください。",
            ));
        }
        let mut folders = Vec::new();
        for folder in split_folders(&self.folder) {
            let path = Path::new(&folder);
            if !path.is_dir() {
                return Err(DialogError::warning(
                    "検索対象フォルダーが正しくありません。",
                ));
            }
            let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
            folders.push(quote_folder(&super::engine::chop_yen(
                &absolute.to_string_lossy(),
            )));
        }
        self.folder = folders.join(";");
        if !self.key.is_empty() && self.options.regex {
            // `CheckRegexpSyntax`
            if let Err(e) = build_matcher(&self.key, &self.options) {
                return Err(DialogError {
                    message: regex_error_message(&e),
                    regex: true,
                });
            }
        }
        // The patterns as the search reads them, the boxes of the excluded files and folders
        // included: a mistake there is told here, with the dialog still open.
        let info = match replace {
            Some(clipboard) => self.replace_info(clipboard),
            None => self.info(),
        };
        super::engine::check(&info).map_err(|e| DialogError {
            regex: matches!(
                e,
                super::engine::GrepError::Pattern(_) | super::engine::GrepError::Replacement(_)
            ),
            message: e.message(),
        })?;
        Ok(info)
    }

    /// The `GrepInfo` for the values as they are (`MakeGrepInfo`).
    pub fn info(&self) -> GrepInfo {
        GrepInfo {
            key: self.key.clone(),
            file: self.packed_files(),
            folder: self.folder.clone(),
            options: self.options,
            sub_folder: self.sub_folder,
            charset: self.charset,
            layout: Layout {
                style: self.style,
                line_type: self.line_type,
                base_folder: self.base_folder,
                separate_folder: self.separate_folder,
            },
            file_only: self.file_only,
            except_file_regex: self.except_file_regex,
            replace: None,
        }
    }

    /// The `GrepInfo` of a Grep 置換 (`CDlgGrepReplace::MakeGrepInfo`). With クリップボードの内容で
    /// 置換 the new text is `clipboard`.
    pub fn replace_info(&self, clipboard: Option<String>) -> GrepInfo {
        GrepInfo {
            replace: Some(ReplaceInfo {
                text: if self.paste {
                    clipboard.unwrap_or_default()
                } else {
                    self.replacement.clone()
                },
                paste: self.paste,
                backup: self.backup,
            }),
            ..self.info()
        }
    }

    /// The flags of the fourth argument of the macro `Grep` (`CMacro::AddStringParam`).
    pub fn macro_flags(&self) -> i64 {
        let mut flags = 0;
        let mut set = |on: bool, bit: i64| {
            if on {
                flags |= bit;
            }
        };
        set(self.sub_folder, 0x01);
        set(self.options.match_case, 0x04);
        set(self.options.regex, 0x08);
        set(self.charset.is_none(), 0x10);
        set(self.line_type == LineType::Line, 0x20);
        set(self.line_type == LineType::NoHitLine, 0x40_0000);
        set(self.style == OutputStyle::PerFile, 0x40);
        set(self.style == OutputStyle::ResultOnly, 0x80);
        set(self.options.whole_word, 0x1_0000);
        set(self.file_only, 0x2_0000);
        set(self.base_folder, 0x4_0000);
        set(self.separate_folder, 0x8_0000);
        set(self.except_file_regex, 0x80_0000);
        // The number of the encoding goes into the flags only for Sakura's own encodings.
        let code = charset_code(self.charset);
        if code == AUTODETECT_CODE || (0..=8).contains(&code) {
            flags |= i64::from(code) << 8;
        }
        flags
    }

    /// The arguments the macro `Grep` is recorded with: the pattern, the file patterns (with the
    /// excluded ones, see [`packed_files`](Self::packed_files)), the folder box, the flags and the
    /// number of the encoding.
    pub fn macro_args(&self) -> Vec<Arg> {
        vec![
            Arg::Text(self.key.clone()),
            // Sakura records the file box only, and the replay loses the excluded files and
            // folders. The packed patterns are read by the replay (`SetFileKeys` takes `!` and
            // `#`), so they are recorded.
            Arg::Text(self.packed_files()),
            Arg::Text(self.folder.clone()),
            Arg::Int(self.macro_flags()),
            Arg::Int(i64::from(charset_code(self.charset))),
        ]
    }

    /// The settings that the arguments of the macro `Grep` stand for
    /// (`CMacro::HandleCommand`, `CMacro::MakeGrepInfo`). The boxes of the excluded files and
    /// folders are empty: the macro does not pass them.
    pub fn from_macro_args(args: &[Arg]) -> Result<GrepSettings, CommandError> {
        Self::from_macro_args_at(args, 0)
    }

    /// The arguments the macro `GrepReplace` stands for: the pattern, the new text, then those
    /// of `Grep` (`CMacro::HandleCommand`; 0x100000 is クリップボードから貼り付け and 0x200000
    /// バックアップ作成).
    pub fn from_replace_macro_args(args: &[Arg]) -> Result<GrepSettings, CommandError> {
        // STR_ERR_DLGMACRO11 and 17: the pattern must not be empty either
        if !matches!(args.first(), Some(Arg::Text(t)) if !t.is_empty()) {
            return Err(CommandError::BadArgs(
                "GREPパターンが指定されていません．".into(),
            ));
        }
        let replacement = match args.get(1) {
            Some(Arg::Text(t)) => t.clone(),
            Some(other) => {
                return Err(CommandError::BadArgs(format!(
                    "argument 2 must be text, not {other:?}"
                )));
            }
            None => {
                return Err(CommandError::BadArgs(
                    "GREP置換後が指定されていません．".into(),
                ));
            }
        };
        let flags = match args.get(4) {
            Some(Arg::Int(n)) => *n,
            _ => 5,
        };
        let mut settings = Self::from_macro_args_at(args, 1)?;
        settings.replacement = replacement;
        settings.paste = flags & 0x10_0000 != 0;
        settings.backup = flags & 0x20_0000 != 0;
        Ok(settings)
    }

    /// The arguments of `Grep` start at `first` (1 after the new text of `GrepReplace`).
    fn from_macro_args_at(args: &[Arg], first: usize) -> Result<GrepSettings, CommandError> {
        let at = |i: usize| i + first;
        let text = |i: usize, missing: &str| match args.get(i) {
            Some(Arg::Text(t)) => Ok(t.clone()),
            Some(other) => Err(CommandError::BadArgs(format!(
                "argument {} must be text, not {other:?}",
                i + 1
            ))),
            None => Err(CommandError::BadArgs(missing.to_owned())),
        };
        // STR_ERR_DLGMACRO11, 12 and 13
        let key = match args.first() {
            Some(Arg::Text(t)) => t.clone(),
            _ => {
                return Err(CommandError::BadArgs(
                    "GREPパターンが指定されていません．".into(),
                ));
            }
        };
        let file = text(at(1), "ファイル種別が指定されていません．")?;
        let folder = text(at(2), "検索先フォルダーが指定されていません．")?;
        // `wtoi_def(Argument[3], 5)`: without the flags the folders and the case are the default.
        let flags = match args.get(at(3)) {
            None => 5,
            Some(Arg::Int(n)) => *n,
            Some(other) => {
                return Err(CommandError::BadArgs(format!(
                    "argument 4 must be a number, not {other:?}"
                )));
            }
        };
        let charset = macro_charset(flags, args.get(at(4)))?;
        Ok(GrepSettings {
            key,
            file,
            folder,
            exclude_file: String::new(),
            exclude_folder: String::new(),
            sub_folder: flags & 0x01 != 0,
            options: SearchOptions {
                match_case: flags & 0x04 != 0,
                regex: flags & 0x08 != 0,
                whole_word: flags & 0x1_0000 != 0,
                ..SearchOptions::default()
            },
            charset,
            line_type: if flags & 0x40_0020 == 0x20 {
                LineType::Line
            } else if flags & 0x40_0020 == 0x40_0000 {
                LineType::NoHitLine
            } else {
                LineType::MatchPart
            },
            style: match flags & 0xC0 {
                0x40 => OutputStyle::PerFile,
                0x80 => OutputStyle::ResultOnly,
                _ => OutputStyle::Normal,
            },
            file_only: flags & 0x2_0000 != 0,
            base_folder: flags & 0x4_0000 != 0,
            separate_folder: flags & 0x8_0000 != 0,
            except_file_regex: flags & 0x80_0000 != 0,
            default_folder: false,
            replacement: String::new(),
            paste: false,
            // not an argument of `Grep`; `from_replace_macro_args` sets it from the flags
            backup: true,
        })
    }
}

/// The encoding of the macro: the flag `0x10` is the automatic detection (kept for old macros),
/// the bits 8 to 15 are an encoding number, and the fifth argument is the number as it is. The
/// default is Shift_JIS.
fn macro_charset(flags: i64, fifth: Option<&Arg>) -> Result<Option<Encoding>, CommandError> {
    let mut charset = Some(Encoding::ShiftJis);
    if flags & 0x10 != 0 {
        charset = None;
    }
    // `IsValidCodeTypeExceptSJIS(nCode) || CODE_AUTODETECT == nCode`
    let code = ((flags >> 8) & 0xff) as i32;
    if code == AUTODETECT_CODE {
        charset = None;
    } else if (1..=8).contains(&code) {
        charset = Encoding::from_sakura_code(code);
    }
    match fifth {
        None => {}
        Some(Arg::Int(n)) => {
            charset = i32::try_from(*n)
                .ok()
                .filter(|n| *n != AUTODETECT_CODE)
                .and_then(Encoding::from_sakura_code);
        }
        Some(other) => {
            return Err(CommandError::BadArgs(format!(
                "argument 5 must be a number, not {other:?}"
            )));
        }
    }
    Ok(charset)
}

/// Sakura's number of an encoding choice (`CODE_AUTODETECT` is 99).
pub fn charset_code(charset: Option<Encoding>) -> i32 {
    charset.map_or(AUTODETECT_CODE, Encoding::sakura_code)
}

/// A folder in the box: one with a `;` is in quotation marks (`SetGrepFolder`).
pub fn quote_folder(folder: &str) -> String {
    if folder.contains(';') {
        format!("\"{folder}\"")
    } else {
        folder.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(s: &str) -> Arg {
        Arg::Text(s.to_owned())
    }

    #[test]
    fn the_defaults_are_sakuras() {
        let s = GrepSettings::default();
        assert_eq!(s.file, "*.*");
        assert_eq!(
            s.exclude_file,
            "*.msi;*.exe;*.obj;*.pdb;*.ilk;*.res;*.pch;*.iobj;*.ipdb"
        );
        assert_eq!(s.exclude_folder, ".git;.svn;.vs");
        assert!(s.sub_folder);
        assert_eq!(s.charset, None);
        assert_eq!(s.line_type, LineType::Line);
        assert_eq!(s.style, OutputStyle::Normal);
        assert!(!s.options.match_case && !s.options.whole_word && !s.options.regex);
        assert!(!s.file_only && !s.base_folder && !s.separate_folder && !s.except_file_regex);
    }

    #[test]
    fn the_boxes_are_packed_into_one_string() {
        let s = GrepSettings {
            file: "*.rs;*.toml".into(),
            exclude_file: r#"*.bak "a b.txt""#.into(),
            exclude_folder: "target,.git".into(),
            ..GrepSettings::default()
        };
        assert_eq!(
            s.packed_files(),
            r#"*.rs;*.toml;#target;#.git;!*.bak;!"a b.txt""#
        );
        // and the parser reads it back
        let keys = GrepKeys::parse(&s.packed_files(), false).unwrap();
        assert_eq!(keys.search_files, vec!["*.rs", "*.toml"]);
        assert_eq!(keys.except_files, vec!["*.bak", "a b.txt"]);
        assert_eq!(keys.except_folders, vec!["target", ".git"]);
    }

    #[test]
    fn the_folder_of_the_file_is_offered() {
        let mut s = GrepSettings::default();
        s.offer_folder(Some(Path::new("/home/me/work/a.txt")));
        assert_eq!(s.folder, "/home/me/work");
        // a folder the user chose stays, unless the option says to follow the file
        s.folder = "/other".into();
        s.offer_folder(Some(Path::new("/home/me/work/a.txt")));
        assert_eq!(s.folder, "/other");
        s.default_folder = true;
        s.offer_folder(Some(Path::new("/home/me/work/a.txt")));
        assert_eq!(s.folder, "/home/me/work");
        // no file, nothing to offer
        s.offer_folder(None);
        assert_eq!(s.folder, "/home/me/work");
    }

    #[test]
    fn ok_checks_in_the_order_of_sakura() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().to_string_lossy().into_owned();
        // quotation marks first
        let mut s = GrepSettings {
            file: "\"a.txt".into(),
            ..GrepSettings::default()
        };
        assert!(s.check().unwrap_err().message.contains("引用符"));
        // then the patterns
        let mut s = GrepSettings {
            file: r"*\a.txt".into(),
            ..GrepSettings::default()
        };
        assert!(s.check().unwrap_err().message.contains("ワイルドカード"));
        let mut s = GrepSettings {
            file: r"C:\a.txt".into(),
            ..GrepSettings::default()
        };
        assert!(s.check().unwrap_err().message.contains("フルパス"));
        // then the folder
        let mut s = GrepSettings::default();
        assert!(s.check().unwrap_err().message.contains("指定してください"));
        let mut s = GrepSettings {
            folder: format!("{folder}/no/such"),
            ..GrepSettings::default()
        };
        assert!(s.check().unwrap_err().message.contains("正しくありません"));
        // then the regular expression
        let mut s = GrepSettings {
            folder: folder.clone(),
            key: "(".into(),
            options: SearchOptions {
                regex: true,
                ..SearchOptions::default()
            },
            ..GrepSettings::default()
        };
        let e = s.check().unwrap_err();
        assert!(e.regex && e.message.contains("正規表現の構文に誤りがあります"));
        // a plain pattern with a bracket is no mistake
        s.options.regex = false;
        assert!(s.check().is_ok());
    }

    #[test]
    fn ok_fills_in_the_file_box_and_makes_the_folders_absolute() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().to_string_lossy().into_owned();
        let mut s = GrepSettings {
            file: String::new(),
            folder: format!("{folder}/"),
            ..GrepSettings::default()
        };
        let info = s.check().unwrap();
        assert_eq!(s.file, "*.*");
        assert_eq!(s.folder, folder);
        assert!(info.file.starts_with("*.*;#.git"));
        assert_eq!(info.folder, folder);
    }

    #[test]
    fn the_macro_flags_are_sakuras() {
        let s = GrepSettings::default();
        // subfolders 0x01, automatic detection 0x10, line 0x20, and the number 99 in 0xff00
        assert_eq!(s.macro_flags(), 0x01 | 0x10 | 0x20 | (99 << 8));
        let s = GrepSettings {
            options: SearchOptions {
                match_case: true,
                regex: true,
                whole_word: true,
                ..SearchOptions::default()
            },
            charset: Some(Encoding::Utf8),
            line_type: LineType::NoHitLine,
            style: OutputStyle::ResultOnly,
            file_only: true,
            base_folder: true,
            separate_folder: true,
            except_file_regex: true,
            sub_folder: false,
            ..GrepSettings::default()
        };
        assert_eq!(
            s.macro_flags(),
            0x04 | 0x08
                | 0x40_0000
                | 0x80
                | 0x1_0000
                | 0x2_0000
                | 0x4_0000
                | 0x8_0000
                | 0x80_0000
                | (4 << 8)
        );
        assert_eq!(
            GrepSettings::default().macro_args(),
            vec![
                text(""),
                // the excluded folders and files are in the file argument
                text(
                    "*.*;#.git;#.svn;#.vs;!*.msi;!*.exe;!*.obj;!*.pdb;!*.ilk;!*.res;!*.pch;!*.iobj;!*.ipdb"
                ),
                text(""),
                Arg::Int(0x01 | 0x10 | 0x20 | (99 << 8)),
                Arg::Int(99),
            ]
        );
    }

    #[test]
    fn the_macro_arguments_make_the_same_settings() {
        let original = GrepSettings {
            key: "foo".into(),
            folder: "/tmp".into(),
            options: SearchOptions {
                match_case: true,
                regex: true,
                ..SearchOptions::default()
            },
            charset: Some(Encoding::Utf8),
            style: OutputStyle::PerFile,
            line_type: LineType::MatchPart,
            file_only: true,
            sub_folder: false,
            exclude_file: String::new(),
            exclude_folder: String::new(),
            ..GrepSettings::default()
        };
        let back = GrepSettings::from_macro_args(&original.macro_args()).unwrap();
        assert_eq!(back, original);
        let original = GrepSettings {
            line_type: LineType::NoHitLine,
            charset: None,
            style: OutputStyle::ResultOnly,
            base_folder: true,
            separate_folder: true,
            except_file_regex: true,
            options: SearchOptions {
                whole_word: true,
                ..SearchOptions::default()
            },
            exclude_file: String::new(),
            exclude_folder: String::new(),
            ..GrepSettings::default()
        };
        assert_eq!(
            GrepSettings::from_macro_args(&original.macro_args()).unwrap(),
            original
        );
    }

    #[test]
    fn the_replay_of_a_recorded_grep_searches_the_same_files() {
        let recorded = GrepSettings {
            key: "x".into(),
            folder: "/tmp".into(),
            file: "*.rs;*.toml".into(),
            exclude_file: "*.bak \"a b.txt\"".into(),
            exclude_folder: "target".into(),
            ..GrepSettings::default()
        };
        let replayed = GrepSettings::from_macro_args(&recorded.macro_args()).unwrap();
        assert_eq!(replayed.info().file, recorded.info().file);
        let keys = GrepKeys::parse(&replayed.info().file, false).unwrap();
        assert_eq!(keys.except_files, vec!["*.bak", "a b.txt"]);
        assert_eq!(keys.except_folders, vec!["target"]);
    }

    #[test]
    fn the_macro_defaults_are_those_of_cmacro() {
        // `Grep('a','*.txt','/tmp')`: the flags are 5 (subfolders, case), Shift_JIS.
        let s = GrepSettings::from_macro_args(&[text("a"), text("*.txt"), text("/tmp")]).unwrap();
        assert!(s.sub_folder && s.options.match_case);
        assert_eq!(s.charset, Some(Encoding::ShiftJis));
        assert_eq!(s.line_type, LineType::MatchPart);
        assert_eq!(s.style, OutputStyle::Normal);
        // 0x10 is the detection; the number in the flags beats it; the fifth argument beats both
        let with = |flags: i64, fifth: Option<i64>| {
            let mut args = vec![text("a"), text("*"), text("/"), Arg::Int(flags)];
            args.extend(fifth.map(Arg::Int));
            GrepSettings::from_macro_args(&args).unwrap().charset
        };
        assert_eq!(with(0x10, None), None);
        assert_eq!(with(0x10 | (4 << 8), None), Some(Encoding::Utf8));
        assert_eq!(with(0x10 | (4 << 8), Some(2)), Some(Encoding::EucJp));
        assert_eq!(with(4 << 8, Some(99)), None);
        assert_eq!(with(0, None), Some(Encoding::ShiftJis));
        // an empty pattern is allowed; a missing one is not
        assert!(GrepSettings::from_macro_args(&[text(""), text("*"), text("/")]).is_ok());
        assert!(GrepSettings::from_macro_args(&[]).is_err());
        assert!(GrepSettings::from_macro_args(&[text("a")]).is_err());
        assert!(GrepSettings::from_macro_args(&[text("a"), text("*")]).is_err());
        assert!(GrepSettings::from_macro_args(&[Arg::Int(1), text("*"), text("/")]).is_err());
    }

    #[test]
    fn folders_with_a_semicolon_are_quoted() {
        assert_eq!(quote_folder("/a/b"), "/a/b");
        assert_eq!(quote_folder("/a;b"), "\"/a;b\"");
        assert_eq!(charset_code(None), 99);
        assert_eq!(charset_code(Some(Encoding::Utf8)), 4);
    }
    #[test]
    fn the_replace_macro_has_the_new_text_and_two_more_flags() {
        let text = |t: &str| Arg::Text(t.to_owned());
        let args = [
            text("foo"),
            text("bar"),
            text("*.txt"),
            text("/tmp"),
            Arg::Int(0x01 | 0x08 | 0x10_0000 | 0x20_0000),
            Arg::Int(4),
        ];
        let s = GrepSettings::from_replace_macro_args(&args).unwrap();
        assert_eq!(s.key, "foo");
        assert_eq!(s.replacement, "bar");
        assert_eq!(s.file, "*.txt");
        assert_eq!(s.folder, "/tmp");
        assert!(s.options.regex && s.sub_folder && s.paste && s.backup);
        assert_eq!(s.charset, Some(Encoding::Utf8));
        // without the flags: the folders and the case, no paste, no backup
        let s = GrepSettings::from_replace_macro_args(&args[..4]).unwrap();
        assert!(s.sub_folder && !s.paste && !s.backup && !s.options.regex);
        // STR_ERR_DLGMACRO11 (also for an empty pattern) and 17
        assert!(GrepSettings::from_replace_macro_args(&[]).is_err());
        assert!(
            GrepSettings::from_replace_macro_args(&[text(""), text("b"), text("*"), text("/")])
                .is_err()
        );
        assert!(GrepSettings::from_replace_macro_args(&[text("a")]).is_err());
        assert!(GrepSettings::from_replace_macro_args(&[text("a"), text("b"), text("*")]).is_err());
        // an empty new text is a deletion
        assert!(
            GrepSettings::from_replace_macro_args(&[text("a"), text(""), text("*"), text("/")])
                .is_ok()
        );
    }

    #[test]
    fn the_replace_dialog_wants_a_condition_and_makes_the_replacement() {
        let dir = std::env::temp_dir();
        let mut s = GrepSettings {
            folder: dir.to_string_lossy().into_owned(),
            replacement: "new".into(),
            ..GrepSettings::default()
        };
        // STR_DLGREPLC_REPSTR
        let error = s.check_replace(None).unwrap_err();
        assert_eq!(error.message, "置換条件を指定してください。");
        s.key = "old".into();
        let info = s.check_replace(None).unwrap();
        let replace = info.replace.unwrap();
        assert_eq!(replace.text, "new");
        assert!(
            !replace.paste && replace.backup,
            "the backup is on by default"
        );
        // the clipboard replaces the text of the box
        s.paste = true;
        let replace = s
            .check_replace(Some("clip".into()))
            .unwrap()
            .replace
            .unwrap();
        assert_eq!(replace.text, "clip");
        assert!(replace.paste);
        // a wrong replacement of a regular expression is a regular expression error
        s.paste = false;
        s.options.regex = true;
        s.replacement = "x\\".into();
        let error = s.check_replace(None).unwrap_err();
        assert!(error.regex, "{error:?}");
        // a Grep is not a replacement
        assert!(s.info().replace.is_none());
    }
}
