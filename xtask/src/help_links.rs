//! The link check of the help (ADR-0031): every link between pages and every `#anchor` points at something
//! that exists. mdBook checks neither anchors nor pages that no chapter lists.
//!
//! The anchors follow mdBook's rule (`mdbook-core` `utils::normalize_id`): a heading's text is lower-cased,
//! letters, digits (of any script), `_` and `-` stay, white space becomes `-`, everything else is dropped; a
//! second heading with the same id gets `-1`, the third `-2`. A heading may name its id with `{#id}`. Raw HTML
//! anchors and links (`id`, `name`, `href` attributes) are not allowed in the help: they are an error.
//!
//! The pages are read with pulldown-cmark, the parser that mdBook uses (the 0.13 series that mdBook 0.5.4 asks for, ^0.13.4), so that
//! headings, code, HTML and links mean what they mean in the built book.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
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

/// The options mdBook 0.5.4 reads Markdown with (`mdbook-markdown`, pulldown-cmark 0.13): tables, footnotes,
/// strikethrough, task lists and heading attributes (`{#id}`). Smart punctuation only changes quotes and
/// dashes, which no id keeps.
pub fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
}

/// The line number (from 1) of each byte offset of a text.
pub struct Lines {
    starts: Vec<usize>,
}

impl Lines {
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(source.match_indices('\n').map(|(i, _)| i + 1));
        Self { starts }
    }

    pub fn of(&self, offset: usize) -> u32 {
        let line = self.starts.partition_point(|start| *start <= offset);
        u32::try_from(line).unwrap_or(u32::MAX)
    }
}

/// The names of the attributes of an HTML tag (`name="value"`, `name='value'`, `name=value` or `name` alone),
/// in lower case.
fn tag_attribute_names(tag: &str) -> Vec<String> {
    let chars: Vec<char> = tag.chars().collect();
    let mut i = 0;
    // The tag name.
    while i < chars.len() && !chars[i].is_whitespace() {
        i += 1;
    }
    let mut out = Vec::new();
    while i < chars.len() {
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == '/') {
            i += 1;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() && !matches!(chars[i], '=' | '/') {
            i += 1;
        }
        let name: String = chars[start..i].iter().collect::<String>().to_lowercase();
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if chars.get(i) == Some(&'=') {
            i += 1;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            match chars.get(i) {
                Some(&quote @ ('"' | '\'')) => {
                    i += 1;
                    while i < chars.len() && chars[i] != quote {
                        i += 1;
                    }
                    i += 1;
                }
                _ => {
                    while i < chars.len() && !chars[i].is_whitespace() {
                        i += 1;
                    }
                }
            }
        }
        if !name.is_empty() {
            out.push(name);
        }
    }
    out
}

/// The raw HTML attributes that the help does not allow (`id`, `name`, `href`, in any case and any quoting) on
/// the real tags of a piece of raw HTML: `(line, attribute name)`. `line` is the line of the start of `html`.
/// Comments are not tags; a `<` that does not start a tag is text. Anchors are headings with `{#id}`, and links
/// are Markdown links, so that the check reads one syntax (the HTML would need character references, quotes and
/// attribute rules of its own).
fn forbidden_html_attributes(html: &str, line: u32, found: &mut Vec<(u32, String)>) {
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        let before = &html[..html.len() - rest.len() + open];
        let tag_line = line + u32::try_from(before.matches('\n').count()).unwrap_or(0);
        let after = &rest[open + 1..];
        if let Some(comment) = after.strip_prefix("!--") {
            rest = comment.find("-->").map_or("", |end| &comment[end + 3..]);
            continue;
        }
        let Some(close) = after.find('>') else { break };
        let tag = &after[..close];
        rest = &after[close + 1..];
        if !tag.starts_with(|c: char| c.is_ascii_alphabetic()) {
            continue;
        }
        for name in tag_attribute_names(tag) {
            if matches!(name.as_str(), "id" | "name" | "href") {
                found.push((tag_line, name));
            }
        }
    }
}

/// What a page offers and asks for: its anchors, its links as `(line, target)`, and the raw HTML attributes
/// that are not allowed, as `(line, attribute name)`.
pub struct Scan {
    pub anchors: HashSet<String>,
    pub links: Vec<(u32, String)>,
    pub raw_html: Vec<(u32, String)>,
}

/// Reads a page the way mdBook does (pulldown-cmark): headings with their ids (the rule above, `{#id}`),
/// Markdown links and images (inline, reference, autolinks, over several lines, with any title form). Code, comments and text that only looks like a link are not
/// read as links. A link with an empty target (the draft chapters of `SUMMARY.md`) is left out.
pub fn scan(source: &str) -> Scan {
    let lines = Lines::new(source);
    let mut anchors = HashSet::new();
    let mut links = Vec::new();
    let mut raw_html = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    // The text and explicit id of the heading being read.
    let mut heading: Option<(String, Option<String>)> = None;
    // A block of raw HTML arrives line by line; it is read as a whole, so that a comment over several lines
    // is one comment.
    let mut block: Option<(u32, String)> = None;
    for (event, range) in Parser::new_ext(source, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { id, .. }) => {
                heading = Some((String::new(), id.map(|id| id.to_string())));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((text, explicit)) = heading.take() {
                    let id = explicit.unwrap_or_else(|| {
                        let id = normalize_id(&text);
                        let count = seen.entry(id.clone()).or_insert(0);
                        let unique = if *count == 0 {
                            id
                        } else {
                            format!("{id}-{count}")
                        };
                        *count += 1;
                        unique
                    });
                    anchors.insert(id);
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((heading_text, _)) = &mut heading {
                    heading_text.push_str(&text);
                }
            }
            Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
                if !dest_url.is_empty() {
                    links.push((lines.of(range.start), dest_url.to_string()));
                }
            }
            Event::Start(Tag::HtmlBlock) => block = Some((lines.of(range.start), String::new())),
            Event::End(TagEnd::HtmlBlock) => {
                if let Some((line, html)) = block.take() {
                    forbidden_html_attributes(&html, line, &mut raw_html);
                }
            }
            Event::Html(html) if block.is_some() => {
                if let Some((_, text)) = &mut block {
                    text.push_str(&html);
                }
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                forbidden_html_attributes(&html, lines.of(range.start), &mut raw_html);
            }
            _ => {}
        }
    }
    Scan {
        anchors,
        links,
        raw_html,
    }
}

/// Every anchor of a page.
pub fn anchors(source: &str) -> HashSet<String> {
    scan(source).anchors
}

/// Every link of a page: `(line, target)`.
pub fn links(source: &str) -> Vec<(u32, String)> {
    scan(source).links
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
        let scanned = scan(&page.source);
        for (line, attribute) in &scanned.raw_html {
            problems.push(format!(
                "{}:{line}: raw HTML with the attribute `{attribute}` is not allowed. For an anchor, give a heading an id (`## Title {{#id}}`); for a link, use a Markdown link (`[text](target)`)",
                page.path
            ));
        }
        for (line, target) in scanned.links {
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
            // A link to a built chapter (`about.html`) is a link to its source (`about.md`).
            let resolved = match resolved.strip_suffix(".html") {
                Some(stem) if anchor_sets.contains_key(format!("{stem}.md").as_str()) => {
                    format!("{stem}.md")
                }
                _ => resolved,
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
    fn character_references_are_decoded_in_the_anchor() {
        let ids = anchors("## &#x8A2D;定 &amp; `&amp;`\n");
        assert!(ids.contains("設定--amp"), "{ids:?}");
    }

    #[test]
    fn raw_html_with_id_name_or_href_is_an_error() {
        let cases = [
            ("<a id=\"x\"></a>", "id"),
            ("<a id='x'></a>", "id"),
            ("<a name=\"x\"></a>", "name"),
            ("<a NAME='x'></a>", "name"),
            ("<a id=\"a&#45;b\"></a>", "id"),
            ("<a href=\"#a&#45;b\">x</a>", "href"),
            ("<a HREF='b.md'>x</a>", "href"),
            ("<a href=b.md>x</a>", "href"),
            ("<span\n  id=x>y</span>", "id"),
            ("text <span id=\"x\">y</span> text", "id"),
            ("<div>\n<a id=\"x\"></a>\n</div>", "id"),
        ];
        for (html, attribute) in cases {
            let found = scan(&format!("# t\n\n{html}\n")).raw_html;
            assert!(
                found.iter().any(|(_, name)| name == attribute),
                "{html:?} gave {found:?}"
            );
            let pages = book(&[
                ("SUMMARY.md", "- [a](a.md)\n"),
                ("a.md", &format!("# t\n\n{html}\n")),
            ]);
            let problems = check(&pages, Path::new("/nonexistent"));
            assert!(
                problems.iter().any(|p| p.starts_with("a.md:")
                    && p.contains("{#id}")
                    && p.contains("Markdown link")),
                "{html:?} gave {problems:?}"
            );
        }
    }

    #[test]
    fn raw_html_without_those_attributes_comments_and_code_are_fine() {
        let source = concat!(
            "# t\n\n",
            "<div class=\"note\" data-id=\"1\">\n\n本文\n\n</div>\n\n",
            "<!-- <a id=\"in-comment\"> -->\n\n",
            "<!--\n<a href=\"x\">\n-->\n\n",
            "`<a id=\"in-code\">` と\n\n",
            "```html\n<a id=\"in-fence\">\n```\n\n",
            "本文の id=\"in-text\" と <b>太字</b>\n",
        );
        assert_eq!(scan(source).raw_html, Vec::new());
    }

    #[test]
    fn the_line_of_a_raw_html_attribute_is_the_line_of_its_tag() {
        let found = scan("# t\n\n<div>\n<p>\n<a id=\"x\"></a>\n</p>\n</div>\n").raw_html;
        assert_eq!(found, vec![(5, "id".to_string())]);
    }

    #[test]
    fn double_underscore_emphasis_is_not_in_the_anchor() {
        let ids = anchors("## __設定__\n");
        assert!(ids.contains("設定"), "{ids:?}");
    }

    #[test]
    fn link_titles_and_fences_in_comments() {
        let pages = book(&[
            ("SUMMARY.md", "- [a](a.md)\n- [b](b.md)\n"),
            (
                "a.md",
                "# a\n\n<!--\n```\n-->\n\n[1](b.md 'タイトル') [2](b.md \"t\") [3](b.md (t)) [4](#missing)\n",
            ),
            ("b.md", "# b\n"),
        ]);
        let problems = check(&pages, Path::new("/nonexistent"));
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("#missing"));
    }

    #[test]
    fn links_over_several_lines_are_checked() {
        let pages = book(&[
            ("SUMMARY.md", "- [a](a.md)\n- [b](b.md)\n"),
            (
                "a.md",
                "# a\n\n[ok\nover two lines](b.md)\n\n[broken\ntext](\n  missing.md\n)\n\n[title](b.md\n  \"a title\non two lines\")\n\n[x](b.md#nope\n)\n",
            ),
            ("b.md", "# b\n"),
        ]);
        let problems = check(&pages, Path::new("/nonexistent"));
        assert_eq!(problems.len(), 2, "{problems:?}");
        // The line is the one where the link starts.
        assert!(
            problems[0].starts_with("a.md:6:") && problems[0].contains("missing.md"),
            "{problems:?}"
        );
        assert!(
            problems[1].starts_with("a.md:15:") && problems[1].contains("#nope"),
            "{problems:?}"
        );
    }

    #[test]
    fn a_link_to_a_built_chapter_is_a_link_to_its_source() {
        let pages = book(&[
            ("SUMMARY.md", "- [a](a.md)\n- [b](sub/b.md)\n"),
            (
                "a.md",
                "# a\n\n[1](sub/b.html#b) [2](sub/b.html#nope) [3](sub/b.html) [4](sub/c.html)\n",
            ),
            ("sub/b.md", "# b\n"),
        ]);
        let problems = check(&pages, Path::new("/nonexistent"));
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(
            problems[0].contains("#nope") && problems[1].contains("sub/c.html"),
            "{problems:?}"
        );
    }

    #[test]
    fn a_footnote_is_not_a_link() {
        let pages = book(&[
            ("SUMMARY.md", "- [a](a.md)\n"),
            ("a.md", "# a\n\n文[^1]\n\n[^1]: 補足説明です。\n"),
        ]);
        assert_eq!(
            check(&pages, Path::new("/nonexistent")),
            Vec::<String>::new()
        );
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
        assert!(problems[0].starts_with("a.md:3:") && problems[0].contains("#missing"));
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
