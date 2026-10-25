//! The options of a plugin: `plugins\<folder>.ini` (ADR-0026).
//!
//! `[Option]` of `plugin.def` says which options there are ([`super::OptionDef`]); the values are in
//! the plugin's own file, in the section and key that the option names (`help/plugin/Text/
//! implementation01.html`, `CPlugin::GetOptionPath`). A plugin reads and writes them with
//! `Plugin.GetOption` and `Plugin.SetOption`; the settings dialog of the plugin edits them.
//!
//! The file is a [`Profile`], so lines that this program does not know stay when it writes.

use std::io;
use std::path::{Path, PathBuf};

use super::def::{OptionDef, OptionType};
use crate::profile::{Profile, ProfileError};

/// The options file of one plugin.
#[derive(Clone, Debug, Default)]
pub struct PluginOptions {
    path: PathBuf,
    profile: Profile,
}

impl PluginOptions {
    /// The options in `path`; a file that is not there is an empty one.
    pub fn load(path: &Path) -> Self {
        PluginOptions {
            path: path.to_owned(),
            profile: Profile::load(path).unwrap_or_default(),
        }
    }

    /// The value of the option `def`: what the file has, else the default of the definition.
    pub fn value(&self, def: &OptionDef) -> String {
        self.profile
            .get(&def.section, &def.key)
            .map_or_else(|| def.default.clone(), str::to_owned)
    }

    /// The value in `section` and `key` (`Plugin.GetOption`), or `None` when the file has none.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.profile.get(section, key)
    }

    /// Sets the value in `section` and `key` (`Plugin.SetOption`).
    pub fn set(&mut self, section: &str, key: &str, value: &str) -> Result<(), ProfileError> {
        self.profile.set(section, key, value)
    }

    /// Sets an option to `value` after making it fit the type: a `Bool` is `0` or `1`, an `Int` a
    /// number (what is not one becomes `0`).
    pub fn set_option(&mut self, def: &OptionDef, value: &str) -> Result<(), ProfileError> {
        let value = match def.kind {
            OptionType::Bool => {
                let on = matches!(value.trim(), "1" | "true" | "True" | "TRUE");
                (if on { "1" } else { "0" }).to_owned()
            }
            OptionType::Int => value.trim().parse::<i32>().unwrap_or(0).to_string(),
            _ => value.to_owned(),
        };
        self.set(&def.section, &def.key, &value)
    }

    /// Writes the file (the folder is made if it is not there).
    pub fn save(&self) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        self.profile.save(&self.path)
    }
}
