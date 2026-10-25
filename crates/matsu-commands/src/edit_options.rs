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
    /// documents read it where they make lines (`matsu_core::buffer::Document::set_ext_eol`).
    pub ext_eol: bool,
    /// `m_bUseOLE_DragDrop` (`bUseOLE_DragDrop`): a selection can be dragged to move or copy it,
    /// and text can be dropped on the view. On by default.
    pub drag_drop: bool,
    /// `m_bUseOLE_DropSource` (`bUseOLE_DropSource`): the view is a drag source, so pressing the
    /// mouse in the selection and moving drags the text. Needs [`drag_drop`](Self::drag_drop).
    /// On by default.
    pub drop_source: bool,
    /// `m_bSelectClickedURL` (`bSelectClickedURL`): a click on a URL selects it. On by default.
    pub select_clicked_url: bool,
    /// `m_bAddCRLFWhenCopy` (`bAddCRLFWhenCopy`): copy and cut end the rows that wrap with a line
    /// ending. Off by default.
    pub add_crlf_when_copy: bool,
    /// `bCopyAndDisablSelection`: the selection goes after a copy.
    /// Off by default.
    pub copy_and_disable_selection: bool,
    /// `m_bNotOverWriteCRLF` (`bNotOverWriteCRLF`): typing in overwrite mode at the end of a line
    /// inserts instead of deleting the line ending. On by default.
    pub not_overwrite_crlf: bool,
    /// `m_bOverWriteFixMode` (`bOverWriteFixMode`): overwriting a char with one of another width
    /// fills the difference with spaces. Off by default.
    pub overwrite_fix_mode: bool,
    /// `m_bOverWriteBoxDelete` (`bOverWriteBoxDelete`): typing over a box in overwrite mode deletes
    /// the box first. Off by default.
    pub overwrite_box_delete: bool,
    /// `m_bIsFreeCursorMode` (`bIsFreeCursorMode`): the caret may stand to the right of the end of
    /// a line. Off by default.
    pub free_cursor: bool,
    /// `GetCaretType() == 1` (`nCaretType`, the low 4 bits): the MS-DOS style caret, a block (half
    /// as high in insert mode) instead of the thin bar of the Windows style. Off by default.
    pub dos_caret: bool,
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
        drag_drop: true,
        drop_source: true,
        select_clicked_url: true,
        add_crlf_when_copy: false,
        copy_and_disable_selection: false,
        not_overwrite_crlf: true,
        overwrite_fix_mode: false,
        overwrite_box_delete: false,
        free_cursor: false,
        dos_caret: false,
    };
}

impl Default for EditOptions {
    fn default() -> Self {
        Self::DEFAULT
    }
}

static OPTIONS: RwLock<EditOptions> = RwLock::new(EditOptions::DEFAULT);

#[cfg(test)]
thread_local! {
    /// The options a test of this crate sets for its own thread: the tests run in parallel and
    /// share the process-wide value otherwise.
    static TEST_OPTIONS: std::cell::Cell<Option<EditOptions>> = const { std::cell::Cell::new(None) };
}

/// Runs `f` with `options` as the options of this thread (tests only).
#[cfg(test)]
pub(crate) fn with_options<R>(options: EditOptions, f: impl FnOnce() -> R) -> R {
    let old = TEST_OPTIONS.with(|c| c.replace(Some(options)));
    let result = f();
    TEST_OPTIONS.with(|c| c.set(old));
    result
}

/// The options now.
pub fn get() -> EditOptions {
    #[cfg(test)]
    if let Some(options) = TEST_OPTIONS.with(std::cell::Cell::get) {
        return options;
    }
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
        assert!(!d.add_crlf_when_copy && !d.copy_and_disable_selection);
        assert!(d.not_overwrite_crlf && !d.overwrite_fix_mode && !d.overwrite_box_delete);
    }
}
