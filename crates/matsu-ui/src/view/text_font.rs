//! The font the text is drawn with.
//!
//! GPUI picks a font by family name and, when the family cannot be loaded, silently falls back to
//! its own list (`.ZedMono`, `.ZedSans`, `Helvetica`, ...), which on a Mac without the fonts of
//! Zed is Helvetica: a proportional font. macOS has no BIZ UDGothic until the user downloads it in
//! Font Book, so a Mac that has not done so would draw with Helvetica. On macOS only, the family is
//! therefore chosen here: the font asked for, else BIZ UDGothic, else Menlo with Hiragino Sans as
//! its fallback (the Japanese font of the system; naming it keeps CoreText from choosing a Chinese
//! face for the kanji). Windows and Linux keep the family they were asked for.

#[cfg(not(test))]
use std::collections::HashMap;
#[cfg(not(test))]
use std::sync::{Mutex, OnceLock};

use gpui::{App, Font, FontFallbacks, SharedString, Styled};
use matsu_config::location::Os;

use super::FONT_FAMILY;

/// The monospaced font that every Mac has (half-width characters are 0.6 em).
const MAC_FALLBACK_FAMILY: &str = "Menlo";
/// The Japanese font that every Mac has, for what Menlo does not draw.
const MAC_FALLBACK_JAPANESE: &str = "Hiragino Sans";

/// Why a family was chosen, for the font diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reason {
    /// The family that was asked for is used (it is installed, or the OS is not macOS).
    AsAsked,
    /// The family that was asked for is not installed; the default family is.
    DefaultFamily { missing: String },
    /// Neither is installed: Menlo with Hiragino Sans.
    SystemFonts { missing: String },
    /// The shell tab: the editor's font is not monospaced, so a monospaced one is used instead.
    ShellMonospaced { editor: String },
    /// The shell tab: the editor's font is not monospaced and no monospaced font was found, so the
    /// editor's font is used.
    ShellNoMonospaced { editor: String },
}

/// The family and the fallback fonts to draw the text with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextFont {
    pub family: SharedString,
    pub fallbacks: Option<Vec<String>>,
    pub reason: Reason,
}

impl TextFont {
    /// The GPUI font (the weight and the slant are the caller's to set).
    pub fn font(&self) -> Font {
        let mut font = gpui::font(self.family.clone());
        font.fallbacks = self.fallbacks.clone().map(FontFallbacks::from_fonts);
        font
    }

    /// The font on an element. Not `Styled::font`, which sets the weight and the slant too.
    pub fn apply<S: Styled>(&self, element: S) -> S {
        let mut element = element.font_family(self.family.clone());
        element.text_style().font_fallbacks = self.fallbacks.clone().map(FontFallbacks::from_fonts);
        element
    }

    /// One line for the font diagnostics.
    pub fn describe(&self) -> String {
        let fonts = match &self.fallbacks {
            Some(list) => format!("{:?} with fallbacks {list:?}", self.family),
            None => format!("{:?}", self.family),
        };
        match &self.reason {
            Reason::AsAsked => format!("{fonts} (as asked)"),
            Reason::DefaultFamily { missing } => {
                format!("{fonts} ({missing:?} is not installed: the default family)")
            }
            Reason::SystemFonts { missing } => {
                format!("{fonts} ({missing:?} is not installed: a monospaced font of the system)")
            }
            Reason::ShellMonospaced { editor } => format!(
                "{fonts} (the shell tab: the editor's font {editor:?} is not monospaced, so a monospaced font is used)"
            ),
            Reason::ShellNoMonospaced { editor } => format!(
                "{fonts} (the shell tab: the editor's font {editor:?} is not monospaced, but no monospaced font was found)"
            ),
        }
    }
}

/// Chooses the font for the family asked for (`None`: the default, [`FONT_FAMILY`]).
/// `installed` tells whether GPUI can load a family; it is not called on Linux.
///
/// When the family is not installed, the text must still be monospaced: GPUI would take its own
/// default list, which is proportional on Windows (DirectWrite) and Helvetica on macOS, and the
/// layout would then follow the widths of that face. Sakura asks GDI for a fixed-pitch face
/// (`LOGFONT` with `FIXED_PITCH | FF_MODERN`, `CShareData::InitShareData` and the font dialog), and GDI's
/// mapper gives a fixed-pitch face when the one asked for is missing. So the default family comes
/// next, then the monospaced faces of the shell tab ([`shell_candidates`]). Linux keeps what it was
/// asked for (GPUI's own default there is monospaced).
pub(crate) fn choose(
    os: Os,
    asked: Option<&str>,
    mut installed: impl FnMut(&str) -> bool,
) -> TextFont {
    let asked = asked.unwrap_or(FONT_FAMILY);
    if os == Os::Linux || installed(asked) {
        return TextFont {
            family: asked.to_owned().into(),
            fallbacks: None,
            reason: Reason::AsAsked,
        };
    }
    let missing = asked.to_owned();
    if asked != FONT_FAMILY && installed(FONT_FAMILY) {
        return TextFont {
            family: FONT_FAMILY.into(),
            fallbacks: None,
            reason: Reason::DefaultFamily { missing },
        };
    }
    for (family, fallbacks) in shell_candidates(os) {
        if family != asked && installed(family) {
            return TextFont {
                family: family.into(),
                fallbacks,
                reason: Reason::SystemFonts { missing },
            };
        }
    }
    if os == Os::MacOs {
        // Menlo is on every Mac, whatever `installed` says.
        return TextFont {
            family: MAC_FALLBACK_FAMILY.into(),
            fallbacks: Some(vec![MAC_FALLBACK_JAPANESE.to_owned()]),
            reason: Reason::SystemFonts { missing },
        };
    }
    TextFont {
        family: asked.to_owned().into(),
        fallbacks: None,
        reason: Reason::AsAsked,
    }
}

/// The characters whose advances tell whether a font is monospaced.
#[cfg(not(test))]
const MONOSPACE_PROBES: [char; 5] = ['i', 'm', 'W', '0', '.'];

/// The size the probes are measured at, in pixels.
#[cfg(not(test))]
const MEASURE_SIZE: f32 = 100.0;

/// How much the advances of the probes may differ, relative to the widest, in a monospaced font
/// (measuring noise from hinting and rounding).
const MONOSPACE_TOLERANCE: f32 = 0.02;

/// Whether the advances (of the probe characters) are equal within the tolerance. Fewer than two
/// advances cannot tell, so they count as monospaced: the font is then left as it is.
pub(crate) fn advances_are_monospaced(advances: &[f32]) -> bool {
    let widest = advances.iter().copied().fold(0.0_f32, f32::max);
    let narrowest = advances.iter().copied().fold(f32::INFINITY, f32::min);
    advances.len() < 2 || widest <= 0.0 || (widest - narrowest) <= widest * MONOSPACE_TOLERANCE
}

/// The monospaced fonts to try for the shell tab, in order, as (family, fallbacks for what the family
/// does not draw). Windows: BIZ UDGothic and MS Gothic (`ＭＳ ゴシック`) draw Japanese themselves;
/// Consolas does not, so Japanese fonts are named for it. macOS: Menlo with Hiragino Sans, as above.
/// Linux: the CJK monospaced families of the common packages (Noto CJK, Source Han Code, IPA, VL),
/// then DejaVu Sans Mono (the Latin monospaced font of every common distribution; GPUI's fallback
/// draws Japanese).
fn shell_candidates(os: Os) -> Vec<(&'static str, Option<Vec<String>>)> {
    let some = |list: &[&str]| Some(list.iter().map(|s| (*s).to_owned()).collect());
    match os {
        Os::Windows => vec![
            ("BIZ UDGothic", None),
            ("ＭＳ ゴシック", None),
            ("Consolas", some(&["Yu Gothic", "Meiryo"])),
        ],
        Os::MacOs => vec![(MAC_FALLBACK_FAMILY, some(&[MAC_FALLBACK_JAPANESE]))],
        Os::Linux => vec![
            ("Noto Sans Mono CJK JP", None),
            ("Source Han Code JP", None),
            ("IPAGothic", None),
            ("VL Gothic", None),
            ("DejaVu Sans Mono", None),
        ],
    }
}

/// The font for the shell tab. The shell draws on a grid of equal cells, so the editor's font is
/// kept when it is monospaced; when it is not, the first installed monospaced font is used.
/// `monospaced` tells whether the editor's font is (by its measured advances).
pub(crate) fn for_shell_with(
    os: Os,
    editor: TextFont,
    monospaced: impl FnOnce(&TextFont) -> bool,
    mut installed: impl FnMut(&str) -> bool,
) -> TextFont {
    if monospaced(&editor) {
        return editor;
    }
    let editor_name = editor.family.to_string();
    for (family, fallbacks) in shell_candidates(os) {
        if family != editor.family.as_ref() && installed(family) {
            return TextFont {
                family: family.into(),
                fallbacks,
                reason: Reason::ShellMonospaced {
                    editor: editor_name,
                },
            };
        }
    }
    TextFont {
        reason: Reason::ShellNoMonospaced {
            editor: editor_name,
        },
        ..editor
    }
}

/// The font for the shell tab, on this OS. Whether the editor's font is monospaced is measured
/// once per family.
pub(crate) fn for_shell(editor: TextFont, cx: &App) -> TextFont {
    for_shell_with(
        Os::current(),
        editor,
        |font| is_monospaced(font, cx),
        |family| installed(family, cx),
    )
}

/// In tests the text system has no real fonts: every font counts as monospaced, so the shell tab
/// has the font it was given. The decision is tested through [`for_shell_with`].
#[cfg(test)]
fn is_monospaced(_font: &TextFont, _cx: &App) -> bool {
    true
}

#[cfg(not(test))]
/// The advances are measured at one fixed size ([`MEASURE_SIZE`], large so that hinting and rounding
/// are small against the advances), whatever size the shell is drawn at, so the family alone decides
/// the answer and is the cache key. A cached answer never goes stale within a process: the answer for
/// a family at that size does not change while the program runs (a font installed meanwhile is not
/// noticed until the next start, as with `installed`).
#[cfg(not(test))]
fn is_monospaced(font: &TextFont, cx: &App) -> bool {
    static CACHE: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = font.family.to_string();
    if let Some(known) = cache.lock().ok().and_then(|c| c.get(&key).copied()) {
        return known;
    }
    let system = cx.text_system();
    let id = system.resolve_font(&font.font());
    let advances: Vec<f32> = MONOSPACE_PROBES
        .iter()
        .filter_map(|&ch| system.advance(id, gpui::px(MEASURE_SIZE), ch).ok())
        .map(|s| f32::from(s.width))
        .collect();
    let result = advances_are_monospaced(&advances);
    if let Ok(mut c) = cache.lock() {
        c.insert(key, result);
    }
    result
}

/// The font for the family asked for, on this OS. What GPUI can load is asked once per family.
pub(crate) fn for_family(asked: Option<&str>, cx: &App) -> TextFont {
    choose(Os::current(), asked, |family| installed(family, cx))
}

/// In tests every family counts as installed, so that the font is the family asked for on every
/// OS. GPUI's test platform has a text system that gives every family the same id, which makes
/// `is_loadable` depend on which family was looked up first, and the answers are kept in a
/// process-wide cache that parallel tests share.
#[cfg(test)]
fn installed(_family: &str, _cx: &App) -> bool {
    true
}

/// A family that cannot exist: GPUI gives the font it falls back to for it.
#[cfg(not(test))]
pub(crate) const IMPOSSIBLE_FAMILY: &str = "matsu-no-such-font-for-diagnostics";

#[cfg(not(test))]
fn installed(family: &str, cx: &App) -> bool {
    static CACHE: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(known) = cache.lock().ok().and_then(|c| c.get(family).copied()) {
        return known;
    }
    let found = is_loadable(cx.text_system(), family);
    if let Ok(mut c) = cache.lock() {
        c.insert(family.to_owned(), found);
    }
    found
}

/// Whether GPUI loaded `family` itself rather than falling back to its default list. The font
/// that `resolve_font` gave is compared with the one for a family that cannot exist; if they are
/// the same, it is a fallback, unless the family is that font (`get_font_for_id` reports the
/// family that the id was made for). The names of the OS are not compared, so a localized name or
/// a different case does not make an installed font look missing.
#[cfg(not(test))]
pub(crate) fn is_loadable(system: &gpui::TextSystem, family: &str) -> bool {
    let id = system.resolve_font(&gpui::font(family.to_owned()));
    let fallback = system.resolve_font(&gpui::font(IMPOSSIBLE_FAMILY));
    if id != fallback {
        return true;
    }
    system
        .get_font_for_id(id)
        .is_some_and(|font| font.family.as_ref() == family)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn only<'a>(names: &'a [&'a str]) -> impl FnMut(&str) -> bool + 'a {
        move |family| names.contains(&family)
    }

    #[test]
    fn mac_uses_the_family_asked_for_when_it_is_installed() {
        let font = choose(Os::MacOs, Some("Osaka"), only(&["Osaka"]));
        assert_eq!(font.family.as_ref(), "Osaka");
        assert_eq!(font.fallbacks, None);
        assert_eq!(font.reason, Reason::AsAsked);
    }

    #[test]
    fn mac_uses_the_default_family_when_the_one_asked_for_is_missing() {
        let font = choose(Os::MacOs, Some("Osaka"), only(&[FONT_FAMILY]));
        assert_eq!(font.family.as_ref(), FONT_FAMILY);
        assert_eq!(font.fallbacks, None);
        assert_eq!(
            font.reason,
            Reason::DefaultFamily {
                missing: "Osaka".into()
            }
        );
    }

    #[test]
    fn mac_uses_menlo_and_hiragino_when_neither_is_installed() {
        let font = choose(Os::MacOs, Some("Osaka"), only(&[]));
        assert_eq!(font.family.as_ref(), "Menlo");
        assert_eq!(font.fallbacks, Some(vec!["Hiragino Sans".to_owned()]));
        assert!(matches!(font.reason, Reason::SystemFonts { .. }));
    }

    #[test]
    fn mac_without_a_family_asked_for_is_the_default_family() {
        assert_eq!(
            choose(Os::MacOs, None, only(&[FONT_FAMILY]))
                .family
                .as_ref(),
            FONT_FAMILY
        );
        let font = choose(Os::MacOs, None, only(&[]));
        assert_eq!(font.family.as_ref(), "Menlo");
        assert_eq!(font.fallbacks, Some(vec!["Hiragino Sans".to_owned()]));
    }

    #[test]
    fn mac_does_not_ask_twice_for_the_default_family() {
        let mut asked = Vec::new();
        let font = choose(Os::MacOs, Some(FONT_FAMILY), |family| {
            asked.push(family.to_owned());
            false
        });
        assert_eq!(font.family.as_ref(), "Menlo");
        // the default family once, then Menlo of the list
        assert_eq!(asked, [FONT_FAMILY, "Menlo"]);
    }

    #[test]
    fn linux_keeps_the_family_asked_for() {
        let font = choose(Os::Linux, Some("Nothing"), |_| panic!("not asked on Linux"));
        assert_eq!(font.family.as_ref(), "Nothing");
        assert_eq!(font.fallbacks, None);
        assert_eq!(font.reason, Reason::AsAsked);
        let font = choose(Os::Linux, None, |_| panic!("not asked on Linux"));
        assert_eq!(font.family.as_ref(), FONT_FAMILY);
    }

    #[test]
    fn windows_keeps_a_family_that_is_installed() {
        let font = choose(Os::Windows, Some("Yu Gothic UI"), only(&["Yu Gothic UI"]));
        assert_eq!(font.family.as_ref(), "Yu Gothic UI");
        assert_eq!(font.reason, Reason::AsAsked);
    }

    #[test]
    fn windows_goes_to_a_monospaced_font_when_the_family_is_missing() {
        // The runner of GitHub has no BIZ UDGothic: DirectWrite would fall back to a proportional
        // face, so the monospaced faces of the shell tab come next.
        let font = choose(Os::Windows, None, only(&["Consolas", "Yu Gothic"]));
        assert_eq!(font.family.as_ref(), "Consolas");
        assert_eq!(
            font.fallbacks,
            Some(vec!["Yu Gothic".to_owned(), "Meiryo".to_owned()])
        );
        assert!(matches!(font.reason, Reason::SystemFonts { .. }));
        // MS Gothic comes before Consolas
        let font = choose(
            Os::Windows,
            Some("Nothing"),
            only(&["ＭＳ ゴシック", "Consolas"]),
        );
        assert_eq!(font.family.as_ref(), "ＭＳ ゴシック");
        // the default family comes before the list
        let font = choose(
            Os::Windows,
            Some("Nothing"),
            only(&[FONT_FAMILY, "Consolas"]),
        );
        assert_eq!(font.family.as_ref(), FONT_FAMILY);
        assert!(matches!(font.reason, Reason::DefaultFamily { .. }));
        // nothing of the list: what GPUI makes of the family asked for
        let font = choose(Os::Windows, None, only(&[]));
        assert_eq!(font.family.as_ref(), FONT_FAMILY);
        assert_eq!(font.reason, Reason::AsAsked);
    }

    #[test]
    fn the_gpui_font_carries_the_fallbacks_and_the_bold_font_keeps_them() {
        let font = choose(Os::MacOs, None, only(&[])).font();
        assert_eq!(font.family.as_ref(), "Menlo");
        let bold = Font {
            weight: gpui::FontWeight::BOLD,
            ..font
        };
        assert_eq!(
            bold.fallbacks.as_ref().map(|f| f.fallback_list().to_vec()),
            Some(vec!["Hiragino Sans".to_owned()])
        );
    }

    #[test]
    fn an_element_gets_the_family_and_the_fallbacks_but_not_the_weight() {
        let font = choose(Os::MacOs, None, only(&[]));
        let mut element = font.apply(gpui::div().font_weight(gpui::FontWeight::BOLD));
        let style = element.text_style().clone();
        assert_eq!(style.font_family.as_deref(), Some("Menlo"));
        assert_eq!(
            style.font_fallbacks.map(|f| f.fallback_list().to_vec()),
            Some(vec!["Hiragino Sans".to_owned()])
        );
        assert_eq!(style.font_weight, Some(gpui::FontWeight::BOLD));
    }

    #[test]
    fn equal_advances_are_monospaced_and_different_ones_are_not() {
        assert!(advances_are_monospaced(&[9.6, 9.6, 9.6, 9.6, 9.6]));
        assert!(advances_are_monospaced(&[9.6, 9.62, 9.6]));
        // A proportional font: `i` is narrow, `m` and `W` are wide.
        assert!(!advances_are_monospaced(&[4.0, 14.0, 15.0, 9.0, 4.5]));
        assert!(advances_are_monospaced(&[]));
        assert!(advances_are_monospaced(&[9.6]));
    }

    fn editor(family: &str) -> TextFont {
        TextFont {
            family: family.to_owned().into(),
            fallbacks: None,
            reason: Reason::AsAsked,
        }
    }

    #[test]
    fn the_shell_keeps_a_monospaced_editor_font() {
        let font = editor("Noto Sans Mono CJK JP");
        let got = for_shell_with(Os::Linux, font.clone(), |_| true, |_| panic!("not asked"));
        assert_eq!(got, font);
    }

    #[test]
    fn the_shell_uses_the_first_installed_monospaced_font_for_a_proportional_one() {
        let font = editor("Noto Sans JP");
        let got = for_shell_with(
            Os::Windows,
            font.clone(),
            |_| false,
            only(&["ＭＳ ゴシック", "Consolas"]),
        );
        assert_eq!(got.family.as_ref(), "ＭＳ ゴシック");
        assert!(matches!(got.reason, Reason::ShellMonospaced { .. }));
        let got = for_shell_with(Os::Windows, font.clone(), |_| false, only(&["Consolas"]));
        assert_eq!(got.family.as_ref(), "Consolas");
        assert_eq!(
            got.fallbacks,
            Some(vec!["Yu Gothic".to_owned(), "Meiryo".to_owned()])
        );
        let got = for_shell_with(Os::MacOs, font.clone(), |_| false, |_| true);
        assert_eq!(got.family.as_ref(), "Menlo");
        assert_eq!(got.fallbacks, Some(vec!["Hiragino Sans".to_owned()]));
        let got = for_shell_with(Os::Linux, font, |_| false, only(&["DejaVu Sans Mono"]));
        assert_eq!(got.family.as_ref(), "DejaVu Sans Mono");
        assert!(got.describe().contains("not monospaced"));
    }

    #[test]
    fn the_shell_keeps_the_editor_font_when_no_monospaced_font_is_installed() {
        let font = editor("Noto Sans JP");
        let got = for_shell_with(Os::Linux, font, |_| false, only(&[]));
        assert_eq!(got.family.as_ref(), "Noto Sans JP");
        assert!(matches!(got.reason, Reason::ShellNoMonospaced { .. }));
    }

    #[test]
    fn choosing_the_shell_font_leaves_the_editor_font_as_it_was() {
        let font = editor("Noto Sans JP");
        let before = font.clone();
        let _ = for_shell_with(Os::Linux, font.clone(), |_| false, |_| true);
        assert_eq!(font, before);
    }
}
