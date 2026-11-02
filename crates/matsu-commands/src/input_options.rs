//! The type settings that Enter and the typed chars read: the indent side of the "画面" page.
//!
//! Sakura keeps them in `STypeConfig` (`sakura_core/env/DocTypeSetting.h`) and `Command_WCHAR`
//! reads them through `m_pTypeData`. The window hands the ones of the type of the document to the
//! context ([`CommandContext::with_input_options`](crate::context::CommandContext::with_input_options)).
//! The defaults are those of `InitTypeConfigDefault` (`sakura_core/types/CType.cpp`).

/// `m_eSmartIndent`: the two kinds Sakura has built in, and the plug of a plugin, which Sakura
/// stores as the function code of the plug (`CPlug::GetSmartIndentType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SmartIndent {
    /// `SMARTINDENT_NONE`.
    #[default]
    None,
    /// `SMARTINDENT_CPP`.
    Cpp,
    /// The plug of the `SmartIndent` jack with this function code
    /// ([`plugin_commands::plug_code`](crate::plugin_commands::plug_code)).
    Plugin(u16),
}

impl SmartIndent {
    /// From the value of `nSmartIndent` in the settings file. A value that is neither of the
    /// built-in kinds nor the code of a plug has nothing to run (Sakura finds no plug for it), so
    /// it is [`SmartIndent::None`].
    pub fn from_setting(value: i32) -> Self {
        match value {
            1 => Self::Cpp,
            _ => u16::try_from(value)
                .ok()
                .filter(|&code| crate::plugin_commands::is_plugin_code(code))
                .map_or(Self::None, Self::Plugin),
        }
    }

    /// The value for `nSmartIndent`.
    pub fn setting(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Cpp => 1,
            Self::Plugin(code) => i32::from(code),
        }
    }
}

/// The plug of a plugin that has to be called for a char that was just typed (`Command_WCHAR` in
/// Sakura calls it right after the insertion). The command only says so; the window decides
/// whether the plugin is called for this char and runs it, because the plugin runs in another
/// process and the command cannot wait for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SmartIndentCall {
    /// The function code of the plug ([`SmartIndent::Plugin`]).
    pub code: u16,
    /// The char that was typed (`\r` for a line break, as `Command_WCHAR` has it).
    pub typed: char,
    /// The command took the char for a line break (`WCODE::IsLineDelimiter`: CR and LF, and NEL, LS and
    /// PS while the document counts them as line endings).
    pub line_break: bool,
}

/// What the color strategies say a line starts in: the state that the lines before it leave
/// (`CLayout::GetColorTypePrev`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LineStart {
    /// Not in a string or a comment, or in one that the C/C++ smart indent does not look at.
    #[default]
    Normal,
    /// In a char literal that goes on from the line before (`COLORIDX_SSTRING`).
    SingleQuote,
    /// In a string that goes on from the line before (`COLORIDX_WSTRING`).
    DoubleQuote,
    /// In the block comment 1 that goes on from the line before (`COLORIDX_BLOCK1`).
    BlockComment,
}

/// Tells the state a line starts in. The window has the color states of the document; a command
/// has no colors of its own. Without one every line starts in [`LineStart::Normal`].
pub trait LineStartColors {
    /// The state at the start of `line` of `doc`.
    fn at_line_start(&self, doc: &matsu_core::buffer::Document, line: usize) -> LineStart;
}

/// The options. Each field names the Sakura setting it stands for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputOptions {
    /// `m_bAutoIndent` (`bAutoIndent`): Enter copies the indentation of the line. On by default.
    pub auto_indent: bool,
    /// `m_bAutoIndent_ZENSPACE` (`bAutoIndent_ZENSPACE`): an ideographic space is indentation.
    /// On by default.
    pub auto_indent_zenspace: bool,
    /// `m_eSmartIndent` (`nSmartIndent`). None by default.
    pub smart_indent: SmartIndent,
    /// `m_szIndentChars` (`szIndentChars`): chars that are indentation besides the blanks. Empty
    /// by default.
    pub indent_chars: String,
    /// `m_bRTrimPrevLine` (`bRTrimPrevLine`): Enter deletes the blanks at the end of the line it
    /// leaves. Off by default.
    pub rtrim_prev_line: bool,
    /// `m_bIndentCppStringIgnore` (`bIndentCppStringIgnore`): the C/C++ smart indent does not count
    /// the brackets in strings and char literals. On by default.
    pub cpp_string_ignore: bool,
    /// `m_bIndentCppCommentIgnore` (`bIndentCppCommentIgnore`): the same for comments. On by
    /// default.
    pub cpp_comment_ignore: bool,
    /// `m_bIndentCppUndoSep` (`bIndentCppUndoSep`): the C/C++ smart indent is an undo step apart
    /// from the typed char. Off by default.
    pub cpp_undo_sep: bool,
}

impl Default for InputOptions {
    fn default() -> Self {
        Self {
            auto_indent: true,
            auto_indent_zenspace: true,
            smart_indent: SmartIndent::None,
            indent_chars: String::new(),
            rtrim_prev_line: false,
            cpp_string_ignore: true,
            cpp_comment_ignore: true,
            cpp_undo_sep: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_sakuras() {
        let d = InputOptions::default();
        assert!(d.auto_indent && d.auto_indent_zenspace && !d.rtrim_prev_line);
        assert_eq!(d.smart_indent, SmartIndent::None);
        assert!(d.indent_chars.is_empty());
    }

    #[test]
    fn the_codes_of_plugs_are_plugin_smart_indents() {
        assert_eq!(SmartIndent::from_setting(0), SmartIndent::None);
        assert_eq!(SmartIndent::from_setting(1), SmartIndent::Cpp);
        // nothing is served by a value that is no kind and no plug
        assert_eq!(SmartIndent::from_setting(2), SmartIndent::None);
        assert_eq!(SmartIndent::from_setting(-1), SmartIndent::None);
        assert_eq!(SmartIndent::from_setting(19_999), SmartIndent::None);
        assert_eq!(SmartIndent::from_setting(22_000), SmartIndent::None);
        assert_eq!(
            SmartIndent::from_setting(20_000),
            SmartIndent::Plugin(20_000)
        );
        assert_eq!(
            SmartIndent::from_setting(21_949),
            SmartIndent::Plugin(21_949)
        );
        for kind in [
            SmartIndent::None,
            SmartIndent::Cpp,
            SmartIndent::Plugin(20_150),
        ] {
            assert_eq!(SmartIndent::from_setting(kind.setting()), kind);
        }
    }
}
