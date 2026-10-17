//! UME Editor's own document types and keyword sets (ADR-0017), for the languages Sakura has no type for.
//!
//! Sakura starts with 17 types and 17 keyword sets. UME Editor adds the types below, in this order, to the
//! numbers after 17 that the settings file does not use for a type of its own (see
//! [`DocumentTypes::from_profile`](super::DocumentTypes::from_profile)), and the keyword sets below to the
//! numbers after the sets of the file ([`KeywordSets`]).
//!
//! | name | extensions | keyword set |
//! |------|------------|-------------|
//! | JavaScript | js, mjs, cjs, jsx | JavaScript |
//! | TypeScript | ts, tsx, mts, cts | TypeScript |
//! | JSON | json, jsonc | JSON |
//! | CSS | css | CSS |
//! | Markdown | md, markdown | (none) |
//! | Rust | rs | Rust |
//! | Go | go | Go |
//! | Ruby | rb, rake, gemspec | Ruby |
//! | シェルスクリプト | sh, bash, zsh | Shell |
//! | YAML | yml, yaml | YAML |
//!
//! The Python type of Sakura also gets the set [`ExtraSet::PythonBuiltins`] as its second keyword set.
//!
//! The words come from the language specifications named in the files of
//! [`keyword_data`](super::keyword_data), never from another editor's syntax file. The rules are the ones the
//! coloring of Sakura can express, with the values of the closest Sakura type: the C-like languages take the
//! values of "C/C++" (the tab width 4, the C comments, backslash escapes in strings that end with the line,
//! numbers and bracket pairs shown, the C smart indent); the others take the values of "基本" for what they do
//! not say. The outline is "Text" for all of them (Sakura has no outline rule for these languages).
//!
//! What the rules cannot say exactly (the coloring of these types is approximate there):
//!
//! * JavaScript, TypeScript: template literals (backquotes) are not strings, so they are not colored and a
//!   `//` inside one starts a comment; regular expression literals are not recognized
//! * Rust: the single quote is not a string quote (a lifetime such as `'a` would start a string that ends
//!   with the line), so character literals are not colored; block comments do not nest; raw strings
//!   (`r#"..."#`) end at the first `"` that a backslash does not escape
//! * Go: raw strings (backquotes) are not strings
//! * Ruby: `=begin` and `=end` are comment delimiters anywhere in a line, not only at its start; `%w[]`,
//!   heredocs and regular expression literals are not recognized
//! * Shell: `$#` and `${#x}` start a comment at the `#`; a single quoted string takes a backslash as an
//!   escape; heredocs are not recognized
//! * YAML: the single quote is not a string quote (a plain scalar such as `don't` would start a string); block
//!   scalars are not recognized
//! * Markdown: only the headings (a regular expression keyword) and the HTML comments are colored; the single
//!   and double quotes are not strings
//! * CSS: the at-rule names with a hyphen (`@font-face`) are not keywords; property names are not keywords
//! * JSON: `//` and `/* */` are comments for the sake of `.jsonc`

use super::builtin_types::{named, set_disp};
use super::color::names;
use super::keyword_data as data;
use super::keywords::{KeywordSet, KeywordSets, SAKURA_KEYWORD_SET_COUNT};
use super::type_rules::{
    BlockComment, DEFAULT_REGEX_COLOR, LineComment, RegexKeyword, outline_type, smart_indent,
};
use super::types::TypeSettings;

/// UME Editor's own keyword sets, in the order they are added after the sets of the settings file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtraSet {
    PythonBuiltins,
    JavaScript,
    TypeScript,
    Json,
    Css,
    Rust,
    Go,
    Ruby,
    Shell,
    Yaml,
}

impl ExtraSet {
    /// All the sets, in order.
    pub const ALL: [ExtraSet; 10] = [
        ExtraSet::PythonBuiltins,
        ExtraSet::JavaScript,
        ExtraSet::TypeScript,
        ExtraSet::Json,
        ExtraSet::Css,
        ExtraSet::Rust,
        ExtraSet::Go,
        ExtraSet::Ruby,
        ExtraSet::Shell,
        ExtraSet::Yaml,
    ];

    /// The position in [`ExtraSet::ALL`].
    pub fn ordinal(self) -> usize {
        Self::ALL.iter().position(|&s| s == self).unwrap_or(0)
    }

    /// The index the set has when the file has Sakura's 17 sets only.
    pub fn default_index(self) -> i32 {
        i32::try_from(SAKURA_KEYWORD_SET_COUNT + self.ordinal()).unwrap_or(-1)
    }

    /// The set that has this index when the file has Sakura's 17 sets only, if there is one.
    pub fn from_default_index(index: i32) -> Option<ExtraSet> {
        let i = usize::try_from(index)
            .ok()?
            .checked_sub(SAKURA_KEYWORD_SET_COUNT)?;
        Self::ALL.get(i).copied()
    }

    fn name(self) -> &'static str {
        match self {
            ExtraSet::PythonBuiltins => "Python built-ins",
            ExtraSet::JavaScript => "JavaScript",
            ExtraSet::TypeScript => "TypeScript",
            ExtraSet::Json => "JSON",
            ExtraSet::Css => "CSS",
            ExtraSet::Rust => "Rust",
            ExtraSet::Go => "Go",
            ExtraSet::Ruby => "Ruby",
            ExtraSet::Shell => "Shell",
            ExtraSet::Yaml => "YAML",
        }
    }

    /// CSS ignores case; the others are case sensitive.
    fn case_sensitive(self) -> bool {
        self != ExtraSet::Css
    }

    fn words(self) -> &'static [&'static str] {
        match self {
            ExtraSet::PythonBuiltins => data::python_builtins::WORDS,
            ExtraSet::JavaScript => data::javascript::WORDS,
            ExtraSet::TypeScript => data::typescript::WORDS,
            ExtraSet::Json => data::json::WORDS,
            ExtraSet::Css => data::css::WORDS,
            ExtraSet::Rust => data::rust::WORDS,
            ExtraSet::Go => data::go::WORDS,
            ExtraSet::Ruby => data::ruby::WORDS,
            ExtraSet::Shell => data::shell::WORDS,
            ExtraSet::Yaml => data::yaml::WORDS,
        }
    }

    /// The set, ready to be put in [`KeywordSets`].
    pub(super) fn to_set(self) -> KeywordSet {
        KeywordSet::new(
            self.name(),
            self.case_sensitive(),
            self.words().iter().map(|w| (*w).to_owned()),
        )
    }
}

/// UME Editor's own types, in order. `sets` says where each keyword set is (a type whose set did not fit in
/// the numbers of the file has no keywords). The index and the id of every type are 0: the caller gives the
/// number of the slot.
pub(super) fn extra_types(sets: &KeywordSets) -> Vec<TypeSettings> {
    let kw = |s: ExtraSet| sets.extra_index(s);
    // The values of "C/C++" (`CType_Cpp.cpp`), without the `#if 0` block.
    let c_like = |name: &str, exts: &str, set: ExtraSet| {
        let mut t = named(0, name, exts);
        t.line_comments[0] = LineComment::new("//", -1);
        t.block_comments[0] = BlockComment::new("/*", "*/");
        t.keyword_set_idx[0] = kw(set);
        t.default_outline = outline_type::TEXT;
        t.smart_indent = smart_indent::CPP;
        set_disp(&mut t, names::NUMBER, true);
        set_disp(&mut t, names::BRACKET_PAIR, true);
        t.string_line_only = true;
        t
    };

    let javascript = c_like("JavaScript", "js,mjs,cjs,jsx", ExtraSet::JavaScript);
    let typescript = c_like("TypeScript", "ts,tsx,mts,cts", ExtraSet::TypeScript);

    // RFC 8259 has no comments, but `.jsonc` has: outside a string, `//` can mean nothing else in JSON.
    let mut json = c_like("JSON", "json,jsonc", ExtraSet::Json);
    json.smart_indent = smart_indent::NONE;
    set_disp(&mut json, names::SINGLE_QUOTE, false);

    let mut css = c_like("CSS", "css", ExtraSet::Css);
    css.line_comments[0] = LineComment::default();
    css.smart_indent = smart_indent::NONE;

    // The headings (`#` to `######` at the start of a line) are a regular expression keyword. Quotes are
    // ordinary text in prose, as in "テキスト".
    let mut markdown = named(0, "Markdown", "md,markdown");
    markdown.block_comments[0] = BlockComment::new("<!--", "-->");
    markdown.default_outline = outline_type::TEXT;
    set_disp(&mut markdown, names::SINGLE_QUOTE, false);
    set_disp(&mut markdown, names::DOUBLE_QUOTE, false);
    markdown.use_regex_keyword = true;
    markdown.regex_keywords = vec![RegexKeyword {
        color: DEFAULT_REGEX_COLOR.to_owned(),
        pattern: r"/^#{1,6}[ \t].*/k".to_owned(),
    }];

    let mut rust = c_like("Rust", "rs", ExtraSet::Rust);
    set_disp(&mut rust, names::SINGLE_QUOTE, false);

    let go = c_like("Go", "go", ExtraSet::Go);

    let mut ruby = named(0, "Ruby", "rb,rake,gemspec");
    ruby.line_comments[0] = LineComment::new("#", -1);
    ruby.block_comments[0] = BlockComment::new("=begin", "=end");
    ruby.keyword_set_idx[0] = kw(ExtraSet::Ruby);
    ruby.default_outline = outline_type::TEXT;
    set_disp(&mut ruby, names::NUMBER, true);
    set_disp(&mut ruby, names::BRACKET_PAIR, true);
    ruby.string_line_only = true;

    let mut shell = named(0, "シェルスクリプト", "sh,bash,zsh");
    shell.line_comments[0] = LineComment::new("#", -1);
    shell.keyword_set_idx[0] = kw(ExtraSet::Shell);
    shell.default_outline = outline_type::TEXT;
    set_disp(&mut shell, names::BRACKET_PAIR, true);
    shell.string_line_only = true;

    let mut yaml = named(0, "YAML", "yml,yaml");
    yaml.line_comments[0] = LineComment::new("#", -1);
    yaml.keyword_set_idx[0] = kw(ExtraSet::Yaml);
    yaml.default_outline = outline_type::TEXT;
    set_disp(&mut yaml, names::NUMBER, true);
    set_disp(&mut yaml, names::SINGLE_QUOTE, false);
    yaml.string_line_only = true;

    vec![
        javascript, typescript, json, css, markdown, rust, go, ruby, shell, yaml,
    ]
}

#[cfg(test)]
mod tests {
    use super::super::DocumentTypes;
    use super::super::builtin_types::{BUILTIN_TYPE_COUNT, builtin_types};
    use super::*;
    use crate::profile::Profile;

    const NAMES: [&str; 10] = [
        "JavaScript",
        "TypeScript",
        "JSON",
        "CSS",
        "Markdown",
        "Rust",
        "Go",
        "Ruby",
        "シェルスクリプト",
        "YAML",
    ];

    fn profile(text: &str) -> Profile {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        Profile::from_bytes(&bytes)
    }

    fn names_from(types: &DocumentTypes, first: usize, n: usize) -> Vec<String> {
        (first..first + n)
            .map(|i| types.get(i).unwrap().type_name.clone())
            .collect()
    }

    /// What Sakura writes for a number it does not use (name `設定N`, no extensions) and some other keys.
    fn placeholder(n: usize) -> String {
        format!(
            "[Types({n})]\r\nnInts={n},10240,0,4,-1,-1,0,1,1,0,0,0\r\nszTypeName=設定{n}\r\nszTypeExts=\r\n\
             szLineComment=\r\nbAutoIndent=1\r\n"
        )
    }

    #[test]
    fn without_a_settings_file_the_types_follow_sakuras_17_in_order() {
        let types = DocumentTypes::builtin();
        assert_eq!(types.len(), 30);
        assert_eq!(names_from(&types, 17, 10), NAMES);
        for i in 17..27 {
            let t = types.get(i).unwrap();
            assert_eq!(t.idx as usize, i);
            assert_eq!(t.id as usize, i);
        }
        // The slots after UME Editor's types are the copies of the basic type, as before.
        assert_eq!(types.get(27).unwrap().type_name, "設定27");
        assert_eq!(types.get(29).unwrap().type_exts, "");
    }

    #[test]
    fn sakuras_17_types_are_not_changed() {
        // Only Python gets a second keyword set; everything else is what `CType_*.cpp` says.
        let mut before = builtin_types();
        let python = 13;
        assert_eq!(before[python].keyword_set_idx[1], 17);
        before[python].keyword_set_idx[1] = -1;
        let types = DocumentTypes::builtin();
        for (i, t) in before.iter().enumerate() {
            let mut t = t.clone();
            if i == python {
                t.keyword_set_idx[1] = 17;
            }
            assert_eq!(types.get(i), Some(&t), "type {i}");
        }
        assert_eq!(builtin_types().len(), BUILTIN_TYPE_COUNT);
        let sets = KeywordSets::builtin();
        assert_eq!(sets.get(16).unwrap().name, "Python");
        assert_eq!(sets.get(17).unwrap().name, "Python built-ins");
    }

    #[test]
    fn none_of_the_extensions_belongs_to_a_type_of_sakura() {
        let sakura = builtin_types();
        for t in extra_types(&KeywordSets::builtin()) {
            for ext in t.type_exts.split(',') {
                for s in &sakura {
                    assert!(
                        !super::super::doc_types::is_file_name_match(&s.type_exts, ext),
                        "{ext} of {} is taken by {}",
                        t.type_name,
                        s.type_name
                    );
                }
            }
        }
    }

    #[test]
    fn files_are_given_the_type_of_their_language() {
        let types = DocumentTypes::builtin();
        let name_of = |f: &str| {
            types
                .get(types.type_index_for_file_name(f))
                .unwrap()
                .type_name
                .clone()
        };
        for (file, name) in [
            ("app.js", "JavaScript"),
            ("app.MJS", "JavaScript"),
            ("a.cjs", "JavaScript"),
            ("view.jsx", "JavaScript"),
            ("app.ts", "TypeScript"),
            ("view.tsx", "TypeScript"),
            ("a.mts", "TypeScript"),
            ("a.cts", "TypeScript"),
            ("package.json", "JSON"),
            ("tsconfig.jsonc", "JSON"),
            ("site.css", "CSS"),
            ("README.md", "Markdown"),
            ("notes.markdown", "Markdown"),
            ("main.rs", "Rust"),
            ("main.go", "Go"),
            ("app.rb", "Ruby"),
            ("tasks.rake", "Ruby"),
            ("my.gemspec", "Ruby"),
            ("run.sh", "シェルスクリプト"),
            ("run.bash", "シェルスクリプト"),
            ("run.zsh", "シェルスクリプト"),
            ("ci.yml", "YAML"),
            ("ci.yaml", "YAML"),
            // Sakura's own types stay where they were.
            ("a.py", "Python"),
            ("a.cpp", "C/C++"),
            ("a.html", "HTML"),
            ("a.txt", "テキスト"),
            ("a.xyz", "基本"),
        ] {
            assert_eq!(name_of(file), name, "{file}");
        }
    }

    #[test]
    fn a_sakura_file_with_all_its_sections_gives_the_unused_ones_to_umes_types() {
        // Sakura writes 30 sections: 17 types and 13 placeholders.
        let mut text = String::from("[Other]\r\nnTypesCount=30\r\n");
        for n in 17..30 {
            text.push_str(&placeholder(n));
        }
        let types = DocumentTypes::from_profile(&profile(&text));
        assert_eq!(types.len(), 30);
        assert_eq!(names_from(&types, 17, 10), NAMES);
        assert_eq!(types.get(27).unwrap().type_name, "設定27");
        assert_eq!(types.type_index_for_file_name("lib.rs"), 22);
        // The keys of the placeholder did not leak in.
        assert_eq!(types.get(17).unwrap().keyword_set_idx[0], 18);
    }

    #[test]
    fn a_type_of_the_file_keeps_its_number_and_umes_types_go_to_the_free_ones() {
        let text = "[Other]\r\nnTypesCount=30\r\n\
            [Types(17)]\r\nszTypeName=Mine\r\nszTypeExts=mine\r\n\
            [Types(18)]\r\nszTypeName=設定18\r\nszTypeExts=\r\n\
            [Types(19)]\r\nszTypeName=Zig\r\nszTypeExts=zig\r\nszLineComment=//\r\nnLineCommentColumn=-1\r\n";
        let types = DocumentTypes::from_profile(&profile(text));
        assert_eq!(types.get(17).unwrap().type_name, "Mine");
        // Read the way Sakura does: a copy of the basic type with the keys of the file.
        assert_eq!(types.get(19).unwrap().type_name, "Zig");
        assert_eq!(types.get(19).unwrap().line_comments[0].delimiter, "//");
        // The free numbers are 18 and 20 to 29: ten of them.
        let free = [18, 20, 21, 22, 23, 24, 25, 26, 27, 28];
        for (n, name) in free.iter().zip(NAMES) {
            assert_eq!(types.get(*n).unwrap().type_name, name, "slot {n}");
        }
        assert_eq!(types.get(29).unwrap().type_name, "設定29");
        assert_eq!(types.type_index_for_file_name("a.mine"), 17);
        assert_eq!(types.type_index_for_file_name("a.zig"), 19);
        assert_eq!(types.type_index_for_file_name("a.js"), 18);
        assert_eq!(types.type_index_for_file_name("a.yml"), 28);
    }

    #[test]
    fn the_types_that_do_not_fit_are_left_out() {
        // nTypesCount 17: no number is free.
        let types = DocumentTypes::from_profile(&profile("[Other]\r\nnTypesCount=17\r\n"));
        assert_eq!(types.len(), 17);
        assert_eq!(types.type_index_for_file_name("a.js"), 0);
        // nTypesCount 20: three.
        let types = DocumentTypes::from_profile(&profile("[Other]\r\nnTypesCount=20\r\n"));
        assert_eq!(types.len(), 20);
        assert_eq!(names_from(&types, 17, 3), &NAMES[..3]);
        assert_eq!(types.type_index_for_file_name("a.css"), 0);
        // nTypesCount 60: UME Editor's ten, then copies of the basic type.
        let types = DocumentTypes::from_profile(&profile("[Other]\r\nnTypesCount=60\r\n"));
        assert_eq!(types.len(), 60);
        assert_eq!(names_from(&types, 17, 10), NAMES);
        assert_eq!(types.get(59).unwrap().type_name, "設定59");
    }

    #[test]
    fn the_keyword_sets_go_after_the_sets_of_the_file() {
        // A file with 20 sets: UME Editor's sets are 20 to 29, and Python's second set follows.
        let mut text =
            String::from("[Other]\r\nnTypesCount=30\r\n[KeyWords]\r\nnKeyWordSetNum=20\r\n");
        for i in 0..20 {
            text.push_str(&format!(
                "szSN[{i:02}]=S{i}\r\nnCASE[{i:02}]=1\r\nnKWN[{i:02}]=1\r\nszKW[{i:02}]=w{i}\t\r\n"
            ));
        }
        let types = DocumentTypes::from_profile(&profile(&text));
        assert_eq!(types.keywords().len(), 30);
        assert_eq!(types.keywords().get(19).unwrap().name, "S19");
        assert_eq!(types.keywords().get(20).unwrap().name, "Python built-ins");
        let python = types.keyword_sets_of(13);
        assert_eq!(python.len(), 2);
        assert!(python[1].1.contains("print"));
        let js = types.keyword_sets_of(types.index_of_name("JavaScript").unwrap());
        assert_eq!(js[0].1.name, "JavaScript");
        // A file with 100 sets has no number left: no keywords, and the types still exist.
        let mut text = String::from("[KeyWords]\r\nnKeyWordSetNum=100\r\n");
        for i in 0..100 {
            text.push_str(&format!("szSN[{i:02}]=S{i}\r\n"));
        }
        let types = DocumentTypes::from_profile(&profile(&text));
        assert_eq!(types.keywords().len(), 100);
        assert_eq!(types.get(13).unwrap().keyword_set_idx[1], -1);
        assert_eq!(types.get(17).unwrap().keyword_set_idx[0], -1);
        assert_eq!(types.get(17).unwrap().type_name, "JavaScript");
    }

    #[test]
    fn a_file_may_give_python_no_second_set() {
        // Read as now for Sakura's types: the keys of the file win.
        let types = DocumentTypes::from_profile(&profile(
            "[Types(13)]\r\nnInts=13,10240,0,4,16,-1,4,1,1,0,0,0\r\n",
        ));
        assert_eq!(types.get(13).unwrap().keyword_set_idx[1], -1);
    }

    #[test]
    fn a_written_type_is_read_back_as_the_same_type() {
        // ADR-0017: the settings file gets all the keys of the type, so the next read gives the same type
        // even though the type has a name and extensions of its own now.
        for name in NAMES {
            let types = DocumentTypes::builtin();
            let index = types.index_of_name(name).unwrap();
            let mut ini = profile("[Other]\r\nnTypesCount=30\r\n");
            types
                .get(index)
                .unwrap()
                .write(&mut ini, i32::try_from(index).unwrap())
                .unwrap();
            let again = DocumentTypes::from_profile(&Profile::from_bytes(&ini.to_bytes()));
            assert_eq!(again.get(index), types.get(index), "{name}");
        }
    }

    #[test]
    fn the_c_like_types_take_the_values_of_cpp() {
        let types = DocumentTypes::builtin();
        let cpp = types.get(2).unwrap();
        for name in ["JavaScript", "TypeScript", "Go", "Rust"] {
            let t = types.get(types.index_of_name(name).unwrap()).unwrap();
            assert_eq!(t.tab_space, cpp.tab_space, "{name}");
            assert_eq!(t.line_comments[0].delimiter, "//", "{name}");
            assert_eq!(t.block_comments[0], BlockComment::new("/*", "*/"), "{name}");
            assert_eq!(t.smart_indent, cpp.smart_indent, "{name}");
            assert!(t.colors_numbers(), "{name}");
            assert!(t.string_line_only, "{name}");
        }
        let rust = types.get(types.index_of_name("Rust").unwrap()).unwrap();
        // A lifetime must not start a string.
        assert!(!rust.colors.get("SQT").unwrap().disp);
        assert!(rust.colors.get("WQT").unwrap().disp);
    }

    #[test]
    fn every_extra_set_has_its_words() {
        let sets = KeywordSets::builtin();
        let has =
            |set: ExtraSet, word: &str| sets.get(sets.extra_index(set)).unwrap().contains(word);
        assert!(has(ExtraSet::PythonBuiltins, "print"));
        assert!(has(ExtraSet::PythonBuiltins, "len"));
        assert!(!has(ExtraSet::PythonBuiltins, "lambda"));
        assert!(has(ExtraSet::JavaScript, "const"));
        assert!(has(ExtraSet::JavaScript, "undefined"));
        assert!(has(ExtraSet::TypeScript, "interface"));
        assert!(has(ExtraSet::TypeScript, "readonly"));
        assert!(has(ExtraSet::Json, "null"));
        assert!(has(ExtraSet::Css, "@media"));
        assert!(has(ExtraSet::Css, "INHERIT"), "CSS ignores case");
        assert!(has(ExtraSet::Rust, "fn"));
        assert!(has(ExtraSet::Rust, "macro_rules"));
        assert!(has(ExtraSet::Go, "func"));
        assert!(has(ExtraSet::Go, "nil"));
        assert!(has(ExtraSet::Ruby, "elsif"));
        assert!(has(ExtraSet::Shell, "fi"));
        assert!(has(ExtraSet::Yaml, "null"));
        assert!(!has(ExtraSet::JavaScript, "Const"));
        for (i, set) in ExtraSet::ALL.iter().enumerate() {
            assert_eq!(set.ordinal(), i);
            assert_eq!(
                ExtraSet::from_default_index(set.default_index()),
                Some(*set)
            );
        }
        assert_eq!(ExtraSet::from_default_index(16), None);
        assert_eq!(ExtraSet::from_default_index(27), None);
        assert_eq!(ExtraSet::from_default_index(-1), None);
    }
}
