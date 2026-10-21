//! Embeds the icon in ume.exe (resource ID 1; see `ume.rc`). Other OSes need nothing here.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=ume.rc");
    println!("cargo:rerun-if-changed=../../assets/icons/ume_editor.ico");
    // `cfg!(windows)` is about the machine that builds, so ask for the target.
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        embed_resource::compile("ume.rc", embed_resource::NONE)
            .manifest_required()
            .expect("embedding the icon failed");
    }
}
