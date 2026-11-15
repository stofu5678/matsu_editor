# zed-industries/zed#56349: evidence from one run

Collected 2026-11-15 15:31 +09:00 by `docs/upstream/zed-56349-evidence.ps1` from stofu5678/matsu_editor@32ec53fdafe5. The script was written with Claude Code (an AI) and reviewed with Codex (an AI); the numbers come from running it on the machine below. Memory is in MiB (1024 x 1024 bytes). Most windows were run 3 times; a cell with several runs shows the median and the range (min-max). Raw data: `measurements.csv`, `runs/`, `captures/`, `log.txt`; the script and the patches: `inputs/`.

## Machine

- Windows: Microsoft Windows 11 Pro 26H2, build 26300.9974, Arm64
- Model: Dell Inc. XPS 13 9345
- CPU: Snapdragon(R) X Elite - X1E80100 - Qualcomm(R) Oryon(TM) CPU
- Memory: 63.5 GiB
- GPU: Qualcomm(R) Adreno(TM) X1-85 GPU, driver 31.0.170.0 (2026-07-25)
- Display: 2880x1800
- Display scale: AppliedDPI 144 (150 %)
- Font smoothing: FontSmoothing 2, FontSmoothingType 2 (2 is ClearType; GPUI then draws glyphs as Subpixel)
- BIZ UDGothic (used by minimal): installed
- PowerShell: 5.1.26100.9974
- Rust: rustc 1.99.0, host aarch64-pc-windows-msvc; cargo 1.99.0 (5f94df478 2026-08-27)
- Git: git version 2.51.2.windows.1
- Zed main: zed-industries/zed@e2f00a41ac67 (2026-11-15T05:20:14Z), crates/gpui_windows/src/directx_atlas.rs: UpdateSubresource( on line 304; D3D11_USAGE_STAGING appears 0 times

Comment: "Surface-class Windows 11 ARM64 (26200), Qualcomm Adreno X1-85, driver 31.0.160.0"; "`crates/gpui_windows/src/directx_atlas.rs` on current `main` still uploads with `UpdateSubresource`" (posted 2026-10-09).

## Builds

- Prototype: stofu5678/matsu_editor@97908ca, `poc/gpui-textview`, taken out with `git archive`. `minimal` is a GPUI window with one line of text; `gpui-textview-poc` is the prototype editor (a menu bar and a text view).
- gpui: as published, `cargo build --release --locked` (gpui-pre 0.3.8, built from zed 279fe070bb389b79652e52065b2f001edcc0b11b).
- fix: gpui-pre-windows 0.3.8 from the `.crate` in Cargo's cache (SHA-256 371f9bb13f97e3bea2f8b8e5e81232bb74d735153c244ae67f7400e743925b1c, the checksum in Cargo.lock) with `gpui-pre-windows-atlas-upload.diff` at 97908ca (SHA-256 6e9ee0f5b8f633de5001327e94e80475f8bd3981f370be8fc4b22ef9bddb8e1a), the patch the comment links to.
- diag: the same crate with `gpui-pre-windows-atlas-diag.diff` (SHA-256 b3be0c2dd84f65b4c3c071fada33d49a57808cef0b85c207b3b25fc62d6fe122), which logs the private bytes right before and after each atlas texture creation and upload (see section 3).
- fix and diag: after each build, Cargo.lock differed from the prototype's only in the `source` and `checksum` lines of gpui-pre-windows 0.3.8 (the script stops otherwise), so every other crate is the same version as in gpui.
- All six programs are ARM64 (PE header); Rust 1.99.0, host aarch64-pc-windows-msvc.

## 1. Memory 6 s after start

Working set and private bytes of the process (Get-Process; Task Manager calls them "Working set (memory)" and "Commit size"), read 6 s after start. Then the window was closed.

| Window | gpui: working set | gpui: private | fix: working set | fix: private | Comment: working set, current -> staging |
|---|---|---|---|---|---|
| Minimal GPUI window, one line of text | 84.0 (84.0-91.0) | 98.2 (98.2-98.7) | 48.7 (48.3-55.8) | 62.7 (62.3-62.7) | 84 MB -> 48 MB |
| Same, plus a line of Japanese in the system UI font | 148.2 (147.8-148.2) | 162.5 (162.5-162.7) | 49.2 (49.0-50.3) | 62.5 (62.4-63.8) | 147 MB -> 48 MB |
| Prototype editor at startup | 308.2 (307.7-315.5) | 319.9 (319.9-320.5) | 54.8 (53.5-61.0) | 65.2 (64.2-65.3) | 307 MB -> 54 MB |
| Second row, `TextRenderingMode::Grayscale` | 70.2 (70.2-76.7) | 84.4 (84.3-84.6) | not run | not run | no number ("smaller but still present") |
| Prototype, `TextRenderingMode::Grayscale` | 198.1 (198.0-204.7) | 209.8 (209.8-210.2) | not run | not run | no number |

Prototype (gpui), one more run read at 6 s and 40 s: working set 308.0 -> 309.0, private 319.8 -> 319.8. Comment: "it is never released (still there after 40 s)".

The prototype printed its first frame 159-417 ms after the start of its main(), over all 33 of its runs that printed it (every build and setting; `stderr.txt`).

## 2. Memory breakdown 6 s after start (memstat)

From `memstat-<pid>.txt`, which the prototype writes with `POC_MEMSTAT=6` (`src/memstat.rs`, also written with an AI). Rust heap: a counting global allocator. Win32 heaps: `HeapSummary`. Private allocations: `VirtualQuery` regions grouped by allocation, counted by committed size rounded up to whole MB, so an allocation a little over 4 MiB is counted as 5 MB; only the 12 sizes with the largest working set are listed.

| Prototype | gpui | fix | diag (with the atlas log; its row is in section 3) | Comment |
|---|---|---|---|---|
| Rust heap live, MB | 2.7 | 2.7 | 2.7 | ~3 MB |
| Win32 heaps committed, MB | 7.3 | 7.3 | 7.3 | ~8 MB |
| Private allocations counted as 5 MB | 61 (working set 244.5 MiB) | 1 (working set 3.8 MiB) | 61 (working set 244.5 MiB) | "~4 MB private allocations: 61 of them (~244 MB)" |
| Private allocations counted as 4 MB | 1 (working set 3.5 MiB) | 1 (working set 3.1 MiB) | 1 (working set 3.1 MiB) | |

## 3. Atlas texture creations and uploads (diag build)

Each line of `atlas-<pid>.txt` has the private bytes (`K32GetProcessMemoryInfo`, `PrivateUsage`) right before and right after one call. "About the texture size" means within 10 % of the size of the texture that the upload wrote to. The sum adds up the changes during the calls only. The logging itself puts time between uploads; section 4 compares the same calls with and without it, the other settings being the same.

| Run | Call | Textures created | Uploads | Grew by about the texture size | Grew by other amounts | Did not grow | Shrank / unknown | Sum |
|---|---|---|---|---|---|---|---|---|
| diag-poc-log-1 | UpdateSubresource | Subpixel[0] 1024x1024 R8G8B8A8_UNORM, 4096 KiB (private +4124 KiB) | 148 | 64 (+4108 to +4116 KiB; uploads 1-64) | 2 (+132 to +792 KiB; uploads 65, 129) | 82 | 0 / 0 | 257.7 MiB |
| diag-poc-update1-log-1 | UpdateSubresource1 (D3D11_COPY_NO_OVERWRITE) | Subpixel[0] 1024x1024 R8G8B8A8_UNORM, 4096 KiB (private +4128 KiB) | 149 | 64 (+4108 to +4116 KiB; uploads 1-64) | 2 (+136 to +812 KiB; uploads 65, 129) | 83 | 0 / 0 | 257.7 MiB |
| diag-poc-flush-log-1 | UpdateSubresource + Flush | Subpixel[0] 1024x1024 R8G8B8A8_UNORM, 4096 KiB (private +4124 KiB) | 149 | 8 (+4108 to +4376 KiB; uploads 3-6, 8-9, 13, 18) | 6 (+132 to +4936 KiB; uploads 1-2, 7, 10, 14, 17) | 135 | 0 / 0 | 43.1 MiB |
| diag-poc-log-2 | UpdateSubresource | Subpixel[0] 1024x1024 R8G8B8A8_UNORM, 4096 KiB (private +4124 KiB) | 149 | 64 (+4108 to +4112 KiB; uploads 1-64) | 2 (+132 to +792 KiB; uploads 65, 129) | 83 | 0 / 0 | 257.7 MiB |
| diag-poc-update1-log-2 | UpdateSubresource1 (D3D11_COPY_NO_OVERWRITE) | Subpixel[0] 1024x1024 R8G8B8A8_UNORM, 4096 KiB (private +4124 KiB) | 149 | 64 (+4108 to +4112 KiB; uploads 1-64) | 2 (+132 to +792 KiB; uploads 65, 129) | 83 | 0 / 0 | 257.7 MiB |
| diag-poc-flush-log-2 | UpdateSubresource + Flush | Subpixel[0] 1024x1024 R8G8B8A8_UNORM, 4096 KiB (private +4124 KiB) | 149 | 4 (+4108 to +4372 KiB; uploads 2, 84, 87-88) | 3 (+264 to +4900 KiB; uploads 1, 6, 86) | 142 | 0 / 0 | 21.7 MiB |
| diag-poc-log-3 | UpdateSubresource | Subpixel[0] 1024x1024 R8G8B8A8_UNORM, 4096 KiB (private +4124 KiB) | 149 | 64 (+4108 to +4116 KiB; uploads 1-64) | 3 (+4 to +812 KiB; uploads 65, 124, 129) | 82 | 0 / 0 | 257.7 MiB |
| diag-poc-update1-log-3 | UpdateSubresource1 (D3D11_COPY_NO_OVERWRITE) | Subpixel[0] 1024x1024 R8G8B8A8_UNORM, 4096 KiB (private +4124 KiB) | 149 | 64 (+4108 to +4116 KiB; uploads 1-64) | 2 (+132 to +792 KiB; uploads 65, 129) | 83 | 0 / 0 | 257.7 MiB |
| diag-poc-flush-log-3 | UpdateSubresource + Flush | Subpixel[0] 1024x1024 R8G8B8A8_UNORM, 4096 KiB (private +4124 KiB) | 148 | 3 (+4108 to +4372 KiB; uploads 2, 33, 36) | 4 (+132 to +4900 KiB; uploads 1, 23, 35, 148) | 141 | 0 / 0 | 17.6 MiB |
| diag-poc-memstat-log-1 | UpdateSubresource | Subpixel[0] 1024x1024 R8G8B8A8_UNORM, 4096 KiB (private +4124 KiB) | 149 | 64 (+4108 to +4112 KiB; uploads 1-64) | 2 (+140 to +792 KiB; uploads 65, 129) | 83 | 0 / 0 | 257.7 MiB |
| diag-poc-gray-log-1 | UpdateSubresource | Monochrome[0] 1024x1024 R8_UNORM, 1024 KiB (private +1028 KiB) | 149 | 149 (+1028 to +1064 KiB; uploads 1-149) | 0 | 0 | 0 / 0 | 150.0 MiB |
| diag-minimal-log-1 | UpdateSubresource | Subpixel[0] 1024x1024 R8G8B8A8_UNORM, 4096 KiB (private +4124 KiB) | 9 | 9 (+4108 KiB; uploads 1-9) | 0 | 0 | 0 / 0 | 36.1 MiB |
| diag-minimal-jp-log-1 | UpdateSubresource | Subpixel[0] 1024x1024 R8G8B8A8_UNORM, 4096 KiB (private +4124 KiB) | 25 | 25 (+4108 KiB; uploads 1-25) | 0 | 0 | 0 / 0 | 100.3 MiB |

In these logs, the last upload is at most 0.086 s after the log's first line (each line starts with the seconds since the first line).

Comment: "Only one atlas texture is created (Subpixel, 1024x1024 RGBA = 4 MB). Logging `PrivateUsage` around each call showed that every `UpdateSubresource` into that texture adds exactly +4.0 MB"; "With `TextRenderingMode::Grayscale` the atlas is R8 (1 MB per page) and the growth is smaller but still present"; "CJK text makes it much worse: more distinct glyphs, so more uploads".

## 4. The upload calls with and without the logging (diag build, prototype, 6 s after start)

| Call | Logged: working set | Not logged (`POC_ATLAS_LOG=0`): working set | Not logged: private |
|---|---|---|---|
| `UpdateSubresource` (as GPUI) | 308.1 (308.0-314.9) | 308.0 (307.9-308.0) | 319.8 (319.8-319.8) |
| `UpdateSubresource1` with `D3D11_COPY_NO_OVERWRITE` | 307.8 (307.6-308.0) | 307.5 (307.5-307.8) | 319.8 (319.8-319.8) |
| `UpdateSubresource`, then `Flush()` | 72.3 (68.1-94.0) | 80.8 (77.0-128.4) | 92.0 (87.7-139.3) |

`UpdateSubresource`, then `Flush()`, not logged, one more run read at 6 s and 40 s: private 113.4 -> 113.3.
Runs with `POC_ATLAS_LOG=0` that still wrote an atlas log: 0.

Comment: "What did not help: `ID3D11DeviceContext1::UpdateSubresource1` with `D3D11_COPY_NO_OVERWRITE`; `Flush()` after each upload".

## 5. Rendering

Pictures of the first run of each window (PrintWindow with `PW_RENDERFULLCONTENT`, after the 6 s reading), gpui against fix:

- Minimal GPUI window: 68133 of 1683332 pixels differ, all within x 0-1521, y 0-44 (`captures/gpui-minimal-1.png`, `captures/fix-minimal-1.png`)
- Same, plus a line of Japanese: 0 of 1683332 pixels differ (`captures/gpui-minimal-jp-1.png`, `captures/fix-minimal-jp-1.png`)
- Prototype editor: 275 of 1683332 pixels differ, all within x 459-487, y 1072-1084 (`captures/gpui-poc-1.png`, `captures/fix-poc-1.png`)

The prototype's status bar shows the time to its first frame in milliseconds, which differs from run to run. The window title of the prototype at 97908ca still has the project's old name. The title bar looks different when the window is not the active one, so a difference only in the title bar can come from which window had the focus when the picture was taken.

Comment: "rendering looks the same".

## 6. Zed main

- zed-industries/zed@e2f00a41ac67 (2026-11-15T05:20:14Z), crates/gpui_windows/src/directx_atlas.rs: UpdateSubresource( on line 304; D3D11_USAGE_STAGING appears 0 times
- The file is the same as src/directx_atlas.rs in the published gpui-pre-windows 0.3.8, which the three builds start from (line ends aside).

## Runs

50 windows were started one at a time, and each ended within 5 s of being closed. Total time: 9 minutes.

## Not shown by this

- What the driver or the D3D11 runtime does inside a call: the logs show only the process's private bytes around each call.
- Other GPUs.
