//! Tests of the analyzers on samples like those of Sakura's own help and sources.
//!
//! Every expected row is `line:depth:info:name`; the expectations follow the rules of the Sakura
//! function that is ported (the comment of each test says which).

use matsu_core::buffer::Document;

use crate::bookmark::make_bookmark_list;
use crate::cpp::{CKind, kind_of_file_name, make_func_list_c};
use crate::html::make_topic_list_html;
use crate::java::make_func_list_java;
use crate::perl::make_func_list_perl;
use crate::python::make_func_list_python;
use crate::text::{DEFAULT_HEADING_MARKS, make_topic_list_txt, make_topic_list_wztxt};
use crate::tree::{
    ListColumn, ListSort, SortType, build_java_tree, build_tree, clip_text_list, clip_text_tree,
    item_at, list_order, prepare_append_texts, sort_tree, vb_remark,
};
use crate::vb::make_func_list_visual_basic;
use crate::{FuncInfoArr, OutlineKind, Request, VecSource, analyze};

fn dump(arr: &FuncInfoArr) -> String {
    arr.items()
        .iter()
        .map(|i| format!("{}:{}:{}:{}", i.line, i.depth, i.info, i.name))
        .collect::<Vec<_>>()
        .join("\n")
}

fn c_list(text: &str, kind: CKind) -> FuncInfoArr {
    make_func_list_c(&VecSource::new(text), kind, None).0
}

fn lines(s: &[&str]) -> String {
    s.iter().map(|l| format!("{l}\n")).collect()
}

// ---- C and C++ (CDocOutline::MakeFuncList_C) ----

#[test]
fn cpp_namespaces_and_nested_classes() {
    let text = lines(&[
        "namespace outer {",
        "namespace inner {",
        "class A {",
        "  class B {",
        "    void f() {}",
        "  };",
        "  void g();",
        "};",
        "}",
        "}",
    ]);
    // a namespace is listed at its definition, a class by its qualified name, the declaration of
    // a member (info 1) and a function (info 2)
    assert_eq!(
        dump(&c_list(&text, CKind::Cpp)),
        "1:0:7:outer::定義位置\n\
         2:0:7:outer::inner::定義位置\n\
         3:0:3:outer::inner::A\n\
         4:0:3:outer::inner::A::B\n\
         5:0:2:outer::inner::A::B::f\n\
         7:0:1:outer::inner::A::g"
    );
}

#[test]
fn cpp_templates_operators_and_constructors() {
    let text = lines(&[
        "template <class T>",
        "class Vec {",
        "public:",
        "  Vec() : size_(0) {}",
        "  T& operator[](int i) { return a[i]; }",
        "  bool operator==(const Vec& o) const;",
        "};",
        "template <typename T>",
        "void swap(T& a, T& b) {",
        "  T t = a;",
        "}",
    ]);
    // the class is listed at the line of "class", not of "template"
    assert_eq!(
        dump(&c_list(&text, CKind::Cpp)),
        "2:0:3:Vec\n\
         4:0:2:Vec::Vec\n\
         5:0:2:Vec::operator[]\n\
         6:0:1:Vec::operator==\n\
         9:0:2:swap"
    );
}

#[test]
fn cpp_skips_comments_strings_and_inactive_preprocessor_branches() {
    let text = lines(&[
        "// void commented() {}",
        "/* void block() {",
        "} */",
        "const char* s = \"void str() {\";",
        "#if 0",
        "void skipped() {}",
        "#else",
        "void used() {}",
        "#endif",
        "void after() {}",
    ]);
    assert_eq!(dump(&c_list(&text, CKind::Cpp)), "8:0:2:used\n10:0:2:after");
}

#[test]
fn cpp_ifdef_branches_after_if_zero_nest() {
    let text = lines(&[
        "#if 0",
        "#ifdef A",
        "void a() {}",
        "#endif",
        "#else",
        "void b() {}",
        "#endif",
        "void c() {}",
    ]);
    assert_eq!(dump(&c_list(&text, CKind::Cpp)), "6:0:2:b\n8:0:2:c");
}

#[test]
fn cpp_multi_line_macros_are_not_code() {
    let text = lines(&["#define M(x) \\", "  void in_macro() {}", "void real() {}"]);
    assert_eq!(dump(&c_list(&text, CKind::Cpp)), "3:0:2:real");
}

#[test]
fn c_lists_functions_and_not_the_declarations_of_k_and_r() {
    let text = lines(&[
        "int add(a, b)",
        "int a, b;",
        "{",
        "  return a + b;",
        "}",
        "void proto(int);",
        "struct S { int x; };",
        "enum E { A, B };",
        "typedef struct T { int y; } T;",
    ]);
    // C: the K&R parameter declarations are not a prototype of "add"
    assert_eq!(
        dump(&c_list(&text, CKind::C)),
        "1:0:2:add\n6:0:1:proto\n7:0:4:S\n8:0:5:E\n9:0:4:T"
    );
    // C++: "int a, b;" after the parentheses is a declaration of the function
    assert_eq!(
        dump(&c_list(&text, CKind::Cpp)),
        "1:0:1:add\n6:0:1:proto\n7:0:4:S\n8:0:5:E\n9:0:4:T"
    );
}

#[test]
fn c_cpp_is_settled_by_the_extension() {
    assert_eq!(kind_of_file_name(Some("a/b/main.c")), CKind::C);
    assert_eq!(kind_of_file_name(Some("C:\\src\\MAIN.C")), CKind::C);
    for name in ["x.cpp", "x.CXX", "x.hpp", "x.h++", "x.c++", "x.hxx"] {
        assert_eq!(kind_of_file_name(Some(name)), CKind::Cpp, "{name}");
    }
    assert_eq!(kind_of_file_name(Some("x.h")), CKind::CCpp);
    assert_eq!(kind_of_file_name(None), CKind::CCpp);
    let (_, kind) = make_func_list_c(&VecSource::new("void f() {}\n"), CKind::CCpp, Some("a.c"));
    assert_eq!(kind, CKind::C);
}

#[test]
fn cpp_raw_strings_do_not_confuse_the_braces() {
    let text = lines(&[
        "const char* r = R\"x(",
        "void inside() {",
        ")x\";",
        "void after() {}",
    ]);
    assert_eq!(dump(&c_list(&text, CKind::Cpp)), "4:0:2:after");
}

#[test]
fn cpp_japanese_names_are_kept() {
    let text = lines(&["void 関数名(int 引数) {", "}"]);
    assert_eq!(dump(&c_list(&text, CKind::Cpp)), "1:0:2:関数名");
}

#[test]
fn cpp_works_with_crlf_lines() {
    let text = "class A {\r\n  void f() {}\r\n};\r\n";
    assert_eq!(dump(&c_list(text, CKind::Cpp)), "1:0:3:A\n2:0:2:A::f");
}

// ---- Java (MakeFuncList_Java) ----

#[test]
fn java_inner_classes_and_methods() {
    let text = lines(&[
        "package a;",
        "public class Outer {",
        "  private int x;",
        "  public Outer(int x) {",
        "    this.x = x;",
        "  }",
        "  public static void main(String[] args) {",
        "    if (args.length > 0) { run(); }",
        "  }",
        "  abstract void todo();",
        "  interface Inner {",
        "    void m();",
        "  }",
        "  class Deep {",
        "    int get() { return 1; }",
        "  }",
        "}",
    ]);
    // Inner classes are joined with a backslash; a call inside a method is not a method
    assert_eq!(
        dump(&make_func_list_java(&VecSource::new(&text))),
        "2:0:0:Outer::定義位置\n\
         4:0:2:Outer::Outer\n\
         7:0:2:Outer::main\n\
         10:0:1:Outer::todo\n\
         11:0:0:Outer\\Inner::定義位置\n\
         12:0:1:Outer\\Inner::m\n\
         14:0:0:Outer\\Deep::定義位置\n\
         15:0:2:Outer\\Deep::get"
    );
}

#[test]
fn java_ignores_comments_and_strings() {
    let text = lines(&[
        "class A {",
        "  // void commented() {}",
        "  /* void block() {} */",
        "  String s = \"void str() {}\";",
        "  void real(",
        "      int a) /* c */ throws E {",
        "  }",
        "}",
    ]);
    // the method name is at the line of "(", the "{" is found after the next line
    assert_eq!(
        dump(&make_func_list_java(&VecSource::new(&text))),
        "1:0:0:A::定義位置\n5:0:2:A::real"
    );
}

// ---- Python (MakeFuncList_python) ----

#[test]
fn python_nests_by_indentation_and_skips_strings() {
    let text = lines(&[
        "import os",
        "",
        "class A(object):",
        "    \"\"\"doc",
        "    def not_a_def(self):",
        "    \"\"\"",
        "    def method(self):",
        "        def inner():",
        "            pass",
        "    # def comment",
        "    x = 'class Z'",
        "",
        "def top(a,",
        "        b):",
        "    return 1",
        "",
        "class B: pass",
        "\tdef tabbed(): pass",
    ]);
    // a class has info 4 and " クラス" after the name, a def info 1; a tab goes to the next 8
    assert_eq!(
        dump(&make_func_list_python(&VecSource::new(&text))),
        "3:0:4:A クラス\n\
         7:1:1:method\n\
         8:2:1:inner\n\
         13:0:1:top\n\
         17:0:4:B クラス\n\
         18:1:1:tabbed"
    );
}

#[test]
fn python_needs_a_blank_after_the_keyword_and_handles_odd_names() {
    let text = lines(&["define x", "classic = 1", "def (x): pass", "class : pass"]);
    // a def without a name is "名称未定", and so is a class without one (Sakura writes the
    // suffix after the end of that name, where it is not seen)
    assert_eq!(
        dump(&make_func_list_python(&VecSource::new(&text))),
        "3:0:1:名称未定\n4:0:4:名称未定"
    );
}

// ---- Perl (MakeFuncList_Perl) ----

#[test]
fn perl_subs_with_package_qualifiers() {
    let text = "package Foo::Bar;\nsub new {\n}\n  sub  Foo::baz  {\n}\nsub'old {}\n#sub no\nmy $x = sub { 1 };\nsub last_one";
    // `sub name` at the start of a line; a name read to the end of the file without a line
    // ending is not added
    assert_eq!(
        dump(&make_func_list_perl(&VecSource::new(text))),
        "2:0:0:new\n4:0:0:Foo::baz"
    );
}

// ---- Visual Basic (MakeFuncList_VisualBasic) ----

const VB_SAMPLE: &[&str] = &[
    "Option Explicit",
    "Public Const MAX = 10",
    "Private Sub Form_Load()",
    "  Const LOCAL = 1",
    "  Dim s As String",
    "  s = \"Sub fake\"",
    "End Sub",
    "Public Static Function Add(a As Long) As Long",
    "End Function",
    "Property Get Name() As String",
    "End Property",
    "Public Enum Color",
    "  Red",
    "End Enum",
    "Private Declare Function GetTick Lib \"kernel32\" () As Long",
    "Public Type Pt",
    "End Type",
    "'Sub comment",
];

#[test]
fn visual_basic_declarations_of_a_module() {
    let text = lines(VB_SAMPLE);
    // info: kind in the low 4 bits, access in bits 4-7, 0x100 Static, 0x200 Declare/Enum/Type/Event
    assert_eq!(
        dump(&make_func_list_visual_basic(
            &VecSource::new(&text),
            Some("a.bas")
        )),
        "2:0:22:MAX\n\
         3:0:34:Form_Load\n\
         4:0:38:LOCAL\n\
         8:0:273:Add\n\
         10:0:3:Name\n\
         12:0:535:Color\n\
         15:0:545:GetTick\n\
         16:0:536:Pt"
    );
}

#[test]
fn visual_basic_class_modules_make_const_and_type_private() {
    let text = lines(VB_SAMPLE);
    let arr = make_func_list_visual_basic(&VecSource::new(&text), Some("C:\\x\\A.CLS"));
    let by_name = |n: &str| arr.items().iter().find(|i| i.name == n).unwrap().info;
    assert_eq!(by_name("MAX"), 0x26);
    assert_eq!(by_name("Pt"), 0x228);
    // the remarks of the list
    assert_eq!(vb_remark(by_name("Add")), "関数（静的 パブリック）");
    assert_eq!(
        vb_remark(by_name("Form_Load")),
        "プロシージャ（プライベート）"
    );
    assert_eq!(vb_remark(by_name("GetTick")), "関数宣言（プライベート）");
    assert_eq!(vb_remark(0), "");
}

#[test]
fn visual_basic_keywords_are_not_case_sensitive() {
    let text = lines(&[
        "public sub lower()",
        "end sub",
        "PRIVATE FUNCTION Upper()",
        "END FUNCTION",
    ]);
    assert_eq!(
        dump(&make_func_list_visual_basic(&VecSource::new(&text), None)),
        "1:0:18:lower\n3:0:33:Upper"
    );
}

// ---- Text (MakeTopicList_txt, MakeTopicList_wztxt) ----

#[test]
fn text_topics_nest_by_the_kind_of_the_heading() {
    let text = lines(&[
        "第1章 はじめに",
        "  1. 概要",
        "  2. 目的",
        "(a) 項目",
        "(1) 数字",
        "ただの文",
        "第2章 つづき",
        "   ① 丸",
        "   ② 丸2",
        "■ 記号",
        "(?)",
    ]);
    // a heading of a kind that was seen goes to its level; "(" needs a digit or a letter after
    assert_eq!(
        dump(&make_topic_list_txt(
            &VecSource::new(&text),
            DEFAULT_HEADING_MARKS
        )),
        "1:0:0:第1章 はじめに\n\
         4:1:0:(a) 項目\n\
         5:2:0:(1) 数字\n\
         7:0:0:第2章 つづき\n\
         8:1:0:① 丸\n\
         9:1:0:② 丸2\n\
         10:2:0:■ 記号"
    );
}

#[test]
fn text_topics_use_the_marks_that_are_given() {
    let text = lines(&["* one", "  ** two", "(a) three", "plain"]);
    // "(" is not a mark here, "*" is
    assert_eq!(
        dump(&make_topic_list_txt(&VecSource::new(&text), "*")),
        "1:0:0:* one\n2:0:0:** two"
    );
}

#[test]
fn text_with_levels_adds_untitled_items_for_skipped_levels() {
    let text = lines(&[".章", "..節", "...項", ".章2", "....深い", "本文"]);
    // the dummy items are not put in the text of the copy button
    let arr = make_topic_list_wztxt(&VecSource::new(&text));
    assert_eq!(
        dump(&arr),
        "1:0:0:1 - 章\n\
         2:1:0:2 - 節\n\
         3:2:0:3 - 項\n\
         4:0:0:1 - 章2\n\
         5:1:65536:(無題)\n\
         5:2:65536:(無題)\n\
         5:3:0:4 - 深い"
    );
    assert_eq!(
        clip_text_tree(&arr, "a.txt", false),
        " 1 - 章\r\n   2 - 節\r\n     3 - 項\r\n 1 - 章2\r\n       4 - 深い\r\n"
    );
}

// ---- HTML and XML (MakeTopicList_html) ----

#[test]
fn html_headings_and_paragraphs_keep_their_order() {
    let text = lines(&[
        "<html>",
        "<body>",
        "<h1>Title</h1>",
        "<p>para one",
        "<p>para two",
        "<h2>Sub</h2>",
        "<div class=\"a\"><b>bold</b>",
        "<br>",
        "<img src=x>",
        "</div>",
        "<!-- <h3>comment</h3> -->",
        "<h2>Sub2</h2>",
        "</body>",
        "</html>",
    ]);
    // a heading is followed by the contents under it; a block closes a paragraph; the tags
    // inside a comment, the physical tags and <br> are left out
    assert_eq!(
        dump(&make_topic_list_html(&VecSource::new(&text), false)),
        "1:0:0:html \n\
         2:1:0:body \n\
         3:2:0:h1 Title\n\
         4:3:0:p para one\n\
         5:3:0:p para two\n\
         6:3:0:h2 Sub\n\
         7:3:0:div \n\
         9:4:0:img src=x\n\
         12:3:0:h2 Sub2"
    );
}

#[test]
fn html_names_are_not_case_sensitive_but_xml_names_are() {
    let text = lines(&["<DIV>", "<Span>x</SPAN>", "</div>", "<after>"]);
    // HTML: </SPAN> closes <Span> and </div> closes <DIV>, so <after> is at the top again
    let html = make_topic_list_html(&VecSource::new(&text), false);
    assert_eq!(dump(&html), "1:0:0:DIV \n2:1:0:Span x\n4:0:0:after ");
    // XML: </SPAN> and </div> match nothing, so nothing is closed
    let xml = make_topic_list_html(&VecSource::new(&text), true);
    assert_eq!(dump(&xml), "1:0:0:DIV \n2:1:0:Span x\n4:2:0:after ");
}

#[test]
fn xml_skips_cdata_and_handles_empty_tags() {
    let text = lines(&[
        "<?xml version=\"1.0\"?>",
        "<root>",
        "<a id=\"1\">text</a>",
        "<![CDATA[ <skip> ]]>",
        "<b/>",
        "</root>",
    ]);
    assert_eq!(
        dump(&make_topic_list_html(&VecSource::new(&text), true)),
        "2:0:0:root \n3:1:0:a text\n5:1:0:b"
    );
}

#[test]
fn the_tag_right_after_a_tag_that_closes_itself_is_read() {
    // Sakura does not read the character after "/>", so it misses the heading here; MATSU Editor
    // reads it (the user's decision, 2026-10-17)
    let text = lines(&["<div/><h1>Title</h1>", "<a/><b/>", "<x></x>"]);
    assert_eq!(
        dump(&make_topic_list_html(&VecSource::new(&text), true)),
        "1:0:0:div\n1:0:0:h1 Title\n2:0:0:a\n2:0:0:b\n3:0:0:x "
    );
}

#[test]
fn html_tag_over_several_lines_is_at_its_last_line() {
    let text = lines(&["<div", "  class=\"x\"", "  id=\"y\">", "text", "</div>"]);
    assert_eq!(
        dump(&make_topic_list_html(&VecSource::new(&text), false)),
        "3:0:0:div "
    );
}

// ---- bookmarks (MakeFuncList_BookMark) ----

#[test]
fn bookmark_list_trims_both_ends() {
    let text = lines(&["  first line  ", "", "\t\t", "　third　", "last"]);
    let src = VecSource::new(&text).with_new_line_len(1);
    // without 空行を無視する every marked line is listed
    assert_eq!(
        dump(&make_bookmark_list(&src, [0, 1, 2, 3, 4], false)),
        "1:0:0:first line\n2:0:0:\n3:0:0:\n4:0:0:third\n5:0:0:last"
    );
    // with it a line of blanks is left out
    assert_eq!(
        dump(&make_bookmark_list(&src, [0, 1, 2, 3, 4], true)),
        "1:0:0:first line\n4:0:0:third\n5:0:0:last"
    );
}

#[test]
fn bookmark_list_blank_test_uses_the_new_line_length_of_the_document() {
    // a line that is no longer than the line ending of the document is only a line ending
    let text = "\r\nx\r\n";
    let crlf = VecSource::new(text).with_new_line_len(2);
    assert_eq!(dump(&make_bookmark_list(&crlf, [0, 1], true)), "2:0:0:x");
}

#[test]
fn bookmark_list_stops_at_a_line_that_does_not_exist() {
    let src = VecSource::new("a\nb\n");
    assert_eq!(dump(&make_bookmark_list(&src, [0, 5, 1], false)), "1:0:0:a");
}

// ---- analyze: the kind decides ----

#[test]
fn analyze_runs_the_analyzer_of_the_kind() {
    let src = VecSource::new("void f() {}\n");
    let mut request = Request::new(OutlineKind::CCpp);
    request.file_path = Some("x.c");
    let analysis = analyze(&src, &request);
    assert_eq!(analysis.kind, OutlineKind::C);
    assert_eq!(dump(&analysis.items), "1:0:2:f");

    // an unknown value of the setting is the text topics, as in Sakura
    assert_eq!(OutlineKind::from_setting(99), OutlineKind::Text);
    // a kind that is not ported gives an empty list
    let rule = OutlineKind::from_setting(matsu_config::settings::type_rules::outline_type::FILE);
    assert!(!rule.is_supported());
    assert!(analyze(&src, &Request::new(rule)).items.is_empty());
}

#[test]
fn views_of_the_kinds() {
    use crate::ViewType::{List, Tree};
    for (kind, view) in [
        (OutlineKind::C, List),
        (OutlineKind::Perl, List),
        (OutlineKind::VisualBasic, List),
        (OutlineKind::Bookmark, List),
        (OutlineKind::Cpp, Tree),
        (OutlineKind::Java, Tree),
        (OutlineKind::Python, Tree),
        (OutlineKind::Text, Tree),
        (OutlineKind::Html, Tree),
    ] {
        assert_eq!(kind.view(), view, "{kind:?}");
    }
}

#[test]
fn a_document_is_a_source_with_its_line_endings() {
    let doc = Document::from_utf8(b"class A {\r\n  void f() {}\r\n};".to_vec()).unwrap();
    let arr = analyze(&doc, &Request::new(OutlineKind::Cpp)).items;
    assert_eq!(dump(&arr), "1:0:3:A\n2:0:2:A::f");
    // a line ending is part of a line: the comment of a line ends at it
    let doc = Document::from_utf8(b"// c\nvoid g() {}\n".to_vec()).unwrap();
    assert_eq!(
        dump(&analyze(&doc, &Request::new(OutlineKind::Cpp)).items),
        "2:0:2:g"
    );
}

// ---- the tree and the list ----

#[test]
fn the_tree_follows_the_depth() {
    let text = lines(&["第1章", "(a) x", "(1) y", "第2章"]);
    let arr = make_topic_list_txt(&VecSource::new(&text), DEFAULT_HEADING_MARKS);
    let tree = build_tree(&arr);
    assert_eq!(tree.len(), 2);
    assert_eq!(tree[0].label, "第1章");
    assert_eq!(tree[0].children[0].label, "(a) x");
    assert_eq!(tree[0].children[0].children[0].label, "(1) y");
    assert_eq!(tree[0].children[0].children[0].item, Some(2));
    assert_eq!(tree[1].label, "第2章");
}

#[test]
fn a_very_deep_tree_is_cut_at_a_depth_the_traversals_can_follow() {
    // a line of 20 000 dots is a level for every dot
    let text = format!("{}deep\n", ".".repeat(20_000));
    let arr = make_topic_list_wztxt(&VecSource::new(&text));
    assert!(arr.len() > 19_000);
    let mut tree = build_tree(&arr);
    sort_tree(&mut tree, SortType::AtoZ);
    let mut depth = 0;
    let mut node = &tree[0];
    while let Some(child) = node.children.first() {
        depth += 1;
        node = child;
    }
    assert_eq!(depth, crate::tree::MAX_TREE_DEPTH);
}

#[test]
fn the_java_tree_groups_members_under_their_classes() {
    let text = lines(&[
        "namespace ns {",
        "class Foo {",
        "  void bar();",
        "};",
        "}",
        "void Foo::baz() {}",
        "void ns::Foo::bar() {}",
        "int main() { return 0; }",
        "struct S {};",
    ]);
    let mut arr = c_list(&text, CKind::Cpp);
    prepare_append_texts(&mut arr);
    let tree = build_java_tree(&arr);
    let top: Vec<_> = tree.iter().map(|n| n.label.as_str()).collect();
    assert_eq!(top, ["ns 名前空間", "Foo クラス", "グローバル", "S 構造体"]);
    let ns = &tree[0];
    let ns_labels: Vec<_> = ns.children.iter().map(|n| n.label.as_str()).collect();
    assert_eq!(ns_labels, ["定義位置", "Foo クラス"]);
    let foo = &ns.children[1];
    // "ns::Foo" is the class item itself, with the declaration and the definition of bar
    assert_eq!(foo.label, "Foo クラス");
    let labels: Vec<_> = foo.children.iter().map(|n| n.label.as_str()).collect();
    assert_eq!(labels, ["bar(宣言)", "bar"]);
    let global: Vec<_> = tree[2].children.iter().map(|n| n.label.as_str()).collect();
    assert_eq!(global, ["main"]);
    // the nodes made for a class or the global space have no item of the analysis
    assert_eq!(tree[2].item, None);
    assert!(foo.children[0].item.is_some());
}

#[test]
fn trees_sort_by_name_and_by_order() {
    let text = lines(&["第1章", "(b) b", "(a) a", "(c) c"]);
    let arr = make_topic_list_txt(&VecSource::new(&text), DEFAULT_HEADING_MARKS);
    // (b), (a) and (c) are all headings of the kind "(a)": the same level
    let mut tree = build_tree(&arr);
    assert_eq!(tree[0].children.len(), 3);
    let labels = |t: &[crate::TreeNode]| {
        t[0].children
            .iter()
            .map(|n| n.label.clone())
            .collect::<Vec<_>>()
    };
    sort_tree(&mut tree, SortType::AtoZ);
    assert_eq!(labels(&tree), ["(a) a", "(b) b", "(c) c"]);
    sort_tree(&mut tree, SortType::ZtoA);
    assert_eq!(labels(&tree), ["(c) c", "(b) b", "(a) a"]);
    sort_tree(&mut tree, SortType::DefaultDesc);
    assert_eq!(labels(&tree), ["(c) c", "(a) a", "(b) b"]);
    sort_tree(&mut tree, SortType::Default);
    assert_eq!(labels(&tree), ["(b) b", "(a) a", "(c) c"]);
}

#[test]
fn the_list_sorts_by_column_and_direction() {
    let arr = c_list(
        &lines(&["void b() {}", "void a() {}", "void c();"]),
        CKind::Cpp,
    );
    let order = |sort| list_order(&arr, sort);
    assert_eq!(order(ListSort::default()), [0, 1, 2]);
    let by_name = ListSort::default().click(ListColumn::Name);
    assert_eq!(order(by_name), [1, 0, 2]);
    // the same column again turns the direction
    assert_eq!(order(by_name.click(ListColumn::Name)), [2, 0, 1]);
    // the remark column sorts by the kind (a prototype is 1, a function 2)
    let by_remark = ListSort::default().click(ListColumn::Remark);
    assert_eq!(order(by_remark), [2, 0, 1]);
}

#[test]
fn the_item_for_the_caret() {
    let text = lines(&["第1章", "text", "第2章", "more", "(a) x"]);
    let arr = make_topic_list_txt(&VecSource::new(&text), DEFAULT_HEADING_MARKS);
    // items at lines 1, 3, 5
    assert_eq!(item_at(&arr, 1, 1), Some(0));
    assert_eq!(item_at(&arr, 2, 1), Some(0));
    assert_eq!(item_at(&arr, 3, 1), Some(1));
    assert_eq!(item_at(&arr, 4, 9), Some(1));
    assert_eq!(item_at(&arr, 100, 1), Some(2));
    assert_eq!(item_at(&FuncInfoArr::new(), 1, 1), None);
}

#[test]
fn a_large_document_is_read_line_by_line() {
    // 60 000 functions: the analysis reads the lines of the document one at a time
    let mut text = String::new();
    for i in 0..60_000 {
        text.push_str(&format!("int f{i}(int a) {{\n  return a;\n}}\n"));
    }
    let doc = Document::from_utf8(text.into_bytes()).unwrap();
    let arr = analyze(&doc, &Request::new(OutlineKind::Cpp)).items;
    assert_eq!(arr.len(), 60_000);
    assert_eq!(arr.items()[59_999].name, "f59999");
    assert_eq!(arr.items()[59_999].line, 3 * 59_999 + 1);
}

#[test]
fn the_text_of_the_copy_button() {
    let arr = c_list(&lines(&["void f();", "void g() {}"]), CKind::Cpp);
    assert_eq!(
        clip_text_list(&arr, "a.cpp"),
        "a.cpp(1,1): f(宣言)\r\na.cpp(2,1): g\r\n"
    );
}
