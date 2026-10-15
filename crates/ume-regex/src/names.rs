//! Names of capture groups, for replacements such as `$+{name}`.
//!
//! The engine behind [`Regex`](crate::Regex) does not tell which number a named group has, and
//! the C shim of the Onigmo build is not ours to extend here, so the numbers are worked out from
//! the pattern text. The scan knows what can hide a `(`: escapes, `\Q...\E`, character classes,
//! `(?#...)` comments and `x`-mode comments. The result is checked against the number of groups
//! the engine counted; if the two disagree the names stay unresolved (and a replacement that
//! uses a name reports it, see [`ReplacementProblem`](crate::ReplacementProblem)).
//!
//! Which groups count is the engine's rule, not ours. With ONIG_SYNTAX_PERL as bregonig builds it
//! (`ONIG_OPTION_CAPTURE_GROUP` is in the syntax's options, `regsyntax.c`) every group captures,
//! named or not, in the order of its `(`. A numbering in which only the named groups count (the
//! Ruby rule) is tried as well, in case the engine is built differently.

/// Group name to the numbers of the groups that have it (in the order of the pattern; the same
/// name may be used more than once, `ONIG_SYN_ALLOW_MULTIPLEX_DEFINITION_NAME`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupNames {
    names: Vec<(String, Vec<usize>)>,
    /// The pattern has named groups, but they could not be matched with the engine's count.
    unresolved: bool,
}

impl GroupNames {
    /// The numbers (1 or more) of the groups called `name`, leftmost first.
    pub fn numbers(&self, name: &str) -> &[usize] {
        self.names
            .iter()
            .find(|(n, _)| n == name)
            .map_or(&[], |(_, v)| v.as_slice())
    }

    /// Whether the pattern has no named group.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && !self.unresolved
    }

    /// Whether the names of the pattern could not be tied to group numbers.
    pub fn is_unresolved(&self) -> bool {
        self.unresolved
    }

    /// Scans `pattern`, which the engine counted as having `capture_count` groups.
    /// `extended` is the `x` option.
    pub fn scan(pattern: &str, extended: bool, capture_count: usize) -> Self {
        let found = scan_groups(pattern, extended);
        let named = found.iter().filter(|g| g.name.is_some()).count();
        if named == 0 {
            return Self::default();
        }
        let plain = found.len() - named;
        let numbering: Vec<Option<usize>> = if capture_count == found.len() {
            (1..=found.len()).map(Some).collect()
        } else if capture_count == named && plain > 0 {
            let mut next = 0;
            found
                .iter()
                .map(|g| {
                    g.name.as_ref().map(|_| {
                        next += 1;
                        next
                    })
                })
                .collect()
        } else {
            return Self {
                names: Vec::new(),
                unresolved: true,
            };
        };
        let mut names: Vec<(String, Vec<usize>)> = Vec::new();
        for (g, number) in found.iter().zip(numbering) {
            let (Some(name), Some(number)) = (&g.name, number) else {
                continue;
            };
            match names.iter_mut().find(|(n, _)| n == name) {
                Some((_, v)) => v.push(number),
                None => names.push((name.clone(), vec![number])),
            }
        }
        Self {
            names,
            unresolved: false,
        }
    }
}

struct Group {
    name: Option<String>,
}

/// The capturing groups of the pattern in order of their `(`.
fn scan_groups(pattern: &str, extended: bool) -> Vec<Group> {
    let c: Vec<char> = pattern.chars().collect();
    let mut groups = Vec::new();
    // One flag per open group: whether `x` mode is on inside it.
    let mut extended_stack = vec![extended];
    let mut class_level = 0usize;
    let mut in_quote = false;
    let mut i = 0;
    while i < c.len() {
        if in_quote {
            if c[i] == '\\' && c.get(i + 1) == Some(&'E') {
                in_quote = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if class_level > 0 {
            match c[i] {
                '\\' => i += escape_len(&c, i),
                '[' => {
                    class_level += 1;
                    i = class_body_start(&c, i);
                }
                ']' => {
                    class_level -= 1;
                    i += 1;
                }
                _ => i += 1,
            }
            continue;
        }
        match c[i] {
            '\\' => {
                if c.get(i + 1) == Some(&'Q') {
                    in_quote = true;
                    i += 2;
                } else {
                    i += escape_len(&c, i);
                }
            }
            '[' => {
                class_level = 1;
                i = class_body_start(&c, i);
            }
            '#' if *extended_stack.last().unwrap_or(&false) => {
                while i < c.len() && c[i] != '\n' {
                    i += 1;
                }
            }
            '(' => i = open_group(&c, i, &mut groups, &mut extended_stack),
            ')' => {
                if extended_stack.len() > 1 {
                    extended_stack.pop();
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    groups
}

/// Where the body of the class that opens at `i` (a `[`) starts: after an optional `^`, and a
/// `]` that comes first is a plain char of the class. Nested classes follow the same rule.
fn class_body_start(c: &[char], i: usize) -> usize {
    let mut j = i + 1;
    if c.get(j) == Some(&'^') {
        j += 1;
    }
    if c.get(j) == Some(&']') {
        j += 1;
    }
    j
}

/// The length of the escape that starts at `i` (a backslash). `\cx` takes the control char too.
fn escape_len(c: &[char], i: usize) -> usize {
    match c.get(i + 1) {
        None => 1,
        Some('c') => match c.get(i + 2) {
            // The operand is an escape of its own, which Onigmo parses the same way.
            Some('\\') => 2 + escape_len(c, i + 2),
            Some(_) => 3,
            None => 2,
        },
        Some(_) => 2,
    }
}

/// Handles the `(` at `i`; returns where the scan goes on.
fn open_group(c: &[char], i: usize, groups: &mut Vec<Group>, stack: &mut Vec<bool>) -> usize {
    let inherited = *stack.last().unwrap_or(&false);
    if c.get(i + 1) != Some(&'?') {
        groups.push(Group { name: None });
        stack.push(inherited);
        return i + 1;
    }
    match c.get(i + 2) {
        // `(?#...)`: a comment up to the next `)` that is not escaped.
        Some('#') => {
            let mut j = i + 3;
            while j < c.len() && c[j] != ')' {
                j += if c[j] == '\\' { 2 } else { 1 };
            }
            (j + 1).min(c.len())
        }
        // `(?<name>...)`; `(?<=` and `(?<!` are look-behind.
        Some('<') if !matches!(c.get(i + 3), Some('=' | '!')) => {
            named(c, i + 3, '>', i, groups, stack, inherited)
        }
        Some('\'') => named(c, i + 3, '\'', i, groups, stack, inherited),
        // `(?P<name>...)` (ONIG_SYN_OP2_QMARK_CAPITAL_P_NAMED_GROUP, on in Onigmo's Perl syntax);
        // `(?P=name)` and `(?P>name)` are references and fall through to the generic case.
        Some('P') if c.get(i + 3) == Some(&'<') => {
            named(c, i + 4, '>', i, groups, stack, inherited)
        }
        // `(?(1)yes|no)`, `(?(<name>)..)`, `(?('name')..)`: the condition in parentheses is not a
        // group; skip it up to its `)` and treat the rest as an ordinary group.
        Some('(') => {
            let mut j = i + 3;
            while j < c.len() && c[j] != ')' {
                j += 1;
            }
            stack.push(inherited);
            (j + 1).min(c.len())
        }
        // `(?imx-imx)` changes the options of the rest of the enclosing group; `(?imx-imx:...)`
        // opens a group with them.
        _ => {
            let mut j = i + 2;
            let mut on = true;
            let mut x = inherited;
            while let Some(&ch) = c.get(j) {
                match ch {
                    '-' => on = false,
                    // `(?^...)` loads the default options: `x` off (regparse.c, `case '^'`).
                    '^' => x = false,
                    'x' => x = on,
                    'i' | 'm' | 's' | 'a' | 'd' | 'l' | 'u' | 'R' => {}
                    _ => break,
                }
                j += 1;
            }
            match c.get(j) {
                Some(')') => {
                    if let Some(top) = stack.last_mut() {
                        *top = x;
                    }
                    j + 1
                }
                Some(':') => {
                    stack.push(x);
                    j + 1
                }
                // Look-ahead, look-behind, atomic and absence groups, and anything else.
                _ => {
                    stack.push(inherited);
                    i + 2
                }
            }
        }
    }
}

fn named(
    c: &[char],
    from: usize,
    end: char,
    open: usize,
    groups: &mut Vec<Group>,
    stack: &mut Vec<bool>,
    inherited: bool,
) -> usize {
    let mut j = from;
    while j < c.len() && c[j] != end {
        j += 1;
    }
    if j >= c.len() {
        // Not closed: the engine rejects the pattern; keep scanning after the `(`.
        return open + 1;
    }
    groups.push(Group {
        name: Some(c[from..j].iter().collect()),
    });
    stack.push(inherited);
    j + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbers(pattern: &str, count: usize, name: &str) -> Vec<usize> {
        GroupNames::scan(pattern, false, count)
            .numbers(name)
            .to_vec()
    }

    #[test]
    fn finds_named_groups_among_plain_ones_when_all_capture() {
        assert_eq!(numbers("(a)(?<x>b)(c)", 3, "x"), vec![2]);
    }

    #[test]
    fn plain_groups_do_not_count_when_only_named_ones_capture() {
        assert_eq!(numbers("(a)(?<x>b)(c)(?'y'd)", 2, "y"), vec![2]);
    }

    #[test]
    fn duplicate_names_list_every_number() {
        assert_eq!(numbers("(?<x>a)|(?<x>b)", 2, "x"), vec![1, 2]);
    }

    #[test]
    fn hidden_parentheses_are_not_groups() {
        // An escape, a class, \Q...\E and a comment hide their parentheses.
        assert_eq!(numbers(r"\((?<x>a)[(]\Q(\E(?#(b)(?<y>c)", 2, "x"), vec![1]);
        assert_eq!(numbers(r"\((?<x>a)[(]\Q(\E(?#(b)(?<y>c)", 2, "y"), vec![2]);
    }

    #[test]
    fn look_behind_is_not_a_named_group() {
        assert_eq!(numbers("(?<=a)(?<x>b)", 1, "x"), vec![1]);
        assert_eq!(numbers("(?<!a)(?<x>b)", 1, "x"), vec![1]);
    }

    #[test]
    fn caret_resets_extended_mode() {
        let names = GroupNames::scan("(?x)(?^)#(?<x>a)", false, 1);
        assert_eq!(names.numbers("x"), &[1]);
    }

    #[test]
    fn a_bracket_that_comes_first_in_a_nested_class_is_a_char() {
        let names = GroupNames::scan("[a[]](?<fake>)](?<x>b)", false, 1);
        assert_eq!(names.numbers("x"), &[1]);
    }

    #[test]
    fn nested_control_escapes_are_consumed() {
        let names = GroupNames::scan(r"\c\c((b)(?<x>a)", false, 2);
        assert_eq!(names.numbers("x"), &[2]);
    }

    #[test]
    fn conditional_headers_are_not_groups() {
        // The engine counts (a), yes and no.
        assert_eq!(numbers("(a)?(?(1)(?<yes>b)|(?<no>c))", 3, "yes"), vec![2]);
        assert_eq!(numbers("(a)?(?(1)(?<yes>b)|(?<no>c))", 3, "no"), vec![3]);
        assert_eq!(numbers("(?<n>a)?(?(<n>)(?<y>b))", 2, "y"), vec![2]);
        assert_eq!(numbers("(?<n>a)?(?('n')(?<y>b))", 2, "y"), vec![2]);
    }

    #[test]
    fn extended_mode_comments_hide_parentheses() {
        let names = GroupNames::scan("(?<x>a) # (not a group\n(?<y>b)", true, 2);
        assert_eq!(names.numbers("y"), &[2]);
        let names = GroupNames::scan("(?x)(?<x>a) # (not a group\n(?<y>b)", false, 2);
        assert_eq!(names.numbers("y"), &[2]);
    }

    #[test]
    fn a_count_that_does_not_fit_leaves_the_names_unresolved() {
        let names = GroupNames::scan("(?<x>a)", false, 5);
        assert!(names.is_unresolved());
        assert!(names.numbers("x").is_empty());
    }

    #[test]
    fn patterns_without_names_have_no_entries() {
        let names = GroupNames::scan("(a)(b)", false, 2);
        assert!(names.is_empty());
    }
}
