//! Builds the vendored MicroPython (the sources the embed port generated) and the glue next to
//! it with the C compiler of the target. No assembly and no libclang: the non-local returns
//! use setjmp (see embed/mpconfigport.h).

use std::path::Path;

fn main() {
    let vendor = Path::new("vendor/micropython");
    let mut build = cc::Build::new();
    build
        .include(vendor)
        .include(vendor.join("port"))
        .include("csrc")
        .include("embed")
        .warnings(false)
        // MicroPython's object model casts between pointer types freely.
        .flag_if_supported("-fno-strict-aliasing")
        .define("_CRT_SECURE_NO_WARNINGS", None)
        .define("_USE_MATH_DEFINES", None);
    if build.get_compiler().is_like_msvc() {
        // A stand-in for <unistd.h>, which MicroPython includes for `ssize_t`.
        build.include("csrc/msvc");
    }
    // The same C files as the embed port builds, except its HAL file: csrc/ume_stdio.c has
    // the output hooks.
    let sources = [
        vendor.join("py"),
        vendor.join("extmod"),
        vendor.join("shared/runtime"),
        vendor.join("port"),
    ];
    for dir in sources {
        for entry in std::fs::read_dir(&dir).expect("the vendored directory exists") {
            let path = entry.expect("readable").path();
            if path.extension().is_some_and(|e| e == "c") && !skipped(&path) {
                build.file(&path);
            }
        }
        println!("cargo:rerun-if-changed={}", dir.display());
    }
    build.file("csrc/ume_mp.c").file("csrc/ume_stdio.c");
    println!("cargo:rerun-if-changed=csrc");
    build.compile("micropython");
}

/// Files of the vendored directories that the build leaves out.
fn skipped(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    // The embed port's own HAL (it prints with printf); ours is in csrc/. The other files that
    // the configuration switches off (the native-code emitters, say) compile to nothing.
    name == "mphalport.c"
}
