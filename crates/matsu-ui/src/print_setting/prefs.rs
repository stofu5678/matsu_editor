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
            for (i, slot) in new.slots().iter().enumerate() {
                if before.slots().get(i) != Some(slot) {
                    now.set(i, slot.clone());
                }
            }
            self.settings = now;
            return;
        };
        let base = default_setting();
        let mut merged = None;
        let saved = update_profile(&path, |profile| {
            (|| -> Result<bool, matsu_config::ProfileError> {
                let on_disk = PrintSettings::from_profile(&base, profile);
                let mut now = on_disk.clone();
                for (i, slot) in new.slots().iter().enumerate() {
                    if before.slots().get(i) != Some(slot) {
                        now.set(i, slot.clone());
                    }
                }
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
            _ => new.clone(),
        };
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
    let prefs = cx.default_global::<PrintPrefs>();
    let original = prefs.settings.clone();
    if original.get(index).printer == printer {
        return;
    }
    let mut new = original.clone();
    new.get_mut(index).printer = printer;
    prefs.change(&new, &original, None);
}
