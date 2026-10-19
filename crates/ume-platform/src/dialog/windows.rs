//! The file dialogs of Windows: `IFileOpenDialog` and `IFileSaveDialog` with `SetFolder`, as
//! Sakura shows them (`CDlgOpenFile_CommonItemDialog.cpp`: `FOS_NOCHANGEDIR | FOS_FORCEFILESYSTEM |
//! FOS_FILEMUSTEXIST`, `SetFileName`, `SetFolder`, `Show(hwndParent)`). Not run by CI, which has
//! no desktop; the user checks it.
//!
//! The dialog is modal to the window and runs on a thread of its own, so that the UI thread of the
//! application keeps running (GPUI draws the window while the dialog is open).

// COM is `unsafe` (ADR-0013).
#![allow(unsafe_code)]

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use futures_channel::oneshot;
use raw_window_handle::RawWindowHandle;
use windows::Win32::Foundation::{ERROR_CANCELLED, HWND};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
    CoUninitialize,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FOS_ALLOWMULTISELECT, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR,
    FOS_OVERWRITEPROMPT, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, FileOpenDialog, FileSaveDialog,
    IFileDialog, IFileOpenDialog, IFileSaveDialog, IShellItem, SHCreateItemFromParsingName,
    SIGDN_FILESYSPATH,
};
use windows::core::{HRESULT, HSTRING, PCWSTR};

use super::{Error, Kind, Request};

/// The window the dialog belongs to: a number rather than a handle, so that it can go to the
/// thread that shows the dialog. 0 (no owner) for a handle that is not a Win32 window.
#[derive(Clone, Copy, Debug, Default)]
pub struct Owner(isize);

impl Owner {
    pub fn from_handle(handle: RawWindowHandle) -> Self {
        match handle {
            RawWindowHandle::Win32(handle) => Self(handle.hwnd.get()),
            _ => Self(0),
        }
    }
}

pub(super) async fn show(owner: Owner, request: Request) -> Result<Option<Vec<PathBuf>>, Error> {
    let (done, answer) = oneshot::channel();
    std::thread::Builder::new()
        .name("file dialog".to_owned())
        .spawn(move || {
            done.send(run(owner, &request)).ok();
        })
        .map_err(|e| Error::Failed(e.to_string()))?;
    answer
        .await
        .map_err(|_| Error::Failed("the dialog thread ended".to_owned()))?
}

/// Initializes COM for the thread while it shows the dialog.
struct Apartment;

impl Apartment {
    fn enter() -> Result<Self, Error> {
        // SAFETY: balanced by `CoUninitialize` in `drop`; the thread has no other COM use.
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
            .ok()
            .map_err(failed)?;
        Ok(Self)
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: pairs the successful `CoInitializeEx` of `enter`.
        unsafe { CoUninitialize() };
    }
}

fn failed(error: windows::core::Error) -> Error {
    Error::Failed(error.to_string())
}

fn run(owner: Owner, request: &Request) -> Result<Option<Vec<PathBuf>>, Error> {
    let _apartment = Apartment::enter()?;
    // SAFETY: the dialog objects are created and used on this thread only, inside the apartment
    // above; every pointer handed to COM is a live `HSTRING` or a value on the stack.
    unsafe {
        let hwnd = (owner.0 != 0).then_some(HWND(owner.0 as *mut _));
        let (dialog, open): (IFileDialog, Option<IFileOpenDialog>) = match request.kind {
            Kind::SaveFile => {
                let dialog: IFileSaveDialog =
                    CoCreateInstance(&FileSaveDialog, None, CLSCTX_ALL).map_err(failed)?;
                (
                    windows::core::Interface::cast(&dialog).map_err(failed)?,
                    None,
                )
            }
            Kind::OpenFile { .. } | Kind::Folder => {
                let dialog: IFileOpenDialog =
                    CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL).map_err(failed)?;
                (
                    windows::core::Interface::cast(&dialog).map_err(failed)?,
                    Some(dialog),
                )
            }
        };
        let mut options =
            dialog.GetOptions().map_err(failed)? | FOS_NOCHANGEDIR | FOS_FORCEFILESYSTEM;
        match request.kind {
            Kind::OpenFile { multiple } => {
                options |= FOS_FILEMUSTEXIST;
                if multiple {
                    options |= FOS_ALLOWMULTISELECT;
                }
            }
            Kind::Folder => options |= FOS_PICKFOLDERS | FOS_PATHMUSTEXIST,
            Kind::SaveFile => options |= FOS_OVERWRITEPROMPT | FOS_PATHMUSTEXIST,
        }
        dialog.SetOptions(options).map_err(failed)?;
        dialog
            .SetTitle(&HSTRING::from(request.title.as_str()))
            .map_err(failed)?;
        // The list of file types of the save dialog (`DoModalSaveDlgImpl1`: `SetDefaultExtension`,
        // `SetFileTypes`). The strings live until the dialog is closed.
        let names: Vec<HSTRING> = request
            .filters
            .iter()
            .map(|filter| HSTRING::from(filter.name.as_str()))
            .collect();
        let specs: Vec<HSTRING> = request
            .filters
            .iter()
            .map(|filter| HSTRING::from(filter.patterns.join(";")))
            .collect();
        if request.kind == Kind::SaveFile {
            if let Some(extension) = &request.default_extension {
                dialog
                    .SetDefaultExtension(&HSTRING::from(extension.as_str()))
                    .map_err(failed)?;
            }
            if !names.is_empty() {
                let types: Vec<COMDLG_FILTERSPEC> = names
                    .iter()
                    .zip(&specs)
                    .map(|(name, spec)| COMDLG_FILTERSPEC {
                        pszName: PCWSTR(name.as_ptr()),
                        pszSpec: PCWSTR(spec.as_ptr()),
                    })
                    .collect();
                dialog.SetFileTypes(&types).map_err(failed)?;
                // the index is 1-based
                let selected = request.selected_filter.min(types.len() - 1) + 1;
                dialog.SetFileTypeIndex(selected as u32).map_err(failed)?;
            }
        }
        if let Some(name) = &request.name {
            dialog
                .SetFileName(&HSTRING::from(name.as_str()))
                .map_err(failed)?;
        }
        if let Some(start) = &request.start {
            // a folder that cannot be made into an item is not the dialog's business
            if let Ok(folder) = SHCreateItemFromParsingName::<_, _, IShellItem>(
                &HSTRING::from(start.as_os_str()),
                None,
            ) {
                dialog.SetFolder(&folder).map_err(failed)?;
            }
        }
        if let Err(error) = dialog.Show(hwnd) {
            return if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
                Ok(None)
            } else {
                Err(failed(error))
            };
        }
        let mut paths = Vec::new();
        match open {
            Some(open) => {
                let items = open.GetResults().map_err(failed)?;
                for i in 0..items.GetCount().map_err(failed)? {
                    paths.push(path_of(&items.GetItemAt(i).map_err(failed)?)?);
                }
            }
            None => paths.push(path_of(&dialog.GetResult().map_err(failed)?)?),
        }
        Ok((!paths.is_empty()).then_some(paths))
    }
}

/// The file system path of an item (`SIGDN_FILESYSPATH`).
///
/// # Safety
///
/// Called from a thread in a COM apartment, with a live item.
unsafe fn path_of(item: &IShellItem) -> Result<PathBuf, Error> {
    // SAFETY: the string belongs to us after `GetDisplayName` and is freed once, after the copy.
    unsafe {
        let name = item.GetDisplayName(SIGDN_FILESYSPATH).map_err(failed)?;
        let path = PathBuf::from(OsString::from_wide(name.as_wide()));
        CoTaskMemFree(Some(name.0.cast_const().cast()));
        Ok(path)
    }
}
