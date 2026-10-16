//! Compiles the vendored mruby (vendor/mruby/README.md says what is in it) and the glue.
//!
//! The list of sources is read off the vendor directory, so that regenerating it with
//! `vendor/mruby/tools/regen.sh` needs no change here. The defines must be the ones that
//! `vendor/mruby/tools/build_config.rb` used to generate the symbol tables.

use std::fs;
use std::path::{Path, PathBuf};

fn c_files(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "c"))
        .collect();
    files.sort();
    into.extend(files);
}

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let mruby = root.join("vendor/mruby");

    let mut sources = Vec::new();
    let mut includes = vec![mruby.join("include"), mruby.join("generated/include")];
    c_files(&mruby.join("src"), &mut sources);
    c_files(&mruby.join("generated"), &mut sources);
    c_files(&mruby.join("generated/mrbgems"), &mut sources);
    let mut gems: Vec<PathBuf> = fs::read_dir(mruby.join("gems"))
        .expect("vendor/mruby/gems")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    gems.sort();
    for gem in gems {
        for sub in ["src", "core"] {
            c_files(&gem.join(sub), &mut sources);
        }
        if gem.join("include").is_dir() {
            includes.push(gem.join("include"));
        }
    }
    sources.push(root.join("glue.c"));

    let mut build = cc::Build::new();
    build
        .files(&sources)
        .includes(&includes)
        // MRB_UTF8_STRING is in tools/build_config.rb; the other two are what the gems
        // mruby-bigint and mruby-set add to the build.
        .define("MRB_UTF8_STRING", None)
        .define("MRB_USE_BIGINT", None)
        .define("MRB_USE_SET", None)
        // It is not our code, and the warnings of one compiler differ from another's.
        .warnings(false)
        .extra_warnings(false);
    if build.get_compiler().is_like_msvc() {
        build.define("_CRT_SECURE_NO_WARNINGS", None);
    }
    build.compile("ume_mruby");

    println!("cargo:rerun-if-changed=glue.c");
    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=build.rs");
}
