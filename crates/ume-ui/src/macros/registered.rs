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
//! * A script runs beside the window, not before the next line of the caller. A script of 保存前
//!   therefore runs while the save goes on; a key macro (which plays at once) finishes before the
//!   file is written, as in Sakura.
//! * `bRunning` of `RunAutoMacro` (an automatic macro does not start another) is "no macro runs in
//!   this window".

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui::{App, AsyncWindowContext, Context, Window};
use ume_commands::key_macro::{KeyMacro, Step};
use ume_config::settings::macros::RegisteredMacros;

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
    let index = ume_config::settings::macros::index_of_function(code)?;
    let rec = table.get(index)?;
    Some(if rec.is_enabled() {
        rec.title().to_owned()
    } else {
        format!("マクロ {index} (未登録)")
    })
}

/// [`title_of`] for the settings now. Not a registered macro: `None`, without reading the settings.
pub(crate) fn title_in(cx: &App, code: u16) -> Option<String> {
    ume_config::settings::macros::index_of_function(code)?;
    title_of(&table(cx), code)
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
                    action: MenuAction::Function(ume_config::settings::macros::function_of_index(
                        index,
                    )?),
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
    /// that a key runs.
    pub(crate) fn run_auto_macro(
        &mut self,
        when: Auto,
        save_path: Option<&Path>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let settings = table(cx);
        let Some(index) = when
            .index(&settings)
            .filter(|&i| settings.get(i).is_some_and(|rec| rec.is_enabled()))
        else {
            return;
        };
        if self.macros.busy() || self.macros.is_suspended() || window.modifiers().shift {
            return;
        }
        self.macros.registered.save_path = save_path.map(Path::to_owned);
        self.run_registered_macro(index, window, cx);
        // A macro that did not start leaves nothing to clear later.
        if !self.macros.is_suspended() {
            self.macros.registered.save_path = None;
        }
    }
}

/// [`EditorView::run_auto_macro`] from an async flow (loading, saving) of the window.
pub(crate) fn run_auto(
    view: &gpui::WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    when: Auto,
    save_path: Option<&Path>,
) {
    let Some(view) = view.upgrade() else {
        return;
    };
    cx.update(|window, app| {
        view.update(app, |view, cx| {
            view.run_auto_macro(when, save_path, window, cx)
        });
    })
    .ok();
}

/// What follows a file that was read into the window: its type is chosen (`OnChangeType`), then the
/// 開いた後 macro runs (`FileLoad`).
pub(crate) fn after_open(view: &gpui::WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    run_auto(view, cx, Auto::TypeChanged, None);
    run_auto(view, cx, Auto::Opened, None);
}

/// [`EditorView::run_auto_macro`] for a window that is open already (the start of an untitled
/// window).
pub(crate) fn run_auto_in(
    handle: gpui::AnyWindowHandle,
    view: &gpui::Entity<EditorView>,
    when: Auto,
    cx: &mut App,
) {
    handle
        .update(cx, |_, window, cx| {
            view.update(cx, |view, cx| view.run_auto_macro(when, None, window, cx));
        })
        .ok();
}
