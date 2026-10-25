// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/doc/CDocOutline.cpp, sakura_core/io/CTextStream.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000, 2001, genta
//   Copyright (C) 2002, frozen
//   Copyright (C) 2003, zenryaku
//   Copyright (C) 2005, genta, D.S.Koba, じゅうじ
//   Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (a UTF-16 buffer
// walked with `CDocLineMgr`, the results put in `CFuncInfoArr`, a Windows dialog) was rewritten in
// Rust on the lines of `LineSource`, the positions in the layout are left to the window, the
// regular expressions go through `matsu-regex` in place of bregonig.dll, and the tests are new.
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

//! The outline by a rule file (アウトライン解析のルールファイル).
//!
//! Sakura: `CDocOutline::ReadRuleFile` and `CDocOutline::MakeFuncList_RuleFile`
//! (`sakura_core/doc/CDocOutline.cpp`, YAZAKI 2002.04.01); the file is read by
//! `CTextInputStream_AbsIni` (`sakura_core/io/CTextStream.cpp`).
//!
//! # The rule file
//!
//! A text file in UTF-8 with a BOM, or else in Shift_JIS. A line with ` /// ` in it is a rule
//! (unless it starts with the comment character, `;` at first):
//!
//! ```text
//! normal mode:             key1,key2 /// GroupName,Lv=1
//! regular expression mode: Regex /// GroupName,Lv=1
//! regex replace mode:      Regex /// TitleReplace /// GroupName
//! ```
//!
//! A line that starts with the comment character is a directive when it is one of
//! `;CommentChar=c`, `;Mode=Regex`, `;Mode=RegexReplace`, `;Title=text` and
//! `;RegexOption=iR...`; any other such line is a comment. The comment character is the first
//! character of a directive too.

use std::ops::Range;

use matsu_core::encoding::Encoding;
use matsu_regex::{
    GroupNames, Onigmo, Options, PatternError, Regex, RegexEngine, ReplacementTemplate,
    ends_with_escape, make_pattern_alternate,
};

use crate::chars::{is_blank, is_line_delimiter};
use crate::info::NO_CLIP_TEXT;
use crate::{FuncInfoArr, LineSource};

/// The most rules (the `1024` of `MakeFuncList_RuleFile`).
const MAX_RULES: usize = 1024;

/// The longest line of the rule file whose key is read (`LINEREADBUFSIZE`).
const LINE_READ_BUFFER: usize = 10240;

/// The delimiter of the key and the group name (`pszDelimit`).
const DELIMITER: &str = " /// ";

/// The size of a match text, a replacement and a group name, with the null (`szMatch[256]`).
const TEXT_BUFFER: usize = 256;

/// The deepest nesting of the groups (`nMaxStack`).
const MAX_STACK: usize = 32;

/// The group name whose lines are not listed.
const EXCEPT_GROUP: &str = "Except";

/// The options of a rule (`nRegexOption`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RuleOptions {
    /// Without `optCaseSensitive`: the `i` of `;RegexOption=`.
    ignore_case: bool,
    /// `optGlobal` (`g`): every match of the line is replaced (replace mode).
    global: bool,
    /// `optExtend` (`x`).
    extended: bool,
    /// `optASCII` (`a`).
    ascii: bool,
    /// `optR` (`R`): CR LF is one line ending.
    crlf: bool,
}

/// One rule (`SOneRule`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Rule {
    /// The key, or the regular expression (`szMatch`).
    pub(crate) match_text: String,
    /// The replacement of the title in the replace mode (`szText`).
    pub(crate) replace: String,
    /// The rest of the line after the delimiter, with `,Lv=n` in it (`szGroupName`).
    pub(crate) group: String,
    /// The level (`nLv`).
    pub(crate) level: i32,
    options: RuleOptions,
    /// `nRegexMode` is 1 (`;Mode=RegexReplace` and a title replacement in the line).
    pub(crate) replace_mode: bool,
}

/// What `ReadRuleFile` gives.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RuleFile {
    pub(crate) rules: Vec<Rule>,
    /// `bRegex`
    pub(crate) regex: bool,
    /// `title`: `;Title=`, empty if none.
    pub(crate) title: String,
}

/// The outline by the rule file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleFileOutline {
    pub items: FuncInfoArr,
    /// The title of the pane that the file sets (`sTitleOverride`).
    pub title: Option<String>,
    /// The message of a regular expression that cannot be used (`STR_DOCOUTLINE_REGEX`): Sakura
    /// shows it in a box and stops with the list empty.
    pub error: Option<String>,
}

/// The characters of the file (`CTextInputStream`): UTF-8 when it starts with a BOM, else
/// Shift_JIS.
fn decode(bytes: &[u8]) -> String {
    let mut text = String::new();
    if let Some(utf8) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        text.push_str(&String::from_utf8_lossy(utf8));
    } else {
        Encoding::ShiftJis.codec().decode(bytes, &mut text);
    }
    text
}

/// `ReadLineW`: the lines, ended by CR LF, LF or CR.
fn split_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                lines.push(std::mem::take(&mut current));
            }
            '\n' => lines.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// The first `n` characters (a UTF-16 unit is a character here).
fn take_chars(text: &str, n: usize) -> String {
    text.chars().take(n).collect()
}

/// `_wtoi`: the number at the start of `text` (blanks, a sign, digits), 0 when there is none.
fn atoi(text: &str) -> i32 {
    let text = text.trim_start_matches([' ', '\t', '\n', '\u{b}', '\u{c}', '\r']);
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let mut value: i64 = 0;
    for d in digits.chars().map_while(|c| c.to_digit(10)) {
        value = (value * 10 + i64::from(d)).min(i64::from(i32::MAX) + 1);
    }
    let value = if negative { -value } else { value };
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// Whether the characters after the first one of `line` are `lit`, ignoring the case of ASCII
/// letters (`wcsnicmp_literal( line + 1, lit )`).
fn directive_starts(line: &[char], lit: &str) -> bool {
    let lit: Vec<char> = lit.chars().collect();
    line.len() > lit.len()
        && line[1..=lit.len()]
            .iter()
            .zip(&lit)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

/// `ReadRuleFile`: the rules of the file.
pub(crate) fn parse_rule_file(bytes: &[u8]) -> RuleFile {
    let mut file = RuleFile::default();
    let mut comment = Some(';');
    let mut regex_replace = false;
    let mut options = RuleOptions::default();
    for line in split_lines(&decode(bytes)) {
        if file.rules.len() >= MAX_RULES {
            break;
        }
        let chars: Vec<char> = line.chars().collect();
        let first_is_comment = chars.first().copied() == comment && comment.is_some();
        if let Some(at) = line.find(DELIMITER)
            && !first_is_comment
        {
            let key = &line[..at];
            if key.chars().count() >= LINE_READ_BUFFER {
                // the line is too long
                continue;
            }
            let mut rest = &line[at + DELIMITER.len()..];
            read_rule(&mut file, key, &mut rest, regex_replace, options);
        } else if first_is_comment {
            // a directive
            let len = chars.len();
            if (13..=14).contains(&len) && directive_starts(&chars, "CommentChar=") {
                comment = if len == 13 { None } else { Some(chars[13]) };
            } else if len == 11 && directive_equals(&chars, "Mode=Regex") {
                file.regex = true;
                regex_replace = false;
            } else if len == 18 && directive_equals(&chars, "Mode=RegexReplace") {
                file.regex = true;
                regex_replace = true;
            } else if len >= 7 && directive_starts(&chars, "Title=") {
                file.title = chars[7..].iter().collect();
            } else if len > 13 && directive_starts(&chars, "RegexOption=") {
                options = RuleOptions::default();
                for &c in &chars[13..] {
                    match c {
                        'i' => options.ignore_case = true,
                        'g' => options.global = true,
                        'x' => options.extended = true,
                        'a' => options.ascii = true,
                        'R' => options.crlf = true,
                        // `u`, `d` and `l` choose how bregonig.dll treats `\w` and the like;
                        // the engine here has the one way of Unicode
                        _ => {}
                    }
                }
            }
        }
    }
    file
}

/// `_wcsicmp( line + 1, lit ) == 0` for a line of the length of `lit` and one.
fn directive_equals(line: &[char], lit: &str) -> bool {
    lit.chars().count() + 1 == line.len() && directive_starts(line, lit)
}

/// The rule line `key /// rest` into rules (the body of the loop of `ReadRuleFile`).
fn read_rule(
    file: &mut RuleFile,
    key: &str,
    rest: &mut &str,
    regex_replace: bool,
    options: RuleOptions,
) {
    let count = file.rules.len();
    let mut replace = String::new();
    let mut top_dummy = false;
    let mut replace_mode = false;
    let tokens: Vec<String>;
    if file.regex {
        // a regular expression is not split at commas
        if key.is_empty() {
            if count > 0 {
                // an empty key is ignored
                tokens = Vec::new();
            } else {
                // an empty key at the start is the root of the tree
                top_dummy = true;
                tokens = vec![String::new()];
            }
        } else {
            tokens = vec![key.to_owned()];
        }
        if regex_replace && !tokens.is_empty() {
            // `titleRep /// group`
            if let Some(group_at) = rest.find(DELIMITER) {
                // the replacement up to 255 characters
                replace = take_chars(&rest[..group_at], TEXT_BUFFER - 1);
                replace_mode = true;
                *rest = &rest[group_at + DELIMITER.len()..];
            }
        }
    } else {
        // `wcstok_s( szLine, ",", ... )`: the keys, without the empty ones
        let keys: Vec<String> = key
            .split(',')
            .filter(|k| !k.is_empty())
            .map(str::to_owned)
            .collect();
        if count == 0 && keys.is_empty() {
            // the root of the tree (the key itself stays, whatever it is made of)
            top_dummy = true;
            tokens = vec![key.to_owned()];
        } else {
            tokens = keys;
        }
    }
    let level = rest.find(",Lv=").map_or(0, |at| atoi(&rest[at + 4..]));
    let group = take_chars(rest, TEXT_BUFFER - 1);
    for token in tokens {
        if file.rules.len() >= MAX_RULES {
            break;
        }
        file.rules.push(Rule {
            match_text: take_chars(&token, TEXT_BUFFER - 1),
            replace: replace.clone(),
            group: group.clone(),
            level,
            options,
            replace_mode,
        });
        if top_dummy || file.regex {
            break;
        }
    }
}

/// A rule whose regular expression is compiled.
struct Compiled {
    regex: Box<dyn Regex>,
    /// The replacement of the replace mode.
    template: Option<ReplacementTemplate>,
    global: bool,
}

/// `STR_DOCOUTLINE_REGEX` of `sakura_rc.rc`.
fn regex_error(rule: &Rule, message: &str) -> String {
    let shown = if rule.replace_mode {
        format!("{}\n{}", rule.match_text, rule.replace)
    } else {
        rule.match_text.clone()
    };
    format!("正規表現ルール\n{shown}\nでエラーが発生しました。\n{message}")
}

/// `CBregexp::Compile` of a rule.
fn compile(rule: &Rule) -> Result<Compiled, String> {
    if ends_with_escape(&rule.match_text) {
        return Err(regex_error(
            rule,
            &PatternError::UnmatchedSeparator.to_string(),
        ));
    }
    let prepared = make_pattern_alternate(&rule.match_text);
    let options = Options {
        ignore_case: rule.options.ignore_case,
        extended: rule.options.extended,
        ascii_range: rule.options.ascii,
        crlf: rule.options.crlf,
    };
    let regex = Onigmo
        .compile(&prepared, options)
        .map_err(|e| regex_error(rule, e.message()))?;
    let template = if rule.replace_mode {
        let names = GroupNames::scan(&prepared, options.extended, regex.capture_count());
        let template = ReplacementTemplate::parse(&rule.replace, &names, regex.capture_count())
            .map_err(|e| regex_error(rule, &e.to_string()))?;
        Some(template)
    } else {
        None
    };
    Ok(Compiled {
        regex,
        template,
        global: rule.options.global,
    })
}

/// The groups of a match as the template wants them (0 is the whole match).
fn groups_of(caps: &matsu_regex::Captures) -> Vec<Option<Range<usize>>> {
    caps.iter().collect()
}

impl Compiled {
    /// `CBregexp::Match( line, len, 0 )`
    fn is_match(&self, line: &str) -> bool {
        matches!(self.regex.captures_at(line, 0), Ok(Some(_)))
    }

    /// `CBregexp::Replace( line, len, 0 )` and the title taken from the result: the text of the
    /// first match after the replacement (from where the first match starts, as long as the
    /// replacement of the first match when only one is made).
    fn replaced_title(&self, line: &str) -> Option<String> {
        let template = self.template.as_ref()?;
        let first = self.regex.captures_at(line, 0).ok().flatten()?;
        let first_range = first.whole();
        let mut out = String::with_capacity(line.len());
        if self.global {
            // every match of the line, as the `g` option does
            let (mut at, mut pos, mut gpos) = (0, 0, 0);
            while let Ok(Some(caps)) = self.regex.captures_from(line, pos, gpos) {
                let m = caps.whole();
                let empty = m.is_empty();
                if m.start >= at {
                    out.push_str(&line[at..m.start]);
                    out.push_str(&template.expand(line, &groups_of(&caps)));
                    at = m.end;
                }
                gpos = m.end;
                pos = if empty {
                    match line[m.end..].chars().next() {
                        Some(c) => m.end + c.len_utf8(),
                        None => break,
                    }
                } else {
                    m.end
                };
            }
            out.push_str(&line[at..]);
        } else {
            out.push_str(&line[..first_range.start]);
            out.push_str(&template.expand(line, &groups_of(&first)));
            out.push_str(&line[first_range.end..]);
        }
        // `pszText = GetString() + GetIndex()`, `nTextLen = GetStringLen() - nLineLen + nMatchLen`
        let index = line[..first_range.start].chars().count();
        let out_len = out.chars().count();
        let line_len = line.chars().count();
        let match_len = line[first_range].chars().count();
        let text_len = (out_len + match_len).saturating_sub(line_len);
        Some(out.chars().skip(index).take(text_len).collect())
    }
}

/// `MakeFuncList_RuleFile`: the outline of the lines by the rule file `file` (its bytes; `None` when
/// the file cannot be opened).
pub fn make_func_list_rule_file(src: &dyn LineSource, file: Option<&[u8]>) -> RuleFileOutline {
    let mut result = RuleFileOutline::default();
    let Some(file) = file else {
        return result;
    };
    let parsed = parse_rule_file(file);
    if parsed.rules.is_empty() {
        return result;
    }
    if !parsed.title.is_empty() {
        result.title = Some(parsed.title.clone());
    }
    let rules = &parsed.rules;

    // the regular expressions
    let mut compiled: Vec<Option<Compiled>> = Vec::new();
    if parsed.regex {
        for rule in rules {
            if rule.match_text.is_empty() {
                compiled.push(None);
                continue;
            }
            match compile(rule) {
                Ok(c) => compiled.push(Some(c)),
                Err(message) => {
                    result.error = Some(message);
                    return result;
                }
            }
        }
    }

    // The levels are `wchar_t` in the stack of Sakura: a negative level wraps around.
    let mut stack: Vec<(String, u16)> = Vec::new();
    let mut depth = 0usize;
    // an empty first key is the root: its name is the group name up to the comma
    if rules[0].match_text.is_empty() {
        let group = &rules[0].group;
        stack.push((group.clone(), rules[0].level as u16));
        let name = group.split(',').next().unwrap_or("");
        result.items.push(1, name, NO_CLIP_TEXT, depth);
        depth = 1;
    }
    for n in 0..src.line_count() {
        let Some(line) = src.line(n) else {
            break;
        };
        // blanks at the start are skipped (normal mode)
        let mut start = 0;
        if !parsed.regex {
            while start < line.len() && is_blank(line[start]) {
                start += 1;
            }
            if start >= line.len() {
                continue;
            }
        }
        let text: String = line.iter().collect();
        // the rule that matches the line
        let mut matched: Option<(usize, Option<String>)> = None;
        for (j, rule) in rules.iter().enumerate() {
            if rule.match_text.is_empty() {
                continue;
            }
            if parsed.regex {
                let Some(Some(c)) = compiled.get(j) else {
                    continue;
                };
                if !rule.replace_mode {
                    if c.is_match(&text) {
                        matched = Some((j, None));
                        break;
                    }
                } else if let Some(title) = c.replaced_title(&text) {
                    matched = Some((j, Some(title)));
                    break;
                }
            } else if line[start..].starts_with(&rule.match_text.chars().collect::<Vec<_>>()) {
                matched = Some((j, None));
                break;
            }
        }
        let Some((j, replaced)) = matched else {
            continue;
        };
        let rule = &rules[j];
        if rule.group == EXCEPT_GROUP {
            continue;
        }

        // the line without its line ending (a replaced title is as it is)
        let title = replaced.unwrap_or_else(|| {
            line[start..]
                .iter()
                .take_while(|&&c| !is_line_delimiter(c))
                .collect()
        });

        // the depth
        let level = rule.level;
        let found = stack[..depth]
            .iter()
            .position(|(name, _)| *name == rule.group);
        let mut append = true;
        if let Some(found) = found {
            // the same group was seen: the item goes to its level
            depth = found;
        } else if depth < MAX_STACK {
            // a new group goes under the nearest group of a smaller or equal level
            let at = (0..depth)
                .rev()
                .find(|&i| i32::from(stack[i].1) <= level)
                .map_or(0, |i| i + 1);
            stack.truncate(at);
            stack.push((rule.group.clone(), level as u16));
            depth = at;
        } else {
            // more nesting than the stack holds
            append = false;
        }
        if append {
            result.items.push(n + 1, title, 0, depth);
            depth += 1;
        }
    }
    result
}
