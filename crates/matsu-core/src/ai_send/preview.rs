//! The head, the tail and the size of a range of a document, and the lines that look like secrets
//! (ADR-0030, 案 2, 4 and 7). The confirmation window shows this.
//!
//! The range is read line by line with [`Document::line_slice`]; it is never gathered into one
//! string. Reading is bounded, so that a huge selection cannot freeze the window:
//!
//! - the head and the tail are read directly by line number (a few lines, each cut at a few KB);
//! - the line count is exact (it comes from the positions);
//! - the character count and the secret scan walk the lines from the start until a budget of
//!   [`PreviewLimits::scan_bytes`] decoded bytes is used up (4 MiB by default, a few
//!   milliseconds). After that the character count is a lower bound
//!   ([`TextPreview::chars_exact`] is false) and the scan stopped
//!   ([`TextPreview::secrets_complete`] is false). A single line longer than what is left of the
//!   budget is read only up to the budget.
//!
//! Finding the two ends of the range costs what a cursor move to them costs ([`Document::clamp_pos`]
//! and [`Document::raw_len_between`], the same as the editor does for any selection): very long
//! lines of a big file are read by ranges and indexed (ADR-0020), so that is cheap there. The
//! budget bounds what this module adds on top.
//!
//! A line ending counts as one character (two for CR LF).

use super::secrets::{SecretKind, find_secrets, mask_line};
use crate::buffer::{Document, Pos, Result};

/// Text over this many bytes is "large" (ADR-0030, 案 4): the window says so and makes
/// 「送らない」 the default.
pub const LARGE_TEXT_BYTES: usize = 100 * 1024;

/// How much of the text is read, and how much is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewLimits {
    /// Lines shown from the start.
    pub head_lines: usize,
    /// Lines shown from the end.
    pub tail_lines: usize,
    /// Decoded bytes read for the character count and the secret scan.
    pub scan_bytes: usize,
    /// Secret lines kept (the total is still counted).
    pub max_secret_lines: usize,
    /// Characters shown of a line; the rest is replaced by `…`.
    pub line_chars: usize,
}

impl Default for PreviewLimits {
    fn default() -> Self {
        Self {
            head_lines: 10,
            tail_lines: 5,
            scan_bytes: 4 * 1024 * 1024,
            max_secret_lines: 20,
            line_chars: 200,
        }
    }
}

/// A line to show, with its number in the document (from 1). Secrets in it are masked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewLine {
    pub number: usize,
    pub text: String,
}

/// A line that looks like it has a secret. `text` is the part around the secret, masked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretLine {
    pub number: usize,
    pub kinds: Vec<SecretKind>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextPreview {
    /// The first lines (all of them when the text is short).
    pub head: Vec<PreviewLine>,
    /// The last lines, empty when the head holds everything.
    pub tail: Vec<PreviewLine>,
    /// How many lines lie between the head and the tail.
    pub omitted_lines: usize,
    /// The number of lines (0 for an empty range).
    pub lines: usize,
    /// The number of characters; a lower bound when `chars_exact` is false.
    pub chars: usize,
    pub chars_exact: bool,
    /// The size in bytes: what the document holds for the range (its own encoding), or what was
    /// read as UTF-8 if that is more.
    pub bytes: usize,
    /// Over [`LARGE_TEXT_BYTES`].
    pub large: bool,
    /// Lines that look like secrets, in order, at most `max_secret_lines`.
    pub secrets: Vec<SecretLine>,
    /// How many lines look like secrets (can be more than `secrets` holds).
    pub secret_lines_total: usize,
    /// False when the scan stopped at its budget before the end of the range.
    pub secrets_complete: bool,
}

/// What the window shows for the text between `from` and `to` (either order, clamped).
pub fn preview_range(
    doc: &Document,
    from: Pos,
    to: Pos,
    limits: &PreviewLimits,
) -> Result<TextPreview> {
    let (a, b) = (doc.clamp_pos(from)?, doc.clamp_pos(to)?);
    let (from, to) = if a <= b { (a, b) } else { (b, a) };
    let raw_len = doc.raw_len_between(from, to)?;
    let mut preview = TextPreview {
        head: Vec::new(),
        tail: Vec::new(),
        omitted_lines: 0,
        lines: 0,
        chars: 0,
        chars_exact: true,
        bytes: raw_len,
        large: raw_len > LARGE_TEXT_BYTES,
        secrets: Vec::new(),
        secret_lines_total: 0,
        secrets_complete: true,
    };
    if from == to {
        return Ok(preview);
    }
    // A range that ends at column 0 of a line ends with the line break of the line before it.
    let last = if to.col == 0 && to.line > from.line {
        to.line - 1
    } else {
        to.line
    };
    let range = Selection { doc, from, to };
    preview.lines = last - from.line + 1;

    // The lines inside `-----BEGIN ... PRIVATE KEY-----` blocks (after the marker, up to the
    // `-----END` line): their text is the secret and is shown masked whole, line by line.
    let mut key_payload: Vec<std::ops::Range<usize>> = Vec::new();
    let mut key_from: Option<usize> = None;
    let mut budget = limits.scan_bytes;
    let mut scanned_bytes = 0;
    for i in from.line..=last {
        if budget == 0 {
            preview.chars_exact = false;
            preview.secrets_complete = false;
            break;
        }
        let (text, cut) = range.read(i, budget)?;
        budget = budget.saturating_sub(text.len().max(1));
        scanned_bytes += text.len();
        preview.chars += text.chars().count();
        if i < to.line {
            // The line break after the line is part of the range.
            if let Some(eol) = doc.line_eol(i)? {
                preview.chars += eol.as_str().chars().count();
                scanned_bytes += eol.as_str().len();
            }
        }
        if let Some(start) = key_from
            && text.contains("-----END")
        {
            key_payload.push(start..i);
            key_from = None;
        }
        let matches = find_secrets(&text);
        if key_from.is_none()
            && matches.iter().any(|m| m.kind == SecretKind::PrivateKey)
            && !text.contains("-----END")
        {
            key_from = Some(i + 1);
        }
        if !matches.is_empty() {
            preview.secret_lines_total += 1;
            if preview.secrets.len() < limits.max_secret_lines {
                let masked = masked_text(&text, cut);
                let at = text[..matches[0].range.start].chars().count();
                let mut kinds: Vec<SecretKind> = Vec::new();
                for m in &matches {
                    if !kinds.contains(&m.kind) {
                        kinds.push(m.kind);
                    }
                }
                preview.secrets.push(SecretLine {
                    number: i + 1,
                    kinds,
                    text: window_around(&masked, at, limits.line_chars),
                });
            }
        }
        if cut {
            preview.chars_exact = false;
            preview.secrets_complete = false;
            break;
        }
    }
    if let Some(start) = key_from {
        // No end marker in what was scanned: the rest of the range is the key.
        key_payload.push(start..last + 1);
    }
    let shown = limits.head_lines + limits.tail_lines;
    if preview.lines <= shown {
        preview.head = range.shown_lines(from.line..last + 1, limits, &key_payload)?;
    } else {
        preview.head = range.shown_lines(
            from.line..from.line + limits.head_lines,
            limits,
            &key_payload,
        )?;
        preview.tail =
            range.shown_lines(last + 1 - limits.tail_lines..last + 1, limits, &key_payload)?;
        preview.omitted_lines = preview.lines - shown;
    }

    preview.bytes = raw_len.max(scanned_bytes);
    preview.large = preview.bytes > LARGE_TEXT_BYTES;
    Ok(preview)
}

/// The preview of an input that is `head` followed by the text that `material` previews (mode b of ADR-0035: the
/// instruction, a blank line and `---`, then the target text). `head` is the text before the material, as
/// [`instruction_head`](super::instruction_head) gives it: its lines are numbered from 1 and the lines of the
/// material go on from there, so that the numbers are those of the input that is sent. The material is not read
/// again: only the head is scanned, so this is cheap to call at every key of the instruction field.
///
/// Lines of the head that look like secrets are counted and masked like those of the material.
pub fn preview_with_head(
    head: &str,
    material: &TextPreview,
    limits: &PreviewLimits,
) -> TextPreview {
    let head_lines: Vec<&str> = head
        .strip_suffix('\n')
        .unwrap_or(head)
        .split('\n')
        .collect();
    let k = head_lines.len();
    // the number of the first line of the material in its own preview (the line of the document)
    let first = material.head.first().map_or(1, |l| l.number);
    let renumber = |n: usize| k + 1 + n.saturating_sub(first);
    let shift = |lines: &[PreviewLine]| -> Vec<PreviewLine> {
        lines
            .iter()
            .map(|l| PreviewLine {
                number: renumber(l.number),
                text: l.text.clone(),
            })
            .collect()
    };

    let mut secrets: Vec<SecretLine> = Vec::new();
    let mut secret_lines_total = 0;
    let mut shown_head: Vec<PreviewLine> = Vec::new();
    for (i, text) in head_lines.iter().enumerate() {
        let matches = find_secrets(text);
        let masked = mask_line(text, &matches);
        if !matches.is_empty() {
            secret_lines_total += 1;
            if secrets.len() < limits.max_secret_lines {
                let at = text[..matches[0].range.start].chars().count();
                let mut kinds: Vec<SecretKind> = Vec::new();
                for m in &matches {
                    if !kinds.contains(&m.kind) {
                        kinds.push(m.kind);
                    }
                }
                secrets.push(SecretLine {
                    number: i + 1,
                    kinds,
                    text: window_around(&masked, at, limits.line_chars),
                });
            }
        }
        shown_head.push(PreviewLine {
            number: i + 1,
            text: window_around(&masked, 0, limits.line_chars),
        });
    }
    for line in &material.secrets {
        if secrets.len() < limits.max_secret_lines {
            secrets.push(SecretLine {
                number: renumber(line.number),
                kinds: line.kinds.clone(),
                text: line.text.clone(),
            });
        }
    }
    secret_lines_total += material.secret_lines_total;

    let total_lines = k + material.lines;
    // what is known: the head and the first lines of the material, then (when the material was cut) its last lines
    let mut front = shown_head;
    front.extend(shift(&material.head));
    let back = shift(&material.tail);
    let shown = limits.head_lines + limits.tail_lines;
    let (head_part, tail_part, omitted_lines) = if back.is_empty() {
        if front.len() <= shown {
            (front, Vec::new(), 0)
        } else {
            let tail = front.split_off(front.len() - limits.tail_lines);
            front.truncate(limits.head_lines);
            let omitted = total_lines - front.len() - tail.len();
            (front, tail, omitted)
        }
    } else {
        // `front` has the head and at least the 10 first lines of the material: more than `head_lines`
        front.truncate(limits.head_lines);
        let omitted = total_lines - front.len() - back.len();
        (front, back, omitted)
    };

    let bytes = head.len() + material.bytes;
    TextPreview {
        head: head_part,
        tail: tail_part,
        omitted_lines,
        lines: total_lines,
        chars: head.chars().count() + material.chars,
        chars_exact: material.chars_exact,
        bytes,
        large: bytes > LARGE_TEXT_BYTES,
        secrets,
        secret_lines_total,
        secrets_complete: material.secrets_complete,
    }
}

/// The most bytes read of a line to show.
const SHOWN_READ_BYTES: usize = 64 * 1024;

struct Selection<'a> {
    doc: &'a Document,
    from: Pos,
    to: Pos,
}

impl Selection<'_> {
    /// The text of line `i` inside the range, read up to `cap` bytes. The flag is true when the
    /// line has more text than was read.
    fn read(&self, i: usize, cap: usize) -> Result<(String, bool)> {
        let from_col = if i == self.from.line {
            self.from.col
        } else {
            0
        };
        let to_col = if i == self.to.line {
            self.to.col
        } else {
            usize::MAX
        };
        // Four bytes more than `cap` show whether anything is left (`line_slice` cuts back to a
        // character start, so a longer line gives at least `cap + 1` bytes).
        let limit = to_col.min(from_col.saturating_add(cap).saturating_add(4));
        let slice = self.doc.line_slice(i, from_col, limit)?;
        let mut text = slice.map(|s| s.text).unwrap_or_default();
        let cut = text.len() > cap;
        if cut {
            let mut at = cap;
            while !text.is_char_boundary(at) {
                at -= 1;
            }
            text.truncate(at);
        }
        Ok((text, cut))
    }

    /// The lines `lines` as the window shows them: masked, and cut to `limits.line_chars`.
    fn shown_lines(
        &self,
        lines: std::ops::Range<usize>,
        limits: &PreviewLimits,
        key_payload: &[std::ops::Range<usize>],
    ) -> Result<Vec<PreviewLine>> {
        // Read as much as the scan does of a line, so that a secret that the scan finds is found
        // here too and masked (the shown part is cut after masking).
        let texts = lines
            .clone()
            .map(|i| self.read(i, SHOWN_READ_BYTES))
            .collect::<Result<Vec<_>>>()?;
        // Inside a private key block the whole line is the secret. The scan knows the blocks it
        // reached (`key_payload`). For the lines it did not reach (the tail of a huge range) the
        // block is found from the shown lines themselves, from its BEGIN or back from its END,
        // and a line that looks like a line of a key (base64, 60 characters or more) is masked
        // too.
        let in_block = private_key_lines(&texts.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>());
        Ok(lines
            .zip(texts)
            .zip(in_block)
            .map(|((i, (text, cut)), in_block)| {
                let masked = if in_block
                    || key_payload.iter().any(|r| r.contains(&i))
                    || looks_like_key_payload(&text)
                {
                    text.chars().map(|_| '*').collect()
                } else {
                    masked_text(&text, cut)
                };
                PreviewLine {
                    number: i + 1,
                    text: window_around(&masked, 0, limits.line_chars),
                }
            })
            .collect())
    }
}

/// `text` with its secrets masked. `cut` is true when the read stopped inside the line: a secret
/// that goes past the stop (a JWT whose last part was cut off, a URL without its `@`) no longer has
/// its shape, so the last word is masked whole.
fn masked_text(text: &str, cut: bool) -> String {
    if !cut {
        return mask_line(text, &find_secrets(text));
    }
    let word = text.rfind(char::is_whitespace).map_or(0, |at| {
        at + text[at..].chars().next().map_or(1, char::len_utf8)
    });
    let (head, last) = text.split_at(word);
    let mut masked = mask_line(head, &find_secrets(head));
    masked.extend(last.chars().map(|_| '*'));
    masked
}

/// Which of `lines` (consecutive lines) are the payload of a private key: the lines after a
/// `-----BEGIN ... PRIVATE KEY-----` line up to the `-----END ... PRIVATE KEY-----` line, or to
/// the last line when no END is among them; and, when the END is there but no BEGIN, the lines
/// before it back to the start (or to the end of an earlier block). Short last lines count.
fn private_key_lines(lines: &[String]) -> Vec<bool> {
    let mut mask = vec![false; lines.len()];
    // The first line that may belong to a block that is open.
    let mut from = 0;
    let mut begun = false;
    for (i, line) in lines.iter().enumerate() {
        let private = line.contains("PRIVATE KEY");
        if line.contains("-----END ") {
            if private {
                mask[from..i].fill(true);
            }
            from = i + 1;
            begun = false;
        } else if line.contains("-----BEGIN ") {
            from = i + 1;
            begun = private;
        }
    }
    if begun {
        mask[from..].fill(true);
    }
    mask
}

/// A line of base64 of 60 characters or more without a space: what the lines of a PEM key look
/// like (they have 64 to 76).
fn looks_like_key_payload(line: &str) -> bool {
    line.len() >= 60
        && line
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
}

/// At most `max` characters of `text`, around the character index `at` (20 characters before it),
/// with `…` where text was left out.
fn window_around(text: &str, at: usize, max: usize) -> String {
    let total = text.chars().count();
    if total <= max {
        return text.to_owned();
    }
    let start = at.saturating_sub(20).min(total - max);
    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.extend(text.chars().skip(start).take(max));
    if start + max < total {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests;
