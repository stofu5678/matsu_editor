//! The registered macros (登録マクロ): running macro `n` of the 「マクロ」 page as the function
//! `F_USERMACRO_0 + n`, and the three macros that run by themselves.
//!
//! Sakura: `CViewCommander::HandleCommand` (`sakura_core/cmd/CViewCommander.cpp`) hands a function in
//! `F_USERMACRO_0 ..< F_USERMACRO_0 + MAX_CUSTMACRO` to `CSMacroMgr::Exec`
//! (`sakura_core/macro/CSMacroMgr.cpp`), which reads the file when it has not been read yet or when
//! the macro says 実行時に読み込み, and runs it; a failure is told with `STR_ERR_MACRO1`.
//! `CEditDoc::RunAutoMacro` (`sakura_core/doc/CEditDoc.cpp`) runs the macro of オープン後, タイプ変更後
//! and 保存前.
//!
//! # Differences from Sakura
//!
//! * Only a key macro (`.mac`) is kept after it is read, so 実行時に読み込み matters for it. A script is
//!   read from its file each time it runs.
//! * A script runs beside the window, not before the next line of the caller. The save waits for the
//!   macro of 保存前 to end (`run_before_save`), so the file is written after the macro as in Sakura.
//!   The macro of オープン後 that follows a read, a close, a save under a name or the start of an
//!   untitled window is waited for in the same way before what comes next (`-M=`). The macros called
//!   by `F_CHANGETYPE` and by a key are not waited for.
//! * The type chosen by `-TYPE=` of the command line is applied after the macro of オープン後 runs.
//! * `bRunning` of `RunAutoMacro` (an automatic macro does not start another) is "no macro runs in
//!   this window".

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui::{App, AsyncWindowContext, Context, Window};
use matsu_commands::command::Arg;
use matsu_commands::key_macro::{KeyMacro, Step};
use matsu_config::settings::macros::RegisteredMacros;

use super::{FileKind, file_kind};
use crate::file_dialog;
use crate::menu::{Item, Lists, MenuAction};
use crate::settings::UserSettings;
use crate::view::EditorView;

/// The three moments of the automatic macros (`CShareData_IO` keys `nMacroOnOpened`,
/// `nMacroOnTypeChanged` and `nMacroOnSave`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Auto {
    /// After a file is opened (`FileLoad`, `ReloadCurrentFile`, the start of an untitled window).
    Opened,
    /// After the type of the document changed (`CEditDoc::OnChangeType`).
    TypeChanged,
    /// Before a file is saved (`DoSaveFlow`), with the path it will be written to.
    BeforeSave,
}

impl Auto {
    /// The macro number the settings give for this moment.
    pub(crate) fn index(self, table: &RegisteredMacros) -> Option<usize> {
        match self {
            Auto::Opened => table.on_opened,
            Auto::TypeChanged => table.on_type_changed,
            Auto::BeforeSave => table.on_save,
        }
    }
}

/// What a view keeps for the registered macros.
#[derive(Default)]
pub(crate) struct State {
    /// The key macros read for a registered macro, with the path they came from
    /// (`CSMacroMgr::m_cSavedKeyMacro`).
    loaded: HashMap<usize, (PathBuf, Vec<Step>)>,
    /// The path a save will write to while the macro of 保存前 runs (`CDocFile::m_szSaveFilePath`).
    save_path: Option<PathBuf>,
}

impl State {
    /// The path of the save that the running macro is the 保存前 macro of.
    pub(super) fn save_path(&self) -> Option<&Path> {
        self.save_path.as_deref()
    }

    /// The macro that was told the path has ended.
    pub(super) fn clear_save_path(&mut self) {
        self.save_path = None;
    }
}

/// The registered macros of the settings now.
pub(crate) fn table(cx: &App) -> RegisteredMacros {
    cx.try_global::<UserSettings>()
        .map(UserSettings::registered_macros)
        .unwrap_or_default()
}

/// The title of the function `code` if it is a registered macro (`CFuncLookup::Funccode2Name`): the name,
/// or the file when there is no name; a macro with no file is `マクロ n (未登録)`
/// (`STR_ERR_DLGFUNCLKUP03`).
pub(crate) fn title_of(table: &RegisteredMacros, code: u16) -> Option<String> {
    let index = matsu_config::settings::macros::index_of_function(code)?;
    let rec = table.get(index)?;
    Some(if rec.is_enabled() {
        rec.title().to_owned()
    } else {
        format!("マクロ {index} (未登録)")
    })
}

/// `F_USERMACRO_LIST` of the main menu (`CEditWnd::InitMenu_Special`): the macros that have a file,
/// each calling its function. A macro with a file and no name shows the file.
pub(crate) struct MacroList(pub RegisteredMacros);

impl Lists for MacroList {
    fn items(&self, code: u16) -> Vec<Item> {
        if code != F_USERMACRO_LIST {
            return Vec::new();
        }
        self.0
            .macros
            .iter()
            .enumerate()
            .filter(|(_, rec)| rec.is_enabled())
            .filter_map(|(index, rec)| {
                Some(Item {
                    label: rec.title().to_owned(),
                    action: MenuAction::Function(
                        matsu_config::settings::macros::function_of_index(index)?,
                    ),
                    shortcut: None,
                    enabled: true,
                })
            })
            .collect()
    }
}

/// `F_USERMACRO_LIST`, the row of the main menu that stands for the registered macros.
const F_USERMACRO_LIST: u16 = 29005;

/// `CShareData::GetMacroFilename`: the path of a registered macro's file. An absolute file is as it
/// is; any other is in the macro folder (a folder that is not known leaves it as it is).
pub(crate) fn resolve(file: &str, folder: Option<&Path>) -> Option<PathBuf> {
    if file.is_empty() {
        return None;
    }
    let native = if cfg!(windows) {
        file.to_owned()
    } else {
        file.replace('\\', "/")
    };
    let path = PathBuf::from(native);
    Some(match folder {
        Some(folder) if !path.is_absolute() => folder.join(path),
        _ => path,
    })
}

impl EditorView {
    /// `F_USERMACRO_0 + index`: runs registered macro `index`. A macro with no file, a file that cannot
    /// be read and a macro that is already running are told with Sakura's words
    /// (`STR_ERR_MACRO1`). Returns whether the macro started (a key macro that plays stops at the
    /// first call that fails, as `exec_ext_macro` does).
    pub(crate) fn run_registered_macro(
        &mut self,
        index: usize,
        record: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let settings = table(cx);
        let rec = settings.get(index).cloned().unwrap_or_default();
        let failed =
            |this: &mut Self, detail: &str, window: &mut Window, cx: &mut Context<Self>| {
                let message = format!("マクロ {index} ({}) の実行に失敗しました。", rec.file);
                let message = if detail.is_empty() {
                    message
                } else {
                    format!("{message}\n{detail}")
                };
                this.macro_failed(message, window, cx);
                false
            };
        let folder = file_dialog::macro_folder(cx);
        let Some(path) = resolve(&rec.file, folder.as_deref()) else {
            return failed(self, "", window, cx);
        };
        // A key macro that is playing may name a macro: its calls go on in the middle.
        if self.macros.running.is_some() {
            return failed(self, "マクロを実行中です", window, cx);
        }
        // While a key macro is recorded, the call is recorded as one call of the file, as for
        // `F_EXECEXTMACRO` (the calls of the macro itself are not). An automatic macro is not
        // (`FA_NONRECORD`).
        if record && self.macros.recorder.is_recording() && !self.macros.suspended {
            self.macros.recorder.record(
                super::F_EXECEXTMACRO,
                &[Arg::Text(path.to_string_lossy().into_owned())],
            );
        }
        let result = match file_kind(&path) {
            FileKind::KeyMacro => match self.registered_steps(index, &path, rec.reload) {
                Ok(steps) => self.start_key_macro(steps, Some(&path), window, cx),
                Err(message) => Err(message),
            },
            _ => self.run_macro_file(&path, window, cx),
        };
        match result {
            Ok(()) => true,
            Err(message) => failed(self, &message, window, cx),
        }
    }

    /// The calls of the key macro of slot `index`: read again when the slot was not read yet, when
    /// its file changed, or when 実行時に読み込み is on (`CSMacroMgr::Exec`).
    fn registered_steps(
        &mut self,
        index: usize,
        path: &Path,
        reload: bool,
    ) -> Result<Vec<Step>, String> {
        if !reload
            && let Some((from, steps)) = self.macros.registered.loaded.get(&index)
            && from == path
        {
            return Ok(steps.clone());
        }
        let key_macro = KeyMacro::load(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let steps = key_macro.steps().to_vec();
        self.macros
            .registered
            .loaded
            .insert(index, (path.to_owned(), steps.clone()));
        Ok(steps)
    }

    /// `CEditDoc::RunAutoMacro`: runs the macro the settings name for `when`, unless it has no file,
    /// another macro runs in this window, or Shift is held. `save_path` is the file a save will
    /// write (`GetSaveFilename` answers it while the macro runs). A failure is told as for a macro
    /// that a key runs. Returns whether the macro started and is still going (a script, or a key
    /// macro that waits for a dialog).
    pub(crate) fn run_auto_macro(
        &mut self,
        when: Auto,
        save_path: Option<&Path>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let settings = table(cx);
        let Some(index) = when
            .index(&settings)
            .filter(|&i| settings.get(i).is_some_and(|rec| rec.is_enabled()))
        else {
            return false;
        };
        if self.macros.busy() || self.macros.is_suspended() || window.modifiers().shift {
            return false;
        }
        self.macros.registered.save_path = save_path.map(Path::to_owned);
        self.run_registered_macro(index, false, window, cx);
        // A macro that has ended (or did not start) leaves nothing to clear later; one that goes
        // on is cleared where it ends.
        if self.macros.is_suspended() {
            true
        } else {
            self.macros.registered.clear_save_path();
            false
        }
    }
}

/// [`EditorView::run_auto_macro`] from an async flow (loading, saving) of the window.
fn run_auto(
    view: &gpui::WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    when: Auto,
    save_path: Option<&Path>,
) -> bool {
    let Some(view) = view.upgrade() else {
        return false;
    };
    cx.update(|window, app| {
        view.update(app, |view, cx| {
            view.run_auto_macro(when, save_path, window, cx)
        })
    })
    .unwrap_or(false)
}

/// Waits until the macro that this window runs has ended (the button 中止 of the panel stops one
/// that does not).
async fn wait_for_the_macro(view: &gpui::WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    while view
        .read_with(cx, |view, _| view.macros.is_suspended())
        .unwrap_or(false)
    {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(50))
            .await;
    }
}

/// The macro of 保存前 (`DoSaveFlow`): it runs to its end before the file is written, as in Sakura, where
/// the call does not return before. A script and a key macro that waits for a dialog go on beside the
/// window, so the save waits for them.
pub(crate) async fn run_before_save(
    view: &gpui::WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: &Path,
) {
    if run_auto(view, cx, Auto::BeforeSave, Some(path)) {
        wait_for_the_macro(view, cx).await;
    }
}

/// The macro of オープン後, after a file was read into the window (`FileLoad`), saved under another
/// name (`FileSaveAs`) or closed to an untitled one (`FileClose`). It is waited for, so that the
/// macro of the command line (`-M=`) that follows is not refused (`CNormalProcess`).
///
/// A read chooses the type of the file without the macro of タイプ変更後: `CLoadAgent::OnLoad` sets the
/// type, and `CEditDoc::OnChangeType`, which runs that macro, is called only by `F_CHANGETYPE` and by
/// the start of a Grep (`sakura_core/cmd/CViewCommander_Settings.cpp`, `CViewCommander_Grep.cpp`).
pub(crate) async fn run_opened(view: &gpui::WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    if run_auto(view, cx, Auto::Opened, None) {
        wait_for_the_macro(view, cx).await;
    }
}

/// [`EditorView::run_auto_macro`] for a window that is open already (the start of an untitled
/// window, or a file that was closed). `then` runs when the macro has ended (at once when none
/// started), so that the macro of the command line (`-M=`) that follows is not refused.
pub(crate) fn run_auto_in(
    handle: gpui::AnyWindowHandle,
    view: &gpui::Entity<EditorView>,
    when: Auto,
    cx: &mut App,
    then: impl FnOnce(&mut App) + 'static,
) {
    let running = handle
        .update(cx, |_, window, cx| {
            view.update(cx, |view, cx| view.run_auto_macro(when, None, window, cx))
        })
        .unwrap_or(false);
    if !running {
        then(cx);
        return;
    }
    let view = view.clone();
    cx.spawn(async move |cx| {
        while view.read_with(cx, |view, _| view.macros.is_suspended()) {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(50))
                .await;
        }
        cx.update(then);
    })
    .detach();
}

/// `F_FILENEW` and `F_FILENEW_NEWWINDOW`: the new untitled document runs the macro of オープン後 (Sakura
/// starts a process, and `CNormalProcess` runs it). It runs once the command has ended, because
/// the window of a new tab is the one that the command is working in.
pub(crate) fn run_opened_in_new(
    cx: &mut App,
    opened: Option<(gpui::AnyWindowHandle, gpui::Entity<EditorView>)>,
) {
    if let Some((handle, view)) = opened {
        cx.defer(move |cx| run_auto_in(handle, &view, Auto::Opened, cx, |_| {}));
    }
}
