//! The width of the caret that the OS is set to (`SPI_GETCARETWIDTH`).
//!
//! Sakura's Windows style caret in insert mode is 2 px wide, or as wide as the system setting when
//! that is more than 2 px (`CCaret::ShowEditCaret`, `sakura_core/view/CCaret.cpp`, 2011.12.22:
//! "システムの設定に従う(けど2px以上)"). Only Windows has the setting; the other systems answer
//! `None` and the caret keeps its 2 px.
//!
//! `unsafe` is in `windows` only.

#[cfg(windows)]
mod windows;

/// The caret width of the OS in pixels, if the OS has one.
pub fn os_caret_width() -> Option<u32> {
    #[cfg(windows)]
    {
        windows::caret_width()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_width_is_asked_without_failing() {
        // Windows answers a width (1 px at least); the other systems have none.
        let width = super::os_caret_width();
        if cfg!(windows) {
            assert!(width.is_none_or(|w| w >= 1));
        } else {
            assert_eq!(width, None);
        }
    }
}
