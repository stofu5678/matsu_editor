//! Lists the commands of each option. The app names no command: if the linker dropped a
//! command file because nothing refers to it, it would be missing here.

fn ids<'a>(all: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut v: Vec<_> = all.collect();
    v.sort_unstable();
    v
}

fn main() {
    println!("inventory: {:?}", ids(cmds_inventory::all().map(|e| e.id)));
    println!("linkme:    {:?}", ids(cmds_linkme::all().map(|e| e.id)));
    println!("build.rs:  {:?}", ids(cmds_buildrs::all().map(|e| e.id)));
}
