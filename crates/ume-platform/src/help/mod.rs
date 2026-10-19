//! Opening an HTML help file at a keyword (外部HTMLヘルプ, `F_EXTHTMLHELP` and `F_EXTHELP1`).
//!
//! Sakura: `CViewCommander::Command_EXTHTMLHELP` (`sakura_core/cmd/CViewCommander_Support.cpp`)
//! calls `HtmlHelp` with `HH_KEYWORD_LOOKUP`: the viewer opens the file at the keyword, or at its
//! index when the file does not have it. That exists on Windows only (`hhctrl.ocx`); the other
//! systems have no such viewer, so the caller opens the file with the OS and the keyword is lost.
//!
//! `unsafe` is in `windows` only.

use std::path::Path;

#[cfg(windows)]
mod windows;

/// Why the viewer was not started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpError {
    /// This system has no HTML help viewer with a keyword lookup.
    Unsupported,
    /// The viewer could not be started.
    Failed(String),
}

/// Whether the file is an HTML help file (`.chi`, `.chm`, `.col`, case ignored): the others
/// are WinHelp files (`Command_EXTHELP1`).
pub fn is_html_help_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        ["chi", "chm", "col"]
            .iter()
            .any(|k| e.eq_ignore_ascii_case(k))
    })
}

/// Opens `file` at `keyword` (`HH_KEYWORD_LOOKUP`, with the index shown when the keyword is not
/// there).
///
/// There is no choice of one viewer or one for each window (`m_bHtmlHelpIsSingle`): Sakura's
/// windows are processes of their own, each with its own viewer unless the process in the tray
/// opens the help for all of them. UME Editor's windows are one process, so its viewer is always
/// the one of the process, as with that option on.
#[cfg(windows)]
pub fn keyword_lookup(file: &Path, keyword: &str) -> Result<(), HelpError> {
    windows::keyword_lookup(file, keyword)
}

/// Opens `file` at `keyword`: not on this system.
#[cfg(not(windows))]
pub fn keyword_lookup(_file: &Path, _keyword: &str) -> Result<(), HelpError> {
    Err(HelpError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_help_files_are_told_by_the_extension() {
        for name in ["a.chm", "a.CHM", "dir/b.chi", "c.col", "x.y.Chm"] {
            assert!(is_html_help_file(Path::new(name)), "{name}");
        }
        for name in ["a.hlp", "a", "a.html", "chm", "a.chm.txt"] {
            assert!(!is_html_help_file(Path::new(name)), "{name}");
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn there_is_no_keyword_lookup_outside_windows() {
        assert_eq!(
            keyword_lookup(Path::new("a.chm"), "word"),
            Err(HelpError::Unsupported)
        );
    }
}
