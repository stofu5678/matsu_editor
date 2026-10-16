//! What the macro functions need of the window besides the document: the state that outlives one
//! call (cookies, the draw switch), what the window reports for one call (the size of the view,
//! the encoding), and what a call asks the window to change afterwards (the tab width, the wrap
//! column).
//!
//! The window fills a [`HostEnv`] before it runs a call and reads the `requests` after it. The
//! macro host that runs without a window passes [`HostEnv::detached`]: the functions that need
//! the window answer [`crate::bridge::UNAVAILABLE`].

use std::collections::BTreeMap;

use ume_commands::context::CommandContext;

/// Sakura's `CCookieManager` (`sakura_core/macro/CCookieManager.h`): two maps of strings. A
/// cookie lives as long as the window; the macro that wrote it may be gone.
///
/// The two scopes are `"window"` and `"document"`. Sakura keeps both in the `CEditDoc` of the
/// window, and a window has one document, so they are two names for two separate maps that
/// have the same life. Here too.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Cookies {
    window: BTreeMap<String, String>,
    document: BTreeMap<String, String>,
}

/// The ways a cookie call can fail, as Sakura numbers them (the result of `SetCookie` and
/// `DeleteCookie`).
pub mod cookie_result {
    pub const OK: i64 = 0;
    /// The scope is not `"window"` or `"document"`.
    pub const BAD_SCOPE: i64 = 1;
    /// The name has a character that a name cannot have.
    pub const BAD_NAME: i64 = 2;
    /// `DeleteCookie`: there is no such cookie.
    pub const NOT_FOUND: i64 = 5;
}

impl Cookies {
    fn scope(&self, scope: &str) -> Option<&BTreeMap<String, String>> {
        match scope {
            "window" => Some(&self.window),
            "document" => Some(&self.document),
            _ => None,
        }
    }

    fn scope_mut(&mut self, scope: &str) -> Option<&mut BTreeMap<String, String>> {
        match scope {
            "window" => Some(&mut self.window),
            "document" => Some(&mut self.document),
            _ => None,
        }
    }

    /// `CCookieManager::GetCookie`: the value, or an empty string for an unknown scope or name.
    pub fn get(&self, scope: &str, name: &str) -> String {
        self.scope(scope)
            .and_then(|map| map.get(name))
            .cloned()
            .unwrap_or_default()
    }

    /// `CCookieManager::GetCookieDefault`: like [`get`](Self::get), but `default` for a name that
    /// is not there. An unknown scope still gives an empty string (as in Sakura).
    pub fn get_default(&self, scope: &str, name: &str, default: &str) -> String {
        match self.scope(scope) {
            None => String::new(),
            Some(map) => map.get(name).cloned().unwrap_or_else(|| default.to_owned()),
        }
    }

    /// `CCookieManager::SetCookie`.
    pub fn set(&mut self, scope: &str, name: &str, value: &str) -> i64 {
        let Some(map) = self.scope_mut(scope) else {
            return cookie_result::BAD_SCOPE;
        };
        if !valid_name(name) {
            return cookie_result::BAD_NAME;
        }
        map.insert(name.to_owned(), value.to_owned());
        cookie_result::OK
    }

    /// `CCookieManager::DeleteCookie`.
    pub fn delete(&mut self, scope: &str, name: &str) -> i64 {
        let Some(map) = self.scope_mut(scope) else {
            return cookie_result::BAD_SCOPE;
        };
        if !valid_name(name) {
            return cookie_result::BAD_NAME;
        }
        if map.remove(name).is_none() {
            return cookie_result::NOT_FOUND;
        }
        cookie_result::OK
    }

    /// `CCookieManager::GetCookieNames`: the names joined with commas, in the order of
    /// `std::map<std::wstring, ...>`, which compares UTF-16 code units. An unknown scope gives an
    /// empty string.
    pub fn names(&self, scope: &str) -> String {
        let Some(map) = self.scope(scope) else {
            return String::new();
        };
        let mut names: Vec<&String> = map.keys().collect();
        names.sort_by_key(|name| name.encode_utf16().collect::<Vec<u16>>());
        names
            .iter()
            .map(|name| name.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// `CCookieManager::ValidateCookieName`. The test in Sakura is
/// `'0'..='9' || 'a'..='z' || 'A'..='Z' || '_' <= c`, and the last term lets in every character
/// from `_` up (`` ` ``, `{`, `|`, `}`, `~` and everything that is not ASCII). That is kept: a
/// macro that works in Sakura works here. The empty name is valid.
fn valid_name(name: &str) -> bool {
    name.encode_utf16()
        .all(|unit| matches!(unit, 0x30..=0x39 | 0x41..=0x5A) || unit >= 0x5F)
}

/// What a window keeps for the macros between two calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostState {
    pub cookies: Cookies,
    /// `GetDrawSwitch`/`SetDrawSwitch` (`CEditView::m_bDrawSwitch`, on at the start).
    pub draw_switch: bool,
}

impl Default for HostState {
    fn default() -> Self {
        Self {
            cookies: Cookies::default(),
            draw_switch: true,
        }
    }
}

/// The size and the position of the text area, in the units of Sakura's view
/// (`CTextArea::m_nViewRowNum`, `m_nViewColNum`, `GetViewTopLine`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewInfo {
    /// How many rows the text area shows, at least 1.
    pub rows: i64,
    /// How many columns the text area shows, at least 1.
    pub columns: i64,
    /// The row at the top of the text area, counted from 0.
    pub top_row: i64,
}

/// What a call asks the window to change once it has returned.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Requests {
    /// `ChangeTabWidth`: the new tab width (this view only).
    pub tab_width: Option<u32>,
    /// `ChangeWrapColumn`: wrap at this column (this view only).
    pub wrap_columns: Option<u32>,
}

/// The decision of a document type for an extension (`CDocTypeManager::GetDocumentTypeOfExt`):
/// the index of the type.
pub type TypeOfExt = Box<dyn Fn(&str) -> usize>;

/// Everything in this file for one call. See the [module documentation](self).
pub struct HostEnv {
    pub state: HostState,
    /// `GetCharCode`: Sakura's code of the encoding of the file (`ECodeType`: 0 is Shift_JIS, 4
    /// is UTF-8, ...). `None` when the host has no file encoding.
    pub char_code: Option<i64>,
    /// The text area, or `None` when there is no window.
    pub view: Option<ViewInfo>,
    /// `IsShownStatus`.
    pub status_bar_shown: bool,
    /// The wrap column in use (`CLayoutMgr::GetMaxLineKetas`), or `None` without a window.
    pub wrap_columns: Option<u32>,
    /// A key macro is being recorded in this window (`$M?` of `ExpandParameter`).
    pub recording_key_macro: bool,
    /// The full path of the macro file that runs (`$M` of `ExpandParameter`).
    pub macro_path: Option<String>,
    /// The index of the document type of the file in the window.
    pub current_type: usize,
    /// The document type of an extension, if the window has the types.
    pub type_of_ext: Option<TypeOfExt>,
    pub requests: Requests,
}

impl HostEnv {
    /// An environment without a window: no view, no file encoding, no types.
    pub fn detached() -> Self {
        Self {
            state: HostState::default(),
            char_code: None,
            view: None,
            status_bar_shown: false,
            wrap_columns: None,
            recording_key_macro: false,
            macro_path: None,
            current_type: 0,
            type_of_ext: None,
            requests: Requests::default(),
        }
    }

    /// The type of the files with extension `ext` (the argument of `IsCurTypeExt`), by the
    /// extension lists of the types if the window has them; otherwise by the extension alone: the
    /// type 0, the basic type, for every extension.
    pub fn type_of(&self, ext: &str) -> usize {
        self.type_of_ext.as_ref().map_or(0, |f| f(ext))
    }
}

impl Default for HostEnv {
    fn default() -> Self {
        Self::detached()
    }
}

/// What a function that needs the editor gets: the command context (the document, the cursor, the
/// layout, the clipboard) and the environment.
pub struct Editor<'a, 'b> {
    pub cx: &'a mut CommandContext<'b>,
    pub env: &'a mut HostEnv,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_scopes_are_separate_and_unknown_scopes_fail() {
        let mut c = Cookies::default();
        assert_eq!(c.set("window", "a", "1"), 0);
        assert_eq!(c.set("document", "a", "2"), 0);
        assert_eq!(c.get("window", "a"), "1");
        assert_eq!(c.get("document", "a"), "2");
        assert_eq!(c.set("buffer", "a", "3"), cookie_result::BAD_SCOPE);
        assert_eq!(c.get("buffer", "a"), "");
        assert_eq!(c.get_default("buffer", "a", "d"), "");
        assert_eq!(c.get_default("window", "zz", "d"), "d");
        assert_eq!(c.delete("buffer", "a"), cookie_result::BAD_SCOPE);
    }

    #[test]
    fn cookie_names_keep_sakuras_odd_rule() {
        let mut c = Cookies::default();
        // Digits, letters and `_`, and everything from `_` up.
        for name in ["a1", "_x", "日本語", "x`y", "x~"] {
            assert_eq!(c.set("window", name, "v"), 0, "{name}");
        }
        // Punctuation below `_` is refused (`-`, space, `.`, `:`, `@`, `[`, `^`).
        for name in ["a-b", "a b", "a.b", "a:b", "a@b", "a[b", "a^b"] {
            assert_eq!(
                c.set("window", name, "v"),
                cookie_result::BAD_NAME,
                "{name}"
            );
            assert_eq!(c.delete("window", name), cookie_result::BAD_NAME, "{name}");
        }
        // The empty name is valid.
        assert_eq!(c.set("window", "", "e"), 0);
        assert_eq!(c.get("window", ""), "e");
    }

    #[test]
    fn deleting_a_missing_cookie_is_5_and_names_sort_by_utf16_units() {
        let mut c = Cookies::default();
        assert_eq!(c.delete("window", "nope"), cookie_result::NOT_FOUND);
        c.set("window", "b", "");
        c.set("window", "a", "");
        c.set("window", "B", "");
        // U+1F600 is the pair D83D DE00 in UTF-16, so it sorts before U+FF21, unlike by code
        // point or in UTF-8.
        c.set("window", "\u{1F600}", "");
        c.set("window", "\u{FF21}", "");
        assert_eq!(c.names("window"), "B,a,b,\u{1F600},\u{FF21}");
        assert_eq!(c.names("document"), "");
        assert_eq!(c.names("nope"), "");
        assert_eq!(c.delete("window", "a"), 0);
        assert_eq!(c.get("window", "a"), "");
    }
}
