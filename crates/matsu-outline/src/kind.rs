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
    /// TeX: a tree of chapters, sections and the like.
    Tex,
    /// Erlang: a list of functions.
    Erlang,
    /// The rule file of the type (`OUTLINE_FILE`): a tree.
    RuleFile,
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
            outline_type::TEX => Self::Tex,
            outline_type::FILE => Self::RuleFile,
            outline_type::ERLANG => Self::Erlang,
            _ => Self::Text,
        }
    }

    /// List or tree (`SetData`).
    pub fn view(self) -> ViewType {
        match self {
            Self::C
            | Self::Perl
            | Self::VisualBasic
            | Self::PlSql
            | Self::Asm
            | Self::Erlang
            | Self::Bookmark => ViewType::List,
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
            // `SetWindowText( GetHwnd(), L"TeX" )`
            Self::Tex => "TeX",
            Self::RuleFile => "ルールファイル",
            Self::Erlang => "Erlang 関数一覧",
        }
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
    /// The bytes of the rule file of the type (`szOutlineRuleFilename`), for
    /// [`OutlineKind::RuleFile`]; `None` when the file cannot be opened.
    pub rule_file: Option<&'a [u8]>,
}

impl<'a> Request<'a> {
    pub fn new(kind: OutlineKind) -> Self {
        Self {
            kind,
            file_path: None,
            heading_marks: crate::text::DEFAULT_HEADING_MARKS,
            bookmarks: &[],
            ignore_blank_bookmarks: false,
            rule_file: None,
        }
    }
}

/// The result of [`analyze`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Analysis {
    pub items: FuncInfoArr,
    /// The kind the list is shown as (`nListType`): C/C++ is settled by the file name here.
    pub kind: OutlineKind,
    /// The title that replaces the one of the kind (`sTitleOverride`): `;Title=` of a rule file.
    pub title: Option<String>,
    /// A message for the user (a regular expression of the rule file that cannot be compiled;
    /// `ErrorMessage( STR_DOCOUTLINE_REGEX )`). The list is empty then.
    pub error: Option<String>,
}

/// Runs the analyzer of `request.kind` on the lines of `src`.
pub fn analyze(src: &dyn LineSource, request: &Request<'_>) -> Analysis {
    let mut kind = request.kind;
    let mut title = None;
    let mut error = None;
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
        OutlineKind::FileTree => FuncInfoArr::new(),
        OutlineKind::Tex => crate::tex::make_topic_list_tex(src),
        OutlineKind::Erlang => crate::erlang::make_func_list_erlang(src),
        OutlineKind::RuleFile => {
            let outline = crate::rule_file::make_func_list_rule_file(src, request.rule_file);
            title = outline.title;
            error = outline.error;
            outline.items
        }
    };
    Analysis {
        items,
        kind,
        title,
        error,
    }
}
