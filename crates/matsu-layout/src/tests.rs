//! Tests of the layout. The expectations come from reading
//! `sakura_core/doc/layout/CLayoutMgr_DoLayout.cpp` (the line numbers in the comments are of the
//! upstream commit 48db6c7c25e2); Sakura has no unit tests for the layout.

use super::*;

fn config(columns: u32) -> LayoutConfig {
    LayoutConfig {
        wrap: Wrap::Columns(columns),
        ..LayoutConfig::default()
    }
}

fn lay(text: &str, has_eol: bool, cfg: &LayoutConfig) -> Vec<Row> {
    let mut rows = Vec::new();
    layout_line(text, has_eol, cfg, &mut rows);
    rows
}

/// The text of each row.
fn split(text: &str, has_eol: bool, cfg: &LayoutConfig) -> Vec<String> {
    lay(text, has_eol, cfg)
        .iter()
        .map(|r| r.text(text).to_string())
        .collect()
}

fn kinsoku(head: bool, tail: bool, kuto: bool) -> Kinsoku {
    Kinsoku {
        head,
        tail,
        kuto,
        ..Kinsoku::default()
    }
}

// ---- plain wrapping ----

#[test]
fn wraps_at_the_wrap_column() {
    let cfg = config(10);
    assert_eq!(split("abcdefghij", false, &cfg), ["abcdefghij"]);
    assert_eq!(split("abcdefghijk", false, &cfg), ["abcdefghij", "k"]);
    let rows = lay("abcdefghijklmnopqrstu", false, &cfg);
    assert_eq!(rows.len(), 3);
    assert!(rows[0].wrapped && rows[1].wrapped && !rows[2].wrapped);
    assert_eq!(
        rows.iter().map(|r| r.end_column).collect::<Vec<_>>(),
        [10, 10, 1]
    );
}

#[test]
fn no_wrap_still_breaks_at_the_sakura_maximum() {
    // CEditDoc.cpp:781: "not wrapped" lays out with MAXLINEKETAS (10240) columns.
    let text = "a".repeat(10240 + 5);
    let rows = lay(&text, false, &LayoutConfig::default());
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].range, 0..10240);
}

#[test]
fn empty_line_has_one_row() {
    let cfg = config(10);
    let rows = lay("", false, &cfg);
    assert_eq!(
        rows,
        [Row {
            range: 0..0,
            start_column: 0,
            end_column: 0,
            wrapped: false
        }]
    );
    let rows = lay("", true, &cfg);
    assert_eq!((rows.len(), rows[0].end_column), (1, 0));
}

#[test]
fn full_width_characters_do_not_straddle_the_edge() {
    // _MakeOneLine, "nPosX + nCharKetas > GetMaxLineLayout()": the wide char goes to the next row.
    let cfg = config(5);
    assert_eq!(split("abcあい", false, &cfg), ["abcあ", "い"]);
    assert_eq!(split("abcdあ", false, &cfg), ["abcd", "あ"]);
    let rows = lay("abcdあ", false, &cfg);
    assert_eq!(rows[0].end_column, 4);
}

#[test]
fn the_line_ending_takes_a_column() {
    // CNativeW::GetHabaOfChar returns 1 for a line ending, so it wraps like a letter.
    let cfg = config(10);
    assert_eq!(split("abcdefghi", true, &cfg), ["abcdefghi"]);
    assert_eq!(split("abcdefghij", true, &cfg), ["abcdefghij", ""]);
    let rows = lay("abcdefghij", true, &cfg);
    assert_eq!(rows[1].range, 10..10);
    // The end column does not count the line ending.
    assert_eq!(lay("abc", true, &cfg)[0].end_column, 3);
    // "改行文字をぶら下げる": m_bKinsokuRet keeps it on the row.
    let hang = LayoutConfig {
        kinsoku: Kinsoku {
            hang_eol: true,
            ..Kinsoku::default()
        },
        ..config(10)
    };
    assert_eq!(split("abcdefghij", true, &hang), ["abcdefghij"]);
    assert_eq!(lay("abcdefghij", true, &hang)[0].end_column, 10);
}

#[test]
fn a_character_wider_than_the_row_is_still_placed() {
    // Sakura keeps the wrap column at 10 or more; here a smaller one must not loop.
    let cfg = config(1);
    assert_eq!(split("あい", false, &cfg), ["あ", "い"]);
}

// ---- tabs ----

#[test]
fn tabs_go_to_the_next_stop() {
    let cfg = config(40);
    let rows = lay("a\tb\t\tc", false, &cfg);
    // a: 1, tab: 3 (to 4), b: 5, tab: 3 (to 8), tab: 4 (to 12), c: 13.
    assert_eq!(rows[0].end_column, 13);
    let text = "a\tb\t\tc";
    assert_eq!(offset_to_position(text, &rows, &cfg, 2).column, 4);
    assert_eq!(offset_to_position(text, &rows, &cfg, 4).column, 8);
    assert_eq!(offset_to_position(text, &rows, &cfg, 5).column, 12);
}

#[test]
fn a_tab_that_does_not_fit_wraps() {
    let cfg = config(10);
    // The tab at column 8 would end at 12.
    let rows = split("abcdefgh\ti", false, &cfg);
    assert_eq!(rows, ["abcdefgh", "\ti"]);
    let text = "abcdefgh\ti";
    let r = lay(text, false, &cfg);
    // On the new row the tab starts at column 0 and takes the whole 4.
    assert_eq!(r[1].end_column, 5);
}

#[test]
fn tab_width_not_less_than_the_wrap_column_is_4() {
    // _DoLayoutSub:429, "if tab >= wrap width, tab = 4".
    let cfg = LayoutConfig {
        tab_width: 8,
        ..config(8)
    };
    let rows = lay("\ta", false, &cfg);
    assert_eq!(rows[0].end_column, 5);
}

// ---- English word wrap ----

#[test]
fn word_wrap_breaks_before_a_word_that_reaches_the_edge() {
    let cfg = LayoutConfig {
        word_wrap: true,
        ..config(10)
    };
    // _DoWordWrap:  "nPosX + nWordKetas >= GetMaxLineLayout()": "twelve" ends at column 10.
    assert_eq!(split("one twelve", false, &cfg), ["one ", "twelve"]);
    // One column less fits.
    assert_eq!(split("one twelv", false, &cfg), ["one twelv"]);
    let off = config(10);
    assert_eq!(split("one twelve", false, &off), ["one twelve"]);
}

#[test]
fn word_wrap_with_a_long_word_breaks_it_where_the_row_ends() {
    let cfg = LayoutConfig {
        word_wrap: true,
        ..config(10)
    };
    // The word starts the row, so it is not moved (pos - bgn > 0), then it wraps as usual.
    assert_eq!(split("abcdefghijkl", false, &cfg), ["abcdefghij", "kl"]);
    // A long word that follows a short one moves to a row of its own first.
    assert_eq!(
        split("xx averyveryverylongword", false, &cfg),
        ["xx ", "averyveryv", "erylongwor", "d"]
    );
}

#[test]
fn word_characters_are_ascii_symbols_and_letters() {
    // gm_keyword_char: digits, letters, `_`, and the user-defined `# $ @ \`.
    let cfg = LayoutConfig {
        word_wrap: true,
        ..config(10)
    };
    assert_eq!(split("ab #$@\\_9aZ", false, &cfg), ["ab ", "#$@\\_9aZ"]);
    // `-` and non-ASCII letters are not part of a word: "yyyy" is a word of its own.
    assert_eq!(split("ab xxxx-yyyy", false, &cfg), ["ab xxxx-", "yyyy"]);
    assert_eq!(split("ab éééé", false, &cfg), ["ab éééé"]);
}

// ---- kinsoku ----

#[test]
fn gyoto_kinsoku_pushes_the_character_before_a_head_prohibited_one() {
    // _DoGyotoKinsoku: at お (column 8), 。 would not fit and may not start a row.
    let on = LayoutConfig {
        kinsoku: kinsoku(true, false, false),
        ..config(10)
    };
    assert_eq!(split("あいうえお。", false, &on), ["あいうえ", "お。"]);
    let off = config(10);
    assert_eq!(split("あいうえお。", false, &off), ["あいうえお", "。"]);
    // A character that is itself prohibited at the head is not pushed (it would loop).
    assert_eq!(split("あいうえ。。", false, &on), ["あいうえ。", "。"]);
}

#[test]
fn gyomatsu_kinsoku_moves_a_tail_prohibited_character() {
    // _DoGyomatsuKinsoku: 「 fits at the end of the row, but may not end it.
    let on = LayoutConfig {
        kinsoku: kinsoku(false, true, false),
        ..config(10)
    };
    assert_eq!(split("あいうえ「お", false, &on), ["あいうえ", "「お"]);
    let off = config(10);
    assert_eq!(split("あいうえ「お", false, &off), ["あいうえ「", "お"]);
    // Nothing follows the 「, so there is nothing to move it away from ("nPos+1 < length").
    assert_eq!(split("あいうえ「", false, &on), ["あいうえ「"]);
    // The line ending follows: it counts as a column.
    assert_eq!(split("あいうえ「", true, &on), ["あいうえ", "「"]);
}

#[test]
fn kuto_burasage_lets_punctuation_hang_over_the_edge() {
    // _DoKutoBurasage: the 、 does not fit at column 10, and hangs.
    let on = LayoutConfig {
        kinsoku: kinsoku(false, false, true),
        ..config(10)
    };
    let rows = lay("あいうえお、か", false, &on);
    assert_eq!(rows[0].text("あいうえお、か"), "あいうえお、");
    assert_eq!(rows[0].end_column, 12);
    assert_eq!(rows[1].text("あいうえお、か"), "か");
    // The head rule does not push お, because 、 left the head list (SetLayoutInfo).
    let both = LayoutConfig {
        kinsoku: kinsoku(true, false, true),
        ..config(10)
    };
    assert_eq!(
        split("あいうえお、か", false, &both),
        ["あいうえお、", "か"]
    );
    // Without hanging, the head rule is the one that applies.
    let head = LayoutConfig {
        kinsoku: kinsoku(true, false, false),
        ..config(10)
    };
    assert_eq!(
        split("あいうえお、か", false, &head),
        ["あいうえ", "お、か"]
    );
}

#[test]
fn kuto_burasage_of_half_width_punctuation_needs_no_room() {
    let on = LayoutConfig {
        kinsoku: kinsoku(false, false, true),
        ..config(10)
    };
    // `,` fits in the last column: nothing hangs.
    assert_eq!(split("abcdefghi,j", false, &on), ["abcdefghi,", "j"]);
    // After a full row it hangs (rest 0 < width 1).
    let rows = lay("abcdefghij,k", false, &on);
    assert_eq!(rows[0].range, 0..11);
    assert_eq!(rows[0].end_column, 11);
}

#[test]
fn a_hanging_punctuation_at_the_end_of_a_line_with_its_ending() {
    let text = "あいうえお、";
    let on = LayoutConfig {
        kinsoku: kinsoku(false, false, true),
        ..config(10)
    };
    // _DoKinsokuSkip:  after the hanging 、 the row closes, and the line ending goes to the next.
    assert_eq!(split(text, true, &on), ["あいうえお、", ""]);
    // m_bKinsokuRet: the line ending hangs too, and the row stays.
    let hang = LayoutConfig {
        kinsoku: Kinsoku {
            hang_eol: true,
            ..kinsoku(false, false, true)
        },
        ..config(10)
    };
    let rows = lay(text, true, &hang);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].end_column, 12);
}

#[test]
fn all_rules_together() {
    let cfg = LayoutConfig {
        kinsoku: Kinsoku::all_on(),
        word_wrap: true,
        ..config(10)
    };
    assert_eq!(
        split("あいうえお、 abc「かき」。", false, &cfg),
        ["あいうえお、", " abc「か", "き」。"]
    );
}

#[test]
fn kinsoku_lists_can_be_replaced() {
    let on = LayoutConfig {
        kinsoku: Kinsoku {
            head: true,
            head_chars: "お".into(),
            ..Kinsoku::default()
        },
        ..config(10)
    };
    assert_eq!(split("あいうえかお", false, &on), ["あいうえ", "かお"]);
}

#[test]
fn the_tail_list_and_kuto_list_can_be_replaced() {
    let tail = LayoutConfig {
        kinsoku: Kinsoku {
            tail: true,
            tail_chars: "う".into(),
            ..Kinsoku::default()
        },
        ..config(6)
    };
    // う would end the row, so it goes to the next one with え.
    assert_eq!(split("あいうえお", false, &tail), ["あい", "うえお"]);
    // With the rule off, the list does nothing.
    let off = LayoutConfig {
        kinsoku: Kinsoku {
            tail_chars: "う".into(),
            ..Kinsoku::default()
        },
        ..config(6)
    };
    assert_eq!(split("あいうえお", false, &off), ["あいう", "えお"]);
    // The hanging punctuation is the list, and only with the rule on.
    let kuto = |on| LayoutConfig {
        kinsoku: Kinsoku {
            kuto: on,
            kuto_chars: "ー".into(),
            ..Kinsoku::default()
        },
        ..config(8)
    };
    assert_eq!(split("あいうえー", false, &kuto(true)), ["あいうえー"]);
    assert_eq!(split("あいうえー", false, &kuto(false)), ["あいうえ", "ー"]);
}

// ---- indent of wrapped rows ----

#[test]
fn indent_after_the_last_tab() {
    // getIndentOffset_Tx2x
    let cfg = LayoutConfig {
        indent: WrapIndent::AfterLastTab,
        ..config(12)
    };
    let text = "\tabcdefghijklmnopqrstu";
    let rows = lay(text, false, &cfg);
    assert_eq!(rows[0].text(text), "\tabcdefgh");
    // The second row starts under the end of the tab, and the third one keeps that.
    assert_eq!(rows[1].start_column, 4);
    assert_eq!(rows[1].text(text), "ijklmnop");
    assert_eq!(rows[2].start_column, 4);
    // Without a tab, no indent.
    let rows = lay("abcdefghijklmnopqrstu", false, &cfg);
    assert_eq!(rows[1].start_column, 0);
}

#[test]
fn indent_under_the_text_after_leading_white_space() {
    // getIndentOffset_LeftSpace, with a full-width space as white space (the default).
    let cfg = LayoutConfig {
        indent: WrapIndent::leading_white_space(),
        ..config(12)
    };
    let text = "  \u{3000}abcdefghijklmnop";
    let rows = lay(text, false, &cfg);
    assert_eq!(rows[0].end_column, 12);
    assert_eq!(rows[1].start_column, 4);
    // A tab counts with its width.
    let rows = lay("\t abcdefghijklmnop", false, &cfg);
    assert_eq!(rows[1].start_column, 5);
    // The full-width space is not white space when the setting says so.
    let strict = LayoutConfig {
        indent: WrapIndent::LeadingWhiteSpace {
            full_width_space: false,
            extra: CharSet::default(),
        },
        ..config(12)
    };
    assert_eq!(lay(text, false, &strict)[1].start_column, 2);
    // Other characters to indent (m_szIndentChars).
    let extra = LayoutConfig {
        indent: WrapIndent::LeadingWhiteSpace {
            full_width_space: true,
            extra: "*".into(),
        },
        ..config(12)
    };
    assert_eq!(lay("*- abcdefghijklmnop", false, &extra)[1].start_column, 1);
}

#[test]
fn indent_is_cut_back_when_less_than_a_tab_and_2_remain() {
    // getIndentOffset_LeftSpace, "2010.07.06 Moca": max - indent < tab + 2 gives max - (tab + 2).
    let cfg = LayoutConfig {
        indent: WrapIndent::leading_white_space(),
        ..config(12)
    };
    let text = "        abcdefghijklmnop";
    let rows = lay(text, false, &cfg);
    assert_eq!(rows[1].start_column, 6);
    // The second row is wrapped at 12 from its own indent.
    assert_eq!(rows[1].end_column, 12);
}

#[test]
fn tabs_on_an_indented_row_count_from_the_screen_edge() {
    // _OnLine1, "nPosX includes the indent (for the tab positions)".
    let cfg = LayoutConfig {
        indent: WrapIndent::AfterLastTab,
        ..config(12)
    };
    let text = "\tabcdefghijkl\tm";
    let rows = lay(text, false, &cfg);
    // Second row starts at 4: ijkl to 8, then the tab goes to 12 (not 4 + 4 = 8 columns of
    // its own: the stops are counted from the left edge); m does not fit.
    assert_eq!(rows[1].start_column, 4);
    assert_eq!(rows[1].text(text), "ijkl\t");
    assert_eq!(rows[1].end_column, 12);
    assert_eq!(rows[2].text(text), "m");
}

// ---- mapping ----

#[test]
fn offsets_and_columns_on_wrapped_rows() {
    let cfg = config(10);
    let text = "abcdefghijkl";
    let rows = lay(text, false, &cfg);
    // The offset at the start of a wrapped row is at the left of that row (LogicToLayout:1067).
    assert_eq!(
        offset_to_position(text, &rows, &cfg, 10),
        Position { row: 1, column: 0 }
    );
    assert_eq!(
        offset_to_position(text, &rows, &cfg, 9),
        Position { row: 0, column: 9 }
    );
    assert_eq!(
        offset_to_position(text, &rows, &cfg, 12),
        Position { row: 1, column: 2 }
    );
    // Past the end is the end.
    assert_eq!(offset_to_position(text, &rows, &cfg, 99).column, 2);
    assert_eq!(
        position_to_offset(text, &rows, &cfg, 1, 1),
        Located {
            offset: 11,
            extra_columns: 0
        }
    );
    // Beyond the row: its end, with the columns that are left over.
    assert_eq!(
        position_to_offset(text, &rows, &cfg, 1, 5),
        Located {
            offset: 12,
            extra_columns: 3
        }
    );
    // A row past the last is the last.
    assert_eq!(position_to_offset(text, &rows, &cfg, 7, 0).offset, 10);
}

#[test]
fn clicks_inside_a_full_width_character_snap_to_its_left() {
    // LayoutToLogicEx:1011, "nX + nCharKetas > ptLayout.x" stops before the character.
    let cfg = config(40);
    let text = "aあb";
    let rows = lay(text, false, &cfg);
    let at = |col| position_to_offset(text, &rows, &cfg, 0, col).offset;
    assert_eq!(
        [at(0), at(1), at(2), at(3), at(4), at(5)],
        [0, 1, 1, 4, 5, 5]
    );
    // The extra columns past the end.
    assert_eq!(position_to_offset(text, &rows, &cfg, 0, 7).extra_columns, 3);
    // The positions of the offsets (3-byte あ).
    let col = |o| offset_to_position(text, &rows, &cfg, o).column;
    assert_eq!([col(0), col(1), col(4), col(5)], [0, 1, 3, 4]);
    // An offset inside a character is the start of it.
    assert_eq!(col(2), 1);
}

#[test]
fn clicks_inside_a_tab_snap_to_its_left() {
    let cfg = config(40);
    let text = "a\tb";
    let rows = lay(text, false, &cfg);
    let at = |col| position_to_offset(text, &rows, &cfg, 0, col).offset;
    // The tab spans columns 1 to 3.
    assert_eq!([at(0), at(1), at(2), at(3), at(4)], [0, 1, 1, 1, 2]);
    // Hmm: column 4 is the start of b.
    assert_eq!(at(4), 2);
}

#[test]
fn the_indent_is_part_of_the_column() {
    let cfg = LayoutConfig {
        indent: WrapIndent::AfterLastTab,
        ..config(12)
    };
    let text = "\tabcdefghijklmn";
    let rows = lay(text, false, &cfg);
    // The wrapped row starts at column 4.
    assert_eq!(
        offset_to_position(text, &rows, &cfg, 9),
        Position { row: 1, column: 4 }
    );
    assert_eq!(position_to_offset(text, &rows, &cfg, 1, 0).offset, 9);
    assert_eq!(position_to_offset(text, &rows, &cfg, 1, 5).offset, 10);
}

#[test]
fn the_end_of_a_line_with_its_ending_row() {
    let cfg = config(10);
    let text = "abcdefghij";
    let rows = lay(text, true, &cfg);
    // The end of the text is on the row of the line ending, as Sakura puts the caret there.
    assert_eq!(
        offset_to_position(text, &rows, &cfg, 10),
        Position { row: 1, column: 0 }
    );
    assert_eq!(position_to_offset(text, &rows, &cfg, 1, 0).offset, 10);
}

#[test]
fn a_column_in_the_indent_of_an_empty_row_is_its_start() {
    // The row of a line ending after an indented wrap is empty and starts at column 4.
    let cfg = LayoutConfig {
        indent: WrapIndent::AfterLastTab,
        ..config(12)
    };
    let text = "\tabcdefgh";
    let rows = lay(text, true, &cfg);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].start_column, 4);
    let at = position_to_offset(text, &rows, &cfg, 1, 1);
    assert_eq!((at.offset, at.extra_columns), (text.len(), 0));
}

#[test]
fn empty_rows_map_to_zero() {
    let cfg = config(10);
    assert_eq!(
        offset_to_position("", &[], &cfg, 3),
        Position { row: 0, column: 0 }
    );
    assert_eq!(position_to_offset("", &[], &cfg, 0, 3).extra_columns, 3);
}

// ---- invariants over many lines ----

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

const POOL: &[char] = &[
    'a',
    'b',
    'c',
    'x',
    'y',
    'z',
    '1',
    '_',
    ' ',
    ' ',
    '\t',
    '-',
    ',',
    '.',
    'あ',
    'い',
    'う',
    'え',
    'お',
    '漢',
    '字',
    '、',
    '。',
    '「',
    '」',
    '）',
    '（',
    'ｱ',
    'é',
    '😀',
    '\u{10FF41}',
    '\u{0301}',
    '○',
];

fn configs() -> Vec<LayoutConfig> {
    let mut out = Vec::new();
    for columns in [1, 2, 3, 5, 10, 17] {
        for (word_wrap, k) in [
            (false, Kinsoku::default()),
            (true, Kinsoku::default()),
            (false, Kinsoku::all_on()),
            (true, Kinsoku::all_on()),
            (true, kinsoku(true, false, false)),
            (false, kinsoku(false, true, false)),
        ] {
            for indent in [
                WrapIndent::None,
                WrapIndent::AfterLastTab,
                WrapIndent::leading_white_space(),
            ] {
                for tab_width in [1, 4, 8] {
                    out.push(LayoutConfig {
                        wrap: Wrap::Columns(columns),
                        tab_width,
                        word_wrap,
                        kinsoku: k.clone(),
                        width: WidthRule::default(),
                        indent: indent.clone(),
                    });
                }
            }
        }
    }
    out
}

#[test]
fn rows_cover_the_line_and_round_trip() {
    let mut rng = Lcg(7);
    let cfgs = configs();
    let mut rows = Vec::new();
    for round in 0..4000 {
        let n = (rng.next() % 40) as usize;
        let text: String = (0..n)
            .map(|_| POOL[(rng.next() % POOL.len() as u64) as usize])
            .collect();
        let has_eol = rng.next().is_multiple_of(2);
        let cfg = &cfgs[round % cfgs.len()];
        layout_line(&text, has_eol, cfg, &mut rows);

        // The rows tile the text.
        assert!(!rows.is_empty());
        assert_eq!(rows[0].range.start, 0, "{text:?} {cfg:?}");
        assert_eq!(rows.last().unwrap().range.end, text.len());
        for pair in rows.windows(2) {
            assert_eq!(pair[0].range.end, pair[1].range.start, "{text:?} {cfg:?}");
            assert!(pair[0].wrapped);
            // Every row but the one of the line ending has a character.
        }
        assert!(!rows.last().unwrap().wrapped);

        let max = cfg.max_columns();
        for (i, row) in rows.iter().enumerate() {
            let mut x = row.start_column;
            for c in row.text(&text).chars() {
                x += cfg.columns_of(c, x);
            }
            assert_eq!(x, row.end_column, "{text:?} {cfg:?} row {i}");
            if i + 1 < rows.len() {
                assert!(!row.range.is_empty(), "{text:?} {cfg:?} row {i}");
            }
            // A row is within the wrap column, except for a first character that is wider than
            // the column (Sakura keeps the column at 10 or more, so it never meets that) and a
            // hanging punctuation (at most 2 columns over).
            let first = row.text(&text).chars().next().map_or(0, |c| {
                row.start_column + cfg.columns_of(c, row.start_column)
            });
            let limit = max.max(first) + if cfg.kinsoku.kuto { 2 } else { 0 };
            assert!(row.end_column <= limit, "{text:?} {cfg:?} row {i}");
        }

        // Every character boundary maps to a column and back.
        for (o, _) in text.char_indices().chain([(text.len(), ' ')]) {
            let p = offset_to_position(&text, &rows, cfg, o);
            let back = position_to_offset(&text, &rows, cfg, p.row, p.column);
            assert_eq!(
                (back.offset, back.extra_columns),
                (o, 0),
                "{text:?} {cfg:?} offset {o}"
            );
        }
    }
}

#[test]
fn a_one_megabyte_line_wraps_fast() {
    // About 1 MB of English with some Japanese; the rules are all on.
    let unit = "hello, world! The quick brown fox 日本語の、テキスト。「かぎ」\t";
    let text = unit.repeat(1_000_000 / unit.len());
    let cfg = LayoutConfig {
        word_wrap: true,
        kinsoku: Kinsoku::all_on(),
        ..config(80)
    };
    let mut rows = Vec::new();
    let t = std::time::Instant::now();
    layout_line(&text, false, &cfg, &mut rows);
    eprintln!(
        "{} rows from {} bytes in {:?}",
        rows.len(),
        text.len(),
        t.elapsed()
    );
    assert!(rows.len() > 10_000);
    assert_eq!(rows.last().unwrap().range.end, text.len());
    for pair in rows.windows(2) {
        assert_eq!(pair[0].range.end, pair[1].range.start);
    }
    // The indent of wrapped rows keeps it linear: each row looks only at the row before it.
    for indent in [WrapIndent::AfterLastTab, WrapIndent::leading_white_space()] {
        let indented = LayoutConfig {
            indent,
            ..cfg.clone()
        };
        layout_line(&format!("\t {text}"), false, &indented, &mut rows);
        assert!(rows.len() > 10_000);
        let indent = rows[1].start_column;
        assert!(indent > 0);
        assert!(rows[1..].iter().all(|r| r.start_column == indent));
    }
    // One long word, and a line without any break opportunity.
    layout_line(&"a".repeat(1_000_000), false, &cfg, &mut rows);
    assert_eq!(rows.len(), 12_500);
    layout_line(&"あ".repeat(500_000), false, &cfg, &mut rows);
    assert_eq!(rows.len(), 12_500);
}
