//! The Migemo of the incremental search: the library, the dictionary, and what the search does
//! when they are not there.
//!
//! Sakura: `sakura_core/extmodule/CMigemo.cpp` (the library), `CEditView::ISearchEnter` and
//! `ISearchWordMake` (`sakura_core/view/CEditView_Cmdisrch.cpp`) and the two messages
//! `STR_EDITVWISRCH_MIGEGO1` and `STR_EDITVWISRCH_MIGEGO2` (`sakura_rc.rc`).
//!
//! The library is behind [`MigemoEngine`], and the way it is opened behind an opener, so that the
//! tests need no library: the real opener (`open_platform`) uses `matsu_platform::migemo`.
//! [`MigemoHost`] is the one place that keeps the opened library for the whole application
//! (Sakura's `CMigemo` is a singleton too); it is a GPUI global that holds nothing of a window.
//! When the two paths of the settings change, the library is opened again at the next use.

use std::path::{Path, PathBuf};

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
}

impl Unavailable {
    /// The text of the status bar.
    pub fn message(self) -> &'static str {
        match self {
            Self::Library => "MIGEMO.DLLが使用できません。",
            Self::Dictionary => "MIGEMOは使用できません。",
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
type Opener = Box<dyn Fn(&MigemoPaths) -> Result<Box<dyn MigemoEngine>, Unavailable>>;

/// The library of Migemo for the whole application, once it is opened.
pub struct MigemoHost {
    opener: Opener,
    /// The library with the paths it was opened for: it is opened again when they change.
    open: Option<(MigemoPaths, Box<dyn MigemoEngine>)>,
}

impl Global for MigemoHost {}

impl Default for MigemoHost {
    fn default() -> Self {
        Self::new(Box::new(open_platform))
    }
}

impl MigemoHost {
    pub fn new(opener: Opener) -> Self {
        Self { opener, open: None }
    }

    /// Opens the library for `paths` if it is not open for them yet (`CMigemo::InitDll` and
    /// `migemo_load_all`, done at every start of the mode: a setting may have changed). When it
    /// cannot be opened nothing is kept, so the next call tries again.
    pub fn prepare(&mut self, paths: &MigemoPaths) -> Result<(), Unavailable> {
        if self.open.as_ref().is_some_and(|(have, _)| have == paths) {
            return Ok(());
        }
        // the old one is closed before the new one is opened
        self.open = None;
        let engine = (self.opener)(paths)?;
        self.open = Some((paths.clone(), engine));
        Ok(())
    }

    /// The regular expression for `text`. When the library gives nothing, the text itself
    /// (`migemo_query_w` returns the query when it cannot convert it).
    pub fn pattern(&self, text: &str) -> String {
        self.open
            .as_ref()
            .and_then(|(_, engine)| engine.query(text))
            .unwrap_or_else(|| text.to_owned())
    }
}

/// The real opener: the library of C/Migemo and its UTF-8 dictionary
/// ([`matsu_platform::migemo`]).
fn open_platform(paths: &MigemoPaths) -> Result<Box<dyn MigemoEngine>, Unavailable> {
    use matsu_platform::migemo::{Migemo, dictionary_dir, library_path};
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
    if !migemo.load_dictionary(&dictionary) {
        return Err(Unavailable::Dictionary);
    }
    Ok(Box::new(migemo))
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    struct Fixed;

    impl MigemoEngine for Fixed {
        fn query(&self, text: &str) -> Option<String> {
            (text == "a").then(|| "(a|あ)".to_owned())
        }
    }

    fn counting(opens: &Rc<Cell<u32>>, fail: Option<Unavailable>) -> MigemoHost {
        let opens = opens.clone();
        MigemoHost::new(Box::new(move |_| {
            opens.set(opens.get() + 1);
            match fail {
                Some(why) => Err(why),
                None => Ok(Box::new(Fixed)),
            }
        }))
    }

    #[test]
    fn the_messages_are_sakuras() {
        assert_eq!(
            Unavailable::Library.message(),
            "MIGEMO.DLLが使用できません。"
        );
        assert_eq!(
            Unavailable::Dictionary.message(),
            "MIGEMOは使用できません。"
        );
    }

    #[test]
    fn the_library_is_opened_once_for_the_same_paths_and_again_when_they_change() {
        let opens = Rc::new(Cell::new(0));
        let mut host = counting(&opens, None);
        let mut paths = MigemoPaths::default();
        host.prepare(&paths).unwrap();
        host.prepare(&paths).unwrap();
        assert_eq!(opens.get(), 1);
        paths.dictionary = "other".to_owned();
        host.prepare(&paths).unwrap();
        assert_eq!(opens.get(), 2);
    }

    #[test]
    fn a_library_that_cannot_be_opened_is_tried_again_next_time() {
        let opens = Rc::new(Cell::new(0));
        let mut host = counting(&opens, Some(Unavailable::Library));
        let paths = MigemoPaths::default();
        assert_eq!(host.prepare(&paths), Err(Unavailable::Library));
        assert_eq!(host.prepare(&paths), Err(Unavailable::Library));
        assert_eq!(opens.get(), 2);
    }

    #[test]
    fn the_pattern_is_the_one_of_the_library_or_else_the_text() {
        let opens = Rc::new(Cell::new(0));
        let mut host = counting(&opens, None);
        // not open yet: the text
        assert_eq!(host.pattern("a"), "a");
        host.prepare(&MigemoPaths::default()).unwrap();
        assert_eq!(host.pattern("a"), "(a|あ)");
        // the library gives nothing for this one
        assert_eq!(host.pattern("zz"), "zz");
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
