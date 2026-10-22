//! The known folders of Windows by the names of the registry key `Shell Folders`, which Sakura
//! reads (`CFileNameManager::ExpandMetaToFolder`). Not run by CI; the user checks it.

// `SHGetKnownFolderPath` is `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Documents, FOLDERID_Favorites, FOLDERID_LocalAppData,
    FOLDERID_Music, FOLDERID_Pictures, FOLDERID_PublicDesktop, FOLDERID_PublicDocuments,
    FOLDERID_PublicMusic, FOLDERID_PublicPictures, FOLDERID_PublicVideos, FOLDERID_RoamingAppData,
    FOLDERID_Videos, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
};
use windows::core::GUID;

fn folder_id(name: &str) -> Option<GUID> {
    let table = [
        ("Personal", FOLDERID_Documents),
        ("Desktop", FOLDERID_Desktop),
        ("My Pictures", FOLDERID_Pictures),
        ("My Music", FOLDERID_Music),
        ("Video", FOLDERID_Videos),
        ("My Video", FOLDERID_Videos),
        ("Favorites", FOLDERID_Favorites),
        ("AppData", FOLDERID_RoamingAppData),
        ("Local AppData", FOLDERID_LocalAppData),
        ("Common Desktop", FOLDERID_PublicDesktop),
        ("Common Documents", FOLDERID_PublicDocuments),
        ("CommonPictures", FOLDERID_PublicPictures),
        ("CommonMusic", FOLDERID_PublicMusic),
        ("CommonVideo", FOLDERID_PublicVideos),
    ];
    table
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, id)| *id)
}

pub(super) fn known_folder(name: &str) -> Option<PathBuf> {
    let id = folder_id(name)?;
    // SAFETY: `id` outlives the call; on success the shell returns a NUL-terminated string that
    // is copied and then freed with `CoTaskMemFree`, as the documentation of the function says.
    unsafe {
        let path = SHGetKnownFolderPath(&id, KF_FLAG_DEFAULT, None).ok()?;
        let folder = OsString::from_wide(path.as_wide());
        CoTaskMemFree(Some(path.0.cast_const().cast()));
        Some(PathBuf::from(folder))
    }
}
