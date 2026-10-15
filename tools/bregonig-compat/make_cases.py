#!/usr/bin/env python3
"""Writes cases.json, the cases that run.sh gives to the harness and that the Rust test replays.

Run it after editing the lists below:  python3 tools/bregonig-compat/make_cases.py

Fields of a case:
  id            unique name
  kind          "match" (BMatchEx), "replace" (BSubstEx without g: the first match from `start`) or
                "replace_all" (BSubstEx with g)
  pattern       the search pattern, as the user types it
  replacement   the replacement text (replace kinds)
  subject       the text of one line (UTF-16 in the DLL), no line ending unless the case is about one
  start         where the search starts, in UTF-16 units (default 0)
  end           where the text handed to the DLL ends, in UTF-16 units (default: the whole subject)
  match_case    true: 英大文字と小文字を区別する ticked; default false, which gives the `i` modifier
  known_difference
                set when UME Editor differs from the DLL on purpose; the text says why. The Rust
                test then checks that the two still differ.
"""
import json
import os

cases = []


def add(id, kind, pattern, subject, replacement=None, start=0, end=None, match_case=True, known=None):
    assert all(c["id"] != id for c in cases), id
    c = {"id": id, "kind": kind, "pattern": pattern}
    if replacement is not None:
        c["replacement"] = replacement
    c["subject"] = subject
    if start:
        c["start"] = start
    if end is not None:
        c["end"] = end
    if match_case is not True:
        c["match_case"] = match_case
    if known:
        c["known_difference"] = known
    cases.append(c)


def match(id, pattern, subject, **kw):
    add(id, "match", pattern, subject, **kw)


def rep(id, pattern, replacement, subject, **kw):
    add(id, "replace", pattern, subject, replacement=replacement, **kw)


def rep_all(id, pattern, replacement, subject, **kw):
    add(id, "replace_all", pattern, subject, replacement=replacement, **kw)


# --- 1. Replacement syntax ---------------------------------------------------------------------

# Unresolved references.
rep("ref-group-missing-dollar", "(a)", "[$2]", "a")
rep("ref-group-missing-brace", "(a)", "[${2}]", "a")
rep("ref-group-missing-backslash", "(a)", r"[\2]", "a")
rep("ref-group-missing-nine", "(a)", "[$9]", "a")
rep("ref-group-zero-brace", "(a)", "[${0}]", "a")
rep("ref-group-big-number", "(a)", "[${99999999999999999999}]", "a")
rep("ref-name-unknown-brace", "(a)", "[${nm}]", "a")
rep("ref-name-unknown-plus", "(a)", "[$+{nm}]", "a")
rep("ref-name-unknown-k-angle", "(a)", "[\\k<nm>]", "a")
rep("ref-name-unknown-k-quote", "(a)", "[\\k'nm']", "a")
rep("ref-name-unknown-minus", "(a)", "[$-{nm}[0]]", "a")
rep("ref-name-known-brace", "(?<n>a)", "[${n}]", "a")
rep("ref-name-known-plus", "(?<n>a)", "[$+{n}]", "a")
rep("ref-name-known-k-angle", "(?<n>a)", "[\\k<n>]", "a")
rep("ref-name-known-k-quote", "(?<n>a)", "[\\k'n']", "a")
rep("ref-name-known-minus", "(?<n>a)", "[$-{n}[0]]", "a")
rep("ref-name-python-style", "(?P<n>a)", "[${n}]", "a")
rep("ref-name-quote-style", "(?'n'a)", "[${n}]", "a")
rep("ref-name-case-differs", "(?<n>a)", "[${N}]", "a")
rep("ref-name-and-number", "(?<n>a)(b)", "[$1|$2]", "ab")
rep("ref-name-number-in-k", "(a)", "[\\k<1>]", "a")
rep("ref-name-number-in-brace-name", "(?<n>a)(b)", "[${1}${2}]", "ab")

# Malformed $ forms.
rep("dollar-brace-unclosed", "(a)", "[${1]", "a")
rep("dollar-brace-unclosed-end", "(a)", "${1", "a")
rep("dollar-brace-empty", "(a)", "[${}]", "a")
rep("dollar-lone-end", "(a)", "x$", "a")
rep("dollar-lone-space", "(a)", "x$ y", "a")
rep("dollar-lone-only", "(a)", "$", "a")
rep("dollar-brace-space-name", "(?<foo>a)", "[${foo bar}]", "a")
rep("dollar-brace-space-after", "(a)", "[${1 }]", "a")
rep("dollar-brace-space-before", "(a)", "[${ 1}]", "a")
rep("dollar-plus-brace-unclosed", "(?<x>a)", "[$+{x]", "a")
rep("dollar-plus-brace-empty", "(?<x>a)", "[$+{}]", "a")
rep("dollar-minus-no-index", "(?<x>a)", "[$-{x}]", "a")
rep("dollar-minus-unclosed-index", "(?<x>a)", "[$-{x}[0]", "a")
rep("dollar-minus-bad-index", "(?<x>a)", "[$-{x}[z]]", "a")
rep("dollar-minus-no-brace", "(?<x>a)", "[$-x]", "a")
rep("dollar-letter", "(a)", "[$a]", "a")
rep("dollar-dollar-one", "(a)", "[$$1]", "a")
rep("dollar-dollar", "(a)", "[$$]", "a")
rep("dollar-zero", "(a)", "[$0]", "a")
rep("dollar-zero-zero", "(a)", "[$00]", "a")
rep("dollar-amp", "(a)", "[$&]", "a")
rep("dollar-amp-twice", "(a)", "[$&$&]", "a")
rep("dollar-digits-then-digit", "(a)", "[$10]", "a")
rep("dollar-eleven-groups", "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)", "[$10|$11|$12|${1}0]", "abcdefghijk")
rep("dollar-backtick", "(a)b", "[$`]", "xab")
rep("dollar-quote", "a(b)", "[$']", "abx")
rep("dollar-underscore", "(a)", "[$_]", "a")
rep("dollar-fullwidth-digit", "(a)", "[$１]", "a")
rep("dollar-brace-fullwidth-digit", "(a)", "[${１}]", "a")
rep("dollar-brace-minus-one", "(a)", "[${-1}]", "a")
rep("dollar-brace-plus-one", "(a)", "[${+1}]", "a")

# $+ : the last group that took part.
rep("last-group-no-groups", "a", "[$+]", "a")
rep("last-group-one", "(a)", "[$+]", "a")
rep("last-group-alternation-second", "(a)|(b)", "[$+]", "b")
rep("last-group-alternation-first", "(a)|(b)", "[$+]", "a")
rep("last-group-optional-unused", "(a)(b)?", "[$+]", "a")
rep("last-group-optional-empty", "(a)(b?)", "[$+]", "a")
rep("last-group-highest-not-last", "(a)((b))", "[$+]", "ab")
rep("last-group-nested", "((a)(b))", "[$+]", "ab")
rep("last-group-brace", "(a)|(b)", "[${+}]", "b")

# Malformed backslash forms.
rep("bs-x-brace-unclosed", "(a)", r"[\x{41]", "a")
rep("bs-k-unclosed", "(?<x>a)", r"[\k<x]", "a")
rep("bs-k-bare", "(a)", r"[\k]", "a")
rep("bs-k-no-delimiter", "(a)", r"[\kx]", "a")
rep("bs-k-empty", "(a)", r"[\k<>]", "a")
rep("bs-o-bare", "(a)", r"[\o]", "a")
rep("bs-o-brace-unclosed", "(a)", r"[\o{101]", "a")
rep("bs-o-brace-empty", "(a)", r"[\o{}]", "a")
rep("bs-o-brace-bad-digit", "(a)", r"[\o{8}]", "a")
rep("bs-o-brace-big", "(a)", r"[\o{777}]", "a")
rep("bs-o-brace-ok", "(a)", r"[\o{101}]", "a")
rep("bs-o-brace-unicode", "(a)", r"[\o{3501}]", "a")
rep("bs-c-bare-bracket", "(a)", r"[\c]", "a")
rep("bs-c-end-text", "(a)", r"\c", "a")
rep("bs-ends-with-two", "(a)", "x\\\\", "a")
rep("bs-ends-with-single", "(a)", "x\\", "a")
rep("bs-ends-with-three", "(a)", "x\\\\\\", "a")
rep("bs-only-single", "(a)", "\\", "a")
rep("bs-dollar-escaped", "(a)", r"[\$1]", "a")
rep("bs-dollar-brace-escaped", "(a)", r"[\${1}]", "a")
rep("bs-backslash-then-dollar", "(a)", r"[\\$1]", "a")
rep("bs-slash", "(a)", r"[\/]", "a")
rep("bs-amp", "(a)", r"[\&]", "a")
rep("bs-space", "(a)", "[\\ ]", "a")
rep("bs-nonascii", "(a)", "[\\é]", "a")
rep("bs-fullwidth-t", "(a)", "[\\ｔ]", "a")

# Hex.
rep("hex-bare", "(a)", r"[\x]", "a")
rep("hex-bare-then-g", "(a)", r"[\xg]", "a")
rep("hex-one-digit", "(a)", r"[\x4]", "a")
rep("hex-one-digit-then-g", "(a)", r"[\x4g]", "a")
rep("hex-two-digits", "(a)", r"[\x41]", "a")
rep("hex-three-digits", "(a)", r"[\x414]", "a")
rep("hex-e9", "(a)", r"[\xE9]", "a")
rep("hex-ff", "(a)", r"[\xFF]", "a")
rep("hex-80", "(a)", r"[\x80]", "a")
rep("hex-7f", "(a)", r"[\x7F]", "a")
rep("hex-utf8-pair", "(a)", r"[\xC3\xA9]", "a")
rep("hex-lowercase-digits", "(a)", r"[\xe9]", "a")
rep("hex-brace-empty", "(a)", r"[\x{}]", "a")
rep("hex-brace-e9", "(a)", r"[\x{E9}]", "a")
rep("hex-brace-3042", "(a)", r"[\x{3042}]", "a")
rep("hex-brace-surrogate-high", "(a)", r"[\x{D800}]", "a")
rep("hex-brace-surrogate-low", "(a)", r"[\x{DFFF}]", "a")
rep("hex-brace-before-surrogates", "(a)", r"[\x{D7FF}]", "a")
rep("hex-brace-after-surrogates", "(a)", r"[\x{E000}]", "a")
rep("hex-brace-max", "(a)", r"[\x{10FFFF}]", "a")
rep("hex-brace-over-max", "(a)", r"[\x{110000}]", "a")
rep("hex-brace-way-over", "(a)", r"[\x{FFFFFFFFFFFF}]", "a")
rep("hex-brace-emoji", "(a)", r"[\x{1F600}]", "a")
rep("hex-brace-ffff", "(a)", r"[\x{FFFF}]", "a")
rep("hex-brace-zero", "(a)", r"[\x{0}]", "a")
rep("hex-brace-space-inside", "(a)", r"[\x{41 }]", "a")
rep("hex-brace-space-leading", "(a)", r"[\x{ 41}]", "a")
rep("hex-brace-bad-digit", "(a)", r"[\x{4G}]", "a")
rep("hex-brace-leading-zeros", "(a)", r"[\x{0000000041}]", "a")

# Octal and \digit.
rep("oct-zero", "(a)", r"[\0]", "a")
rep("oct-zero-twelve", "(a)", r"[\012]", "a")
rep("oct-zero-101", "(a)", r"[\0101]", "a")
rep("oct-zero-bad", "(a)", r"[\08]", "a")
rep("oct-101-no-group", "(a)", r"[\101]", "a")
rep("oct-18", "(a)", r"[\18]", "a")
rep("oct-108", "(a)", r"[\108]", "a")
rep("oct-377", "(a)", r"[\377]", "a")
rep("oct-400", "(a)", r"[\400]", "a")
nine = "(a)(b)(c)(d)(e)(f)(g)(h)(i)"
ten = nine + "(j)"
eleven = ten + "(k)"
for label, pat, subj in (("9", nine, "abcdefghi"), ("10", ten, "abcdefghij"), ("11", eleven, "abcdefghijk")):
    for r in (r"\9", r"\10", r"\11", r"\12", r"\19", r"\100", r"\101", r"\110"):
        rep("backref-%s-groups-%s" % (label, r.lstrip("\\")), pat, "[" + r + "]", subj)

# Escapes that are listed in bregonig.html and ones that are not.
for name, esc in (("t", r"\t"), ("n", r"\n"), ("r", r"\r"), ("f", r"\f"), ("e", r"\e"), ("a", r"\a"), ("b", r"\b"),
                  ("v", r"\v"), ("Q", r"\Q"), ("E", r"\E"), ("z", r"\z"), ("d", r"\d"), ("w", r"\w"),
                  ("s", r"\s"), ("N", r"\N"), ("R", r"\R"), ("h", r"\h"), ("G", r"\G"), ("A", r"\A"),
                  ("B", r"\B"), ("K", r"\K"), ("X", r"\X"), ("p", r"\p"), ("g", r"\g"), ("u-bare", r"\u"),
                  ("U-bare", r"\U")):
    rep("esc-" + name, "(a)", "[" + esc + "]", "a")
rep("esc-Q-text-E", "(a)", r"[\Qa$1\E]", "a")
rep("esc-only-n", "(a)", r"\n", "a")
rep("esc-only-t", "(a)", r"\t", "a")
rep("esc-only-r", "(a)", r"\r", "a")
rep("esc-n-twice", "(a)", r"\n\n", "a")
rep("esc-n-with-text", "(a)", r"x\ny", "a")
rep("esc-unicode-u", "(a)", "[\\u0041]", "a")

# \c.
rep("ctrl-A", "(a)", r"[\cA]", "a")
rep("ctrl-a-lower", "(a)", r"[\ca]", "a")
rep("ctrl-bracket", "(a)", r"[\c[]", "a")
rep("ctrl-question", "(a)", r"[\c?]", "a")
rep("ctrl-at", "(a)", r"[\c@]", "a")
rep("ctrl-digit", "(a)", r"[\c1]", "a")
rep("ctrl-backslash-x", "(a)", "[\\c\\A]", "a")
rep("ctrl-nonascii-e-acute", "(a)", "[\\cé]", "a")
rep("ctrl-nonascii-hiragana", "(a)", "[\\cあ]", "a")
rep("ctrl-nonascii-fullwidth-a", "(a)", "[\\cＡ]", "a")
rep("ctrl-space", "(a)", "[\\c ]", "a")
rep("ctrl-dollar", "(a)", r"[\c$1]", "a")
rep("ctrl-end-in-brackets", "(a)", r"[\c", "a")

# Case conversion.
rep("case-upper-ascii", "(.+)", r"\U$1", "abc")
rep("case-lower-ascii", "(.+)", r"\L$1", "ABC")
rep("case-upper-e-acute", "(.+)", r"\U$1", "é")
rep("case-lower-e-acute", "(.+)", r"\L$1", "É")
rep("case-upper-fullwidth", "(.+)", r"\U$1", "ｆｇ")
rep("case-lower-fullwidth", "(.+)", r"\L$1", "ＦＧ")
rep("case-upper-greek", "(.+)", r"\U$1", "αβγ")
rep("case-lower-greek", "(.+)", r"\L$1", "ΑΒΓ")
rep("case-upper-cyrillic", "(.+)", r"\U$1", "дж")
rep("case-lower-cyrillic", "(.+)", r"\L$1", "ДЖ")
rep("case-upper-sharp-s", "(.+)", r"\U$1", "ß")
rep("case-upper-dotless-i", "(.+)", r"\U$1", "ı")
rep("case-upper-long-s", "(.+)", r"\U$1", "ſ")
rep("case-lower-kelvin", "(.+)", r"\L$1", "\u212a")
rep("case-upper-mixed", "(.+)", r"\U$1", "aéb")
rep("case-upper-literal-nonascii", "(a)", "\\Uéｆ", "a")
rep("case-first-upper-nonascii", "(.+)", r"\u$1", "éa")
rep("case-first-lower-nonascii", "(.+)", r"\l$1", "Éa")
rep("case-first-upper-ascii", "(.+)", r"\u$1", "abc")
rep("case-first-lower-ascii", "(.+)", r"\l$1", "ABC")
rep("case-upper-then-end", "(.+)", r"\U$1\Ex", "abc")
rep("case-upper-then-end-twice", "(.+)", r"\U$1\E\Ex", "abc")
rep("case-end-alone", "(.+)", r"\E$1", "abc")
rep("case-upper-nested-lower", "(.+)", r"\U$1\L$1\Ex", "abc")
rep("case-lower-nested-upper", "(.+)", r"\L$1\U$1\Ex", "ABC")
rep("case-first-upper-in-upper-mode", "(.+)", r"\L\u$1", "ABC")
rep("case-first-lower-in-lower-mode", "(.+)", r"\U\l$1", "abc")
rep("case-first-upper-then-lower-mode", "(.+)", r"\u\L$1", "ABC")
rep("case-first-lower-then-upper-mode", "(.+)", r"\l\U$1", "abc")
rep("case-first-upper-twice", "(.+)", r"\u\u$1", "abc")
rep("case-first-upper-lower", "(.+)", r"\u\l$1", "abc")
rep("case-first-upper-end", "(.+)", r"\u\E$1", "abc")
rep("case-first-upper-at-end", "(.+)", r"$1\u", "abc")
rep("case-upper-at-end", "(.+)", r"$1\U", "abc")
rep("case-first-upper-empty-group-then-text", "(x?)abc", r"\u$1def", "abc")
rep("case-first-lower-empty-group-then-text", "(x?)abc", r"\l$1DEF", "abc")
rep("case-first-upper-empty-group-then-group", "(x?)(abc)", r"\u$1$2", "abc")
rep("case-first-upper-missing-group-then-text", "(x)?abc", r"\u$1def", "abc")
rep("case-first-upper-unresolved-then-text", "abc", r"\u$9def", "abc")
rep("case-first-upper-empty-text", "abc", r"\u\Edef", "abc")
rep("case-first-upper-dollar-amp", "abc", r"\u$&", "abc")
rep("case-first-upper-escape", "abc", r"\u\x61", "abc")
rep("case-first-upper-newline-then-text", "abc", r"\u\nabc", "abc")
rep("case-upper-escape-hex", "abc", r"\U\x{e9}\x61", "abc")
rep("case-upper-over-newline-and-group", "(b)", "\\Ua$1\\nc$1", "b")
rep("case-first-upper-surrogate-pair", "(.+)", r"\u$1", "\U0001F600a")
rep("case-upper-deseret", "(.+)", r"\U$1", "\U00010428")
rep("case-upper-astral-and-ascii", "(.+)", r"\U$1", "\U00010428a")

# Duplicate names.
dup = "(?<n>a)|(?<n>b)"
for subj in ("a", "b"):
    for label, r in (("plus", "$+{n}"), ("brace", "${n}"), ("k", r"\k<n>"), ("minus0", "$-{n}[0]"),
                     ("minus1", "$-{n}[1]"), ("minus2", "$-{n}[2]"), ("minus-1", "$-{n}[-1]"),
                     ("minus-2", "$-{n}[-2]"), ("minus-3", "$-{n}[-3]"), ("minus99", "$-{n}[99]")):
        rep("dupname-alt-%s-%s" % (label, subj), dup, "[" + r + "]", subj)
for label, r in (("plus", "$+{n}"), ("brace", "${n}"), ("k", r"\k<n>"), ("minus0", "$-{n}[0]"),
                 ("minus1", "$-{n}[1]"), ("minus2", "$-{n}[2]"), ("minus-1", "$-{n}[-1]"),
                 ("minus-2", "$-{n}[-2]"), ("minus-3", "$-{n}[-3]")):
    rep("dupname-seq-%s" % label, "(?<n>a)(?<n>b)", "[" + r + "]", "ab")
    rep("dupname-seq-empty-%s" % label, "(?<n>a)(?<n>b?)", "[" + r + "]", "a")
rep("dupname-optional-first-unused", "(?<n>a)?(?<n>b)", "[${n}]", "b")
rep("dupname-number-after", "(?<n>a)(?<n>b)(c)", "[$3|$2|$1]", "abc")
rep("minus-single-0", "(?<n>a)", "[$-{n}[0]]", "a")
rep("minus-single-1", "(?<n>a)", "[$-{n}[1]]", "a")
rep("minus-single--1", "(?<n>a)", "[$-{n}[-1]]", "a")
rep("minus-single--2", "(?<n>a)", "[$-{n}[-2]]", "a")
rep("minus-single-01", "(?<n>a)", "[$-{n}[01]]", "a")
rep("minus-single-plus0", "(?<n>a)", "[$-{n}[+0]]", "a")
rep("minus-single-space", "(?<n>a)", "[$-{n}[ 0]]", "a")
rep("minus-unmatched-group", "(?<n>a)|(?<n>b)", "[$-{n}[0]|$-{n}[1]]", "b")

# Named groups and plain groups together (ONIG_OPTION_CAPTURE_GROUP in the Perl syntax).
rep("named-plain-numbering", "(?<n>a)(b)", "[$1|$2]", "ab")
rep("named-plain-mixed-order", "(a)(?<n>b)(c)", "[$1|$2|$3]", "abc")
rep("named-group-count", "(?<n>a)(?<m>b)", "[$1|$2|$3]", "ab")
rep("named-reuse-in-pattern", "(?<q>['\"]).*?\\k<q>", "[$&|${q}]", "'x'")

# --- Replace (the first match, from `start`) ------------------------------------------------

rep("start-prefix-kept", "(a)", "<$1>", "abab", start=2)
rep("start-after-match", "a", "X", "abab", start=3)
rep("start-at-end", "a*", "X", "abab", start=4)
rep("start-no-match", "z", "X", "abab", start=1)
rep("start-lookbehind-sees-before", "(?<=a)b", "X", "abab", start=1)
rep("start-caret-not-at-start", "^a", "X", "aab", start=1)
rep("start-G-at-start", r"\Ga", "X", "aab", start=1)
rep("start-b-at-start", r"\bb", "X", "ab", start=1)
rep("empty-match-replace", "x*", "-", "abc")
rep("empty-match-replace-at-end", "x*", "-", "abc", start=3)
rep("empty-subject-empty-match", "x*", "-", "")
rep("empty-subject-caret", "^", "-", "")
rep("empty-subject-dollar", "$", "-", "")
rep("empty-subject-no-match", "a", "-", "")
rep("no-match-out", "z", "X", "abc")
rep("replacement-empty", "b", "", "abc")
rep("replacement-empty-whole-line", "abc", "", "abc")
rep("replacement-nonbmp", "a", "😀", "abc")
rep("replacement-fullwidth", "(a)", "（$1）", "abc")
rep("subject-astral-offsets", r"(.)(b)", "[$1|$2]", "😀b")
rep("subject-astral-dot-count", r"^.$", "X", "😀")
rep("subject-astral-start-after", "b", "X", "😀b", start=2)
rep("subject-combining", r"(e)", "[$1]", "éx")

# --- 2. Replace all with g -------------------------------------------------------------------

rep_all("g-lookbehind-aaa", "(?<=a)a", "X", "aaa")
rep_all("g-lookbehind-aaaa", "(?<=a)a", "X", "aaaa")
rep_all("g-lookbehind-captured-text", "(?<=(a))a", "[$1]", "aaa")
rep_all("g-lookahead-aaa", "a(?=a)", "X", "aaa")
rep_all("g-empty-star", "x*", "-", "abc")
rep_all("g-empty-star-empty-subject", "x*", "-", "")
rep_all("g-empty-star-with-match", "x*", "-", "axxb")
rep_all("g-empty-star-a", "a*", "-", "baac")
rep_all("g-empty-star-a-end", "a*", "-", "baa")
rep_all("g-empty-plus-or-empty", "a|", "-", "bab")
rep_all("g-caret", "^", ">", "abc")
rep_all("g-dollar", "$", "<", "abc")
rep_all("g-caret-dollar", "^|$", "|", "abc")
rep_all("g-caret-dollar-empty", "^|$", "|", "")
rep_all("g-caret-star", "^a*", "-", "aab")
rep_all("g-word-boundary", r"\b", "|", "ab cd")
rep_all("g-non-word-boundary", r"\B", "|", "ab cd")
rep_all("g-G-empty", r"\G", "-", "abc")
rep_all("g-G-star", r"\Gx*", "-", "abc")
rep_all("g-G-char", r"\G.", "[$&]", "abc")
rep_all("g-G-a", r"\Ga", "X", "aab")
rep_all("g-G-a-after-gap", r"\Ga", "X", "abaa")
rep_all("g-G-a-or-empty", r"\Ga|\G", "X", "aab")
rep_all("g-G-b-star", r"\Gb*", "-", "bbac")
rep_all("g-capture-line-start", r"^(.)", "[$1]", "abc")
rep_all("g-capture-line-end", r"(.)$", "[$1]", "abc")
rep_all("g-capture-start-and-end", r"^(a)|(c)$", "[$1$2]", "abc")
rep_all("g-capture-start-and-end-one-char", r"^(a)|(a)$", "[$1$2]", "a")
rep_all("g-captures-each", r"(\w)(\w)", "$2$1", "abcde")
rep_all("g-captures-global-reset", r"(a)|(b)", "[$1|$2]", "ab")
rep_all("g-case-conversion", r"(\w+)", r"\u$1", "hello world")
rep_all("g-multiple-matches", "a", "bb", "aaa")
rep_all("g-no-match", "z", "X", "abc")
rep_all("g-nonoverlap", "aa", "X", "aaaaa")
rep_all("g-newline-in-replacement", ",", r"\n", "a,b")
rep_all("g-dot-star", ".*", "[$&]", "abc")
rep_all("g-dot-star-lazy", ".*?", "-", "ab")
rep_all("g-dot-star-then-empty", ".*", "-", "")
rep_all("g-alt-empty-second", "b|", "-", "abc")
rep_all("g-surrogate-empty", "x*", "-", "😀")
rep_all("g-surrogate-dot", ".", "[$&]", "😀a")
rep_all("g-lookbehind-word", r"(?<=\w)(?=\w)", "-", "abc")
rep_all("g-keep-K", r"a\Kb", "X", "abab")
rep_all("g-keep-K-empty", r"a\K", "X", "aa")
rep_all("g-lookbehind-replaced-text", r"(?<!X)a", "X", "aaa")
rep_all("g-backref-in-pattern", r"(a)\1", "[$1]", "aaaa")

# --- 3. Syntax in the Unicode DLL -----------------------------------------------------------

match("syn-absent-operator", "(?~ab)", "xabc")
match("syn-absent-operator-c", "(?~ab)c", "xabc")
match("syn-absent-operator-whole", "a(?~bc)", "abcd")
match("syn-recursion-name", r"(?<p>\((?:[^()]|\g<p>)*\))", "x(a(b))y")
match("syn-recursion-number", r"(\((?:[^()]|\g<1>)*\))", "x(a(b))y")
match("syn-recursion-whole", r"a\g<0>?b", "aabb")
match("syn-recursion-relative", r"(\((?:[^()]|\g<-1>)*\))", "x(a(b))y")
match("syn-recursion-undefined-name", r"\g<nope>", "a")
match("syn-set-and", "[a-z&&[^aeiou]]+", "aeblcdoz")
match("syn-set-and-class", r"[\w&&[^\d]]+", "12ab34")
match("syn-set-nested", "[a-c[x-z]]+", "qazbq")
match("syn-lookbehind-alt", "(?<=a|bc)d", "bcd")
match("syn-lookbehind-alt-short", "(?<=a|bc)d", "ad")
match("syn-lookbehind-alt-none", "(?<=a|bc)d", "xd")
match("syn-neg-lookbehind-alt", "(?<!a|bc)d", "bcd")
match("syn-neg-lookbehind-alt-hit", "(?<!a|bc)d", "xd")
match("syn-lookbehind-quantifier", "(?<=a+)b", "aab")
match("syn-lookbehind-nested-alt", "(?<=(?:a|bc)d)e", "bcde")
match("syn-default-ignore-case", "abc", "xABCx", match_case=False)
match("syn-match-case", "abc", "xABCx", match_case=True)
match("syn-ignore-case-nonascii", "é", "É", match_case=False)
match("syn-ignore-case-greek", "σ", "Σς", match_case=False)
match("syn-ignore-case-sharp-s", "ss", "ß", match_case=False)
match("syn-ignore-case-sharp-s-rev", "ß", "SS", match_case=False)
match("syn-ignore-case-fullwidth", "ａ", "Ａ", match_case=False)
match("syn-ignore-case-kelvin", "k", "\u212a", match_case=False)
match("syn-ignore-case-class", "[a-c]+", "ABC", match_case=False)
match("syn-ignore-case-backref", r"(a)\1", "aA", match_case=False)
match("syn-ignore-case-off-inline", "(?-i)abc", "ABC", match_case=False)
match("syn-ignore-case-on-inline", "(?i)abc", "ABC", match_case=True)
match("syn-digit-fullwidth", r"\d+", "１２３")
match("syn-digit-arabic-indic", r"\d+", "٣٤")
match("syn-word-unicode", r"\w+", "éあ_1")
match("syn-space-unicode", r"\s+", "a　 b")
match("syn-posix-alpha", "[[:alpha:]]+", "1éa2")
match("syn-posix-digit", "[[:digit:]]+", "a１２b")
match("syn-posix-punct", "[[:punct:]]+", "a!?b")
match("syn-posix-negated", "[[:^alpha:]]+", "ab12cd")
match("syn-prop-han", r"\p{Han}+", "aあ漢字b")
match("syn-prop-hiragana", r"\p{Hiragana}+", "aあい漢")
match("syn-prop-katakana", r"\p{Katakana}+", "aアイ漢")
match("syn-prop-negated", r"\p{^Alpha}+", "ab12cd")
match("syn-prop-alpha-upper", r"\p{Lu}+", "abCDef")
match("syn-hex-digit-class", r"\h+", "xyzAF09g")
match("syn-linebreak-R", r"a\Rb", "a\r\nb")
match("syn-grapheme-X", r"\X", "éx")
match("syn-keep-K", r"a\Kb", "ab")
match("syn-possessive", "a++a", "aaa")
match("syn-atomic", "(?>a+)a", "aaa")
match("syn-lazy", "a+?", "aaa")
match("syn-conditional", "(a)?(?(1)b|c)", "ab")
match("syn-conditional-else", "(a)?(?(1)b|c)", "c")
match("syn-named-backref", r"(?<n>a)\k<n>", "xaa")
match("syn-named-backref-python", "(?P<n>a)(?P=n)", "aa")
match("syn-named-python-group", "(?P<n>a)", "a")
match("syn-subexp-call-python-p", "(?P<n>a)(?P>n)", "aa")
match("syn-inline-comment", "a(?#comment)b", "ab")
match("syn-inline-flags-scope", "(?i:a)b", "Ab")
match("syn-inline-flag-x", "(?x: a b )", "ab")
match("syn-inline-flag-s", "(?s:a.b)", "a\nb")
match("syn-inline-flag-m-off", "(?-m:^b)", "a\nb")
match("syn-quantifier-open-lower", "a{,3}", "aaaa")
match("syn-quantifier-literal-brace", "a{", "a{")
match("syn-quantifier-literal-brace-2", "a{x}", "a{x}")
match("syn-quantifier-literal-brace-3", "a{1,x}", "a{1,x}")
match("syn-quantifier-exact", "a{2}", "aaa")
match("syn-quantifier-possessive-brace", "a{1,2}+", "aaa")
match("syn-quoted", r"\Qa.b\E+", "a.bbb")
match("syn-quoted-unclosed", r"\Qa.b", "xa.b")
match("syn-A-z", r"\Aa\z", "a")
match("syn-Z-before-end", r"a\Z", "a")
match("syn-caret-multiline-start", "^a", "a")
match("syn-dollar-end", "a$", "a")
match("syn-dot-nonbmp", r"^.$", "😀")
match("syn-class-nonbmp", "[😀]", "x😀")
match("syn-class-range-nonbmp", "[😀-😂]+", "x😁😂")
match("syn-hex-pattern-nonbmp", r"\x{1F600}", "x😀")
match("syn-hex-pattern-bmp", r"\x{3042}", "あ")
match("syn-hex-pattern-two-digits", r"\xe9", "é")
match("syn-hex-pattern-surrogate", r"\x{D800}", "a")
match("syn-hex-pattern-over-max", r"\x{110000}", "a")
match("syn-octal-pattern", r"\101", "A")
match("syn-control-pattern", r"\cA", "\x01")
match("syn-unicode-escape-u", "\\u0041", "A")
match("syn-empty-pattern", "", "abc")
match("syn-empty-pattern-start", "", "abc", start=2)
match("syn-empty-pattern-start-end", "", "abc", start=3)
match("syn-star-only", "*", "a")
match("syn-plus-only", "+", "a")
match("syn-question-only", "?", "a")
match("syn-open-paren", "(", "a")
match("syn-close-paren", ")", "a")
match("syn-open-bracket", "[", "a")
match("syn-close-bracket", "]", "]")
match("syn-empty-class", "[]", "a")
match("syn-bad-range", "[z-a]", "a")
match("syn-bad-backref", r"\1", "a")
match("syn-bad-name", "(?<1a>x)", "x")
match("syn-bad-group-option", "(?z)", "a")
match("syn-lookbehind-unbounded", "(?<=a*)b", "ab")
match("syn-nested-quantifier", "(a*)*", "aa")
match("syn-nested-quantifier-plus", "(a*)+", "aa")
match("syn-possessive-star", "a*+", "aa")
match("syn-trailing-backslash", "a\\", "a")
match("syn-trailing-two-backslashes", "a\\\\", "a\\")
match("syn-trailing-three-backslashes", "a\\\\\\", "a\\")
match("syn-trailing-backslash-in-class", "[a\\", "a")
match("syn-backslash-only", "\\", "a")
match("syn-slash", "a/b", "a/b")
match("syn-escaped-slash", r"a\/b", "a/b")
match("syn-ffff-in-pattern", "a￿b", "a￿b")
match("syn-ffff-in-subject", "a.b", "a￿b")
match("syn-ffff-in-class", "[￿]", "a￿")
match("syn-extended-hash", "a#b", "a#b")

# Match positions and captures.
match("pos-captures-all", "(a)(b)?(c)", "xac")
match("pos-nested-captures", "((a)(b))", "ab")
match("pos-optional-group-in-alt", "(a)|(b)", "b")
match("pos-captures-repeat", "(a|b)+", "abab")
match("pos-empty-capture", "(x*)a", "a")
match("pos-capture-astral", "(.)(.)", "😀a")
match("pos-start-skips", "a", "aba", start=1)
match("pos-start-at-end", "a", "aba", start=3)
match("pos-start-empty-pattern-at-end", "$", "aba", start=3)
match("pos-start-lookbehind", "(?<=b)a", "aba", start=2)
match("pos-start-caret-line-start-only", "^a", "aba", start=2)
match("pos-start-caret-at-zero", "^a", "aba", start=0)
match("pos-start-G", r"\Ga", "baa", start=1)
match("pos-start-G-miss", r"\Ga", "bba", start=1)
match("pos-start-word-boundary", r"\bb", "ab", start=1)
match("pos-start-past-end-by-one", "a", "ab", start=3)
match("pos-end-limits-lookahead", "a(?=b)", "ab", end=1,
      known="UME Editor looks at the whole line and only checks that the match lies in the range (PR #38, section 1), so a look-ahead sees past the selection end")
match("pos-end-limits-match", "ab", "abab", start=0, end=3)
match("pos-end-limits-dollar", "b$", "abc", end=2,
      known="UME Editor looks at the whole line and only checks that the match lies in the range (PR #38, section 1), so the end of the range is not the end of the line")
match("pos-end-limits-word-boundary", r"a\b", "ab", end=1,
      known="UME Editor looks at the whole line and only checks that the match lies in the range (PR #38, section 1), so a word boundary sees the character after the range")

# Line endings. UME Editor matches a line without its ending (ADR-0006, PR #38 section 1).
match("eol-dot-star-stops", ".*", "ab\r\ncd")
match("eol-dollar-before-crlf", "b$", "ab\r\ncd")
match("eol-dollar-before-lf", "b$", "ab\ncd")
match("eol-dollar-before-cr", "b$", "ab\rcd")
match("eol-space-class-reaches", r"b\s", "ab\r\n",
      known="UME Editor matches the line without its ending, so `\\s` cannot match the line ending")
match("eol-negated-class-reaches", "b[^a]", "ab\r\n",
      known="UME Editor matches the line without its ending, so a negated class cannot match the line ending")
match("eol-cr-in-pattern", r"b\r", "ab\r\n",
      known="UME Editor matches the line without its ending, so `\\r` cannot match the line ending")
match("eol-lf-in-pattern", r"b\r\n", "ab\r\n",
      known="UME Editor matches the line without its ending, so `\\r\\n` cannot match the line ending")
match("eol-dot-star-dollar", ".*$", "ab\r\n")
match("eol-W-reaches", r"b\W", "ab\r\n",
      known="UME Editor matches the line without its ending, so `\\W` cannot match the line ending")
match("eol-subject-with-only-eol", "^$", "\r\n")
rep_all("eol-g-dollar", "$", "<", "ab\r\n")
rep_all("eol-g-caret", "^", ">", "ab\r\n")
rep_all("eol-g-space-class", r"\s+", "_", "a b\r\n",
        known="UME Editor replaces within the line without its ending, so `\\s+` cannot take the line ending")

# Option letters that change the pattern (Sakura passes them only from its own checkboxes; the
# regex dialog has none of them, so this is a check on the base case only).
match("opt-m-always-caret", "^b", "a\nb")
match("opt-m-always-dollar", "a$", "a\nb")
match("opt-no-s-dot", "a.b", "a\nb")
match("opt-no-x-space", "a b", "a b")

# --- Probes added after the first measurement: how far the DLL's readings of `${...}`, `$-{..}[i]`,
# `\x{...}`, `\NNN` and the end of the text reach.
PROBES = (
    ("[${foo", "(?<foo>a)"), ("[${a b", "(a)"), ("[${a}b}]", "(?<a>a)"), ("[${a$b}]", "(a)"),
    ("[${1a}]", "(a)"), ("[${12}]", "(a)"), ("[${1}2]", "(a)"), ("[${ }]", "(a)"), ("[${01}]", "(a)"),
    ("[${1x", "(a)"), ("[${", "(a)"), ("[${}", "(a)"), ("[${a{b}]", "(a)"),
    ("[$+{a b}]", "(a)"), ("[$+{ }]", "(a)"), ("[$+{1}]", "(a)"), ("[$+{0}]", "(a)"),
    ("[$-{ }[0]]", "(a)"), ("[$-{n x}[0]]", "(?<n>a)"), ("[$-{n}[0 ]]", "(?<n>a)"),
    ("[$-{n}[1x]]", "(?<n>a)"), ("[$-{n}[]]", "(?<n>a)"), ("[$-{n}[ -1]]", "(?<n>a)"),
    ("[$-{n}[0", "(?<n>a)"), ("[$-{n}[0]]", "(?<n>a)"), ("[$-{1}[0]]", "(a)"), ("[$-{}[0]]", "(a)"),
    ("[$-{n}[0]x[-1]]", "(?<n>a)"), ("[$-{n}[ +0 ]]", "(?<n>a)"), ("[$-{n}[--1]]", "(?<n>a)"),
    ("[\\k<a b>]", "(a)"), ("[\\k<a>b>]", "(?<a>a)"), ("[\\k'a b']", "(a)"), ("[\\k<n']", "(?<n>a)"),
    ("[\\k<n'>]", "(?<n>a)"), ("[\\k<-1>]", "(a)"), ("[\\k<0>]", "(a)"), ("[\\k'", "(a)"),
    ("[\\k<", "(a)"), ("[\\g<n>]", "(?<n>a)"), ("[\\k{n}]", "(?<n>a)"), ("[\\k<n>>]", "(?<n>a)"),
    ("[$$$1]", "(a)"), ("[$$$$1]", "(a)"), ("[$$&]", "(a)"), ("[$$$&]", "(a)"), ("[$$+]", "(a)"),
    ("[$$ 1]", "(a)"), ("[$\\n]", "(a)"), ("[$\\U1]", "(a)"), ("[$\\\\1]", "(a)"), ("[$${1}]", "(a)"),
    ("[$ $1]", "(a)"), ("[$x$1]", "(a)"), ("[$$a$1]", "(a)"), ("[$\\$1]", "(a)"), ("[$-$1]", "(a)"),
    ("[$+$1]", "(a)"), ("[$&$1]", "(a)"), ("[$$$$]", "(a)"),
    ("[\\x{00000041}]", "(a)"), ("[\\x{000000041}]", "(a)"), ("[\\x{FFFFFFFF}]", "(a)"),
    ("[\\x{7FFFFFFF}]", "(a)"), ("[\\x{80000000}]", "(a)"), ("[\\x{1FFFFF}]", "(a)"),
    ("[\\x{200000}]", "(a)"), ("[\\x{10000}]", "(a)"), ("[\\x{FFFF1}]", "(a)"), ("[\\x{4}]", "(a)"),
    ("[\\x{ }]", "(a)"), ("[\\x{-1}]", "(a)"), ("[\\x{41}2]", "(a)"), ("[\\x{e9}]", "(a)"),
    ("[\\o{0000000101}]", "(a)"), ("[\\o{00101}]", "(a)"), ("[\\o{37777777777}]", "(a)"),
    ("[\\o{4100}]", "(a)"), ("[\\o{1000000}]", "(a)"), ("[\\o{ }]", "(a)"), ("[\\o{1 }]", "(a)"),
    ("[\\1000]", "(a)"), ("[\\0777]", "(a)"), ("[\\00]", "(a)"), ("[\\000]", "(a)"), ("[\\0001]", "(a)"),
    ("[\\9]", "(a)"), ("[\\81]", "(a)"), ("[\\8]", "(a)"), ("[\\1]", "(a)"), ("[\\1a]", "(a)"),
    ("[\\01]", "(a)"), ("[\\07]", "(a)"), ("[\\7]", "(a)"), ("[\\77]", "(a)"), ("[\\777]", "(a)"),
    ("[\\378]", "(a)"), ("[\\1111]", "(a)"),
    ("abc\\c", "(a)"), ("abc\\x", "(a)"), ("abc\\o", "(a)"), ("abc\\k", "(a)"), ("abc\\u", "(a)"),
    ("abc\\L", "(a)"), ("abc\\U", "(a)"), ("abc\\E", "(a)"), ("abc\\l", "(a)"), ("abc\\0", "(a)"),
    ("abc\\1", "(a)"), ("abc\\x4", "(a)"), ("abc\\x{", "(a)"), ("abc\\k<", "(a)"), ("abc$", "(a)"),
    ("abc$+", "(a)"), ("abc${", "(a)"), ("abc$-", "(a)"), ("abc$&", "(a)"), ("abc\\c\\\\", "(a)"),
    ("\\c\\\\", "(a)"), ("[\\c\\\\]", "(a)"), ("[\\c\\\\\\\\]", "(a)"), ("[\\c\\n]", "(a)"),
    ("[\\c\\\\A]", "(a)"), ("[\\c\\c]", "(a)"), ("[\\c\\ca]", "(a)"),
    ("[\\U\\Q$1\\E]", "(a)"), ("[\\Q\\Q]", "(a)"), ("[\\Ec]", "(a)"), ("[\\Lx\\Uy\\Ez]", "(a)"),
    ("[\\u\\Lx]", "(a)"),
)
for i, (r, pat) in enumerate(PROBES):
    rep("probe-%03d" % i, pat, r, "a")

# Probes of the index of `$-{name}[i]` and of a search whose start equals its end.
for i, r in enumerate(("[$-{n}[\t0]]", "[$-{n}[ ]]", "[$-{n}[+]]", "[$-{n}[-]]", "[$-{n}[ 0]]",
                       "[$-{n}[+ 0]]", "[$-{n}[- 1]]", "[$-{n}[٠]]", "[$-{n}[0\t]]", "[$-{n}[1]$-{n}[0]]")):
    rep("probe-index-%02d" % i, "(?<n>a)|(?<n>b)", r, "b")
match("probe-start-equals-end-inside", "b", "ab", start=1, end=1)
match("probe-start-equals-end-empty-pattern", "", "ab", start=1, end=1)
rep("probe-replace-start-equals-end", "b", "X", "ab", start=1, end=1)

# Pattern-side escapes that depend on the encoding of the engine (UTF-16 in the DLL).
match("probe-pat-x41", r"\x41", "A")
match("probe-pat-x-brace-41", r"\x{41}", "A")
match("probe-pat-x-brace-e9", r"\x{E9}", "é")
match("probe-pat-octal-0", r"\0", "\x00")
match("probe-pat-octal-o-brace", r"\o{101}", "A")
match("probe-pat-x-ffff", r"\x{FFFF}", "￿")
match("probe-pat-surrogate-class", "[\U0001F600]", "\U0001F600")

# Line endings, now that the harness rewrites `.` and `$` as Sakura does.
match("eol-dot-before-cr", "a.b", "a\rb")
match("eol-dot-before-lf", "a.b", "a\nb")
match("eol-dollar-with-capture", "(b)$", "ab\r\n")
match("eol-dollar-middle", "b$\\s", "ab\r\ncd")
match("eol-caret-after-cr", "^c", "ab\r\ncd")
match("eol-caret-after-lf", "^c", "ab\ncd")
match("eol-class-negated-reaches-cr", "b[^x]", "ab\r\n")
match("eol-word-boundary-before-eol", r"b\b", "ab\r\n")
match("eol-lookahead-eol", r"b(?=\r)", "ab\r\n")
match("eol-lookahead-dollar-eol", r"b(?=$)", "ab\r\n")
match("eol-dollar-only", "$", "ab\r\n")
match("eol-dollar-only-lf", "$", "ab\n")
match("eol-dollar-only-start-at-eol", "$", "ab\r\n", start=2)
match("eol-caret-dollar-empty-line", "^$", "\r\n")
match("eol-caret-dollar-empty-line-lf", "^$", "\n")
match("eol-dot-star-dollar-lf", ".*$", "ab\n")
rep_all("eol-g-dot-star", ".*", "[$&]", "ab\r\n")
rep_all("eol-g-newline-class", r"[\r\n]+", "_", "ab\r\n")
rep_all("eol-g-caret-twice", "^", ">", "ab\r\n")
rep_all("eol-g-dollar-lf", "$", "<", "ab\n")
rep("eol-replace-newline-escape", "b", r"\r\n", "ab")

# The scan of `g` after an empty match, more cases for the pinned \G.
rep_all("g-G-after-empty-plain", r"\G|b", "X", "abc")
rep_all("g-G-b-after-empty", r"\Gb|x*", "-", "bbac")
rep_all("g-G-alt-class", r"\G[ab]", "X", "abac")
rep_all("g-empty-then-nonempty", r"x*|a", "-", "ba")
rep_all("g-nonempty-then-empty-same-place", r"a|x*", "-", "aab")
rep_all("g-nonempty-then-empty-after", r"a+|x*", "-", "aab")
rep_all("g-lazy-empty", r"a*?", "-", "aa")
rep_all("g-caret-g-mid", r"^a", "X", "aaa")
rep_all("g-lookbehind-empty", r"(?<=a)", "-", "aa")
rep_all("g-lookahead-empty", r"(?=a)", "-", "aa")
rep_all("g-b-empty-in-words", r"\b", "|", "ab, cd")

# --- Known differences: where UME Editor does not do what the DLL does, on purpose or because it
# cannot. The reason is stored with the case. The Rust test checks that the two still differ.
LONE_SURROGATE = (
    "bregonig.dll gives a lone UTF-16 surrogate (a value in D800-DFFF, or past U+10FFFF turned into "
    "surrogates by a formula that does not fit); a Rust string cannot hold one, so UME Editor "
    "gives nothing"
)
EMPTY_TARGET = (
    "bregonig.dll refuses a search that starts at the end of the text (\"invalid target parameter\"). "
    "In Sakura every line but the last has its line ending, so the end of the text is the end of the "
    "last line only. UME Editor matches lines without their endings and finds an empty match at the "
    "end of any line (ADR-0006, PR #38 section 1)"
)
MULTI_LINE = (
    "UME Editor matches one line at a time, without its line ending (ADR-0006, PR #38 section 1); "
    "this subject has a line ending in the middle or the pattern reaches over the ending"
)
BYTE_ESCAPE = (
    "bregonig.dll runs Onigmo on UTF-16, where a byte escape (`\\xHH`, `\\NNN`, `\\0`) is half a "
    "character and is refused (\"too short multibyte code string\"); UME Editor runs Onigmo on UTF-8, "
    "where the same byte escape is a valid ASCII character"
)
KNOWN = {
    "ref-group-big-number": (
        "a group number of 20 digits wraps around in the DLL (it gives the whole match); here it "
        "saturates and gives nothing"
    ),
    "minus-single--2": (
        "`$-{name}[n]` with an `n` past the groups of that name reads outside the DLL's list (the "
        "32-bit and 64-bit DLL differ for other indexes); UME Editor gives nothing"
    ),
    "probe-index-07": "the DLL reads a non-ASCII decimal digit in `[n]` (here U+0660); UME Editor reads ASCII digits only",
    "syn-hex-pattern-over-max": (
        "`\\x{110000}` in a pattern: the DLL's Onigmo (UTF-16) takes it and matches nothing; the "
        "UTF-8 Onigmo refuses a code point past U+10FFFF"
    ),
    "syn-ffff-in-pattern": (
        "U+FFFF is the delimiter Sakura puts around the pattern, so a U+FFFF typed in a pattern ends "
        "it for the DLL; UME Editor does not use a delimiter"
    ),
    "syn-ffff-in-class": (
        "U+FFFF is the delimiter Sakura puts around the pattern, so a U+FFFF typed in a pattern ends "
        "it for the DLL; UME Editor does not use a delimiter"
    ),
}
for id in ("hex-brace-surrogate-high", "hex-brace-surrogate-low", "hex-brace-over-max", "probe-062",
           "probe-063", "probe-064", "probe-065", "probe-066", "probe-076"):
    KNOWN[id] = LONE_SURROGATE
for id in ("start-at-end", "empty-match-replace-at-end", "empty-subject-empty-match", "empty-subject-caret",
           "empty-subject-dollar", "syn-empty-pattern-start-end", "pos-start-empty-pattern-at-end",
           "probe-start-equals-end-empty-pattern"):
    KNOWN[id] = EMPTY_TARGET
for id in ("syn-linebreak-R", "opt-m-always-caret", "eol-dollar-middle", "eol-caret-after-cr",
           "eol-caret-after-lf", "eol-class-negated-reaches-cr", "eol-lookahead-eol", "eol-g-dot-star",
           "eol-g-newline-class"):
    KNOWN[id] = MULTI_LINE
for id in ("syn-octal-pattern", "probe-pat-x41", "probe-pat-octal-0"):
    KNOWN[id] = BYTE_ESCAPE
for c in cases:
    if c["id"] in KNOWN:
        assert "known_difference" not in c, c["id"]
        c["known_difference"] = KNOWN.pop(c["id"])
assert not KNOWN, "unknown ids: %s" % sorted(KNOWN)

with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "cases.json"), "w", encoding="utf-8") as f:
    f.write('{"cases":[\n')
    f.write(",\n".join(json.dumps(c, ensure_ascii=True, separators=(",", ":")) for c in cases))
    f.write("\n]}\n")
print(len(cases), "cases")
