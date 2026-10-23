//! The icons of the toolbar and the menus: `mytool.bmp` of Sakura (`crates/matsu-ui/assets/sakura/NOTICE`),
//! or the user's `my_icons.bmp`, cut into pictures when they are first drawn.
//!
//! Sakura: `CImageListMgr` (`sakura_core/uiparts/CImageListMgr.cpp`) keeps the bitmap as 32
//! icons of 16x16 pixels in a row, 15 rows (`MAX_TOOLBAR_ICON_X`, `MAX_TOOLBAR_ICON_Y`,
//! `sakura_core/env/CommonSetting.h`); icon N is at column `N % 32`, row `N / 32`. The colour of
//! the top left pixel is the transparent colour (`ResizeToolIcons`). GPUI has no way to draw part
//! of a bitmap with a transparent colour, so each icon becomes a small PNG with an alpha channel
//! (written here, without compression, so that no crate is needed), which GPUI decodes.
//!
//! # `my_icons.bmp`
//!
//! `CImageListMgr::Create` loads `my_icons.bmp` (`FN_TOOL_BMP`, `system_constants.h`) with
//! `LoadImage` from the file `GetInidirOrExedir` finds: the one in the folder of the settings
//! file, else the one next to the program (`util/file.cpp`: the first path that exists). When
//! that fails, the bitmap built into the program is used. Here [`install`] does the same once
//! when the application starts; a file that cannot be read as a bitmap (not a bitmap, a format
//! `LoadImage` cannot read, cut short) leaves the built-in icons. A bitmap that is smaller than
//! 528x240 pixels has no pixels for the icons beyond its edge: Sakura reads past the end of the
//! bitmap for them (`DrawToolIcon` does not check), here they have no picture.
//!
//! # Sharp icons on high-DPI screens
//!
//! GPUI draws a picture with linear filtering, so a 16x16 icon drawn at 125 % or 150 % turns
//! blurry. The icons are therefore made bigger by a whole number [`pixel_factor`] (each dot
//! becomes a square of dots, nearest neighbour) and drawn at the size that makes one picture dot
//! exactly one screen dot ([`logical_size`]); GPUI snaps the bounds of a picture to the screen's
//! dots (`Window::paint_image`), so nothing is resampled.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use gpui::{Image, ImageFormat};

/// Sakura's `mytool.bmp`, unchanged.
static MYTOOL_BMP: &[u8] = include_bytes!("../../assets/sakura/mytool.bmp");

/// `FN_TOOL_BMP`: the name of the file that replaces the built-in bitmap.
pub(crate) const FILE_NAME: &str = "my_icons.bmp";

/// `cx` and `cy` of an icon.
pub(crate) const ICON_SIZE: usize = 16;
/// `MAX_TOOLBAR_ICON_X`: icons in a row.
const COLUMNS: usize = 32;
/// `MAX_TOOLBAR_ICON_Y`: rows of icons.
const ROWS: usize = 15;
/// `MAX_TOOLBAR_ICON_COUNT`.
pub(crate) const ICON_COUNT: usize = COLUMNS * ROWS;

/// The bitmap as RGBA pixels, row by row from the top.
pub(crate) struct Sheet {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

fn le_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let b = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Reads a Windows bitmap without compression, of 1, 4, 8, 24 or 32 bits per pixel, upside down
/// (the usual) or not: what `LoadImage` reads of the files that matter for icons. The pixel at
/// the top left is the transparent colour (`ResizeToolIcons`); a bitmap with an alpha channel
/// is taken as opaque, as `LoadImage` + `BitBlt` do. `None` for anything else.
fn decode_bmp(bytes: &[u8]) -> Option<Sheet> {
    if bytes.get(..2)? != b"BM" {
        return None;
    }
    let data_at = le_u32(bytes, 10)? as usize;
    let header_size = le_u32(bytes, 14)? as usize;
    let width = le_u32(bytes, 18)? as i32;
    let height = le_u32(bytes, 22)? as i32;
    let bits = usize::from(u16::from_le_bytes([*bytes.get(28)?, *bytes.get(29)?]));
    let compression = le_u32(bytes, 30)?;
    if header_size < 40 || compression != 0 || width <= 0 || height == 0 {
        return None;
    }
    if !matches!(bits, 1 | 4 | 8 | 24 | 32) {
        return None;
    }
    let top_down = height < 0;
    let (width, height) = (width as usize, height.unsigned_abs() as usize);
    // Each row is padded to a multiple of 4 bytes. Check the size first: a header must not make
    // us allocate what the file does not hold.
    let stride = (width.checked_mul(bits)?).div_ceil(32) * 4;
    let needed = stride.checked_mul(height)?;
    if bytes.len() < data_at.checked_add(needed)? {
        return None;
    }
    let palette: Vec<[u8; 3]> = if bits <= 8 {
        let used = le_u32(bytes, 46)? as usize;
        let count = if used == 0 || used > 1 << bits {
            1 << bits
        } else {
            used
        };
        let palette_at = 14 + header_size;
        (0..1usize << bits)
            .map(|i| {
                if i >= count {
                    return Some([0, 0, 0]);
                }
                let p = bytes.get(palette_at + i * 4..palette_at + i * 4 + 3)?;
                Some([p[2], p[1], p[0]])
            })
            .collect::<Option<_>>()?
    } else {
        Vec::new()
    };
    let mut rgba = vec![0u8; width * height * 4];
    for y in 0..height {
        let source_row = if top_down { y } else { height - 1 - y };
        let row = bytes.get(data_at + source_row * stride..)?;
        for x in 0..width {
            let [r, g, b] = match bits {
                24 | 32 => {
                    let p = row.get(x * bits / 8..x * bits / 8 + 3)?;
                    [p[2], p[1], p[0]]
                }
                8 => palette[usize::from(*row.get(x)?)],
                4 => {
                    let byte = *row.get(x / 2)?;
                    palette[usize::from(if x % 2 == 0 { byte >> 4 } else { byte & 15 })]
                }
                _ => palette[usize::from((*row.get(x / 8)? >> (7 - x % 8)) & 1)],
            };
            let at = (y * width + x) * 4;
            rgba[at..at + 4].copy_from_slice(&[r, g, b, 255]);
        }
    }
    let transparent = [rgba[0], rgba[1], rgba[2]];
    for pixel in rgba.as_chunks_mut::<4>().0 {
        if pixel[..3] == transparent {
            pixel[3] = 0;
        }
    }
    Some(Sheet {
        width,
        height,
        rgba,
    })
}

fn builtin() -> Option<&'static Sheet> {
    static SHEET: OnceLock<Option<Sheet>> = OnceLock::new();
    SHEET.get_or_init(|| decode_bmp(MYTOOL_BMP)).as_ref()
}

/// The bitmap `my_icons.bmp` gave, while there is one.
fn external() -> &'static RwLock<Option<Arc<Sheet>>> {
    static EXTERNAL: OnceLock<RwLock<Option<Arc<Sheet>>>> = OnceLock::new();
    EXTERNAL.get_or_init(Default::default)
}

/// The pictures made so far, by icon number and [`pixel_factor`].
type Cache = Mutex<HashMap<(usize, u32), Arc<Image>>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// `GetInidirOrExedir(FN_TOOL_BMP)` and `LoadImage`: `my_icons.bmp` of the folder of the
/// settings file if it is there, else the one of the folder of the program; `None` when there is
/// none or it cannot be read (the built-in bitmap is used then). A file in the first folder that
/// cannot be read is not replaced by the one in the second: `GetInidirOrExedir` only checks that
/// a file exists.
fn load_external(settings_dir: Option<&Path>, program_dir: Option<&Path>) -> Option<Sheet> {
    let path = [settings_dir, program_dir]
        .into_iter()
        .flatten()
        .map(|dir| dir.join(FILE_NAME))
        .find(|path| path.is_file())?;
    decode_bmp(&std::fs::read(path).ok()?)
}

/// Looks for `my_icons.bmp` in `settings_dir` and `program_dir` and makes it the bitmap of the
/// icons, or the built-in one when there is none that can be read.
pub(crate) fn install_from(settings_dir: Option<&Path>, program_dir: Option<&Path>) {
    let sheet = load_external(settings_dir, program_dir).map(Arc::new);
    *external().write().unwrap_or_else(|e| e.into_inner()) = sheet;
    cache().lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// Reads `my_icons.bmp` for the application: the folder of the settings file, then the folder
/// of the program. Call it once when the application starts.
pub(crate) fn install(user: &crate::settings::UserSettings) {
    // The tests choose the bitmap with `install_from`; the setup of an application under test must
    // not put it back while they run in parallel.
    if cfg!(test) {
        return;
    }
    let settings_dir = user.path().and_then(Path::parent);
    let program = std::env::current_exe().ok();
    install_from(settings_dir, program.as_deref().and_then(Path::parent));
}

/// Whether the icons come from a `my_icons.bmp` now.
#[cfg(test)]
pub(crate) fn is_external() -> bool {
    external()
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .is_some()
}

/// The factor icons are made bigger by for a screen with `scale_factor` (1.0 is 100 %): the
/// whole number that makes the icon closest to the 16 dots times the scale that Sakura draws
/// (`SM_CXSMICON`), and at least 1. A half rounds up. 100 % and 125 % give 1 (16 dots; at
/// 125 % the icon is 12.8 logical pixels, smaller than the 20 dots Sakura stretches it to), 150
/// % to 249 % give 2 (32 dots), 250 % to 349 % give 3. The dots stay sharp; the cost is an icon
/// that is up to a fifth too small or too big, instead of one that is blurred.
pub(crate) fn pixel_factor(scale_factor: f32) -> u32 {
    if !scale_factor.is_finite() || scale_factor < 1.0 {
        return 1;
    }
    (scale_factor.round() as u32).max(1)
}

/// The size, in logical pixels, to draw an icon made with [`pixel_factor`] at so that each of
/// its dots covers exactly one dot of the screen.
pub(crate) fn logical_size(scale_factor: f32) -> f32 {
    let scale = if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    };
    (ICON_SIZE as u32 * pixel_factor(scale)) as f32 / scale
}

fn crc32(bytes: impl IntoIterator<Item = u8>) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in bytes {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(kind.iter().chain(data).copied());
    out.extend_from_slice(&crc.to_be_bytes());
}

/// A PNG of RGBA pixels, with the data stored without compression.
fn encode_png(width: usize, height: usize, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((width * 4 + 1) * height);
    for row in rgba.chunks_exact(width * 4) {
        raw.push(0); // filter: none
        raw.extend_from_slice(row);
    }
    // A zlib stream of stored blocks (each at most 65535 bytes).
    let mut z = vec![0x78, 0x01];
    let mut blocks = raw.chunks(65535).peekable();
    while let Some(block) = blocks.next() {
        z.push(u8::from(blocks.peek().is_none()));
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    z.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut header = Vec::new();
    header.extend_from_slice(&(width as u32).to_be_bytes());
    header.extend_from_slice(&(height as u32).to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]); // 8 bits, RGBA, no interlace
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &z);
    chunk(&mut png, b"IEND", &[]);
    png
}

/// The RGBA pixels of icon `index` (16x16) of `sheet`, or `None` when there is no such icon or
/// the bitmap does not reach it.
fn icon_pixels(sheet: &Sheet, index: usize) -> Option<Vec<u8>> {
    if index >= ICON_COUNT {
        return None;
    }
    let (x0, y0) = ((index % COLUMNS) * ICON_SIZE, (index / COLUMNS) * ICON_SIZE);
    if x0 + ICON_SIZE > sheet.width || y0 + ICON_SIZE > sheet.height {
        return None;
    }
    let mut pixels = Vec::with_capacity(ICON_SIZE * ICON_SIZE * 4);
    for y in y0..y0 + ICON_SIZE {
        let from = (y * sheet.width + x0) * 4;
        pixels.extend_from_slice(&sheet.rgba[from..from + ICON_SIZE * 4]);
    }
    Some(pixels)
}

/// Every dot of `rgba` (`width` x `height`) becomes a square of `factor` x `factor` dots.
fn enlarge(rgba: &[u8], width: usize, height: usize, factor: usize) -> Vec<u8> {
    if factor <= 1 {
        return rgba.to_vec();
    }
    let mut out = Vec::with_capacity(rgba.len() * factor * factor);
    for y in 0..height {
        let mut row = Vec::with_capacity(width * factor * 4);
        for x in 0..width {
            let dot = &rgba[(y * width + x) * 4..(y * width + x) * 4 + 4];
            for _ in 0..factor {
                row.extend_from_slice(dot);
            }
        }
        for _ in 0..factor {
            out.extend_from_slice(&row);
        }
    }
    out
}

/// The RGBA pixels of icon `index` made `factor` times as big, with the size of the picture, from
/// the bitmap in use.
fn scaled_pixels(index: usize, factor: u32) -> Option<(Vec<u8>, usize)> {
    let factor = factor.max(1) as usize;
    let external = external().read().unwrap_or_else(|e| e.into_inner()).clone();
    let pixels = match &external {
        Some(sheet) => icon_pixels(sheet, index)?,
        None => icon_pixels(builtin()?, index)?,
    };
    Some((
        enlarge(&pixels, ICON_SIZE, ICON_SIZE, factor),
        ICON_SIZE * factor,
    ))
}

/// Icon `index` as an image GPUI can draw, made `factor` ([`pixel_factor`]) times as big, or
/// `None` when there is no such icon.
pub(crate) fn icon(index: usize, factor: u32) -> Option<Arc<Image>> {
    let factor = factor.max(1);
    let mut cache = cache().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(image) = cache.get(&(index, factor)) {
        return Some(image.clone());
    }
    let (pixels, size) = scaled_pixels(index, factor)?;
    let png = encode_png(size, size, &pixels);
    let image = Arc::new(Image::from_bytes(ImageFormat::Png, png));
    cache.insert((index, factor), image.clone());
    Some(image)
}

#[cfg(test)]
mod tests;
