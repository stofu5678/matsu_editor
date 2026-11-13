//! Lines that look like secrets (ADR-0030, 案 7).
//!
//! This is a short list of well-known token shapes, not a guarantee: a secret in any other shape
//! is not found, and the confirmation window says so. Each shape needs a fixed prefix at the start
//! of a word plus a minimum run of token characters (with a digit and a letter among them), so that
//! ordinary prose, Japanese text and code (`task-runner`, `risk_level`, `SKIP`) do not raise an
//! alarm.
//!
//! Shapes (the prefixes are the documented formats of the services):
//!
//! - `sk-...` API keys (OpenAI, Anthropic: `sk-ant-...`, `sk-proj-...`)
//! - GitHub tokens: `ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_`, `github_pat_`
//! - GitLab personal access tokens: `glpat-`
//! - AWS access key IDs: `AKIA` or `ASIA` and 16 upper-case letters or digits
//! - `-----BEGIN ... PRIVATE KEY-----` (PEM, also `OPENSSH` and `PGP PRIVATE KEY BLOCK`)
//! - Slack tokens: `xoxb-`, `xoxa-`, `xoxp-`, `xoxr-`, `xoxs-`
//! - Google API keys: `AIza` and 35 token characters
//! - Stripe secret keys: `sk_live_`, `rk_live_`
//! - JSON Web Tokens: three dot-separated parts, the first starting with `eyJ`
//! - `Bearer` followed by a long token (an `Authorization` header)
//! - a password inside a URL: `scheme://user:password@host`

use std::ops::Range;

/// The shape of a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretKind {
    ApiKey,
    GitHubToken,
    GitLabToken,
    AwsAccessKey,
    PrivateKey,
    SlackToken,
    GoogleApiKey,
    StripeKey,
    Jwt,
    BearerToken,
    UrlPassword,
}

impl SecretKind {
    /// A short name for the window (Japanese, as the window is).
    pub fn label(self) -> &'static str {
        match self {
            Self::ApiKey => "API キー (sk-)",
            Self::GitHubToken => "GitHub のトークン",
            Self::GitLabToken => "GitLab のトークン",
            Self::AwsAccessKey => "AWS のアクセスキー",
            Self::PrivateKey => "秘密鍵",
            Self::SlackToken => "Slack のトークン",
            Self::GoogleApiKey => "Google の API キー",
            Self::StripeKey => "Stripe のキー",
            Self::Jwt => "JWT",
            Self::BearerToken => "Bearer トークン",
            Self::UrlPassword => "URL の中のパスワード",
        }
    }
}

/// One finding in a line: the byte range of the secret part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretMatch {
    pub kind: SecretKind,
    pub range: Range<usize>,
}

/// A character that can be part of a token (`[A-Za-z0-9_-]`).
fn is_token(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

/// The words of `line`: maximal runs of token bytes. Every non-ASCII byte ends a run, so Japanese
/// text never forms a token.
fn words(line: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let bytes = line.as_bytes();
    let mut at = 0;
    std::iter::from_fn(move || {
        while at < bytes.len() && !is_token(bytes[at]) {
            at += 1;
        }
        if at >= bytes.len() {
            return None;
        }
        let start = at;
        while at < bytes.len() && is_token(bytes[at]) {
            at += 1;
        }
        Some(start..at)
    })
}

/// The kind of the word, if it has a known prefix and is long enough to be a real token.
fn word_kind(word: &str) -> Option<SecretKind> {
    // (prefix, minimum number of characters after the prefix, kind)
    const PREFIXED: &[(&str, usize, SecretKind)] = &[
        ("sk_live_", 16, SecretKind::StripeKey),
        ("rk_live_", 16, SecretKind::StripeKey),
        ("sk-", 20, SecretKind::ApiKey),
        ("ghp_", 30, SecretKind::GitHubToken),
        ("gho_", 30, SecretKind::GitHubToken),
        ("ghu_", 30, SecretKind::GitHubToken),
        ("ghs_", 30, SecretKind::GitHubToken),
        ("ghr_", 30, SecretKind::GitHubToken),
        ("github_pat_", 20, SecretKind::GitHubToken),
        ("glpat-", 20, SecretKind::GitLabToken),
        ("xoxb-", 10, SecretKind::SlackToken),
        ("xoxa-", 10, SecretKind::SlackToken),
        ("xoxp-", 10, SecretKind::SlackToken),
        ("xoxr-", 10, SecretKind::SlackToken),
        ("xoxs-", 10, SecretKind::SlackToken),
        ("AIza", 35, SecretKind::GoogleApiKey),
    ];
    for &(prefix, min, kind) in PREFIXED {
        if let Some(rest) = word.strip_prefix(prefix)
            && rest.len() >= min
            && looks_random(rest)
        {
            return Some(kind);
        }
    }
    for prefix in ["AKIA", "ASIA"] {
        if let Some(rest) = word.strip_prefix(prefix)
            && rest.len() == 16
            && rest
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        {
            return Some(SecretKind::AwsAccessKey);
        }
    }
    None
}

/// Whether `rest` has both a digit and a letter: a token is random, a hyphenated phrase
/// (`sk-this-is-a-long-word-list`) has no digits.
fn looks_random(rest: &str) -> bool {
    rest.bytes().any(|b| b.is_ascii_digit()) && rest.bytes().any(|b| b.is_ascii_alphabetic())
}

/// All findings in one line (without its line ending), in order, not overlapping.
pub fn find_secrets(line: &str) -> Vec<SecretMatch> {
    let mut found: Vec<SecretMatch> = Vec::new();
    if let Some(at) = line.find("-----BEGIN ")
        && line[at..].contains("PRIVATE KEY")
    {
        found.push(SecretMatch {
            kind: SecretKind::PrivateKey,
            range: at..line.len(),
        });
    }
    for range in words(line) {
        if let Some(kind) = word_kind(&line[range.clone()]) {
            found.push(SecretMatch { kind, range });
        }
    }
    found.extend(find_jwt(line));
    found.extend(find_bearer(line));
    found.extend(find_url_password(line));
    // Overlapping matches become one: the union, with the kind of the first.
    found.sort_by_key(|m| (m.range.start, std::cmp::Reverse(m.range.end)));
    let mut out: Vec<SecretMatch> = Vec::with_capacity(found.len());
    for m in found {
        match out.last_mut() {
            Some(last) if m.range.start < last.range.end => {
                last.range.end = last.range.end.max(m.range.end);
            }
            _ => out.push(m),
        }
    }
    out
}

/// `eyJ...` `.` part `.` part, each part at least 10 token characters.
fn find_jwt(line: &str) -> Vec<SecretMatch> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = line[from..].find("eyJ") {
        let start = from + i;
        from = start + 3;
        if start > 0 && is_token(bytes[start - 1]) {
            continue;
        }
        let mut end = start;
        let mut parts = 1;
        let mut part_len = 0;
        let mut ok = true;
        while end < bytes.len() && (is_token(bytes[end]) || bytes[end] == b'.') {
            if bytes[end] == b'.' {
                if part_len < 10 {
                    ok = false;
                    break;
                }
                parts += 1;
                part_len = 0;
                if parts > 3 {
                    ok = false;
                    break;
                }
            } else {
                part_len += 1;
            }
            end += 1;
        }
        if ok && parts == 3 && part_len >= 10 {
            out.push(SecretMatch {
                kind: SecretKind::Jwt,
                range: start..end,
            });
        }
        // What was walked is not walked again (a line of `eyJ...` words stays linear).
        from = from.max(end);
    }
    out
}

/// `Bearer` and a token of at least 20 characters with a digit and a letter.
fn find_bearer(line: &str) -> Vec<SecretMatch> {
    const MARK: &str = "Bearer ";
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = line[from..].find(MARK) {
        let start = from + i + MARK.len();
        from = start;
        let len = line.as_bytes()[start..]
            .iter()
            .take_while(|&&b| is_token(b) || matches!(b, b'.' | b'=' | b'+' | b'/'))
            .count();
        let token = start..start + len;
        if token.len() >= 20 && looks_random(&line[token.clone()]) {
            out.push(SecretMatch {
                kind: SecretKind::BearerToken,
                range: token,
            });
        }
    }
    out
}

/// The most bytes of `user:password@host` that are looked at.
const MAX_AUTHORITY: usize = 512;

/// `://user:password@host`: the password part.
fn find_url_password(line: &str) -> Vec<SecretMatch> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = line[from..].find("://") {
        let start = from + i + 3;
        from = start;
        // Only the start of what follows is looked at, so that a long line full of `://` stays
        // linear. A password longer than this is not found.
        let mut stop = (start + MAX_AUTHORITY).min(line.len());
        while !line.is_char_boundary(stop) {
            stop -= 1;
        }
        let rest = &line[start..stop];
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '/' | '"' | '\''))
            .unwrap_or(rest.len());
        let authority = &rest[..end];
        let Some(at) = authority.rfind('@') else {
            continue;
        };
        let Some(colon) = authority[..at].find(':') else {
            continue;
        };
        if colon + 1 < at {
            out.push(SecretMatch {
                kind: SecretKind::UrlPassword,
                range: start + colon + 1..start + at,
            });
        }
    }
    out
}

/// `line` with each match partly masked: the first 4 characters of a match of 12 or more
/// characters stay (enough to see what it is), the rest becomes `*`; a shorter match is masked
/// whole. A private key marker keeps only `-----BEGIN`. No secret is shown whole. `matches` must
/// come from [`find_secrets`] for the same line.
pub fn mask_line(line: &str, matches: &[SecretMatch]) -> String {
    let mut out = String::with_capacity(line.len());
    let mut at = 0;
    for m in matches {
        if m.range.start < at
            || m.range.end > line.len()
            || !line.is_char_boundary(m.range.start)
            || !line.is_char_boundary(m.range.end)
        {
            continue;
        }
        out.push_str(&line[at..m.range.start]);
        let text = &line[m.range.clone()];
        let keep = if m.kind == SecretKind::PrivateKey {
            "-----BEGIN".len()
        } else if text.chars().count() >= 12 {
            text.chars().take(4).map(char::len_utf8).sum()
        } else {
            0
        };
        out.push_str(&text[..keep]);
        out.extend(text[keep..].chars().map(|_| '*'));
        at = m.range.end;
    }
    out.push_str(&line[at..]);
    out
}

#[cfg(test)]
mod tests;
