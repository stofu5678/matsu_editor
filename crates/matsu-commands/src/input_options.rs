//! The type settings that Enter and the typed chars read: the indent side of the "画面" page.
//!
//! Sakura keeps them in `STypeConfig` (`sakura_core/env/DocTypeSetting.h`) and `Command_WCHAR`
//! reads them through `m_pTypeData`. The window hands the ones of the type of the document to the
//! context ([`CommandContext::with_input_options`](crate::context::CommandContext::with_input_options)).
//! The defaults are those of `InitTypeConfigDefault` (`sakura_core/types/CType.cpp`).

/// `m_eSmartIndent`: the kinds Sakura has built in. The values of the plug-ins (`>= 2`) are not
/// supported yet and count as [`SmartIndent::None`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SmartIndent {
    /// `SMARTINDENT_NONE`.
    #[default]
    None,
    /// `SMARTINDENT_CPP`.
    Cpp,
}

impl SmartIndent {
    /// From the value of `nSmartIndent` in the settings file.
    pub fn from_setting(value: i32) -> Self {
        match value {
            1 => Self::Cpp,
            _ => Self::None,
        }
    }
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
}

impl Default for InputOptions {
    fn default() -> Self {
        Self {
            auto_indent: true,
            auto_indent_zenspace: true,
            smart_indent: SmartIndent::None,
            indent_chars: String::new(),
            rtrim_prev_line: false,
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
    fn plug_in_smart_indents_count_as_none() {
        assert_eq!(SmartIndent::from_setting(0), SmartIndent::None);
        assert_eq!(SmartIndent::from_setting(1), SmartIndent::Cpp);
        assert_eq!(SmartIndent::from_setting(2), SmartIndent::None);
        assert_eq!(SmartIndent::from_setting(-1), SmartIndent::None);
    }
}
