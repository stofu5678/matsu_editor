//! The C/C++ header or source of the same name (`F_OPEN_HfromtoC`).
//!
//! Sakura: `Command_OPEN_HfromtoC`, `Command_OPEN_HHPP`, `Command_OPEN_CCPP`
//! (`sakura_core/cmd/CViewCommander_File.cpp`) and `CEditView::OPEN_ExtFromtoExt`
//! (`sakura_core/view/CEditView_Command.cpp`). A source file (`.c .cpp .cxx .cc .cp .c++`) looks
//! for a header (`.h .hpp .hxx .hh .hp .h++`) of the same name in the same folder, a header for a
//! source, each in the order of the list, and the first that exists is opened. The extension of
//! the open file is compared without regard to case.

use std::path::{Path, PathBuf};

/// `source_ext` of `Command_OPEN_HHPP`.
pub(crate) const SOURCE_EXTENSIONS: [&str; 6] = ["c", "cpp", "cxx", "cc", "cp", "c++"];
/// `header_ext` of `Command_OPEN_HHPP`.
pub(crate) const HEADER_EXTENSIONS: [&str; 6] = ["h", "hpp", "hxx", "hh", "hp", "h++"];

/// The file to open from `path`: the first file of the same name with an extension from the other
/// list that `exists`. `None` if `path` is neither a source nor a header, or none of them exists
/// (Sakura beeps).
///
/// A header is tried first (`Command_OPEN_HHPP`), then a source (`Command_OPEN_CCPP`); the lists do
/// not overlap, so only one of them applies.
pub(crate) fn counterpart(path: &Path, exists: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let ext = path.extension()?.to_str()?;
    let wanted = if SOURCE_EXTENSIONS
        .iter()
        .any(|e| e.eq_ignore_ascii_case(ext))
    {
        &HEADER_EXTENSIONS
    } else if HEADER_EXTENSIONS
        .iter()
        .any(|e| e.eq_ignore_ascii_case(ext))
    {
        &SOURCE_EXTENSIONS
    } else {
        return None;
    };
    wanted
        .iter()
        .map(|ext| path.with_extension(ext))
        .find(|candidate| exists(candidate))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn with(files: &[&str]) -> impl Fn(&Path) -> bool {
        let set: HashSet<PathBuf> = files.iter().map(PathBuf::from).collect();
        move |p| set.contains(p)
    }

    #[test]
    fn a_source_finds_its_header_in_sakuras_order() {
        let path = Path::new("/p/main.cpp");
        assert_eq!(
            counterpart(path, with(&["/p/main.h"])),
            Some(PathBuf::from("/p/main.h"))
        );
        // h before hpp, hpp before hxx
        assert_eq!(
            counterpart(path, with(&["/p/main.hxx", "/p/main.hpp"])),
            Some(PathBuf::from("/p/main.hpp"))
        );
        assert_eq!(
            counterpart(path, with(&["/p/main.h++", "/p/main.hh"])),
            Some(PathBuf::from("/p/main.hh"))
        );
    }

    #[test]
    fn a_header_finds_its_source_in_sakuras_order() {
        let path = Path::new("/p/main.h");
        assert_eq!(
            counterpart(path, with(&["/p/main.cc", "/p/main.cxx", "/p/main.c"])),
            Some(PathBuf::from("/p/main.c"))
        );
        assert_eq!(
            counterpart(path, with(&["/p/main.c++", "/p/main.cp"])),
            Some(PathBuf::from("/p/main.cp"))
        );
    }

    #[test]
    fn nothing_is_found_when_no_file_exists_or_the_type_is_wrong() {
        assert_eq!(counterpart(Path::new("/p/main.cpp"), with(&[])), None);
        assert_eq!(
            counterpart(Path::new("/p/main.h"), with(&["/p/main.h"])),
            None
        );
        // not C/C++
        assert_eq!(
            counterpart(Path::new("/p/main.rs"), with(&["/p/main.h", "/p/main.c"])),
            None
        );
        assert_eq!(
            counterpart(Path::new("/p/Makefile"), with(&["/p/Makefile.h"])),
            None
        );
        // a file that is both sides of nothing: the same extension is not its own partner
        assert_eq!(counterpart(Path::new("/p/a.c"), with(&["/p/a.c"])), None);
    }

    #[test]
    fn the_extension_is_compared_without_case_and_dots_in_the_name_stay() {
        assert_eq!(
            counterpart(Path::new("/p/Main.CPP"), with(&["/p/Main.h"])),
            Some(PathBuf::from("/p/Main.h"))
        );
        assert_eq!(
            counterpart(Path::new("/p/a.b.c"), with(&["/p/a.b.h"])),
            Some(PathBuf::from("/p/a.b.h"))
        );
        assert_eq!(
            counterpart(
                Path::new("/日本語/ふぁいる.h"),
                with(&["/日本語/ふぁいる.cpp"])
            ),
            Some(PathBuf::from("/日本語/ふぁいる.cpp"))
        );
    }

    #[test]
    fn real_files_are_looked_up_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("x.hpp"), b"").unwrap();
        // a folder with the right name is not a file
        std::fs::create_dir(dir.path().join("x.h")).unwrap();
        let found = counterpart(&dir.path().join("x.cpp"), Path::is_file);
        assert_eq!(found, Some(dir.path().join("x.hpp")));
    }
}
