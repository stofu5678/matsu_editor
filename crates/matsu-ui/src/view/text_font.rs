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
                format!("{fonts} ({missing:?} is not installed: Menlo with Hiragino Sans)")
            }
        }
    }
}

/// Chooses the font for the family asked for (`None`: the default, [`FONT_FAMILY`]).
/// `installed` tells whether GPUI can load a family; it is not called off macOS.
pub(crate) fn choose(
    os: Os,
    asked: Option<&str>,
    mut installed: impl FnMut(&str) -> bool,
) -> TextFont {
    let asked = asked.unwrap_or(FONT_FAMILY);
    if os != Os::MacOs || installed(asked) {
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
    TextFont {
        family: MAC_FALLBACK_FAMILY.into(),
        fallbacks: Some(vec![MAC_FALLBACK_JAPANESE.to_owned()]),
        reason: Reason::SystemFonts { missing },
    }
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
        assert_eq!(asked, [FONT_FAMILY]);
    }

    #[test]
    fn windows_and_linux_keep_the_family_asked_for() {
        for os in [Os::Windows, Os::Linux] {
            let font = choose(os, Some("Nothing"), |_| panic!("not asked off macOS"));
            assert_eq!(font.family.as_ref(), "Nothing");
            assert_eq!(font.fallbacks, None);
            assert_eq!(font.reason, Reason::AsAsked);
            let font = choose(os, None, |_| panic!("not asked off macOS"));
            assert_eq!(font.family.as_ref(), FONT_FAMILY);
        }
    }

    #[test]
    fn the_gpui_font_carries_the_fallbacks_and_the_bold_font_keeps_them() {
        let font = choose(Os::MacOs, None, only(&[])).font();
        assert_eq!(font.family.as_ref(), "Menlo");
        let bold = Font {
            weight: gpui::FontWeight::BOLD,
            ..font.clone()
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
}
