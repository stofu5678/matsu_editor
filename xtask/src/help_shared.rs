//! The check for text shared with Sakura's help (ADR-0031).
//!
//! Sakura's help is GFDL 1.1 or later; MATSU's help is MIT OR Apache-2.0 and is written from behavior, never
//! from Sakura's help text. This check reads both, reduces them to plain text, and reports every run of the
//! same text that is long enough to be more than chance.
//!
//! How the texts are compared:
//! 1. Sakura's HTML and MATSU's Markdown are stripped to their text (tags, code blocks, link targets and
//!    entities are dropped or decoded).
//! 2. The text is normalized: every white space and every punctuation mark is dropped, full-width ASCII
//!    letters and digits become half-width, ASCII letters become lower case. A line break inside a paragraph,
//!    a full-width space, or `、` against `,` therefore cannot hide a copy.
//! 3. Every name (the labels of Sakura's functions and menus, and the macro function names) becomes one
//!    placeholder character. A page that only uses Sakura's names shares no text with Sakura's help, but a
//!    copied sentence that merely contains a name still matches, and so does a copy in which one name was
//!    changed to another name (both are the placeholder).
//! 4. A run of the same text counts when it holds at least [`MIN_SHARED_CHARS`] characters that are neither the
//!    placeholder nor ASCII letters or digits (see [`counts`]).

use crate::help_links::{fence_closes, fence_open};
use std::collections::HashMap;
use std::fmt::Write as _;

/// The placeholder that stands for a name (a private use character, so it never occurs in a page).
const NAME: char = '\u{E000}';

/// The length of a shared run, in normalized characters that are not names, that fails the check.
///
/// How it was chosen is in `docs/help/README.md` ("共有する文字列の検査") with the numbers.
pub const MIN_SHARED_CHARS: usize = 20;

/// One piece of text with the source line of each character.
pub type RawText = Vec<(char, u32)>;

// ---------------------------------------------------------------------------------------------------------
// Stripping

/// The text of a Markdown page: without fenced code blocks, HTML tags and comments, link targets, link
/// reference definitions and mdBook's `{{#include}}` lines. The syntax characters that remain (`#`, `*`, `|`,
/// backticks) are punctuation, which [`normalize`] drops. Inline code keeps its text.
pub fn strip_markdown(source: &str) -> RawText {
    let mut out = RawText::new();
    let mut fence: Option<(char, usize)> = None;
    let mut in_comment = false;
    for (index, line) in source.lines().enumerate() {
        let number = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let trimmed = line.trim_start();
        if let Some(open) = fence {
            if fence_closes(trimmed, open) {
                fence = None;
            }
            continue;
        }
        if !in_comment && let Some(open) = fence_open(trimmed) {
            fence = Some(open);
            continue;
        }
        if !in_comment && (trimmed.starts_with("{{#") || is_reference_definition(trimmed)) {
            continue;
        }
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if in_comment {
                // Looking for "-->".
                if c == '-' && chars.clone().take(2).collect::<String>() == "->" {
                    chars.next();
                    chars.next();
                    in_comment = false;
                }
                continue;
            }
            match c {
                '<' if chars.clone().take(3).collect::<String>() == "!--" => {
                    in_comment = true;
                }
                '<' if chars
                    .peek()
                    .is_some_and(|n| n.is_ascii_alphabetic() || *n == '/') =>
                {
                    // A tag or an autolink: skip to '>'.
                    for n in chars.by_ref() {
                        if n == '>' {
                            break;
                        }
                    }
                }
                ']' if chars.peek() == Some(&'[') => {
                    // The label of a reference link, `[text][label]` (or `[text][]`): not text.
                    for n in chars.by_ref() {
                        if n == ']' {
                            break;
                        }
                    }
                }
                ']' if chars.peek() == Some(&'(') => {
                    // The target of a link or an image: skip to the matching ')'.
                    chars.next();
                    let mut depth = 1;
                    for n in chars.by_ref() {
                        match n {
                            '(' => depth += 1,
                            ')' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                '&' => {
                    // A character reference (`&nbsp;`, `&#12354;`), as the HTML side decodes it.
                    let ahead: Vec<char> =
                        std::iter::once('&').chain(chars.clone().take(12)).collect();
                    if let Some((decoded, used)) = decode_entity(&ahead) {
                        for _ in 1..used {
                            chars.next();
                        }
                        out.push((decoded, number));
                    } else {
                        out.push((c, number));
                    }
                }
                _ => out.push((c, number)),
            }
        }
        out.push(('\n', number));
    }
    out
}

fn is_reference_definition(line: &str) -> bool {
    // `[label]: target`
    line.strip_prefix('[')
        .filter(|rest| !rest.starts_with('^'))
        .and_then(|rest| rest.split_once("]:"))
        .is_some()
}

/// The text of one of Sakura's HTML pages: without tags, comments, scripts and styles; entities decoded.
pub fn strip_html(source: &str) -> RawText {
    let chars: Vec<char> = source.chars().collect();
    let mut out = RawText::new();
    let mut line = 1u32;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '<' {
            let rest: String = chars[i + 1..chars.len().min(i + 10)].iter().collect();
            let rest_lower = rest.to_ascii_lowercase();
            let end = if rest.starts_with("!--") {
                find(&chars, i + 4, "-->").map(|p| p + 3)
            } else if rest_lower.starts_with("script") || rest_lower.starts_with("style") {
                let close = if rest_lower.starts_with("script") {
                    "</script"
                } else {
                    "</style"
                };
                find_ci(&chars, i + 1, close)
                    .and_then(|p| find(&chars, p, ">"))
                    .map(|p| p + 1)
            } else {
                find(&chars, i + 1, ">").map(|p| p + 1)
            };
            let end = end.unwrap_or(chars.len());
            line +=
                u32::try_from(chars[i..end].iter().filter(|c| **c == '\n').count()).unwrap_or(0);
            i = end;
            continue;
        }
        if c == '\n' {
            line += 1;
        }
        if c == '&'
            && let Some((decoded, used)) = decode_entity(&chars[i..])
        {
            out.push((decoded, line));
            i += used;
            continue;
        }
        out.push((c, line));
        i += 1;
    }
    out
}

fn find(chars: &[char], from: usize, needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    (from..=chars.len().checked_sub(needle.len())?).find(|&p| chars[p..].starts_with(&needle))
}

fn find_ci(chars: &[char], from: usize, needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    (from..=chars.len().checked_sub(needle.len())?).find(|&p| {
        chars[p..p + needle.len()]
            .iter()
            .zip(&needle)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    })
}

/// Decodes `&name;`, `&#123;` or `&#x1F;` at the start of `chars`. Returns the character and the number of
/// characters used.
pub fn decode_entity(chars: &[char]) -> Option<(char, usize)> {
    let end = chars.iter().take(12).position(|c| *c == ';')?;
    let body: String = chars[1..end].iter().collect();
    let decoded = if let Some(hex) = body.strip_prefix("#x").or_else(|| body.strip_prefix("#X")) {
        char::from_u32(u32::from_str_radix(hex, 16).ok()?)?
    } else if let Some(dec) = body.strip_prefix('#') {
        char::from_u32(dec.parse().ok()?)?
    } else {
        match body.as_str() {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            "nbsp" => ' ',
            "copy" => '\u{a9}',
            "reg" => '\u{ae}',
            "yen" => '\u{a5}',
            "times" => '\u{d7}',
            "hellip" => '\u{2026}',
            "rarr" => '\u{2192}',
            "larr" => '\u{2190}',
            _ => return None,
        }
    };
    Some((decoded, end + 1))
}

// ---------------------------------------------------------------------------------------------------------
// Normalizing

/// A character of normalized text.
#[derive(Clone, Copy)]
pub struct Norm {
    c: char,
    line: u32,
    /// White space or punctuation was dropped just before this character.
    after_break: bool,
}

/// Drops white space and punctuation, folds full-width ASCII to half-width, and lowers ASCII letters.
pub fn normalize(raw: &[(char, u32)]) -> Vec<Norm> {
    let mut out = Vec::with_capacity(raw.len());
    let mut after_break = false;
    for &(c, line) in raw {
        let c = fold(c);
        if c.is_whitespace() || is_punctuation(c) {
            after_break = true;
        } else {
            out.push(Norm {
                c: c.to_ascii_lowercase(),
                line,
                after_break,
            });
            after_break = false;
        }
    }
    out
}

fn fold(c: char) -> char {
    match c {
        '\u{ff01}'..='\u{ff5e}' => char::from_u32(u32::from(c) - 0xfee0).unwrap_or(c),
        _ => c,
    }
}

fn is_punctuation(c: char) -> bool {
    c.is_ascii_punctuation()
        || matches!(c,
            '\u{3000}'..='\u{303f}' // CJK symbols and punctuation (、。「」『』【】 and so on)
            | '\u{30fb}' // katakana middle dot
            | '\u{2010}'..='\u{2027}' // dashes, quotes, ellipsis, bullets
            | '\u{2030}'..='\u{205e}'
            | '\u{00a0}'..='\u{00bf}' // Latin-1 symbols
            | '\u{2190}'..='\u{21ff}' // arrows
            | '\u{25a0}'..='\u{25ff}' // geometric shapes
            | '\u{ff5f}'..='\u{ff65}') // half-width CJK punctuation
}

// ---------------------------------------------------------------------------------------------------------
// Names

/// The set of names that the check does not count: the labels and macro names of Sakura's tables.
#[derive(Default)]
pub struct Names {
    root: TrieNode,
}

#[derive(Default)]
struct TrieNode {
    next: HashMap<char, TrieNode>,
    /// A name ends here.
    end: bool,
}

impl Names {
    /// A name is normalized the same way as the texts. A name of one character is ignored (it would hide too
    /// much), and so is a name that is empty after normalizing.
    pub fn add(&mut self, name: &str) {
        let name: Vec<char> = normalize(&name.chars().map(|c| (c, 0)).collect::<Vec<_>>())
            .into_iter()
            .map(|n| n.c)
            .collect();
        if name.len() < 2 {
            return;
        }
        let mut node = &mut self.root;
        for c in name {
            node = node.next.entry(c).or_default();
        }
        node.end = true;
    }

    /// The length of the longest name that starts at `at`. A name of ASCII letters and digits only counts when
    /// no ASCII letter or digit touches it on either side (`Up` is not in `Update`).
    fn longest_at(&self, text: &[Norm], at: usize) -> Option<usize> {
        let mut node = &self.root;
        let mut best = None;
        for (offset, n) in text[at..].iter().enumerate() {
            let Some(next) = node.next.get(&n.c) else {
                break;
            };
            node = next;
            if node.end {
                let len = offset + 1;
                let word = text[at..at + len]
                    .iter()
                    .all(|n| n.c.is_ascii_alphanumeric());
                let touches_left =
                    at > 0 && !text[at].after_break && text[at - 1].c.is_ascii_alphanumeric();
                let touches_right = text
                    .get(at + len)
                    .is_some_and(|n| !n.after_break && n.c.is_ascii_alphanumeric());
                if !(word && (touches_left || touches_right)) {
                    best = Some(len);
                }
            }
        }
        best
    }
}

/// Reads the names from the generated tables of `crates/matsu-commands/src/sakura/`: every `label: Some("..")`
/// (functions and the main menu), every macro function `name: ".."`, and the function groups. The text is read
/// rather than linked so that regenerating the tables cannot break this crate's build.
pub fn names_from_tables(sakura_tables: &std::path::Path) -> Result<Names, String> {
    let mut names = Names::default();
    for file in ["functions.rs", "main_menu.rs", "macro_functions.rs"] {
        let path = sakura_tables.join(file);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        for line in text.lines() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for key in ["label: Some(\"", "MacroFunction { name: \""] {
                let mut rest = line;
                while let Some(start) = rest.find(key) {
                    rest = &rest[start + key.len()..];
                    if let Some(end) = rest.find('"') {
                        add_label(&mut names, &rest[..end]);
                    }
                }
            }
            // The group names: a string on a line of its own in `FUNCTION_GROUPS`.
            let t = line.trim();
            if file == "functions.rs" && t.starts_with('"') && t.ends_with("\",") {
                add_label(&mut names, &t[1..t.len() - 2]);
            }
        }
    }
    Ok(names)
}

/// Adds the texts of the dialog controls of Sakura's resource file (`sakura_core/sakura_rc.rc`, UTF-16): the
/// labels of check boxes, buttons, group boxes and the dialog captions. They are the names of Sakura's
/// settings; a help page that says "チェックを入れると、「…」が…" has to quote them. The file is Sakura's source
/// (zlib License), not its help.
pub fn add_names_from_rc(names: &mut Names, sakura: &std::path::Path) -> Result<(), String> {
    const CONTROLS: [&str; 12] = [
        "CONTROL",
        "LTEXT",
        "CTEXT",
        "RTEXT",
        "GROUPBOX",
        "PUSHBUTTON",
        "DEFPUSHBUTTON",
        "AUTOCHECKBOX",
        "CHECKBOX",
        "AUTORADIOBUTTON",
        "RADIOBUTTON",
        "CAPTION",
    ];
    let path = sakura.join("sakura_core/sakura_rc.rc");
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes(*b))
        .collect();
    let text = String::from_utf16(&units).map_err(|e| format!("{}: {e}", path.display()))?;
    for line in text.lines() {
        let line = line.trim_start_matches('\u{feff}').trim_start();
        let Some((keyword, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        if !CONTROLS.contains(&keyword) {
            continue;
        }
        // The first string of the line; `""` is a quote inside it and `\n` a line break.
        let Some(rest) = rest.trim_start().strip_prefix('"') else {
            continue;
        };
        let mut label = String::new();
        let mut chars = rest.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    label.push('"');
                }
                '"' => break,
                _ => label.push(c),
            }
        }
        for part in label.split("\\n") {
            add_label(names, part);
        }
    }
    Ok(())
}

/// A label as Sakura's string table writes it, reduced to its words: `&&` is `&`, `&F` is `F`, `(&F)` is
/// nothing, and `...` at the end is dropped.
fn add_label(names: &mut Names, label: &str) {
    let label = label.replace("(&&)", "").replace("&&", "\u{1}");
    let mut plain = String::new();
    let mut chars = label.chars();
    while let Some(c) = chars.next() {
        if c == '&' {
            chars.next();
        } else {
            plain.push(c);
        }
    }
    let plain = plain.replace('\u{1}', "&").replace("()", "");
    names.add(plain.trim_end_matches(['.', '…']).trim());
}

// ---------------------------------------------------------------------------------------------------------
// Comparing

/// A text with its names turned into [`NAME`].
pub struct Prepared {
    chars: Vec<char>,
    lines: Vec<u32>,
    /// The name behind a placeholder, by position (for printing).
    shown: HashMap<usize, String>,
}

pub fn prepare(raw: &[(char, u32)], names: &Names) -> Prepared {
    let text = normalize(raw);
    let mut prepared = Prepared {
        chars: Vec::with_capacity(text.len()),
        lines: Vec::with_capacity(text.len()),
        shown: HashMap::new(),
    };
    let mut i = 0;
    while i < text.len() {
        prepared.lines.push(text[i].line);
        if let Some(len) = names.longest_at(&text, i) {
            prepared.shown.insert(
                prepared.chars.len(),
                text[i..i + len].iter().map(|n| n.c).collect(),
            );
            prepared.chars.push(NAME);
            i += len;
        } else {
            prepared.chars.push(text[i].c);
            i += 1;
        }
    }
    prepared
}

/// One run of text that both sides have.
#[derive(Debug, PartialEq, Eq)]
pub struct Shared {
    pub matsu_file: String,
    pub matsu_line: u32,
    pub sakura_file: String,
    pub sakura_line: u32,
    /// The shared text in its normalized form; a name shows as `〔name〕`.
    pub text: String,
    /// The characters of it that count (see [`counts`]).
    pub real_chars: usize,
}

/// Whether a normalized character counts toward the length of a shared run: not a name, and not an ASCII letter
/// or digit. The help is Japanese; ASCII text in it is identifiers, paths, URLs and code (plugin event names,
/// `matsu.ini` keys), which are facts that two pages describing the same thing share by necessity.
fn counts(c: char) -> bool {
    c != NAME && !c.is_ascii()
}

const MULTIPLIER: u64 = 1_000_003;

fn window_hashes(chars: &[char], width: usize) -> impl Iterator<Item = (usize, u64)> + '_ {
    let mut power = 1u64;
    for _ in 1..width {
        power = power.wrapping_mul(MULTIPLIER);
    }
    let mut hash = 0u64;
    (0..chars.len()).filter_map(move |i| {
        if i >= width {
            hash = hash.wrapping_sub(u64::from(chars[i - width]).wrapping_mul(power));
        }
        hash = hash
            .wrapping_mul(MULTIPLIER)
            .wrapping_add(u64::from(chars[i]));
        (i + 1 >= width).then(|| (i + 1 - width, hash))
    })
}

/// Finds the runs of `min_real` or more characters (not counting names) that a MATSU page shares with one of
/// Sakura's pages. Both lists are `(file name, prepared text)`.
pub fn find_shared(
    matsu: &[(String, Prepared)],
    sakura: &[(String, Prepared)],
    min_real: usize,
) -> Vec<Shared> {
    // Every window of `min_real` characters of Sakura's help. A run with `min_real` real characters is at least
    // that long. The positions per window are capped: a window that occurs very often is made of names.
    const MAX_POSITIONS: usize = 4096;
    let mut index: HashMap<u64, Vec<(usize, usize)>> = HashMap::new();
    for (file, (_, page)) in sakura.iter().enumerate() {
        for (start, hash) in window_hashes(&page.chars, min_real) {
            let slot = index.entry(hash).or_default();
            if slot.len() < MAX_POSITIONS {
                slot.push((file, start));
            }
        }
    }
    let mut found = Vec::new();
    for (matsu_name, page) in matsu {
        let hashes: HashMap<usize, u64> = window_hashes(&page.chars, min_real).collect();
        let mut i = 0;
        while i + min_real <= page.chars.len() {
            let best = hashes
                .get(&i)
                .and_then(|hash| index.get(hash))
                .into_iter()
                .flatten()
                .map(|&(file, start)| {
                    let len = page.chars[i..]
                        .iter()
                        .zip(&sakura[file].1.chars[start..])
                        .take_while(|(a, b)| a == b)
                        .count();
                    let real = page.chars[i..i + len]
                        .iter()
                        .filter(|c| counts(**c))
                        .count();
                    (len, real, file, start)
                })
                // The candidate with the most counted characters: a longer one may hold fewer of them.
                .max_by_key(|&(_, real, ..)| real);
            match best {
                Some((len, real, file, start)) if real >= min_real => {
                    found.push(Shared {
                        matsu_file: matsu_name.clone(),
                        matsu_line: page.lines[i],
                        sakura_file: sakura[file].0.clone(),
                        sakura_line: sakura[file].1.lines[start],
                        text: show(page, i, len),
                        real_chars: real,
                    });
                    i += len;
                }
                // Not enough counted characters (names, ASCII): a run that starts later may be enough.
                _ => i += 1,
            }
        }
    }
    found
}

fn show(page: &Prepared, start: usize, len: usize) -> String {
    let mut text = String::new();
    for position in start..start + len {
        match page.shown.get(&position) {
            Some(name) => {
                let _ = write!(text, "〔{name}〕");
            }
            None => text.push(page.chars[position]),
        }
    }
    text
}

/// The length of the longest run (in characters that are not names) that the texts share, found by bisection
/// (a longer limit finds fewer runs). 0 when they share nothing of `from` characters or more. For choosing the
/// limit; the check does not use it.
pub fn longest_shared(
    matsu: &[(String, Prepared)],
    sakura: &[(String, Prepared)],
    from: usize,
) -> usize {
    let shares = |n: usize| !find_shared(matsu, sakura, n).is_empty();
    if !shares(from) {
        return 0;
    }
    let (mut low, mut high) = (from, from * 2);
    while shares(high) {
        low = high;
        high *= 2;
    }
    // `low` shares, `high` does not.
    while high - low > 1 {
        let middle = usize::midpoint(low, high);
        if shares(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    low
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Names {
        let mut names = Names::default();
        for name in list {
            names.add(name);
        }
        names
    }

    fn md(text: &str, names: &Names) -> [(String, Prepared); 1] {
        [("page.md".into(), prepare(&strip_markdown(text), names))]
    }

    fn html(text: &str, names: &Names) -> [(String, Prepared); 1] {
        [("HLP.html".into(), prepare(&strip_html(text), names))]
    }

    // Fixtures: sentences written for these tests (not taken from any help), in the style of help text.
    const COPY: &str = "検索を始める前に選択範囲があると、その範囲の中だけを対象にして、見つかった文字列を順に置き換えていきます。";

    #[test]
    fn a_copied_sentence_is_caught() {
        let n = Names::default();
        let sakura = html(&format!("<p>{COPY}</p>"), &n);
        let matsu = md(&format!("# 見出し\n\n{COPY}\n"), &n);
        let found = find_shared(&matsu, &sakura, MIN_SHARED_CHARS);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].matsu_line, 3);
        assert_eq!(found[0].sakura_file, "HLP.html");
        assert!(found[0].text.starts_with("検索を始める前に"));
    }

    #[test]
    fn a_copied_sentence_with_one_name_changed_is_caught() {
        let n = names(&["置換", "検索"]);
        let sakura = html(
            "<p>「置換」を選ぶと、見つけた文字列を、入力した文字列へ順に書き換えていく動きになります。</p>",
            &n,
        );
        let matsu = md(
            "「検索」を選ぶと、見つけた文字列を、入力した文字列へ順に書き換えていく動きになります。",
            &n,
        );
        assert_eq!(find_shared(&matsu, &sakura, MIN_SHARED_CHARS).len(), 1);
    }

    #[test]
    fn a_copy_inside_a_longer_text_is_caught_with_its_line() {
        let n = Names::default();
        let sakura = html(&format!("<p>前の文。{COPY}後の文。</p>"), &n);
        let matsu = md(&format!("あ\nい\nう {COPY} え"), &n);
        let found = find_shared(&matsu, &sakura, MIN_SHARED_CHARS);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].matsu_line, 3);
    }

    #[test]
    fn a_page_that_only_uses_the_same_names_is_not_caught() {
        let list = [
            "名前を付けて保存",
            "上書き保存",
            "開き直す",
            "新規ウインドウを開く",
            "FileSaveAs",
        ];
        let sakura_text = "<p>名前を付けて保存、上書き保存、開き直す、新規ウインドウを開く、FileSaveAs、名前を付けて保存。</p>";
        let matsu_text = "- 名前を付けて保存\n- 上書き保存\n- 開き直す\n- 新規ウインドウを開く\n- `FileSaveAs`\n- 名前を付けて保存\n";
        let n = names(&list);
        assert!(
            find_shared(
                &md(matsu_text, &n),
                &html(sakura_text, &n),
                MIN_SHARED_CHARS
            )
            .is_empty()
        );
        // Without the names, the same two pages do share text: the names are what the check leaves out.
        let none = Names::default();
        assert!(
            !find_shared(
                &md(matsu_text, &none),
                &html(sakura_text, &none),
                MIN_SHARED_CHARS
            )
            .is_empty()
        );
    }

    #[test]
    fn a_copied_sentence_that_contains_a_name_is_caught() {
        let n = names(&["上書き保存"]);
        let text = "変更した内容をファイルへ書き出したいときは「上書き保存」を選びます。そのとき確認は出ません。";
        let found = find_shared(
            &md(text, &n),
            &html(&format!("<p>{text}</p>"), &n),
            MIN_SHARED_CHARS,
        );
        assert_eq!(found.len(), 1);
        assert!(found[0].text.contains("〔上書き保存〕"));
    }

    #[test]
    fn white_space_and_line_breaks_do_not_hide_a_copy() {
        let n = Names::default();
        let sakura = html(&format!("<p>{COPY}</p>"), &n);
        let matsu = md(
            "検索を始める前に選択範囲があると、\nその範囲の中だけを\u{3000}対象にして、\n  見つかった文字列を順に\n置き換えていきます。\n",
            &n,
        );
        assert_eq!(find_shared(&matsu, &sakura, MIN_SHARED_CHARS).len(), 1);
    }

    #[test]
    fn full_width_letters_and_punctuation_are_folded() {
        let n = Names::default();
        let sakura = html(
            "<p>ＵＴＦ－８のファイルを開くと、先頭にあるＢＯＭを見て文字コードを決めるので、指定は要りません。</p>",
            &n,
        );
        let matsu = md(
            "UTF-8のファイルを開くと,先頭にあるBOMを見て文字コードを決めるので,指定は要りません。",
            &n,
        );
        assert_eq!(find_shared(&matsu, &sakura, MIN_SHARED_CHARS).len(), 1);
    }

    #[test]
    fn html_entities_and_tags_are_decoded() {
        let text = strip_html(
            "<P>A&amp;B&nbsp;&lt;C&gt;&#12354;&#x3044;<br>&copy;<!-- x --><script>if (a < b) {}</script>d</P>",
        );
        let s: String = text.iter().map(|(c, _)| *c).collect();
        assert_eq!(s, "A&B <C>あい\u{a9}d");
    }

    #[test]
    fn html_entities_do_not_hide_a_copy() {
        let n = Names::default();
        let sakura = html(
            "<p>設定した値が&quot;0&quot;のとき&nbsp;は、&lt;何も&gt;しないで、入力した文字列をそのまま&amp;残します。</p>",
            &n,
        );
        let matsu = md(
            "設定した値が\"0\"のときは、<何も>しないで、入力した文字列をそのまま&残します。",
            &n,
        );
        assert_eq!(find_shared(&matsu, &sakura, MIN_SHARED_CHARS).len(), 1);
    }

    #[test]
    fn markdown_syntax_is_not_text() {
        let text = strip_markdown(
            "# 見出し\n\n```\nコードの中の長い文は数えない数えない数えない\n```\n\n[リンク](other.md#anchor) と ![図](a.png)\n\n| a | b |\n|---|---|\n| c | d |\n\n[ref]: http://example.com\n<!-- 隠す\n複数行 -->残る\n{{#include x.txt}}\n",
        );
        let s: String = normalize(&text).iter().map(|n| n.c).collect();
        assert_eq!(s, "見出しリンクと図abcd残る");
    }

    #[test]
    fn a_match_with_too_few_counted_characters_does_not_hide_a_later_one() {
        // The MATSU page has 20 ASCII letters and then 20 Japanese characters. One Sakura page shares the
        // letters and the first 19 characters (19 counted), another shares only the 20 characters.
        let n = Names::default();
        let japanese = "あいうえおかきくけこさしすせそたちつてと";
        let letters = "abcdefghijklmnopqrst";
        let first = format!(
            "{letters}{}Z",
            &japanese[..japanese.char_indices().nth(19).unwrap().0]
        );
        let sakura = [
            ("A.html".to_string(), prepare(&strip_html(&first), &n)),
            ("B.html".to_string(), prepare(&strip_html(japanese), &n)),
        ];
        let matsu = md(&format!("{letters}{japanese}"), &n);
        let found = find_shared(&matsu, &sakura, 20);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].sakura_file, "B.html");
    }

    #[test]
    fn a_longer_fence_is_not_closed_by_a_shorter_one() {
        let text = strip_markdown("前\n````md\n```\n中の文\n```\n中の文2\n````\n後\n");
        let s: String = normalize(&text).iter().map(|n| n.c).collect();
        assert_eq!(s, "前後");
    }

    #[test]
    fn a_fence_inside_a_comment_opens_nothing() {
        let text = strip_markdown("前\n<!--\n```\n-->\n後\n");
        let s: String = normalize(&text).iter().map(|n| n.c).collect();
        assert_eq!(s, "前後");
    }

    #[test]
    fn footnote_text_is_prose() {
        let text = strip_markdown("本文[^1]\n\n[^1]: 補足説明です\n");
        let s: String = normalize(&text).iter().map(|n| n.c).collect();
        assert!(s.contains("補足説明です"), "{s}");
    }

    #[test]
    fn the_labels_of_reference_links_are_not_text() {
        let text = strip_markdown("[検索][search]して[置換][]する [図][x]\n\n[search]: a.md\n");
        let s: String = normalize(&text).iter().map(|n| n.c).collect();
        assert_eq!(s, "検索して置換する図");
    }

    #[test]
    fn markdown_character_references_are_decoded() {
        let text = strip_markdown("a&nbsp;b&#12354;&amp;c &unknown; d");
        let s: String = text.iter().map(|(c, _)| *c).collect();
        assert_eq!(s, "a b\u{3042}&c &unknown; d\n");
    }

    #[test]
    fn short_common_phrases_are_not_caught() {
        let n = Names::default();
        let sakura = html("<p>ファイルを開いて、編集します。</p>", &n);
        let matsu = md("ファイルを開いて、編集します。", &n);
        assert!(find_shared(&matsu, &sakura, MIN_SHARED_CHARS).is_empty());
    }

    #[test]
    fn ascii_names_match_whole_words_only() {
        let n = names(&["Up"]);
        let p = prepare(&strip_markdown("Update Up Up2"), &n);
        // "Update" and "Up2" keep their letters; the lone "Up" is a name.
        assert_eq!(p.chars.iter().filter(|c| **c == NAME).count(), 1);
    }

    #[test]
    fn labels_are_reduced_to_words() {
        let mut n = Names::default();
        add_label(&mut n, "名前を付けて保存...");
        add_label(&mut n, "ファイル(&F)");
        add_label(&mut n, "A&&B");
        let p = prepare(&strip_markdown("名前を付けて保存 ファイル a&b"), &n);
        assert_eq!(p.chars.iter().filter(|c| **c == NAME).count(), 3);
    }
}
