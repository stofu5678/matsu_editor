//! The `SmartIndent` jack of the plugins (ADR-0026, chapter 3 and the ninth PR).
//!
//! Sakura calls the plug of the type's smart indent for every char that `Command_WCHAR` inserts,
//! right after the insertion (`sakura_core/cmd/CViewCommander_Edit.cpp:163-196`), and the plugin
//! decides by itself whether the char is a line break (`implementation03.html`: the column is 1).
//! The plugin gets the typed char from the `Indent` object (`CSmartIndentIfObj`, `GetChar`). What it
//! does is an undo step of its own, apart from the typed char (`SetUndoBuffer()` before the call and a
//! block of its own around it); the actions of the plugin are not recorded in a key macro.
//!
//! MATSU keeps that and changes when the plugin is called (ADR-0026, chapter 3, the fifth decision):
//!
//! * for a line break (Enter), and for the chars the plugin declares in `plugin.def` under
//!   `[Matsu] SmartIndent.Chars=`; not for the other chars, so no plugin code runs for most key
//!   presses;
//! * only for a resident plugin ([`Jack::needs_resident`](matsu_config::plugins::def::Jack)), because
//!   a key press cannot wait for a process to start;
//! * with the time limit of the jack (0.5 seconds), and a plugin that is not done by then adds
//!   nothing more; what was typed is in the text already, as the call runs apart from the window.
//!
//! The command that typed the char does not wait for the plugin (it runs in another process):
//! `input_char` says that the plugin is due ([`Changes::smart_indent_plugin`]) and the window starts
//! the call once the command is over. A char that comes while the call runs is typed as usual, so no
//! input is lost. The place the plugin was called for (the text, the caret and the selection) is
//! watched ([`EditorView::indent_guard_broken`]): when the user changes it the plugin is stopped, on
//! the next turn of the call's loop or at its next request, and says nothing; what the plugin did
//! until then is an undo step that ends before the user's input
//! ([`EditorView::cut_indent_undo_step`]). A plugin that is still busy with an earlier char does
//! not get the next one (the call is dropped, as ADR-0026 says of the calls that are of no use late).
//!
//! [`Changes::smart_indent_plugin`]: matsu_commands::context::Changes::smart_indent_plugin

use gpui::{App, Context, Entity};
use matsu_commands::input_options::SmartIndentCall;
use matsu_commands::plugin_commands;
use matsu_config::plugins::def::Jack;
use matsu_config::plugins::{DEFAULT_LANGUAGE, PluginDef, PluginSettings, PluginState, SearchDirs};
use matsu_plugin::json;

use super::{Prepared, Target, prepare, run};
use crate::view::EditorView;

/// Whether the plugin wants the `SmartIndent` jack after `typed`: a line break always (`line_break`
/// is what the command that typed it took for one), any other char only when
/// `[Matsu] SmartIndent.Chars` names it.
pub(crate) fn wants(def: &PluginDef, typed: char, line_break: bool) -> bool {
    line_break || def.smart_indent_chars.contains(typed)
}

/// The call for `call`, or `None` when there is nothing to run: the type's plug is not there any
/// more, the plugin is off, not agreed to, stopped, not resident, or does not want this char. A key
/// press says nothing about it; the list of the plugins shows the state.
pub(crate) fn prepare_smart_indent(
    cx: &mut App,
    dirs: &SearchDirs,
    call: SmartIndentCall,
) -> Option<Prepared> {
    let (slot, plug) = plugin_commands::decode(call.code)?;
    let settings =
        crate::doc_types::with_user(cx, |user| user.profile().map(PluginSettings::from_profile))?;
    if !settings.enabled {
        return None;
    }
    let rec = settings
        .table
        .get(slot)
        .filter(|rec| rec.is_used() && rec.state != PluginState::Deleted && !rec.name.is_empty())?;
    let def = dirs.load_def(&rec.name, DEFAULT_LANGUAGE)?;
    // The slot may hold another plugin by now than the one the type was set to.
    if def.id != rec.id || !wants(&def, call.typed, call.line_break) {
        return None;
    }
    let handler = def
        .plugs
        .iter()
        .find(|p| p.jack == Jack::SmartIndent && p.index == plug)?
        .handler
        .clone();
    let prepared = prepare(
        cx,
        dirs,
        &rec.name,
        Target::Event {
            jack: Jack::SmartIndent,
            handler,
        },
    )
    .ok()?;
    // `Indent.GetChar()` answers with this.
    Some(prepared.with_args(json!({ "char": call.typed.to_string() })))
}

impl EditorView {
    /// Calls the plugin of the type's smart indent for the char just typed, once the command is
    /// over. Not while a macro or another call of a plugin has the window (the actions of those are
    /// not typing).
    pub(crate) fn call_smart_indent_plugin(
        &mut self,
        call: SmartIndentCall,
        cx: &mut Context<Self>,
    ) {
        if self.plugin_in_call(cx) || self.macros.busy() {
            return;
        }
        let editor = cx.entity();
        // The call needs this view, which the command that asked for it is still in.
        cx.defer(move |cx| start(cx, editor, call));
    }

    /// The text, the caret or the selection is not as the call of the `SmartIndent` jack left it: the
    /// user went on.
    pub(crate) fn indent_guard_broken(&self, cx: &App) -> bool {
        self.macros
            .indent_guard
            .as_ref()
            .is_some_and(|(version, cursor)| {
                self.macros.indent_closed
                    || self.document.read(cx).version() != *version
                    || self.cursor != *cursor
            })
    }

    /// Stops the call of the `SmartIndent` jack, without a word, if the user went on. Returns whether it is
    /// to be stopped (the caller does it).
    pub(crate) fn stop_indent_if_moved(&mut self, cx: &App) -> bool {
        let moved = self.indent_guard_broken(cx);
        if moved {
            self.macros.indent_stopped = true;
        }
        moved
    }

    /// The call of the `SmartIndent` jack has done a request: its place is where that left it.
    pub(crate) fn indent_guard_follow(&mut self, cx: &App) {
        if self.macros.indent_guard.is_some() {
            self.macros.indent_guard =
                Some((self.document.read(cx).version(), self.cursor.clone()));
        }
    }

    /// The user does something (a command, typed text, an IME composition) while the call of the
    /// `SmartIndent` jack runs, on this view or on another that shows the document: the undo block of
    /// the call is closed now, so that one Undo takes the plugin's part and what the user does is
    /// outside of it. The call is stopped on the next turn of its loop.
    pub(crate) fn cut_indent_undo_step(&mut self, cx: &mut Context<Self>) {
        // Only a call has an open undo block that the document shares; most of the time none runs.
        if self.document.read(cx).plugin_busy() {
            let me = cx.entity_id();
            let others: Vec<_> = self
                .document
                .read(cx)
                .views()
                .filter(|other| other.entity_id() != me)
                .collect();
            for other in others {
                other.update(cx, |view, cx| view.cut_own_indent_undo_step(cx));
            }
        }
        self.cut_own_indent_undo_step(cx);
    }

    fn cut_own_indent_undo_step(&mut self, cx: &mut Context<Self>) {
        if self.macros.indent_guard.is_none() || self.macros.indent_closed {
            return;
        }
        self.macros.indent_closed = true;
        if let Some(depth) = self.macros.indent_undo_depth.take() {
            self.leave_undo_block(Some(depth), cx);
        }
    }
}

fn start(cx: &mut App, editor: Entity<EditorView>, call: SmartIndentCall) {
    let Some(dirs) = crate::plugin_commands::search_dirs(cx) else {
        return;
    };
    let Some(prepared) = prepare_smart_indent(cx, &dirs, call) else {
        return;
    };
    // The host serves one call at a time; a smart indent that waits would come when the user has
    // typed on, so it is dropped (ADR-0026, chapter 2).
    if prepared.host_busy() {
        return;
    }
    let begun = editor.update(cx, |view, cx| {
        if view.plugin_in_call(cx) || view.macros.busy() {
            return false;
        }
        view.macros.indent_guard = Some((view.document.read(cx).version(), view.cursor.clone()));
        view.macros.indent_closed = false;
        true
    });
    if begun && !run(editor.clone(), prepared, cx) {
        editor.update(cx, |view, _| view.macros.indent_guard = None);
    }
}

#[cfg(test)]
mod tests;
