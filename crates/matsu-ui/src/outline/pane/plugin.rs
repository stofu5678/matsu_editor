//! The outline of a plugin in the pane (the jack `Outline`, ADR-0026).
//!
//! Sakura asks the plug once when the outline is analyzed (`Command_FUNCLIST`, the `default:` branch,
//! `sakura_core/cmd/CViewCommander_Outline.cpp`) and waits for it. Here the call runs while the window
//! goes on painting; the pane shows the result when the call is over, or an empty list and the reason
//! when it failed or ran out of time (ADR-0026, chapter 2: the built-in analysis does not take its place).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, Entity};
use matsu_core::buffer::Pos;
use matsu_outline::{Analysis, FuncInfoArr, OutlineKind};
use matsu_plugin::plugin_host::limit_of;

use super::{HUGE_FILE_MESSAGE, OutlinePane};
use crate::macros::plugin_call::{self, Turn};
use crate::plugin_outline::{self, Collected, Ending};
use crate::view::EditorView;

/// How often the pane looks again while it waits.
const TICK: Duration = Duration::from_millis(10);

/// How often it looks while the host of the plugin is busy with another window (looking prepares the
/// call again, which reads the files of the plugin).
const HOST_BUSY_TICK: Duration = Duration::from_millis(50);

impl OutlinePane {
    /// Whether an item with the file name `file` is in a file other than the document of the pane
    /// (`CDlgFuncList::OnJump`: an item with a file name jumps to that file). The files are compared
    /// the way the windows are (`same_file`: the real paths, so case matters only where the file
    /// system says so).
    pub(super) fn is_in_another_file(&self, file: &str, cx: &gpui::App) -> bool {
        if file.is_empty() {
            return false;
        }
        let this = self.document.read(cx).path();
        this.is_none_or(|path| {
            !crate::file_ops::windows::same_file(path, std::path::Path::new(file))
        })
    }

    /// The place of an item whose line and column are counted from 1, the column in the UTF-16 units of
    /// the line (`AddFuncInfo`; the same count as `Editor.MoveCursor`). The analyzers of MATSU give
    /// column 1.
    pub(super) fn item_position(&self, line: usize, col: usize, cx: &gpui::App) -> Pos {
        let row = line.saturating_sub(1);
        let skip = col.saturating_sub(1);
        if skip == 0 {
            return Pos::new(row, 0);
        }
        let text = self
            .document
            .read(cx)
            .doc()
            .line(row)
            .ok()
            .flatten()
            .map(|line| line.text)
            .unwrap_or_default();
        let body = text.trim_end_matches(['\r', '\n']);
        Pos::new(row, matsu_plugin::api::byte_offset_for_utf16(body, skip))
    }

    /// Analyzes the document of the pane with the plug whose number is `code`. `generation` and
    /// `version` are those of the analysis that began (see [`OutlinePane::analyze`]).
    pub(super) fn analyze_plugin(
        &mut self,
        code: i32,
        generation: u64,
        version: u64,
        file_path: String,
        cx: &mut Context<Self>,
    ) {
        // A file read by ranges has no outline, the plugins' included (ADR-0020, 9.11): the plug would
        // read the whole file.
        let huge = self.document.read(cx).is_ranged();
        let view = self.view.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let analysis = if huge {
                Some(Analysis {
                    items: FuncInfoArr::new(),
                    kind: OutlineKind::List,
                    title: None,
                    error: Some(HUGE_FILE_MESSAGE.to_owned()),
                })
            } else {
                run_plug(&this, &view, code, generation, cx).await
            };
            // `None`: the pane was given up (another analysis began, or it is gone).
            let Some(analysis) = analysis else {
                return;
            };
            this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                this.finish(analysis, file_path, cx);
                // The text changed while the plug ran: the result is of the text it saw. Sakura waits for
                // the plug, so it never meets this; the pane keeps the result and says it is old, rather
                // than calling the plug again (it is called when the outline is analyzed, not per edit).
                if this.document.read(cx).version() != version {
                    this.stale = true;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }
}

/// Waits for the window to be free of other calls of plugins and macros, runs the plug, and waits for
/// its end. `None` when the pane stopped waiting for it.
async fn run_plug(
    pane: &gpui::WeakEntity<OutlinePane>,
    view: &gpui::WeakEntity<EditorView>,
    code: i32,
    generation: u64,
    cx: &mut gpui::AsyncApp,
) -> Option<Analysis> {
    let given_up = |cx: &mut gpui::AsyncApp| {
        pane.read_with(cx, |pane, _| pane.generation != generation)
            .unwrap_or(true)
    };
    let collected = Rc::new(RefCell::new(Collected::new().in_view(view.entity_id())));
    // The window has one call at a time, and so has the host of a plugin (another window may be using
    // it): an event of a plugin, a command or a macro that runs goes first. The calls have their time
    // limits, so the line moves.
    loop {
        if given_up(cx) {
            return None;
        }
        let entity: Entity<EditorView> = view.upgrade()?;
        match cx.update(|app| begin(app, entity, code, &collected)) {
            Begin::Started => break,
            Begin::Refused(reason) => {
                collected.borrow_mut().ending = Some(Ending::Refused(reason));
                break;
            }
            Begin::Wait(time) => cx.background_executor().timer(time).await,
        }
    }
    // The call has its own time limit ([`limit_of`]); the host kills the process at it.
    while collected.borrow().ending.is_none() {
        if given_up(cx) {
            return None;
        }
        cx.background_executor().timer(TICK).await;
    }
    let analysis = collected.borrow().to_analysis(limit_of("Outline"));
    Some(analysis)
}

/// What the attempt to start the call came to.
enum Begin {
    /// The call runs.
    Started,
    /// It will not run; the reason.
    Refused(String),
    /// Not now: wait this long and try again.
    Wait(Duration),
}

/// Starts the call if the window and the host of the plugin are free.
fn begin(
    cx: &mut gpui::App,
    view: Entity<EditorView>,
    code: i32,
    collected: &Rc<RefCell<Collected>>,
) -> Begin {
    if view.read(cx).event_turn(cx) != Turn::Free {
        return Begin::Wait(TICK);
    }
    let Some(plug) = plugin_outline::find(cx, code) else {
        return Begin::Refused(
            "このアウトラインのプラグインは、今は使えません（無効か、登録されていません）"
                .to_owned(),
        );
    };
    collected.borrow_mut().label.clone_from(&plug.folder);
    let Some(dirs) = crate::plugin_commands::search_dirs(cx) else {
        return Begin::Refused("設定ファイルの場所が分かりません".to_owned());
    };
    let prepared = match plugin_call::prepare_outline(
        cx,
        &dirs,
        &plug.folder,
        &plug.handler,
        collected.clone(),
    ) {
        Ok(prepared) => prepared,
        Err(reason) => return Begin::Refused(reason),
    };
    // One host serves all the windows: a call of the same plugin on another window goes first.
    if prepared.host_busy() {
        return Begin::Wait(HOST_BUSY_TICK);
    }
    if plugin_call::run(view, prepared, cx) || collected.borrow().ending.is_some() {
        // (a call refused at its start has told why in the result)
        Begin::Started
    } else {
        Begin::Wait(TICK)
    }
}
