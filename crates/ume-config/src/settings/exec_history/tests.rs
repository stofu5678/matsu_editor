use super::*;

fn profile(text: &str) -> Profile {
    Profile::from_bytes(text.as_bytes())
}

#[test]
fn the_newest_command_is_first_and_a_repeat_moves_up() {
    let mut h = ExecHistory::default();
    assert!(h.add_command("a") && h.add_command("b") && h.add_command("c"));
    assert_eq!(h.commands.items(), ["c", "b", "a"]);
    assert!(h.add_command("a"));
    assert_eq!(h.commands.items(), ["a", "c", "b"]);
    // already first: nothing changes
    assert!(!h.add_command("a"));
    assert_eq!(h.commands.newest(), Some("a"));
}

#[test]
fn thirty_two_are_kept_and_the_oldest_goes() {
    let mut h = ExecHistory::default();
    for i in 0..40 {
        h.add_command(&i.to_string());
        h.add_cur_dir(&i.to_string());
    }
    assert_eq!(h.commands.items().len(), 32);
    assert_eq!(h.cur_dirs.items().len(), 32);
    assert_eq!(h.commands.newest(), Some("39"));
    assert_eq!(h.commands.items().last().map(String::as_str), Some("8"));
}

#[test]
fn an_empty_or_too_long_string_is_not_kept() {
    let mut h = ExecHistory::default();
    assert!(!h.add_command(""));
    assert!(!h.add_cur_dir(""));
    // a command must be shorter than 1024 units, a folder than 260
    assert!(h.add_command(&"c".repeat(1023)));
    assert!(!h.add_command(&"c".repeat(1024)));
    assert!(h.add_cur_dir(&"d".repeat(259)));
    assert!(!h.add_cur_dir(&"d".repeat(260)));
}

#[test]
fn it_is_written_as_sakura_writes_it() {
    let mut h = ExecHistory::default();
    h.add_command("dir");
    h.add_command("sort /R");
    h.add_cur_dir("C:\\work");
    h.flags = Some(0x205);
    let mut p = Profile::new();
    h.write_to(&mut p).unwrap();
    assert_eq!(p.get("Cmd", "nCmdArrNum"), Some("2"));
    assert_eq!(p.get("Cmd", "szCmdArr[00]"), Some("sort /R"));
    assert_eq!(p.get("Cmd", "szCmdArr[01]"), Some("dir"));
    assert_eq!(p.get("Cmd", "nCurDirArrNum"), Some("1"));
    assert_eq!(p.get("Cmd", "szCurDirArr[00]"), Some("C:\\work"));
    assert_eq!(p.get("Other", "nExecFlgOpt"), Some("517"));
}

#[test]
fn it_is_read_back_and_the_rest_of_the_file_stays() {
    let mut h = ExecHistory::default();
    h.add_command("日本語のコマンド");
    h.add_command("b");
    h.flags = Some(9);
    let mut p = profile("[Keys]\r\nSEARCHKEY[00]=x\r\n");
    h.write_to(&mut p).unwrap();
    assert_eq!(p.get("Keys", "SEARCHKEY[00]"), Some("x"));
    let again = ExecHistory::from_profile(&Profile::from_bytes(&p.to_bytes()));
    assert_eq!(again, h);
}

#[test]
fn a_missing_count_reads_every_place() {
    let p = profile("[Cmd]\r\nszCmdArr[00]=a\r\nszCmdArr[01]=b\r\n");
    let h = ExecHistory::from_profile(&p);
    assert_eq!(h.commands.items(), ["a", "b"]);
    assert!(h.cur_dirs.items().is_empty());
    assert_eq!(h.flags, None);
}

#[test]
fn a_count_limits_what_is_read_and_a_bad_one_is_mended() {
    let p = profile("[Cmd]\r\nnCmdArrNum=1\r\nszCmdArr[00]=a\r\nszCmdArr[01]=b\r\n");
    assert_eq!(ExecHistory::from_profile(&p).commands.items(), ["a"]);
    let p = profile("[Cmd]\r\nnCmdArrNum=-5\r\nszCmdArr[00]=a\r\n");
    assert!(ExecHistory::from_profile(&p).commands.items().is_empty());
    let p = profile("[Cmd]\r\nnCmdArrNum=999\r\nszCmdArr[00]=a\r\n");
    assert_eq!(ExecHistory::from_profile(&p).commands.items(), ["a"]);
}

#[test]
fn the_old_name_of_the_choice_is_read_when_the_new_one_is_missing() {
    let p = profile("[Other]\r\nbGetStdout=1\r\n");
    assert_eq!(ExecHistory::from_profile(&p).flags, Some(1));
    let p = profile("[Other]\r\nbGetStdout=1\r\nnExecFlgOpt=7\r\n");
    assert_eq!(ExecHistory::from_profile(&p).flags, Some(7));
}

#[test]
fn a_string_with_a_line_break_is_not_written() {
    let mut h = ExecHistory::default();
    h.add_command("a\nb");
    h.add_command("ok");
    let mut p = Profile::new();
    h.write_to(&mut p).unwrap();
    assert_eq!(p.get("Cmd", "nCmdArrNum"), Some("1"));
    assert_eq!(p.get("Cmd", "szCmdArr[00]"), Some("ok"));
}

#[test]
fn what_the_file_has_goes_on_top_when_it_is_merged() {
    let mut ours = ExecHistory::default();
    ours.add_command("ours");
    ours.add_command("both");
    ours.flags = Some(3);
    let mut file = ExecHistory::default();
    file.add_command("both");
    file.add_command("theirs");
    file.flags = Some(1);
    ours.merge_from(&file);
    assert_eq!(ours.commands.items(), ["theirs", "both", "ours"]);
    // our choice is the later one
    assert_eq!(ours.flags, Some(3));
    let mut empty = ExecHistory::default();
    empty.merge_from(&file);
    assert_eq!(empty.flags, Some(1));
}
