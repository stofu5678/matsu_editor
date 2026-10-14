//! Builds the vendored Onigmo (see `onigmo/README.md`) and the C shim.

use std::path::Path;

/// Onigmo sources the UTF-8-only build needs.
const ONIGMO_SOURCES: &[&str] = &[
    "regcomp.c",
    "regenc.c",
    "regerror.c",
    "regexec.c",
    "regparse.c",
    "regsyntax.c",
    "regversion.c",
    "st.c",
    "enc/ascii.c",
    "enc/unicode.c",
    "enc/utf_8.c",
];

fn main() {
    let onigmo = Path::new("onigmo");
    let mut build = cc::Build::new();
    build
        .include("c") // our config.h
        .include(onigmo)
        .include(onigmo.join("enc/unicode"))
        // Static link: no dllimport/dllexport decoration on MSVC.
        .define("ONIG_EXTERN", "extern")
        .warnings(false)
        .file("c/shim.c");
    for source in ONIGMO_SOURCES {
        build.file(onigmo.join(source));
    }
    build.compile("onigmo");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=c");
    println!("cargo:rerun-if-changed=onigmo");
}
