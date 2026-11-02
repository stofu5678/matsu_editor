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
use matsu_outline::{Analysis, FuncInfoArr, OutlineKind};
use matsu_plugin::plugin_host::limit_of;

use super::{HUGE_FILE_MESSAGE, OutlinePane};
use crate::macros::plugin_call::{self, Turn};
use crate::plugin_outline::{self, Collected, Ending};
use crate::view::EditorView;

/// How often the pane looks again while it waits.
const TICK: Duration = Duration::from_millis(10);

impl OutlinePane {
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
    // The window has one call at a time: an event of a plugin, a command or a macro that runs goes first
    // (the calls have their time limits, so the line moves).
    loop {
        if given_up(cx) {
            return None;
        }
        match view.read_with(cx, |view, app| view.event_turn(app)).ok()? {
            Turn::Free => break,
            Turn::AfterEvent | Turn::Taken => cx.background_executor().timer(TICK).await,
        }
    }
    let entity: Entity<EditorView> = view.upgrade()?;
    let collected = Rc::new(RefCell::new(Collected::new()));
    let started = cx.update(|app| begin(app, entity, code, &collected));
    if let Err(reason) = started {
        collected.borrow_mut().ending = Some(Ending::Refused(reason));
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

/// Starts the call. `Err`: why it did not begin.
fn begin(
    cx: &mut gpui::App,
    view: Entity<EditorView>,
    code: i32,
    collected: &Rc<RefCell<Collected>>,
) -> Result<(), String> {
    let plug = plugin_outline::find(cx, code)
        .ok_or("このアウトラインのプラグインは、今は使えません（無効か、登録されていません）")?;
    collected.borrow_mut().label.clone_from(&plug.folder);
    let dirs = crate::plugin_commands::search_dirs(cx).ok_or("設定ファイルの場所が分かりません")?;
    let prepared =
        plugin_call::prepare_outline(cx, &dirs, &plug.folder, &plug.handler, collected.clone())?;
    if !plugin_call::run(view, prepared, cx) && collected.borrow().ending.is_none() {
        return Err("ほかのプラグインかマクロが動いているため、実行できませんでした".to_owned());
    }
    Ok(())
}
