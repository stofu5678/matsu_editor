//! Embeds the icon and the version information in matsu.exe (see `matsu.rc`). Other OSes need nothing here.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=matsu.rc");
    println!("cargo:rerun-if-changed=../../assets/icons/matsu_editor.ico");
    // `cfg!(windows)` is about the machine that builds, so ask for the target.
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        embed_resource::compile(resource_file(), embed_resource::NONE)
            .manifest_required()
            .expect("embedding the icon and the version information failed");
    }
}

/// Writes `matsu.rc` with its placeholders filled in (the icon's path and the crate's version) to the
/// build's output folder, and returns its path.
fn resource_file() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env("CARGO_MANIFEST_DIR"));
    // The resource compiler reads forward slashes in a string, but takes a backslash as an escape.
    let icon = manifest_dir
        .join("../../assets/icons/matsu_editor.ico")
        .to_string_lossy()
        .replace('\\', "/");
    let version = env("CARGO_PKG_VERSION");
    // `FILEVERSION` takes four numbers: major, minor, patch and a build number (always 0 here).
    // A pre-release suffix ("0.14.0-rc.1") is left out of the numbers and kept in the text.
    let numbers: Vec<&str> = version
        .split(['-', '+'])
        .next()
        .unwrap_or_default()
        .split('.')
        .collect();
    assert!(
        numbers.len() == 3 && numbers.iter().all(|n| n.parse::<u16>().is_ok()),
        "the crate version {version} is not major.minor.patch"
    );
    let version_comma = format!("{},0", numbers.join(","));

    let template =
        std::fs::read_to_string(manifest_dir.join("matsu.rc")).expect("reading matsu.rc");
    let filled = template
        .replace("@ICON@", &icon)
        .replace("@VERSION_COMMA@", &version_comma)
        .replace("@VERSION@", &version);
    let path = std::path::PathBuf::from(env("OUT_DIR")).join("matsu.rc");
    std::fs::write(&path, filled).expect("writing the filled matsu.rc");
    path
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("Cargo did not set {name}"))
}
