//! The records of the AI log tab (ADR-0030 decision 6), as plain data: what was sent to an AI, what came back, and
//! the text the tab shows for them.
//!
//! The records are kept in memory only. Nothing here reads or writes a file, and the types are not serializable:
//! the log is not part of the autosave of unsaved documents (ADR-0022) or of any saved state, and it is gone when
//! the tab or the app closes (ADR-0028 decision 3's way of thinking: the output of a tool is not written to disk).

use std::collections::VecDeque;
use std::time::Duration;

use chrono::{DateTime, Local};

/// How many records the log keeps. The oldest ones are dropped (ADR-0030 案 6 says "about 50").
pub(crate) const MAX_RECORDS: usize = 50;

/// How many bytes of one text of a record (the command line, the input, the answer, stderr) are kept.
///
/// 256 KiB is what the preview tab shows of a document ([`crate::preview::text::MAX_BYTES`]) through the same text
/// view, so it is known to draw. The worst case is `MAX_RECORDS` records of four such texts, 50 MiB; an ordinary
/// prompt and answer are a few KiB. The log is for reading what was sent and what came back, not for storing it,
/// and the text that was really sent is in the document.
pub(crate) const MAX_TEXT_BYTES: usize = 256 << 10;

/// The name of one record, unique in a log. A record that was dropped by the limit keeps its number: it is never
/// given to another record, so a late `finish` cannot land on the wrong one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RecordId(pub(crate) u64);

/// A text of a record, cut to [`MAX_TEXT_BYTES`] if it was longer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LogText {
    text: String,
    /// The size of the text before it was cut, when it was cut.
    cut_from: Option<usize>,
}

impl LogText {
    pub(crate) fn new(text: String) -> Self {
        Self::sized(text, None)
    }

    /// `text` is the head of a text that has `whole` bytes (`None`: `text` is all of it).
    pub(crate) fn sized(mut text: String, whole: Option<usize>) -> Self {
        let original = whole.unwrap_or(text.len()).max(text.len());
        if original <= MAX_TEXT_BYTES {
            return Self {
                text,
                cut_from: None,
            };
        }
        if text.len() > MAX_TEXT_BYTES {
            let mut end = MAX_TEXT_BYTES;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
        }
        // `truncate` keeps the allocation of the whole text; the limit is for the memory the log holds.
        text.shrink_to_fit();
        Self {
            text,
            cut_from: Some(original),
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }

    /// The size before the cut, or `None` if the text is whole.
    pub(crate) fn cut_from(&self) -> Option<usize> {
        self.cut_from
    }
}

/// What the caller knows when a run starts.
#[derive(Clone, Debug)]
pub(crate) struct Started {
    /// The name of the one-liner (the name the user gave it).
    pub(crate) name: String,
    /// The command line as it is run.
    pub(crate) command_line: String,
    /// The text sent to the command's stdin.
    pub(crate) input: String,
    /// The size of the whole input when `input` is only its head (a text that is read up to a limit); `None` when
    /// `input` is all of it.
    pub(crate) input_size: Option<usize>,
    pub(crate) started_at: DateTime<Local>,
}

/// What the caller knows when the run ends.
#[derive(Clone, Debug, Default)]
pub(crate) struct Finished {
    /// The answer: the command's stdout.
    pub(crate) output: String,
    pub(crate) stderr: String,
    /// `None` when the command did not start, or was ended by a signal.
    pub(crate) exit_code: Option<i32>,
    pub(crate) elapsed: Duration,
}

/// One run.
#[derive(Clone, Debug)]
pub(crate) struct Record {
    pub(crate) id: RecordId,
    pub(crate) name: String,
    pub(crate) command_line: LogText,
    pub(crate) input: LogText,
    pub(crate) started_at: DateTime<Local>,
    /// `None` while the run goes on.
    pub(crate) result: Option<RecordResult>,
}

/// How a run ended.
#[derive(Clone, Debug)]
pub(crate) struct RecordResult {
    pub(crate) output: LogText,
    pub(crate) stderr: LogText,
    pub(crate) exit_code: Option<i32>,
    pub(crate) elapsed: Duration,
}

/// The records of one window, the newest last.
#[derive(Debug, Default)]
pub(crate) struct Log {
    records: VecDeque<Record>,
    next_id: u64,
}

impl Log {
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }

    #[cfg(test)]
    pub(crate) fn records(&self) -> impl Iterator<Item = &Record> {
        self.records.iter()
    }

    #[cfg(test)]
    pub(crate) fn get(&self, id: RecordId) -> Option<&Record> {
        self.records.iter().find(|record| record.id == id)
    }

    /// Adds a running record at the end; the oldest is dropped when there are more than [`MAX_RECORDS`].
    pub(crate) fn begin(&mut self, started: Started) -> RecordId {
        let id = RecordId(self.next_id);
        self.next_id += 1;
        self.records.push_back(Record {
            id,
            name: started.name,
            command_line: LogText::new(started.command_line),
            input: LogText::sized(started.input, started.input_size),
            started_at: started.started_at,
            result: None,
        });
        while self.records.len() > MAX_RECORDS {
            self.records.pop_front();
        }
        id
    }

    /// Ends record `id`. Returns `false` when the record is not there (dropped by the limit) or has ended.
    pub(crate) fn finish(&mut self, id: RecordId, finished: Finished) -> bool {
        let Some(record) = self.records.iter_mut().find(|record| record.id == id) else {
            return false;
        };
        if record.result.is_some() {
            return false;
        }
        record.result = Some(RecordResult {
            output: LogText::new(finished.output),
            stderr: LogText::new(finished.stderr),
            exit_code: finished.exit_code,
            elapsed: finished.elapsed,
        });
        true
    }

    /// The text of the whole log, as Markdown for the text view.
    pub(crate) fn markdown(&self) -> String {
        if self.records.is_empty() {
            return "まだ記録がありません。".to_owned();
        }
        let mut out = String::new();
        for (number, record) in self.records.iter().enumerate() {
            if number > 0 {
                out.push_str("\n---\n\n");
            }
            write_record(&mut out, record);
        }
        out
    }
}

fn write_record(out: &mut String, record: &Record) {
    out.push_str(&format!(
        "## {} (#{})\n\n",
        escape_inline(&record.name),
        record.id.0 + 1
    ));
    out.push_str(&format!(
        "開始: {}\n\n",
        record.started_at.format("%Y-%m-%d %H:%M:%S")
    ));
    match &record.result {
        None => out.push_str("状態: **実行中**\n\n"),
        Some(result) => {
            let code = match result.exit_code {
                Some(code) => format!("終了コード {code}"),
                None => "終了コードなし".to_owned(),
            };
            out.push_str(&format!(
                "状態: {code}、{}\n\n",
                format_elapsed(result.elapsed)
            ));
        }
    }
    write_text(out, "コマンド行", &record.command_line);
    write_text(out, "入力", &record.input);
    if let Some(result) = &record.result {
        write_text(out, "答え（標準出力）", &result.output);
        write_text(out, "標準エラー", &result.stderr);
    }
}

fn write_text(out: &mut String, label: &str, text: &LogText) {
    out.push_str(&format!("**{label}**"));
    if text.cut_from().is_none() {
        out.push_str(&format!("（{} 文字）", text.as_str().chars().count()));
    }
    out.push_str("\n\n");
    if let Some(original) = text.cut_from() {
        out.push_str(&format!(
            "*長いので、先頭の {} KiB だけを残しました（元は {original} バイト）。*\n\n",
            MAX_TEXT_BYTES / 1024
        ));
    }
    if text.as_str().is_empty() {
        out.push_str("（なし）\n\n");
        return;
    }
    // Only the line endings are changed, for the text view; the record keeps the text as it was.
    let shown = text.as_str().replace("\r\n", "\n").replace('\r', "\n");
    let fence = "`".repeat(longest_backtick_run(&shown).max(2) + 1);
    out.push_str(&fence);
    out.push('\n');
    out.push_str(&shown);
    if !shown.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&fence);
    out.push_str("\n\n");
}

fn longest_backtick_run(text: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    longest
}

/// `text` for a line of Markdown that is not to be read as Markdown.
fn escape_inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\r' | '\n' => out.push(' '),
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '#' | '|' | '~' | '&' | '!' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

fn format_elapsed(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs_f64();
    if seconds < 60.0 {
        format!("{seconds:.2} 秒")
    } else {
        let whole = elapsed.as_secs();
        format!("{} 分 {} 秒", whole / 60, whole % 60)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone as _;

    use super::*;

    fn at() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 13, 9, 5, 7).unwrap()
    }

    fn started(name: &str) -> Started {
        Started {
            name: name.to_owned(),
            command_line: "claude -p".to_owned(),
            input: "こんにちは".to_owned(),
            input_size: None,
            started_at: at(),
        }
    }

    fn finished(output: &str) -> Finished {
        Finished {
            output: output.to_owned(),
            stderr: String::new(),
            exit_code: Some(0),
            elapsed: Duration::from_millis(1250),
        }
    }

    #[test]
    fn a_record_is_added_running_and_ended_in_place() {
        let mut log = Log::default();
        let first = log.begin(started("要約"));
        let second = log.begin(started("翻訳"));
        assert_eq!(log.len(), 2);
        assert!(log.get(first).unwrap().result.is_none());
        let running = log.markdown();
        assert!(running.contains("**実行中**"), "{running}");
        assert!(running.contains("claude -p"));
        // The answer is not there before the run ends.
        assert!(!running.contains("答え"));

        assert!(log.finish(second, finished("Hello")));
        let order: Vec<&str> = log.records().map(|r| r.name.as_str()).collect();
        assert_eq!(order, ["要約", "翻訳"], "the newest is last");
        let result = log.get(second).unwrap().result.as_ref().unwrap();
        assert_eq!(result.output.as_str(), "Hello");
        assert_eq!(result.exit_code, Some(0));
        let text = log.markdown();
        assert!(text.contains("終了コード 0、1.25 秒"), "{text}");
        assert!(text.contains("Hello"));
        // The first is still running.
        assert!(text.contains("**実行中**"));
        // A run ends once.
        assert!(!log.finish(second, finished("again")));
    }

    #[test]
    fn it_keeps_the_last_fifty_records() {
        let mut log = Log::default();
        let ids: Vec<RecordId> = (0..MAX_RECORDS + 3)
            .map(|n| log.begin(started(&format!("run{n}"))))
            .collect();
        assert_eq!(log.len(), MAX_RECORDS);
        assert_eq!(log.records().next().unwrap().name, "run3");
        assert_eq!(log.records().last().unwrap().name, "run52");
        // The end of a dropped run is ignored, and the numbers are not reused.
        assert!(!log.finish(ids[0], finished("late")));
        let next = log.begin(started("next"));
        assert_eq!(next, RecordId(MAX_RECORDS as u64 + 3));
        assert!(log.get(ids[0]).is_none());
    }

    #[test]
    fn a_long_text_is_cut_on_a_character_boundary_and_says_so() {
        // Three bytes per character: the limit falls inside a character.
        let text = "あ".repeat(MAX_TEXT_BYTES);
        let cut = LogText::new(text.clone());
        assert!(cut.as_str().len() <= MAX_TEXT_BYTES);
        assert!(cut.as_str().len() > MAX_TEXT_BYTES - 3);
        assert_eq!(cut.cut_from(), Some(text.len()));
        assert!(text.starts_with(cut.as_str()));
        // The memory of the cut part is given back, not only the length.
        assert!(cut.text.capacity() < text.len() / 2);
        // A text of exactly the limit is whole.
        let whole = LogText::new("a".repeat(MAX_TEXT_BYTES));
        assert_eq!(whole.cut_from(), None);

        let mut log = Log::default();
        let id = log.begin(started("長い"));
        log.finish(
            id,
            Finished {
                output: text.clone(),
                ..finished("")
            },
        );
        let shown = log.markdown();
        assert!(shown.contains("長いので、先頭の 256 KiB だけを残しました"));
        assert!(shown.contains(&format!("元は {} バイト", text.len())));
    }

    #[test]
    fn a_head_that_was_read_up_to_a_limit_reports_the_size_of_the_whole() {
        let head = "a".repeat(MAX_TEXT_BYTES + 10);
        let cut = LogText::sized(head, Some(5_000_000));
        assert_eq!(cut.as_str().len(), MAX_TEXT_BYTES);
        assert_eq!(cut.cut_from(), Some(5_000_000));
        // a whole text that is short stays whole, whatever the size says
        let whole = LogText::sized("abc".to_owned(), None);
        assert_eq!(whole.cut_from(), None);
    }

    #[test]
    fn the_command_line_is_cut_too() {
        let mut log = Log::default();
        let mut s = started("x");
        s.command_line = "a".repeat(MAX_TEXT_BYTES + 1);
        let id = log.begin(s);
        assert_eq!(
            log.get(id).unwrap().command_line.cut_from(),
            Some(MAX_TEXT_BYTES + 1)
        );
    }

    #[test]
    fn text_with_backticks_and_markdown_stays_text() {
        let mut log = Log::default();
        let mut s = started("*強調* [x]");
        s.input = "```rust\nfn main() {}\n```\n# 見出し\r\n行".to_owned();
        let id = log.begin(s);
        log.finish(id, finished("答え"));
        let md = log.markdown();
        // The name is escaped, so it cannot turn into emphasis or a link.
        assert!(md.contains(r"## \*強調\* \[x\] (#1)"), "{md}");
        // The fence is longer than the longest run in the text.
        assert!(
            md.contains("````\n```rust\nfn main() {}\n```\n# 見出し\n行\n````\n"),
            "{md}"
        );
        // Line endings are shown as line feeds, the record keeps its own.
        assert!(log.get(id).unwrap().input.as_str().contains("\r\n"));
    }

    #[test]
    fn an_empty_text_and_a_missing_exit_code_are_said() {
        let mut log = Log::default();
        assert_eq!(log.markdown(), "まだ記録がありません。");
        let id = log.begin(started("x"));
        log.finish(
            id,
            Finished {
                output: String::new(),
                stderr: "spawn failed".to_owned(),
                exit_code: None,
                elapsed: Duration::from_secs(75),
            },
        );
        let md = log.markdown();
        assert!(md.contains("終了コードなし、1 分 15 秒"), "{md}");
        assert!(md.contains("（なし）"));
        assert!(md.contains("spawn failed"));
        assert!(md.contains("開始: 2026-11-13 09:05:07"));
    }
}
