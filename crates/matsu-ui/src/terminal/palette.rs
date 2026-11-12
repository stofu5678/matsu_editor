//! The colors of the terminal: the 256-color palette and what a cell's colors turn into.
//!
//! The 16 ANSI colors follow the background: a light set (VS Code's "Light+" terminal colors) when the
//! default background is light, a dark one (its "Dark+") otherwise, so that yellow and white stay readable
//! on either. The 6x6x6 cube and the 24 grays are xterm's (`256colres.pl`). Programs that ask for a color
//! (OSC 4) are told xterm's palette by `matsu-terminal`, not these; the difference is small and only
//! matters to a program that matches its own colors to the terminal's.

use matsu_terminal::{Color, Rgb};

/// `0xRRGGBB`, the form GPUI's `rgb()` takes.
pub(super) type Packed = u32;

const LIGHT: [Packed; 16] = [
    0x000000, 0xcd3131, 0x00bc00, 0x949800, 0x0451a5, 0xbc05bc, 0x0598bc, 0x555555, 0x666666,
    0xcd3131, 0x14ce14, 0xb5ba00, 0x0451a5, 0xbc05bc, 0x0598bc, 0xa5a5a5,
];

const DARK: [Packed; 16] = [
    0x000000, 0xcd3131, 0x0dbc79, 0xe5e510, 0x2472c8, 0xbc3fbc, 0x11a8cd, 0xe5e5e5, 0x666666,
    0xf14c4c, 0x23d18b, 0xf5f543, 0x3b8eea, 0xd670d6, 0x29b8db, 0xe5e5e5,
];

pub(super) fn pack(rgb: Rgb) -> Packed {
    (u32::from(rgb.r) << 16) | (u32::from(rgb.g) << 8) | u32::from(rgb.b)
}

/// How bright a packed color is, 0 to 255 (the weights of Rec. 601).
fn brightness(color: Packed) -> u32 {
    let (r, g, b) = ((color >> 16) & 0xff, (color >> 8) & 0xff, color & 0xff);
    (r * 299 + g * 587 + b * 114) / 1000
}

/// The colors a terminal draws with.
#[derive(Clone, Debug)]
pub(super) struct Palette {
    pub foreground: Packed,
    pub background: Packed,
    ansi: [Packed; 16],
    /// Colors a program set (OSC 4, 10, 11): `(index, color)`, with 256 the foreground and 257 the
    /// background (`matsu_terminal::Snapshot::color_overrides`).
    overrides: Vec<(usize, Packed)>,
}

impl Palette {
    pub(super) fn new(foreground: Packed, background: Packed) -> Self {
        let ansi = if brightness(background) > 127 {
            LIGHT
        } else {
            DARK
        };
        Self {
            foreground,
            background,
            ansi,
            overrides: Vec::new(),
        }
    }

    /// The colors a program set are used until the next call.
    pub(super) fn set_overrides(&mut self, overrides: &[(usize, Rgb)]) {
        self.overrides.clear();
        self.overrides
            .extend(overrides.iter().map(|&(index, rgb)| (index, pack(rgb))));
    }

    fn overridden(&self, index: usize) -> Option<Packed> {
        self.overrides
            .iter()
            .find(|(i, _)| *i == index)
            .map(|&(_, color)| color)
    }

    /// An entry of the 256-color palette.
    pub(super) fn indexed(&self, index: u8) -> Packed {
        if let Some(color) = self.overridden(usize::from(index)) {
            return color;
        }
        match index {
            0..=15 => self.ansi[usize::from(index)],
            16..=231 => {
                let n = u32::from(index - 16);
                let level = |x: u32| if x == 0 { 0 } else { 55 + 40 * x };
                (level(n / 36) << 16) | (level(n / 6 % 6) << 8) | level(n % 6)
            }
            232..=255 => {
                let gray = 8 + 10 * u32::from(index - 232);
                (gray << 16) | (gray << 8) | gray
            }
        }
    }

    /// The color of the default foreground, or the one a program set.
    pub(super) fn default_foreground(&self) -> Packed {
        self.overridden(256).unwrap_or(self.foreground)
    }

    pub(super) fn default_background(&self) -> Packed {
        self.overridden(257).unwrap_or(self.background)
    }

    pub(super) fn foreground_of(&self, color: Color) -> Packed {
        match color {
            Color::Default => self.default_foreground(),
            Color::Indexed(index) => self.indexed(index),
            Color::Rgb(rgb) => pack(rgb),
        }
    }

    pub(super) fn background_of(&self, color: Color) -> Packed {
        match color {
            Color::Default => self.default_background(),
            Color::Indexed(index) => self.indexed(index),
            Color::Rgb(rgb) => pack(rgb),
        }
    }
}

/// `color` moved towards `towards` by `part` of 256 (dim text is the foreground half way to the background).
pub(super) fn blend(color: Packed, towards: Packed, part: u32) -> Packed {
    let mix = |shift: u32| {
        let (a, b) = ((color >> shift) & 0xff, (towards >> shift) & 0xff);
        ((a * (256 - part) + b * part) / 256) << shift
    };
    mix(16) | mix(8) | mix(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cube_and_the_grays_are_xterms() {
        let palette = Palette::new(0x000000, 0xffffff);
        assert_eq!(palette.indexed(16), 0x000000);
        assert_eq!(palette.indexed(21), 0x0000ff);
        assert_eq!(palette.indexed(196), 0xff0000);
        assert_eq!(palette.indexed(231), 0xffffff);
        assert_eq!(palette.indexed(232), 0x080808);
        assert_eq!(palette.indexed(255), 0xeeeeee);
        // 95 = 55 + 40: the first lit step of the cube
        assert_eq!(palette.indexed(17), 0x00005f);
    }

    #[test]
    fn the_ansi_colors_follow_the_background() {
        assert_eq!(Palette::new(0, 0xffffff).indexed(7), LIGHT[7]);
        assert_eq!(Palette::new(0xffffff, 0x000000).indexed(7), DARK[7]);
        // white text on a light page would vanish; the light set has none that bright
        assert!(brightness(LIGHT[7]) < 128);
    }

    #[test]
    fn the_colors_a_program_set_win() {
        let mut palette = Palette::new(0x111111, 0xeeeeee);
        palette.set_overrides(&[
            (1, Rgb { r: 1, g: 2, b: 3 }),
            (257, Rgb { r: 9, g: 9, b: 9 }),
        ]);
        assert_eq!(palette.indexed(1), 0x010203);
        assert_eq!(palette.default_background(), 0x090909);
        assert_eq!(palette.default_foreground(), 0x111111);
        assert_eq!(
            palette.background_of(Color::Rgb(Rgb { r: 255, g: 0, b: 0 })),
            0xff0000
        );
    }

    #[test]
    fn dim_text_is_half_way() {
        assert_eq!(blend(0x000000, 0xffffff, 128), 0x7f7f7f);
        assert_eq!(blend(0x123456, 0xffffff, 0), 0x123456);
    }
}
