//! The paper and the direction of the paper in a `DEVMODE`, which the Windows print dialogs take
//! and give back. Only the rules for the values, so it needs no OS and is tested on every one.
//!
//! Sakura puts the print setting's paper size and orientation into the `DEVMODE` before it shows
//! a printer dialog (`CPrint::PrintDlg`, `CPrint.cpp` 117-123) and, when the printer button of the
//! preview bar was OK'd, takes them back into the print setting
//! (`CPrintPreview::DispatchEvent_PPB`, `CPrintPreview.cpp` 2101-2103).

use matsu_print::{Orientation, PrintSetting, find_paper_info};

/// `DM_ORIENTATION`: `dmOrientation` of a `DEVMODE` has a value (`wingdi.h`).
pub const DM_ORIENTATION: u32 = 0x1;
/// `DM_PAPERSIZE`: `dmPaperSize` of a `DEVMODE` has a value.
pub const DM_PAPERSIZE: u32 = 0x2;

/// `DMORIENT_PORTRAIT` and `DMORIENT_LANDSCAPE`.
const DMORIENT_PORTRAIT: i16 = 1;
const DMORIENT_LANDSCAPE: i16 = 2;

/// The value of `dmOrientation` for `orientation`.
pub fn orientation_value(orientation: Orientation) -> i16 {
    match orientation {
        Orientation::Portrait => DMORIENT_PORTRAIT,
        Orientation::Landscape => DMORIENT_LANDSCAPE,
    }
}

/// What the dialog changed in the print setting: the printer, the paper and its direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrinterChoice {
    /// The printer the dialog gave, or `None` when it named none (the setting keeps its own).
    pub printer: Option<matsu_print::Printer>,
    /// The paper and its direction, as the setting has them after the dialog.
    pub paper_size: i16,
    pub orientation: Orientation,
}

/// The paper size and the direction the setting has after the dialog gave back a `DEVMODE` with
/// `fields` (`dmFields`), `paper_size` (`dmPaperSize`) and `orientation` (`dmOrientation`).
///
/// Sakura copies both values whatever `dmFields` says. A value whose bit is not set was not
/// filled in by the driver, and a paper that is not in Sakura's list
/// ([`matsu_print::PAPER_INFO`]) is one the layout cannot size (it falls back to A4 portrait
/// then): those keep what the setting had (type B of ADR-0018).
pub fn after_dialog(
    setting: &PrintSetting,
    fields: u32,
    paper_size: i16,
    orientation: i16,
) -> (i16, Orientation) {
    let size = if fields & DM_PAPERSIZE != 0 && find_paper_info(paper_size).is_some() {
        paper_size
    } else {
        setting.print_paper_size
    };
    let direction = if fields & DM_ORIENTATION == 0 {
        setting.print_paper_orientation
    } else if orientation == DMORIENT_LANDSCAPE {
        Orientation::Landscape
    } else {
        // Anything but landscape is portrait for the layout (`CPrint::GetPaperSize` turns the
        // paper only for `DMORIENT_LANDSCAPE`).
        Orientation::Portrait
    };
    (size, direction)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setting(size: i16, orientation: Orientation) -> PrintSetting {
        PrintSetting {
            print_paper_size: size,
            print_paper_orientation: orientation,
            ..PrintSetting::default()
        }
    }

    #[test]
    fn the_values_of_the_orientation_are_the_win32_ones() {
        assert_eq!(orientation_value(Orientation::Portrait), 1);
        assert_eq!(orientation_value(Orientation::Landscape), 2);
    }

    #[test]
    fn the_paper_and_direction_of_the_dialog_go_to_the_setting() {
        // DMPAPER_A4 is 9 and DMPAPER_B5 (JIS) is 13.
        let s = setting(9, Orientation::Portrait);
        assert_eq!(
            after_dialog(&s, DM_PAPERSIZE | DM_ORIENTATION, 13, 2),
            (13, Orientation::Landscape)
        );
        assert_eq!(
            after_dialog(&s, DM_PAPERSIZE | DM_ORIENTATION, 9, 1),
            (9, Orientation::Portrait)
        );
    }

    #[test]
    fn a_value_the_driver_did_not_fill_in_keeps_the_setting() {
        let s = setting(13, Orientation::Landscape);
        assert_eq!(after_dialog(&s, 0, 9, 1), (13, Orientation::Landscape));
        assert_eq!(
            after_dialog(&s, DM_ORIENTATION, 9, 1),
            (13, Orientation::Portrait)
        );
        assert_eq!(
            after_dialog(&s, DM_PAPERSIZE, 9, 1),
            (9, Orientation::Landscape)
        );
    }

    #[test]
    fn a_paper_that_is_not_in_the_list_keeps_the_setting() {
        // DMPAPER_USER (256) is a paper of a size the driver names itself.
        let s = setting(9, Orientation::Portrait);
        assert_eq!(
            after_dialog(&s, DM_PAPERSIZE | DM_ORIENTATION, 256, 1),
            (9, Orientation::Portrait)
        );
    }
}
