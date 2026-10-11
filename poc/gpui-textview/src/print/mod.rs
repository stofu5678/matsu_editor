//! Printing (ADR-0011): the page layout shared by the preview and the printers, the print
//! preview in GPUI, and printing on each OS: AppKit and Core Text on macOS, the print dialog and
//! GDI on Windows, and the print portal with a PDF on Linux.

pub mod layout;
#[cfg(not(any(windows, target_os = "macos")))]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod mac;
#[cfg(not(target_os = "macos"))]
pub mod pdf;
pub mod preview;
#[cfg(windows)]
pub mod windows;

use layout::{DocInfo, Paper, PrintSetting};

/// What printing needs, taken from the editor when it starts: the text, the setting, and what
/// the header shows of the document.
pub struct PrintJob {
    pub lines: Vec<String>,
    pub setting: PrintSetting,
    pub doc: DocInfo,
}

/// Sakura prints in ＭＳ 明朝 by default. These are the Mincho fonts that each OS has.
#[cfg(target_os = "macos")]
const PRINT_FONT: &str = "Hiragino Mincho ProN";
#[cfg(windows)]
const PRINT_FONT: &str = "MS Mincho";
#[cfg(not(any(windows, target_os = "macos")))]
const PRINT_FONT: &str = "Noto Serif CJK JP";

/// Sakura's default print setting, in the Mincho font of the OS.
pub fn default_setting() -> PrintSetting {
    PrintSetting {
        font_han: PRINT_FONT.into(),
        font_zen: PRINT_FONT.into(),
        ..PrintSetting::default()
    }
}

/// The paper to lay the pages out on, and its name for the preview bar.
pub fn paper() -> (Paper, String) {
    #[cfg(target_os = "macos")]
    return mac::paper();
    // Sakura's default, with no printer to ask.
    #[cfg(not(target_os = "macos"))]
    (Paper::A4, "A4  縦".into())
}

/// `--print-pdf <input> <output.pdf>` writes the PDF that Linux prints, on Windows too.
/// `--print-test <input> <output.pdf>` prints without the print dialog: on Windows through GDI to
/// "Microsoft Print to PDF" (or the printer in `POC_PRINTER`), and elsewhere to the PDF. Both
/// print in Sakura's default setting on A4. `None` when the arguments are neither.
#[cfg(not(target_os = "macos"))]
pub fn run_cli(args: &[String]) -> Option<Result<String, String>> {
    let [_, mode, input, output] = args else {
        return None;
    };
    if mode != "--print-pdf" && mode != "--print-test" {
        return None;
    }
    let (input, output) = (std::path::Path::new(input), std::path::Path::new(output));
    Some((|| {
        let bytes = std::fs::read(input).map_err(|e| format!("{}: {e}", input.display()))?;
        // Split as the editor splits the text it opens.
        let job = PrintJob {
            lines: String::from_utf8_lossy(&bytes)
                .split('\n')
                .map(|line| line.trim_end_matches('\r').to_string())
                .collect(),
            setting: default_setting(),
            doc: DocInfo {
                file_name: input
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned()),
                path: Some(input.display().to_string()),
            },
        };
        #[cfg(windows)]
        if mode == "--print-test" {
            let printer =
                std::env::var("POC_PRINTER").unwrap_or_else(|_| "Microsoft Print to PDF".into());
            let output = std::path::absolute(output).map_err(|e| e.to_string())?;
            let pages = windows::print_to_file(&printer, &output, &job)?;
            return Ok(format!("{pages} pages to {printer}: {}", output.display()));
        }
        let (pdf, pages) = pdf::render(&job, Paper::A4)?;
        std::fs::write(output, pdf).map_err(|e| format!("{}: {e}", output.display()))?;
        Ok(format!("{pages} pages: {}", output.display()))
    })())
}
