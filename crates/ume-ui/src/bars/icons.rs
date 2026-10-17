//! The icons of the toolbar: `mytool.bmp` of Sakura (`crates/ume-ui/assets/sakura/NOTICE`), cut
//! into pictures when they are first drawn.
//!
//! Sakura: `CImageListMgr` (`sakura_core/uiparts/CImageListMgr.cpp`) keeps the bitmap as 32
//! icons of 16x16 pixels in a row, 15 rows (`MAX_TOOLBAR_ICON_X`, `MAX_TOOLBAR_ICON_Y`,
//! `sakura_core/env/CommonSetting.h`); icon N is at column `N % 32`, row `N / 32`. The colour of
//! the top left pixel is the transparent colour (`ResizeToolIcons`). GPUI has no way to draw part
//! of a bitmap with a transparent colour, so each icon becomes a small PNG with an alpha channel
//! (written here, without compression, so that no crate is needed), which GPUI decodes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use gpui::{Image, ImageFormat};

/// Sakura's `mytool.bmp`, unchanged.
static MYTOOL_BMP: &[u8] = include_bytes!("../../assets/sakura/mytool.bmp");

/// `cx` and `cy` of an icon.
pub(super) const ICON_SIZE: usize = 16;
/// `MAX_TOOLBAR_ICON_X`: icons in a row.
const COLUMNS: usize = 32;
/// `MAX_TOOLBAR_ICON_Y`: rows of icons.
const ROWS: usize = 15;
/// `MAX_TOOLBAR_ICON_COUNT`.
pub(super) const ICON_COUNT: usize = COLUMNS * ROWS;

/// The bitmap as RGBA pixels, row by row from the top.
struct Sheet {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

fn le_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let b = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Reads a bottom-up Windows bitmap of 4 bits per pixel without compression (the format of
/// `mytool.bmp`). The pixel at the top left is the transparent colour.
fn decode_4bpp_bmp(bytes: &[u8]) -> Option<Sheet> {
    if bytes.get(..2)? != b"BM" {
        return None;
    }
    let data_at = le_u32(bytes, 10)? as usize;
    let header_size = le_u32(bytes, 14)? as usize;
    let width = le_u32(bytes, 18)? as usize;
    let height = le_u32(bytes, 22)? as i32;
    let bits = u16::from_le_bytes([*bytes.get(28)?, *bytes.get(29)?]);
    let compression = le_u32(bytes, 30)?;
    if header_size < 40 || bits != 4 || compression != 0 || height <= 0 {
        return None;
    }
    let height = height as usize;
    let palette_at = 14 + header_size;
    let palette: Vec<[u8; 3]> = (0..16)
        .map(|i| {
            let p = bytes.get(palette_at + i * 4..palette_at + i * 4 + 3)?;
            Some([p[2], p[1], p[0]])
        })
        .collect::<Option<_>>()?;
    // Each row is padded to a multiple of 4 bytes.
    let stride = (width * 4).div_ceil(32) * 4;
    let mut rgba = vec![0u8; width * height * 4];
    for y in 0..height {
        let row = bytes.get(data_at + (height - 1 - y) * stride..)?;
        for x in 0..width {
            let byte = *row.get(x / 2)?;
            let index = if x % 2 == 0 { byte >> 4 } else { byte & 15 };
            let [r, g, b] = palette[usize::from(index)];
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

fn sheet() -> Option<&'static Sheet> {
    static SHEET: OnceLock<Option<Sheet>> = OnceLock::new();
    SHEET.get_or_init(|| decode_4bpp_bmp(MYTOOL_BMP)).as_ref()
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

/// The RGBA pixels of icon `index` (16x16), or `None` when there is no such icon.
fn icon_pixels(index: usize) -> Option<Vec<u8>> {
    if index >= ICON_COUNT {
        return None;
    }
    let sheet = sheet()?;
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

/// Icon `index` of `mytool.bmp` as an image GPUI can draw, or `None` when there is no such icon.
pub(super) fn icon(index: usize) -> Option<Arc<Image>> {
    static CACHE: OnceLock<Mutex<HashMap<usize, Arc<Image>>>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(image) = cache.get(&index) {
        return Some(image.clone());
    }
    let pixels = icon_pixels(index)?;
    let png = encode_png(ICON_SIZE, ICON_SIZE, &pixels);
    let image = Arc::new(Image::from_bytes(ImageFormat::Png, png));
    cache.insert(index, image.clone());
    Some(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bitmap_is_read_and_its_top_left_colour_is_transparent() {
        let sheet = sheet().expect("mytool.bmp decodes");
        assert_eq!((sheet.width, sheet.height), (528, 240));
        // 32 icons of 16 pixels fit in the width, 15 rows in the height.
        assert!(sheet.width >= COLUMNS * ICON_SIZE && sheet.height >= ROWS * ICON_SIZE);
        assert_eq!(sheet.rgba[3], 0);
    }

    #[test]
    fn an_icon_has_pixels_and_the_ones_past_the_end_do_not_exist() {
        // The first icon (new file) draws something and keeps some pixels transparent.
        let pixels = icon_pixels(0).unwrap();
        let alphas: Vec<u8> = pixels.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
        assert!(alphas.contains(&255) && alphas.contains(&0));
        assert_eq!(pixels.len(), 16 * 16 * 4);
        assert!(icon_pixels(ICON_COUNT).is_none());
        assert!(icon(ICON_COUNT - 1).is_some());
    }

    #[test]
    fn the_png_has_the_right_checksums() {
        // CRC-32 and Adler-32 of the standard test string.
        assert_eq!(crc32(*b"123456789"), 0xCBF4_3926);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        let png = encode_png(2, 1, &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        // IHDR: length 13, then the size.
        assert_eq!(&png[8..16], &[0, 0, 0, 13, b'I', b'H', b'D', b'R']);
        assert_eq!(&png[16..24], &[0, 0, 0, 2, 0, 0, 0, 1]);
        assert!(png.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82]));
    }
}
