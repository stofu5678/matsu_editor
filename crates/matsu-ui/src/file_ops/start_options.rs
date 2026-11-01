//! What Sakura's command-line options do to the documents a launch opens.
//!
//! The options are read in [`crate::command_line`]. A launch (the first one, or one that a running
//! process receives, ADR-0016) reaches [`open_from_outside_with`](super::windows::open_from_outside_with),
//! which opens the documents and uses [`LaunchOptions`] for the rest.
//!
//! Sakura, `CNormalProcess::InitializeProcess` (`sakura_core/_main/CNormalProcess.cpp`) and
//! `CControlTray::OpenNewEditor2` (`sakura_core/_main/CControlTray.cpp`):
//!
//! * `-X=` `-Y=` `-VX=` `-VY=`, `-CODE=` and `-R` are for every file of the launch (`OpenFiles`
//!   passes them on). After the load, the view goes to `-VX=` `-VY=` (when both are given and the
//!   row exists), and then the caret goes to `-X=` `-Y=` (when either is given), in the line,
//!   not in the line ending. A file that is open already is not loaded again: its window comes to
//!   the front, and the caret goes to the line of `-Y=` (and `-X=`, else the column it has).
//! * `-TYPE=`, `-SX=` `-SY=` `-WX=` `-WY=` and `-M=` are for the first document of the launch
//!   (the other files are opened by `OpenNewEditor2`, which does not pass them on). The type is
//!   chosen by the extension and kept for the document (`SetDocumentTypeWhenCreate` locks it).
//!   The window gets the size and the place, if it is a new window; a new tab does not. The macro
//!   runs after the load.
//! * A launch without a file opens an empty document, with the type, the encoding and the view
//!   mode of the options (`SetDocumentTypeWhenCreate`): the encoding of the type if `-CODE=` is
//!   the one the type has, else `-CODE=` with the default BOM and CR LF.
//! * `-GREPMODE` opens a window for a Grep and runs it with the conditions of the options
//!   (`CGrepAgent::DoGrep`); `-GREPDLG` opens the Grep dialog with them instead.
//!
//! The options that have nothing to do in MATSU Editor are read and dropped: `-NOWIN` (MATSU Editor has
//! no process in the tray: the launch opens its window as any launch does), `-GROUP=` (the groups
//! of the tabs are not numbered), `-DEBUGMODE` (no output window for `OutputDebugString`),
//! `-PROFMGR` (no profile manager), and from `-GOPT=` the letters `U` (the standard output), `H`
//! (no heading; the engine always writes it) and `X` (MATSU Editor does not change its current
//! folder).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui::{
    AnyWindowHandle, App, AsyncWindowContext, Bounds, Context, Entity, Global, Pixels, Size,
    WeakEntity, Window, point, px, size,
};
use matsu_core::buffer::{Eol, Pos};
use matsu_core::encoding::{AUTODETECT_CODE, Encoding};

use crate::command_line::{CommandLine, GrepArgs};
use crate::grep::engine::{GrepInfo, ReplaceInfo};
use crate::grep::format::{Layout, LineType, OutputStyle};
use crate::grep::keys::{is_relative, split_folders, split_pattern_keep_quotes};
use crate::grep::settings::{GrepSettings, quote_folder};
use crate::search::state::SearchOptions;
use crate::settings::NewDocumentDefaults;
use crate::view::EditorView;

/// What a launch asks of each file it opens.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FileOptions {
    /// `-X=`: the column of the caret in UTF-16 code units, from 0.
    pub caret_x: Option<usize>,
    /// `-Y=`: the line of the caret, from 0.
    pub caret_y: Option<usize>,
    /// `-VX=`: the column at the left of the view.
    pub view_left: Option<usize>,
    /// `-VY=`: the row at the top of the view.
    pub view_top: Option<usize>,
    /// `-CODE=`, if Sakura's number is an encoding (99, the detection, and the numbers that are no
    /// encoding are none).
    pub encoding: Option<Encoding>,
    /// `-R`.
    pub view_mode: bool,
}

impl FileOptions {
    fn from_command_line(line: &CommandLine) -> Self {
        Self {
            caret_x: line.caret_x,
            caret_y: line.caret_y,
            view_left: line.view_left,
            view_top: line.view_top,
            encoding: line
                .code
                .filter(|code| *code != AUTODETECT_CODE)
                .and_then(Encoding::from_sakura_code),
            view_mode: line.view_mode,
        }
    }

    fn moves_view(&self) -> bool {
        self.view_left.is_some() && self.view_top.is_some()
    }

    fn moves_caret(&self) -> bool {
        self.caret_x.is_some() || self.caret_y.is_some()
    }
}

/// The macro of `-M=`.
#[derive(Clone, Debug, PartialEq, Eq)]
enum MacroLaunch {
    /// A file (`-MTYPE=` is empty or `file`), made absolute with the folder of the launch.
    File(PathBuf),
    /// The text of a macro (`-M=`) in the language of the extension `-MTYPE=` names.
    Text { language: String, code: String },
}

/// The size and the place of a new window (`-SX=` `-SY=` `-WX=` `-WY=`), in pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Placement {
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub x: Option<i32>,
    pub y: Option<i32>,
}

impl Placement {
    fn from_command_line(line: &CommandLine) -> Self {
        Self {
            width: line.window_width,
            height: line.window_height,
            x: line.window_x,
            y: line.window_y,
        }
    }

    fn is_empty(&self) -> bool {
        *self == Placement::default()
    }

    /// The size of the window: the given one where there is one. A size of 0 pixels (`-SX=1`) is
    /// no window, so it is the default size.
    pub(crate) fn size_or(&self, default: Size<Pixels>) -> Size<Pixels> {
        let pick = |given: Option<i32>, default: Pixels| match given {
            Some(pixels) if pixels > 0 => px(pixels as f32),
            _ => default,
        };
        size(
            pick(self.width, default.width),
            pick(self.height, default.height),
        )
    }

    /// Moves the window to the given coordinates; the one that is not given stays.
    pub(crate) fn place(&self, bounds: &mut Bounds<Pixels>) {
        if let Some(x) = self.x {
            bounds.origin = point(px(x as f32), bounds.origin.y);
        }
        if let Some(y) = self.y {
            bounds.origin = point(bounds.origin.x, px(y as f32));
        }
    }
}

/// The Grep of `-GREPMODE` / `-GREPDLG`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct GrepLaunch {
    args: GrepArgs,
    /// `-GREPDLG`: the dialog instead of the search.
    dialog: bool,
}

/// What a launch asks, apart from the files.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LaunchOptions {
    pub(crate) file: FileOptions,
    /// `-TYPE=`.
    doc_type: Option<String>,
    placement: Placement,
    macro_: Option<MacroLaunch>,
    grep: Option<GrepLaunch>,
    /// The working folder of the launch.
    cwd: PathBuf,
}

impl LaunchOptions {
    /// Whether the launch asks for nothing but a window: no macro, no Grep, no type, no place, none of the
    /// options of the files. With the documents of the last time restored, such a launch needs no window of its own.
    pub(crate) fn asks_only_for_a_window(&self) -> bool {
        self.macro_.is_none()
            && self.grep.is_none()
            && self.doc_type.is_none()
            && self.placement == Placement::default()
            && self.file == FileOptions::default()
    }

    /// The options of a request.
    pub(crate) fn from_request(request: &crate::launch::OpenRequest) -> Self {
        Self::new(&request.options, &request.cwd)
    }

    pub(crate) fn new(line: &CommandLine, cwd: &Path) -> Self {
        let macro_ = (!line.macro_.is_empty()).then(|| {
            if line.macro_type.is_empty() || line.macro_type.eq_ignore_ascii_case("file") {
                MacroLaunch::File(cwd.join(&line.macro_))
            } else {
                MacroLaunch::Text {
                    language: line.macro_type.clone(),
                    code: line.macro_.clone(),
                }
            }
        });
        // `-GREPDLG` works with `-GREPMODE` only (`CNormalProcess`).
        let grep = line.grep_mode.then(|| GrepLaunch {
            args: line.grep.clone(),
            dialog: line.grep_dialog,
        });
        Self {
            file: FileOptions::from_command_line(line),
            // `-DEBUGMODE` alone gives the type `output`; that window is not in MATSU Editor.
            doc_type: line
                .doc_type
                .clone()
                .filter(|t| !(line.debug_mode && t == "output")),
            placement: Placement::from_command_line(line),
            macro_,
            grep,
            cwd: cwd.to_owned(),
        }
    }

    /// Whether the launch is a Grep.
    pub(crate) fn is_grep(&self) -> bool {
        self.grep.is_some()
    }

    /// What follows the load of the first document of the launch, or of a later one.
    pub(crate) fn after_load(&self, first: bool) -> Option<AfterLoad> {
        let file = FileOptions {
            encoding: None,
            view_mode: false,
            ..self.file.clone()
        };
        let after = AfterLoad {
            file,
            doc_type: self.doc_type.clone().filter(|_| first),
            macro_: self.macro_.clone().filter(|_| first),
        };
        (after.file.moves_caret()
            || after.file.moves_view()
            || after.doc_type.is_some()
            || after.macro_.is_some())
        .then_some(after)
    }

    /// The size and the place for the first window of the launch.
    pub(crate) fn placement(&self) -> Placement {
        self.placement
    }

    /// A file that is open already: its caret goes to the line of `-Y=`
    /// (`CNormalProcess::InitializeProcess`, `MYWM_SETCARETPOS`).
    pub(crate) fn jump_in_open_file(&self, view: &Entity<EditorView>, cx: &mut App) {
        if self.file.caret_y.is_some() {
            let caret = FileOptions {
                view_left: None,
                view_top: None,
                ..self.file.clone()
            };
            let scan = view
                .read(cx)
                .document()
                .read(cx)
                .scan_id()
                .filter(|_| view.read(cx).document().read(cx).is_scanning());
            if let Some(scan) = scan {
                // The line may be past what is scanned (ADR-0020): go there when the scan is done.
                let view = view.downgrade();
                cx.spawn(async move |cx| {
                    if wait_for_scan(&view, scan, cx).await {
                        view.update(cx, |view, cx| view.go_by_launch(&caret, true, cx))
                            .ok();
                    }
                })
                .detach();
                return;
            }
            view.update(cx, |view, cx| view.go_by_launch(&caret, true, cx));
        }
    }

    /// The document of a launch without a file, when its window is open. `-TYPE=` and `-CODE=`
    /// (`SetDocumentTypeWhenCreate`), `-R`, and then the macro.
    pub(crate) fn start_untitled(
        &self,
        handle: AnyWindowHandle,
        view: &Entity<EditorView>,
        cx: &mut App,
    ) {
        view.update(cx, |view, cx| {
            let forced = self
                .doc_type
                .as_deref()
                .map(|ext| view.apply_launch_type(ext, cx));
            if let Some(encoding) = self.file.encoding {
                let index = forced.unwrap_or(view.prefs.settings.type_index);
                let defaults = crate::doc_types::with_user(cx, |user| {
                    user.new_document_defaults_of_type(index)
                });
                let defaults = if encoding == defaults.encoding {
                    defaults
                } else {
                    NewDocumentDefaults {
                        encoding,
                        bom: encoding.bom_default_on(),
                        eol: Eol::CrLf,
                        mime_decode: defaults.mime_decode,
                    }
                };
                view.document()
                    .update(cx, |shared, _| shared.set_new_document_defaults(&defaults));
            }
            if self.file.view_mode {
                view.document()
                    .update(cx, |shared, cx| shared.set_view_mode(true, cx));
            }
        });
        // The macro of オープン後 comes before the macro of `-M=` (`CNormalProcess`).
        let macro_ = self.macro_.clone();
        let view = view.clone();
        crate::macros::run_auto_in(
            handle,
            &view.clone(),
            crate::macros::Auto::Opened,
            cx,
            move |cx| {
                if let Some(m) = &macro_ {
                    run_macro(m, handle, &view, cx);
                }
            },
        );
    }

    /// The Grep of `-GREPMODE` in the window `handle` that was opened for it, or the dialog of
    /// `-GREPDLG`.
    pub(crate) fn start_grep(
        &self,
        handle: AnyWindowHandle,
        view: &Entity<EditorView>,
        cx: &mut App,
    ) {
        let Some(grep) = &self.grep else {
            return;
        };
        if grep.dialog {
            // `-GREPDLG`: the key, the file and the folder of the command line go into the
            // histories before the dialog opens (`CNormalProcess::InitializeProcess`).
            if !grep.args.key.is_empty() {
                crate::search::history::remember_search(cx, &grep.args.key);
            }
            crate::search::history::remember_grep_boxes(
                cx,
                &crate::search::history::GrepBoxes {
                    file: &grep.args.file,
                    folder: &grep.args.folder,
                    exclude_file: "",
                    exclude_folder: "",
                },
            );
            let remembered = crate::grep::window::remembered(cx);
            let settings = grep_dialog_settings(&grep.args, &self.cwd, remembered);
            handle
                .update(cx, |_, window, cx| {
                    view.update(cx, |view, cx| {
                        view.open_grep_panel_with(settings, window, cx)
                    });
                })
                .ok();
            return;
        }
        let clipboard = grep
            .args
            .paste
            .then(|| crate::grep::window::clipboard_replacement(cx))
            .flatten();
        if grep.args.paste && grep.args.replace.is_some() && clipboard.is_none() {
            // `CGrepAgent::DoGrep`: the clipboard has no text
            view.update(cx, |view, cx| {
                view.notice = Some(crate::grep::panel::NO_CLIPBOARD.to_owned());
                cx.notify();
            });
            return;
        }
        let info = grep_info(&grep.args, &self.cwd, clipboard);
        handle
            .update(cx, |_, window, cx| {
                view.update(cx, |view, cx| {
                    // `DoGrep` is called directly: no `Command_GREP`, so no type change.
                    view.start_grep_in(info, false, false, window, cx);
                });
            })
            .ok();
    }
}

/// What to do to a document when its load is over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AfterLoad {
    file: FileOptions,
    doc_type: Option<String>,
    macro_: Option<MacroLaunch>,
}

/// The loads that wait for their file, by path. A launch notes what it wants of a file before the
/// window begins to read it; `load::load_here` asks for it when the read is over.
#[derive(Default)]
struct Expected(HashMap<PathBuf, AfterLoad>);

impl Global for Expected {}

/// The next load of `path` is followed by `after`.
pub(crate) fn expect(cx: &mut App, path: &Path, after: AfterLoad) {
    cx.default_global::<Expected>()
        .0
        .insert(path.to_owned(), after);
}

/// The `-TYPE=` that the launch waiting for `path` asked for. The type is chosen when the file
/// has been read and before the macro of オープン後 runs, as Sakura does with
/// `SetDocumentTypeWhenCreate` (`sLoadInfo.nType`, `CLoadAgent::OnLoad`), so that the macro
/// sees it.
pub(crate) fn expected_type(cx: &App, path: &Path) -> Option<String> {
    cx.try_global::<Expected>()?.0.get(path)?.doc_type.clone()
}

/// The request for `path` is not needed any more (the window could not be made).
pub(crate) fn forget(cx: &mut App, path: &Path) {
    cx.default_global::<Expected>().0.remove(path);
}

/// The read of `path` into `view` is over; `loaded` says whether the document is in the view.
/// Does what the launch asked: the type, the place of the view and the caret, then the macro
/// (which runs also when the file could not be read: `CNormalProcess` goes on with an untitled
/// document).
pub(crate) fn after_load(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    path: &Path,
    loaded: bool,
) {
    cx.update(|window, app| apply_after_load(view, path, loaded, window, app))
        .ok();
}

/// [`after_load`] where the window is in hand.
fn apply_after_load(
    view: &WeakEntity<EditorView>,
    path: &Path,
    loaded: bool,
    window: &mut Window,
    app: &mut App,
) {
    let Some(after) = app.default_global::<Expected>().0.remove(path) else {
        return;
    };
    let Some(view) = view.upgrade() else {
        return;
    };
    if loaded {
        // `-TYPE=` was applied when the file was read (see `expected_type`).
        view.update(app, |view, cx| view.go_by_launch(&after.file, false, cx));
    }
    if let Some(m) = &after.macro_ {
        run_macro_in(m, &view, window, app);
    }
}

/// The macro of オープン後 and then the options of the launch (`-X=` `-Y=` `-M=`...), for a file
/// that is scanned in the background (ADR-0020): they wait for the end of the scan, because a
/// place in the file may be one the scan has not reached, and the macro may move there. Nothing
/// waits for them; the next file can be opened meanwhile.
pub(crate) fn after_load_when_scanned(
    view: &Entity<EditorView>,
    path: &Path,
    hold: Option<crate::macros::wait::FlowGuard>,
    cx: &mut App,
) {
    // The task holds the view weakly: a tab that is closed meanwhile ends the scan with its
    // document, and the options are dropped.
    let (view, path) = (view.downgrade(), path.to_owned());
    let Some(scan) = view
        .upgrade()
        .and_then(|view| view.read(cx).document().read(cx).scan_id())
    else {
        return;
    };
    cx.spawn(async move |cx| {
        let finished = wait_for_scan(&view, scan, cx).await;
        cx.update(|app| {
            // Only for the document that was opened: it was closed or replaced meanwhile, or the
            // scan failed, and the options were not for what is there.
            let tab = finished
                .then(|| view.upgrade())
                .flatten()
                .and_then(|entity| {
                    // The tab may have moved to another window meanwhile.
                    crate::file_ops::windows::find_tab_of(app, &view)
                        .map(|(handle, _)| (handle, entity))
                });
            let Some((handle, entity)) = tab else {
                app.default_global::<Expected>().0.remove(&path);
                return;
            };
            crate::macros::run_auto_in(
                handle,
                &entity,
                crate::macros::Auto::Opened,
                app,
                move |app| {
                    if let Some((handle, _)) = crate::file_ops::windows::find_tab_of(app, &view) {
                        handle
                            .update(app, |_, window, app| {
                                apply_after_load(&view, &path, true, window, app)
                            })
                            .ok();
                    }
                    // A macro that opened the file goes on when the macros of the opening are over.
                    drop(hold);
                },
            );
        });
    })
    .detach();
}

/// Waits until the scan of number `scan` of the document of `view` is over. Whether it ended with
/// the complete document (not failed, and the view and its document not gone meanwhile).
async fn wait_for_scan(view: &WeakEntity<EditorView>, scan: u64, cx: &mut gpui::AsyncApp) -> bool {
    loop {
        let state = view.read_with(cx, |view, app| {
            let shared = view.document().read(app);
            if shared.scan_finished(scan) {
                Some(true)
            } else if shared.scan_id() != Some(scan) || !shared.is_scanning() {
                Some(false)
            } else {
                None
            }
        });
        match state {
            Ok(Some(done)) => return done,
            Ok(None) => {}
            Err(_) => return false,
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(50))
            .await;
    }
}

/// `-M=`: runs the macro in the window `handle` (`F_EXECEXTMACRO`).
fn run_macro(m: &MacroLaunch, handle: AnyWindowHandle, view: &Entity<EditorView>, cx: &mut App) {
    handle
        .update(cx, |_, window, cx| run_macro_in(m, view, window, cx))
        .ok();
}

/// [`run_macro`] where the window is in hand already.
fn run_macro_in(m: &MacroLaunch, view: &Entity<EditorView>, window: &mut Window, cx: &mut App) {
    view.update(cx, |view, cx| match m {
        MacroLaunch::File(path) => {
            view.exec_ext_macro(path, window, cx).ok();
        }
        MacroLaunch::Text { language, code } => {
            view.exec_macro_text(code, language, window, cx).ok();
        }
    });
}

impl EditorView {
    /// `-TYPE=ext`: the document takes the type of the extension and keeps it
    /// (`SetDocumentType` and `LockDocumentType`). Returns the index of the type.
    pub(crate) fn apply_launch_type(&mut self, ext: &str, cx: &mut Context<Self>) -> usize {
        let index =
            crate::doc_types::with_user(cx, |user| user.document_types().type_index_for_ext(ext));
        self.change_type(index as i64 + 1, cx);
        index
    }

    /// The view and the caret go where `-VX=` `-VY=` and `-X=` `-Y=` say. The view goes first,
    /// and only when both are given and the row exists; the caret goes to the line (the end of
    /// the text if the line is past it), at the column of `-X=` in UTF-16 code units, and not
    /// past the end of the line. A column that is not given is 0, or, with `keep_column`, the
    /// column the caret has.
    pub(crate) fn go_by_launch(
        &mut self,
        options: &FileOptions,
        keep_column: bool,
        cx: &mut Context<Self>,
    ) {
        if let (true, Some(top), Some(left)) =
            (options.moves_view(), options.view_top, options.view_left)
            && top < self.layout.row_count()
        {
            self.scroll_top_to(top as isize, cx);
            self.scroll_x = left as f32 * self.metrics.cell_width;
        }
        if !options.moves_caret() {
            return;
        }
        let caret = self.cursor.caret();
        let doc = self.document().read(cx).doc();
        let line = options
            .caret_y
            .unwrap_or(if keep_column { caret.line } else { 0 });
        let target = if line >= doc.line_count() {
            doc.end_pos().unwrap_or_default()
        } else {
            let units = match options.caret_x {
                Some(x) => x,
                None if keep_column => doc.utf16_col(caret.line, caret.col),
                None => 0,
            };
            Pos::new(line, doc.col_from_utf16_col(line, units))
        };
        self.go_to(target, cx);
    }
}

// ---- Grep ----

/// The conditions of `-GREPMODE` as a Grep (`GrepInfo` of `CCommandLine`). Relative folders are in
/// the folder of the launch. `clipboard` is the text for `-GOPT=C`.
pub(crate) fn grep_info(args: &GrepArgs, cwd: &Path, clipboard: Option<String>) -> GrepInfo {
    GrepInfo {
        key: args.key.clone(),
        file: args.file.clone(),
        folder: absolute_folders(&args.folder, cwd),
        options: SearchOptions {
            match_case: args.match_case,
            whole_word: args.whole_word,
            regex: args.regex,
            ..SearchOptions::default()
        },
        sub_folder: args.sub_folder,
        charset: grep_charset(args.charset),
        layout: Layout {
            style: OutputStyle::from_number(args.output_style),
            line_type: LineType::from_number(args.output_line_type),
            base_folder: args.base_folder,
            separate_folder: args.separate_folder,
        },
        file_only: args.file_only,
        except_file_regex: args.except_file_regex,
        mime_decode: false,
        window: None,
        replace: args.replace.as_ref().map(|text| ReplaceInfo {
            text: if args.paste {
                clipboard.clone().unwrap_or_default()
            } else {
                text.clone()
            },
            paste: args.paste,
            backup: args.backup,
        }),
    }
}

/// `-GCODE=`: Sakura's number of the encoding of the files; 99 (and a number that is none) is the
/// detection.
fn grep_charset(code: i32) -> Option<Encoding> {
    if code == AUTODETECT_CODE {
        None
    } else {
        Encoding::from_sakura_code(code)
    }
}

/// The folders of `-GFOLDER=` (separated by `;`) with the relative ones in `cwd`.
fn absolute_folders(folders: &str, cwd: &Path) -> String {
    split_folders(folders)
        .into_iter()
        .map(|folder| {
            let folder = if is_relative(&folder) && !cwd.as_os_str().is_empty() {
                cwd.join(&folder).to_string_lossy().into_owned()
            } else {
                folder
            };
            quote_folder(&folder)
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// `-GREPDLG`: the dialog with the conditions of the command line over what the last Grep
/// left (`CNormalProcess::InitializeProcess`): the condition, the files and the folder (the
/// folder of the launch if none is given), and 下位フォルダー, the case, the regular expression and
/// the word option, the encoding, and the form of the result. The patterns of `-GFILE=` that are
/// excluded (`!file`, `#folder`) go to the boxes of the excluded files and folders when there
/// are any.
pub(crate) fn grep_dialog_settings(
    args: &GrepArgs,
    cwd: &Path,
    mut settings: GrepSettings,
) -> GrepSettings {
    settings.key = args.key.clone();
    let (mut files, mut exclude_folders, mut exclude_files) = (Vec::new(), Vec::new(), Vec::new());
    for pattern in split_pattern_keep_quotes(&args.file) {
        match pattern.strip_prefix('#') {
            Some(folder) => exclude_folders.push(folder),
            None => match pattern.strip_prefix('!') {
                Some(file) => exclude_files.push(file),
                None => files.push(pattern),
            },
        }
    }
    settings.file = files.join(";");
    if !exclude_folders.is_empty() {
        settings.exclude_folder = exclude_folders.join(";");
    }
    if !exclude_files.is_empty() {
        settings.exclude_file = exclude_files.join(";");
    }
    settings.folder = if args.folder.is_empty() {
        // 2013.05.21: not given means the current folder
        let current = if cwd.as_os_str().is_empty() {
            std::env::current_dir().unwrap_or_default()
        } else {
            cwd.to_owned()
        };
        quote_folder(&current.to_string_lossy())
    } else {
        absolute_folders(&args.folder, cwd)
    };
    settings.sub_folder = args.sub_folder;
    settings.options = SearchOptions {
        match_case: args.match_case,
        whole_word: args.whole_word,
        regex: args.regex,
        ..settings.options
    };
    settings.charset = grep_charset(args.charset);
    settings.line_type = LineType::from_number(args.output_line_type);
    settings.style = OutputStyle::from_number(args.output_style);
    settings
}

// ---- the window ----

/// The place for the next window that opens, for the one launch that asked for it.
#[derive(Default)]
struct NextPlacement(Option<Placement>);

impl Global for NextPlacement {}

/// The next new window gets this size and place. Call [`take_next_placement`] after the opening
/// that may or may not make a window (a tab does not).
pub(crate) fn set_next_placement(cx: &mut App, placement: Placement) {
    if !placement.is_empty() {
        cx.set_global(NextPlacement(Some(placement)));
    }
}

/// The placement that was set and not used.
pub(crate) fn take_next_placement(cx: &mut App) -> Option<Placement> {
    cx.try_global::<NextPlacement>()?;
    cx.global_mut::<NextPlacement>().0.take()
}

#[cfg(test)]
mod tests;
