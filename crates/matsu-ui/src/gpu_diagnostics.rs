//! `MATSU_GPU_DIAGNOSTICS`: shows where the graphics memory of a window goes (`docs/poc/gpu-memory.md`).
//!
//! Only when the environment variable is set (to anything but empty or `0`), the program installs the
//! logger of `MATSU_FONT_DIAGNOSTICS` (so GPUI's debug records reach the standard error; the one that
//! matters is `Updating <kind> buffer size from A to B`, which GPUI's DirectX renderer logs each time an
//! instance buffer grows) and prints, to the standard error:
//!
//! - once at the start: the size of one instance of each primitive kind, and what GPUI's DirectX renderer
//!   allocates for a window of a given size (the numbers are computed from GPUI's source, not read from
//!   the driver);
//! - every [`INTERVAL`], when it changed: each window's size, its scale and what its fixed textures take,
//!   and the number of open documents.
//!
//! GPUI does not tell the program how many primitives a frame had or how big its atlas is, so the instance
//! buffer lines of the log are the only measure of a frame's size that MATSU can show.
//!
//! Why a variable of its own and not `MATSU_FONT_DIAGNOSTICS`: that one prints a long report of fonts at
//! the start; this one prints a short line again and again. Nothing is written to a file (where the user's
//! data lives is the user's to decide), and nothing is printed without the variable.

use std::ffi::OsStr;
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui::{App, AsyncApp};

use crate::font_diagnostics::{install_logger, is_on};

/// The name of the environment variable that turns the diagnostics on.
pub(crate) const ENV_NAME: &str = "MATSU_GPU_DIAGNOSTICS";

/// How often the state is looked at (a line is printed only when it changed).
const INTERVAL: Duration = Duration::from_secs(5);

/// `DirectXRenderer`'s `BUFFER_COUNT`: the buffers of a swap chain.
const SWAP_CHAIN_BUFFERS: u64 = 3;
/// `PATH_MULTISAMPLE_COUNT`: the samples of the texture that paths are drawn into.
const PATH_SAMPLES: u64 = 4;
/// Bytes of a pixel of `DXGI_FORMAT_B8G8R8A8_UNORM`, the format of every one of these textures.
const PIXEL_BYTES: u64 = 4;

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Installs the logger if the environment asks for it. Returns whether the diagnostics are on.
pub(crate) fn install_from_env() -> bool {
    install(std::env::var_os(ENV_NAME).as_deref())
}

fn install(value: Option<&OsStr>) -> bool {
    if !is_on(value) {
        return false;
    }
    install_logger();
    ENABLED.store(true, Ordering::Relaxed);
    log::info!(target: "matsu::gpu_diagnostics", "{ENV_NAME} is set: logging to the standard error");
    true
}

/// What GPUI's DirectX renderer allocates for a window of `width` x `height` device pixels, apart from
/// the instance buffers and the atlas (`directx_renderer.rs`: `create_swap_chain*`,
/// `create_path_intermediate_texture`, `create_path_intermediate_msaa_texture_and_view`). The sizes are
/// those that the formats ask for; a driver may add padding (tiled memory, alignment) on top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Fixed {
    /// `BufferCount` (3) buffers of the swap chain.
    pub swap_chain: u64,
    /// The texture that the paths are resolved into: one sample, the size of the window.
    pub path_intermediate: u64,
    /// The texture that the paths are drawn into: [`PATH_SAMPLES`] samples, the size of the window.
    pub path_msaa: u64,
}

impl Fixed {
    pub(crate) fn of(width: u32, height: u32) -> Self {
        let one = u64::from(width.max(1)) * u64::from(height.max(1)) * PIXEL_BYTES;
        Self {
            swap_chain: one * SWAP_CHAIN_BUFFERS,
            path_intermediate: one,
            path_msaa: one * PATH_SAMPLES,
        }
    }

    pub(crate) fn total(self) -> u64 {
        self.swap_chain + self.path_intermediate + self.path_msaa
    }
}

/// Megabytes (10^6 bytes are not used: Task Manager shows MiB), with one decimal.
pub(crate) fn mib(bytes: u64) -> String {
    format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
}

/// The instance buffers' kinds as GPUI's DirectX renderer creates them: its label, the size of one
/// instance and the number of instances of the buffer when it is made.
pub(crate) fn instance_kinds() -> [(&'static str, usize, usize); 6] {
    [
        ("shadow", size_of::<gpui::Shadow>(), 4),
        ("quad", size_of::<gpui::Quad>(), 64),
        ("underline", size_of::<gpui::Underline>(), 4),
        (
            "monochrome sprite",
            size_of::<gpui::MonochromeSprite>(),
            512,
        ),
        ("subpixel sprite", size_of::<gpui::SubpixelSprite>(), 512),
        ("polychrome sprite", size_of::<gpui::PolychromeSprite>(), 16),
    ]
}

/// The size GPUI gives an instance buffer that has to hold `count` instances
/// (`PipelineState::update_buffer`: `count.next_power_of_two()`), in instances. The buffer is never
/// made smaller again.
pub(crate) fn buffer_instances_for(count: usize) -> usize {
    count.next_power_of_two()
}

/// The text printed once at the start.
pub(crate) fn start_text() -> String {
    let mut out = format!("== {ENV_NAME}: what GPUI's DirectX renderer allocates ==\n");
    out.push_str("per instance (bytes), instances of a new buffer, bytes of a new buffer:\n");
    for (label, size, initial) in instance_kinds() {
        out.push_str(&format!(
            "  {label}: {size} B x {initial} = {} B\n",
            size * initial
        ));
    }
    out.push_str(
        "an instance buffer grows to the next power of two of the largest frame so far, and is never \
         made smaller (the log line `Updating <kind> buffer size from A to B` shows each growth)\n",
    );
    let (_, sprite_size, _) = instance_kinds()[3];
    for count in [3_000, 10_000, 40_000] {
        let instances = buffer_instances_for(count);
        out.push_str(&format!(
            "  e.g. a frame of {count} monochrome sprites: a buffer of {instances} = {}\n",
            mib((instances * sprite_size) as u64)
        ));
    }
    out.push_str(
        "fixed textures of a window (3 swap chain buffers, path texture, 4x MSAA path texture):\n",
    );
    for (width, height) in [(1200, 900), (1782, 1180), (2560, 1440)] {
        let fixed = Fixed::of(width, height);
        out.push_str(&format!(
            "  {width}x{height}: {} (swap chain {}, path {}, path MSAA {})\n",
            mib(fixed.total()),
            mib(fixed.swap_chain),
            mib(fixed.path_intermediate),
            mib(fixed.path_msaa)
        ));
    }
    out.push_str("== end ==\n");
    out
}

/// One window as the line shows it: logical size, scale, device size and the fixed textures.
pub(crate) fn window_line(index: usize, logical: (f32, f32), scale: f32) -> String {
    let device = (
        (logical.0 * scale).round().max(1.0) as u32,
        (logical.1 * scale).round().max(1.0) as u32,
    );
    let fixed = Fixed::of(device.0, device.1);
    format!(
        "window {index}: {}x{} logical at {}% = {}x{} px, fixed textures {}",
        logical.0.round(),
        logical.1.round(),
        (scale * 100.0).round(),
        device.0,
        device.1,
        mib(fixed.total())
    )
}

/// Prints the start text and then a line every [`INTERVAL`] while the state changes. Without the
/// variable it does nothing.
pub(crate) fn start(cx: &mut App) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    // One write, so that the lines of a record from another thread do not split it.
    write!(std::io::stderr().lock(), "{}", start_text()).ok();
    cx.spawn(async move |cx| watch(cx).await).detach();
}

async fn watch(cx: &mut AsyncApp) {
    let mut last = String::new();
    loop {
        cx.background_executor().timer(INTERVAL).await;
        let state = cx.update(state_text);
        if state != last {
            let time = chrono::Local::now().format("%H:%M:%S%.3f");
            writeln!(std::io::stderr().lock(), "{time} [GPU] {state}").ok();
            last = state;
        }
    }
}

/// The windows and the documents, as one line (the windows separated by `; `).
fn state_text(cx: &mut App) -> String {
    let handles = cx.windows();
    let mut windows = Vec::new();
    let mut fixed = 0;
    for (index, handle) in handles.iter().enumerate() {
        let seen = handle
            .update(cx, |_, window, _| {
                let size = window.viewport_size();
                (
                    f32::from(size.width),
                    f32::from(size.height),
                    window.scale_factor(),
                )
            })
            .ok();
        if let Some((w, h, scale)) = seen {
            windows.push(window_line(index, (w, h), scale));
            fixed += Fixed::of((w * scale).round() as u32, (h * scale).round() as u32).total();
        }
    }
    let documents = crate::file_ops::windows::editors(cx).len();
    format!(
        "{} windows (fixed textures {} in all), {documents} documents open; {}",
        handles.len(),
        mib(fixed),
        windows.join("; ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixed_textures_follow_gpuis_formats() {
        // 1782x1180 at 4 bytes: 8_411_040 B a texture; 3 swap chain buffers, 1 path texture and a
        // 4-sample one make 8 textures.
        let fixed = Fixed::of(1782, 1180);
        assert_eq!(fixed.swap_chain, 3 * 8_411_040);
        assert_eq!(fixed.path_intermediate, 8_411_040);
        assert_eq!(fixed.path_msaa, 4 * 8_411_040);
        assert_eq!(fixed.total(), 8 * 8_411_040);
    }

    #[test]
    fn a_window_has_at_least_one_pixel() {
        assert_eq!(Fixed::of(0, 0).total(), 8 * PIXEL_BYTES);
    }

    #[test]
    fn a_buffer_grows_to_the_next_power_of_two() {
        assert_eq!(buffer_instances_for(513), 1024);
        assert_eq!(buffer_instances_for(512), 512);
        assert_eq!(buffer_instances_for(1), 1);
    }

    #[test]
    fn the_variable_turns_it_on_unless_empty_or_zero() {
        assert!(!install(None));
        assert!(!install(Some(OsStr::new(""))));
        assert!(!install(Some(OsStr::new("0"))));
        assert!(!ENABLED.load(Ordering::Relaxed));
    }

    #[test]
    fn the_start_text_names_every_kind_and_the_sizes() {
        let text = start_text();
        for (label, size, _) in instance_kinds() {
            assert!(text.contains(label));
            assert!(size > 0 && size % 4 == 0);
        }
        assert!(text.contains("1782x1180"));
        assert!(text.contains("never made smaller"));
    }

    #[test]
    fn a_window_line_gives_the_device_size() {
        let line = window_line(0, (1188.0, 787.0), 1.5);
        assert!(
            line.contains("1188x787 logical at 150% = 1782x1181 px"),
            "{line}"
        );
    }
}
