//! The help functions: the external help files (`F_EXTHELP1`, `F_EXTHTMLHELP`) and the help of the
//! program (`F_HELP_CONTENTS`, `F_HELP_SEARCH`).
//!
//! Sakura: `Command_EXTHELP1`, `Command_EXTHTMLHELP`, `Command_HELP_CONTENTS` and
//! `Command_HELP_SEARCH` (`sakura_core/cmd/CViewCommander_Support.cpp`) and `CHelpManager`
//! (`sakura_core/env/CHelpManager.cpp`).
//!
//! * The file is the one of the document type if it has one, else the one of the common settings
//!   (the page 支援). A relative path starts at the folder of the settings file, else at the
//!   folder of the program (`GetInidirOrExedir`).
//! * `F_EXTHELP1` with an HTML help file (`.chi`, `.chm`, `.col`) is `F_EXTHTMLHELP`; any other
//!   file is a WinHelp file, which Windows has not shown since Windows 10, so here it is opened
//!   with the OS.
//! * The HTML help is opened at the word under the caret or the selection. Windows does that
//!   (`HtmlHelp`, [`ume_platform::help`]); the other systems have no such viewer, so the file is
//!   opened with the OS and the word is not looked up (the status bar says so).
//! * Sakura has its own help file. UME Editor has none; `F_HELP_CONTENTS` and `F_HELP_SEARCH`
//!   open the documents of the project on the web.

use std::path::{Path, PathBuf};

use gpui::App;
use ume_config::settings::TypeSettings;

use super::prefs::HelperPrefs;

/// The documents of the project: what `F_HELP_CONTENTS` opens in place of the table of contents of
/// Sakura's help.
pub(crate) const CONTENTS_URL: &str = concat!(env!("CARGO_PKG_REPOSITORY"), "/tree/main/docs");
/// The list of the decisions, which stands for the index that `F_HELP_SEARCH` opens in Sakura.
pub(crate) const INDEX_URL: &str = concat!(
    env!("CARGO_PKG_REPOSITORY"),
    "/blob/main/docs/adr/README.md"
);

/// The status bar says this when the HTML help is opened without the word.
pub(crate) const NO_KEYWORD_LOOKUP: &str =
    "この環境ではヘルプをキーワードで検索できないため、ファイルを開きました";

/// `STR_ERR_CEDITVIEW_CMD01`
pub(crate) const EXT_HELP_NOT_SET: &str = "外部ヘルプ１が設定されていません。\n今すぐ設定しますか?";
/// `STR_ERR_CEDITVIEW_CMD02`
pub(crate) const EXT_HTML_HELP_NOT_SET: &str =
    "外部HTMLヘルプが設定されていません。\n今すぐ設定しますか?";

/// What is to be opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Opening {
    /// An HTML help file at a word.
    Keyword {
        file: PathBuf,
        keyword: String,
        single: bool,
    },
    /// A file, with the program the OS has for it.
    File(PathBuf),
    /// A web page.
    Url(String),
}

/// `CHelpManager::GetExtWinHelp`: the external help file of the type if it has one, else the one
/// of the common settings. `None` when neither is set (`ExtWinHelpIsSet`).
pub(crate) fn ext_help<'a>(t: &'a TypeSettings, prefs: &'a HelperPrefs) -> Option<&'a str> {
    [t.ext_help.as_str(), prefs.ext_help.as_str()]
        .into_iter()
        .find(|file| !file.is_empty())
}

/// `CHelpManager::HTMLHelpIsSingle`: whether one viewer serves all HTML help. The type decides
/// when it has an HTML help file, else the common settings do.
pub(crate) fn html_help_single(t: &TypeSettings, prefs: &HelperPrefs) -> bool {
    if t.ext_html_help.is_empty() {
        prefs.html_help_is_single
    } else {
        t.html_help_is_single
    }
}

/// `CHelpManager::GetExtHTMLHelp`: the external HTML help file of the type if it has one, else
/// the one of the common settings, and whether one viewer serves it.
pub(crate) fn ext_html_help<'a>(
    t: &'a TypeSettings,
    prefs: &'a HelperPrefs,
) -> Option<(&'a str, bool)> {
    [t.ext_html_help.as_str(), prefs.ext_html_help.as_str()]
        .into_iter()
        .find(|file| !file.is_empty())
        .map(|file| (file, html_help_single(t, prefs)))
}

/// `GetInidirOrExedir`: a relative path is under the folder of the settings file if the file is
/// there, else under the folder of the program if the file is there, else under the first.
pub(crate) fn inidir_or_exedir(
    path: &str,
    ini_dir: Option<&Path>,
    exe_dir: Option<&Path>,
) -> PathBuf {
    let typed = Path::new(path);
    if !typed.is_relative() {
        return typed.to_owned();
    }
    let in_ini = ini_dir.map(|dir| dir.join(typed));
    if let Some(found) = in_ini.as_ref().filter(|p| p.exists()) {
        return found.clone();
    }
    if let Some(found) = exe_dir.map(|dir| dir.join(typed)).filter(|p| p.exists()) {
        return found;
    }
    in_ini.unwrap_or_else(|| typed.to_owned())
}

/// The folder of the program (`GetExedir`).
pub(crate) fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_owned))
}

/// What `Command_EXTHELP1` opens: an HTML help file is looked up (`Command_EXTHTMLHELP( path,
/// word )`, with the choice of one viewer of the type or the common settings); any other file is
/// a WinHelp file and is opened as a file.
pub(crate) fn plan_ext_help(path: PathBuf, keyword: &str, single: bool) -> Opening {
    if ume_platform::help::is_html_help_file(&path) {
        Opening::Keyword {
            file: path,
            keyword: keyword.to_owned(),
            single,
        }
    } else {
        Opening::File(path)
    }
}

/// What `Command_EXTHTMLHELP` opens: the file at the word, whatever the extension.
pub(crate) fn plan_html_help(path: PathBuf, keyword: &str, single: bool) -> Opening {
    Opening::Keyword {
        file: path,
        keyword: keyword.to_owned(),
        single,
    }
}

/// What the opening of a help file leaves to say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Opened {
    /// The word could not be looked up and the file was opened as it is.
    pub keyword_lost: bool,
}

#[cfg(test)]
thread_local! {
    /// The tests look at what would be opened instead of starting programs.
    pub(crate) static OPENED: std::cell::RefCell<Vec<Opening>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Opens it. Returns what the caller should tell the user.
#[cfg(not(test))]
pub(crate) fn perform(cx: &mut App, opening: Opening) -> Opened {
    match opening {
        Opening::Url(url) => {
            cx.open_url(&url);
            Opened {
                keyword_lost: false,
            }
        }
        Opening::File(path) => {
            cx.open_with_system(&path);
            Opened {
                keyword_lost: false,
            }
        }
        Opening::Keyword {
            file,
            keyword,
            single,
        } => {
            // `single` (one viewer for all windows) is what UME Editor always does: its windows
            // are one process (`ume_platform::help::keyword_lookup`).
            let _ = single;
            if ume_platform::help::keyword_lookup(&file, &keyword).is_ok() {
                return Opened {
                    keyword_lost: false,
                };
            }
            cx.open_with_system(&file);
            Opened { keyword_lost: true }
        }
    }
}

/// Test version: records the opening. The HTML help is lost as it is outside Windows.
#[cfg(test)]
pub(crate) fn perform(_cx: &mut App, opening: Opening) -> Opened {
    let keyword_lost = matches!(opening, Opening::Keyword { .. });
    OPENED.with(|opened| opened.borrow_mut().push(opening));
    Opened { keyword_lost }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_with(help: &str, html: &str, single: bool) -> TypeSettings {
        TypeSettings {
            ext_help: help.to_owned(),
            ext_html_help: html.to_owned(),
            html_help_is_single: single,
            ..TypeSettings::default()
        }
    }

    fn prefs(help: &str, html: &str, single: bool) -> HelperPrefs {
        HelperPrefs {
            ext_help: help.to_owned(),
            ext_html_help: html.to_owned(),
            html_help_is_single: single,
            ..HelperPrefs::default()
        }
    }

    #[test]
    fn the_type_has_the_say_over_the_common_settings() {
        // `GetExtWinHelp`
        assert_eq!(
            ext_help(&type_with("t.hlp", "", true), &prefs("c.hlp", "", true)),
            Some("t.hlp")
        );
        assert_eq!(
            ext_help(&type_with("", "", true), &prefs("c.hlp", "", true)),
            Some("c.hlp")
        );
        assert_eq!(
            ext_help(&type_with("", "", true), &prefs("", "", true)),
            None
        );
        // `GetExtHTMLHelp` and `HTMLHelpIsSingle`: the type's single flag counts with its file
        assert_eq!(
            ext_html_help(&type_with("", "t.chm", false), &prefs("", "c.chm", true)),
            Some(("t.chm", false))
        );
        assert_eq!(
            ext_html_help(&type_with("", "", false), &prefs("", "c.chm", true)),
            Some(("c.chm", true))
        );
        assert_eq!(
            ext_html_help(&type_with("", "", true), &prefs("", "", true)),
            None
        );
    }

    #[test]
    fn a_relative_path_is_found_under_the_settings_folder_then_the_program_folder() {
        let ini = tempfile::tempdir().unwrap();
        let exe = tempfile::tempdir().unwrap();
        std::fs::write(exe.path().join("only-exe.chm"), b"").unwrap();
        std::fs::write(ini.path().join("both.chm"), b"").unwrap();
        std::fs::write(exe.path().join("both.chm"), b"").unwrap();
        let find = |name| inidir_or_exedir(name, Some(ini.path()), Some(exe.path()));
        assert_eq!(find("both.chm"), ini.path().join("both.chm"));
        assert_eq!(find("only-exe.chm"), exe.path().join("only-exe.chm"));
        // in neither: the settings folder
        assert_eq!(find("none.chm"), ini.path().join("none.chm"));
        // an absolute path stays
        let abs = exe.path().join("only-exe.chm");
        assert_eq!(
            inidir_or_exedir(abs.to_str().unwrap(), Some(ini.path()), None),
            abs
        );
        assert_eq!(
            inidir_or_exedir("x.chm", None, None),
            PathBuf::from("x.chm")
        );
    }

    #[test]
    fn exthelp1_looks_up_html_help_and_opens_other_files() {
        assert_eq!(
            plan_ext_help(PathBuf::from("a.CHM"), "word", true),
            Opening::Keyword {
                file: PathBuf::from("a.CHM"),
                keyword: "word".to_owned(),
                single: true
            }
        );
        assert_eq!(
            plan_ext_help(PathBuf::from("a.hlp"), "word", true),
            Opening::File(PathBuf::from("a.hlp"))
        );
        // exthtmlhelp does not look at the extension
        assert!(matches!(
            plan_html_help(PathBuf::from("a.hlp"), "w", false),
            Opening::Keyword { .. }
        ));
    }

    #[test]
    fn the_urls_are_the_documents_of_the_project() {
        assert!(CONTENTS_URL.starts_with("https://github.com/stofu5678/ume_editor/"));
        assert!(CONTENTS_URL.ends_with("/docs"));
        assert!(INDEX_URL.ends_with("/docs/adr/README.md"));
    }
}
