//! The background image of a document type (`bgImg*` of `[Types(N)]`; 背景画像 of the type
//! settings, 「ウィンドウ」).
//!
//! * [`BackImageSpec`]: what the type says (the file, the position, the offset, the repeat, the
//!   scroll and the opacity), kept in the view settings;
//! * [`load`]: reads and decodes the file once, when the type of a view is chosen or its settings
//!   change (`CEditDoc::SetBackgroundImage`), not when the view is painted. The picture is kept
//!   for as long as a view uses it, so views of the same type share one;
//! * `placement`: the tiles of the picture in the view (`CEditView::DrawBackImage`).
//!
//! The file is only read. A file that cannot be read, or is not a picture the decoder knows,
//! gives no image and no message, as in Sakura (`SetBackgroundImage` returns).

pub(crate) mod placement;

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::SystemTime;

use gpui::RenderImage;
use matsu_config::settings::TypeSettings;

/// What a document type says about the background image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackImageSpec {
    /// The file. A relative path stays as typed until [`Self::resolved`].
    pub path: PathBuf,
    /// `bgImgPos` (`EBackgroundImagePos`).
    pub position: i32,
    /// `bgImgPosOffsetX` and `Y`, in pixels.
    pub offset: (i32, i32),
    /// `bgImgRepeartX` and `Y`.
    pub repeat: (bool, bool),
    /// `bgImgScrollX` and `Y`: the image moves with the text.
    pub scroll: (bool, bool),
    /// `bgImgOpacity`: 255 shows the image as it is; 0 shows nothing (`IsBkBitmap`).
    pub opacity: u8,
}

impl BackImageSpec {
    /// The image of the type, if it names a file.
    pub fn from_type(t: &TypeSettings) -> Option<Self> {
        if t.back_img_path.is_empty() {
            return None;
        }
        // The settings are written with Windows paths; a backslash separates folders on every OS.
        let typed = if cfg!(windows) {
            t.back_img_path.clone()
        } else {
            t.back_img_path.replace('\\', "/")
        };
        Some(Self {
            path: PathBuf::from(typed),
            position: t.back_img_pos,
            offset: (t.back_img_offset_x, t.back_img_offset_y),
            repeat: (t.back_img_repeat_x, t.back_img_repeat_y),
            scroll: (t.back_img_scroll_x, t.back_img_scroll_y),
            opacity: t.back_img_opacity,
        })
    }

    /// The same with a relative path looked for under the folder of the settings file, then under
    /// the folder of the program (`GetInidirOrExedir`, `CEditDoc::SetBackgroundImage`).
    pub fn resolved(mut self, ini_dir: Option<&Path>, exe_dir: Option<&Path>) -> Self {
        let path = self.path.to_string_lossy().into_owned();
        self.path = crate::support::help::inidir_or_exedir(&path, ini_dir, exe_dir);
        self
    }

    /// Whether an image that is loaded is drawn: `CEditView::IsBkBitmap` is false for an image
    /// that is fully transparent.
    pub fn is_visible(&self) -> bool {
        self.opacity != 0
    }
}

/// The widest and highest picture that is shown. The graphics API holds a picture in one texture,
/// and 8192 is what every one of them has; a bigger file gives no image.
pub const MAX_SIDE: u32 = 8192;

/// A file bigger than this is not read (it would be read whole into memory before the decoder
/// can look at the size of the picture).
const MAX_FILE_LEN: u64 = 256 * 1024 * 1024;

/// A repeating picture smaller than this (in either direction) is made into copies of itself, so
/// that painting it takes a few tiles and not thousands.
const LEAST_TILE: u32 = 256;

/// The pixels of a picture the way the renderer takes them.
#[derive(Debug, PartialEq, Eq)]
pub struct Decoded {
    pub width: u32,
    pub height: u32,
    /// Blue, green, red and alpha of each pixel, not premultiplied; the alpha has the opacity of
    /// the type in it (the renderer blends `alpha` of the source over the window, which is what
    /// `AlphaBlend` does with `SourceConstantAlpha`).
    pub bgra: Vec<u8>,
    /// How many copies of the file's picture there are across and down (see `LEAST_TILE`).
    pub copies: (u32, u32),
}

/// Decodes `bytes` as a picture of any format the decoder knows by looking at its first bytes
/// (BMP, PNG, JPEG, GIF, ...), the way WIC does. `None` when it is not one, is too big, or is
/// damaged.
pub fn decode(bytes: &[u8], opacity: u8, repeat: (bool, bool)) -> Option<Decoded> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    reader.limits(limits);
    // The first frame, as `GetFrame(0)`.
    let mut rgba = reader.decode().ok()?.into_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        return None;
    }
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel[3] = scale_alpha(pixel[3], opacity);
        pixel.swap(0, 2);
    }
    let mut bgra = rgba.into_raw();
    let copies_of = |size: u32, repeats: bool| {
        if repeats && size < LEAST_TILE {
            LEAST_TILE.div_ceil(size)
        } else {
            1
        }
    };
    let copies = (copies_of(width, repeat.0), copies_of(height, repeat.1));
    let (mut width, mut height) = (width, height);
    if copies != (1, 1) {
        (bgra, width, height) = tile(&bgra, width, height, copies);
    }
    Some(Decoded {
        width,
        height,
        bgra,
        copies,
    })
}

/// `alpha * opacity / 255`, rounded.
fn scale_alpha(alpha: u8, opacity: u8) -> u8 {
    ((u32::from(alpha) * u32::from(opacity) + 127) / 255) as u8
}

/// The picture `copies.0` times across and `copies.1` times down.
fn tile(pixels: &[u8], width: u32, height: u32, copies: (u32, u32)) -> (Vec<u8>, u32, u32) {
    let row = width as usize * 4;
    let (new_width, new_height) = (width * copies.0, height * copies.1);
    let mut out = Vec::with_capacity(new_width as usize * new_height as usize * 4);
    for _ in 0..copies.1 {
        for line in pixels.chunks_exact(row) {
            for _ in 0..copies.0 {
                out.extend_from_slice(line);
            }
        }
    }
    (out, new_width, new_height)
}

/// A picture ready to be painted.
pub struct BackBitmap {
    /// The size in pixels of the file's picture.
    pub size: (u32, u32),
    /// How many copies of it `render` holds across and down.
    pub copies: (u32, u32),
    pub render: Arc<RenderImage>,
}

impl BackBitmap {
    /// `None` when the pixels are not the size the picture says.
    pub fn from_decoded(decoded: Decoded) -> Option<Self> {
        let buffer = image::RgbaImage::from_raw(decoded.width, decoded.height, decoded.bgra)?;
        let render = RenderImage::new(vec![image::Frame::new(buffer)]);
        Some(Self {
            size: (
                decoded.width / decoded.copies.0,
                decoded.height / decoded.copies.1,
            ),
            copies: decoded.copies,
            render: Arc::new(render),
        })
    }

    /// The bytes the picture takes in memory (before the graphics API has a copy).
    pub fn bytes(&self) -> usize {
        self.size.0 as usize
            * self.copies.0 as usize
            * self.size.1 as usize
            * self.copies.1 as usize
            * 4
    }
}

/// What makes two loads the same: the same file as it was last changed, read the same way.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Key {
    path: PathBuf,
    modified: Option<SystemTime>,
    len: u64,
    opacity: u8,
    repeat: (bool, bool),
}

static LOADED: Mutex<Vec<(Key, Weak<BackBitmap>)>> = Mutex::new(Vec::new());

/// Reads the picture of `spec`. `None` when the file cannot be read or is not a picture.
///
/// A picture that a view still holds is not read again for another view of the same file; a file
/// that changed on disk is read again (the next time the settings are applied).
pub fn load(spec: &BackImageSpec) -> Option<Arc<BackBitmap>> {
    let meta = std::fs::metadata(&spec.path).ok()?;
    if !meta.is_file() || meta.len() > MAX_FILE_LEN {
        return None;
    }
    let key = Key {
        path: spec.path.clone(),
        modified: meta.modified().ok(),
        len: meta.len(),
        opacity: spec.opacity,
        repeat: spec.repeat,
    };
    if let Some(found) = lookup(&key) {
        return Some(found);
    }
    // The file is read once, whole (a picture is not read in ranges), and let go of when decoded.
    let decoded = {
        let bytes = std::fs::read(&spec.path).ok()?;
        decode(&bytes, spec.opacity, spec.repeat)?
    };
    let bitmap = Arc::new(BackBitmap::from_decoded(decoded)?);
    let mut loaded = LOADED.lock().unwrap_or_else(|e| e.into_inner());
    loaded.retain(|(_, weak)| weak.strong_count() > 0);
    loaded.push((key, Arc::downgrade(&bitmap)));
    Some(bitmap)
}

fn lookup(key: &Key) -> Option<Arc<BackBitmap>> {
    let loaded = LOADED.lock().unwrap_or_else(|e| e.into_inner());
    loaded
        .iter()
        .filter(|(k, _)| k == key)
        .find_map(|(_, weak)| weak.upgrade())
}

#[cfg(test)]
mod tests;
