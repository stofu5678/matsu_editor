//! `-GOPT=U`: the Grep of the command line writes its result to the standard output of the process
//! instead of a window, and the editor ends (`CGrepAgent::AddTail` with `bAddStdout`,
//! `CNormalProcess::Initialize`, which posts `MYWM_CLOSE` with `PM_CLOSE_GREPNOCONFIRM` when
//! `bGrepStdout` is set and the dialog was not asked for).
//!
//! * The text is written in the encoding of the document of the window (`GetDocumentEncoding`),
//!   a piece at a time as the engine produces it.
//! * Sakura gets the handle with `GetStdHandle` and writes only if there is one. A GUI program on
//!   Windows has a handle when the caller redirected it (`matsu ... > out.txt`, a pipe); without
//!   one the standard library's `stdout` does nothing, which is the same.
//! * A mistake that stops the search is written as a line too (`STR_DLGGREP5` for a missing
//!   folder, in Sakura, where a message box would stop a batch); the other mistakes, which Sakura
//!   tells in a message box, are written the same way, so that a batch does not wait for a box.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui::Context;
use matsu_core::encoding::Encoding;

use super::engine::{self, GrepInfo, Sink};
use crate::view::EditorView;

/// The sink that encodes and writes. A write that fails (the reader of a pipe is gone) stops the
/// search through `cancel`.
struct WriteSink<'a, W: Write> {
    out: W,
    encoding: Encoding,
    cancel: &'a AtomicBool,
    bytes: Vec<u8>,
}

impl<W: Write> Sink for WriteSink<'_, W> {
    fn write(&mut self, text: &str) {
        self.bytes.clear();
        // A character the encoding cannot hold is written as Sakura writes it (`UnicodeToCode`):
        // the error only lists them.
        let _ = self.encoding.codec().encode(text, &mut self.bytes);
        if self.out.write_all(&self.bytes).is_err() {
            self.cancel.store(true, Ordering::Relaxed);
        }
    }
}

/// Runs the Grep and writes its result to `out`, in `encoding`.
pub(crate) fn write_grep<W: Write>(
    info: &GrepInfo,
    encoding: Encoding,
    cancel: &AtomicBool,
    out: W,
) {
    let mut sink = WriteSink {
        out,
        encoding,
        cancel,
        bytes: Vec::new(),
    };
    if let Err(error) = engine::run(info, cancel, &mut sink) {
        sink.write(&format!("{}\r\n", error.message()));
    }
    let _ = sink.out.flush();
}

impl EditorView {
    /// `-GREPMODE` with `-GOPT=U`: searches on a background thread, writes to the standard output,
    /// and ends the application when it is done.
    pub(crate) fn start_grep_stdout(&mut self, mut info: GrepInfo, cx: &mut Context<Self>) {
        let prefs = crate::file_ops::prefs::prefs(cx);
        info.mime_decode = prefs.auto_mime_decode;
        info.huge_file_len = Some(prefs.huge_file_len());
        // `:HWND:` names a window of another process: it is gone from here (`GetHwndTitle`).
        if super::window::parse_window_token(&info.file).is_some() {
            info.window_gone = true;
        }
        let encoding = self.document.read(cx).file().encoding;
        let task = cx.background_executor().spawn(async move {
            let cancel = AtomicBool::new(false);
            write_grep(&info, encoding, &cancel, std::io::stdout().lock());
        });
        cx.spawn(async move |_, cx| {
            task.await;
            // `PM_CLOSE_GREPNOCONFIRM | PM_CLOSE_EXIT`: no question, and the process ends.
            cx.update(crate::file_ops::close::quit_at_once);
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grep::engine::GrepInfo;
    use crate::grep::format::Layout;
    use crate::search::state::SearchOptions;

    fn info(folder: &str, key: &str) -> GrepInfo {
        GrepInfo {
            key: key.to_owned(),
            file: "*.txt".to_owned(),
            folder: folder.to_owned(),
            options: SearchOptions::default(),
            sub_folder: true,
            charset: None,
            layout: Layout::default(),
            file_only: false,
            except_file_regex: false,
            replace: None,
            mime_decode: false,
            huge_file_len: None,
            window: None,
            window_gone: false,
        }
    }

    #[test]
    fn the_result_is_written_in_the_encoding_of_the_window() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "あいう\r\n").unwrap();
        let folder = dir.path().to_string_lossy().into_owned();
        let mut out = Vec::new();
        write_grep(
            &info(&folder, "い"),
            Encoding::ShiftJis,
            &AtomicBool::new(false),
            &mut out,
        );
        // Shift_JIS, not UTF-8: "あいう" is 82 A0 82 A2 82 A4
        assert!(
            out.windows(6)
                .any(|w| w == [0x82, 0xA0, 0x82, 0xA2, 0x82, 0xA4]),
            "{out:?}"
        );
        assert!(String::from_utf8(out.clone()).is_err());
        let (text, _, _) = encoding_rs::SHIFT_JIS.decode(&out);
        assert!(text.contains("□検索条件  \"い\""), "{text}");
        assert!(text.ends_with("1 個が検索されました。\r\n"), "{text}");
    }

    #[test]
    fn a_folder_that_is_not_there_is_written_as_a_line() {
        let mut out = Vec::new();
        write_grep(
            &info("/no/such/folder/for/matsu", "x"),
            Encoding::Utf8,
            &AtomicBool::new(false),
            &mut out,
        );
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "検索対象フォルダーが正しくありません。\r\n"
        );
    }

    #[test]
    fn a_reader_that_is_gone_stops_the_search() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "x\r\n").unwrap();
        let cancel = AtomicBool::new(false);
        write_grep(
            &info(&dir.path().to_string_lossy(), "x"),
            Encoding::Utf8,
            &cancel,
            Broken,
        );
        assert!(cancel.load(Ordering::Relaxed));
    }
}
