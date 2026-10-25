// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/cmd/CViewCommander_Outline.cpp, sakura_core/outline/CDlgFuncList.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2014, the authors named in the original files (genta, zenryaku, Moca,
//     aroka, hor, YAZAKI, little YOSHI, ryoji, syat and others)
//   Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (a UTF-16 buffer
// walked with `CDocLineMgr`, the results put in `CFuncInfoArr`, a Windows dialog) was rewritten in
// Rust on the lines of `LineSource`, the positions in the layout are left to the window, and the
// tests are new.
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

//! The kinds of outline, and the function that runs the analyzer of a kind.
//!
//! Sakura: `CViewCommander::Command_FUNCLIST` (`sakura_core/cmd/CViewCommander_Outline.cpp`)
//! chooses the analyzer; `CDlgFuncList::SetData` (`sakura_core/outline/CDlgFuncList.cpp`) chooses
//! the way the result is shown and the title of the dialog.

use matsu_config::settings::type_rules::outline_type;

use crate::cpp::{CKind, make_func_list_c};
use crate::{FuncInfoArr, LineSource};

/// The outline kinds that are analyzed (`EOutlineType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutlineKind {
    /// C: a list of functions.
    C,
    /// C/C++: C or C++ by the extension of the file (see [`crate::cpp::kind_of_file_name`]).
    CCpp,
    /// C++: a tree.
    Cpp,
    Java,
    Perl,
    VisualBasic,
    /// Topics that start with a heading mark.
    Text,
    /// Text with levels (WZ editor style).
    Wztxt,
    Html,
    Xml,
    Python,
    /// PL/SQL: a list of functions, procedures and packages.
    PlSql,
    /// COBOL: a tree of labels under their divisions.
    Cobol,
    /// Assembler: a list of labels and procedures.
    Asm,
    /// The bookmarked lines (`F_BOOKMARK_VIEW`).
    Bookmark,
    /// The file tree (`F_FILETREE`, `OUTLINE_FILETREE`): nothing is analyzed in the document
    /// (`Command_FUNCLIST` does nothing for it); the pane builds the tree from the settings.
    FileTree,
    /// A kind that is not ported yet: TeX, the rule file or Erlang
    /// (the value is `nDefaultOutline`). The list is empty.
    Unsupported(i32),
}

/// How a result is shown (`VIEWTYPE_LIST`, `VIEWTYPE_TREE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewType {
    List,
    Tree,
}

impl OutlineKind {
    /// The kind for `nDefaultOutline` of a document type. A value that Sakura does not know
    /// falls to the text topics, as `Command_FUNCLIST` does (`default:`).
    pub fn from_setting(value: i32) -> Self {
        match value {
            outline_type::C => Self::C,
            outline_type::C_CPP => Self::CCpp,
            outline_type::CPP => Self::Cpp,
            outline_type::JAVA => Self::Java,
            outline_type::PERL => Self::Perl,
            outline_type::VB => Self::VisualBasic,
            outline_type::TEXT => Self::Text,
            outline_type::WZTXT => Self::Wztxt,
            outline_type::HTML => Self::Html,
            outline_type::XML => Self::Xml,
            outline_type::PYTHON => Self::Python,
            outline_type::PLSQL => Self::PlSql,
            outline_type::COBOL => Self::Cobol,
            outline_type::ASM => Self::Asm,
            outline_type::TEX | outline_type::FILE | outline_type::ERLANG => {
                Self::Unsupported(value)
            }
            _ => Self::Text,
        }
    }

    /// List or tree (`SetData`).
    pub fn view(self) -> ViewType {
        match self {
            Self::C | Self::Perl | Self::VisualBasic | Self::PlSql | Self::Asm | Self::Bookmark => {
                ViewType::List
            }
            Self::Unsupported(v) => {
                // Erlang is a list; TeX and the rule file are trees
                if v == outline_type::ERLANG {
                    ViewType::List
                } else {
                    ViewType::Tree
                }
            }
            _ => ViewType::Tree,
        }
    }

    /// The title of the pane (`STR_DLGFNCLST_TITLE_*`).
    pub fn title(self) -> &'static str {
        match self {
            Self::C => "C 関数一覧",
            Self::CCpp | Self::Cpp => "C++ メソッドツリー",
            Self::Java => "Java メソッドツリー",
            Self::Perl => "Perl 関数一覧",
            Self::VisualBasic => "Visual Basic アウトライン",
            Self::Text => "テキスト トピックツリー",
            Self::Wztxt => "WZ階層付テキスト",
            Self::Html => "HTML",
            Self::Xml => "XML",
            Self::Python => "Python メソッドツリー",
            Self::PlSql => "PL/SQL 関数一覧",
            Self::Cobol => "COBOL アウトライン",
            Self::Asm => "アセンブラ アウトライン",
            Self::Bookmark => "ブックマーク",
            // `LS(F_FILETREE)`, the name of the function
            Self::FileTree => "ファイルツリー",
            Self::Unsupported(v) => match v {
                outline_type::TEX => "TeX",
                outline_type::FILE => "ルールファイル",
                _ => "Erlang 関数一覧",
            },
        }
    }

    /// Whether the analyzer of this kind exists here.
    pub fn is_supported(self) -> bool {
        !matches!(self, Self::Unsupported(_))
    }
}

/// What an analysis needs besides the lines.
#[derive(Clone, Debug)]
pub struct Request<'a> {
    pub kind: OutlineKind,
    /// The path of the file, for C/C++ (the extension) and Visual Basic (`.cls`).
    pub file_path: Option<&'a str>,
    /// 見出し記号 for the text topics ([`crate::text::DEFAULT_HEADING_MARKS`]).
    pub heading_marks: &'a str,
    /// The bookmarked lines, 0 origin, for [`OutlineKind::Bookmark`].
    pub bookmarks: &'a [usize],
    /// 空行を無視する of the bookmark list.
    pub ignore_blank_bookmarks: bool,
}

impl<'a> Request<'a> {
    pub fn new(kind: OutlineKind) -> Self {
        Self {
            kind,
            file_path: None,
            heading_marks: crate::text::DEFAULT_HEADING_MARKS,
            bookmarks: &[],
            ignore_blank_bookmarks: false,
        }
    }
}

/// The result of [`analyze`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Analysis {
    pub items: FuncInfoArr,
    /// The kind the list is shown as (`nListType`): C/C++ is settled by the file name here.
    pub kind: OutlineKind,
}

/// Runs the analyzer of `request.kind` on the lines of `src`.
pub fn analyze(src: &dyn LineSource, request: &Request<'_>) -> Analysis {
    let mut kind = request.kind;
    let items = match request.kind {
        OutlineKind::C | OutlineKind::CCpp | OutlineKind::Cpp => {
            let c_kind = match request.kind {
                OutlineKind::C => CKind::C,
                OutlineKind::Cpp => CKind::Cpp,
                _ => CKind::CCpp,
            };
            let (items, settled) = make_func_list_c(src, c_kind, request.file_path);
            kind = match settled {
                CKind::C => OutlineKind::C,
                // "C/C++" that no extension decides is shown as C++ (the tree)
                CKind::CCpp => OutlineKind::CCpp,
                CKind::Cpp => OutlineKind::Cpp,
            };
            items
        }
        OutlineKind::Java => crate::java::make_func_list_java(src),
        OutlineKind::Perl => crate::perl::make_func_list_perl(src),
        OutlineKind::VisualBasic => crate::vb::make_func_list_visual_basic(src, request.file_path),
        OutlineKind::Text => crate::text::make_topic_list_txt(src, request.heading_marks),
        OutlineKind::Wztxt => crate::text::make_topic_list_wztxt(src),
        OutlineKind::Html => crate::html::make_topic_list_html(src, false),
        OutlineKind::Xml => crate::html::make_topic_list_html(src, true),
        OutlineKind::Python => crate::python::make_func_list_python(src),
        OutlineKind::PlSql => crate::plsql::make_func_list_plsql(src),
        OutlineKind::Cobol => crate::cobol::make_topic_list_cobol(src),
        OutlineKind::Asm => crate::asm::make_topic_list_asm(src),
        OutlineKind::Bookmark => crate::bookmark::make_bookmark_list(
            src,
            request.bookmarks.iter().copied(),
            request.ignore_blank_bookmarks,
        ),
        // `case OUTLINE_FILETREE: /* 特に何もしない*/` (`Command_FUNCLIST`)
        OutlineKind::FileTree | OutlineKind::Unsupported(_) => FuncInfoArr::new(),
    };
    Analysis { items, kind }
}
