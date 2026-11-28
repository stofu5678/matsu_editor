//! Grep 置換 of a file of the size of a huge file or more: the file is read line by line in order
//! and the new file is written as it goes (ADR-0020, chapter 5), so that the memory in use is that
//! of the longest line and a chunk, not of the file.
//!
//! The lines are handled by [`Runner::replace_line`], the same code as for a document: the same
//! hits in the result, the same matches and replacements. What is written is what the save of a
//! document writes after the same edits ([`Runner::plan_replacements`], [`super::content_edit`]):
//!
//! * a line without a change is copied byte for byte, with its break;
//! * a line whose break stays and whose new text has no break in it keeps what the line has at
//!   its start and its end (what the old and the new text share), and only the middle is
//!   encoded;
//! * any other changed line is encoded whole, with its new break.
//!
//! The new file is not begun until the first change: the lines before it are copied from the
//! file when it comes, so a file without a hit is read and nothing is written. A line over
//! [`LONG_LINE_LIMIT`](super::super::LONG_LINE_LIMIT) is not looked at (as in a Grep) and is copied
//! as it is: the result says so (type C). A stop, a character that the encoding cannot write, a
//! full disk and every other failure drop the new file and leave the old one as it was.

use std::io::{self, Write};
use std::sync::atomic::Ordering;

use matsu_core::buffer::Codec;
use matsu_core::encoding::Encoding;
use matsu_core::file::{FileError, Limited, LineReader, StreamWrite};

use super::{
    LineEnv, LineOutcome, LineRef, LineWork, ReplaceInfo, Staged, content_edit, copy_stoppable,
};
use crate::grep::engine::{CANCEL_CHECK_LINES, Runner, Stopped, line_limit};
use crate::grep::format::{self, FilePaths, PathInfoState};

/// Why the new file is not made: the user stopped it, or the result already tells why.
enum Abort {
    Stopped,
    Told,
}

impl From<Stopped> for Abort {
    fn from(_: Stopped) -> Self {
        Self::Stopped
    }
}

/// The new file, and the copy of the old one for the backup, once they are begun.
#[derive(Default)]
struct Output {
    writer: Option<StreamWrite>,
    staged: Option<Staged>,
}

/// Where the bytes of a line that is too long go while the reader drops it: into the new file,
/// when it is begun (when it is not, the copy of the start of the file will have them).
struct Spill<'a> {
    to: Option<&'a mut StreamWrite>,
    /// The kind of the error of the write that failed. The reader gets the error itself and
    /// returns it, and an `io::Error` cannot be copied whole; what the caller needs is that it
    /// was the write to the new file that failed, and not the read or a stop (a write can fail
    /// with the kind `Interrupted` too), so the kind is all that is kept.
    error: Option<io::ErrorKind>,
}

impl Spill<'_> {
    fn take_error(&mut self) -> Option<io::ErrorKind> {
        self.error.take()
    }
}

impl Write for Spill<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(to) = &mut self.to
            && let Err(e) = to.write_all(buf)
        {
            self.error = Some(e.kind());
            return Err(e);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Runner<'_> {
    /// `DoGrepReplaceFile` for a huge file (`from` bytes or more), read in order.
    pub(super) fn replace_file_stream(
        &mut self,
        paths: &FilePaths<'_>,
        state: &mut PathInfoState,
        from: u64,
        replace: &ReplaceInfo,
    ) -> Result<(), Stopped> {
        let Some(matcher) = self.matcher.clone() else {
            return Ok(());
        };
        let stream = match self.open_stream(paths.full, from) {
            Ok(stream) => stream,
            Err(reason) => {
                // Not read whole instead, as in a Grep: that could use all the memory.
                self.buffer.push_str(&format::file_open_error(paths.full));
                self.buffer
                    .push_str(&format!("  ({reason}){}", format::EOL));
                return Ok(());
            }
        };
        let encoding = stream.encoding;
        let bom_len = if stream.bom { encoding.bom().len() } else { 0 };
        let code_name = self.code_name(encoding);
        let env = LineEnv {
            matcher: matcher.as_ref(),
            paths,
            code_name: &code_name,
            replace,
        };
        let file_len = stream.reader().total() + bom_len as u64;
        let mut output = Output::default();
        let run = self.stream_lines(
            stream.into_reader(),
            &env,
            state,
            (encoding, bom_len, file_len),
            &mut output,
        );
        match run {
            Ok(replaced) if replaced > 0 => {
                if self.commit_stream(paths.full, replace.backup, output)? {
                    self.hits += replaced;
                }
                Ok(())
            }
            // Nothing is replaced: nothing was written.
            Ok(_) => Ok(()),
            // The new file and the copy of the old one go with `output`.
            Err(Abort::Stopped) => Err(Stopped),
            Err(Abort::Told) => Ok(()),
        }
    }

    /// The loop over the lines. Returns the number of replacements, with the new file made in
    /// `output` unless there are none.
    fn stream_lines(
        &mut self,
        mut reader: LineReader,
        env: &LineEnv<'_, '_>,
        state: &mut PathInfoState,
        (encoding, bom_len, file_len): (Encoding, usize, u64),
        output: &mut Output,
    ) -> Result<usize, Abort> {
        let cancel = self.cancel;
        let limit = line_limit();
        let codec = encoding.codec();
        let path = env.paths.full;
        let mut work = LineWork::default();
        let mut scratch = Vec::new();
        let mut replaced = 0usize;
        let mut number = 0u64;
        let mut refused = false;
        loop {
            if number.is_multiple_of(CANCEL_CHECK_LINES as u64) {
                self.check_cancel()?;
            }
            let before = reader.consumed();
            let mut spill = Spill {
                to: output.writer.as_mut(),
                error: None,
            };
            let next = reader.next_line_spilling(
                limit,
                &mut || cancel.load(Ordering::Relaxed),
                &mut spill,
            );
            let spill_error = spill.take_error();
            let line = match next {
                Ok(Some(Limited::Line(line))) => line,
                Ok(Some(Limited::TooLong { bytes, .. })) => {
                    number += 1;
                    // Not looked at, and copied as it is (the reader wrote it to the new file).
                    self.buffer.push_str(&format::line_too_long_kept(
                        path,
                        number,
                        bytes,
                        limit as u64,
                    ));
                    continue;
                }
                Ok(None) => break,
                Err(_) if spill_error.is_some() => {
                    self.buffer.push_str(&format::file_write_error(path));
                    return Err(Abort::Told);
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                    return Err(Abort::Stopped);
                }
                Err(_) => {
                    self.buffer.push_str(&format::file_read_error(path));
                    return Err(Abort::Told);
                }
            };
            number += 1;
            let eol = line.eol.map_or("", matsu_core::model::eol_text);
            let outcome = self.replace_line(
                &mut work,
                env,
                state,
                LineRef {
                    number,
                    text: &line.text,
                    eol,
                },
                None,
            );
            let raw = reader.last_raw();
            let changed = match outcome {
                LineOutcome::Failed => return Err(Abort::Told),
                LineOutcome::NoHit => false,
                LineOutcome::Hit {
                    replaced: count,
                    changed,
                } => {
                    replaced += count;
                    changed
                }
            };
            if changed && output.writer.is_none() && !refused {
                self.begin_stream(
                    path,
                    env.replace.backup,
                    bom_len as u64 + before,
                    file_len,
                    output,
                )?;
            }
            if let Some(writer) = &mut output.writer {
                let written = if changed {
                    write_changed(
                        codec,
                        writer,
                        raw,
                        &line.text,
                        eol,
                        &work.new_whole,
                        &mut scratch,
                    )
                } else {
                    writer.write_all(raw)
                };
                match written {
                    Ok(()) => {}
                    // A character that the encoding cannot write: the file is not written, but
                    // the lines go on, so that the result shows the hits of the whole file as
                    // for a document.
                    Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                        refused = true;
                        // The new file and the copy of the old one (as large as the file) are
                        // not needed any more: they go now, not when the rest has been read.
                        output.writer = None;
                        output.staged = None;
                    }
                    Err(_) => {
                        self.buffer.push_str(&format::file_write_error(path));
                        return Err(Abort::Told);
                    }
                }
            }
            self.flush_when_due();
        }
        #[cfg(test)]
        crate::grep::engine::stream_probe::record(&reader);
        // Hits that change nothing still make the file again, as for a document: it is the same
        // file, and the backup is made.
        if refused {
            self.buffer.push_str(&format::file_write_error(path));
            return Err(Abort::Told);
        }
        if replaced > 0 && output.writer.is_none() {
            self.begin_stream(path, env.replace.backup, file_len, file_len, output)?;
        }
        Ok(replaced)
    }

    /// Begins the new file: the backup copy of the old one, then the new file with the first
    /// `prefix` bytes of the old one (the lines before the first change, and the BOM).
    fn begin_stream(
        &mut self,
        path: &str,
        backup: bool,
        prefix: u64,
        needed: u64,
        output: &mut Output,
    ) -> Result<(), Abort> {
        if backup {
            let Some(staged) = self.stage_backup(path)? else {
                return Err(Abort::Told);
            };
            output.staged = Some(staged);
        }
        let mut writer = match StreamWrite::create(std::path::Path::new(path), needed, None) {
            Ok(writer) => writer,
            Err(error) => {
                self.report_write_error(path, &error);
                return Err(Abort::Told);
            }
        };
        let copied = std::fs::File::open(path)
            .and_then(|mut file| copy_stoppable(&mut file, &mut writer, prefix, self.cancel));
        match copied {
            Ok(n) if n == prefix => {}
            Err(e) if e.kind() == io::ErrorKind::Interrupted => return Err(Abort::Stopped),
            _ => {
                self.buffer.push_str(&format::file_write_error(path));
                return Err(Abort::Told);
            }
        }
        output.writer = Some(writer);
        Ok(())
    }

    /// Puts the new file in the place of the old one, and keeps the backup. Whether the file was
    /// replaced; what went wrong is told in the result. A stop, also one that comes while the
    /// new file is flushed, leaves the old file as it was.
    fn commit_stream(
        &mut self,
        path: &str,
        backup: bool,
        mut output: Output,
    ) -> Result<bool, Stopped> {
        let Some(writer) = output.writer.take() else {
            return Ok(false);
        };
        let cancel = self.cancel;
        match writer.commit(&mut || cancel.load(Ordering::Relaxed)) {
            Ok(_) => {
                if backup && let Some(staged) = output.staged.take() {
                    let (old, staged) = staged.release();
                    self.keep_backup(&staged, &old);
                }
                Ok(true)
            }
            Err(FileError::Cancelled) => Err(Stopped),
            Err(error) => {
                self.report_write_error(path, &error);
                Ok(false)
            }
        }
    }
}

/// The error of a line whose new text the encoding cannot write.
fn unmappable() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "a character that the encoding cannot write",
    )
}

/// Writes a changed line (see the module documentation for the three ways).
fn write_changed(
    codec: &dyn Codec,
    out: &mut StreamWrite,
    raw: &[u8],
    text: &str,
    eol: &str,
    new_whole: &str,
    scratch: &mut Vec<u8>,
) -> io::Result<()> {
    scratch.clear();
    match content_edit(text, eol, new_whole) {
        Some((from, to, middle)) => {
            // both ends from the start of the line, as the document finds the ends of an edit
            let head = raw_len_of(codec, raw, from);
            let cut = raw_len_of(codec, raw, to);
            codec.encode(middle, scratch).map_err(|_| unmappable())?;
            out.write_all(&raw[..head])?;
            out.write_all(scratch)?;
            out.write_all(&raw[cut..])
        }
        None => {
            codec.encode(new_whole, scratch).map_err(|_| unmappable())?;
            out.write_all(scratch)
        }
    }
}

/// The bytes of `raw` before the place `decoded` bytes into its text, found as the document does
/// (`Document::raw_for_col`): character by character (the codec is stateless), at the start of
/// the character that holds the place. A character of the file can decode to more than one
/// character of the text (invalid bytes), and a place between them is rounded down, not up.
fn raw_len_of(codec: &dyn Codec, raw: &[u8], decoded: usize) -> usize {
    let mut text = String::new();
    let (mut at, mut seen) = (0, 0);
    while at < raw.len() {
        let n = codec.char_len(raw, at).clamp(1, raw.len() - at);
        text.clear();
        codec.decode(&raw[at..at + n], &mut text);
        if seen + text.len() > decoded {
            break;
        }
        seen += text.len();
        at += n;
    }
    at
}
