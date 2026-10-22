//! Embeds the icon in matsu.exe (resource ID 1; see `matsu.rc`). Other OSes need nothing here.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=matsu.rc");
    println!("cargo:rerun-if-changed=../../assets/icons/matsu_editor.ico");
    // `cfg!(windows)` is about the machine that builds, so ask for the target.
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        embed_resource::compile("matsu.rc", embed_resource::NONE)
            .manifest_required()
            .expect("embedding the icon failed");
    }
}
