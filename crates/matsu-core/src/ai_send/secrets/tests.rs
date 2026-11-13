use super::*;

fn kinds(line: &str) -> Vec<SecretKind> {
    find_secrets(line).into_iter().map(|m| m.kind).collect()
}

#[test]
fn finds_each_documented_shape() {
    let cases: &[(&str, SecretKind)] = &[
        (
            "OPENAI_API_KEY=sk-proj-abc123DEF456ghi789JKL012mno345",
            SecretKind::ApiKey,
        ),
        (
            "key: sk-ant-api03-Abcdefghij0123456789KLMNOP",
            SecretKind::ApiKey,
        ),
        (
            "token = ghp_0123456789abcdefghijABCDEFGHIJ012345",
            SecretKind::GitHubToken,
        ),
        (
            "github_pat_11ABCDEFG0abcdefghij_klmnopqrstuvwxyz0123456789",
            SecretKind::GitHubToken,
        ),
        ("aws AKIAIOSFODNN7EXAMPLE here", SecretKind::AwsAccessKey),
        ("-----BEGIN RSA PRIVATE KEY-----", SecretKind::PrivateKey),
        (
            "-----BEGIN OPENSSH PRIVATE KEY-----",
            SecretKind::PrivateKey,
        ),
        (
            "SLACK=xoxb-123456789012-abcdefghijklmnop",
            SecretKind::SlackToken,
        ),
        (
            "AIzaSyA1234567890abcdefghijklmnopqrstuv",
            SecretKind::GoogleApiKey,
        ),
        ("glpat-abcdefghij0123456789", SecretKind::GitLabToken),
        ("sk_live_0123456789abcdefABCDEF", SecretKind::StripeKey),
        (
            "jwt eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dBjftJeZ4CVPmB92K27uhbUJU1p1r",
            SecretKind::Jwt,
        ),
        (
            "Authorization: Bearer abcDEF0123456789abcdefGHIJ",
            SecretKind::BearerToken,
        ),
        (
            "git clone https://user:hunter2pass@example.com/repo.git",
            SecretKind::UrlPassword,
        ),
    ];
    for (line, kind) in cases {
        assert_eq!(kinds(line), vec![*kind], "{line}");
    }
}

#[test]
fn ordinary_text_and_code_raise_no_alarm() {
    let lines = [
        "これは普通の日本語の文章です。sk-learn のような言葉も含みます。",
        "task-runner and risk_level, SKIP and ask-me-anything",
        "let sk = 3; // sk-",
        "sk-this-is-a-long-hyphenated-phrase-with-no-digits",
        "ghp_ is the prefix GitHub uses",
        "AKIA is short, and AKIAlowercaseletters1234 is not a key",
        "https://example.com/path?q=1",
        "ssh://git@github.com/owner/repo.git",
        "http://localhost:8080/api",
        "Bearer token auth is described here",
        "BEGIN and END markers: -----BEGIN CERTIFICATE-----",
        "fn main() { println!(\"hello\"); }",
        "eyJ is a base64 start but eyJabc.def.ghi is too short",
        "xox is a word, xoxo-hugs",
        "",
    ];
    for line in lines {
        assert!(find_secrets(line).is_empty(), "{line}: {:?}", kinds(line));
    }
}

#[test]
fn a_key_next_to_japanese_text_is_found() {
    let line = "キーは sk-proj-abc123DEF456ghi789JKL012mno345 です";
    let found = find_secrets(line);
    assert_eq!(found.len(), 1);
    assert_eq!(
        &line[found[0].range.clone()],
        "sk-proj-abc123DEF456ghi789JKL012mno345"
    );
}

#[test]
fn several_secrets_in_one_line_are_found_in_order() {
    let line = "a=AKIAIOSFODNN7EXAMPLE b=ghp_0123456789abcdefghijABCDEFGHIJ012345";
    assert_eq!(
        kinds(line),
        vec![SecretKind::AwsAccessKey, SecretKind::GitHubToken]
    );
}

#[test]
fn masking_never_shows_a_whole_secret() {
    let line = "OPENAI_API_KEY=sk-proj-abc123DEF456ghi789JKL012mno345 # note";
    let masked = mask_line(line, &find_secrets(line));
    assert!(masked.starts_with("OPENAI_API_KEY=sk-p"));
    assert!(masked.ends_with("* # note"));
    assert!(!masked.contains("abc123"));
    assert_eq!(masked.chars().count(), line.chars().count());

    // A short secret is masked whole.
    let line = "https://user:hunter2@example.com/";
    let masked = mask_line(line, &find_secrets(line));
    assert_eq!(masked, "https://user:*******@example.com/");

    // A private key keeps only the marker.
    let line = "-----BEGIN RSA PRIVATE KEY-----";
    let masked = mask_line(line, &find_secrets(line));
    assert!(masked.starts_with("-----BEGIN*"));
    assert!(!masked.contains("PRIVATE"));
}

#[test]
fn masking_a_line_without_matches_changes_nothing() {
    assert_eq!(mask_line("日本語 text", &[]), "日本語 text");
}
