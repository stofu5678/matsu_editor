//! The ten keyword sets of a type (`m_nKeyWordSetIdx[0..10]`, 強調キーワード1 to 10): the coloring uses
//! the set of every slot, with the colors of that slot, not only the first two.

use matsu_coloring::{ColorKind as K, ColorRules, ColorState, line_runs};
use matsu_config::Profile;
use matsu_config::settings::{KeywordSets, TypeSettings};
use matsu_core::buffer::Eol;

/// A settings file with ten keyword sets, `set0` with the word `word0` and so on.
fn ten_sets() -> KeywordSets {
    let mut text = String::from("[KeyWords]\r\nnKeyWordSetNum=10\r\n");
    for i in 0..10 {
        text.push_str(&format!(
            "szSN[{i:02}]=set{i}\r\nnCASE[{i:02}]=1\r\nnKWN[{i:02}]=1\r\nszKW[{i:02}]=word{i}\t\r\n"
        ));
    }
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    KeywordSets::from_profile(&Profile::from_bytes(&bytes))
}

fn kinds(rules: &ColorRules, text: &str) -> Vec<(String, K)> {
    line_runs(rules, text, Some(Eol::Lf), &ColorState::Normal, text.len())
        .iter()
        .filter_map(|r| {
            let end = r.end.min(text.len());
            (r.start < end).then(|| (text[r.start..end].to_owned(), r.kind))
        })
        .collect()
}

#[test]
fn every_one_of_the_ten_slots_colors_its_set() {
    let keywords = ten_sets();
    let mut t = TypeSettings::basic();
    // Slot `n` holds set `9 - n`, so that a slot is not mistaken for its set.
    for slot in 0..10 {
        t.keyword_set_idx[slot] = 9 - slot as i32;
    }
    let rules = ColorRules::from_type(&t, &keywords);
    let line = (0..10)
        .map(|i| format!("word{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let pieces = kinds(&rules, &line);
    for set in 0..10usize {
        let slot = 9 - set;
        assert!(
            pieces.contains(&(format!("word{set}"), K::Keyword(slot as u8))),
            "word{set} is colored as keyword {} in {pieces:?}",
            slot + 1
        );
    }
}

#[test]
fn a_slot_without_a_set_colors_nothing() {
    let keywords = ten_sets();
    let mut t = TypeSettings::basic();
    t.keyword_set_idx = [-1; 10];
    t.keyword_set_idx[4] = 7;
    let rules = ColorRules::from_type(&t, &keywords);
    let pieces = kinds(&rules, "word7 word3");
    assert!(pieces.contains(&("word7".to_owned(), K::Keyword(4))));
    assert!(
        pieces
            .iter()
            .all(|(text, kind)| text != "word3" || *kind == K::Text)
    );
}

#[test]
fn a_slot_is_off_with_its_color_switch() {
    let keywords = ten_sets();
    let mut t = TypeSettings::basic();
    t.keyword_set_idx = [-1; 10];
    t.keyword_set_idx[9] = 0;
    // 強調キーワード10 is "KWA".
    t.colors.get_mut("KWA").expect("KWA").disp = false;
    let rules = ColorRules::from_type(&t, &keywords);
    assert!(
        kinds(&rules, "word0")
            .iter()
            .all(|(_, kind)| *kind == K::Text)
    );
}
