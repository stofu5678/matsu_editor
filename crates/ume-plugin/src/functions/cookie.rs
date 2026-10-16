//! Cookies: strings that a macro leaves for the next macro of the same window.
//!
//! `sakura_core/macro/CCookieManager.cpp` is the source; the maps are [`crate::host::Cookies`].

use serde_json::Value;

use super::EditorFunc;
use crate::api::{Args, Ty, req};
use crate::host::Editor;
use crate::protocol::RpcError;

type Result<T = Value> = std::result::Result<T, RpcError>;

/// F_GETCOOKIE.
fn get_cookie(ed: &mut Editor, args: &Args) -> Result {
    Ok(ed.env.state.cookies.get(args.str(0), args.str(1)).into())
}

/// F_GETCOOKIEDEFAULT.
fn get_cookie_default(ed: &mut Editor, args: &Args) -> Result {
    Ok(ed
        .env
        .state
        .cookies
        .get_default(args.str(0), args.str(1), args.str(2))
        .into())
}

/// F_SETCOOKIE.
fn set_cookie(ed: &mut Editor, args: &Args) -> Result {
    Ok(ed
        .env
        .state
        .cookies
        .set(args.str(0), args.str(1), args.str(2))
        .into())
}

/// F_DELETECOOKIE.
fn delete_cookie(ed: &mut Editor, args: &Args) -> Result {
    Ok(ed.env.state.cookies.delete(args.str(0), args.str(1)).into())
}

/// F_GETCOOKIENAMES.
fn get_cookie_names(ed: &mut Editor, args: &Args) -> Result {
    Ok(ed.env.state.cookies.names(args.str(0)).into())
}

pub const FUNCS: &[EditorFunc] = &[
    EditorFunc {
        name: "GetCookie",
        params: &[req("scope", Ty::Str), req("name", Ty::Str)],
        result: Ty::Str,
        doc: "The value of a cookie, or an empty string when there is none. \
              The scope is \"window\" or \"document\".",
        run: get_cookie,
    },
    EditorFunc {
        name: "GetCookieDefault",
        params: &[
            req("scope", Ty::Str),
            req("name", Ty::Str),
            req("defaultValue", Ty::Str),
        ],
        result: Ty::Str,
        doc: "The value of a cookie, or the default when there is none (an unknown scope still \
              gives an empty string). The scope is \"window\" or \"document\".",
        run: get_cookie_default,
    },
    EditorFunc {
        name: "SetCookie",
        params: &[
            req("scope", Ty::Str),
            req("name", Ty::Str),
            req("value", Ty::Str),
        ],
        result: Ty::Int,
        doc: "Sets a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a name \
              with a character that a name cannot have (Sakura lets in digits, letters and \
              every character from \"_\" up in the character table). The scope is \"window\" \
              or \"document\".",
        run: set_cookie,
    },
    EditorFunc {
        name: "DeleteCookie",
        params: &[req("scope", Ty::Str), req("name", Ty::Str)],
        result: Ty::Int,
        doc: "Deletes a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a bad \
              name, 5 when there is no such cookie. The scope is \"window\" or \"document\".",
        run: delete_cookie,
    },
    EditorFunc {
        name: "GetCookieNames",
        params: &[req("scope", Ty::Str)],
        result: Ty::Str,
        doc: "The names of the cookies of a scope, joined with commas, in the order of the \
              names (UTF-16 code units). An unknown scope gives an empty string. The scope is \
              \"window\" or \"document\".",
        run: get_cookie_names,
    },
];

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::testing::Fixture;

    #[test]
    fn cookies_live_in_the_environment_and_by_scope() {
        let mut f = Fixture::new("");
        assert_eq!(f.ok("GetCookie", json!(["window", "a"])), json!(""));
        assert_eq!(
            f.ok("GetCookieDefault", json!(["window", "a", "既定"])),
            json!("既定")
        );
        assert_eq!(f.ok("SetCookie", json!(["window", "a", "値"])), json!(0));
        assert_eq!(f.ok("GetCookie", json!(["window", "a"])), json!("値"));
        assert_eq!(f.ok("GetCookie", json!(["document", "a"])), json!(""));
        assert_eq!(
            f.ok("GetCookieDefault", json!(["window", "a", "x"])),
            json!("値")
        );
        assert_eq!(f.ok("SetCookie", json!(["document", "b", ""])), json!(0));
        assert_eq!(f.ok("GetCookieNames", json!(["window"])), json!("a"));
        assert_eq!(f.ok("GetCookieNames", json!(["document"])), json!("b"));
        assert_eq!(f.ok("DeleteCookie", json!(["window", "a"])), json!(0));
        assert_eq!(f.ok("DeleteCookie", json!(["window", "a"])), json!(5));
        assert_eq!(f.ok("GetCookieNames", json!(["window"])), json!(""));
    }

    #[test]
    fn bad_scopes_and_names_answer_codes() {
        let mut f = Fixture::new("");
        assert_eq!(f.ok("SetCookie", json!(["other", "a", "x"])), json!(1));
        assert_eq!(f.ok("SetCookie", json!(["window", "a-b", "x"])), json!(2));
        assert_eq!(f.ok("DeleteCookie", json!(["other", "a"])), json!(1));
        assert_eq!(f.ok("DeleteCookie", json!(["window", "a b"])), json!(2));
        assert_eq!(f.ok("GetCookieNames", json!(["other"])), json!(""));
        // too few arguments are an error, as Sakura's `ArgSize >= 2`
        assert!(f.call("GetCookie", json!(["window"])).is_err());
    }

    #[test]
    fn the_state_outlives_a_call() {
        let mut f = Fixture::new("");
        f.ok("SetCookie", json!(["window", "keep", "1"]));
        // a second context over the same environment sees it
        assert_eq!(f.ok("GetCookie", json!(["window", "keep"])), json!("1"));
    }
}
