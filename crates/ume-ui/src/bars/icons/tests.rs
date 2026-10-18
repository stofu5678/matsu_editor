use super::*;

/// The tests change the process-wide bitmap: one at a time.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// A 24-bit bottom-up bitmap of `width` x `height`; `pixel(x, y)` is (r, g, b) from the top.
fn bmp24(width: usize, height: usize, pixel: impl Fn(usize, usize) -> [u8; 3]) -> Vec<u8> {
    let stride = (width * 24).div_ceil(32) * 4;
    let mut out = b"BM".to_vec();
    out.extend_from_slice(&((54 + stride * height) as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&(height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for y in (0..height).rev() {
        let mut row = Vec::new();
        for x in 0..width {
            let [r, g, b] = pixel(x, y);
            row.extend_from_slice(&[b, g, r]);
        }
        row.resize(stride, 0);
        out.extend_from_slice(&row);
    }
    out
}

#[test]
fn the_bitmap_is_read_and_its_top_left_colour_is_transparent() {
    let sheet = builtin().expect("mytool.bmp decodes");
    assert_eq!((sheet.width, sheet.height), (528, 240));
    assert!(sheet.width >= COLUMNS * ICON_SIZE && sheet.height >= ROWS * ICON_SIZE);
    assert_eq!(sheet.rgba[3], 0);
}

#[test]
fn an_icon_has_pixels_and_the_ones_past_the_end_do_not_exist() {
    let _g = serial();
    install_from(None, None);
    let pixels = icon_pixels(builtin().unwrap(), 0).unwrap();
    let alphas: Vec<u8> = pixels.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
    assert!(alphas.contains(&255) && alphas.contains(&0));
    assert_eq!(pixels.len(), 16 * 16 * 4);
    assert!(icon_pixels(builtin().unwrap(), ICON_COUNT).is_none());
    assert!(icon(ICON_COUNT - 1, 1).is_some());
}

#[test]
fn the_png_has_the_right_checksums() {
    assert_eq!(crc32(*b"123456789"), 0xCBF4_3926);
    assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    let png = encode_png(2, 1, &[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(&png[8..16], &[0, 0, 0, 13, b'I', b'H', b'D', b'R']);
    assert_eq!(&png[16..24], &[0, 0, 0, 2, 0, 0, 0, 1]);
    assert!(png.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82]));
}

#[test]
fn the_factor_keeps_the_dots_whole() {
    for (scale, factor) in [
        (0.5, 1),
        (1.0, 1),
        (1.25, 1),
        (1.5, 2),
        (1.75, 2),
        (2.0, 2),
        (2.5, 3),
        (3.0, 3),
        (f32::NAN, 1),
    ] {
        assert_eq!(pixel_factor(scale), factor, "scale {scale}");
    }
    // One picture dot is one screen dot: 16 * factor screen dots.
    for scale in [1.0f32, 1.25, 1.5, 2.0] {
        let screen = logical_size(scale) * scale;
        assert!((screen - (16 * pixel_factor(scale)) as f32).abs() < 1e-3);
    }
}

#[test]
fn enlarging_repeats_each_dot() {
    let rgba = [1, 2, 3, 4, 5, 6, 7, 8];
    let big = enlarge(&rgba, 2, 1, 2);
    assert_eq!(big.len(), 2 * 2 * 2 * 4);
    assert_eq!(
        &big[..16],
        &[1, 2, 3, 4, 1, 2, 3, 4, 5, 6, 7, 8, 5, 6, 7, 8]
    );
    assert_eq!(&big[16..], &big[..16]);
}

#[test]
fn my_icons_bmp_replaces_the_builtin_one_and_the_settings_folder_wins() {
    let _g = serial();
    let settings = tempfile::tempdir().unwrap();
    let program = tempfile::tempdir().unwrap();
    // 512 x 16: only the first 32 icons exist. Top left is the transparent colour (white).
    let red = bmp24(512, 16, |x, y| {
        if x == 0 && y == 0 || x > 16 {
            [255, 255, 255]
        } else {
            [255, 0, 0]
        }
    });
    let green = bmp24(512, 16, |_, _| [0, 255, 0]);
    std::fs::write(program.path().join(FILE_NAME), &green).unwrap();
    install_from(Some(settings.path()), Some(program.path()));
    let (px, _) = scaled_pixels(0, 1).unwrap();
    assert_eq!(
        &px[4..8],
        &[0, 255, 0, 0],
        "the program folder's file, all transparent"
    );
    std::fs::write(settings.path().join(FILE_NAME), &red).unwrap();
    install_from(Some(settings.path()), Some(program.path()));
    assert!(is_external());
    let (px, size) = scaled_pixels(0, 2).unwrap();
    assert_eq!(size, 32);
    // The dot at (1, 0) is red and opaque, enlarged to 2x2; (0, 0) is transparent.
    assert_eq!(&px[0..4], &[255, 255, 255, 0]);
    assert_eq!(&px[2 * 4..2 * 4 + 4], &[255, 0, 0, 255]);
    // The bitmap is one row of icons high: icon 32 is on the second row, which it lacks.
    assert!(icon(32, 1).is_none());
    assert!(icon(0, 1).is_some());
    install_from(None, None);
    assert!(!is_external());
    assert!(icon(32, 1).is_some());
}

#[test]
fn a_broken_my_icons_bmp_leaves_the_builtin_icons() {
    let _g = serial();
    let dir = tempfile::tempdir().unwrap();
    for bytes in [
        b"not a bitmap".to_vec(),
        b"BM".to_vec(),
        bmp24(512, 16, |_, _| [1, 2, 3])[..100].to_vec(),
    ] {
        std::fs::write(dir.path().join(FILE_NAME), &bytes).unwrap();
        install_from(Some(dir.path()), None);
        assert!(!is_external());
        assert!(icon(0, 1).is_some());
    }
    // A header that promises more than the file holds is refused before anything is allocated.
    let mut huge = bmp24(16, 16, |_, _| [1, 2, 3]);
    huge[18..22].copy_from_slice(&i32::MAX.to_le_bytes());
    assert!(decode_bmp(&huge).is_none());
    install_from(None, None);
}

#[test]
fn bitmaps_of_other_depths_and_top_down_ones_are_read() {
    let mut top_down = bmp24(2, 2, |x, y| [(x * 100) as u8, (y * 100) as u8, 7]);
    // Make it top-down: negative height, rows in top-to-bottom order.
    top_down[22..26].copy_from_slice(&(-2i32).to_le_bytes());
    let stride = 8;
    let (first, second) = (
        top_down[54..54 + stride].to_vec(),
        top_down[54 + stride..54 + 2 * stride].to_vec(),
    );
    top_down[54..54 + stride].copy_from_slice(&second);
    top_down[54 + stride..54 + 2 * stride].copy_from_slice(&first);
    let sheet = decode_bmp(&top_down).unwrap();
    // The pixel at the top left is (0, 0, 7): transparent; the one below it has g = 100.
    assert_eq!(sheet.rgba[3], 0);
    assert_eq!(&sheet.rgba[8..11], &[0, 100, 7]);
    // 8 bits per pixel with a palette of two colours.
    let mut eight = b"BM".to_vec();
    eight.extend_from_slice(&[0; 8]);
    eight.extend_from_slice(&(14u32 + 40 + 256 * 4).to_le_bytes());
    eight.extend_from_slice(&40u32.to_le_bytes());
    eight.extend_from_slice(&2i32.to_le_bytes());
    eight.extend_from_slice(&1i32.to_le_bytes());
    eight.extend_from_slice(&1u16.to_le_bytes());
    eight.extend_from_slice(&8u16.to_le_bytes());
    // compression, image size, two resolutions, colours used and important (all 0).
    eight.extend_from_slice(&[0; 24]);
    eight.extend_from_slice(&[0, 0, 255, 0, 0, 255, 0, 0]);
    eight.resize(14 + 40 + 256 * 4, 0);
    eight.extend_from_slice(&[0, 1, 0, 0]);
    let sheet = decode_bmp(&eight).unwrap();
    assert_eq!(&sheet.rgba[4..8], &[0, 255, 0, 255]);
}
