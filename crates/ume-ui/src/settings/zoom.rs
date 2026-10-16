// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/util/zoom.cpp (GetZoomedValue and its helpers)
//   sakura_core/util/zoom.h (ZoomSetting)
//
// Original copyright notice of those files:
//   Copyright (C) 2021-2022, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! The zoom steps of the font size (`F_SETFONTSIZEUP` / `F_SETFONTSIZEDOWN`).
//!
//! `zoomed_value` is `GetZoomedValue` of `sakura_core/util/zoom.cpp`. The table and the limits of
//! the font size are the ones `CViewCommander::Command_SETFONTSIZE` passes to it
//! (`sakura_core/cmd/CViewCommander_Settings.cpp`): they are numbers, not code, and are written
//! again here.

/// `ZoomSetting`: the zoom factors (ascending) and the limits and unit of the zoomed value.
#[derive(Debug, Clone, Copy)]
pub struct ZoomSetting {
    pub factors: &'static [f64],
    pub value_min: f64,
    pub value_max: f64,
    /// The smallest step of the value; `0.0` means no rounding.
    pub value_unit: f64,
}

/// The zoom factors of the font size (`aZoomFactors`).
const FONT_ZOOM_FACTORS: [f64; 72] = [
    0.01, 0.011, 0.0125, 0.015, 0.0175, 0.02, 0.0225, 0.025, 0.0275, 0.03, 0.035, 0.04, 0.045,
    0.05, 0.06, 0.07, 0.08, 0.09, 0.1, 0.11, 0.125, 0.15, 0.175, 0.2, 0.225, 0.25, 0.275, 0.3,
    0.35, 0.4, 0.45, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 2.75, 3.0,
    3.5, 4.0, 4.5, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.5, 15.0, 17.5, 20.0, 22.5, 25.0, 27.5,
    30.0, 35.0, 40.0, 45.0, 50.0, 60.0, 70.0, 80.0, 90.0,
];

/// The font size in 1/10 point: from 1 point to 72 points, in steps of 0.5 point.
pub const FONT_ZOOM: ZoomSetting = ZoomSetting {
    factors: &FONT_ZOOM_FACTORS,
    value_min: 10.0,
    value_max: 720.0,
    value_unit: 5.0,
};

/// The position of `value` in the table: the index when it is in it, `index - 0.5` between two
/// entries (and before the first), `len - 0.5` after the last (`GetPositionInTable`).
fn position_in_table(table: &[f64], value: f64) -> f64 {
    for (i, &t) in table.iter().enumerate() {
        if value <= t {
            return if value == t { i as f64 } else { i as f64 - 0.5 };
        }
    }
    table.len() as f64 - 0.5
}

fn quantized(value: f64, unit: f64) -> f64 {
    if unit > 0.0 {
        (value / unit).floor() * unit
    } else {
        value
    }
}

fn zoom_up_index(z: &ZoomSetting, base: f64, current: f64, current_index: i32, steps: i32) -> i32 {
    let mut index = current_index + steps;
    let max = z.factors.len() as i32 - 1;
    while index < max {
        let next = quantized(base * z.factors[index as usize], z.value_unit);
        if next != current {
            break;
        }
        index += 1;
    }
    index.min(max)
}

fn zoom_down_index(
    z: &ZoomSetting,
    base: f64,
    current: f64,
    current_index: i32,
    steps: i32,
) -> i32 {
    let mut index = current_index - steps;
    let mut last = current;
    let mut finding_one_more_change = false;
    while index >= 0 {
        let value = quantized(base * z.factors[index as usize], z.value_unit);
        if finding_one_more_change && value != last {
            break;
        } else if value != current {
            // Go on until the value changes once more.
            finding_one_more_change = true;
            last = value;
        }
        index -= 1;
    }
    if finding_one_more_change {
        index += 1;
    }
    index.max(0)
}

/// `GetZoomedValue`: the value `steps` zoom steps up (positive) or down (negative) from the zoom
/// `current_zoom` of `base`, and the new zoom. `None` when the value would not change.
pub fn zoomed_value(
    z: &ZoomSetting,
    base: f64,
    current_zoom: f64,
    steps: i32,
) -> Option<(f64, f64)> {
    if steps == 0 {
        return None;
    }
    let up = steps > 0;
    let index_max = z.factors.len() as i32 - 1;

    let position = position_in_table(z.factors, current_zoom);
    let current_index = if up {
        position.floor()
    } else {
        position.ceil()
    } as i32;
    if (!up && current_index <= 0) || (up && index_max <= current_index) {
        // Already outside the table and asked to go further out: stay.
        return None;
    }

    let current_value = quantized(base * current_zoom, z.value_unit);
    let value_min = z.value_min.min(base).min(current_value);
    let value_max = z.value_max.max(base).max(current_value);

    let next_index = if up {
        zoom_up_index(z, base, current_value, current_index, steps)
    } else {
        zoom_down_index(z, base, current_value, current_index, -steps)
    };
    let mut next_zoom = z.factors[next_index as usize];
    let mut next_value = quantized(base * next_zoom, z.value_unit);

    if next_value < value_min || value_max < next_value {
        // Beyond the limits: clamp, and work the zoom out again from the clamped size.
        next_value = next_value.clamp(value_min, value_max);
        if base != 0.0 {
            next_zoom = next_value / base;
        }
    }
    if current_value == next_value {
        return None;
    }
    Some((next_value, next_zoom))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 10 pt (100 in 1/10 pt) at zoom 1.0: the next factor above 1.0 is 1.1, so 11 pt.
    #[test]
    fn zooming_up_from_ten_points_goes_to_eleven() {
        let (value, zoom) = zoomed_value(&FONT_ZOOM, 100.0, 1.0, 1).unwrap();
        assert_eq!(value, 110.0);
        assert_eq!(zoom, 1.1);
        // Zoom 1.1 -> 1.25 = 12.5 pt.
        let (value, zoom) = zoomed_value(&FONT_ZOOM, 100.0, zoom, 1).unwrap();
        assert_eq!(value, 125.0);
        assert_eq!(zoom, 1.25);
    }

    #[test]
    fn zooming_down_from_ten_points_goes_to_nine() {
        let (value, zoom) = zoomed_value(&FONT_ZOOM, 100.0, 1.0, -1).unwrap();
        assert_eq!(value, 90.0);
        assert_eq!(zoom, 0.9);
    }

    #[test]
    fn the_size_stops_at_the_limits() {
        // Down from 1 pt: the minimum is 1 pt, so nothing changes.
        assert!(zoomed_value(&FONT_ZOOM, 10.0, 1.0, -1).is_none());
        // Up to the maximum, 72 pt, and then it stays.
        let mut zoom = 1.0;
        let mut value = 100.0;
        while let Some((v, z)) = zoomed_value(&FONT_ZOOM, 100.0, zoom, 1) {
            assert!(v > value, "{v} is not above {value}");
            (value, zoom) = (v, z);
        }
        assert_eq!(value, 720.0);
    }

    #[test]
    fn a_tiny_font_steps_by_the_unit_not_by_the_factor() {
        // 1.5 pt = 15: the factors move the value by less than the 0.5 pt unit, so the search
        // goes on until the value changes.
        let (value, _) = zoomed_value(&FONT_ZOOM, 15.0, 1.0, 1).unwrap();
        assert!(value > 15.0 && value % 5.0 == 0.0);
    }

    #[test]
    fn zero_steps_do_nothing() {
        assert!(zoomed_value(&FONT_ZOOM, 100.0, 1.0, 0).is_none());
    }
}
