//! Scans `src/commands/<group>/<name>.rs` and writes `commands_generated.rs`.
//!
//! The struct of `delete_line.rs` must be `DeleteLine` (snake_case file, CamelCase struct).
//! A real version would check this and fail with a clear message.

use std::fmt::Write as _;
use std::path::Path;
use std::{env, fs};

fn camel(snake: &str) -> String {
    snake
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().chain(c).collect::<String>())
                .unwrap_or_default()
        })
        .collect()
}

fn sorted_entries(dir: &Path) -> Vec<fs::DirEntry> {
    let mut v: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|e| e.expect("directory entry"))
        .collect();
    v.sort_by_key(|e| e.file_name());
    v
}

fn main() {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("src/commands");
    // Rescans when a file is added or removed in the directory tree.
    println!("cargo:rerun-if-changed=src/commands");

    let mut modules = String::from("pub mod commands {\n");
    let mut list = String::from("pub static ALL: &[CommandEntry] = &[\n");
    for group in sorted_entries(&root) {
        if !group.file_type().unwrap().is_dir() {
            continue;
        }
        let g = group.file_name().into_string().unwrap();
        writeln!(modules, "    pub mod {g} {{").unwrap();
        for file in sorted_entries(&group.path()) {
            let path = file.path();
            if path.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let f = path.file_stem().unwrap().to_str().unwrap().to_string();
            println!("cargo:rerun-if-changed={}", path.display());
            writeln!(
                modules,
                "        #[path = {:?}]\n        pub mod {f};",
                path.display().to_string()
            )
            .unwrap();
            writeln!(
                list,
                "    CommandEntry::of::<commands::{g}::{f}::{}>(),",
                camel(&f)
            )
            .unwrap();
        }
        modules.push_str("    }\n");
    }
    modules.push_str("}\n");
    list.push_str("];\n");

    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("commands_generated.rs");
    fs::write(out, format!("{modules}\n{list}")).unwrap();
}
