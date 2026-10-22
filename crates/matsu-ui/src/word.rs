//! The word under a double click.
//!
//! This is a cheap rule, not Sakura's: a word is a run of characters of one class (letters and
//! digits, hiragana, katakana, kanji, white space, or one other symbol). Sakura's word search
//! (`CWordParse`, `CNativeW::GetSizeOfChar` and the `m_bUseKeyWordSet` tables) also joins
//! things like `foo.bar` in some modes and splits kanji from okurigana differently; that comes
//! with the word commands (`F_WORDLEFT` and friends).

/// The class of a character for word selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Word,
    Hiragana,
    /// Katakana, including the long vowel mark.
    Katakana,
    Kanji,
    Space,
    /// Any other character; a run of the same character is one word, anything else is alone.
    Other(char),
}

fn class_of(c: char) -> Class {
    match c {
        ' ' | '\t' | '\u{3000}' => Class::Space,
        '_' => Class::Word,
        c if c.is_ascii_alphanumeric() => Class::Word,
        '\u{3041}'..='\u{309F}' => Class::Hiragana,
        '\u{30A0}'..='\u{30FF}' | '\u{31F0}'..='\u{31FF}' | '\u{FF66}'..='\u{FF9F}' => {
            Class::Katakana
        }
        '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}' | '々' => {
            Class::Kanji
        }
        c if c.is_alphanumeric() => Class::Word,
        c => Class::Other(c),
    }
}

/// The byte range of the word that contains the byte offset `col` of `line`. A click at the end
/// of the line takes the word before it; an empty line has an empty range.
pub fn word_range(line: &str, col: usize) -> (usize, usize) {
    if line.is_empty() {
        return (0, 0);
    }
    let col = col.min(line.len());
    // The char at `col`, or the last one when the click is past the end.
    let at = if col >= line.len() {
        line.char_indices().next_back().map_or(0, |(i, _)| i)
    } else {
        let mut i = col;
        while !line.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let Some(class) = line[at..].chars().next().map(class_of) else {
        return (0, 0);
    };
    let start = line[..at]
        .char_indices()
        .rev()
        .take_while(|&(_, c)| class_of(c) == class)
        .last()
        .map_or(at, |(i, _)| i);
    let end = line[at..]
        .char_indices()
        .find(|&(_, c)| class_of(c) != class)
        .map_or(line.len(), |(i, _)| at + i);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(line: &str, col: usize) -> &str {
        let (a, b) = word_range(line, col);
        &line[a..b]
    }

    #[test]
    fn selects_a_run_of_the_same_class() {
        assert_eq!(word("foo_bar baz", 0), "foo_bar");
        assert_eq!(word("foo_bar baz", 5), "foo_bar");
        assert_eq!(word("foo_bar baz", 7), " ");
        assert_eq!(word("foo_bar baz", 8), "baz");
        assert_eq!(word("foo.bar", 3), ".");
        assert_eq!(word("a  b", 1), "  ");
    }

    #[test]
    fn japanese_scripts_split_words() {
        let line = "今日はテストーです";
        assert_eq!(word(line, 0), "今日");
        assert_eq!(word(line, "今日".len()), "は");
        assert_eq!(word(line, "今日は".len()), "テストー");
        assert_eq!(word(line, line.len() - 3), "です");
    }

    #[test]
    fn the_end_of_the_line_and_the_middle_of_a_char() {
        assert_eq!(word("abc def", 7), "def");
        assert_eq!(word("日本", 1), "日本");
        assert_eq!(word("", 0), "");
        assert_eq!(word_range("", 5), (0, 0));
    }
}
