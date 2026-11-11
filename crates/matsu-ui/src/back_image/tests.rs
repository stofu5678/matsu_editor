use std::io::Cursor;

use super::*;

/// A picture of `w` x `h` pixels, each `rgba(x, y)`, in the format.
fn encode(
    w: u32,
    h: u32,
    format: image::ImageFormat,
    rgba: impl Fn(u32, u32) -> [u8; 4],
) -> Vec<u8> {
    let mut img = image::RgbaImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = image::Rgba(rgba(x, y));
    }
    let mut out = Cursor::new(Vec::new());
    match format {
        // JPEG has no alpha
        image::ImageFormat::Jpeg => image::DynamicImage::ImageRgba8(img)
            .to_rgb8()
            .write_to(&mut out, format),
        _ => img.write_to(&mut out, format),
    }
    .unwrap();
    out.into_inner()
}

fn spec(path: &Path) -> BackImageSpec {
    BackImageSpec {
        path: path.to_owned(),
        position: 0,
        offset: (0, 0),
        repeat: (false, false),
        scroll: (true, true),
        opacity: 255,
    }
}

#[test]
fn a_png_is_decoded_to_bgra() {
    let png = encode(2, 1, image::ImageFormat::Png, |x, _| {
        if x == 0 {
            [10, 20, 30, 255]
        } else {
            [1, 2, 3, 100]
        }
    });
    let d = decode(&png, 255, (false, false)).unwrap();
    assert_eq!((d.width, d.height, d.copies), (2, 1, (1, 1)));
    assert_eq!(d.bgra, [30, 20, 10, 255, 3, 2, 1, 100]);
}

#[test]
fn the_opacity_scales_the_alpha_the_way_alphablend_does() {
    let png = encode(1, 2, image::ImageFormat::Png, |_, y| {
        if y == 0 {
            [9, 9, 9, 255]
        } else {
            [9, 9, 9, 100]
        }
    });
    let d = decode(&png, 128, (false, false)).unwrap();
    // 255 * 128 / 255 = 128; 100 * 128 / 255 = 50.2
    assert_eq!([d.bgra[3], d.bgra[7]], [128, 50]);
    let d = decode(&png, 0, (false, false)).unwrap();
    assert_eq!([d.bgra[3], d.bgra[7]], [0, 0]);
}

#[test]
fn bmp_and_jpeg_are_read_too() {
    let bmp = encode(3, 2, image::ImageFormat::Bmp, |_, _| [200, 100, 50, 255]);
    let d = decode(&bmp, 255, (false, false)).unwrap();
    assert_eq!((d.width, d.height), (3, 2));
    assert_eq!(&d.bgra[..4], &[50, 100, 200, 255]);
    let jpeg = encode(16, 16, image::ImageFormat::Jpeg, |_, _| [200, 100, 50, 255]);
    let d = decode(&jpeg, 255, (false, false)).unwrap();
    assert_eq!((d.width, d.height), (16, 16));
    // the JPEG is lossy: close to the color
    assert!(d.bgra[0].abs_diff(50) < 12 && d.bgra[2].abs_diff(200) < 12);
    assert_eq!(d.bgra[3], 255);
}

#[test]
fn what_is_not_a_picture_gives_none() {
    assert!(decode(b"", 255, (false, false)).is_none());
    assert!(decode(b"this is text, not a picture", 255, (false, false)).is_none());
    let png = encode(8, 8, image::ImageFormat::Png, |_, _| [1, 2, 3, 255]);
    assert!(decode(&png[..png.len() / 2], 255, (false, false)).is_none());
    // damaged data: an error or (if the damage is in a checksum) a picture; it must not panic
    let mut broken = png;
    broken[40] ^= 0xff;
    let _ = decode(&broken, 255, (false, false));
}

#[test]
fn a_picture_bigger_than_the_limit_gives_none() {
    let wide = encode(MAX_SIDE + 1, 1, image::ImageFormat::Png, |_, _| {
        [0, 0, 0, 255]
    });
    assert!(decode(&wide, 255, (false, false)).is_none());
    let high = encode(1, MAX_SIDE + 1, image::ImageFormat::Png, |_, _| {
        [0, 0, 0, 255]
    });
    assert!(decode(&high, 255, (false, false)).is_none());
    let ok = encode(MAX_SIDE, 1, image::ImageFormat::Png, |_, _| [0, 0, 0, 255]);
    assert!(decode(&ok, 255, (false, false)).is_some());
}

#[test]
fn a_small_repeating_picture_is_made_into_copies_of_itself() {
    let png = encode(3, 2, image::ImageFormat::Png, |x, y| {
        [x as u8, y as u8, 0, 255]
    });
    let d = decode(&png, 255, (true, false)).unwrap();
    // 256 / 3 rounded up copies across, one down (it does not repeat down)
    assert_eq!(d.copies, (86, 1));
    assert_eq!((d.width, d.height), (258, 2));
    assert_eq!(d.bgra.len(), 258 * 2 * 4);
    // the second row of the first copy and of the second copy are the same
    let row = 258 * 4;
    assert_eq!(&d.bgra[row..row + 12], &d.bgra[row + 12..row + 24]);
    let d = decode(&png, 255, (true, true)).unwrap();
    assert_eq!(d.copies, (86, 128));
    // a picture that is big enough is left alone
    let big = encode(300, 300, image::ImageFormat::Png, |_, _| [0, 0, 0, 255]);
    assert_eq!(decode(&big, 255, (true, true)).unwrap().copies, (1, 1));
}

#[test]
fn the_spec_comes_from_the_type() {
    let mut t = TypeSettings::basic();
    assert_eq!(BackImageSpec::from_type(&t), None);
    t.back_img_path = "img\\a.png".to_owned();
    t.back_img_pos = 5;
    t.back_img_offset_x = -3;
    t.back_img_offset_y = 4;
    t.back_img_repeat_x = false;
    t.back_img_scroll_y = false;
    t.back_img_opacity = 77;
    let s = BackImageSpec::from_type(&t).unwrap();
    let expected = if cfg!(windows) {
        "img\\a.png"
    } else {
        "img/a.png"
    };
    assert_eq!(s.path, PathBuf::from(expected));
    assert_eq!(s.position, 5);
    assert_eq!(s.offset, (-3, 4));
    assert_eq!(s.repeat, (false, true));
    assert_eq!(s.scroll, (true, false));
    assert_eq!(s.opacity, 77);
    assert!(s.is_visible());
    t.back_img_opacity = 0;
    assert!(!BackImageSpec::from_type(&t).unwrap().is_visible());
}

#[test]
fn a_relative_path_is_looked_for_in_the_ini_folder_then_the_program_folder() {
    let ini = tempfile::tempdir().unwrap();
    let exe = tempfile::tempdir().unwrap();
    let relative = spec(Path::new("pic.png"));
    // in neither: under the ini folder
    let r = relative
        .clone()
        .resolved(Some(ini.path()), Some(exe.path()));
    assert_eq!(r.path, ini.path().join("pic.png"));
    // only in the program folder
    std::fs::write(exe.path().join("pic.png"), b"x").unwrap();
    let r = relative
        .clone()
        .resolved(Some(ini.path()), Some(exe.path()));
    assert_eq!(r.path, exe.path().join("pic.png"));
    // in both: the ini folder wins
    std::fs::write(ini.path().join("pic.png"), b"x").unwrap();
    let r = relative.resolved(Some(ini.path()), Some(exe.path()));
    assert_eq!(r.path, ini.path().join("pic.png"));
    // an absolute path stays
    let absolute = ini.path().join("abs.png");
    let r = spec(&absolute).resolved(Some(exe.path()), Some(exe.path()));
    assert_eq!(r.path, absolute);
}

#[test]
fn a_file_that_cannot_be_read_gives_no_image() {
    let dir = tempfile::tempdir().unwrap();
    // missing, a folder, an empty file, text, a cut picture
    assert!(load(&spec(&dir.path().join("none.png"))).is_none());
    assert!(load(&spec(dir.path())).is_none());
    let empty = dir.path().join("empty.png");
    std::fs::write(&empty, b"").unwrap();
    assert!(load(&spec(&empty)).is_none());
    let text = dir.path().join("text.png");
    std::fs::write(&text, "テキスト").unwrap();
    assert!(load(&spec(&text)).is_none());
    let png = encode(8, 8, image::ImageFormat::Png, |_, _| [1, 2, 3, 255]);
    let cut = dir.path().join("cut.png");
    std::fs::write(&cut, &png[..png.len() / 2]).unwrap();
    assert!(load(&spec(&cut)).is_none());
}

#[test]
fn a_loaded_picture_is_shared_until_the_file_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.png");
    let first = encode(4, 4, image::ImageFormat::Png, |_, _| [1, 2, 3, 255]);
    std::fs::write(&path, &first).unwrap();
    let a = load(&spec(&path)).unwrap();
    assert_eq!(a.size, (4, 4));
    let b = load(&spec(&path)).unwrap();
    assert!(
        Arc::ptr_eq(&a, &b),
        "the second view uses the picture of the first"
    );
    // a different opacity is a different picture
    let faded = BackImageSpec {
        opacity: 100,
        ..spec(&path)
    };
    assert!(!Arc::ptr_eq(&a, &load(&faded).unwrap()));
    // the file is another one now (another size, so that the check does not depend on the
    // resolution of the time stamp)
    let second = encode(40, 30, image::ImageFormat::Png, |x, y| {
        [(x * 50) as u8, (y * 80) as u8, (x * y * 7) as u8, 255]
    });
    std::fs::write(&path, &second).unwrap();
    let c = load(&spec(&path)).unwrap();
    assert_eq!(c.size, (40, 30));
    assert!(!Arc::ptr_eq(&a, &c));
}

#[test]
fn loading_does_not_write_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.png");
    let png = encode(4, 4, image::ImageFormat::Png, |_, _| [1, 2, 3, 255]);
    std::fs::write(&path, &png).unwrap();
    let before = std::fs::metadata(&path).unwrap().modified().unwrap();
    // a read-only file loads too
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&path, permissions).unwrap();
    assert!(load(&spec(&path)).is_some());
    assert_eq!(std::fs::read(&path).unwrap(), png);
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        before
    );
}
