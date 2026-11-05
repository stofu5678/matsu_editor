//! The scan that is fed bytes as a file is written ([`StreamScanner`]) makes the index that the
//! scan of the finished file makes.

use std::ops::ControlFlow;

use super::*;
use crate::buffer::{EncodingCodec, MemorySource};
use crate::encoding::Encoding;

fn codec(enc: Encoding) -> Arc<dyn Codec> {
    Arc::new(EncodingCodec(enc))
}

fn source(bytes: &[u8]) -> Arc<dyn Source> {
    Arc::new(MemorySource::new(bytes.to_vec()))
}

fn fresh(bytes: &[u8], enc: Encoding, o: ScanOptions) -> BlockIndex {
    scan_blocks(source(bytes), codec(enc), o, |_| ControlFlow::Continue(())).unwrap()
}

fn streamed(bytes: &[u8], enc: Encoding, o: ScanOptions, chunk: usize) -> BlockIndex {
    let mut scan = StreamScanner::new(codec(enc), o).unwrap();
    for part in bytes.chunks(chunk.max(1)) {
        scan.feed(part).unwrap();
    }
    assert_eq!(scan.fed(), bytes.len() as u64);
    scan.finish(source(bytes)).unwrap()
}

fn assert_same(a: &BlockIndex, b: &BlockIndex, what: &str) {
    assert_eq!(a.starts, b.starts, "{what}: block starts");
    assert_eq!(a.breaks_before, b.breaks_before, "{what}: breaks before");
    assert_eq!(a.covered, b.covered, "{what}");
    assert_eq!(a.breaks, b.breaks, "{what}");
    assert_eq!(a.is_complete(), b.is_complete(), "{what}");
    let anchors = |i: &BlockIndex| {
        i.long_anchors
            .iter()
            .map(|a| (a.block, a.line_start, a.dec_before))
            .collect::<Vec<_>>()
    };
    assert_eq!(anchors(a), anchors(b), "{what}: anchors");
}

fn lcg(seed: &mut u64, n: usize) -> usize {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*seed >> 33) as usize) % n
}

#[test]
fn a_streamed_scan_makes_the_index_of_the_finished_file() {
    let alphabet: [&[u8]; 8] = [
        b"a",
        b"\n",
        b"\r",
        b"\r\n",
        "あ".as_bytes(),
        "\u{85}".as_bytes(),
        b"\xFF",
        "😀".as_bytes(),
    ];
    let mut seed = 11;
    for round in 0..60 {
        let mut bytes = Vec::new();
        for _ in 0..lcg(&mut seed, 200) {
            bytes.extend_from_slice(alphabet[lcg(&mut seed, alphabet.len())]);
        }
        for ext_eol in [false, true] {
            for block_size in [1, 3, 7, 16] {
                let o = ScanOptions {
                    block_size,
                    window_blocks: 1 + lcg(&mut seed, 3),
                    ext_eol,
                    long_line: [0, 20][round % 2],
                    ..ScanOptions::default()
                };
                let want = fresh(&bytes, Encoding::Utf8, o);
                for chunk in [1, 5, 13, 1000] {
                    let got = streamed(&bytes, Encoding::Utf8, o, chunk);
                    assert_same(&got, &want, &format!("round {round} {o:?} chunk {chunk}"));
                }
            }
        }
    }
}

#[test]
fn a_streamed_scan_finds_the_anchors_of_a_very_long_line() {
    let long = "日本語abc\t漢字 テキスト".repeat(40);
    let text = format!("short\n{long}\nafter\r\n{long}");
    for enc in [Encoding::Utf8, Encoding::ShiftJis, Encoding::Utf16Le] {
        let mut bytes = Vec::new();
        enc.codec().encode(&text, &mut bytes).unwrap();
        for block_size in [8, 16, 64] {
            for long_line in [100, 400] {
                let o = ScanOptions {
                    block_size,
                    window_blocks: 3,
                    long_line,
                    ..ScanOptions::default()
                };
                let want = fresh(&bytes, enc, o);
                assert!(!want.long_anchors.is_empty());
                for chunk in [1, 17, 4096] {
                    let got = streamed(&bytes, enc, o, chunk);
                    assert_same(&got, &want, &format!("{enc:?} {o:?} chunk {chunk}"));
                }
            }
        }
    }
}

#[test]
fn a_streamed_scan_of_other_encodings_matches() {
    let text = "ab\r\ncd\nあいう\r\n".repeat(30);
    for enc in [Encoding::ShiftJis, Encoding::EucJp, Encoding::Utf16Be] {
        let mut bytes = Vec::new();
        enc.codec().encode(&text, &mut bytes).unwrap();
        let o = ScanOptions {
            block_size: 10,
            window_blocks: 2,
            ..ScanOptions::default()
        };
        assert_same(
            &streamed(&bytes, enc, o, 9),
            &fresh(&bytes, enc, o),
            &format!("{enc:?}"),
        );
    }
}

#[test]
fn a_streamed_scan_refuses_a_stateful_encoding_and_a_wrong_length() {
    assert!(matches!(
        StreamScanner::new(codec(Encoding::Jis), ScanOptions::default()),
        Err(ScanError::Stateful)
    ));
    let mut scan = StreamScanner::new(codec(Encoding::Utf8), ScanOptions::default()).unwrap();
    scan.feed(b"abc\n").unwrap();
    assert!(scan.finish(source(b"abc")).is_err());
}

#[test]
fn nothing_fed_is_an_empty_index() {
    let scan = StreamScanner::new(codec(Encoding::Utf8), ScanOptions::default()).unwrap();
    let index = scan.finish(source(b"")).unwrap();
    assert_eq!(index.covered_len(), 0);
    assert!(index.is_complete());
}
