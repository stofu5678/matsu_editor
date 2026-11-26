//! The Migemo of the incremental search: the library, the dictionary, and what the search does
//! when they are not there.
//!
//! Sakura: `sakura_core/extmodule/CMigemo.cpp` (the library), `CEditView::ISearchEnter` and
//! `ISearchWordMake` (`sakura_core/view/CEditView_Cmdisrch.cpp`) and the two messages
//! `STR_EDITVWISRCH_MIGEGO1` and `STR_EDITVWISRCH_MIGEGO2` (`sakura_rc.rc`).
//!
//! The library is behind [`MigemoEngine`], and the way it is opened behind an opener, so that the
//! tests need no library: the real opener (`open_platform`) uses `matsu_platform::migemo`.
//! [`MigemoHost`] opens it for the whole application (Sakura's `CMigemo` is a singleton; here
//! the library is shared by the searches that are running and closed with the last of them).
//! Every incremental search holds its own [`Engine`], so that another window's settings never
//! change what a running search finds.

use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

use gpui::{App, Global};

use crate::support::prefs;

/// What the incremental search needs of the library: the regular expression for a text.
pub trait MigemoEngine {
    /// `CMigemo::migemo_query_w`. `None` when the library gives nothing.
    fn query(&self, text: &str) -> Option<String>;
}

impl MigemoEngine for matsu_platform::migemo::Migemo {
    fn query(&self, text: &str) -> Option<String> {
        Self::query(self, text)
    }
}

/// Why Migemo cannot be used. The messages are Sakura's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    /// The library cannot be loaded (`STR_EDITVWISRCH_MIGEGO1`).
    Library,
    /// The dictionary is not there or cannot be read (`STR_EDITVWISRCH_MIGEGO2`).
    Dictionary,
    /// The path of the dictionary has a character that the system code page cannot write, and
    /// the library reads the path in it. Sakura has no such message (its conversion of the path
    /// throws); the text is MATSU's (型 F).
    DictionaryPath,
}

impl Unavailable {
    /// The text of the status bar.
    pub fn message(self) -> &'static str {
        match self {
            Self::Library => "MIGEMO.DLLが使用できません。",
            Self::Dictionary => "MIGEMOは使用できません。",
            Self::DictionaryPath => {
                "MIGEMOの辞書の場所に、システムの文字コードで書けない文字があります。"
            }
        }
    }
}

/// Where the library and the dictionary are: the two boxes of 共通設定 / 支援 and the two folders
/// a relative path starts from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MigemoPaths {
    /// `szMigemoDll`.
    pub library: String,
    /// `szMigemoDict`.
    pub dictionary: String,
    /// The folder of the settings file.
    pub ini_dir: Option<PathBuf>,
    /// The folder of the program.
    pub exe_dir: Option<PathBuf>,
}

impl MigemoPaths {
    /// The paths of the settings now.
    pub fn current(cx: &App) -> Self {
        let helper = prefs::current(cx);
        let ini_dir = crate::doc_types::with_user(cx, |user| {
            crate::settings::history_file(user).and_then(|path| path.parent().map(Path::to_owned))
        });
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_owned));
        Self {
            library: helper.migemo_dll,
            dictionary: helper.migemo_dict,
            ini_dir,
            exe_dir,
        }
    }
}

/// How the library is opened. A function so that a test gives a fake.
type Opener = Box<dyn Fn(&MigemoPaths) -> Result<Rc<dyn MigemoEngine>, Unavailable>>;

/// The library of Migemo that one incremental search uses. It is shared by the searches that
/// were opened for the same paths and closed (the library unloaded) when the last of them lets
/// go, so a window never loses its library to another window's settings.
#[derive(Clone)]
pub struct Engine(Rc<dyn MigemoEngine>);

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Engine")
    }
}

impl Engine {
    /// The regular expression for `text` (`CMigemo::migemo_query_w`). `None` when the library
    /// gives nothing (there is no pattern then, and the search ends).
    pub fn pattern(&self, text: &str) -> Option<String> {
        self.0.query(text)
    }
}

/// Opens the library for the searches of the whole application.
///
/// It keeps no library of its own: it only remembers the one that a running search holds (a
/// weak reference, with the paths it was opened for), so that a second window with the same
/// settings uses it too. When the last search that holds it ends, the library is closed and
/// unloaded, and the next search opens it again. A search that begins with other paths opens
/// its own, so a change of the two boxes of 支援 reaches the next search and no running one.
/// This is a GPUI global: the application has one, and it holds nothing of a window.
pub struct MigemoHost {
    opener: Opener,
    running: Option<(MigemoPaths, Weak<dyn MigemoEngine>)>,
}

impl Global for MigemoHost {}

impl Default for MigemoHost {
    fn default() -> Self {
        Self::new(Box::new(open_platform))
    }
}

impl MigemoHost {
    pub fn new(opener: Opener) -> Self {
        Self {
            opener,
            running: None,
        }
    }

    /// The library for a search that begins now (`CMigemo::InitDll` and `migemo_load_all`, done
    /// at every start of the mode: a setting may have changed). A library that is open for the
    /// same paths is shared; else it is opened. When it cannot be opened nothing is kept, so
    /// the next search tries again.
    pub fn open(&mut self, paths: &MigemoPaths) -> Result<Engine, Unavailable> {
        if let Some((have, weak)) = &self.running
            && have == paths
            && let Some(engine) = weak.upgrade()
        {
            return Ok(Engine(engine));
        }
        let engine = (self.opener)(paths)?;
        self.running = Some((paths.clone(), Rc::downgrade(&engine)));
        Ok(Engine(engine))
    }
}

/// The real opener: the library of C/Migemo and its UTF-8 dictionary
/// ([`matsu_platform::migemo`]).
fn open_platform(paths: &MigemoPaths) -> Result<Rc<dyn MigemoEngine>, Unavailable> {
    use matsu_platform::migemo::{DictionaryError, Migemo, dictionary_dir, library_path};
    let ini_dir = paths.ini_dir.as_deref();
    let library = library_path(
        &paths.library,
        ini_dir,
        paths.exe_dir.as_deref(),
        Path::is_file,
    )
    .ok_or(Unavailable::Library)?;
    let mut migemo = Migemo::open(&library).map_err(|_| Unavailable::Library)?;
    let dictionary =
        dictionary_dir(&paths.dictionary, ini_dir, Path::is_file).ok_or(Unavailable::Dictionary)?;
    match migemo.load_dictionary(&dictionary) {
        Ok(true) => Ok(Rc::new(migemo)),
        Ok(false) => Err(Unavailable::Dictionary),
        Err(DictionaryError::PathNotRepresentable) => Err(Unavailable::DictionaryPath),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    struct Fixed(Rc<Cell<u32>>);

    impl MigemoEngine for Fixed {
        fn query(&self, text: &str) -> Option<String> {
            (text == "a").then(|| "(a|あ)".to_owned())
        }
    }

    impl Drop for Fixed {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    /// A host whose library counts how often it was opened (`opens`) and closed (`closes`).
    fn counting(
        opens: &Rc<Cell<u32>>,
        closes: &Rc<Cell<u32>>,
        fail: Option<Unavailable>,
    ) -> MigemoHost {
        let (opens, closes) = (opens.clone(), closes.clone());
        MigemoHost::new(Box::new(move |_| {
            opens.set(opens.get() + 1);
            match fail {
                Some(why) => Err(why),
                None => Ok(Rc::new(Fixed(closes.clone())) as Rc<dyn MigemoEngine>),
            }
        }))
    }

    #[test]
    fn the_messages_are_sakuras_and_the_one_of_the_path_is_matsus() {
        assert_eq!(
            Unavailable::Library.message(),
            "MIGEMO.DLLが使用できません。"
        );
        assert_eq!(
            Unavailable::Dictionary.message(),
            "MIGEMOは使用できません。"
        );
        assert!(Unavailable::DictionaryPath.message().contains("辞書の場所"));
    }

    #[test]
    fn searches_with_the_same_paths_share_one_library_and_it_closes_with_the_last() {
        let (opens, closes) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let mut host = counting(&opens, &closes, None);
        let paths = MigemoPaths::default();
        let first = host.open(&paths).unwrap();
        let second = host.open(&paths).unwrap();
        assert_eq!(opens.get(), 1);
        drop(first);
        assert_eq!(closes.get(), 0, "another search still uses it");
        assert_eq!(second.pattern("a").as_deref(), Some("(a|あ)"));
        drop(second);
        assert_eq!(closes.get(), 1, "the last search closed it");
        // nothing is kept: the next search opens it again
        let _third = host.open(&paths).unwrap();
        assert_eq!(opens.get(), 2);
    }

    #[test]
    fn other_paths_open_their_own_library_and_leave_the_running_one_alone() {
        let (opens, closes) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let mut host = counting(&opens, &closes, None);
        let mut paths = MigemoPaths::default();
        let a = host.open(&paths).unwrap();
        paths.dictionary = "other".to_owned();
        let b = host.open(&paths).unwrap();
        assert_eq!(opens.get(), 2);
        // the first search still has its library
        assert_eq!(closes.get(), 0);
        assert_eq!(a.pattern("a").as_deref(), Some("(a|あ)"));
        drop(b);
        assert_eq!(closes.get(), 1);
        assert_eq!(a.pattern("a").as_deref(), Some("(a|あ)"));
    }

    #[test]
    fn a_library_that_cannot_be_opened_is_tried_again_next_time() {
        let (opens, closes) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let mut host = counting(&opens, &closes, Some(Unavailable::Library));
        let paths = MigemoPaths::default();
        assert_eq!(host.open(&paths).err(), Some(Unavailable::Library));
        assert_eq!(host.open(&paths).err(), Some(Unavailable::Library));
        assert_eq!(opens.get(), 2);
    }

    #[test]
    fn a_text_the_library_gives_nothing_for_has_no_pattern() {
        let (opens, closes) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let mut host = counting(&opens, &closes, None);
        let engine = host.open(&MigemoPaths::default()).unwrap();
        assert_eq!(engine.pattern("zz"), None);
    }

    #[test]
    fn the_real_opener_says_which_part_is_missing() {
        // A folder where nothing is: no library.
        let dir = tempfile::tempdir().unwrap();
        let paths = MigemoPaths {
            ini_dir: Some(dir.path().to_owned()),
            exe_dir: Some(dir.path().to_owned()),
            ..MigemoPaths::default()
        };
        assert_eq!(open_platform(&paths).err(), Some(Unavailable::Library));
        // a file that is not a library is a library that cannot be loaded
        let name = matsu_platform::migemo::library_file_name();
        std::fs::write(dir.path().join(name), b"not a library").unwrap();
        assert_eq!(open_platform(&paths).err(), Some(Unavailable::Library));
        // no folder at all: nothing to look in
        assert_eq!(
            open_platform(&MigemoPaths::default()).err(),
            Some(Unavailable::Library)
        );
    }
}
