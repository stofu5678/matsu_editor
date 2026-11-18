//! The eight print settings the application holds, and which one each document type prints with.
//!
//! Sakura keeps them in the shared data (`m_PrintSettingArr`) and the number in the settings of the
//! document type (`m_nCurrentPrintSetting`). They are saved to `[Print]` of the settings file by
//! OK of the dialog, under the lock every writer of the file takes
//! (`matsu_config::history::update_profile`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui::{App, Global};
use matsu_config::Profile;
use matsu_config::history::update_profile;
use matsu_config::settings::TypeSettings;
use matsu_config::settings::print::{self, MAX_PRINT_SETTINGS, PrintSettings};
use matsu_platform::print::default_setting;
#[cfg(any(windows, test))]
use matsu_platform::print::devmode::PrinterChoice;
use matsu_print::{PrintSetting, Printer};

pub struct PrintPrefs {
    settings: PrintSettings,
    /// The number chosen in the dialog for a document type, over the one the type has. It is also
    /// written to the file, but this window needs it at once and without a file.
    chosen: HashMap<usize, usize>,
    /// The settings file to save to; nothing is saved without one (the tests).
    store: Option<PathBuf>,
}

impl Global for PrintPrefs {}

impl Default for PrintPrefs {
    fn default() -> Self {
        Self {
            settings: PrintSettings::new(&default_setting()),
            chosen: HashMap::new(),
            store: None,
        }
    }
}

impl PrintPrefs {
    /// What the file the application read has, to be saved to `store`. What the file does not
    /// have is Sakura's default in the Mincho font of the OS.
    pub fn load(settings: Option<&Path>, store: Option<PathBuf>) -> Self {
        let profile = settings.and_then(|path| Profile::load(path).ok());
        let base = default_setting();
        let settings = match &profile {
            Some(profile) => PrintSettings::from_profile(&base, profile),
            None => PrintSettings::new(&base),
        };
        Self {
            settings,
            chosen: HashMap::new(),
            store,
        }
    }

    /// Remembers `printer` in the setting `index`. Only the printer is written, inside the locked
    /// update of the file, so what another process saved meanwhile (margins, fonts, paper) stays;
    /// the settings are then what the file has.
    fn set_printer(&mut self, index: usize, printer: Printer) {
        let Some(path) = self.store.clone() else {
            self.settings.get_mut(index).printer = Some(printer);
            return;
        };
        // Not compared with the printer this process has cached: another process may have saved
        // another one since, and the last one used must win. The file is what is compared (the
        // byte comparison below writes nothing when it is the same).
        let base = default_setting();
        let mut merged = None;
        let saved = update_profile(&path, |profile| {
            let before = profile.to_bytes();
            print::write_printer(Some(&printer), profile, index)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
            merged = Some(PrintSettings::from_profile(&base, profile));
            Ok(profile.to_bytes() != before)
        });
        match (saved, merged) {
            (Ok(_), Some(now)) => self.settings = now,
            // A file that cannot be written leaves the printer for this session.
            _ => self.settings.get_mut(index).printer = Some(printer),
        }
    }

    /// Writes what the printer dialog of the preview bar gave into the setting `index`: the
    /// paper size and the direction, and the printer when the dialog named one. Sakura changes
    /// those three of the setting in the shared data (`CPrintPreview::DispatchEvent_PPB`,
    /// `PrintDlg` and `m_nPrintPaperSize`). The setting is written to the file as it is there with
    /// the three changed (inside the locked update, so what another process saved meanwhile
    /// stays), and the settings are then what the file has.
    #[cfg(any(windows, test))]
    fn set_choice(&mut self, index: usize, choice: &PrinterChoice) {
        let apply = |slot: &mut PrintSetting| {
            slot.print_paper_size = choice.paper_size;
            slot.print_paper_orientation = choice.orientation;
            if let Some(printer) = &choice.printer {
                slot.printer = Some(printer.clone());
            }
        };
        let Some(path) = self.store.clone() else {
            apply(self.settings.get_mut(index));
            return;
        };
        let base = default_setting();
        let mut merged = None;
        let saved = update_profile(&path, |profile| {
            let before = profile.to_bytes();
            let on_disk = PrintSettings::from_profile(&base, profile);
            let mut now = on_disk.clone();
            apply(now.get_mut(index));
            now.write_changed(&on_disk, profile)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
            merged = Some(now);
            Ok(profile.to_bytes() != before)
        });
        match (saved, merged) {
            (Ok(_), Some(now)) => self.settings = now,
            // A file that cannot be written leaves the choice for this session.
            _ => apply(self.settings.get_mut(index)),
        }
    }

    /// Makes `new` the settings. The ones that differ from `original`, which the dialog started
    /// with, are written to the file (the file is read again first, so what another window saved
    /// meanwhile stays), and so is `current`, the number of the setting of a document type
    /// (`(type_index, number)`), when there is one.
    fn change(
        &mut self,
        new: &PrintSettings,
        original: &PrintSettings,
        current: Option<(usize, usize)>,
    ) {
        let before = original.clone();
        if let Some((type_index, number)) = current {
            self.chosen.insert(type_index, number);
        }
        let Some(path) = self.store.clone() else {
            // the slots another dialog changed meanwhile stay
            let mut now = self.settings.clone();
            replace_changed(&mut now, new, &before);
            self.settings = now;
            return;
        };
        let base = default_setting();
        let mut merged = None;
        let saved = update_profile(&path, |profile| {
            (|| -> Result<bool, matsu_config::ProfileError> {
                let on_disk = PrintSettings::from_profile(&base, profile);
                let mut now = on_disk.clone();
                replace_changed(&mut now, new, &before);
                let before_bytes = profile.to_bytes();
                now.write_changed(&on_disk, profile)?;
                if let Some((type_index, number)) = current {
                    print::store_current(profile, type_index, number)?;
                }
                merged = Some(now);
                Ok(profile.to_bytes() != before_bytes)
            })()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
        });
        // A file that cannot be written leaves the settings for this session.
        self.settings = match (saved, merged) {
            (Ok(_), Some(now)) => now,
            // The dialog's changes go over what this session has, which keeps its printer.
            _ => {
                let mut now = self.settings.clone();
                replace_changed(&mut now, new, &before);
                now
            }
        };
    }
}

/// Puts the slots of `new` that the dialog changed (they differ from `before`) over `now`. The dialog
/// cannot edit the printer, which a print sets: the printer `now` has stays (one owner, the
/// preferences), whatever the dialog's older copy holds.
fn replace_changed(now: &mut PrintSettings, new: &PrintSettings, before: &PrintSettings) {
    for (i, slot) in new.slots().iter().enumerate() {
        if before.slots().get(i) != Some(slot) {
            let printer = now.get(i).printer.clone();
            now.set(
                i,
                PrintSetting {
                    printer,
                    ..slot.clone()
                },
            );
        }
    }
}

/// Makes the settings of the application. Called once at startup.
pub fn install(cx: &mut App, prefs: PrintPrefs) {
    cx.set_global(prefs);
}

/// The eight settings as they are now.
pub(crate) fn settings(cx: &App) -> PrintSettings {
    cx.try_global::<PrintPrefs>()
        .map_or_else(|| PrintPrefs::default().settings, |p| p.settings.clone())
}

/// Which of the eight the document type prints with: the one chosen in the dialog, or the number
/// the type has.
pub(crate) fn current_index(cx: &App, type_index: usize, type_settings: &TypeSettings) -> usize {
    cx.try_global::<PrintPrefs>()
        .and_then(|p| p.chosen.get(&type_index).copied())
        .unwrap_or_else(|| print::current_of(type_settings))
        .min(MAX_PRINT_SETTINGS - 1)
}

/// The setting the document type prints with.
pub(crate) fn current_setting(
    cx: &App,
    type_index: usize,
    type_settings: &TypeSettings,
) -> PrintSetting {
    settings(cx)
        .get(current_index(cx, type_index, type_settings))
        .clone()
}

/// What OK of the dialog keeps (`CEditWnd::OnPrintPageSetting`).
pub(crate) fn commit(
    cx: &mut App,
    new: &PrintSettings,
    original: &PrintSettings,
    type_index: usize,
    current: usize,
) {
    cx.default_global::<PrintPrefs>()
        .change(new, original, Some((type_index, current)));
}

/// Remembers `printer` in the setting `index`, which the print dialog just printed on
/// (`CPrint::PrintDlg` stores the printer it was given in the setting's `m_mdmDevMode`). Nothing
/// is written when the setting has that printer already.
pub(crate) fn remember_printer(cx: &mut App, index: usize, printer: Printer) {
    cx.default_global::<PrintPrefs>()
        .set_printer(index, printer);
}

/// Writes what the printer dialog of the preview bar gave into the setting `index` (see
/// [`PrintPrefs::set_choice`]).
#[cfg(any(windows, test))]
pub(crate) fn remember_choice(cx: &mut App, index: usize, choice: &PrinterChoice) {
    cx.default_global::<PrintPrefs>().set_choice(index, choice);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printer(name: &str) -> Printer {
        Printer {
            driver: "winspool".into(),
            device: name.into(),
            output: "PORTPROMPT:".into(),
        }
    }

    /// A settings file written by another process (or this one), and the preferences that read it.
    fn prefs_with_file(dir: &tempfile::TempDir) -> (PrintPrefs, PathBuf) {
        let path = dir.path().join("matsu.ini");
        let defaults = PrintSettings::new(&default_setting());
        let mut changed = defaults.clone();
        changed.get_mut(0).print_dansuu = 2;
        update_profile(&path, |profile| {
            changed.write_changed(&defaults, profile).unwrap();
            Ok(true)
        })
        .unwrap();
        (PrintPrefs::load(Some(&path), Some(path.clone())), path)
    }

    fn on_disk(path: &Path) -> PrintSettings {
        PrintSettings::from_profile(&default_setting(), &Profile::load(path).unwrap())
    }

    /// The dialog cannot edit the printer: when it commits an older copy, the printer another
    /// window remembered meanwhile stays, in memory and in the file.
    #[test]
    fn the_dialog_does_not_put_an_old_printer_back() {
        let dir = tempfile::tempdir().unwrap();
        let (mut prefs, path) = prefs_with_file(&dir);
        let original = prefs.settings.clone();
        // another window prints and remembers a printer
        prefs.set_printer(0, printer("PDF"));
        // the dialog, opened before, changes a margin and presses OK
        let mut new = original.clone();
        new.get_mut(0).print_margin_lx = 150;
        prefs.change(&new, &original, None);
        assert_eq!(prefs.settings.get(0).printer, Some(printer("PDF")));
        assert_eq!(prefs.settings.get(0).print_margin_lx, 150);
        assert_eq!(on_disk(&path).get(0).printer, Some(printer("PDF")));
        assert_eq!(on_disk(&path).get(0).print_margin_lx, 150);
    }

    /// Remembering the printer writes only the printer: what another process saved to the setting
    /// meanwhile is not lost, and the preferences take it.
    #[test]
    fn remembering_the_printer_keeps_what_another_process_saved() {
        let dir = tempfile::tempdir().unwrap();
        let (mut prefs, path) = prefs_with_file(&dir);
        let before = on_disk(&path);
        let mut theirs = before.clone();
        theirs.get_mut(0).print_margin_lx = 330;
        theirs.get_mut(0).print_font_height = 40;
        update_profile(&path, |profile| {
            theirs.write_changed(&before, profile).unwrap();
            Ok(true)
        })
        .unwrap();
        prefs.set_printer(0, printer("PDF"));
        let now = on_disk(&path);
        assert_eq!(now.get(0).print_margin_lx, 330);
        assert_eq!(now.get(0).print_font_height, 40);
        assert_eq!(now.get(0).printer, Some(printer("PDF")));
        assert_eq!(prefs.settings.get(0).print_margin_lx, 330);
        assert_eq!(prefs.settings.get(0).printer, Some(printer("PDF")));
    }

    /// A settings file that cannot be written: the session keeps the printer it remembered when an
    /// older dialog commits.
    #[test]
    fn the_dialog_does_not_put_an_old_printer_back_when_the_file_cannot_be_written() {
        let dir = tempfile::tempdir().unwrap();
        // A directory is not a file that can be written.
        let mut prefs = PrintPrefs::load(None, Some(dir.path().to_owned()));
        let original = prefs.settings.clone();
        prefs.set_printer(0, printer("PDF"));
        let mut new = original.clone();
        new.get_mut(0).print_margin_lx = 150;
        prefs.change(&new, &original, None);
        assert_eq!(prefs.settings.get(0).print_margin_lx, 150);
        assert_eq!(prefs.settings.get(0).printer, Some(printer("PDF")));
    }

    /// Two processes share the file: both load printer A, one prints on B, then this one prints
    /// on A. The last one used (A) is written, although this process had A cached.
    #[test]
    fn the_last_printer_used_is_written_although_this_process_had_it_cached() {
        let dir = tempfile::tempdir().unwrap();
        let (mut mine, path) = prefs_with_file(&dir);
        mine.set_printer(0, printer("A"));
        let mut other = PrintPrefs::load(Some(&path), Some(path.clone()));
        other.set_printer(0, printer("B"));
        assert_eq!(on_disk(&path).get(0).printer, Some(printer("B")));
        // this process still has A cached and prints on A again
        mine.set_printer(0, printer("A"));
        assert_eq!(on_disk(&path).get(0).printer, Some(printer("A")));
        assert_eq!(mine.settings.get(0).printer, Some(printer("A")));
    }

    fn choice(
        device: Option<&str>,
        size: i16,
        orientation: matsu_print::Orientation,
    ) -> PrinterChoice {
        PrinterChoice {
            printer: device.map(printer),
            paper_size: size,
            orientation,
        }
    }

    /// The printer button of the preview bar writes the paper and the direction with the printer,
    /// and keeps what the file has in the rest of the setting (another process may have saved
    /// it).
    #[test]
    fn the_choice_of_the_printer_dialog_is_written_with_the_rest_of_the_file_kept() {
        let dir = tempfile::tempdir().unwrap();
        let (mut mine, path) = prefs_with_file(&dir);
        // another process changes a margin after this one loaded
        let mut other = PrintPrefs::load(Some(&path), Some(path.clone()));
        let original = other.settings.clone();
        let mut new = original.clone();
        new.get_mut(0).print_margin_lx = 150;
        other.change(&new, &original, None);

        mine.set_choice(
            0,
            &choice(
                Some("PDF"),
                matsu_print::DMPAPER_A3,
                matsu_print::Orientation::Landscape,
            ),
        );
        for s in [&mine.settings, &on_disk(&path)] {
            assert_eq!(s.get(0).print_paper_size, matsu_print::DMPAPER_A3);
            assert_eq!(
                s.get(0).print_paper_orientation,
                matsu_print::Orientation::Landscape
            );
            assert_eq!(s.get(0).printer, Some(printer("PDF")));
            assert_eq!(s.get(0).print_margin_lx, 150);
            // what the file had for the settings of this one: dansuu 2 of `prefs_with_file`
            assert_eq!(s.get(0).print_dansuu, 2);
            // another slot is untouched
            assert_eq!(s.get(1).print_paper_size, matsu_print::DMPAPER_A4);
        }

        // a dialog that names no printer keeps the printer
        mine.set_choice(
            0,
            &choice(
                None,
                matsu_print::DMPAPER_A4,
                matsu_print::Orientation::Portrait,
            ),
        );
        assert_eq!(mine.settings.get(0).printer, Some(printer("PDF")));
        assert_eq!(on_disk(&path).get(0).printer, Some(printer("PDF")));
        assert_eq!(
            on_disk(&path).get(0).print_paper_size,
            matsu_print::DMPAPER_A4
        );
    }

    #[test]
    fn the_choice_stays_in_the_session_when_the_file_cannot_be_written() {
        let dir = tempfile::tempdir().unwrap();
        // A directory is not a file that can be written.
        let mut prefs = PrintPrefs::load(None, Some(dir.path().to_owned()));
        prefs.set_choice(
            2,
            &choice(
                Some("PDF"),
                matsu_print::DMPAPER_A3,
                matsu_print::Orientation::Landscape,
            ),
        );
        assert_eq!(
            prefs.settings.get(2).print_paper_size,
            matsu_print::DMPAPER_A3
        );
        assert_eq!(prefs.settings.get(2).printer, Some(printer("PDF")));
        // without a file at all, too
        let mut prefs = PrintPrefs::default();
        prefs.set_choice(
            2,
            &choice(
                Some("PDF"),
                matsu_print::DMPAPER_A3,
                matsu_print::Orientation::Landscape,
            ),
        );
        assert_eq!(
            prefs.settings.get(2).print_paper_size,
            matsu_print::DMPAPER_A3
        );
    }

    #[test]
    fn the_same_printer_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (mut prefs, path) = prefs_with_file(&dir);
        prefs.set_printer(0, printer("PDF"));
        let bytes = std::fs::read(&path).unwrap();
        prefs.set_printer(0, printer("PDF"));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}
