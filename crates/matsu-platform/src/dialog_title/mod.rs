//! The title of the confirmation boxes (確認の箱: `window.prompt` of GPUI), as Sakura's.
//!
//! Sakura gives its message boxes the name of the application as the title (`MessageBoxS` in
//! `sakura_core/util/MessageBoxF.cpp`: `optCaption.value_or(GSTR_APPNAME)`, and `GSTR_APPNAME` is
//! `STR_GSTR_APPNAME`, "サクラエディタ" and "(64bit)" for a 64 bit build, `sakura_core/sakura_rc.rc2`);
//! the icon says what kind of box it is. GPUI's box on Windows (`gpui-pre-windows`, `prompt` of
//! `window.rs`) is a TaskDialog whose title is the English word of its level: "Info", "Warning" or
//! "Critical". GPUI cannot be changed from here, so on Windows the title of such a box is replaced when
//! the box is shown (the module `windows`): a window of the dialog class of this process whose title is one of those
//! three words gets [`app_title`].
//!
//! macOS (`NSAlert`) and Linux (GPUI's own box) have no title bar text for these boxes, so nothing changes
//! there and [`install`] does nothing.
//!
//! `unsafe` is in `windows` only.

#[cfg(windows)]
mod windows;

/// The words that GPUI puts in the title of its boxes on Windows (`PromptLevel::Info`, `Warning` and
/// `Critical`).
pub const GPUI_TITLES: [&str; 3] = ["Info", "Warning", "Critical"];

/// The class of the window of a message box and of a TaskDialog (`#32770`).
pub const DIALOG_CLASS: &str = "#32770";

/// The title of Sakura's message boxes for MATSU Editor: the name of the application in katakana (the
/// rule of the names in `CLAUDE.md`) and the bit width of the build, as Sakura's `_GSTR_APPNAME` does
/// (`サクラエディタ(64bit)`).
pub fn app_title() -> String {
    let bits = if cfg!(target_pointer_width = "64") {
        "64bit"
    } else {
        "32bit"
    };
    format!("マツエディタ({bits})")
}

/// The title that a window should get, or `None` to leave it as it is: only a window of the dialog
/// class with one of GPUI's words as its title is a box of GPUI (a title of a window of ours or of
/// a box that the user can see with its own title is never changed).
pub fn replacement(class: &str, title: &str, new_title: &str) -> Option<String> {
    (class == DIALOG_CLASS && GPUI_TITLES.contains(&title) && title != new_title)
        .then(|| new_title.to_owned())
}

/// Starts replacing the titles of the boxes of this process (Windows; nothing on the other systems). Call it
/// once, on the thread that runs the message loop of the application. A second call does nothing.
pub fn install() {
    #[cfg(windows)]
    windows::install(app_title());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_boxes_of_gpui_get_the_title() {
        let new = app_title();
        for word in GPUI_TITLES {
            assert_eq!(replacement(DIALOG_CLASS, word, &new), Some(new.clone()));
        }
        // another class (a window of ours that is called "Info"), another title, or the title already set
        assert_eq!(replacement("GPUI::Window", "Info", &new), None);
        assert_eq!(replacement(DIALOG_CLASS, "info", &new), None);
        assert_eq!(replacement(DIALOG_CLASS, "ファイルを開く", &new), None);
        assert_eq!(replacement(DIALOG_CLASS, &new, &new), None);
    }

    #[test]
    fn the_title_is_the_name_in_katakana_and_the_bit_width() {
        let title = app_title();
        assert!(title.starts_with("マツエディタ("), "{title}");
        assert!(title.ends_with("bit)"), "{title}");
        // Sakura's name is not kept
        assert!(!title.contains("サクラ"));
    }
}
