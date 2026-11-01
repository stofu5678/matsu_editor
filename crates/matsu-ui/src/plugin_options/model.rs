//! What the plugin option dialog holds: one entry for each `[Option]` item of `plugin.def`, with
//! the value it has now, and how the values become the text of the list and the file
//! (`CDlgPluginOption::SetData`, `GetData`, `SetFromEdit`, `sakura_core/dlg/CDlgPluginOption.cpp`).

use std::io;
use std::path::Path;

use matsu_config::plugins::PluginDef;
use matsu_config::plugins::def::{OptionDef, OptionType};
use matsu_config::plugins::options::PluginOptions;

/// `BOOL_DISP_TRUE` and `BOOL_DISP_FALSE`.
pub(crate) const BOOL_TRUE: &str = "\u{2611}";
pub(crate) const BOOL_FALSE: &str = "\u{2610}";

/// `MAX_LENGTH_VALUE`: the longest value a text box takes.
pub(crate) const MAX_LENGTH_VALUE: usize = 1024;

/// `_wtoi` on a value, as `int`: leading white space, a sign and digits; anything else is 0, and
/// a number out of range stops at the end of the range.
pub(crate) fn atoi(text: &str) -> i32 {
    let text = text.trim_start();
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let mut value: i64 = 0;
    for byte in digits.bytes().take_while(u8::is_ascii_digit) {
        value = (value * 10 + i64::from(byte - b'0')).min(i64::from(i32::MAX) + 1);
    }
    let value = if negative { -value } else { value };
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// Sakura treats an empty value and `0` as off, and everything else as on.
fn is_on(value: &str) -> bool {
    !(value.is_empty() || value == "0")
}

/// One item of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) def: OptionDef,
    /// The value as the options file keeps it (a bool is `0` or `1`, an int a number, a choice
    /// the value of the item).
    pub(crate) value: String,
}

impl Entry {
    /// The text of the column 値.
    pub(crate) fn shown(&self) -> String {
        match self.def.kind {
            OptionType::Bool => (if is_on(&self.value) {
                BOOL_TRUE
            } else {
                BOOL_FALSE
            })
            .to_owned(),
            OptionType::Int => atoi(&self.value).to_string(),
            // A value that no item has shows nothing.
            OptionType::Sel => self
                .def
                .selects()
                .into_iter()
                .find(|(_, value)| *value == self.value)
                .map(|(shown, _)| shown)
                .unwrap_or_default(),
            OptionType::Str | OptionType::Dir => self.value.clone(),
        }
    }
}

/// The dialog's data: the plugin, its options and the file they are kept in.
pub(crate) struct Form {
    /// The name of the plugin (`m_sName`), for the title.
    pub(crate) name: String,
    pub(crate) entries: Vec<Entry>,
    options: PluginOptions,
}

impl Form {
    /// The entries of `def`, with the values of the file `option_file` (the default of the
    /// definition for an option the file does not have, `SetData`).
    pub(crate) fn load(def: &PluginDef, option_file: &Path) -> Self {
        let options = PluginOptions::load(option_file);
        let entries = def
            .options
            .iter()
            .map(|option| Entry {
                value: normalize(option, &options.value(option)),
                def: option.clone(),
            })
            .collect();
        Form {
            name: def.name.clone(),
            entries,
            options,
        }
    }

    /// The title of the dialog (`STR_DLGPLUGINOPT_TITLE`).
    pub(crate) fn title(&self) -> String {
        format!("{} プラグインの設定", self.name)
    }

    /// Sets the value of the entry `index` (`SetFromEdit`). For a bool, `value` is `0` or `1`;
    /// for an int, the text of the box; for a choice, the value of the item.
    pub(crate) fn set(&mut self, index: usize, value: &str) {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.value = normalize(&entry.def, value);
        }
    }

    /// Writes the values to the options file (`GetData`): all of them, also the ones that were not
    /// changed, as Sakura does. The lines of the file that are not options stay.
    pub(crate) fn save(&self) -> io::Result<()> {
        let mut options = self.options.clone();
        for entry in &self.entries {
            options
                .set_option(&entry.def, &entry.value)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
        }
        options.save()
    }
}

/// The value as the file keeps it for the type of the option.
fn normalize(def: &OptionDef, value: &str) -> String {
    match def.kind {
        OptionType::Bool => (if is_on(value) { "1" } else { "0" }).to_owned(),
        OptionType::Int => atoi(value).to_string(),
        _ => value.chars().take(MAX_LENGTH_VALUE).collect(),
    }
}

/// `SelectDirectory`: the folder chosen, with the separator at its end.
pub(crate) fn with_trailing_separator(dir: &str) -> String {
    if dir.ends_with(['/', '\\']) {
        dir.to_owned()
    } else {
        format!("{dir}{}", std::path::MAIN_SEPARATOR)
    }
}
