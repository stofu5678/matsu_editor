//! The link check of the help (ADR-0031): every link between pages and every `#anchor` points at something
//! that exists. mdBook checks neither anchors nor pages that no chapter lists.
//!
//! The anchors follow mdBook's rule (`mdbook-core` `utils::normalize_id`): a heading's text is lower-cased,
//! letters, digits (of any script), `_` and `-` stay, white space becomes `-`, everything else is dropped; a
//! second heading with the same id gets `-1`, the third `-2`. A heading may name its id with `{#id}`, and an
//! HTML `id="..."` or `name="..."` in the page is an anchor too.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

/// A page of the book: its path relative to the book's source folder (with `/`), and its text.
pub struct Page {
    pub path: String,
    pub source: String,
}

/// Reads every `*.md` file under `dir` (recursively), sorted by path.
pub fn read_pages(dir: &Path) -> Result<Vec<Page>, String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<Page>) -> Result<(), String> {
        let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else if path.extension().is_some_and(|e| e == "md") {
                let source = std::fs::read_to_string(&path)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                let relative = path.strip_prefix(root).map_err(|e| e.to_string())?;
                let path = relative
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.push(Page { path, source });
            }
        }
        Ok(())
    }
    let mut pages = Vec::new();
    walk(dir, dir, &mut pages)?;
    pages.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(pages)
}

/// The id that mdBook gives a heading.
pub fn normalize_id(text: &str) -> String {
    text.chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                Some(c.to_lowercase().next().unwrap_or(c))
            } else if c.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

/// The opening of a fenced code block: the fence character (`` ` `` or `~`) and its length (3 or more).
pub fn fence_open(trimmed: &str) -> Option<(char, usize)> {
    let mark = trimmed.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let len = trimmed.chars().take_while(|c| *c == mark).count();
    (len >= 3).then_some((mark, len))
}

/// Whether a line closes a block opened with `len` fence characters `mark`: the same character, at least as
/// many, and nothing else on the line.
pub fn fence_closes(trimmed: &str, (mark, len): (char, usize)) -> bool {
    let count = trimmed.chars().take_while(|c| *c == mark).count();
    count >= len && trimmed[count * mark.len_utf8()..].trim().is_empty()
}

/// The lines of a page outside its fenced code blocks, with their numbers (from 1).
fn prose_lines(source: &str) -> Vec<(u32, &str)> {
    let mut fence: Option<(char, usize)> = None;
    let mut comment = false;
    let mut out = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(open) = fence {
            if fence_closes(trimmed, open) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_open(trimmed) {
            fence = Some(open);
            continue;
        }
        // A comment that starts at the beginning of a line hides its lines (and any heading in them).
        if comment {
            comment = !line.contains("-->");
            continue;
        }
        if trimmed.starts_with("<!--") {
            comment = !trimmed.contains("-->");
            if comment || trimmed.trim_end().ends_with("-->") {
                continue;
            }
        }
        out.push((u32::try_from(index + 1).unwrap_or(u32::MAX), line));
    }
    out
}

/// The text of a heading as the reader sees it: no link targets, emphasis marks, code marks or tags.
fn heading_text(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    let mut previous: Option<char> = None;
    let mut in_code = false;
    while let Some(c) = chars.next() {
        let before = previous.replace(c);
        match c {
            '`' => in_code = !in_code,
            // Inside a code span everything is literal text.
            _ if in_code => out.push(c),
            // An underscore at the edge of a word is an emphasis mark (`_設定_`); one inside a word stays.
            '_' if before.is_none_or(char::is_whitespace)
                || chars.peek().is_none_or(|n| n.is_whitespace()) => {}
            ']' if chars.peek() == Some(&'[') => {
                // The label of a reference link, `[text][label]`.
                for n in chars.by_ref() {
                    if n == ']' {
                        break;
                    }
                }
            }
            ']' if chars.peek() == Some(&'(') => {
                for n in chars.by_ref() {
                    if n == ')' {
                        break;
                    }
                }
            }
            '[' | ']' | '*' => {}
            '<' => {
                for n in chars.by_ref() {
                    if n == '>' {
                        break;
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Every anchor of a page.
pub fn anchors(source: &str) -> HashSet<String> {
    let mut ids = HashSet::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    let lines = prose_lines(source);
    for (index, &(number, line)) in lines.iter().enumerate() {
        // An ATX heading may be indented by up to 3 spaces. A Setext heading is a line of text followed by a
        // line of `=` or `-`.
        let indent = line.len() - line.trim_start_matches(' ').len();
        let atx = line.trim_start_matches(' ');
        let hashes = atx.chars().take_while(|c| *c == '#').count();
        let underline = lines.get(index + 1).is_some_and(|&(next, below)| {
            let below = below.trim();
            next == number + 1
                && !below.is_empty()
                && (below.chars().all(|c| c == '=') || below.chars().all(|c| c == '-'))
        });
        let plain = !atx.is_empty()
            && !atx.starts_with(['#', '-', '*', '+', '>', '|', '<', '['])
            && !atx.starts_with(|c: char| c.is_ascii_digit());
        let heading = if indent <= 3 && (1..=6).contains(&hashes) && atx[hashes..].starts_with(' ')
        {
            Some(&atx[hashes..])
        } else if indent <= 3 && underline && plain {
            Some(atx)
        } else {
            None
        };
        if let Some(text) = heading {
            let text = text.trim().trim_end_matches('#').trim_end();
            let id = match text.rsplit_once("{#") {
                Some((_, rest)) if rest.ends_with('}') => rest[..rest.len() - 1].to_string(),
                _ => {
                    let id = normalize_id(&heading_text(text));
                    let count = seen.entry(id.clone()).or_insert(0);
                    let unique = if *count == 0 {
                        id
                    } else {
                        format!("{id}-{count}")
                    };
                    *count += 1;
                    unique
                }
            };
            ids.insert(id);
        }
        for key in ["id=\"", "name=\""] {
            let mut rest = line;
            while let Some(start) = rest.find(key) {
                rest = &rest[start + key.len()..];
                if let Some(end) = rest.find('"') {
                    ids.insert(rest[..end].to_string());
                }
            }
        }
    }
    ids
}

/// The line without its inline code spans (`` `...` ``) and HTML comments; `in_comment` carries a comment that
/// goes over several lines.
fn without_code_and_comments(line: &str, in_comment: &mut bool) -> String {
    let mut out = String::new();
    let mut rest = line;
    loop {
        if *in_comment {
            match rest.find("-->") {
                Some(end) => {
                    rest = &rest[end + 3..];
                    *in_comment = false;
                }
                None => return out,
            }
        }
        let comment = rest.find("<!--");
        let code = rest.find('`');
        match (comment, code) {
            (Some(c), code) if code.is_none_or(|k| c < k) => {
                out.push_str(&rest[..c]);
                rest = &rest[c + 4..];
                *in_comment = true;
            }
            (_, Some(k)) => {
                out.push_str(&rest[..k]);
                // The span ends at the next run of backticks (an unclosed one is plain text).
                let ticks = rest[k..].chars().take_while(|c| *c == '`').count();
                let marker = "`".repeat(ticks);
                match rest[k + ticks..].find(&marker) {
                    Some(end) => rest = &rest[k + ticks + end + ticks..],
                    None => {
                        out.push_str(&rest[k..]);
                        return out;
                    }
                }
            }
            _ => {
                out.push_str(rest);
                return out;
            }
        }
    }
}

/// Every link of a page: `(line, target)`. Links with an empty target (the draft chapters of `SUMMARY.md`) are
/// left out.
pub fn links(source: &str) -> Vec<(u32, String)> {
    let mut out = Vec::new();
    let mut in_comment = false;
    for (number, line) in prose_lines(source) {
        // What is not rendered as Markdown is not a link: inline code and HTML comments.
        let line = without_code_and_comments(line, &mut in_comment);
        let line = line.as_str();
        // A reference definition, `[label]: target "title"`: the target counts as a link whether or not
        // a reference uses it (a reference such as `[text][label]` is then checked through it).
        if let Some(definition) = line
            .trim_start()
            .strip_prefix('[')
            .and_then(|rest| rest.split_once("]:"))
        {
            let target = definition.1.split_whitespace().next().unwrap_or("");
            let target = target.trim_start_matches('<').trim_end_matches('>');
            if !target.is_empty() {
                out.push((number, target.to_string()));
            }
            continue;
        }
        let mut rest = line;
        while let Some(start) = rest.find("](") {
            rest = &rest[start + 2..];
            let mut depth = 1;
            let mut end = None;
            for (i, c) in rest.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(end) = end else { break };
            let mut target = rest[..end].trim();
            // `[text](target "title")` and `[text](<target>)`.
            if let Some((t, _)) = target.split_once(" \"") {
                target = t.trim();
            }
            let target = target.trim_start_matches('<').trim_end_matches('>');
            if !target.is_empty() {
                out.push((number, target.to_string()));
            }
            rest = &rest[end..];
        }
    }
    out
}

/// Decodes `%XX` escapes (UTF-8) in a URL fragment: `#%E8%A8%AD%E5%AE%9A` is `#設定`.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (byte, _) => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn is_external(target: &str) -> bool {
    // `http:`, `https:`, `mailto:` and any other scheme.
    target.split_once(':').is_some_and(|(scheme, _)| {
        !scheme.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
    })
}

/// Joins a relative link to the folder of the page that holds it, resolving `.` and `..`. `None` when the link
/// leaves the book.
fn resolve(page: &str, target: &str) -> Option<String> {
    let mut parts: Vec<String> = Path::new(page)
        .parent()
        .into_iter()
        .flat_map(|p| p.components())
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    for component in Path::new(target).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::Normal(name) => parts.push(name.to_string_lossy().into_owned()),
            _ => return None,
        }
    }
    Some(parts.join("/"))
}

/// Checks the links of every page (and `SUMMARY.md`) and that every page is in `SUMMARY.md`. `dir` is the
/// book's source folder (for links to files that are not pages, such as images). Returns the problems, one
/// line each.
pub fn check(pages: &[Page], dir: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let anchor_sets: HashMap<&str, HashSet<String>> = pages
        .iter()
        .map(|p| (p.path.as_str(), anchors(&p.source)))
        .collect();
    for page in pages {
        for (line, target) in links(&page.source) {
            if is_external(&target) {
                continue;
            }
            let (path, anchor) = match target.split_once('#') {
                Some((path, anchor)) => (path, Some(anchor)),
                None => (target.as_str(), None),
            };
            let resolved = if path.is_empty() {
                Some(page.path.clone())
            } else {
                resolve(&page.path, path)
            };
            let Some(resolved) = resolved else {
                problems.push(format!(
                    "{}:{line}: the link `{target}` leaves the book",
                    page.path
                ));
                continue;
            };
            if let Some(anchors) = anchor_sets.get(resolved.as_str()) {
                if let Some(anchor) = anchor
                    && !anchor.is_empty()
                    && !anchors.contains(anchor)
                    && !anchors.contains(&percent_decode(anchor))
                {
                    problems.push(format!(
                        "{}:{line}: `{target}` points at the anchor `#{anchor}`, which `{resolved}` does not have",
                        page.path
                    ));
                }
            } else if !PathBuf::from(dir).join(&resolved).exists() {
                problems.push(format!(
                    "{}:{line}: `{target}` points at `{resolved}`, which does not exist",
                    page.path
                ));
            }
        }
    }
    // Every page is a chapter: mdBook leaves out a page that SUMMARY.md does not list.
    if let Some(summary) = pages.iter().find(|p| p.path == "SUMMARY.md") {
        let listed: HashSet<String> = links(&summary.source)
            .into_iter()
            .filter(|(_, t)| !is_external(t))
            .filter_map(|(_, t)| resolve("SUMMARY.md", t.split('#').next().unwrap_or("")))
            .collect();
        for page in pages {
            if page.path != "SUMMARY.md" && !listed.contains(&page.path) {
                problems.push(format!(
                    "{}: the page is not listed in SUMMARY.md",
                    page.path
                ));
            }
        }
    } else {
        problems.push("SUMMARY.md is missing".into());
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(path: &str, source: &str) -> Page {
        Page {
            path: path.into(),
            source: source.into(),
        }
    }

    fn book(files: &[(&str, &str)]) -> Vec<Page> {
        files.iter().map(|(p, s)| page(p, s)).collect()
    }

    #[test]
    fn heading_ids_follow_mdbook() {
        let ids = anchors(
            "# このヘルプについて\n\n## Hello, World!\n\n## Hello, World!\n\n## `code` と **強調** と [リンク](a.md)\n\n## 名前 {#custom}\n\n```\n# 飛ばす\n```\n",
        );
        for id in [
            "このヘルプについて",
            "hello-world",
            "hello-world-1",
            "code-と-強調-と-リンク",
            "custom",
        ] {
            assert!(ids.contains(id), "{id} is missing in {ids:?}");
        }
        assert!(!ids.contains("飛ばす"));
    }

    #[test]
    fn a_good_book_has_no_problems() {
        let pages = book(&[
            (
                "SUMMARY.md",
                "# 目次\n\n- [前書き](intro.md)\n- [まだない]()\n  - [次](sub/next.md)\n",
            ),
            (
                "intro.md",
                "# 前書き\n\n## 節\n\n[次](sub/next.md#終わり) [節](#節) [外](https://example.com/a(b))\n",
            ),
            ("sub/next.md", "# 次\n\n## 終わり\n\n[戻る](../intro.md)\n"),
        ]);
        assert_eq!(
            check(&pages, Path::new("/nonexistent")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_broken_page_link_is_caught() {
        let pages = book(&[
            ("SUMMARY.md", "- [前書き](intro.md)\n"),
            ("intro.md", "# 前書き\n\n[無い](missing.md)\n"),
        ]);
        let problems = check(&pages, Path::new("/nonexistent"));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].starts_with("intro.md:3:"), "{problems:?}");
        assert!(problems[0].contains("missing.md"));
    }

    #[test]
    fn a_broken_anchor_is_caught() {
        let pages = book(&[
            ("SUMMARY.md", "- [前書き](intro.md)\n- [次](next.md)\n"),
            (
                "intro.md",
                "# 前書き\n\n[a](next.md#無い) [b](#これも無い)\n",
            ),
            ("next.md", "# 次\n"),
        ]);
        let problems = check(&pages, Path::new("/nonexistent"));
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].contains("#無い"));
        assert!(problems[1].contains("#これも無い"));
    }

    #[test]
    fn links_in_inline_code_and_comments_are_not_checked() {
        let pages = book(&[
            ("SUMMARY.md", "- [a](a.md)\n"),
            (
                "a.md",
                "# a\n\n`[x](nowhere.md)` と ``[y](nowhere.md)`` と <!-- [z](nowhere.md) --> と\n<!-- 複数行\n[w](nowhere.md)\n-->と [自分](#a)\n",
            ),
        ]);
        assert_eq!(
            check(&pages, Path::new("/nonexistent")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn setext_and_indented_headings_have_anchors() {
        let ids =
            anchors("見出し一\n=====\n\n本文\n\n見出し二\n---\n\n   ### 字下げ\n\n- 項目\n---\n");
        for id in ["見出し一", "見出し二", "字下げ"] {
            assert!(ids.contains(id), "{id} is missing in {ids:?}");
        }
        assert!(!ids.contains("項目") && !ids.contains("本文"));
    }

    #[test]
    fn headings_in_comments_have_no_anchor() {
        let ids = anchors("# a\n\n<!--\n## 隠す\n-->\n\n<!-- ## 一行 -->\n\n## 見せる\n");
        assert!(
            ids.contains("見せる") && !ids.contains("隠す") && !ids.contains("一行"),
            "{ids:?}"
        );
    }

    #[test]
    fn a_longer_fence_is_not_closed_by_a_shorter_one() {
        let pages = book(&[
            ("SUMMARY.md", "- [a](a.md)\n"),
            (
                "a.md",
                "# a\n\n````md\n```\n[x](nowhere.md)\n## 隠す\n```\n[y](nowhere.md)\n````\n\n~~~\n```\n~~~\n\n[自分](#a)\n",
            ),
        ]);
        assert_eq!(
            check(&pages, Path::new("/nonexistent")),
            Vec::<String>::new()
        );
        assert!(!anchors(&pages[1].source).contains("隠す"));
    }

    #[test]
    fn a_percent_encoded_fragment_matches_the_heading() {
        let pages = book(&[
            ("SUMMARY.md", "- [a](a.md)\n"),
            (
                "a.md",
                "# a\n\n## 設定\n\n[x](#%E8%A8%AD%E5%AE%9A) [y](#%E7%84%A1%E3%81%84) [z](#100%)\n",
            ),
        ]);
        let problems = check(&pages, Path::new("/nonexistent"));
        assert_eq!(problems.len(), 2, "{problems:?}");
    }

    #[test]
    fn reference_link_labels_are_not_in_the_anchor() {
        let ids = anchors("## [設定][settings]\n\n[settings]: a.md\n");
        assert!(
            ids.contains("設定") && !ids.contains("設定settings"),
            "{ids:?}"
        );
    }

    #[test]
    fn code_spans_keep_their_text_in_the_anchor() {
        let ids = anchors("## `<検索文字列>` と `a_b_`\n");
        assert!(ids.contains("検索文字列-と-a_b_"), "{ids:?}");
    }

    #[test]
    fn emphasis_underscores_are_not_in_the_anchor() {
        let ids = anchors("## _設定_\n\n## my_name\n");
        assert!(ids.contains("設定") && ids.contains("my_name"), "{ids:?}");
    }

    #[test]
    fn a_reference_style_link_is_checked() {
        let pages = book(&[
            ("SUMMARY.md", "- [a](a.md)\n- [b](b.md)\n"),
            (
                "a.md",
                "# a\n\n[次][next] [b][]\n\n[next]: b.md#missing \"title\"\n[b]: b.md\n",
            ),
            ("b.md", "# b\n"),
        ]);
        let problems = check(&pages, Path::new("/nonexistent"));
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].starts_with("a.md:5:") && problems[0].contains("#missing"));
    }

    #[test]
    fn a_page_missing_from_the_summary_is_caught() {
        let pages = book(&[
            ("SUMMARY.md", "- [前書き](intro.md)\n"),
            ("intro.md", "# a\n"),
            ("lost.md", "# b\n"),
        ]);
        let problems = check(&pages, Path::new("/nonexistent"));
        assert_eq!(
            problems,
            vec!["lost.md: the page is not listed in SUMMARY.md".to_string()]
        );
    }

    #[test]
    fn links_in_code_blocks_are_not_checked() {
        let pages = book(&[
            ("SUMMARY.md", "- [a](a.md)\n"),
            ("a.md", "# a\n\n```md\n[x](nowhere.md)\n```\n"),
        ]);
        assert!(check(&pages, Path::new("/nonexistent")).is_empty());
    }

    #[test]
    fn a_link_that_leaves_the_book_is_caught() {
        let pages = book(&[
            ("SUMMARY.md", "- [a](a.md)\n"),
            ("a.md", "# a\n\n[x](../../outside.md)\n"),
        ]);
        let problems = check(&pages, Path::new("/nonexistent"));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("leaves the book"));
    }
}
