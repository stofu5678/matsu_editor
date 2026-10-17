//! The date and time formats of Sakura's common settings page 書式, which `F_INS_DATE` and
//! `F_INS_TIME` read.
//!
//! Sakura keeps them in `CommonSetting_Format` (`sakura_core/env/CommonSetting.h`) and formats with
//! `CFormatManager::MyGetDateFormat` and `MyGetTimeFormat` (`sakura_core/env/CFormatManager.cpp`):
//! type 0 is the format of the OS (here: Sakura's default picture, there is no such locale), type 1
//! the picture the user wrote. Like [`crate::edit_options`], they are one value for the whole
//! process, set by the application when it reads the settings file and when the dialog is closed
//! with OK.

use std::sync::RwLock;

use crate::convert::datetime::{DEFAULT_DATE_PICTURE, DEFAULT_TIME_PICTURE};

/// The formats. Each field names the Sakura setting it stands for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatOptions {
    /// `m_nDateFormatType` (`nDateFormatType`): 0 standard, 1 custom.
    pub date_type: i32,
    /// `m_szDateFormat` (`szDateFormat`): the picture of type 1.
    pub date_picture: String,
    /// `m_nTimeFormatType` (`nTimeFormatType`): 0 standard, 1 custom.
    pub time_type: i32,
    /// `m_szTimeFormat` (`szTimeFormat`): the picture of type 1.
    pub time_picture: String,
}

impl FormatOptions {
    /// The picture `F_INS_DATE` formats with (`MyGetDateFormat`: type 0 is the standard one).
    pub fn date_picture(&self) -> &str {
        if self.date_type == 0 {
            DEFAULT_DATE_PICTURE
        } else {
            &self.date_picture
        }
    }

    /// The picture `F_INS_TIME` formats with (`MyGetTimeFormat`).
    pub fn time_picture(&self) -> &str {
        if self.time_type == 0 {
            DEFAULT_TIME_PICTURE
        } else {
            &self.time_picture
        }
    }
}

/// Sakura's defaults (`CShareData::InitShareData`).
impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            date_type: 0,
            date_picture: DEFAULT_DATE_PICTURE.to_owned(),
            time_type: 0,
            time_picture: DEFAULT_TIME_PICTURE.to_owned(),
        }
    }
}

static OPTIONS: RwLock<Option<FormatOptions>> = RwLock::new(None);

/// The formats now.
pub fn get() -> FormatOptions {
    OPTIONS
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_default()
}

/// Changes the formats for every command from now on.
pub fn set(options: FormatOptions) {
    *OPTIONS.write().unwrap_or_else(|e| e.into_inner()) = Some(options);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_0_is_the_standard_picture_and_type_1_the_written_one() {
        let mut o = FormatOptions {
            date_picture: "yyyy/MM/dd".to_owned(),
            time_picture: "HH:mm".to_owned(),
            ..FormatOptions::default()
        };
        assert_eq!(o.date_picture(), DEFAULT_DATE_PICTURE);
        assert_eq!(o.time_picture(), DEFAULT_TIME_PICTURE);
        o.date_type = 1;
        o.time_type = 1;
        assert_eq!(o.date_picture(), "yyyy/MM/dd");
        assert_eq!(o.time_picture(), "HH:mm");
    }
}
