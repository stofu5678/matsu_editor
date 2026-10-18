// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/env/CDocTypeManager.cpp (ConvertTypesExtToDlgExt)
//   sakura_core/doc/CDocFileOperation.cpp (SaveFileDialog, the default wildcard)
//
// Copyright (C) 2008, Uchi
// Copyright (C) 2014, syat
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (wide strings and
// `wcstok_s`) was rewritten in Rust, the inputs are passed in instead of being read from the
// document and the shared data, and the tests are new.
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

//! The file types of the save dialog: the wildcard of 「ユーザー指定」 (`SaveFileDialog`), made
//! from the extensions of the document type (`ConvertTypesExtToDlgExt`).

/// Separators of the extension list of a type (`m_typeExtSeps`).
const SEPARATORS: [char; 3] = [' ', ';', ','];
/// Wildcards in an extension (`m_typeExtWildcards`).
const WILDCARDS: [char; 2] = ['*', '?'];

/// The extension of a file name with its dot (`CFilePath::GetExt`), or empty when there is none.
fn extension_of(path: &str) -> &str {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rfind('.').map_or("", |at| &name[at..])
}

/// `ConvertTypesExtToDlgExt`: the extension list of a type as a wildcard list for the dialog.
///
/// `type_exts` is the list of the type (`"c cpp;.h"`: the dot is optional, the separators are a
/// space, `;` and `,`). `first` is the extension, with its dot, that goes to the top of the list
/// (the extension of the file the document has); the same extension is not listed again.
/// `"c cpp;.h"` and `".h"` give `"*.h;*.c;*.cpp"`.
pub fn types_ext_to_dialog_ext(type_exts: &str, first: Option<&str>) -> String {
    let first = first.filter(|ext| !ext.is_empty());
    let mut list = String::new();
    if let Some(ext) = first {
        list.push('*');
        list.push_str(ext);
    }
    for token in type_exts.split(SEPARATORS).filter(|t| !t.is_empty()) {
        // the extension of the file is listed already; compared without the dot, ignoring case
        if first.is_some_and(|ext| token.to_lowercase() == ext[1..].to_lowercase()) {
            continue;
        }
        if !list.is_empty() {
            list.push(';');
        }
        if !token.contains(WILDCARDS) {
            list.push_str(if token.starts_with('.') { "*" } else { "*." });
        }
        list.push_str(token);
    }
    list
}

/// What `SaveFileDialog` knows when it makes the wildcard of 「ユーザー指定」.
#[derive(Clone, Copy, Debug)]
pub struct SaveWildcard<'a> {
    /// The document is of the basic type (type 0), whose extension list is empty.
    pub basic_type: bool,
    /// The extension list of the document type (`szTypeExts`).
    pub type_exts: &'a str,
    /// The path of the file of the document, `None` for a document without a file.
    pub path: Option<&'a str>,
    /// 「新規から保存時は全ファイル表示」 (`bNoFilterSaveNew`).
    pub no_filter_save_new: bool,
    /// 「新規以外から保存時は全ファイル表示」 (`bNoFilterSaveFile`).
    pub no_filter_save_file: bool,
}

impl SaveWildcard<'_> {
    /// The wildcard list of 「ユーザー指定」, `*.cpp;*.h` for example.
    ///
    /// * The basic type lists the extension of the file, or `*.txt` for a document without a file
    ///   or a file without an extension. Any other type lists its own extensions, the extension
    ///   of the file first.
    /// * `;*.*` is added when the option for this kind of document (a new one, or one that has a
    ///   file) says to show all the files.
    pub fn wildcard(&self) -> String {
        let ext = self.path.map_or("", extension_of);
        let mut list = if self.basic_type {
            if ext.is_empty() {
                "*.txt".to_owned()
            } else {
                format!("*{ext}")
            }
        } else {
            types_ext_to_dialog_ext(self.type_exts, Some(ext))
        };
        let all_files = if self.path.is_some() {
            self.no_filter_save_file
        } else {
            self.no_filter_save_new
        };
        if all_files {
            list.push_str(";*.*");
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wildcard(basic: bool, exts: &str, path: Option<&str>, new: bool, file: bool) -> String {
        SaveWildcard {
            basic_type: basic,
            type_exts: exts,
            path,
            no_filter_save_new: new,
            no_filter_save_file: file,
        }
        .wildcard()
    }

    #[test]
    fn the_list_of_a_type_gets_a_star_and_a_dot() {
        assert_eq!(
            types_ext_to_dialog_ext("c cpp;.h,hpp", None),
            "*.c;*.cpp;*.h;*.hpp"
        );
        assert_eq!(types_ext_to_dialog_ext("", None), "");
        assert_eq!(types_ext_to_dialog_ext(" ;; ,", None), "");
    }

    #[test]
    fn a_token_with_a_wildcard_is_kept_as_it_is() {
        assert_eq!(
            types_ext_to_dialog_ext("txt Make* *.log a?c", None),
            "*.txt;Make*;*.log;a?c"
        );
    }

    #[test]
    fn the_extension_of_the_file_goes_first_and_is_not_listed_twice() {
        assert_eq!(
            types_ext_to_dialog_ext("c cpp;h", Some(".h")),
            "*.h;*.c;*.cpp"
        );
        // Sakura compares the token with the extension without its dot: `.h` is listed twice
        assert_eq!(
            types_ext_to_dialog_ext("c cpp;.h", Some(".h")),
            "*.h;*.c;*.cpp;*.h"
        );
        // case does not matter, and the file's spelling is the one that is kept
        assert_eq!(
            types_ext_to_dialog_ext("c cpp;h", Some(".H")),
            "*.H;*.c;*.cpp"
        );
        // an extension the type does not list is added at the top
        assert_eq!(types_ext_to_dialog_ext("c", Some(".xyz")), "*.xyz;*.c");
        // no extension: the list as it is
        assert_eq!(types_ext_to_dialog_ext("c", Some("")), "*.c");
    }

    #[test]
    fn the_extension_of_a_path() {
        assert_eq!(extension_of("/a/b/c.txt"), ".txt");
        assert_eq!(extension_of("C:\\a.b\\c"), "");
        assert_eq!(extension_of("x.tar.gz"), ".gz");
        assert_eq!(extension_of("Makefile"), "");
    }

    #[test]
    fn a_new_basic_document_is_txt() {
        assert_eq!(wildcard(true, "", None, false, false), "*.txt");
        assert_eq!(wildcard(true, "", None, true, false), "*.txt;*.*");
        // the option for named documents does not touch a new one
        assert_eq!(wildcard(true, "", None, false, true), "*.txt");
    }

    #[test]
    fn a_named_basic_document_lists_its_extension() {
        assert_eq!(wildcard(true, "", Some("/a/b.md"), false, false), "*.md");
        assert_eq!(wildcard(true, "", Some("/a/b.md"), false, true), "*.md;*.*");
        // the option for new documents does not touch a named one
        assert_eq!(wildcard(true, "", Some("/a/b.md"), true, false), "*.md");
        // a file without an extension is *.txt
        assert_eq!(
            wildcard(true, "", Some("/a/Makefile"), false, false),
            "*.txt"
        );
    }

    #[test]
    fn a_type_with_several_extensions() {
        let exts = "c cpp cc;h hpp";
        assert_eq!(
            wildcard(false, exts, None, false, false),
            "*.c;*.cpp;*.cc;*.h;*.hpp"
        );
        assert_eq!(
            wildcard(false, exts, None, true, false),
            "*.c;*.cpp;*.cc;*.h;*.hpp;*.*"
        );
        assert_eq!(
            wildcard(false, exts, Some("/src/a.h"), false, true),
            "*.h;*.c;*.cpp;*.cc;*.hpp;*.*"
        );
    }

    #[test]
    fn a_type_without_extensions_has_an_empty_list_before_all_files() {
        // Sakura passes this on as it is.
        assert_eq!(wildcard(false, "", None, true, false), ";*.*");
        assert_eq!(wildcard(false, "", None, false, false), "");
    }
}
