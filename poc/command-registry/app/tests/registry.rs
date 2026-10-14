//! The test crate names no command either. Each option must list all three commands, with no
//! duplicate ID, and a listed command must run.

const EXPECTED: [&str; 3] = ["delete_line", "duplicate_line", "open_file"];

fn sorted(mut v: Vec<&'static str>) -> Vec<&'static str> {
    v.sort_unstable();
    v
}

#[test]
fn inventory_lists_all_commands() {
    let ids = sorted(cmds_inventory::all().map(|e| e.id).collect());
    assert_eq!(ids, EXPECTED);
    let mut cx = cmds_inventory::CommandContext::default();
    for e in cmds_inventory::all() {
        (e.run)(&mut cx).unwrap();
    }
    assert_eq!(cx.log.len(), 3);
}

#[test]
fn linkme_lists_all_commands() {
    let ids = sorted(cmds_linkme::all().map(|e| e.id).collect());
    assert_eq!(ids, EXPECTED);
    let mut cx = cmds_linkme::CommandContext::default();
    for e in cmds_linkme::all() {
        (e.run)(&mut cx).unwrap();
    }
    assert_eq!(cx.log.len(), 3);
}

#[test]
fn buildrs_lists_all_commands() {
    let ids = sorted(cmds_buildrs::all().map(|e| e.id).collect());
    assert_eq!(ids, EXPECTED);
    let mut cx = cmds_buildrs::CommandContext::default();
    for e in cmds_buildrs::all() {
        (e.run)(&mut cx).unwrap();
    }
    assert_eq!(cx.log.len(), 3);
}
