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
    CDCS_ENABLEDVISIBLE, CDCS_VISIBLE, FDE_OVERWRITE_RESPONSE, FDE_SHAREVIOLATION_RESPONSE,
    FDEOR_DEFAULT, FDESVR_DEFAULT, FOS_ALLOWMULTISELECT, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM,
    FOS_NOCHANGEDIR, FOS_OVERWRITEPROMPT, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS, FileOpenDialog,
    FileSaveDialog, IFileDialog, IFileDialogControlEvents, IFileDialogControlEvents_Impl,
    IFileDialogCustomize, IFileDialogEvents, IFileDialogEvents_Impl, IFileOpenDialog,
    IFileSaveDialog, IShellItem, SHCreateItemFromParsingName, SIGDN_FILESYSPATH,
};
use windows::core::{BOOL, HRESULT, HSTRING, PCWSTR, Ref, implement};

use super::{Chosen, Error, Extra, ExtraValue, Follow, Kind, RecentKind, Request};

/// The window the dialog belongs to: a number rather than a handle, so that it can go to the
/// thread that shows the dialog. 0 (no owner) for a handle that is not a Win32 window.
#[derive(Clone, Copy, Debug, Default)]
pub struct Owner(isize);

impl Owner {
    /// The window handle, 0 for none (the font dialog takes it too).
    pub(crate) fn hwnd(self) -> isize {
        self.0
    }

    pub fn from_handle(handle: RawWindowHandle) -> Self {
        match handle {
            RawWindowHandle::Win32(handle) => Self(handle.hwnd.get()),
            _ => Self(0),
        }
    }
}

pub(super) async fn show(owner: Owner, request: Request) -> Result<Option<Chosen>, Error> {
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

fn run(owner: Owner, request: &Request) -> Result<Option<Chosen>, Error> {
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
        if request.kind == Kind::SaveFile
            && let Some(extension) = &request.default_extension
        {
            dialog
                .SetDefaultExtension(&HSTRING::from(extension.as_str()))
                .map_err(failed)?;
        }
        // The open dialog has the list too (`DoModal_GetOpenFileName`); the dialog that picks a
        // folder does not.
        if request.kind != Kind::Folder && !names.is_empty() {
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
        // Sakura's `Customize` (`IFileDialogCustomize`), and the events that keep the BOM in step
        // with the encoding (`OnItemSelected`). The events are let go of when the dialog closes.
        // The open dialog has them too, for the lists of recent files and folders.
        let customize = (matches!(request.kind, Kind::SaveFile | Kind::OpenFile { .. })
            && !request.extras.is_empty())
        .then(|| add_extras(&dialog, request))
        .transpose()?;
        let recent = recent_lists(&request.extras);
        let events: Option<(IFileDialogEvents, u32)> =
            if customize.is_some() && (request.follow.is_some() || !recent.is_empty()) {
                let sink: IFileDialogEvents = Sink {
                    follow: request.follow.clone(),
                    dialog: dialog.clone(),
                    recent,
                }
                .into();
                let cookie = dialog.Advise(&sink).map_err(failed)?;
                Some((sink, cookie))
            } else {
                None
            };
        let shown = dialog.Show(hwnd);
        if let Some((_sink, cookie)) = events {
            dialog.Unadvise(cookie).ok();
        }
        if let Err(error) = shown {
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
        let extras = match &customize {
            Some(customize) => read_extras(customize, &request.extras)?,
            None => Vec::new(),
        };
        Ok((!paths.is_empty()).then_some(Chosen { paths, extras }))
    }
}

/// The id of the control for the field at `index`.
fn control_id(index: usize) -> u32 {
    1000 + index as u32
}

/// The id of the visual group of the list at `index`: every control of the dialog, a group too,
/// needs an id of its own (Sakura's `LABEL_CODE` is not `COMBO_CODE`).
fn group_id(index: usize) -> u32 {
    2000 + index as u32
}

/// Adds the fields to the save dialog as Sakura does: a visual group for each list, with the check
/// boxes that follow it inside (`CDlgOpenFile_CommonItemDialog::Customize`). Code pages are all in
/// the list: Sakura's `C&P` check box, which adds them to the list later, is left out.
///
/// # Safety
///
/// Called from a thread in a COM apartment, before the dialog is shown.
unsafe fn add_extras(
    dialog: &IFileDialog,
    request: &Request,
) -> Result<IFileDialogCustomize, Error> {
    // SAFETY: the labels are live `HSTRING`s for the calls; COM copies them.
    unsafe {
        let customize: IFileDialogCustomize =
            windows::core::Interface::cast(dialog).map_err(failed)?;
        let mut open_group = false;
        for (index, extra) in request.extras.iter().enumerate() {
            let id = control_id(index);
            match extra {
                Extra::Combo {
                    label,
                    items,
                    selected,
                } => {
                    if open_group {
                        customize.EndVisualGroup().map_err(failed)?;
                    }
                    customize
                        .StartVisualGroup(group_id(index), &HSTRING::from(label.as_str()))
                        .map_err(failed)?;
                    open_group = true;
                    customize.AddComboBox(id).map_err(failed)?;
                    for (n, item) in items.iter().enumerate() {
                        customize
                            .AddControlItem(id, n as u32, &HSTRING::from(item.as_str()))
                            .map_err(failed)?;
                    }
                    customize
                        .SetSelectedControlItem(id, *selected as u32)
                        .map_err(failed)?;
                }
                Extra::Check {
                    label,
                    checked,
                    enabled,
                } => {
                    customize
                        .AddCheckButton(id, &HSTRING::from(label.as_str()), *checked)
                        .map_err(failed)?;
                    if !enabled {
                        customize
                            .SetControlState(id, CDCS_VISIBLE)
                            .map_err(failed)?;
                    }
                }
                Extra::Recent { label, items, .. } => {
                    // `COMBO_MRU`, `COMBO_OPENFOLDER`: a group of their own, the items numbered from
                    // 1, and nothing selected.
                    if open_group {
                        customize.EndVisualGroup().map_err(failed)?;
                    }
                    customize
                        .StartVisualGroup(group_id(index), &HSTRING::from(label.as_str()))
                        .map_err(failed)?;
                    open_group = true;
                    customize.AddComboBox(id).map_err(failed)?;
                    for (n, item) in items.iter().enumerate() {
                        customize
                            .AddControlItem(id, n as u32 + 1, &HSTRING::from(item.as_str()))
                            .map_err(failed)?;
                    }
                }
            }
        }
        if open_group {
            customize.EndVisualGroup().map_err(failed)?;
        }
        Ok(customize)
    }
}

/// The value of every field, after the dialog was accepted.
///
/// # Safety
///
/// Called from a thread in a COM apartment, with the dialog's `customize`.
unsafe fn read_extras(
    customize: &IFileDialogCustomize,
    extras: &[Extra],
) -> Result<Vec<ExtraValue>, Error> {
    // SAFETY: plain calls on a live interface.
    unsafe {
        extras
            .iter()
            .enumerate()
            .map(|(index, extra)| {
                let id = control_id(index);
                Ok(match extra {
                    Extra::Combo { .. } => ExtraValue::Combo(
                        customize.GetSelectedControlItem(id).map_err(failed)? as usize,
                    ),
                    Extra::Check { .. } => ExtraValue::Check(
                        customize.GetCheckButtonState(id).map_err(failed)?.as_bool(),
                    ),
                    Extra::Recent { .. } => ExtraValue::Nothing,
                })
            })
            .collect()
    }
}

/// A list of recent paths of the dialog: the id of its control, what its items are, and the items.
struct RecentList {
    control: u32,
    kind: RecentKind,
    items: Vec<String>,
}

fn recent_lists(extras: &[Extra]) -> Vec<RecentList> {
    extras
        .iter()
        .enumerate()
        .filter_map(|(index, extra)| match extra {
            Extra::Recent { items, kind, .. } => Some(RecentList {
                control: control_id(index),
                kind: *kind,
                items: items.clone(),
            }),
            _ => None,
        })
        .collect()
}

/// The events of the dialog (`CDlgOpenFile_CommonItemDialog::OnItemSelected`): choosing an item of
/// the list changes the check box that follows it, and choosing a recent file or folder puts the
/// file in the name field, or shows the folder. Everything else is left alone.
#[implement(IFileDialogEvents, IFileDialogControlEvents)]
struct Sink {
    follow: Option<Follow>,
    /// The dialog the events come from. It lets go of this sink when the dialog is unadvised.
    dialog: IFileDialog,
    recent: Vec<RecentList>,
}

impl IFileDialogEvents_Impl for Sink_Impl {
    fn OnFileOk(&self, _: Ref<IFileDialog>) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnFolderChanging(
        &self,
        _: Ref<IFileDialog>,
        _: Ref<IShellItem>,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnFolderChange(&self, _: Ref<IFileDialog>) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnSelectionChange(&self, _: Ref<IFileDialog>) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnShareViolation(
        &self,
        _: Ref<IFileDialog>,
        _: Ref<IShellItem>,
    ) -> windows::core::Result<FDE_SHAREVIOLATION_RESPONSE> {
        Ok(FDESVR_DEFAULT)
    }
    fn OnTypeChange(&self, _: Ref<IFileDialog>) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnOverwrite(
        &self,
        _: Ref<IFileDialog>,
        _: Ref<IShellItem>,
    ) -> windows::core::Result<FDE_OVERWRITE_RESPONSE> {
        Ok(FDEOR_DEFAULT)
    }
}

impl IFileDialogControlEvents_Impl for Sink_Impl {
    fn OnItemSelected(
        &self,
        pfdc: Ref<IFileDialogCustomize>,
        control: u32,
        item: u32,
    ) -> windows::core::Result<()> {
        if let Some(list) = self.recent.iter().find(|list| list.control == control) {
            // item 0 is "nothing"; the items are numbered from 1
            if let Some(path) = item.checked_sub(1).and_then(|n| list.items.get(n as usize)) {
                // SAFETY: called by the dialog, on its thread, with the dialog's own interface.
                unsafe {
                    match list.kind {
                        RecentKind::Files => {
                            self.dialog.SetFileName(&HSTRING::from(path.as_str()))?;
                        }
                        RecentKind::Folders => {
                            // a folder that cannot be made into an item is left alone
                            if let Ok(folder) = SHCreateItemFromParsingName::<_, _, IShellItem>(
                                &HSTRING::from(path.as_str()),
                                None,
                            ) {
                                self.dialog.SetFolder(&folder)?;
                            }
                        }
                    }
                }
            }
            return Ok(());
        }
        if let Some(follow) = &self.follow
            && control == control_id(follow.combo)
            && let Some(customize) = pfdc.as_ref()
        {
            let (enabled, checked) = (follow.rule)(item as usize);
            let check = control_id(follow.check);
            // SAFETY: called by the dialog, on its thread, with its own interface.
            unsafe {
                customize.SetControlState(
                    check,
                    if enabled {
                        CDCS_ENABLEDVISIBLE
                    } else {
                        CDCS_VISIBLE
                    },
                )?;
                customize.SetCheckButtonState(check, checked)?;
            }
        }
        Ok(())
    }
    fn OnButtonClicked(&self, _: Ref<IFileDialogCustomize>, _: u32) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnCheckButtonToggled(
        &self,
        _: Ref<IFileDialogCustomize>,
        _: u32,
        _: BOOL,
    ) -> windows::core::Result<()> {
        Ok(())
    }
    fn OnControlActivating(
        &self,
        _: Ref<IFileDialogCustomize>,
        _: u32,
    ) -> windows::core::Result<()> {
        Ok(())
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
