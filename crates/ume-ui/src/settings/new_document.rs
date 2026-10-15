//! What a new document (or a file that does not say) gets: the encoding, the BOM and the line
//! ending.
//!
//! Sakura keeps them per document type (`STypeConfig::m_encoding`: `m_eDefaultCodetype`,
//! `m_bDefaultBom`, `m_eDefaultEoltype`, `sakura_core/types/CType.cpp`, read as `eDefaultCodetype`,
//! `bDefaultBom` and `eDefaultEoltype` of `Types(N)`) and applies them when a document is made or
//! a file has no line ending (`CDocEditor::OnAfterLoad`, `sakura_core/doc/CDocEditor.cpp`).
//!
//! UME Editor differs from Sakura in one default (the user decided on 2026-10-15, ADR-0010):
//! when `ume.ini` does not set the line ending, it follows the OS ([`Eol::platform_default`]),
//! where Sakura always says CR LF. So the values the file sets must be told from the built-in
//! defaults ([`TypeSettings::new_file_keys`]). A value of the document's own type wins; a type
//! that does not set a value takes the basic type's (`Types(0)`); then comes UME Editor's default.

use ume_config::Profile;
use ume_config::settings::types::eol_type;
use ume_config::settings::{TypeSettings, types_count};
use ume_core::encoding::{Encoding, Eol};

use super::doc_type::type_index_for;

/// The encoding, BOM and line ending of a new document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewDocumentDefaults {
    pub encoding: Encoding,
    pub bom: bool,
    pub eol: Eol,
}

impl Default for NewDocumentDefaults {
    /// Without a settings file: UTF-8, no BOM, and the line ending of the OS.
    fn default() -> Self {
        Self {
            encoding: Encoding::Utf8,
            bom: false,
            eol: Eol::platform_default(),
        }
    }
}

impl NewDocumentDefaults {
    /// The defaults for a file called `file_name` (the name only; `None` for an untitled
    /// document, which has the basic type).
    pub fn from_profile(profile: &Profile, file_name: Option<&str>) -> Self {
        let index = file_name.map_or(0, |name| {
            let exts: Vec<String> = (0..types_count(profile))
                .map(|i| TypeSettings::read(profile, i).type_exts)
                .collect();
            type_index_for(exts.iter().map(String::as_str), name)
        });
        let own = TypeSettings::new_file_keys(profile, i32::try_from(index).unwrap_or(0));
        let basic = TypeSettings::new_file_keys(profile, 0);

        let fallback = Self::default();
        // Encodings that are Windows code pages or UTF-32 are not supported yet (another PR adds
        // code pages); until then they fall back to UTF-8.
        let encoding = own
            .codetype
            .or(basic.codetype)
            .and_then(encoding_of)
            .unwrap_or(fallback.encoding);
        let bom = own
            .bom
            .or(basic.bom)
            .unwrap_or_else(|| encoding.bom_default_on());
        let eol = own
            .eoltype
            .or(basic.eoltype)
            .and_then(eol_of)
            .unwrap_or(fallback.eol);
        Self { encoding, bom, eol }
    }
}

/// `ECodeType` of the setting as an encoding the default may be (`SCodeSet::m_bCanDefault`).
fn encoding_of(code: i32) -> Option<Encoding> {
    Encoding::from_sakura_code(code).filter(|e| e.can_be_default())
}

/// `EEolType` of the setting. "No conversion" (0) and unknown values are no line ending to use.
fn eol_of(value: i32) -> Option<Eol> {
    match value {
        eol_type::CRLF => Some(Eol::CrLf),
        eol_type::LF => Some(Eol::Lf),
        eol_type::CR => Some(Eol::Cr),
        eol_type::NEL => Some(Eol::Nel),
        eol_type::LS => Some(Eol::Ls),
        eol_type::PS => Some(Eol::Ps),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(text: &str) -> Profile {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        Profile::from_bytes(&bytes)
    }

    fn os_eol() -> Eol {
        if cfg!(windows) { Eol::CrLf } else { Eol::Lf }
    }

    #[test]
    fn without_a_file_it_is_utf8_without_bom_and_the_os_line_ending() {
        let d = NewDocumentDefaults::from_profile(&Profile::new(), None);
        assert_eq!(d, NewDocumentDefaults::default());
        assert_eq!(d.encoding, Encoding::Utf8);
        assert!(!d.bom);
        assert_eq!(d.eol, os_eol());
    }

    #[test]
    fn a_file_that_does_not_set_the_values_follows_the_os() {
        let p = profile("[Types(0)]\r\nszTypeName=基本\r\n");
        assert_eq!(NewDocumentDefaults::from_profile(&p, None).eol, os_eol());
    }

    #[test]
    fn values_of_the_basic_type_win() {
        let p =
            profile("[Types(0)]\r\neDefaultCodetype=0\r\neDefaultEoltype=1\r\nbDefaultBom=0\r\n");
        let d = NewDocumentDefaults::from_profile(&p, None);
        assert_eq!(d.encoding, Encoding::ShiftJis);
        assert_eq!(d.eol, Eol::CrLf);
        assert!(!d.bom);
        // Set to LF: LF on every OS, also where the default is CR LF.
        let p = profile("[Types(0)]\r\neDefaultEoltype=2\r\n");
        assert_eq!(NewDocumentDefaults::from_profile(&p, None).eol, Eol::Lf);
    }

    #[test]
    fn the_type_of_the_file_name_wins_over_the_basic_type() {
        let p = profile(
            "[Other]\r\nnTypesCount=2\r\n[Types(0)]\r\neDefaultEoltype=2\r\neDefaultCodetype=4\r\n\
             [Types(1)]\r\nszTypeExts=bat,cmd\r\neDefaultEoltype=1\r\neDefaultCodetype=0\r\nbDefaultBom=0\r\n",
        );
        let bat = NewDocumentDefaults::from_profile(&p, Some("run.bat"));
        assert_eq!(bat.eol, Eol::CrLf);
        assert_eq!(bat.encoding, Encoding::ShiftJis);
        let other = NewDocumentDefaults::from_profile(&p, Some("a.txt"));
        assert_eq!(other.eol, Eol::Lf);
        assert_eq!(other.encoding, Encoding::Utf8);
    }

    #[test]
    fn a_type_without_the_key_takes_the_basic_types_value() {
        let p = profile(
            "[Other]\r\nnTypesCount=2\r\n[Types(0)]\r\neDefaultEoltype=3\r\n\
             [Types(1)]\r\nszTypeExts=c\r\n",
        );
        let d = NewDocumentDefaults::from_profile(&p, Some("a.c"));
        assert_eq!(d.eol, Eol::Cr);
    }

    #[test]
    fn utf16_has_a_bom_unless_the_file_says_not() {
        let p = profile("[Types(0)]\r\neDefaultCodetype=3\r\n");
        let d = NewDocumentDefaults::from_profile(&p, None);
        assert_eq!(d.encoding, Encoding::Utf16Le);
        assert!(d.bom);
        let p = profile("[Types(0)]\r\neDefaultCodetype=3\r\nbDefaultBom=0\r\n");
        assert!(!NewDocumentDefaults::from_profile(&p, None).bom);
    }

    #[test]
    fn unsupported_values_fall_back() {
        // UTF-32 and code pages are not supported yet; UTF-7 cannot be a default; 0 for the line
        // ending is "no conversion".
        for code in [12000, 932, 5] {
            let p = profile(&format!("[Types(0)]\r\neDefaultCodetype={code}\r\n"));
            let d = NewDocumentDefaults::from_profile(&p, None);
            assert_eq!(d.encoding, Encoding::Utf8, "{code}");
        }
        let p = profile("[Types(0)]\r\neDefaultEoltype=0\r\n");
        assert_eq!(NewDocumentDefaults::from_profile(&p, None).eol, os_eol());
    }
}
