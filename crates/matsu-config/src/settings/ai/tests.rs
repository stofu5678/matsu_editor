use super::*;
use crate::settings::oneliner::OneLiner;

fn profile(text: &str) -> Profile {
    // A file with the BOM is UTF-8, as the one MATSU Editor writes.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

fn dest(name: &str, command: &str, output: AiOutput) -> AiDestination {
    AiDestination {
        name: name.to_owned(),
        command: command.to_owned(),
        output,
        ..AiDestination::default()
    }
}

#[test]
fn an_absent_section_is_50_empty_slots() {
    let table = AiDestinations::from_profile(&profile("[Other]\nA=1\n"));
    assert_eq!(table, AiDestinations::default());
    assert_eq!(table.slots.len(), 50);
    assert!(table.slots.iter().all(|s| !s.is_registered()));
}

#[test]
fn slots_are_read_with_their_keys_and_defaults() {
    let table = AiDestinations::from_profile(&profile(
        "[AI]\nName[000]=英訳\nCommand[000]=claude -p\nOutput[000]=0\nAskBeforeSend[000]=0\n\
         Command[049]=codex exec\n",
    ));
    assert_eq!(
        table.slots[0],
        AiDestination {
            ask_before_send: false,
            ..dest("英訳", "claude -p", AiOutput::Replace)
        }
    );
    // Output and AskBeforeSend absent: next line, ask.
    assert_eq!(table.slots[49], dest("", "codex exec", AiOutput::NextLine));
    assert_eq!(table.slots[49].title(), "codex exec");
}

#[test]
fn other_values_of_output_and_ask_read_as_1() {
    let table = AiDestinations::from_profile(&profile(
        "[AI]\nCommand[000]=a\nOutput[000]=7\nAskBeforeSend[000]=2\n\
         Command[001]=b\nOutput[001]=x\nAskBeforeSend[001]=\nCommand[002]=c\nOutput[002]=2\n",
    ));
    assert_eq!(table.slots[0].output, AiOutput::NextLine);
    assert!(table.slots[0].ask_before_send);
    assert_eq!(table.slots[1].output, AiOutput::NextLine);
    assert!(table.slots[1].ask_before_send);
    assert_eq!(table.slots[2].output, AiOutput::LogOnly);
}

#[test]
fn unusable_commands_are_empty_slots() {
    let long = "x".repeat(COMMAND_MAX_UNITS);
    let table = AiDestinations::from_profile(&profile(&format!(
        "[AI]\nCommand[000]=   \nName[000]=n\nCommand[001]={long}\nCommand[050]=past\n"
    )));
    assert_eq!(table, AiDestinations::default());
}

#[test]
fn a_round_trip_keeps_everything_and_a_second_store_changes_nothing() {
    let mut table = AiDestinations::default();
    table.slots[3] = dest("英訳", "claude -p", AiOutput::LogOnly);
    table.slots[7] = AiDestination {
        ask_before_send: false,
        ..dest("要約", "codex exec -", AiOutput::Replace)
    };
    let mut p = profile("");
    assert_eq!(table.store(&mut p), Ok(true));
    assert_eq!(p.get(SECTION, "Output[003]"), Some("2"));
    assert_eq!(p.get(SECTION, "AskBeforeSend[007]"), Some("0"));
    assert_eq!(AiDestinations::from_profile(&p), table);
    let again = Profile::from_bytes(&p.to_bytes());
    assert_eq!(AiDestinations::from_profile(&again), table);
    assert_eq!(table.store(&mut p), Ok(false));
}

#[test]
fn storing_keeps_unknown_keys_and_removes_a_cleared_slot() {
    let mut p = profile(
        "[AI]\nFuture=keep\nName[000]=a\nCommand[000]=x\nOutput[000]=1\nAskBeforeSend[000]=1\n\
         [Other]\nCommand[000]=mine\n",
    );
    let mut table = AiDestinations::from_profile(&p);
    table.slots[0] = AiDestination::default();
    assert_eq!(table.store(&mut p), Ok(true));
    assert_eq!(p.get(SECTION, "Future"), Some("keep"));
    assert_eq!(p.get(SECTION, "Command[000]"), None);
    assert_eq!(p.get(SECTION, "AskBeforeSend[000]"), None);
    assert_eq!(p.get("Other", "Command[000]"), Some("mine"));
}

#[test]
fn the_limits_are_refused_before_anything_is_written() {
    let mut table = AiDestinations::default();
    table.slots[0] = dest("ok", "fine", AiOutput::NextLine);
    table.slots[1] = dest("two\nlines", "fine", AiOutput::NextLine);
    let mut p = profile("");
    assert_eq!(
        table.store(&mut p),
        Err(OneLinerError::LineBreak {
            key: "Name[001]".to_owned()
        })
    );
    assert_eq!(p.get(SECTION, "Command[000]"), None);
    table.slots[1] = dest("ok", &"x".repeat(COMMAND_MAX_UNITS), AiOutput::NextLine);
    assert!(matches!(
        table.store(&mut p),
        Err(OneLinerError::TooLong { .. })
    ));
    // A long name is cut when read, not refused there.
    let name = "あ".repeat(NAME_MAX_UNITS);
    let read = AiDestinations::from_profile(&profile(&format!(
        "[AI]\nName[000]={name}\nCommand[000]=x\n"
    )));
    assert_eq!(units(&read.slots[0].name), NAME_MAX_UNITS - 1);
}

// ---- migrate ----

const ONE_AI: &str = "[OneLiner]\nName[000]=sorted\nCommand[000]=sort -u\nOutput[000]=0\n\
    Name[001]=英訳\nCommand[001]=claude -p\nOutput[001]=2\nAI[001]=1\nAskBeforeSend[001]=0\n\
    Name[002]=last\nCommand[002]=wc -l\nAI[002]=0\nUnknown[001]=keep\nRecent[00]=tr a-z A-Z\n";

#[test]
fn nothing_to_move_changes_nothing() {
    let mut p = profile("[OneLiner]\nCommand[000]=sort\nAI[000]=0\n[Other]\nA=1\n");
    let before = p.to_bytes();
    assert_eq!(migrate(&mut p), Ok(AiMigration::NothingToMove));
    assert_eq!(p.to_bytes(), before);
    // A mark without a command is not a registered slot: nothing to move either.
    let mut p = profile("[OneLiner]\nAI[000]=1\n");
    assert_eq!(migrate(&mut p), Ok(AiMigration::NothingToMove));
}

#[test]
fn a_marked_slot_moves_with_its_fields() {
    let mut p = profile(ONE_AI);
    assert_eq!(migrate(&mut p), Ok(AiMigration::Moved { count: 1 }));
    assert!(AiMigration::Moved { count: 1 }.changed());
    let ai = AiDestinations::from_profile(&p);
    // Same number; the output window became the log tab.
    assert_eq!(
        ai.slots[1],
        AiDestination {
            ask_before_send: false,
            ..dest("英訳", "claude -p", AiOutput::LogOnly)
        }
    );
    for k in ["Name", "Command", "Output", "AI", "AskBeforeSend"] {
        assert_eq!(p.get(ONELINER, &format!("{k}[001]")), None, "{k}");
    }
    // The other slots, the unknown key and the history stay.
    let one = OneLiners::from_profile(&p);
    assert_eq!(one.slots[0].command, "sort -u");
    assert_eq!(one.slots[2].command, "wc -l");
    assert_eq!(one.recent, ["tr a-z A-Z"]);
    assert_eq!(p.get(ONELINER, "Unknown[001]"), Some("keep"));
    assert!(one.ai_marked().is_empty());
}

#[test]
fn a_second_run_changes_nothing() {
    let mut p = profile(ONE_AI);
    migrate(&mut p).unwrap();
    let after = p.to_bytes();
    assert_eq!(migrate(&mut p), Ok(AiMigration::NothingToMove));
    assert_eq!(p.to_bytes(), after);
}

#[test]
fn a_number_taken_in_ai_goes_to_the_first_free_one() {
    let mut p = profile(&format!(
        "{ONE_AI}[AI]\nCommand[001]=taken\nName[001]=T\nCommand[000]=also\n"
    ));
    assert_eq!(migrate(&mut p), Ok(AiMigration::Moved { count: 1 }));
    let ai = AiDestinations::from_profile(&p);
    assert_eq!(ai.slots[0].command, "also");
    assert_eq!(ai.slots[1].command, "taken");
    assert_eq!(ai.slots[2].command, "claude -p");
    assert_eq!(ai.slots[2].name, "英訳");
}

#[test]
fn several_marked_slots_all_move_in_order() {
    let mut p = profile(
        "[OneLiner]\nCommand[000]=a\nAI[000]=1\nCommand[005]=b\nAI[005]=1\nCommand[006]=c\nAI[006]=1\n\
         [AI]\nCommand[000]=old\nCommand[005]=old5\n",
    );
    assert_eq!(migrate(&mut p), Ok(AiMigration::Moved { count: 3 }));
    let ai = AiDestinations::from_profile(&p);
    let commands: Vec<_> = ai
        .slots
        .iter()
        .filter(|s| s.is_registered())
        .map(|s| s.command.as_str())
        .collect();
    // 006 keeps its number; 000 and 005 (taken) take the first free ones, 001 and 002.
    assert_eq!(commands, ["old", "a", "b", "old5", "c"]);
    assert_eq!(ai.slots[6].command, "c");
    assert!(
        OneLiners::from_profile(&p)
            .slots
            .iter()
            .all(|s| !s.is_registered())
    );
}

#[test]
fn when_ai_is_too_full_nothing_changes() {
    let mut ai_part = String::from("[AI]\n");
    for i in 0..MAX_AI - 1 {
        ai_part.push_str(&format!("Command[{i:03}]=c{i}\n"));
    }
    // One free slot, two marked.
    let mut p = profile(&format!(
        "[OneLiner]\nCommand[000]=a\nAI[000]=1\nCommand[001]=b\nAI[001]=1\n{ai_part}"
    ));
    let before = p.to_bytes();
    assert_eq!(migrate(&mut p), Ok(AiMigration::Blocked { count: 2 }));
    assert!(!AiMigration::Blocked { count: 2 }.changed());
    assert_eq!(p.to_bytes(), before);
    // The slots stay marked, so the runner can refuse to run them as one-liners.
    let one = OneLiners::from_profile(&p);
    assert_eq!(one.ai_marked(), [0, 1]);
    assert!(one.slots[0].is_ai_marked());
    assert!(!OneLiner::default().is_ai_marked());
}

#[test]
fn exactly_enough_room_moves_everything() {
    let mut ai_part = String::from("[AI]\n");
    for i in 0..MAX_AI - 2 {
        ai_part.push_str(&format!("Command[{i:03}]=c{i}\n"));
    }
    let mut p = profile(&format!(
        "[OneLiner]\nCommand[000]=a\nAI[000]=1\nCommand[001]=b\nAI[001]=1\n{ai_part}"
    ));
    assert_eq!(migrate(&mut p), Ok(AiMigration::Moved { count: 2 }));
    let ai = AiDestinations::from_profile(&p);
    assert!(ai.slots.iter().all(AiDestination::is_registered));
}

#[test]
fn migrating_works_through_the_bytes_of_the_file() {
    let mut p = profile(ONE_AI);
    migrate(&mut p).unwrap();
    let again = Profile::from_bytes(&p.to_bytes());
    assert_eq!(AiDestinations::from_profile(&again).slots[1].name, "英訳");
    assert!(OneLiners::from_profile(&again).ai_marked().is_empty());
}
