use super::*;

fn profile(text: &str) -> Profile {
    // A file with the BOM is UTF-8, as the one MATSU Editor writes.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

fn one(name: &str, command: &str, output: OutputTo) -> OneLiner {
    OneLiner {
        name: name.to_owned(),
        command: command.to_owned(),
        output,
        ..OneLiner::default()
    }
}

#[test]
fn an_absent_section_is_50_empty_slots_and_no_history() {
    let table = OneLiners::from_profile(&profile("[Other]\nA=1\n"));
    assert_eq!(table, OneLiners::default());
    assert_eq!(table.slots.len(), 50);
    assert!(table.slots.iter().all(|s| !s.is_registered()));
    assert!(table.recent.is_empty());
}

#[test]
fn rows_are_read_with_their_keys() {
    let table = OneLiners::from_profile(&profile(
        "[OneLiner]\nName[000]=sorted\nCommand[000]=sort -u\nOutput[000]=1\n\
         Command[049]=jq .\nOutput[049]=2\n",
    ));
    assert_eq!(table.slots[0], one("sorted", "sort -u", OutputTo::NextLine));
    // No name: the command is the title.
    assert_eq!(table.slots[49], one("", "jq .", OutputTo::OutputWindow));
    assert_eq!(table.slots[49].title(), "jq .");
    assert_eq!(table.of_function(F_ONELINER_0).unwrap().title(), "sorted");
}

#[test]
fn a_round_trip_with_japanese_names_keeps_everything() {
    let mut table = OneLiners::default();
    table.slots[3] = one("重複を除いて並べ替え", "sort -u", OutputTo::Replace);
    table.slots[7] = one("大文字に", "tr a-z A-Z", OutputTo::OutputWindow);
    table.add_recent("grep あ");
    table.add_recent("wc -l");
    let mut p = profile("");
    assert_eq!(table.store(&mut p), Ok(true));
    assert_eq!(p.get(SECTION, "Name[003]"), Some("重複を除いて並べ替え"));
    assert_eq!(p.get(SECTION, "Output[007]"), Some("2"));
    assert_eq!(OneLiners::from_profile(&p), table);
    // Through the bytes of the file, too.
    let again = Profile::from_bytes(&p.to_bytes());
    assert_eq!(OneLiners::from_profile(&again), table);
    // Writing the same table again changes nothing.
    assert_eq!(table.store(&mut p), Ok(false));
}

#[test]
fn an_invalid_output_reads_as_replace() {
    let table = OneLiners::from_profile(&profile(
        "[OneLiner]\nCommand[000]=a\nOutput[000]=3\nCommand[001]=b\nOutput[001]=-1\n\
         Command[002]=c\nOutput[002]=x\nCommand[003]=d\n",
    ));
    for i in 0..4 {
        assert_eq!(table.slots[i].output, OutputTo::Replace, "{i}");
        assert!(table.slots[i].is_registered());
    }
}

#[test]
fn a_slot_with_an_empty_command_is_not_registered_and_its_keys_go() {
    let mut p = profile(
        "[OneLiner]\nName[000]=n\nCommand[000]=\nOutput[000]=1\nName[001]=m\nCommand[001]=x\n",
    );
    let mut table = OneLiners::from_profile(&p);
    assert!(!table.slots[0].is_registered());
    assert_eq!(table.slots[0], OneLiner::default());
    assert!(table.slots[1].is_registered());
    assert_eq!(table.store(&mut p), Ok(true));
    assert_eq!(p.get(SECTION, "Name[000]"), None);
    assert_eq!(p.get(SECTION, "Output[000]"), None);
    // Emptying a registered slot removes it, too.
    table.slots[1].command.clear();
    assert_eq!(table.store(&mut p), Ok(true));
    assert_eq!(p.get(SECTION, "Command[001]"), None);
}

#[test]
fn slots_beyond_49_are_ignored() {
    let table = OneLiners::from_profile(&profile(
        "[OneLiner]\nCommand[050]=x\nCommand[100]=y\nCommand[049]=z\n",
    ));
    assert_eq!(table.slots.len(), 50);
    assert_eq!(table.slots[49].command, "z");
    assert_eq!(table.slots.iter().filter(|s| s.is_registered()).count(), 1);
    // A table with more slots writes only 50.
    let mut big = OneLiners::default();
    big.slots.push(one("x", "echo", OutputTo::Replace));
    let mut p = profile("");
    big.store(&mut p).unwrap();
    assert_eq!(p.get(SECTION, "Command[050]"), None);
}

#[test]
fn the_history_is_newest_first_without_repeats_and_has_a_limit() {
    let mut table = OneLiners::default();
    for n in 0..25 {
        table.add_recent(&format!("cmd {n}"));
    }
    assert_eq!(table.recent.len(), MAX_ONELINER_RECENT);
    assert_eq!(table.recent[0], "cmd 24");
    assert_eq!(table.recent[19], "cmd 5");
    // A line that is there moves to the front.
    assert!(table.add_recent("cmd 10"));
    assert_eq!(table.recent[0], "cmd 10");
    assert_eq!(table.recent.iter().filter(|r| *r == "cmd 10").count(), 1);
    assert!(!table.add_recent("cmd 10"));
    // Empty, broken and too long lines are not kept.
    assert!(!table.add_recent(""));
    assert!(!table.add_recent("a\nb"));
    assert!(!table.add_recent(&"x".repeat(COMMAND_MAX_UNITS)));
    assert_eq!(table.recent.len(), MAX_ONELINER_RECENT);
    // The file keeps the order; the reader drops empty lines and repeats, and stops at 20.
    let mut p = profile("");
    table.store(&mut p).unwrap();
    assert_eq!(p.get(SECTION, "Recent[00]"), Some("cmd 10"));
    assert_eq!(OneLiners::from_profile(&p).recent, table.recent);
    let read = OneLiners::from_profile(&profile(
        "[OneLiner]\nRecent[00]=a\nRecent[01]=\nRecent[02]=b\nRecent[03]=a\nRecent[20]=late\n",
    ));
    assert_eq!(read.recent, ["a", "b"]);
}

#[test]
fn a_shorter_history_removes_the_keys_after_it() {
    let mut p = profile("[OneLiner]\nRecent[00]=a\nRecent[01]=b\nRecent[02]=c\n");
    let mut table = OneLiners::from_profile(&p);
    table.recent.truncate(1);
    assert_eq!(table.store(&mut p), Ok(true));
    assert_eq!(p.get(SECTION, "Recent[01]"), None);
    assert_eq!(p.get(SECTION, "Recent[00]"), Some("a"));
}

#[test]
fn a_line_break_is_refused_and_nothing_is_written() {
    for (name, command) in [("n", "a\nb"), ("n\r", "a"), ("n", "a\rb")] {
        let mut table = OneLiners::default();
        table.slots[0] = one("ok", "echo", OutputTo::Replace);
        table.slots[1] = one(name, command, OutputTo::Replace);
        let mut p = profile("");
        let before = p.to_bytes();
        let Err(OneLinerError::LineBreak { key }) = table.store(&mut p) else {
            panic!("not refused: {name:?} {command:?}");
        };
        assert!(key.ends_with("[001]"), "{key}");
        assert_eq!(p.to_bytes(), before);
    }
    let mut table = OneLiners::default();
    table.recent.push("a\nb".to_owned());
    assert!(matches!(
        table.store(&mut profile("")),
        Err(OneLinerError::LineBreak { .. })
    ));
}

#[test]
fn a_broken_file_reads_up_to_the_line_break() {
    // The command was broken over two lines by hand: the second line is not an entry.
    let table = OneLiners::from_profile(&profile(
        "[OneLiner]\nName[000]=n\nCommand[000]=echo a\nb\nOutput[000]=1\n",
    ));
    assert_eq!(table.slots[0], one("n", "echo a", OutputTo::NextLine));
}

#[test]
fn the_limits_of_a_name_and_a_command() {
    // 63 units fit, 64 do not.
    let mut table = OneLiners::default();
    table.slots[0] = one(&"あ".repeat(63), &"x".repeat(1023), OutputTo::Replace);
    let mut p = profile("");
    assert_eq!(table.store(&mut p), Ok(true));
    assert_eq!(OneLiners::from_profile(&p), table);
    table.slots[0].name = "あ".repeat(64);
    assert!(matches!(
        table.store(&mut profile("")),
        Err(OneLinerError::TooLong { .. })
    ));
    table.slots[0].name.clear();
    table.slots[0].command = "x".repeat(1024);
    assert!(matches!(
        table.store(&mut profile("")),
        Err(OneLinerError::TooLong { .. })
    ));
    // The reader cuts a name (at a character, an emoji is 2 units) but drops a command that is too long.
    let long_name = format!("{}😀😀", "a".repeat(61));
    let read = OneLiners::from_profile(&profile(&format!(
        "[OneLiner]\nName[000]={long_name}\nCommand[000]=x\nName[001]=n\nCommand[001]={}\n",
        "y".repeat(1024)
    )));
    assert_eq!(read.slots[0].name, format!("{}😀", "a".repeat(61)));
    assert_eq!(units(&read.slots[0].name), 63);
    assert!(!read.slots[1].is_registered());
}

#[test]
fn unknown_keys_and_other_sections_are_kept() {
    let mut p = profile(
        "[Before]\nA=1\n[OneLiner]\nFuture=keep\nName[000]=old\nCommand[000]=a\n[After]\nB=2\n",
    );
    let mut table = OneLiners::from_profile(&p);
    table.slots[0].name = "new".to_owned();
    table.slots[2] = one("two", "b", OutputTo::NextLine);
    table.add_recent("r");
    assert_eq!(table.store(&mut p), Ok(true));
    assert_eq!(p.get(SECTION, "Future"), Some("keep"));
    assert_eq!(p.get(SECTION, "Name[000]"), Some("new"));
    assert_eq!(p.get("Before", "A"), Some("1"));
    assert_eq!(p.get("After", "B"), Some("2"));
}

#[test]
fn the_functions_and_the_numbers_go_both_ways() {
    assert_eq!(index_of_function(F_ONELINER_0), Some(0));
    assert_eq!(index_of_function(F_ONELINER_0 + 49), Some(49));
    assert_eq!(index_of_function(F_ONELINER_0 + 50), None);
    assert_eq!(index_of_function(F_ONELINER_0 - 1), None);
    assert_eq!(index_of_function(0), None);
    assert_eq!(function_of_index(0), Some(F_ONELINER_0));
    assert_eq!(function_of_index(49), Some(F_ONELINER_0 + 49));
    assert_eq!(function_of_index(50), None);
    assert_eq!(function_of_index(usize::MAX), None);
    for i in 0..MAX_ONELINER {
        assert_eq!(index_of_function(function_of_index(i).unwrap()), Some(i));
    }
    // The three others follow the 50, and none is a one-liner.
    assert_eq!(F_ONELINER_0 + 50, F_ONELINER_LIST);
    assert_eq!(F_ONELINER_LIST + 1, F_ONELINER_DIALOG);
    assert_eq!(F_ONELINER_DIALOG + 1, F_ONELINER_SETTINGS);
    assert_eq!(index_of_function(F_ONELINER_LIST), None);
}

#[test]
fn spaces_of_a_command_are_kept_but_a_command_of_spaces_is_not_registered() {
    // `printf '%s' foo\ ` ends with an escaped space that is part of the argument.
    let mut table = OneLiners::default();
    table.slots[0] = one("n", "printf '%s' foo\\ ", OutputTo::Replace);
    table.slots[1] = one("blank", "   ", OutputTo::Replace);
    let mut p = profile("");
    assert_eq!(table.store(&mut p), Ok(true));
    assert_eq!(p.get(SECTION, "Command[001]"), None);
    let read = OneLiners::from_profile(&Profile::from_bytes(&p.to_bytes()));
    assert_eq!(read.slots[0].command, "printf '%s' foo\\ ");
    assert!(!read.slots[1].is_registered());
}

// ---- AI and AskBeforeSend (ADR-0030, decision 7) ----

#[test]
fn ai_defaults_to_off_and_asking_to_on() {
    let table = OneLiners::from_profile(&profile("[OneLiner]\nCommand[000]=cat\n"));
    assert!(!table.slots[0].ai);
    assert!(table.slots[0].ask_before_send);
    assert!(!OneLiner::default().ai);
    assert!(OneLiner::default().ask_before_send);
}

#[test]
fn ai_and_ask_are_read_and_a_value_that_is_not_0_or_1_is_the_default() {
    let table = OneLiners::from_profile(&profile(
        "[OneLiner]\nCommand[000]=a\nAI[000]=1\nAskBeforeSend[000]=0\n\
         Command[001]=b\nAI[001]=2\nAskBeforeSend[001]=2\n\
         Command[002]=c\nAI[002]=yes\nAskBeforeSend[002]=\n\
         Command[003]=d\nAI[003]=-1\nAskBeforeSend[003]=x\n",
    ));
    assert!(table.slots[0].ai && !table.slots[0].ask_before_send);
    for i in 1..=3 {
        assert!(!table.slots[i].ai, "{i}");
        assert!(table.slots[i].ask_before_send, "{i}");
    }
}

#[test]
fn ai_and_ask_round_trip_and_are_written_only_when_they_differ_from_the_default() {
    let mut table = OneLiners::default();
    table.slots[0] = OneLiner {
        ai: true,
        ask_before_send: false,
        ..one("claude", "claude -p", OutputTo::NextLine)
    };
    table.slots[1] = OneLiner {
        ai: true,
        ..one("ask", "codex exec -", OutputTo::NextLine)
    };
    // Not asking with no AI mark is kept too.
    table.slots[2] = OneLiner {
        ask_before_send: false,
        ..one("", "cat", OutputTo::Replace)
    };
    table.slots[3] = one("", "sort", OutputTo::Replace);
    let mut p = profile("");
    assert_eq!(table.store(&mut p), Ok(true));
    assert_eq!(p.get(SECTION, "AI[000]"), Some("1"));
    assert_eq!(p.get(SECTION, "AskBeforeSend[000]"), Some("0"));
    assert_eq!(p.get(SECTION, "AI[001]"), Some("1"));
    assert_eq!(p.get(SECTION, "AskBeforeSend[001]"), None);
    assert_eq!(p.get(SECTION, "AI[002]"), None);
    assert_eq!(p.get(SECTION, "AskBeforeSend[002]"), Some("0"));
    assert_eq!(p.get(SECTION, "AI[003]"), None);
    assert_eq!(p.get(SECTION, "AskBeforeSend[003]"), None);
    assert_eq!(OneLiners::from_profile(&p), table);
    assert_eq!(table.store(&mut p), Ok(false));
    // Back to the defaults: the keys go.
    table.slots[0].ai = false;
    table.slots[0].ask_before_send = true;
    assert_eq!(table.store(&mut p), Ok(true));
    assert_eq!(p.get(SECTION, "AI[000]"), None);
    assert_eq!(p.get(SECTION, "AskBeforeSend[000]"), None);
}

#[test]
fn a_key_written_by_hand_is_left_when_it_reads_as_the_same_value() {
    let mut p = profile(
        "[OneLiner]\nName[000]=\nCommand[000]=a\nOutput[000]=0\nAI[000]=0\nAskBeforeSend[000]=1\n\
         Name[001]=\nCommand[001]=b\nOutput[001]=0\nAI[001]=junk\n",
    );
    let table = OneLiners::from_profile(&p);
    assert_eq!(table.store(&mut p), Ok(false));
    assert_eq!(p.get(SECTION, "AI[000]"), Some("0"));
    assert_eq!(p.get(SECTION, "AskBeforeSend[000]"), Some("1"));
    assert_eq!(p.get(SECTION, "AI[001]"), Some("junk"));
}

#[test]
fn emptying_a_slot_removes_its_ai_keys_and_unknown_keys_stay() {
    let mut p = profile(
        "[OneLiner]\nCommand[000]=a\nAI[000]=1\nAskBeforeSend[000]=0\nFuture=keep\nAI[049]=1\n",
    );
    let mut table = OneLiners::from_profile(&p);
    table.slots[0] = OneLiner::default();
    assert_eq!(table.store(&mut p), Ok(true));
    assert_eq!(p.get(SECTION, "AI[000]"), None);
    assert_eq!(p.get(SECTION, "AskBeforeSend[000]"), None);
    // A slot with no command is not registered, so its stray key goes too.
    assert_eq!(p.get(SECTION, "AI[049]"), None);
    assert_eq!(p.get(SECTION, "Future"), Some("keep"));
}
