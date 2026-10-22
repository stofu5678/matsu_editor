//! The arguments of the file commands. Sakura's macros pass numbers for the encoding and the line
//! ending (`help/macro/source/reference/file/S_FileSaveAs.html`).

use std::path::PathBuf;

use matsu_commands::command::{Arg, CommandError};
use matsu_core::buffer::Eol;
use matsu_core::encoding::{AUTODETECT_CODE, Encoding};

/// The path in argument `i`, if there is one. An empty text is no path.
pub(crate) fn path_arg(args: &[Arg], i: usize) -> Result<Option<PathBuf>, CommandError> {
    match args.get(i) {
        None => Ok(None),
        Some(Arg::Text(t)) if t.is_empty() => Ok(None),
        Some(Arg::Text(t)) => Ok(Some(PathBuf::from(t))),
        Some(other) => Err(CommandError::BadArgs(format!(
            "argument {} must be a file name, not {other:?}",
            i + 1
        ))),
    }
}

fn int_arg(args: &[Arg], i: usize) -> Result<Option<i64>, CommandError> {
    match args.get(i) {
        None => Ok(None),
        Some(Arg::Int(n)) => Ok(Some(*n)),
        Some(other) => Err(CommandError::BadArgs(format!(
            "argument {} must be a number, not {other:?}",
            i + 1
        ))),
    }
}

/// The encoding in argument `i`: Sakura's `ECodeType` number (0 SJIS, 1 JIS, 2 EUC, 3 UTF-16,
/// 4 UTF-8, 5 UTF-7, 6 UTF-16BE, 7 CESU-8, 8 Latin1) or a Windows code page number that
/// `Encoding::from_sakura_code` knows (936 for GBK and so on). Missing and 99 (`CODE_AUTODETECT`)
/// mean "keep the current one" for saving and "detect" for opening.
pub(crate) fn encoding_arg(args: &[Arg], i: usize) -> Result<Option<Encoding>, CommandError> {
    let Some(code) = int_arg(args, i)? else {
        return Ok(None);
    };
    if code == i64::from(AUTODETECT_CODE) {
        return Ok(None);
    }
    i32::try_from(code)
        .ok()
        .and_then(Encoding::from_sakura_code)
        .map(Some)
        .ok_or_else(|| CommandError::BadArgs(format!("{code} is not an encoding number")))
}

/// The line ending in argument `i`: 0 do not change, 1 CRLF, 2 LF, 3 CR (`S_FileSaveAs`).
pub(crate) fn eol_arg(args: &[Arg], i: usize) -> Result<Option<Eol>, CommandError> {
    match int_arg(args, i)? {
        None | Some(0) => Ok(None),
        Some(1) => Ok(Some(Eol::CrLf)),
        Some(2) => Ok(Some(Eol::Lf)),
        Some(3) => Ok(Some(Eol::Cr)),
        Some(n) => Err(CommandError::BadArgs(format!(
            "{n} is not a line ending number (0 to 3)"
        ))),
    }
}

/// "Do not ask" for the reopen commands (`bNoConfirm`, `S_FileReopenUTF8`): non-zero.
pub(crate) fn no_confirm_arg(args: &[Arg]) -> bool {
    matches!(args.first(), Some(Arg::Int(n)) if *n != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_numbers_are_sakuras() {
        assert_eq!(
            encoding_arg(&[Arg::Int(4)], 0).unwrap(),
            Some(Encoding::Utf8)
        );
        assert_eq!(
            encoding_arg(&[Arg::Int(0)], 0).unwrap(),
            Some(Encoding::ShiftJis)
        );
        assert_eq!(
            encoding_arg(&[Arg::Int(936)], 0).unwrap(),
            Encoding::from_sakura_code(936)
        );
        assert!(encoding_arg(&[Arg::Int(936)], 0).unwrap().is_some());
        assert_eq!(encoding_arg(&[Arg::Int(99)], 0).unwrap(), None);
        assert_eq!(encoding_arg(&[], 0).unwrap(), None);
        assert!(encoding_arg(&[Arg::Int(42)], 0).is_err());
        assert!(encoding_arg(&[Arg::Text("x".into())], 0).is_err());
    }

    #[test]
    fn line_ending_numbers_are_sakuras() {
        assert_eq!(eol_arg(&[Arg::Int(0)], 0).unwrap(), None);
        assert_eq!(eol_arg(&[Arg::Int(1)], 0).unwrap(), Some(Eol::CrLf));
        assert_eq!(eol_arg(&[Arg::Int(2)], 0).unwrap(), Some(Eol::Lf));
        assert_eq!(eol_arg(&[Arg::Int(3)], 0).unwrap(), Some(Eol::Cr));
        assert!(eol_arg(&[Arg::Int(4)], 0).is_err());
    }

    #[test]
    fn paths_and_the_confirm_flag() {
        assert_eq!(path_arg(&[], 0).unwrap(), None);
        assert_eq!(path_arg(&[Arg::Text(String::new())], 0).unwrap(), None);
        assert_eq!(
            path_arg(&[Arg::Text("a.txt".into())], 0).unwrap(),
            Some(PathBuf::from("a.txt"))
        );
        assert!(path_arg(&[Arg::Int(1)], 0).is_err());
        assert!(!no_confirm_arg(&[]));
        assert!(!no_confirm_arg(&[Arg::Int(0)]));
        assert!(no_confirm_arg(&[Arg::Int(1)]));
    }
}
