//! The `DEVNAMES` block of the Windows print dialog, which names the printer the dialog starts on
//! and the one the user picked (`PRINTDLG::hDevNames`). Sakura keeps the three names of it in the
//! print setting (`CPrint::PrintDlg` copies them out of `DEVNAMES`; `CPrint::GetDefaultPrinter`
//! and `CPrint::CreateDC` use them). This is only the layout of the block, so it needs no OS and
//! is tested on every one.
//!
//! The block is four `WORD`s (`wDriverOffset`, `wDeviceOffset`, `wOutputOffset`, `wDefault`) and
//! then three NUL-terminated UTF-16 strings; the offsets count `WCHAR`s from the start of the
//! block.

use matsu_print::Printer;

/// The size of the four `WORD`s of the header, in `WCHAR`s.
const HEADER: usize = 4;

/// The block for `printer`, as the `WORD`s of a `DEVNAMES` (`wDefault` is 0: the printer is
/// asked for by name, not as the default). `None` for a printer that is not remembered, or one
/// whose names do not fit the 16-bit offsets.
pub fn build(printer: &Printer) -> Option<Vec<u16>> {
    if printer.is_none() {
        return None;
    }
    let mut block = vec![0u16; HEADER];
    let mut offsets = [0u16; 3];
    for (offset, text) in
        offsets
            .iter_mut()
            .zip([&printer.driver, &printer.device, &printer.output])
    {
        *offset = u16::try_from(block.len()).ok()?;
        block.extend(text.encode_utf16());
        block.push(0);
    }
    block[..3].copy_from_slice(&offsets);
    Some(block)
}

/// Whether the print dialog is asked again without a printer: it failed (`error` is what
/// `CommDlgExtendedError` gave; 0 is the user cancelling) while it was given a remembered
/// printer, which may be gone or renamed (`PDERR_PRINTERNOTFOUND`; the API does not promise a
/// fall back to the default printer).
pub fn retry_without_printer(error: u32, had_printer: bool) -> bool {
    had_printer && error != 0
}

/// The printer a `DEVNAMES` names (what the dialog returns). A block that is too short or has
/// an offset outside it gives empty names for those it cannot read.
pub fn parse(block: &[u16]) -> Printer {
    let string_at = |index: usize| -> String {
        let Some(&offset) = block.get(index) else {
            return String::new();
        };
        let Some(rest) = block.get(usize::from(offset)..) else {
            return String::new();
        };
        let end = rest.iter().position(|&c| c == 0).unwrap_or(rest.len());
        String::from_utf16_lossy(&rest[..end])
    };
    Printer {
        driver: string_at(0),
        device: string_at(1),
        output: string_at(2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printer() -> Printer {
        Printer {
            driver: "winspool".into(),
            device: "Microsoft Print to PDF".into(),
            output: "PORTPROMPT:".into(),
        }
    }

    #[test]
    fn a_block_has_the_header_and_three_strings_at_their_offsets() {
        let block = build(&printer()).unwrap();
        assert_eq!(&block[..3], &[4, 4 + 9, 4 + 9 + 23]);
        assert_eq!(block[3], 0);
        // the last string ends the block with its NUL
        assert_eq!(block.len(), 4 + 9 + 23 + 12);
        assert_eq!(*block.last().unwrap(), 0);
    }

    #[test]
    fn what_is_built_is_parsed_back() {
        let p = Printer {
            device: "複合機 (オフィス)".into(),
            ..printer()
        };
        assert_eq!(parse(&build(&p).unwrap()), p);
    }

    #[test]
    fn no_remembered_printer_gives_no_block() {
        assert_eq!(build(&Printer::default()), None);
        // a driver and a port without a name pick nothing
        let p = Printer {
            device: String::new(),
            ..printer()
        };
        assert_eq!(build(&p), None);
    }

    #[test]
    fn a_name_too_long_for_the_offsets_gives_no_block() {
        let p = Printer {
            device: "x".repeat(70_000),
            ..printer()
        };
        assert_eq!(build(&p), None);
    }

    #[test]
    fn the_dialog_is_asked_again_only_for_a_failure_with_a_remembered_printer() {
        // PDERR_PRINTERNOTFOUND
        assert!(retry_without_printer(0x100B, true));
        assert!(!retry_without_printer(0x100B, false));
        // the user cancelled
        assert!(!retry_without_printer(0, true));
    }

    #[test]
    fn a_broken_block_gives_empty_names() {
        assert_eq!(parse(&[]), Printer::default());
        // an offset past the end
        assert_eq!(parse(&[9, 9, 9, 0, b'a'.into()]), Printer::default());
        // no NUL: the string runs to the end
        let p = parse(&[4, 4, 4, 0, b'a'.into(), b'b'.into()]);
        assert_eq!(p.device, "ab");
    }
}
