//! Which document type a file has: by its name, from the extensions of each type.
//!
//! Sakura: `CDocTypeManager::GetDocumentTypeOfPath` and `IsFileNameMatch`
//! (`sakura_core/env/CDocTypeManager.cpp`). The types are tried in order from type 0; the first
//! whose extension list (`szTypeExts`) matches wins, and a file that matches none is type 0
//! ("基本", the basic type).

/// Separators of the extension list (`m_typeExtSeps`).
const EXT_SEPARATORS: [char; 3] = [' ', ';', ','];
/// Wildcards (`m_typeExtWildcards`).
const WILDCARDS: [char; 2] = ['*', '?'];

/// `IsFileNameMatch`: does `file_name` match the extension list `type_exts` (`"c cpp;h"`,
/// `"*.txt"`, `"Makefile"`)?
///
/// A token without wildcards matches the whole file name, or the text after the last `.`; a token
/// with wildcards is a pattern for the whole file name (`PathMatchSpec`). Case does not matter.
pub fn is_file_name_match(type_exts: &str, file_name: &str) -> bool {
    let name = lower(file_name);
    let ext = name.rfind('.').map(|i| &name[i + 1..]);
    type_exts
        .split(EXT_SEPARATORS)
        .filter(|t| !t.is_empty())
        .any(|token| {
            let token = lower(token);
            if token.contains(WILDCARDS) {
                glob_match(&token, &name)
            } else {
                token == name || ext == Some(token.as_str())
            }
        })
}

fn lower(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

/// `*` (any run of characters, also none) and `?` (one character) against the whole of `text`.
fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0, 0);
    // Where the last `*` was in the pattern, and the text position it matched up to.
    let mut star: Option<(usize, usize)> = None;
    while ti < t.len() {
        match p.get(pi) {
            Some('*') => {
                star = Some((pi, ti));
                pi += 1;
            }
            Some(&c) if c == '?' || c == t[ti] => {
                pi += 1;
                ti += 1;
            }
            _ => match star {
                Some((sp, st)) => {
                    star = Some((sp, st + 1));
                    pi = sp + 1;
                    ti = st + 1;
                }
                None => return false,
            },
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

/// The index of the first type whose extension list matches `file_name`, else 0
/// (`GetDocumentTypeOfPath`). `type_exts` has the extension list of each type, in the order of
/// the types.
pub fn type_index_for<'a>(type_exts: impl IntoIterator<Item = &'a str>, file_name: &str) -> usize {
    type_exts
        .into_iter()
        .position(|exts| is_file_name_match(exts, file_name))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_token_matches_the_extension_or_the_whole_name() {
        assert!(is_file_name_match("txt", "memo.txt"));
        assert!(is_file_name_match("txt", "MEMO.TXT"));
        assert!(is_file_name_match("Makefile", "makefile"));
        assert!(!is_file_name_match("txt", "memo.txt.bak"));
        assert!(!is_file_name_match("txt", "txt.bak"));
        // `szTypeExts` is not "a dot and an extension": ".txt" does not match "memo.txt".
        assert!(!is_file_name_match(".txt", "memo.txt"));
    }

    #[test]
    fn the_separators_are_space_semicolon_and_comma() {
        for exts in ["c cpp h", "c;cpp;h", "c,cpp,h", "c ;, cpp"] {
            assert!(is_file_name_match(exts, "x.cpp"), "{exts}");
            assert!(!is_file_name_match(exts, "x.rs"), "{exts}");
        }
        assert!(!is_file_name_match("", "x.cpp"));
    }

    #[test]
    fn a_double_extension_matches_its_last_part() {
        // Sakura compares with the text after the last dot only.
        assert!(is_file_name_match("gz", "a.tar.gz"));
        assert!(!is_file_name_match("tar", "a.tar.gz"));
    }

    #[test]
    fn wildcards_match_the_whole_name() {
        assert!(is_file_name_match("*.txt", "memo.txt"));
        assert!(is_file_name_match("memo?.txt", "memo1.txt"));
        assert!(!is_file_name_match("memo?.txt", "memo.txt"));
        assert!(is_file_name_match("*", "anything"));
        assert!(is_file_name_match("a*b*c", "aXXbYYc"));
        assert!(!is_file_name_match("a*b*c", "aXXbYY"));
        assert!(is_file_name_match("README*", "readme"));
        assert!(!is_file_name_match("*.txt", "memo.txt.bak"));
    }

    #[test]
    fn the_first_matching_type_wins_and_type_zero_is_the_fallback() {
        let types = ["", "txt doc", "c cpp h", "*.h"];
        assert_eq!(type_index_for(types, "a.txt"), 1);
        assert_eq!(type_index_for(types, "a.h"), 2);
        assert_eq!(type_index_for(types, "a.rs"), 0);
        assert_eq!(type_index_for([], "a.rs"), 0);
        // Type 0 is tried too.
        assert_eq!(type_index_for(["rs", "txt"], "a.rs"), 0);
    }
}
