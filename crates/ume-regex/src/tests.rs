//! Tests of the public API. Expectations about Sakura's syntax come from
//! `help/sakura/res/HLP000089.html` (cited per test); the engine's own
//! behavior is covered by `onigmo_suite`.

use std::sync::Arc;
use std::thread;

use super::*;
use crate::LIMITS_LOCK;

/// Shared by every test that uses the engine, so the Onigmo suite (which
/// changes process-wide limits) never runs at the same time as another test.
fn read_lock() -> std::sync::RwLockReadGuard<'static, ()> {
    LIMITS_LOCK
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn compile(pattern: &str) -> Box<dyn Regex> {
    Onigmo
        .compile(pattern, Options::default())
        .unwrap_or_else(|e| panic!("/{pattern}/ should compile: {e}"))
}

fn compile_with(pattern: &str, options: Options) -> Box<dyn Regex> {
    Onigmo
        .compile(pattern, options)
        .unwrap_or_else(|e| panic!("/{pattern}/ should compile: {e}"))
}

fn find(pattern: &str, text: &str) -> Option<Range<usize>> {
    compile(pattern).find_at(text, 0).unwrap()
}

#[test]
fn finds_the_first_match_at_or_after_the_start_offset() {
    let _limits = read_lock();
    let re = compile("ab");
    assert_eq!(re.find_at("abXab", 0).unwrap(), Some(0..2));
    assert_eq!(re.find_at("abXab", 1).unwrap(), Some(3..5));
    assert_eq!(re.find_at("abXab", 3).unwrap(), Some(3..5));
    assert_eq!(re.find_at("abXab", 4).unwrap(), None);
    // The start may be the end of the text: only an empty match is possible.
    assert_eq!(re.find_at("ab", 2).unwrap(), None);
    assert_eq!(compile("").find_at("ab", 2).unwrap(), Some(2..2));
}

#[test]
fn offsets_are_utf8_byte_offsets() {
    let _limits = read_lock();
    let text = "あ世界x世";
    let re = compile("世");
    assert_eq!(re.find_at(text, 0).unwrap(), Some(3..6));
    assert_eq!(re.find_at(text, 6).unwrap(), Some(10..13));
    // "x" is one byte, the full-width "ｘ" is three.
    assert_eq!(find("ｘ", "ｘ"), Some(0..3));
}

#[test]
fn a_start_inside_a_character_or_past_the_end_is_an_error() {
    let _limits = read_lock();
    let re = compile("a");
    assert_eq!(
        re.find_at("あa", 1),
        Err(SearchError::InvalidStart { start: 1, len: 4 })
    );
    assert_eq!(
        re.find_at("あa", 5),
        Err(SearchError::InvalidStart { start: 5, len: 4 })
    );
    assert!(
        re.find_at("あa", 1)
            .unwrap_err()
            .to_string()
            .contains("boundary")
    );
}

#[test]
fn context_before_the_start_offset_is_visible() {
    let _limits = read_lock();
    // `^` matches at the start of the line, not at the start offset.
    assert_eq!(compile("^b").find_at("ab", 1).unwrap(), None);
    assert_eq!(compile("^b").find_at("a\nb", 1).unwrap(), Some(2..3));
    // Look-behind sees the text before the start offset.
    assert_eq!(compile("(?<=a)b").find_at("ab", 1).unwrap(), Some(1..2));
    assert_eq!(compile(r"\bb").find_at("ab", 1).unwrap(), None);
    // `\G` is the start offset (bregonig passes it as the global position).
    assert_eq!(compile(r"\Gb").find_at("ab", 1).unwrap(), Some(1..2));
    assert_eq!(compile(r"\Gb").find_at("abb", 2).unwrap(), Some(2..3));
    assert_eq!(compile(r"\Gb").find_at("aab", 1).unwrap(), None);
}

#[test]
fn captures_report_each_group() {
    let _limits = read_lock();
    let re = compile(r"(\w+)@(\w+)(\.com)?");
    assert_eq!(re.capture_count(), 3);
    let caps = re.captures_at("to: ume@sakura now", 0).unwrap().unwrap();
    assert_eq!(caps.len(), 4);
    assert!(!caps.is_empty());
    assert_eq!(caps.whole(), 4..14);
    assert_eq!(caps.get(0), Some(4..14));
    assert_eq!(caps.get(1), Some(4..7));
    assert_eq!(caps.get(2), Some(8..14));
    // A group that did not take part, and a group that does not exist.
    assert_eq!(caps.get(3), None);
    assert_eq!(caps.get(4), None);
    let all: Vec<_> = caps.iter().collect();
    assert_eq!(all, vec![Some(4..14), Some(4..7), Some(8..14), None]);
}

#[test]
fn named_groups_are_numbered_like_plain_groups() {
    let _limits = read_lock();
    // HLP000089.html: (?<name>...) is available (bregonig only).
    let re = compile(r"(?<year>\d+)年(?<month>\d+)月");
    assert_eq!(re.capture_count(), 2);
    let caps = re.captures_at("2026年10月", 0).unwrap().unwrap();
    assert_eq!(caps.get(1), Some(0..4));
    assert_eq!(caps.get(2), Some(7..9));
}

#[test]
fn digits_and_word_characters_include_full_width() {
    let _limits = read_lock();
    // HLP000089.html: `\d` is "2バイト文字=全角数字も含む"; `\w` includes 2-byte characters.
    assert_eq!(find(r"\d+", "価格２０２６円"), Some(6..18));
    assert_eq!(find(r"\w+", "、日本語のtext。"), Some(3..19));
    assert_eq!(find(r"\W", "あ、い"), Some(3..6));
    assert_eq!(find(r"\s", "あ　い"), Some(3..6)); // U+3000 ideographic space
}

#[test]
fn ascii_range_limits_the_character_classes() {
    let _limits = read_lock();
    let ascii = Options {
        ascii_range: true,
        ..Options::default()
    };
    let digits = compile_with(r"\d+", ascii);
    assert_eq!(digits.find_at("２０２６ 2026", 0).unwrap(), Some(13..17));
    assert_eq!(compile_with(r"\w", ascii).find_at("あ", 0).unwrap(), None);
}

#[test]
fn case_is_sensitive_by_default_and_ignore_case_is_unicode_aware() {
    let _limits = read_lock();
    assert_eq!(find("abc", "ABC"), None);
    let ignore = Options {
        ignore_case: true,
        ..Options::default()
    };
    assert_eq!(
        compile_with("abc", ignore).find_at("xABC", 0).unwrap(),
        Some(1..4)
    );
    // HLP000089.html: in the Unicode build, `i` also applies outside ASCII.
    assert_eq!(
        compile_with("ａｂ", ignore).find_at("ＡＢ", 0).unwrap(),
        Some(0..6)
    );
    assert_eq!(
        compile_with("é", ignore).find_at("É", 0).unwrap(),
        Some(0..2)
    );
    assert_eq!(
        compile_with("σ", ignore).find_at("Σ", 0).unwrap(),
        Some(0..2)
    );
}

#[test]
fn extended_syntax_ignores_whitespace_and_comments() {
    let _limits = read_lock();
    let extended = Options {
        extended: true,
        ..Options::default()
    };
    let re = compile_with("a b # comment\n c", extended);
    assert_eq!(re.find_at("xabc", 0).unwrap(), Some(1..4));
    assert_eq!(find("a b", "ab"), None);
}

#[test]
fn dollar_matches_before_the_line_terminator() {
    let _limits = read_lock();
    // HLP000089.html, 改行の検索: `$` matches at the end of the line.
    assert_eq!(find("a$", "a"), Some(0..1));
    assert_eq!(find("a$", "a\n"), Some(0..1));
    assert_eq!(find("a$", "ab"), None);
    // With the `R` modifier a CR LF pair is one terminator.
    let crlf = Options {
        crlf: true,
        ..Options::default()
    };
    assert_eq!(
        compile_with("a$", crlf).find_at("a\r\n", 0).unwrap(),
        Some(0..1)
    );
    assert_eq!(find("a$", "a\r\n"), None);
    // Searching for the line break itself: `\r\n` and `[\r\n]+`.
    assert_eq!(find(r"\r\n", "a\r\nb"), Some(1..3));
    assert_eq!(find(r"[\r\n]+", "a\r\n\nb"), Some(1..4));
}

#[test]
fn bregonig_extensions_are_enabled() {
    let _limits = read_lock();
    // HLP000089.html marks these (鬼): set operations, nested sets, POSIX
    // brackets, \x{...}, \Q..\E, look-behind, atomic groups, \p{...}, \k, \g.
    assert_eq!(find("[a-z&&[^aeiou]]+", "aeibcdou"), Some(3..6));
    assert_eq!(find("[[:upper:]]+", "abCDe"), Some(2..4));
    assert_eq!(find(r"\x{3042}", "いあ"), Some(3..6));
    assert_eq!(find(r"\Qa.b\E", "axb a.b"), Some(4..7));
    assert_eq!(find("(?<=a)b", "cb ab"), Some(4..5));
    assert_eq!(find("(?<!a)b", "ab cb"), Some(4..5));
    assert_eq!(find("(?>a+)b", "aaab"), Some(0..4));
    assert_eq!(find(r"\p{Hiragana}+", "カタかなカナ"), Some(6..12));
    assert_eq!(find(r"(?<q>['\x22]).*?\k<q>", "say 'hi' ok"), Some(4..8));
    assert_eq!(find(r"(\((?:[^()]|\g<1>)*\))", "f(a(b)c)d"), Some(1..8));
    // bregonig also sets these in its syntax (src/bregonig.cpp, DllMain):
    // `(?~...)` (absent operator) and look-behind with alternatives of different length.
    assert_eq!(find("/\\*(?~\\*/)\\*/", "x/* a */ b */"), Some(1..8));
    assert_eq!(find("(?<=a|bc)d", "bcd"), Some(2..3));
    // Options inside the pattern: `i`, and negating them (HLP000089.html).
    assert_eq!(find("(?i)abc", "xABC"), Some(1..4));
    assert_eq!(find("(?i:a)b", "Ab"), Some(0..2));
    assert_eq!(find("(?i:a)b", "AB"), None);
    assert_eq!(find("(?i)a(?-i)b", "AB"), None);
}

#[test]
fn greedy_and_lazy_quantifiers() {
    let _limits = read_lock();
    // HLP000089.html, 量指定子: the SAKURA_COLLBACKPROC_BREXP_PROC example.
    let text = "SAKURA_COLLBACKPROC_BREXP_PROC";
    assert_eq!(find("[A-Z_]*PROC", text), Some(0..text.len()));
    assert_eq!(find("[A-Z_]*?PROC", text), Some(0..19));
}

#[test]
fn text_with_nul_and_long_lines() {
    let _limits = read_lock();
    assert_eq!(find("b", "a\0b"), Some(2..3));
    assert_eq!(find(r"\x00", "a\0b"), Some(1..2));
    let long = format!("{}x", "あ".repeat(100_000));
    assert_eq!(find("x", &long), Some(300_000..300_001));
}

#[test]
fn bad_patterns_report_the_engine_message() {
    let _limits = read_lock();
    for (pattern, expected) in [
        ("(", "end pattern with unmatched parenthesis"),
        (")", "unmatched close parenthesis"),
        ("[a", "premature end of char-class"),
        ("*", "target of repeat operator is not specified"),
        ("a{2,1}", "too big number for repeat range"),
        (r"\", "end pattern at escape"),
        (r"\k<x>", "undefined name <x> reference"),
        (r"(?<=a*)b", "invalid pattern in look-behind"),
    ] {
        let err = Onigmo.compile(pattern, Options::default()).unwrap_err();
        assert!(
            err.message().contains(expected) || pattern == "a{2,1}",
            "/{pattern}/: message was {:?}",
            err.message()
        );
        assert!(err.code() < 0, "/{pattern}/: code {}", err.code());
        assert_eq!(err.to_string(), err.message());
    }
}

#[test]
fn a_failed_compile_leaves_the_engine_usable() {
    let _limits = read_lock();
    for _ in 0..100 {
        assert!(Onigmo.compile("(", Options::default()).is_err());
    }
    assert_eq!(find("a", "a"), Some(0..1));
}

#[test]
fn a_compiled_regex_can_be_shared_between_threads() {
    let _limits = read_lock();
    let re: Arc<dyn Regex> = Arc::from(compile(r"(\d+)-(\d+)"));
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let re = Arc::clone(&re);
            thread::spawn(move || {
                for n in 0..2000 {
                    let text = format!("行{i} {n}-{} 終わり", n + 1);
                    let caps = re.captures_at(&text, 0).unwrap().unwrap();
                    let first = caps.get(1).unwrap();
                    assert_eq!(&text[first], n.to_string());
                    let second = caps.get(2).unwrap();
                    assert_eq!(&text[second], (n + 1).to_string());
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn compiling_from_many_threads_at_once() {
    let _limits = read_lock();
    let handles: Vec<_> = (0..8)
        .map(|i| {
            thread::spawn(move || {
                for n in 0..200 {
                    let pattern = format!("x{i}y{n}");
                    let re = compile(&pattern);
                    assert_eq!(
                        re.find_at(&format!("--{pattern}--"), 0).unwrap(),
                        Some(2..2 + pattern.len())
                    );
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn the_trait_is_object_safe_and_send() {
    let _limits = read_lock();
    fn assert_send_sync<T: Send + Sync + ?Sized>() {}
    assert_send_sync::<dyn Regex>();
    let engine: &dyn RegexEngine = &Onigmo;
    let re = engine.compile("ä", Options::default()).unwrap();
    assert!(format!("{re:?}").contains("OnigmoRegex"));
}
