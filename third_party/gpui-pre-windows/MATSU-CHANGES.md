# Changes by MATSU Editor

This directory is `gpui-pre-windows` 0.3.8 from crates.io (Apache-2.0, part of GPUI in
https://github.com/zed-industries/zed), copied as published (without cargo's `.cargo-ok` marker), with one
change. The workspace uses it instead of the crates.io copy through `[patch.crates-io]` in the root
`Cargo.toml` (ADR-0004).

## Changed file

`src/directx_atlas.rs` (it starts with a notice comment, as Apache-2.0 section 4 asks):

- `DirectXAtlasTexture::upload` writes the glyph into a small staging texture (`D3D11_USAGE_STAGING`) and
  copies it into the atlas with `CopySubresourceRegion`, instead of calling `UpdateSubresource` on the
  atlas.
- Why: on the Qualcomm Adreno X1-85 driver (31.0.160.0), every `UpdateSubresource` into an atlas texture
  that is in use keeps a copy of about 4 MB. Measured on 2026-11-16, 25 s after start: 397 MB working set
  and 359 MB GPU memory without the change, 73 MB and 27 MB with it.
- Upstream issue: https://github.com/zed-industries/zed/issues/56349

The change as a patch is `third_party/patches/gpui-pre-windows-atlas-upload.diff` (kept outside this
directory, so this directory stays a copy of the crate plus its change). The test
`crates/matsu/tests/vendored_gpui.rs` fails when the staging upload is gone from the file.

## When GPUI is upgraded

1. Upgrade the `gpui-pre*` versions in `Cargo.lock` (and the crates that name them). Check whether the upstream
   issue was fixed; if so, delete `third_party/` and the `[patch.crates-io]` and `exclude` lines in the root
   `Cargo.toml`, and the test.
2. Otherwise copy the new `gpui-pre-windows` from the cargo registry over this directory (delete `.cargo-ok`),
   then apply the patch inside this directory: `patch -p1 < ../patches/gpui-pre-windows-atlas-upload.diff`.
   If it does not apply, redo the staging upload by hand in `src/directx_atlas.rs`, keep the notice comment at
   the top, and regenerate the patch with `diff -u` against the pristine file.
3. Update the version in this file.
4. Run `cargo check -p matsu --target x86_64-pc-windows-msvc` and `cargo test -p matsu --test vendored_gpui`.
