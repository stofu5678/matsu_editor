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
//!   (`HtmlHelp`, [`matsu_platform::help`]); the other systems have no such viewer, so the file is
//!   opened with the OS and the word is not looked up (the status bar says so).
//! * Sakura has its own help file (`sakura.chm`). MATSU Editor's help is a built book (ADR-0031):
//!   `F_HELP_CONTENTS` opens the copy that comes with the program ([`local_dir`]) and, when there
//!   is none, the same pages on the web ([`ONLINE_BASE`]). `F_HELP_SEARCH` opens a search on the
//!   web, or, with the local copy, its one-page version to search with the browser (see
//!   [`plan_search`]).
//! * [`PAGES`] is the one table from a function to its page; the help buttons of the dialogs
//!   (`MyWinHelp( ..., FuncID_To_HelpContextID( F_... ) )` in Sakura) read it.

use std::path::{Path, PathBuf};

use gpui::App;
use matsu_config::settings::TypeSettings;

use super::prefs::HelperPrefs;

/// The pages of the help on the web, until GitHub Pages is on (ADR-0031, PR order 20): the sources
/// on GitHub, which show Markdown. Switch only this constant then.
pub(crate) const ONLINE_BASE: &str =
    concat!(env!("CARGO_PKG_REPOSITORY"), "/blob/main/docs/help/ja/");
/// The table of contents of the help on the web (the book's `SUMMARY.md`).
pub(crate) const ONLINE_CONTENTS_PAGE: &str = "SUMMARY.md";
/// The place of the help in the repository, for the search of GitHub.
const ONLINE_SEARCH_PATH: &str = "docs/help";
/// The start of the search on the web; the query follows.
const ONLINE_SEARCH_URL: &str = "https://github.com/search?type=code&q=";
/// The longest query sent to the search (characters). A longer selection is cut.
const MAX_QUERY_CHARS: usize = 100;

/// The folder of the help that comes with the program has this file.
const LOCAL_INDEX: &str = "index.html";

/// Function name to the page of the help (a path under `docs/help/ja/`, with `.md`), as in
/// Sakura's `FuncID_To_HelpContextID` (`sakura_core/func/Funccode.cpp`). A renamed page is found by
/// the test `every_page_of_the_table_exists`.
pub(crate) const PAGES: &[(&str, &str)] = &[
    // `CDlgFileTree`: `IDC_BUTTON_HELP` is `FuncID_To_HelpContextID( F_FILETREE )`
    ("F_FILETREE", "search/file-tree.md"),
    // `CDlgTypeList`: `FuncID_To_HelpContextID( F_TYPE_LIST )`
    ("F_TYPE_LIST", "type-settings-window/type-list.md"),
    ("F_OPTION", "common-settings/menu-common-settings.md"),
    ("F_HELP_CONTENTS", "help-other/help-contents.md"),
    ("F_HELP_SEARCH", "help-other/help-search.md"),
];

/// The page of the help of a function, if it has one.
pub(crate) fn page_of(function: &str) -> Option<&'static str> {
    PAGES
        .iter()
        .find(|(name, _)| *name == function)
        .map(|(_, page)| *page)
}

/// The page for a page of the common settings: the help button of the dialog opens the page of the
/// sheet that is shown (each page of Sakura's sheet has its own topic).
pub(crate) fn common_settings_page(page: crate::common_settings::Page) -> Option<&'static str> {
    use crate::common_settings::Page;
    Some(match page {
        Page::General => "common-settings/page-general.md",
        Page::Window => "common-settings/page-window.md",
        Page::MainMenu => "common-settings/page-main-menu.md",
        Page::Toolbar => "common-settings/page-toolbar.md",
        Page::Tab => "common-settings/page-tab.md",
        Page::StatusBar => "common-settings/page-status-bar.md",
        Page::Edit => "common-settings/page-edit.md",
        Page::File => "common-settings/page-file.md",
        Page::FileName => "common-settings/page-file-name.md",
        Page::Backup => "common-settings/page-backup.md",
        Page::Format => "common-settings/page-format.md",
        Page::Search => "common-settings/page-search.md",
        Page::Keys => "common-settings/page-keys.md",
        Page::CustomMenu => "common-settings/page-custom-menu.md",
        Page::Keyword => "common-settings/page-keyword.md",
        Page::Support => "common-settings/page-support.md",
        Page::Macro => "common-settings/page-macro.md",
        Page::Plugin => "common-settings/page-plugin.md",
        Page::OneLiner => "common-settings/page-oneliner.md",
        // the shell has no page yet
        Page::Terminal => return None,
    })
}

/// The page of the dialogs 「文字色統一」 and 「背景色統一」 (`CDlgSameColor`: `HLP000316`).
pub(crate) const SAME_COLOR_PAGE: &str = "type-settings-window/type-page-color.md";

/// Every page the code opens, for the test that they exist.
#[cfg(test)]
pub(crate) fn all_pages() -> Vec<&'static str> {
    PAGES
        .iter()
        .map(|(_, page)| *page)
        .chain(
            crate::common_settings::Page::ALL
                .into_iter()
                .filter_map(common_settings_page),
        )
        .chain([SAME_COLOR_PAGE, ONLINE_CONTENTS_PAGE])
        .collect()
}

/// The folder of the help that comes with the program, if it is there: the first of the places
/// where the packages put it that has an `index.html` (ADR-0031, 決めたこと 6).
///
/// 1. `help` next to the program (the Windows zip and installer, the tar.gz of macOS and Linux)
/// 2. `../Resources/help` (the macOS app: `Contents/MacOS` next to `Contents/Resources`)
/// 3. `../share/matsu-editor/help` (`.deb` and `.rpm`: the program is in `/usr/bin`)
pub(crate) fn local_dir(exe_dir: Option<&Path>) -> Option<PathBuf> {
    let exe_dir = exe_dir?;
    let up = exe_dir.parent();
    [
        Some(exe_dir.join("help")),
        up.map(|up| up.join("Resources").join("help")),
        up.map(|up| up.join("share").join("matsu-editor").join("help")),
    ]
    .into_iter()
    .flatten()
    .find(|dir| dir.join(LOCAL_INDEX).is_file())
}

/// Whether a `#anchor` survives when a page is opened as a `file://` URL.
///
/// Not checked on any system yet: GPUI's `open_url` and `open_with_system` both go to
/// `ShellExecuteW` on Windows (and a path with `#` is a file name there), and to the system's
/// opener elsewhere; whether the browser gets the fragment was not measured. Until it is, a page
/// is opened without the anchor. [`plan_page`] and [`file_url`] are ready for the day this is true.
const LOCAL_ANCHOR_KEPT: bool = false;

/// The help page `page` (a path under `docs/help/ja/`, ending in `.md`): the built page of the
/// local help if there is one, else the page on the web. The anchor is a heading id; it goes only
/// to the local page (GitHub makes its own ids for the headings of Markdown).
pub(crate) fn plan_page(local: Option<&Path>, page: &str, anchor: Option<&str>) -> Opening {
    if let Some(dir) = local {
        let built = match page.strip_suffix(".md") {
            Some(stem) => format!("{stem}.html"),
            None => page.to_owned(),
        };
        let file = dir.join(built);
        if file.is_file() {
            return match anchor.filter(|_| LOCAL_ANCHOR_KEPT) {
                Some(anchor) => Opening::Url(format!("{}#{anchor}", file_url(&file))),
                None => Opening::File(file),
            };
        }
    }
    Opening::Url(format!("{ONLINE_BASE}{page}"))
}

/// What `Command_HELP_CONTENTS` opens: the `index.html` of the local help, else the contents on
/// the web.
pub(crate) fn plan_contents(local: Option<&Path>) -> Opening {
    match local {
        Some(dir) => Opening::File(dir.join(LOCAL_INDEX)),
        None => Opening::Url(format!("{ONLINE_BASE}{ONLINE_CONTENTS_PAGE}")),
    }
}

/// The one-page version of the book for the search: the same book as mdBook's `print.html`, written
/// next to it by the help build without the automatic `window.print()` that `print.html` runs on
/// load. A bundle without it (an older one) gets the search on the web.
const LOCAL_ONE_PAGE: &str = "search.html";

/// The status bar says this when the one-page version is opened for the search.
pub(crate) fn search_in_page_message() -> String {
    // the key of this system only
    let key = if cfg!(target_os = "macos") {
        "Command+F"
    } else {
        "Ctrl+F"
    };
    format!("ヘルプ全体を 1 ページで開きました。ブラウザーの検索（{key}）で探してください")
}

/// Whether `opening` is the one-page version opened for the search ([`plan_search`]).
pub(crate) fn is_one_page(opening: &Opening) -> bool {
    matches!(opening, Opening::File(file) if file.file_name().is_some_and(|n| n == LOCAL_ONE_PAGE))
}

/// What `Command_HELP_SEARCH` opens (ADR-0031 案 5).
///
/// * With the local help: its one-page version, where the reader searches with the browser's find
///   (the status bar says so). The search of the book (Pagefind) is not used: it reads its index
///   with `fetch`, which browsers are not known to allow for `file://` (not tried here), and the
///   app has no server for it (ADR-0031 決めたこと 8). No login is needed.
/// * Without it (or without `print.html`): a search on the web, in the help of the repository, for
///   `query` (the selection or the word at the caret). Without a query the contents open instead.
///   GitHub's code search needs a login.
pub(crate) fn plan_search(local: Option<&Path>, query: &str) -> Opening {
    if let Some(page) = local
        .map(|dir| dir.join(LOCAL_ONE_PAGE))
        .filter(|page| page.is_file())
    {
        return Opening::File(page);
    }
    let query: String = query
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(MAX_QUERY_CHARS)
        .collect();
    if query.is_empty() {
        return plan_contents(local);
    }
    // The text is one literal term: quoted, so that `NOT` or `path:` in it is not search syntax.
    let literal = format!("\"{}\"", query.replace('\\', "\\\\").replace('"', "\\\""));
    let scope = format!("repo:{} path:{ONLINE_SEARCH_PATH}", repo_slug());
    Opening::Url(format!(
        "{ONLINE_SEARCH_URL}{}",
        percent_encode(&format!("{literal} {scope}"))
    ))
}

/// `owner/name` of the repository, from `CARGO_PKG_REPOSITORY`.
fn repo_slug() -> &'static str {
    let url = env!("CARGO_PKG_REPOSITORY");
    url.strip_prefix("https://github.com/")
        .unwrap_or(url)
        .trim_end_matches('/')
}

/// Escapes every byte of `text` that is not an unreserved character of RFC 3986.
fn percent_encode(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// A `file://` URL of a path: `/` separators, `file:///C:/...` for a path with a drive, and the
/// bytes outside the unreserved characters escaped.
pub(crate) fn file_url(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let escaped = text
        .split('/')
        .map(|part| {
            // the `C:` of a drive stays
            if part.len() == 2 && part.ends_with(':') {
                part.to_owned()
            } else {
                percent_encode(part)
            }
        })
        .collect::<Vec<_>>()
        .join("/");
    if escaped.starts_with('/') {
        format!("file://{escaped}")
    } else {
        format!("file:///{escaped}")
    }
}

/// Opens the help page of a function (the help button of a dialog). Does nothing for a function
/// without a page.
pub(crate) fn open_function_page(cx: &mut App, function: &str) {
    if let Some(page) = page_of(function) {
        open_page(cx, page, None);
    }
}

/// Opens a help page, from the local help if the program has it.
pub(crate) fn open_page(cx: &mut App, page: &str, anchor: Option<&str>) {
    let local = local_dir(exe_dir().as_deref());
    perform(cx, plan_page(local.as_deref(), page, anchor));
}

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
    if matsu_platform::help::is_html_help_file(&path) {
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
            // `single` (one viewer for all windows) is what MATSU Editor always does: its windows
            // are one process (`matsu_platform::help::keyword_lookup`).
            let _ = single;
            if matsu_platform::help::keyword_lookup(&file, &keyword).is_ok() {
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

    /// Makes `<root>/<sub>/index.html` and returns the root.
    fn root_with_help(sub: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join(sub);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.html"), b"").unwrap();
        root
    }

    #[test]
    fn the_local_help_is_looked_for_in_the_places_of_the_packages() {
        // the Windows zip, the tar.gz: `help` next to the program
        let root = root_with_help("bin/help");
        assert_eq!(
            local_dir(Some(&root.path().join("bin"))),
            Some(root.path().join("bin/help"))
        );
        // the macOS app
        let root = root_with_help("Contents/Resources/help");
        assert_eq!(
            local_dir(Some(&root.path().join("Contents/MacOS"))),
            Some(root.path().join("Contents/Resources/help"))
        );
        // `.deb` and `.rpm`
        let root = root_with_help("usr/share/matsu-editor/help");
        assert_eq!(
            local_dir(Some(&root.path().join("usr/bin"))),
            Some(root.path().join("usr/share/matsu-editor/help"))
        );
    }

    /// The Linux packages: `/usr/bin/matsu` is a link and `current_exe` gives the real folder
    /// `/usr/lib/matsu-editor`, which holds a link `help` to `../../share/matsu-editor/help`.
    #[cfg(unix)]
    #[test]
    fn a_link_to_the_help_in_the_folder_of_the_program_is_followed() {
        let root = root_with_help("usr/share/matsu-editor/help");
        let exe = root.path().join("usr/lib/matsu-editor");
        std::fs::create_dir_all(&exe).unwrap();
        std::os::unix::fs::symlink("../../share/matsu-editor/help", exe.join("help")).unwrap();
        assert_eq!(local_dir(Some(&exe)), Some(exe.join("help")));
    }

    #[test]
    fn the_first_place_with_an_index_wins_and_a_folder_without_one_is_skipped() {
        let root = root_with_help("bin/help");
        let other = root.path().join("share/matsu-editor/help");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("index.html"), b"").unwrap();
        assert_eq!(
            local_dir(Some(&root.path().join("bin"))),
            Some(root.path().join("bin/help"))
        );
        // `help` without `index.html` is not the help
        std::fs::remove_file(root.path().join("bin/help/index.html")).unwrap();
        assert_eq!(local_dir(Some(&root.path().join("bin"))), Some(other));
    }

    #[test]
    fn there_is_no_local_help_without_the_folder() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(local_dir(Some(root.path())), None);
        assert_eq!(local_dir(None), None);
        // a program with no parent folder
        assert_eq!(local_dir(Some(Path::new("/"))), None);
    }

    #[test]
    fn the_contents_are_the_local_index_else_the_page_on_the_web() {
        let root = root_with_help("help");
        let dir = root.path().join("help");
        assert_eq!(
            plan_contents(Some(&dir)),
            Opening::File(dir.join("index.html"))
        );
        assert_eq!(
            plan_contents(None),
            Opening::Url(
                "https://github.com/stofu5678/matsu_editor/blob/main/docs/help/ja/SUMMARY.md"
                    .to_owned()
            )
        );
    }

    #[test]
    fn a_page_is_the_built_file_when_it_is_there_else_the_page_on_the_web() {
        let root = root_with_help("help");
        let dir = root.path().join("help");
        std::fs::create_dir_all(dir.join("search")).unwrap();
        std::fs::write(dir.join("search/file-tree.html"), b"").unwrap();
        assert_eq!(
            plan_page(Some(&dir), "search/file-tree.md", Some("anchor")),
            Opening::File(dir.join("search/file-tree.html"))
        );
        // a page the local build lacks (an older help) is read on the web
        assert_eq!(
            plan_page(Some(&dir), "search/grep.md", Some("anchor")),
            Opening::Url(format!("{ONLINE_BASE}search/grep.md"))
        );
        assert_eq!(
            plan_page(None, "search/grep.md", None),
            Opening::Url(format!("{ONLINE_BASE}search/grep.md"))
        );
    }

    #[test]
    fn the_search_is_a_code_search_of_the_help_for_the_word() {
        let Opening::Url(url) = plan_search(None, "grep の 検索") else {
            panic!("a URL")
        };
        assert_eq!(
            url,
            "https://github.com/search?type=code&q=%22grep%20%E3%81%AE%20%E6%A4%9C%E7%B4%A2%22\
             %20repo%3Astofu5678%2Fmatsu_editor%20path%3Adocs%2Fhelp"
        );
        // only the first line, trimmed, at most 100 characters
        let Opening::Url(url) = plan_search(None, "  one \ntwo") else {
            panic!("a URL")
        };
        assert!(url.contains("q=%22one%22%20repo"), "{url}");
        let Opening::Url(url) = plan_search(None, &"a".repeat(300)) else {
            panic!("a URL")
        };
        assert!(
            url.contains(&format!("q=%22{}%22%20repo", "a".repeat(100))),
            "{url}"
        );
    }

    #[test]
    fn the_word_is_a_literal_term_of_the_search() {
        // `NOT` is not an operator, and a quote or a backslash cannot end the term
        let Opening::Url(url) = plan_search(None, r#"NOT a"b\c"#) else {
            panic!("a URL")
        };
        assert!(
            url.contains("q=%22NOT%20a%5C%22b%5C%5Cc%22%20repo"),
            "{url}"
        );
    }

    #[test]
    fn the_search_opens_the_one_page_book_when_the_program_has_the_help() {
        let root = root_with_help("help");
        let dir = root.path().join("help");
        // no `search.html`: the web
        assert!(matches!(plan_search(Some(&dir), "word"), Opening::Url(_)));
        // `print.html` alone is not used: it prints on load
        std::fs::write(dir.join("print.html"), b"").unwrap();
        assert!(matches!(plan_search(Some(&dir), "word"), Opening::Url(_)));
        std::fs::write(dir.join("search.html"), b"").unwrap();
        // with or without a word, the one page; the word is not needed
        for query in ["word", ""] {
            let opening = plan_search(Some(&dir), query);
            assert_eq!(opening, Opening::File(dir.join("search.html")));
            assert!(is_one_page(&opening));
        }
        assert!(!is_one_page(&plan_contents(Some(&dir))));
        assert!(!is_one_page(&plan_search(None, "word")));
        let message = search_in_page_message();
        assert_eq!(
            message.contains("Command+F"),
            cfg!(target_os = "macos"),
            "{message}"
        );
        assert_eq!(message.contains("Ctrl+F"), !cfg!(target_os = "macos"));
    }

    #[test]
    fn a_search_without_a_word_opens_the_contents() {
        assert_eq!(plan_search(None, " \n"), plan_contents(None));
        assert_eq!(plan_search(None, ""), plan_contents(None));
    }

    #[test]
    fn a_file_url_keeps_a_drive_and_escapes_the_rest() {
        assert_eq!(
            file_url(Path::new(r"C:\Program Files\MATSU Editor\help\a b.html")),
            "file:///C:/Program%20Files/MATSU%20Editor/help/a%20b.html"
        );
        assert_eq!(
            file_url(Path::new("/usr/share/matsu-editor/help/日本.html")),
            "file:///usr/share/matsu-editor/help/%E6%97%A5%E6%9C%AC.html"
        );
    }

    #[test]
    fn the_table_finds_the_page_of_a_function() {
        assert_eq!(page_of("F_FILETREE"), Some("search/file-tree.md"));
        assert_eq!(page_of("F_FILEOPEN"), None);
    }

    #[test]
    fn every_page_of_the_table_exists() {
        // so that a renamed or removed page breaks the build
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/help/ja");
        let missing: Vec<_> = all_pages()
            .into_iter()
            .filter(|page| !root.join(page).is_file())
            .collect();
        assert!(missing.is_empty(), "pages that do not exist: {missing:?}");
    }

    #[test]
    fn the_functions_of_the_table_exist() {
        for (name, _) in PAGES {
            assert!(
                matsu_commands::sakura::FUNCTIONS
                    .iter()
                    .any(|f| f.name == *name),
                "{name}"
            );
        }
    }
}
