//! `cargo xtask help-check`: the checks of the help in `docs/help/` (ADR-0031), run by the `help` job of CI.
//!
//! - Links: every link between pages and every `#anchor` exists ([`crate::help_links`]).
//! - Shared text: no run of [`MIN_SHARED_CHARS`] or more characters is the same as in Sakura's help, which is
//!   GFDL and must not be copied or reworded ([`crate::help_shared`]).
//!
//! Later check (not built yet, there are no pages): that the function and macro names on the pages exist in the
//! code tables. Add it to [`run`] next to the two checks, taking the pages that `read_pages` returns and the
//! names that `names_from_tables` reads.

use std::path::{Path, PathBuf};

use crate::help_links;
use crate::help_shared::{self, MIN_SHARED_CHARS};

const USAGE: &str = "usage: cargo xtask help-check [--sakura <Sakura clone>] [--book <folder>] [--min <chars>] [--longest]\n\
\n\
  --sakura   a clone of https://github.com/sakura-editor/sakura (default: $SAKURA_DIR); without it the\n\
             shared-text check is skipped and only the links are checked\n\
  --book     the Japanese source of the book (default: docs/help/ja)\n\
  --min      the length of a shared run that fails (default: 20)\n\
  --longest  print the longest run that --book shares with Sakura's help, for choosing --min";

pub fn run(args: &[String], root: &Path) -> Result<(), String> {
    let mut sakura = std::env::var_os("SAKURA_DIR").map(PathBuf::from);
    let mut book = root.join("docs/help/ja");
    let mut min = MIN_SHARED_CHARS;
    let mut longest = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
        };
        match arg.as_str() {
            "--sakura" => sakura = Some(PathBuf::from(value("--sakura")?)),
            "--book" => book = PathBuf::from(value("--book")?),
            "--min" => {
                min = value("--min")?
                    .parse()
                    .map_err(|_| format!("--min needs a number\n{USAGE}"))?;
            }
            "--longest" => longest = true,
            _ => return Err(format!("unknown argument {arg}\n{USAGE}")),
        }
    }
    if min == 0 {
        return Err("--min must be 1 or more".into());
    }

    let pages = help_links::read_pages(&book)?;
    let mut problems = Vec::new();

    if !longest {
        let found = help_links::check(&pages, &book);
        println!("links: {} pages, {} problems", pages.len(), found.len());
        problems.extend(found);
    }

    match sakura {
        Some(sakura) => {
            let found = shared_text(&pages, &book, &sakura, root, min, longest)?;
            problems.extend(found);
        }
        None => println!("shared text: skipped (no --sakura and no SAKURA_DIR)"),
    }

    if problems.is_empty() {
        Ok(())
    } else {
        for problem in &problems {
            eprintln!("{problem}");
        }
        Err(format!("{} problems in the help", problems.len()))
    }
}

/// Reads every HTML file under `dir` (recursively), sorted by path: `(path relative to dir, text)`.
fn read_html(dir: &Path) -> Result<Vec<(String, String)>, String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) -> Result<(), String> {
        for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else if path.extension().is_some_and(|e| e == "html" || e == "htm") {
                let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                let text = String::from_utf8(bytes)
                    .map_err(|_| format!("{} is not UTF-8", path.display()))?;
                let relative = path.strip_prefix(root).map_err(|e| e.to_string())?;
                out.push((
                    relative.to_string_lossy().replace('\\', "/"),
                    text.trim_start_matches('\u{feff}').to_string(),
                ));
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn shared_text(
    pages: &[help_links::Page],
    book: &Path,
    sakura: &Path,
    root: &Path,
    min: usize,
    longest: bool,
) -> Result<Vec<String>, String> {
    let help = sakura.join("help");
    let html = read_html(&help)?;
    if html.is_empty() {
        return Err(format!(
            "{} has no HTML files: is it a Sakura clone?",
            help.display()
        ));
    }
    let mut names = help_shared::names_from_tables(&root.join("crates/matsu-commands/src/sakura"))?;
    help_shared::add_names_from_rc(&mut names, sakura)?;
    let sakura_pages: Vec<_> = html
        .iter()
        .map(|(path, text)| {
            (
                path.clone(),
                help_shared::prepare(&help_shared::strip_html(text), &names),
            )
        })
        .collect();
    let matsu_pages: Vec<_> = pages
        .iter()
        .map(|p| {
            (
                p.path.clone(),
                help_shared::prepare(&help_shared::strip_markdown(&p.source), &names),
            )
        })
        .collect();

    if longest {
        let n = help_shared::longest_shared(&matsu_pages, &sakura_pages, 1);
        println!("the longest shared run is {n} characters (not counting names)");
        if n == 0 {
            return Ok(Vec::new());
        }
        for shared in help_shared::find_shared(&matsu_pages, &sakura_pages, n) {
            println!(
                "  {}:{} = {}:{} ({} real characters): {}",
                shared.matsu_file,
                shared.matsu_line,
                shared.sakura_file,
                shared.sakura_line,
                shared.real_chars,
                shared.text
            );
        }
        return Ok(Vec::new());
    }

    let found = help_shared::find_shared(&matsu_pages, &sakura_pages, min);
    println!(
        "shared text: {} MATSU pages against {} Sakura pages, runs of {min} or more characters: {}",
        matsu_pages.len(),
        sakura_pages.len(),
        found.len()
    );
    Ok(found
        .into_iter()
        .map(|s| {
            let text: String = s.text.chars().take(200).collect();
            format!(
                "{}/{}:{}: shares {} characters with Sakura's help (help/{}, line {}). Write it from behavior, not from Sakura's help. Shared text (normalized): {text}",
                book.strip_prefix(root).unwrap_or(book).display(), s.matsu_file, s.matsu_line, s.real_chars, s.sakura_file, s.sakura_line
            )
        })
        .collect())
}
