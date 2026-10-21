//! The options of Sakura's common settings that the editing and moving commands read.
//!
//! Sakura keeps them in `CommonSetting` (`sakura_core/env/CommonSetting.h`), shared by every
//! window, and the commands read them where they need them (`m_Common.m_sGeneral`,
//! `m_Common.m_sEdit`). Here they are one value for the whole process, set by the application
//! when it reads the settings file and when the common settings dialog is closed with OK
//! ([`set`]), and read by the commands ([`get`]). The defaults are Sakura's
//! (`CShareData::InitShareData`, `sakura_core/env/CShareData.cpp`).

use std::sync::RwLock;

/// The options. Each field names the Sakura setting it stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditOptions {
    /// `m_bStopsBothEndsWhenSearchWord` (`bStopsBothEndsWhenSearchWord`): moving by words stops at
    /// both ends of a word. Off by default.
    pub stops_both_ends_word: bool,
    /// `m_bStopsBothEndsWhenSearchParagraph` (`bStopsBothEndsWhenSearchParagraph`): moving by
    /// paragraphs stops at both ends of a paragraph. Off by default.
    pub stops_both_ends_paragraph: bool,
    /// `m_bEnableNoSelectCopy` (`bEnableNoSelectCopy`): copy and cut without a selection take the
    /// row. On by default.
    pub enable_no_select_copy: bool,
    /// `m_bEnableLineModePaste` (`bEnableLineModePaste`): a copy of a row is pasted at the start
    /// of the row. On by default.
    pub enable_line_mode_paste: bool,
    /// `m_bConvertEOLPaste` (`bConvertEOLPaste`): paste converts the line endings to the
    /// document's. Off by default.
    pub convert_eol_paste: bool,
    /// `m_bEnableExtEol` (`bEnableExtEol`): NEL, LS and PS are line endings. Off by default. The
    /// documents read it where they make lines (`ume_core::buffer::Document::set_ext_eol`).
    pub ext_eol: bool,
}

impl EditOptions {
    /// Sakura's defaults.
    pub const DEFAULT: Self = Self {
        stops_both_ends_word: false,
        stops_both_ends_paragraph: false,
        enable_no_select_copy: true,
        enable_line_mode_paste: true,
        convert_eol_paste: false,
        ext_eol: false,
    };
}

impl Default for EditOptions {
    fn default() -> Self {
        Self::DEFAULT
    }
}

static OPTIONS: RwLock<EditOptions> = RwLock::new(EditOptions::DEFAULT);

/// The options now.
pub fn get() -> EditOptions {
    // A lock that a panic poisoned still holds a whole value: `EditOptions` is `Copy`.
    *OPTIONS.read().unwrap_or_else(|e| e.into_inner())
}

/// Changes the options for every command from now on.
pub fn set(options: EditOptions) {
    *OPTIONS.write().unwrap_or_else(|e| e.into_inner()) = options;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_sakuras() {
        let d = EditOptions::default();
        assert!(!d.stops_both_ends_word && !d.stops_both_ends_paragraph && !d.convert_eol_paste);
        assert!(!d.ext_eol);
        assert!(d.enable_no_select_copy && d.enable_line_mode_paste);
    }
}
