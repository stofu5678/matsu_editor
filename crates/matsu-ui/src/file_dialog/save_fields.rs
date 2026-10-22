//! The fields of the save dialog of 名前を付けて保存: the encoding, the BOM and the line ending,
//! next to the name as in Sakura (`CDlgOpenFile_CommonItemDialog::Customize`: the groups 文字コードセット
//! with `COMBO_CODE` and `CHECK_BOM`, and 改行コード with `COMBO_EOL`).
//!
//! `matsu_platform::dialog` shows them without knowing what they mean ([`Extra`]) and gives the
//! values back; this module makes them from the document's format and reads the answer.
//!
//! * The encoding list is the one of the second dialog (`save_as_dialog::encoding_choices`),
//!   without 自動選択. Sakura's `C&P` check box, which adds the Windows code pages to the list
//!   later, is left out: the list has them from the start.
//! * The BOM follows the encoding while the dialog is open where the OS lets it (Windows and
//!   macOS: [`Follow`], the rule of `OnItemSelected`). The portal cannot change one choice when
//!   another changes, so there the BOM is read as it was left, and an encoding that cannot have a
//!   BOM ignores it ([`SaveFields::choice`]).
//! * The first line ending is the document's (`変換なし` unless a line ending was chosen before),
//!   where Sakura's list always starts at `変換なし`.
//! * The lists of recent files and folders (`COMBO_MRU`, `COMBO_OPENFOLDER`) come after these fields
//!   (`Ask::recent`), so the places below do not move.

use matsu_core::buffer::Eol;
use matsu_core::encoding::Encoding;
use matsu_platform::dialog::{Extra, ExtraValue, Follow};

use crate::file_ops::save_as_dialog::{
    EOL_CHOICES, SaveChoice, bom_after_select, encoding_choices,
};

/// The fields, and what is needed to read the answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SaveFields {
    pub extras: Vec<Extra>,
    pub follow: Follow,
    choices: Vec<Encoding>,
}

/// Where the fields are in [`SaveFields::extras`].
const ENCODING: usize = 0;
const BOM: usize = 1;
const EOL: usize = 2;

impl SaveFields {
    /// The fields for a document whose encoding and BOM are `current` and whose line ending is
    /// `eol` (`None`: 変換なし).
    pub(crate) fn new(current: (Encoding, bool), eol: Option<Eol>) -> Self {
        Self::with_choices(encoding_choices(), current, eol)
    }

    fn with_choices(
        mut choices: Vec<Encoding>,
        current: (Encoding, bool),
        eol: Option<Eol>,
    ) -> Self {
        if choices.is_empty() {
            choices.push(current.0);
        }
        let selected = choices.iter().position(|e| *e == current.0).unwrap_or(0);
        let (enabled, checked) = bom_after_select(current, choices[selected]);
        // the labels have the access keys of Sakura's resources on Windows only
        let label =
            |with_key: &str, plain: &str| if cfg!(windows) { with_key } else { plain }.to_owned();
        let extras = vec![
            Extra::Combo {
                label: label("文字コードセット(&C):", "文字コードセット"),
                items: choices.iter().map(|e| e.combo_name().to_owned()).collect(),
                selected,
            },
            Extra::Check {
                label: label("&BOM", "BOM"),
                checked,
                enabled,
            },
            Extra::Combo {
                label: label("改行コード(&E):", "改行コード"),
                items: EOL_CHOICES.iter().map(|c| c.0.to_owned()).collect(),
                selected: EOL_CHOICES.iter().position(|c| c.1 == eol).unwrap_or(0),
            },
        ];
        let rule_choices = choices.clone();
        let follow = Follow::new(ENCODING, BOM, move |item| {
            let chosen = rule_choices.get(item).copied().unwrap_or(current.0);
            bom_after_select(current, chosen)
        });
        Self {
            extras,
            follow,
            choices,
        }
    }

    /// What the user chose, from the values the dialog gave back. `None` if they are not the
    /// ones asked for (the dialog showed no fields).
    pub(crate) fn choice(&self, values: &[ExtraValue]) -> Option<SaveChoice> {
        let (ExtraValue::Combo(encoding), ExtraValue::Check(bom), ExtraValue::Combo(eol)) =
            (*values.get(ENCODING)?, *values.get(BOM)?, *values.get(EOL)?)
        else {
            return None;
        };
        let encoding = *self.choices.get(encoding)?;
        Some(SaveChoice {
            encoding,
            // an encoding that cannot have a BOM ignores the check box
            bom: bom && encoding.can_have_bom(),
            eol: EOL_CHOICES.get(eol)?.1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(current: Encoding, bom: bool, eol: Option<Eol>) -> SaveFields {
        SaveFields::new((current, bom), eol)
    }

    fn index_of(fields: &SaveFields, encoding: Encoding) -> usize {
        fields.choices.iter().position(|e| *e == encoding).unwrap()
    }

    #[test]
    fn the_fields_start_with_the_format_of_the_document() {
        let f = fields(Encoding::Utf8, true, Some(Eol::Lf));
        let Extra::Combo {
            items, selected, ..
        } = &f.extras[ENCODING]
        else {
            panic!("the first field is the encoding");
        };
        assert_eq!(items.len(), f.choices.len());
        assert_eq!(*selected, index_of(&f, Encoding::Utf8));
        assert!(matches!(
            f.extras[BOM],
            Extra::Check {
                checked: true,
                enabled: true,
                ..
            }
        ));
        assert!(matches!(
            &f.extras[EOL],
            Extra::Combo { items, selected: 2, .. } if items[0] == "変換なし" && items[2] == "LF (UNIX)"
        ));
        // untouched, the values give back the document's own format
        let values = [
            ExtraValue::Combo(index_of(&f, Encoding::Utf8)),
            ExtraValue::Check(true),
            ExtraValue::Combo(2),
        ];
        assert_eq!(
            f.choice(&values),
            Some(SaveChoice {
                encoding: Encoding::Utf8,
                bom: true,
                eol: Some(Eol::Lf)
            })
        );
    }

    #[test]
    fn an_encoding_without_a_bom_starts_with_the_box_off_and_disabled() {
        let f = fields(Encoding::ShiftJis, true, None);
        assert!(matches!(
            f.extras[BOM],
            Extra::Check {
                checked: false,
                enabled: false,
                ..
            }
        ));
        assert!(matches!(f.extras[EOL], Extra::Combo { selected: 0, .. }));
    }

    #[test]
    fn the_bom_follows_the_encoding_chosen_in_the_list() {
        let f = fields(Encoding::Utf8, true, None);
        let rule = &f.follow.rule;
        // the document's encoding: its own BOM; another with a BOM: the default of that one
        assert_eq!(rule(index_of(&f, Encoding::Utf8)), (true, true));
        assert_eq!(
            rule(index_of(&f, Encoding::Utf16Le)),
            (true, Encoding::Utf16Le.bom_default_on())
        );
        // one that cannot have a BOM: off and disabled
        assert_eq!(rule(index_of(&f, Encoding::ShiftJis)), (false, false));
        // an item that is not in the list changes nothing wrong
        assert_eq!(rule(10_000), (true, true));
        assert_eq!((f.follow.combo, f.follow.check), (ENCODING, BOM));
    }

    #[test]
    fn a_bom_left_checked_is_ignored_for_an_encoding_without_one() {
        // the portal cannot uncheck the box when the encoding changes
        let f = fields(Encoding::Utf8, true, None);
        let values = [
            ExtraValue::Combo(index_of(&f, Encoding::ShiftJis)),
            ExtraValue::Check(true),
            ExtraValue::Combo(1),
        ];
        let choice = f.choice(&values).unwrap();
        assert_eq!(choice.encoding, Encoding::ShiftJis);
        assert!(!choice.bom);
        assert_eq!(choice.eol, Some(Eol::CrLf));
    }

    #[test]
    fn values_that_are_not_the_fields_give_nothing() {
        let f = fields(Encoding::Utf8, false, None);
        assert_eq!(f.choice(&[]), None);
        let wrong = [
            ExtraValue::Check(true),
            ExtraValue::Check(true),
            ExtraValue::Combo(0),
        ];
        assert_eq!(f.choice(&wrong), None);
        let out_of_range = [
            ExtraValue::Combo(10_000),
            ExtraValue::Check(false),
            ExtraValue::Combo(0),
        ];
        assert_eq!(f.choice(&out_of_range), None);
    }

    #[test]
    fn an_encoding_missing_from_the_list_does_not_break_the_fields() {
        let f = SaveFields::with_choices(Vec::new(), (Encoding::Latin1, false), None);
        assert_eq!(f.choices, [Encoding::Latin1]);
    }
}
