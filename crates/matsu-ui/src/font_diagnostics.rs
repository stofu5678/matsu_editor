//! `MATSU_FONT_DIAGNOSTICS`: finds out why the text is drawn in a font other than the one asked for.
//!
//! Only when the environment variable is set (to anything but empty or `0`), the program
//! installs a small [`log`] logger that writes GPUI's and MATSU's records to the standard error
//! with a time, and prints one report of how the font of the text is chosen once the text system
//! and the settings are known. Without the variable nothing is installed and nothing is printed.
//! Nothing is written to a file (where the user's data lives is the user's to decide).
//!
//! The pieces that build text from collected values ([`render`], [`matches_exact`],
//! [`filter_names`], [`width_ratio`], [`format_record`]) need no window and are tested alone.

use std::ffi::OsStr;
use std::io::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui::App;
use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::settings::{UserSettings, ViewSettings};

/// The name of the environment variable that turns the diagnostics on.
pub(crate) const ENV_NAME: &str = "MATSU_FONT_DIAGNOSTICS";

/// Parts of a font name that the report lists: the font MATSU asks for and its likely neighbors.
const NAME_NEEDLES: [&str; 6] = ["BIZ", "UD", "ゴシック", "Osaka", "Hiragino", "ヒラギノ"];

/// The characters whose advance widths the report measures: a half-width digit, a wide Latin
/// letter, a hiragana and a kanji.
const PROBE_CHARS: [char; 4] = ['0', 'M', 'あ', '漢'];

/// The prefixes of the targets (the modules of the crates) whose info and debug records are kept:
/// GPUI and the font libraries under it, and MATSU. Records of any other target are kept from
/// `Warn` up.
const DETAILED_TARGETS: [&str; 7] = [
    "gpui",
    "matsu",
    "font_kit",
    "core_text",
    "cosmic_text",
    "dwrote",
    "swash",
];

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Whether the variable's value turns the diagnostics on: set, not empty and not `0`.
pub(crate) fn is_on(value: Option<&OsStr>) -> bool {
    value.is_some_and(|v| !v.is_empty() && v != OsStr::new("0"))
}

/// Installs the logger if the environment asks for it. Returns whether the diagnostics are on.
/// Called first thing in `launch`, so that the records of GPUI's start-up are seen.
pub(crate) fn install_from_env() -> bool {
    install(std::env::var_os(ENV_NAME).as_deref())
}

fn install(value: Option<&OsStr>) -> bool {
    if !is_on(value) {
        return false;
    }
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(LevelFilter::Debug);
    }
    ENABLED.store(true, Ordering::Relaxed);
    log::info!(target: "matsu::font_diagnostics", "{ENV_NAME} is set: logging to the standard error");
    true
}

/// Whether the diagnostics are on in this process.
pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

struct Logger;

static LOGGER: Logger = Logger;

/// Whether a record of `level` from `target` is kept.
pub(crate) fn wants(level: Level, target: &str) -> bool {
    level <= Level::Warn
        || (level <= Level::Debug && DETAILED_TARGETS.iter().any(|p| target.starts_with(p)))
}

/// One line of the log: `12:34:56.789 [WARN] target: message`.
pub(crate) fn format_record(time: &str, level: Level, target: &str, message: &str) -> String {
    format!("{time} [{level}] {target}: {message}")
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        wants(metadata.level(), metadata.target())
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let time = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        let line = format_record(
            &time,
            record.level(),
            record.target(),
            &record.args().to_string(),
        );
        // A failed write to the standard error is not worth a panic.
        writeln!(std::io::stderr().lock(), "{line}").ok();
    }

    fn flush(&self) {}
}

/// What the report knows about how GPUI resolved the font that was asked for.
#[derive(Clone, Debug, Default)]
pub(crate) struct Resolution {
    /// The `FontId` of each family that was resolved, with a label: the asked family first, then
    /// a family that does not exist (what GPUI gives for a font it cannot load), then the
    /// fallbacks GPUI tries in order.
    pub ids: Vec<(String, usize)>,
    /// The family that `TextSystem::get_font_for_id` reports for the id of the asked family.
    pub reported_family: Option<String>,
}

/// One measured advance width (in pixels at the font size), or why it could not be measured.
pub(crate) type Advance = (char, Result<f32, String>);

/// The facts the report is made of.
#[derive(Clone, Debug, Default)]
pub(crate) struct Facts {
    pub os: String,
    pub matsu_version: String,
    pub locale: String,
    /// The family of the settings file (`None`: the settings name no font).
    pub settings_family: Option<String>,
    /// `FONT_FAMILY`, what MATSU asks for when the settings name no font.
    pub default_family: String,
    /// The family that MATSU asks GPUI for.
    pub asked_family: String,
    /// The font the text is drawn with, and why (`view::text_font`): on macOS, what stands in for
    /// a family that is not installed.
    pub used_font: String,
    pub font_size: f32,
    /// The document types that have a font of their own (`bUseTypeFont`): the type's index and
    /// the family asked of GPUI for it. A window draws with the font of its document's type, which
    /// can differ from the basic type's (the one the rest of the report is about).
    pub type_fonts: Vec<(usize, String)>,
    pub weight: f32,
    pub italic: bool,
    pub all_names: Vec<String>,
    pub resolution: Resolution,
    pub advances: Vec<Advance>,
    /// Lines of the facts that only one OS can collect.
    pub extra: Vec<String>,
}

/// Whether `names` has `family` as it is.
pub(crate) fn matches_exact(names: &[String], family: &str) -> bool {
    names.iter().any(|n| n == family)
}

/// The names equal to `family` when the case is ignored (a name that is there only in another
/// case shows that the lookup, not the font, is wrong).
pub(crate) fn matches_ignoring_case(names: &[String], family: &str) -> Vec<String> {
    let wanted = family.to_lowercase();
    names
        .iter()
        .filter(|n| n.to_lowercase() == wanted)
        .cloned()
        .collect()
}

/// The names that contain any of `needles`, ignoring the case.
pub(crate) fn filter_names(names: &[String], needles: &[&str]) -> Vec<String> {
    let needles: Vec<String> = needles.iter().map(|n| n.to_lowercase()).collect();
    names
        .iter()
        .filter(|n| {
            let lower = n.to_lowercase();
            needles.iter().any(|needle| lower.contains(needle))
        })
        .cloned()
        .collect()
}

/// The width of a full-width character over that of a half-width one; 2.0 means 1:2. `None` if
/// the half-width one has no width.
pub(crate) fn width_ratio(full: f32, half: f32) -> Option<f32> {
    (half > 0.0).then(|| full / half)
}

fn advance_of(advances: &[Advance], ch: char) -> Option<f32> {
    advances
        .iter()
        .find(|(c, _)| *c == ch)
        .and_then(|(_, w)| w.as_ref().ok().copied())
}

/// The report, from the facts.
pub(crate) fn render(facts: &Facts) -> String {
    let mut out = String::new();
    let mut line = |text: String| {
        out.push_str(&text);
        out.push('\n');
    };
    line(format!("== {ENV_NAME}: how the text font is chosen =="));
    line(format!("os: {}", facts.os));
    line(format!("matsu: {}", facts.matsu_version));
    line(format!("locale: {}", facts.locale));
    line(format!(
        "settings font family: {}",
        facts
            .settings_family
            .as_deref()
            .map_or("(none: the settings name no font)".to_owned(), |f| format!(
                "{f:?}"
            ))
    ));
    line(format!(
        "default family (FONT_FAMILY): {:?}",
        facts.default_family
    ));
    line(format!("family asked of GPUI: {:?}", facts.asked_family));
    line(format!("font the text is drawn with: {}", facts.used_font));
    line(format!(
        "font size: {} px, weight {}, italic {}",
        facts.font_size, facts.weight, facts.italic
    ));

    let names = &facts.all_names;
    if facts.type_fonts.is_empty() {
        line("types with a font of their own: none".to_owned());
    } else {
        line("types with a font of their own (a window uses its document's type):".to_owned());
        for (index, family) in &facts.type_fonts {
            line(format!("  type {index}: {family:?}"));
        }
    }
    line(format!("all_font_names(): {} names", names.len()));
    line(format!(
        "  has {:?} exactly: {}",
        facts.asked_family,
        matches_exact(names, &facts.asked_family)
    ));
    let loose = matches_ignoring_case(names, &facts.asked_family);
    line(format!(
        "  has it ignoring the case: {}{}",
        !loose.is_empty(),
        if loose.is_empty() {
            String::new()
        } else {
            format!(" {loose:?}")
        }
    ));
    let near = filter_names(names, &NAME_NEEDLES);
    line(format!(
        "  names containing one of {NAME_NEEDLES:?} ({}):",
        near.len()
    ));
    for name in &near {
        line(format!("    {name}"));
    }

    line("resolve_font:".to_owned());
    let asked_id = facts.resolution.ids.first().map(|(_, id)| *id);
    for (index, (label, id)) in facts.resolution.ids.iter().enumerate() {
        let same = if index > 0 && Some(*id) == asked_id {
            "  <- the same font as the asked family"
        } else {
            ""
        };
        line(format!("  {label}: FontId({id}){same}"));
    }
    match (asked_id, facts.resolution.ids.get(1)) {
        (Some(asked), Some((_, missing))) if asked == *missing => line(
            "  => the asked family has the same id as a family that does not exist: GPUI fell back to that font (or the asked family is that very font, e.g. Helvetica)"
                .to_owned(),
        ),
        (Some(_), Some(_)) => line(
            "  => the asked family got an id of its own (it loaded, or it fell back to a font not listed here)"
                .to_owned(),
        ),
        _ => line("  => no id".to_owned()),
    }
    if let Some(family) = &facts.resolution.reported_family {
        line(format!(
            "  get_font_for_id(asked id) reports the family: {family:?}"
        ));
    }

    line(format!(
        "advance widths at {} px (of the font the text is drawn with):",
        facts.font_size
    ));
    for (ch, width) in &facts.advances {
        match width {
            Ok(w) => line(format!("  {ch:?} (U+{:04X}): {w:.3}", *ch as u32)),
            Err(e) => line(format!("  {ch:?} (U+{:04X}): error: {e}", *ch as u32)),
        }
    }
    let ratio = advance_of(&facts.advances, 'あ')
        .zip(advance_of(&facts.advances, '0'))
        .and_then(|(full, half)| width_ratio(full, half));
    match ratio {
        Some(r) => line(format!(
            "  full-width (あ) / half-width (0): {r:.3}{}",
            if (r - 2.0).abs() < 0.01 { " (1:2)" } else { "" }
        )),
        None => line("  full-width / half-width: unknown".to_owned()),
    }
    for extra in &facts.extra {
        line(extra.clone());
    }
    line("== end ==".to_owned());
    out
}

/// Prints the report to the standard error if the diagnostics are on. Called when the text system
/// and the settings are ready (the start of the application).
pub(crate) fn report(cx: &App, user: &UserSettings) {
    if !enabled() {
        return;
    }
    // The basic type (type 0), which a new document has; the types with a font of their own
    // are listed beside it.
    let text = render(&collect(cx, &user.view_settings(None), type_fonts(user)));
    // One write, so that the lines of a record from another thread do not split the report.
    write!(std::io::stderr().lock(), "{text}").ok();
}

/// The types of the user's settings that use a font of their own, with the family asked of GPUI
/// for each.
fn type_fonts(user: &UserSettings) -> Vec<(usize, String)> {
    (0..user.document_types().len())
        .filter_map(|index| {
            let settings = user.view_settings_of_type(index);
            settings.use_type_font.then(|| {
                let family = settings
                    .font_family
                    .unwrap_or_else(|| crate::view::FONT_FAMILY.to_owned());
                (index, family)
            })
        })
        .collect()
}

/// Collects the facts from the text system, the settings and the OS.
fn collect(cx: &App, settings: &ViewSettings, type_fonts: Vec<(usize, String)>) -> Facts {
    let system = cx.text_system();
    let default_family = crate::view::FONT_FAMILY.to_owned();
    let asked_family = settings
        .font_family
        .clone()
        .unwrap_or_else(|| default_family.clone());
    let font_size = settings.font_px();

    let all_names = system.all_font_names();

    // The font the view asks for: its weight and slant too (`EditorView::render`).
    let with_style = |family: &str| {
        let mut font = gpui::font(family.to_owned());
        font.weight = gpui::FontWeight(settings.gpui_font_weight());
        if settings.font_italic {
            font.style = gpui::FontStyle::Italic;
        }
        font
    };
    let asked_id = system.resolve_font(&with_style(&asked_family));
    let reported_family = system
        .get_font_for_id(asked_id)
        .map(|f| f.family.to_string());
    let mut ids = vec![(asked_family.clone(), asked_id.0)];
    // GPUI falls back to a list of its own (`TextSystem::new`) when a family cannot be loaded; a
    // family that cannot exist shows which font that is.
    for (label, family) in [
        (
            "(a family that does not exist)",
            "matsu-no-such-font-for-diagnostics",
        ),
        (".ZedMono", ".ZedMono"),
        (".ZedSans", ".ZedSans"),
        ("Helvetica", "Helvetica"),
        ("Arial", "Arial"),
    ] {
        let id = system.resolve_font(&with_style(family));
        ids.push((label.to_owned(), id.0));
    }

    // What the text is drawn with: the asked family, or on macOS what stands in for it.
    let used = crate::view::text_font::for_family(settings.font_family.as_deref(), cx);
    let used_id = system.resolve_font(&gpui::Font {
        weight: gpui::FontWeight(settings.gpui_font_weight()),
        style: if settings.font_italic {
            gpui::FontStyle::Italic
        } else {
            gpui::FontStyle::Normal
        },
        ..used.font()
    });
    let size = gpui::px(font_size);
    let advances = PROBE_CHARS
        .iter()
        .map(|&ch| {
            let width = system
                .advance(used_id, size, ch)
                .map(|s| f32::from(s.width))
                .map_err(|e| e.to_string());
            (ch, width)
        })
        .collect();

    Facts {
        os: os_description(),
        matsu_version: env!("CARGO_PKG_VERSION").to_owned(),
        locale: locale_description(),
        settings_family: settings.font_family.clone(),
        default_family,
        asked_family,
        used_font: used.describe(),
        font_size,
        type_fonts,
        weight: settings.gpui_font_weight(),
        italic: settings.font_italic,
        all_names,
        resolution: Resolution {
            ids,
            reported_family,
        },
        advances,
        extra: platform_lines(),
    }
}

/// The output of `program args` (trimmed), or `None` if it cannot be run or fails.
fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn os_description() -> String {
    let version = if cfg!(target_os = "macos") {
        command_output("sw_vers", &["-productVersion"])
    } else if cfg!(windows) {
        command_output("cmd", &["/C", "ver"])
    } else {
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|text| {
                text.lines()
                    .find_map(|l| l.strip_prefix("PRETTY_NAME="))
                    .map(|v| v.trim_matches('"').to_owned())
            })
    };
    format!(
        "{} {}, version {}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        version.as_deref().unwrap_or("(unknown)")
    )
}

fn locale_description() -> String {
    let mut parts: Vec<String> = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| std::env::var(name).ok().map(|v| format!("{name}={v}")))
        .collect();
    if cfg!(target_os = "macos") {
        if let Some(v) = command_output("defaults", &["read", "-g", "AppleLocale"]) {
            parts.push(format!("AppleLocale={v}"));
        }
        if let Some(v) = command_output("defaults", &["read", "-g", "AppleLanguages"]) {
            let compact: String = v.split_whitespace().collect();
            parts.push(format!("AppleLanguages={compact}"));
        }
    }
    if parts.is_empty() {
        "(unknown)".to_owned()
    } else {
        parts.join(", ")
    }
}

/// What only this OS can tell. On macOS: where Spotlight finds the BIZ UDGothic file. (It is not
/// what CoreText sees: that needs `core-text` as a direct dependency, and GPUI's own log lines
/// show what CoreText did with the family.)
#[cfg(target_os = "macos")]
fn platform_lines() -> Vec<String> {
    let found = command_output("mdfind", &["-name", "BIZ_UDGothic"]);
    vec![format!(
        "mdfind -name BIZ_UDGothic (Spotlight, not CoreText): {}",
        found.as_deref().unwrap_or("(nothing found)")
    )]
}

#[cfg(not(target_os = "macos"))]
fn platform_lines() -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn the_variable_turns_it_on_unless_empty_or_zero() {
        assert!(!is_on(None));
        assert!(!is_on(Some(OsStr::new(""))));
        assert!(!is_on(Some(OsStr::new("0"))));
        assert!(is_on(Some(OsStr::new("1"))));
        assert!(is_on(Some(OsStr::new("yes"))));
    }

    #[test]
    fn without_the_variable_no_logger_is_installed() {
        assert!(!install(None));
        assert!(!install(Some(OsStr::new("0"))));
        assert!(!enabled());
        assert_eq!(log::max_level(), LevelFilter::Off);
    }

    #[test]
    fn warnings_are_kept_everywhere_and_the_rest_only_for_gpui_and_matsu() {
        assert!(wants(Level::Error, "anything"));
        assert!(wants(Level::Warn, "hyper"));
        assert!(!wants(Level::Info, "hyper"));
        assert!(wants(Level::Debug, "gpui_pre_macos::text_system"));
        assert!(wants(Level::Info, "matsu_ui::view"));
        assert!(wants(Level::Debug, "font_kit::loaders::core_text"));
        assert!(!wants(Level::Trace, "gpui"));
    }

    #[test]
    fn a_record_is_one_line_with_the_time() {
        assert_eq!(
            format_record("12:34:56.789", Level::Warn, "gpui::x", "no glyph"),
            "12:34:56.789 [WARN] gpui::x: no glyph"
        );
    }

    #[test]
    fn the_family_is_looked_up_exactly_and_ignoring_the_case() {
        let list = names(&["Arial", "BIZ UDGothic", "biz udpgothic"]);
        assert!(matches_exact(&list, "BIZ UDGothic"));
        assert!(!matches_exact(&list, "BIZ UDPGothic"));
        assert_eq!(
            matches_ignoring_case(&list, "BIZ UDPGothic"),
            names(&["biz udpgothic"])
        );
        assert!(matches_ignoring_case(&list, "Courier").is_empty());
    }

    #[test]
    fn the_names_are_narrowed_by_the_needles() {
        let list = names(&[
            "Arial",
            "BIZ UDPGothic",
            "Hiragino Sans",
            "ヒラギノ角ゴ ProN",
            "Osaka",
            "MS ゴシック",
            "Times",
            "Menlo",
        ]);
        assert_eq!(
            filter_names(&list, &NAME_NEEDLES),
            names(&[
                "BIZ UDPGothic",
                "Hiragino Sans",
                "ヒラギノ角ゴ ProN",
                "Osaka",
                "MS ゴシック"
            ])
        );
        // the case is ignored
        assert_eq!(
            filter_names(&names(&["Studio", "Times"]), &["UD"]),
            names(&["Studio"])
        );
    }

    #[test]
    fn the_width_ratio_is_two_for_a_one_to_two_font() {
        assert_eq!(width_ratio(16.0, 8.0), Some(2.0));
        assert_eq!(width_ratio(7.0, 0.0), None);
        let helvetica = width_ratio(16.0, 8.896).unwrap();
        assert!((helvetica - 1.7986).abs() < 0.001);
    }

    fn facts(asked_id: usize, missing_id: usize) -> Facts {
        Facts {
            os: "macos aarch64, version 27.0.1".into(),
            matsu_version: "0.8.0".into(),
            locale: "LANG=ja_JP.UTF-8".into(),
            settings_family: None,
            default_family: "BIZ UDGothic".into(),
            asked_family: "BIZ UDGothic".into(),
            used_font: "\"Menlo\" with fallbacks [\"Hiragino Sans\"] (\"BIZ UDGothic\" is not installed: Menlo with Hiragino Sans)".into(),
            font_size: 16.0,
            type_fonts: Vec::new(),
            weight: 400.0,
            italic: false,
            all_names: names(&["Arial", "BIZ UDPGothic"]),
            resolution: Resolution {
                ids: vec![
                    ("BIZ UDGothic".into(), asked_id),
                    ("(a family that does not exist)".into(), missing_id),
                    (".ZedMono".into(), 7),
                ],
                reported_family: Some("Helvetica".into()),
            },
            advances: vec![
                ('0', Ok(8.896)),
                ('M', Ok(13.3)),
                ('あ', Ok(16.0)),
                ('漢', Err("glyph not found".into())),
            ],
            extra: vec!["extra line".into()],
        }
    }

    #[test]
    fn the_report_says_the_asked_family_did_not_load() {
        let text = render(&facts(3, 3));
        assert!(
            text.contains("same id as a family that does not exist"),
            "{text}"
        );
        assert!(
            text.contains("has \"BIZ UDGothic\" exactly: false"),
            "{text}"
        );
        assert!(text.contains("BIZ UDPGothic"), "{text}");
        assert!(
            text.contains("(a family that does not exist): FontId(3)  <- the same font"),
            "{text}"
        );
        assert!(
            text.contains("'漢' (U+6F22): error: glyph not found"),
            "{text}"
        );
        assert!(
            text.contains("font the text is drawn with: \"Menlo\" with fallbacks"),
            "{text}"
        );
        assert!(text.contains("1.799"), "{text}");
        assert!(!text.contains("(1:2)"));
        assert!(text.contains("none: the settings name no font"));
        assert!(text.ends_with("== end ==\n"));
        assert!(text.contains("extra line"));
    }

    #[test]
    fn the_report_lists_the_types_with_a_font_of_their_own() {
        let mut f = facts(1, 3);
        assert!(render(&f).contains("types with a font of their own: none"));
        f.type_fonts = vec![(2, "Consolas".into()), (5, "BIZ UDGothic".into())];
        let text = render(&f);
        assert!(text.contains("  type 2: \"Consolas\""), "{text}");
        assert!(text.contains("  type 5: \"BIZ UDGothic\""), "{text}");
    }

    #[test]
    fn the_report_says_the_asked_family_loaded_and_the_one_to_two_ratio() {
        let mut f = facts(1, 3);
        f.advances = vec![('0', Ok(8.0)), ('あ', Ok(16.0))];
        let text = render(&f);
        assert!(text.contains("an id of its own"), "{text}");
        assert!(text.contains("2.000 (1:2)"), "{text}");
    }
}
