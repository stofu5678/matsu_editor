//! The choices of the Grep and Grep 置換 dialogs that the next dialog starts with.
//!
//! Sakura: `CommonSetting::m_sSearch` (`m_bGrepSubFolder`, `m_nGrepOutputLineType`,
//! `m_nGrepOutputStyle`, `m_bGrepOutputFileOnly`, `m_bGrepOutputBaseFolder`,
//! `m_bGrepSeparateFolder`, `m_bGrepExceptFileRegexp`, `m_bGrepDefaultFolder`, `m_bGrepBackup`,
//! `m_nGrepCharSet`; `sakura_core/env/CommonSetting.h`) with the keys `bGrepSubFolder`,
//! `bGrepOutputLine`, `nGrepOutputStyle`, `bGrepOutputFileOnly`, `bGrepOutputBaseFolder`,
//! `bGrepSeparateFolder`, `bGrepExceptFileRegexp`, `bGrepDefaultFolder`, `bGrepBackup` and
//! `nGrepCharSet` of `[Common]` (`ShareData_IO_Common`). The defaults are those of
//! `CShareData::InitShareData`.
//!
//! When they are read and written (`CDlgGrep.cpp`, `CDlgGrepReplace.cpp`):
//!
//! * `DoModal` takes them for the dialog that opens. The Grep 置換 dialog does not take
//!   `bGrepOutputFileOnly`, `bGrepOutputBaseFolder` and `bGrepSeparateFolder`: its own copies stay
//!   as the last Grep 置換 of the window left them (off at first).
//! * `bGrepDefaultFolder` is written as soon as its check box is clicked
//!   (`CDlgGrep::OnBnClicked`).
//! * OK (`GetData`) writes the character set, the form of the result, the three check boxes of
//!   the output and the one of the excluded files before it checks anything; Grep 置換 writes
//!   `bGrepBackup` first, once it has a condition. `bGrepSubFolder` is written at the end, once
//!   the boxes are accepted, and not when the text of the window is searched. The pattern
//!   options (`bSearchRegularExp` and the others, shared with the find dialog) are written when
//!   the condition is accepted ([`super::search`]). キャンセル writes nothing.
//! * `-GREPDLG` writes the sub folder, the pattern options, the character set and the form of the
//!   result when the process starts (`CNormalProcess::InitializeProcess`); `-GREP` and the macro
//!   `Grep` write nothing: they do not use the dialog.

use gpui::{App, Global};
use matsu_config::Profile;
use matsu_core::encoding::{AUTODETECT_CODE, Encoding};

use crate::grep::format::{LineType, OutputStyle};
use crate::grep::settings::{GrepSettings, charset_code};

const KEY_SUB_FOLDER: &str = "bGrepSubFolder";
const KEY_LINE_TYPE: &str = "bGrepOutputLine";
const KEY_STYLE: &str = "nGrepOutputStyle";
const KEY_FILE_ONLY: &str = "bGrepOutputFileOnly";
const KEY_BASE_FOLDER: &str = "bGrepOutputBaseFolder";
const KEY_SEPARATE_FOLDER: &str = "bGrepSeparateFolder";
const KEY_EXCEPT_REGEX: &str = "bGrepExceptFileRegexp";
const KEY_DEFAULT_FOLDER: &str = "bGrepDefaultFolder";
const KEY_BACKUP: &str = "bGrepBackup";
const KEY_CHARSET: &str = "nGrepCharSet";

/// What the dialogs start with. Sakura's defaults (`CShareData::InitShareData`): search the
/// sub folders, the lines of the hits in the normal form, 自動選択, back up on a replacement, the
/// other choices off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SavedGrep {
    pub sub_folder: bool,
    pub line_type: LineType,
    pub style: OutputStyle,
    pub file_only: bool,
    pub base_folder: bool,
    pub separate_folder: bool,
    pub except_file_regex: bool,
    pub default_folder: bool,
    pub backup: bool,
    /// `None` is 自動選択 (`CODE_AUTODETECT`).
    pub charset: Option<Encoding>,
}

impl Default for SavedGrep {
    fn default() -> Self {
        Self {
            sub_folder: true,
            line_type: LineType::Line,
            style: OutputStyle::Normal,
            file_only: false,
            base_folder: false,
            separate_folder: false,
            except_file_regex: false,
            default_folder: false,
            backup: true,
            charset: None,
        }
    }
}

impl Global for SavedGrep {}

impl SavedGrep {
    pub(crate) fn from_profile(profile: &Profile) -> Self {
        let d = Self::default();
        let get = |key: &str, default: bool| profile.get_bool("Common", key).unwrap_or(default);
        Self {
            sub_folder: get(KEY_SUB_FOLDER, d.sub_folder),
            line_type: profile
                .get_i32("Common", KEY_LINE_TYPE)
                .map_or(d.line_type, LineType::from_number),
            style: profile
                .get_i32("Common", KEY_STYLE)
                .map_or(d.style, OutputStyle::from_number),
            file_only: get(KEY_FILE_ONLY, d.file_only),
            base_folder: get(KEY_BASE_FOLDER, d.base_folder),
            separate_folder: get(KEY_SEPARATE_FOLDER, d.separate_folder),
            except_file_regex: get(KEY_EXCEPT_REGEX, d.except_file_regex),
            default_folder: get(KEY_DEFAULT_FOLDER, d.default_folder),
            backup: get(KEY_BACKUP, d.backup),
            charset: profile
                .get_i32("Common", KEY_CHARSET)
                .map_or(d.charset, charset_of_code),
        }
    }

    /// The keys and the numbers the file holds.
    fn pairs(self) -> [(&'static str, i32); 10] {
        [
            (KEY_SUB_FOLDER, i32::from(self.sub_folder)),
            (KEY_LINE_TYPE, self.line_type.number()),
            (KEY_STYLE, self.style.number()),
            (KEY_FILE_ONLY, i32::from(self.file_only)),
            (KEY_BASE_FOLDER, i32::from(self.base_folder)),
            (KEY_SEPARATE_FOLDER, i32::from(self.separate_folder)),
            (KEY_EXCEPT_REGEX, i32::from(self.except_file_regex)),
            (KEY_DEFAULT_FOLDER, i32::from(self.default_folder)),
            (KEY_BACKUP, i32::from(self.backup)),
            (KEY_CHARSET, charset_code(self.charset)),
        ]
    }

    /// The values of a dialog that opens now (`CDlgGrep::DoModal`, `CDlgGrepReplace::DoModal`),
    /// over what the dialog keeps for itself. `replace` is the Grep 置換 dialog, which leaves
    /// the three check boxes of the output as the window had them.
    pub(crate) fn apply(self, settings: &mut GrepSettings, replace: bool) {
        settings.sub_folder = self.sub_folder;
        settings.line_type = self.line_type;
        settings.style = self.style;
        settings.except_file_regex = self.except_file_regex;
        settings.default_folder = self.default_folder;
        settings.backup = self.backup;
        settings.charset = self.charset;
        if !replace {
            settings.file_only = self.file_only;
            settings.base_folder = self.base_folder;
            settings.separate_folder = self.separate_folder;
        }
    }
}

/// The encoding of `nGrepCharSet`. A number that no encoding of this program has is the default,
/// 自動選択 (Sakura lists such a code page too; this program has no list of code pages).
fn charset_of_code(code: i32) -> Option<Encoding> {
    if code == AUTODETECT_CODE {
        None
    } else {
        Encoding::from_sakura_code(code)
    }
}

/// The choices now.
pub(crate) fn saved(cx: &App) -> SavedGrep {
    cx.try_global::<SavedGrep>().copied().unwrap_or_default()
}

/// Changes the choices and writes the keys that changed to the settings file (the file is not
/// touched when nothing changed).
fn change(cx: &mut App, change: impl FnOnce(&mut SavedGrep)) {
    let old = saved(cx);
    let mut now = old;
    change(&mut now);
    if now == old && cx.has_global::<SavedGrep>() {
        return;
    }
    cx.set_global(now);
    crate::history::write_settings(cx, |profile| {
        let mut changed = false;
        for (key, number) in now.pairs() {
            // A key the file has with the same number stays as written (`+1`, ` 1`).
            if profile.get_i32("Common", key) != Some(number) {
                profile.set("Common", key, &number.to_string())?;
                changed = true;
            }
        }
        Ok(changed)
    });
}

/// The check box 初期値をカレントフォルダーにする was clicked.
pub(crate) fn remember_default_folder(cx: &mut App, on: bool) {
    change(cx, |s| s.default_folder = on);
}

/// Grep 置換 has a condition: バックアップ作成 is kept (`CDlgGrepReplace::GetData`).
pub(crate) fn remember_backup(cx: &mut App, on: bool) {
    change(cx, |s| s.backup = on);
}

/// OK of the dialog, before anything is checked (`CDlgGrep::GetData`): the character set, the
/// form of the result and the check boxes of it, and 除外ファイルを正規表現で指定する.
pub(crate) fn remember_output(cx: &mut App, settings: &GrepSettings) {
    change(cx, |s| {
        s.charset = settings.charset;
        s.line_type = settings.line_type;
        s.style = settings.style;
        s.file_only = settings.file_only;
        s.base_folder = settings.base_folder;
        s.separate_folder = settings.separate_folder;
        s.except_file_regex = settings.except_file_regex;
    });
}

/// OK of the dialog, once the boxes are accepted and the text of the window is not what is
/// searched: サブフォルダーからも検索する.
pub(crate) fn remember_sub_folder(cx: &mut App, on: bool) {
    change(cx, |s| s.sub_folder = on);
}

/// `-GREPDLG`: the sub folder, the character set and the form of the result of the command line
/// (`CNormalProcess::InitializeProcess`). The pattern options go through
/// [`super::search::remember_pattern_options`].
pub(crate) fn remember_command_line(cx: &mut App, settings: &GrepSettings) {
    change(cx, |s| {
        s.sub_folder = settings.sub_folder;
        s.charset = settings.charset;
        s.line_type = settings.line_type;
        s.style = settings.style;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(text: &str) -> Profile {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        Profile::from_bytes(&bytes)
    }

    #[test]
    fn the_defaults_are_sakuras() {
        let s = SavedGrep::from_profile(&Profile::new());
        assert_eq!(s, SavedGrep::default());
        assert!(s.sub_folder);
        assert_eq!(s.line_type, LineType::Line);
        assert_eq!(s.style, OutputStyle::Normal);
        assert!(s.backup);
        assert_eq!(s.charset, None);
        assert!(!s.file_only && !s.base_folder && !s.separate_folder);
        assert!(!s.except_file_regex && !s.default_folder);
    }

    #[test]
    fn the_keys_of_the_file_are_read() {
        let s = SavedGrep::from_profile(&profile(
            "[Common]\r\nbGrepSubFolder=0\r\nbGrepOutputLine=2\r\nnGrepOutputStyle=3\r\n\
             bGrepOutputFileOnly=1\r\nbGrepOutputBaseFolder=1\r\nbGrepSeparateFolder=1\r\n\
             bGrepExceptFileRegexp=1\r\nbGrepDefaultFolder=1\r\nbGrepBackup=0\r\n\
             nGrepCharSet=4\r\n",
        ));
        assert!(!s.sub_folder);
        assert_eq!(s.line_type, LineType::NoHitLine);
        assert_eq!(s.style, OutputStyle::ResultOnly);
        assert!(s.file_only && s.base_folder && s.separate_folder);
        assert!(s.except_file_regex && s.default_folder);
        assert!(!s.backup);
        assert_eq!(s.charset, Some(Encoding::Utf8));
    }

    #[test]
    fn a_missing_or_unreadable_value_is_the_default() {
        let s = SavedGrep::from_profile(&profile(
            "[Common]\r\nbGrepSubFolder=x\r\nnGrepCharSet=9999\r\nbGrepBackup=\r\n",
        ));
        assert_eq!(s, SavedGrep::default());
    }

    #[test]
    fn the_numbers_written_are_sakuras() {
        let s = SavedGrep {
            sub_folder: false,
            line_type: LineType::MatchPart,
            style: OutputStyle::PerFile,
            file_only: true,
            charset: Some(Encoding::Utf8),
            ..SavedGrep::default()
        };
        let numbers: Vec<_> = s.pairs().into_iter().collect();
        assert!(numbers.contains(&("bGrepSubFolder", 0)));
        assert!(numbers.contains(&("bGrepOutputLine", 0)));
        assert!(numbers.contains(&("nGrepOutputStyle", 2)));
        assert!(numbers.contains(&("bGrepOutputFileOnly", 1)));
        assert!(numbers.contains(&("bGrepBackup", 1)));
        assert!(numbers.contains(&("nGrepCharSet", 4)));
        assert!(SavedGrep::default().pairs().contains(&("nGrepCharSet", 99)));
    }

    #[test]
    fn the_grep_replace_dialog_leaves_the_output_check_boxes_alone() {
        let saved = SavedGrep {
            file_only: true,
            base_folder: true,
            separate_folder: true,
            sub_folder: false,
            ..SavedGrep::default()
        };
        let mut grep = GrepSettings::default();
        saved.apply(&mut grep, false);
        assert!(grep.file_only && grep.base_folder && grep.separate_folder);
        assert!(!grep.sub_folder);
        let mut replace = GrepSettings::default();
        saved.apply(&mut replace, true);
        assert!(!replace.file_only && !replace.base_folder && !replace.separate_folder);
        assert!(!replace.sub_folder);
    }
}
