//! The long form of a path (`GetLongFileName` in `sakura_core/util/file.cpp`).
//!
//! * Windows: `GetFullPathNameW` (a relative path becomes absolute) and then `GetLongPathNameW`
//!   (a short `PROGRA~1` name becomes the long one, and the letters get the case the disk has).
//!   If the path does not exist, the full path is returned as it is, as Sakura does.
//! * macOS and Linux: there are no short names, so the path is left as it is (`None`).

#[cfg(windows)]
mod windows;

/// The long form of `path`, or `None` when there is nothing to change or the OS cannot tell
/// (`GetLongFileName` returned `FALSE`, or this OS has no long and short names). The caller keeps
/// the path it has then.
pub fn long_path(path: &str) -> Option<String> {
    #[cfg(windows)]
    {
        windows::long_path(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    #[test]
    fn a_path_is_left_as_it_is_where_there_are_no_short_names() {
        assert_eq!(long_path("/tmp"), None);
    }
}
