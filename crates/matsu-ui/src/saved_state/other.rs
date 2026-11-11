//! The choices of the DIFF, the tag file and the tag jump dialogs that the next dialog starts
//! with.
//!
//! Sakura: `DLLSHAREDATA::m_nDiffFlgOpt`, `m_nTagsOpt`, `m_szTagsCmdLine` and `m_sTagJump`
//! (`m_bTagJumpICase`, `m_bTagJumpPartialMatch`; `sakura_core/env/DLLSHAREDATA.h`,
//! `CTagJumpManager.h`) with the keys `nDiffFlgOpt`, `nTagsOpt`, `szTagsCmdLine`, `m_bTagJumpICase`
//! and `m_bTagJumpAnyWhere` of `[Other]` (`ShareData_IO_Other`). The defaults (all off, no command
//! line) are those of `CShareData::InitShareData`.
//!
//! * The DIFF dialog (`CDlgDiff::SetData`) takes `nDiffFlgOpt` when it opens, and OK (`GetData`)
//!   writes it before it looks for the other side. Cancel writes nothing. The macro `Diff` runs
//!   with its own flags and does not touch it (`CMacro`, `Command_Diff`), nor does the command
//!   line.
//! * The dialog of 「タグファイルの作成」 (`CDlgTagsMake`) takes `nTagsOpt` (the bit 1 is
//!   サブフォルダーも対象にする) and `szTagsCmdLine` when it opens; OK writes both. The folder is
//!   not kept.
//! * The keyword form of the tag jump list (`CDlgTagJumpList::SetData`) takes the two check boxes
//!   when it opens; ジャンプ, Enter and a double click (`GetData`) write them, the other closes do
//!   not. The direct form has neither.

use gpui::App;
use matsu_config::Profile;
use matsu_core::tags_make::TagsMakeOptions;

const SECTION: &str = "Other";
const KEY_DIFF: &str = "nDiffFlgOpt";
const KEY_TAGS_OPT: &str = "nTagsOpt";
const KEY_TAGS_CMD: &str = "szTagsCmdLine";
const KEY_ICASE: &str = "m_bTagJumpICase";
const KEY_ANYWHERE: &str = "m_bTagJumpAnyWhere";

/// `m_nTagsOpt & 0x0001`: サブフォルダーも対象にする.
const TAGS_RECURSIVE: i32 = 0x0001;

/// Takes the choices of the settings file when the application starts. A key that is missing
/// leaves the default of the dialog.
pub(crate) fn install(cx: &mut App, profile: &Profile) {
    if let Some(flags) = profile.get_i32(SECTION, KEY_DIFF) {
        let previous = crate::diff::remembered(cx);
        // The bits are Sakura's (`m_nDiffFlgOpt`); a number with a bit that is none of them
        // keeps its other bits.
        crate::diff::remember(
            cx,
            crate::diff::Remembered {
                flags: flags as u32,
                ..previous
            },
        );
    }
    crate::tags::remember_prefs(cx, |prefs| {
        if let Some(on) = profile.get_bool(SECTION, KEY_ICASE) {
            prefs.ignore_case = on;
        }
        if let Some(on) = profile.get_bool(SECTION, KEY_ANYWHERE) {
            prefs.partial = on;
        }
        if let Some(options) = profile.get_i32(SECTION, KEY_TAGS_OPT) {
            prefs.make.recursive = options & TAGS_RECURSIVE != 0;
        }
        if let Some(line) = profile.get_string(SECTION, KEY_TAGS_CMD) {
            prefs.make.command_line = line;
        }
    });
}

/// Writes the entries that are not what the file has. Without a file on disk nothing is written.
fn write(cx: &App, entries: &[(&'static str, Entry)]) {
    crate::history::write_settings(cx, |profile| {
        let mut changed = false;
        for (key, entry) in entries {
            let same = match entry {
                // A number the file already has stays as written.
                Entry::Number(n) => profile.get_i32(SECTION, key) == Some(*n),
                Entry::Text(text) => profile.get(SECTION, key) == Some(text.as_str()),
            };
            if !same {
                let text = match entry {
                    Entry::Number(n) => n.to_string(),
                    Entry::Text(text) => text.clone(),
                };
                profile.set(SECTION, key, &text)?;
                changed = true;
            }
        }
        Ok(changed)
    });
}

enum Entry {
    Number(i32),
    Text(String),
}

/// The DIFF dialog's OK (`CDlgDiff::GetData`): the options and the choice of the new or the old
/// file.
pub(crate) fn remember_diff_flags(cx: &App, flags: u32) {
    write(cx, &[(KEY_DIFF, Entry::Number(flags as i32))]);
}

/// The OK of 「タグファイルの作成」 (`CDlgTagsMake::GetData`).
pub(crate) fn remember_tags_make(cx: &App, options: &TagsMakeOptions) {
    write(
        cx,
        &[
            (
                KEY_TAGS_OPT,
                Entry::Number(if options.recursive { TAGS_RECURSIVE } else { 0 }),
            ),
            (KEY_TAGS_CMD, Entry::Text(options.command_line.clone())),
        ],
    );
}

/// ジャンプ of the keyword form of the tag jump list (`CDlgTagJumpList::GetData`).
pub(crate) fn remember_tag_jump_options(cx: &App, ignore_case: bool, partial: bool) {
    write(
        cx,
        &[
            (KEY_ICASE, Entry::Number(i32::from(ignore_case))),
            (KEY_ANYWHERE, Entry::Number(i32::from(partial))),
        ],
    );
}
