//! The vendored GPUI Windows backend keeps its change (ADR-0004, `third_party/gpui-pre-windows/MATSU-CHANGES.md`).
//!
//! On the Qualcomm Adreno driver, `UpdateSubresource` into an atlas in use keeps about 4 MB per glyph, so the
//! atlas upload goes through a staging texture. The real effect needs that driver; this test only fails when a
//! GPUI upgrade copies a fresh crate over the vendored one and drops the change.

use std::path::PathBuf;

fn vendored_file(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/gpui-pre-windows")
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

#[test]
fn atlas_upload_goes_through_a_staging_texture() {
    let source = vendored_file("src/directx_atlas.rs");
    assert!(
        source.contains("D3D11_USAGE_STAGING"),
        "the staging texture upload is gone from the vendored directx_atlas.rs; re-apply \
         third_party/patches/gpui-pre-windows-atlas-upload.diff (see MATSU-CHANGES.md)"
    );
    assert!(source.contains("CopySubresourceRegion"));
    assert!(
        !source.contains("UpdateSubresource("),
        "the atlas must not be written with UpdateSubresource"
    );
}

#[test]
fn modified_file_carries_the_notice() {
    let source = vendored_file("src/directx_atlas.rs");
    assert!(
        source
            .lines()
            .take(5)
            .any(|l| l.contains("Modified by the MATSU Editor")),
        "Apache-2.0 section 4: the modified file needs a notice at the top"
    );
}
