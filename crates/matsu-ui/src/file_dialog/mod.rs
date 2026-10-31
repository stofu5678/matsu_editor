//! The file dialogs, opened where Sakura's open (「ファイルダイアログの初期位置」, `GetDlgInitialDir`).
//!
//! GPUI's open dialog cannot be told where to start, so the dialogs of the OS are used
//! (`matsu_platform::dialog`: `IFileOpenDialog`, `NSOpenPanel`, the file chooser portal) with a start
//! folder. When the OS cannot show one (no portal on Linux) GPUI's dialog is the fallback, without
//! a start folder; its save dialog takes one, so that keeps it.
//!
//! * [`start_folder`] chooses the folder: the pure rules are `matsu_config::dialog_dir`.
//! * [`ask`] shows a dialog and gives the paths chosen.
//! * [`save_types`] makes the file types and the first name of the save dialog.
//!
//! GPUI's save dialog has no list of file types: when it is the fallback, the user sees every
//! file, and the default extension is added to the name it answers.

use std::future::Future;
use std::path::{Path, PathBuf};

use gpui::{App, AsyncApp, PathPromptOptions, Window};
use matsu_config::dialog_dir::{DialogDirSetting, MetaSource, Surroundings};
use matsu_config::history::History;
use matsu_platform::dialog::{
    Chosen, Extra, Kind, Owner, RecentKind, Request, extension_for_filter, with_default_extension,
};
use raw_window_handle::HasWindowHandle;

use crate::settings::UserSettings;

/// What a dialog gives: the paths chosen (`None` when the user cancels), or the text of the error.
pub(crate) type Answer = Result<Option<Vec<PathBuf>>, String>;

/// The dialog to show.
#[derive(Clone, Debug)]
pub(crate) struct Ask {
    kind: Kind,
    start: Option<PathBuf>,
    name: Option<String>,
    title: String,
    /// The label of the button of GPUI's dialog (`PathPromptOptions::prompt`).
    prompt: Option<String>,
    /// The file types and the default extension of a save dialog.
    types: Option<save_types::SaveTypes>,
    /// The encoding, BOM and line ending next to the name of a save dialog.
    fields: Option<save_fields::SaveFields>,
    /// The lists of recent files and folders ([`recent_extras`]), after the fields.
    recent: Vec<Extra>,
}

impl Ask {
    /// The open dialog: one file, or several.
    pub(crate) fn open(multiple: bool) -> Self {
        Self::new(Kind::OpenFile { multiple }, "ファイルを開く")
    }

    /// The dialog that chooses a folder.
    pub(crate) fn folder() -> Self {
        Self::new(Kind::Folder, "フォルダーの選択")
    }

    /// The dialog that chooses a name to save under.
    pub(crate) fn save() -> Self {
        Self::new(Kind::SaveFile, "名前を付けて保存")
    }

    fn new(kind: Kind, title: &str) -> Self {
        Self {
            kind,
            start: None,
            name: None,
            title: title.to_owned(),
            prompt: None,
            types: None,
            fields: None,
            recent: Vec::new(),
        }
    }

    /// The fields next to the name of a save dialog. A dialog that cannot show them (the panel of
    /// GPUI, the fallback) answers without values: the caller then asks another way.
    pub(crate) fn save_fields(mut self, fields: save_fields::SaveFields) -> Self {
        self.fields = Some(fields);
        self
    }

    /// The lists of recent files and folders next to the name (Sakura's `COMBO_MRU` and
    /// `COMBO_OPENFOLDER` of the open dialog and of the save dialog with fields), from the history
    /// of the application. Only the dialog of Windows shows them.
    pub(crate) fn recent(mut self, cx: &App) -> Self {
        self.recent = recent_extras(crate::history::current(cx).as_ref());
        self
    }

    /// The list of file types of a save dialog, and the extension a name typed without one gets.
    pub(crate) fn save_types(mut self, types: save_types::SaveTypes) -> Self {
        self.types = Some(types);
        self
    }

    /// The list of file types of an open dialog (or of a save dialog without a default
    /// extension). The first entry is shown first.
    pub(crate) fn file_types(self, filters: Vec<matsu_platform::dialog::FileFilter>) -> Self {
        self.save_types(save_types::SaveTypes {
            filters,
            default_extension: None,
        })
    }

    /// The folder the dialog starts in. `None`: where the OS wants.
    pub(crate) fn start(mut self, start: Option<PathBuf>) -> Self {
        self.start = start;
        self
    }

    /// The file name the dialog offers.
    pub(crate) fn name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    /// The title of the dialog and the label of the button of GPUI's dialog.
    pub(crate) fn titled(mut self, title: impl Into<String>, prompt: impl Into<String>) -> Self {
        self.title = title.into();
        self.prompt = Some(prompt.into());
        self
    }

    fn request(&self) -> Request {
        Request {
            kind: self.kind,
            start: self.start.clone().filter(|dir| dir.is_dir()),
            name: self.name.clone(),
            title: self.title.clone(),
            filters: self.types.iter().flat_map(|t| t.filters.clone()).collect(),
            selected_filter: 0,
            default_extension: self
                .types
                .as_ref()
                .and_then(|t| t.default_extension.clone()),
            extras: self
                .fields
                .as_ref()
                .map_or_else(Vec::new, |fields| fields.extras.clone())
                .into_iter()
                .chain(self.recent.iter().cloned())
                .collect(),
            follow: self.fields.as_ref().map(|fields| fields.follow.clone()),
        }
    }
}

/// The two lists of recent paths of Sakura's file dialogs: 最近のファイル (`CMRUFile::GetPathList`) and
/// 最近のフォルダー (`CMRUFolder::GetPathList`), `COMBO_MRU` and `COMBO_OPENFOLDER` of
/// `CDlgOpenFile_CommonItemDialog::Customize`. The labels are `STR_FILEDIALOG_MRU` and
/// `STR_FILEDIALOG_OPENFOLDER` (`sakura_core/sakura_rc.rc`). The lists are there even when empty.
pub(crate) fn recent_extras(history: Option<&History>) -> Vec<Extra> {
    let paths = |list: Vec<&str>| list.into_iter().map(str::to_owned).collect();
    vec![
        Extra::Recent {
            label: "最近のファイル(&F):".to_owned(),
            items: history.map_or_else(Vec::new, |h| paths(h.file_paths())),
            kind: RecentKind::Files,
        },
        Extra::Recent {
            label: "最近のフォルダー(&D):".to_owned(),
            items: history.map_or_else(Vec::new, |h| paths(h.folder_paths())),
            kind: RecentKind::Folders,
        },
    ]
}

/// Shows the dialog, modal to `window`, and answers when the user closes it. The returned future does
/// not borrow anything: it can be awaited after the command has returned.
pub(crate) fn ask(window: &Window, cx: &mut App, ask: Ask) -> impl Future<Output = Answer> + use<> {
    let chosen = ask_chosen(window, cx, ask);
    async move { Ok(chosen.await?.map(|chosen| chosen.paths)) }
}

/// What [`ask_chosen`] gives: the paths chosen (`None` when the user cancels), with the values of
/// the fields of [`Ask::save_fields`], or the text of the error. The values are empty when the
/// dialog showed no fields.
pub(crate) type ChosenAnswer = Result<Option<Chosen>, String>;

/// [`ask`] with the values of the fields next to the name.
pub(crate) fn ask_chosen(
    window: &Window,
    cx: &mut App,
    ask: Ask,
) -> impl Future<Output = ChosenAnswer> + use<> {
    let owner = HasWindowHandle::window_handle(window)
        .ok()
        .map(|handle| Owner::from_handle(handle.as_raw()))
        .unwrap_or_default();
    let app = cx.to_async();
    async move {
        let request = ask.request();
        #[cfg(test)]
        tests::record(&request);
        // Whatever stopped the dialog of the OS (it does not exist here, or it failed), GPUI's
        // dialog is still there.
        match native(owner, request).await {
            Ok(chosen) => Ok(chosen),
            Err(_) => fallback(&app, &ask).await.map(|paths| {
                // GPUI's dialog does not add the default extension, as the one of Windows does
                let paths = match (&ask.types, ask.kind) {
                    (Some(types), Kind::SaveFile) => {
                        // no list is shown: the type shown first decides
                        let extension = types
                            .default_extension
                            .as_deref()
                            .map(|fallback| extension_for_filter(types.filters.first(), fallback));
                        paths.map(|paths| {
                            paths
                                .into_iter()
                                .map(|path| match &extension {
                                    Some(extension) => with_default_extension(path, extension),
                                    None => path,
                                })
                                .collect()
                        })
                    }
                    _ => paths,
                };
                // and has no fields: the values are left empty
                paths.map(|paths| Chosen {
                    paths,
                    extras: Vec::new(),
                })
            }),
        }
    }
}

/// The dialog of the OS. The tests of the windows have no desktop to show it on, except for the
/// answer a test prepared ([`tests::answer_with`]).
async fn native(
    owner: Owner,
    request: Request,
) -> Result<Option<Chosen>, matsu_platform::dialog::Error> {
    #[cfg(test)]
    {
        let _ = owner;
        match tests::prepared_answer(&request) {
            Some(answer) => Ok(answer),
            None => Err(matsu_platform::dialog::Error::Unavailable(format!(
                "no desktop in the tests ({:?})",
                request.kind
            ))),
        }
    }
    #[cfg(not(test))]
    {
        matsu_platform::dialog::show_with_extras(owner, request).await
    }
}

/// GPUI's dialog: no start folder for the open dialogs.
async fn fallback(app: &AsyncApp, ask: &Ask) -> Answer {
    match ask.kind {
        Kind::SaveFile => {
            let directory = ask
                .start
                .clone()
                .filter(|dir| dir.is_dir())
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_default();
            let receiver = app.update(|cx| cx.prompt_for_new_path(&directory, ask.name.as_deref()));
            match receiver.await {
                Ok(Ok(path)) => Ok(path.map(|path| vec![path])),
                Ok(Err(error)) => Err(error.to_string()),
                Err(_) => Ok(None),
            }
        }
        Kind::OpenFile { .. } | Kind::Folder => {
            let options = PathPromptOptions {
                files: matches!(ask.kind, Kind::OpenFile { .. }),
                directories: ask.kind == Kind::Folder,
                multiple: matches!(ask.kind, Kind::OpenFile { multiple: true }),
                prompt: ask.prompt.clone().map(Into::into),
            };
            let receiver = app.update(|cx| cx.prompt_for_paths(options));
            match receiver.await {
                Ok(Ok(paths)) => Ok(paths.filter(|paths| !paths.is_empty())),
                Ok(Err(error)) => Err(error.to_string()),
                Err(_) => Ok(None),
            }
        }
    }
}

/// Where the meta folders of the settings (`%MYDOC%` ...) come from on this machine.
pub(crate) struct Meta {
    settings_dir: Option<PathBuf>,
}

impl Meta {
    pub(crate) fn new(cx: &App) -> Self {
        Self {
            settings_dir: settings_dir(cx),
        }
    }
}

impl MetaSource for Meta {
    fn program_dir(&self) -> Option<String> {
        program_dir().map(|dir| dir.to_string_lossy().into_owned())
    }

    fn settings_dir(&self) -> Option<String> {
        self.settings_dir
            .as_ref()
            .map(|dir| dir.to_string_lossy().into_owned())
    }

    fn shell_folder(&self, name: &str) -> Option<String> {
        matsu_platform::folders::known_folder(name).map(|dir| dir.to_string_lossy().into_owned())
    }

    fn env_var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

/// The folder of the program (`GetExedir`).
fn program_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_owned))
}

/// The folder of the settings file (`GetInidir`).
pub(crate) fn settings_dir(cx: &App) -> Option<PathBuf> {
    cx.try_global::<UserSettings>()?
        .path()?
        .parent()
        .map(Path::to_owned)
}

/// The folder the dialogs start in (`CSakuraEnvironment::GetDlgInitialDir`): the folder of the
/// file of the document `document`, else what 「ファイルダイアログの初期位置」 says. `None` when the
/// dialog is to start where the OS wants.
pub(crate) fn start_folder(cx: &App, document: Option<&Path>) -> Option<PathBuf> {
    let setting: DialogDirSetting = cx
        .try_global::<UserSettings>()
        .map(UserSettings::dialog_dir)
        .unwrap_or_default();
    // the folders the menu shows (`CMRUFolder::GetPathList`)
    let recent: Vec<String> = crate::history::current(cx)
        .map(|history| {
            history
                .folder_paths()
                .into_iter()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let meta = Meta::new(cx);
    setting.initial_dir(&Surroundings {
        document,
        recent_folders: &recent,
        current_dir: std::env::current_dir().ok(),
        meta: &meta,
        is_dir: &|path| path.is_dir(),
    })
}

/// The text of a settings box with its meta folders (`%MYDOC%` ...) expanded, as the file has it.
pub(crate) fn expanded_text(cx: &App, text: &str) -> String {
    matsu_config::dialog_dir::expand_meta(text, &Meta::new(cx)).text
}

/// The folder that the text of a settings box names (「指定フォルダー」): its meta folders
/// (`%MYDOC%` ...) expanded, `None` when there is no such folder.
pub(crate) fn expanded_folder(cx: &App, text: &str) -> Option<PathBuf> {
    let expanded = expanded_text(cx, text);
    // The settings file of Windows holds backslashes; on the other OSes they are separators too.
    let native = if cfg!(windows) {
        expanded
    } else {
        expanded.replace('\\', "/")
    };
    Some(PathBuf::from(native)).filter(|path| !path.as_os_str().is_empty() && path.is_dir())
}

/// The folder the macro dialogs start in (マクロ用フォルダー).
pub(crate) fn macro_folder(cx: &App) -> Option<PathBuf> {
    matsu_config::dialog_dir::macro_folder(
        macro_folder_setting(cx).as_deref(),
        settings_dir(cx).as_deref(),
        program_dir().as_deref(),
        &|path| path.is_dir(),
    )
}

/// The file that stopping a key macro recording writes (`RecKey.mac` in the macro folder).
pub(crate) fn key_macro_file(cx: &App) -> Option<PathBuf> {
    matsu_config::dialog_dir::key_macro_file(
        macro_folder_setting(cx).as_deref(),
        settings_dir(cx).as_deref(),
        program_dir().as_deref(),
        &|path| path.is_dir(),
    )
}

fn macro_folder_setting(cx: &App) -> Option<String> {
    cx.try_global::<UserSettings>()
        .and_then(UserSettings::macro_folder)
}

pub(crate) mod import_folder;
pub(crate) mod save_fields;
pub(crate) mod save_types;
#[cfg(test)]
pub(crate) mod tests;
