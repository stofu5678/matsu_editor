//! The text rules of one-liners (ADR-0029) that need no window and no process.
//!
//! A one-liner is a shell command line that gets the selection (else the whole document) on its
//! standard input and whose output replaces it, goes on the next line, or goes to the output
//! window. The process is `matsu_platform::exec` and the shell is `matsu_oneliner`; this module is
//! the text on both sides of it, written from ADR-0029 (no Sakura code: Sakura has no such
//! feature).
//!
//! * [`feed_lf`]: the input, read by lines with the line breaks made LF, in chunks. The document
//!   is never made into one string (CLAUDE.md).
//! * [`Collector`]: the output, decoded in pieces ([`TextDecoder`]; [`Utf8Decoder`] turns bytes
//!   that are not UTF-8 into U+FFFD), with CRLF, CR and LF made LF ([`LfNormalizer`]), kept until
//!   the process ends, up to a cap. The caller turns LF into the line break of the document.
//! * [`for_replace`] and [`for_next_line`]: what is done to the collected output at the end.
//!
//! The caps and why they are what they are: [`OUTPUT_CAP_BYTES`], [`ERROR_CAP_BYTES`].

use crate::buffer::{BufferError, Document, Pos};

/// How much decoded output is kept: 128 MiB.
///
/// The output is kept until the process ends (a failure must change nothing, ADR-0029), so
/// something has to bound it: `yes` or a runaway filter would use all the memory. The editor
/// opens files of this size, and putting the output into the document needs the output once more
/// plus the text it replaces for the undo, so this bounds the whole at a few times 128 MiB. A
/// filter that makes more than that from a selection is not one a one-liner is for.
pub const OUTPUT_CAP_BYTES: usize = 128 * 1024 * 1024;

/// How much of the standard error is kept: 64 KiB, the first part. The error is for the output
/// window, and a message that matters comes before a flood of the same line; the rest is dropped
/// ([`Collected::truncated`]).
pub const ERROR_CAP_BYTES: usize = 64 * 1024;

/// The document, or `range` of it, read by lines and given to `sink` in chunks of about
/// [`crate::exec::FEED_CHUNK_BYTES`], all line breaks LF. `sink` says whether to go on (`false`:
/// the command no longer reads). Whether the text ends with a line break is for the caller to see
/// in the chunks (the last chunk ends with `\n` or not).
pub fn feed_lf(
    doc: &Document,
    range: Option<(Pos, Pos)>,
    sink: impl FnMut(&str) -> bool,
) -> Result<(), BufferError> {
    crate::exec::read_chunks(doc, range, Some("\n"), sink)
}

/// Bytes of output in pieces to text.
pub trait TextDecoder {
    /// The text of what has come; what may continue in the next piece waits.
    fn push(&mut self, bytes: &[u8]) -> String;
    /// The text of what waits: the output has ended.
    fn finish(&mut self) -> String;
}

/// UTF-8 in pieces: a character cut between two pieces is put together, bytes that are not UTF-8
/// become U+FFFD, and a BOM at the very start is not text.
#[derive(Debug, Default)]
pub struct Utf8Decoder {
    held: Vec<u8>,
    started: bool,
}

impl Utf8Decoder {
    pub fn new() -> Self {
        Self::default()
    }

    fn without_bom(&mut self, text: String) -> String {
        if text.is_empty() || self.started {
            return text;
        }
        self.started = true;
        match text.strip_prefix('\u{FEFF}') {
            Some(rest) => rest.to_owned(),
            None => text,
        }
    }
}

impl TextDecoder for Utf8Decoder {
    fn push(&mut self, bytes: &[u8]) -> String {
        let mut data = std::mem::take(&mut self.held);
        data.extend_from_slice(bytes);
        let mut text = String::new();
        let mut rest = data.as_slice();
        while !rest.is_empty() {
            match std::str::from_utf8(rest) {
                Ok(valid) => {
                    text.push_str(valid);
                    break;
                }
                Err(error) => {
                    let (valid, bad) = rest.split_at(error.valid_up_to());
                    text.push_str(&String::from_utf8_lossy(valid));
                    match error.error_len() {
                        Some(len) => {
                            text.push('\u{FFFD}');
                            rest = &bad[len..];
                        }
                        None => {
                            // an unfinished character at the end: it waits
                            self.held = bad.to_vec();
                            break;
                        }
                    }
                }
            }
        }
        self.without_bom(text)
    }

    fn finish(&mut self) -> String {
        if std::mem::take(&mut self.held).is_empty() {
            String::new()
        } else {
            "\u{FFFD}".to_owned()
        }
    }
}

/// Makes CRLF, CR and LF into LF, in pieces: a CR at the end of a piece waits for the next piece
/// (it may be the first half of a CRLF).
#[derive(Debug, Default)]
pub struct LfNormalizer {
    pending_cr: bool,
}

impl LfNormalizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for c in text.chars() {
            if self.pending_cr {
                self.pending_cr = false;
                out.push('\n');
                if c == '\n' {
                    continue;
                }
            }
            if c == '\r' {
                self.pending_cr = true;
            } else {
                out.push(c);
            }
        }
        out
    }

    /// What waits at the end of the output: a CR is a line break.
    pub fn finish(&mut self) -> String {
        if std::mem::take(&mut self.pending_cr) {
            "\n".to_owned()
        } else {
            String::new()
        }
    }
}

/// What the collector does with output past its cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overflow {
    /// It is an error ([`TooLarge`]): the output must be whole or not used.
    Fail,
    /// The rest is dropped and [`Collected::truncated`] says so.
    Truncate,
}

/// The output is over the cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TooLarge;

impl std::fmt::Display for TooLarge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the output is too large")
    }
}

impl std::error::Error for TooLarge {}

/// What a [`Collector`] kept.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Collected {
    /// The text in pieces, all line breaks LF.
    pub pieces: Vec<String>,
    /// Some was dropped ([`Overflow::Truncate`]).
    pub truncated: bool,
}

impl Collected {
    /// The pieces as one string (for the error text and for tests; the output of a large run is
    /// kept in pieces on purpose).
    pub fn text(&self) -> String {
        self.pieces.concat()
    }
}

/// The bytes of an output or an error, decoded in pieces, made LF, and kept up to a cap.
pub struct Collector {
    decoder: Box<dyn TextDecoder + Send>,
    lf: LfNormalizer,
    pieces: Vec<String>,
    kept: usize,
    cap: usize,
    overflow: Overflow,
    truncated: bool,
}

impl Collector {
    pub fn new(decoder: Box<dyn TextDecoder + Send>, cap: usize, overflow: Overflow) -> Self {
        Self {
            decoder,
            lf: LfNormalizer::new(),
            pieces: Vec::new(),
            kept: 0,
            cap,
            overflow,
            truncated: false,
        }
    }

    /// Takes bytes of the output. `Err` when [`Overflow::Fail`] and the cap is passed; the
    /// collector is of no use then.
    pub fn push(&mut self, bytes: &[u8]) -> Result<(), TooLarge> {
        let text = self.decoder.push(bytes);
        self.keep(&text)
    }

    fn keep(&mut self, text: &str) -> Result<(), TooLarge> {
        let text = self.lf.push(text);
        self.add(text)
    }

    fn add(&mut self, mut text: String) -> Result<(), TooLarge> {
        if text.is_empty() || self.truncated {
            return Ok(());
        }
        let room = self.cap.saturating_sub(self.kept);
        if text.len() > room {
            match self.overflow {
                Overflow::Fail => return Err(TooLarge),
                Overflow::Truncate => {
                    let mut at = room;
                    while !text.is_char_boundary(at) {
                        at -= 1;
                    }
                    text.truncate(at);
                    self.truncated = true;
                }
            }
        }
        self.kept += text.len();
        if !text.is_empty() {
            self.pieces.push(text);
        }
        Ok(())
    }

    /// The output has ended: what waits is decoded too.
    pub fn finish(mut self) -> Result<Collected, TooLarge> {
        let rest = self.decoder.finish();
        self.keep(&rest)?;
        let rest = self.lf.finish();
        self.add(rest)?;
        Ok(Collected {
            pieces: self.pieces,
            truncated: self.truncated,
        })
    }
}

/// The last character of the nonempty pieces.
fn last_char(pieces: &[String]) -> Option<char> {
    pieces.iter().rev().find_map(|p| p.chars().next_back())
}

/// Replace (ADR-0029, 置き換える): when the input did not end with a line break and the output
/// does, exactly one is dropped (`sort` and PowerShell add one that the selection did not have).
pub fn for_replace(pieces: &mut Vec<String>, input_ended_with_break: bool) {
    if input_ended_with_break || last_char(pieces) != Some('\n') {
        return;
    }
    while let Some(last) = pieces.last_mut() {
        if last.pop().is_some() {
            if last.is_empty() {
                pieces.pop();
            }
            return;
        }
        pieces.pop();
    }
}

/// Next line (ADR-0029, 次の行に入れる): the output is lines, so a line break is added when the
/// output has none at its end. No output stays no output (a line break alone would put an empty
/// line into the document).
pub fn for_next_line(pieces: &mut Vec<String>) {
    match last_char(pieces) {
        None | Some('\n') => {}
        Some(_) => pieces.push("\n".to_owned()),
    }
}

#[cfg(test)]
mod tests;
