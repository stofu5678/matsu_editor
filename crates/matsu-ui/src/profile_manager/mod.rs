//! プロファイルマネージャ: the dialog of `F_PROFILEMGR` and of `-PROFMGR`
//! (`CDlgProfileMgr`, `sakura_core/dlg/CDlgProfileMgr.cpp`; the dialog is `IDD_PROFILEMGR` in
//! `sakura_core/sakura_rc.rc`, the texts are `STR_DLGPROFILE_*` in the same file).
//!
//! A profile is a named set of settings: `-PROF=name` runs the editor with the settings file in the
//! folder `name` next to the usual one (`matsu_config::location`). The dialog keeps the list of the
//! names in `matsu_prof.ini` (`matsu_config::profile_manager` has the file, the rules for the
//! names and the decision when the dialog is shown at the start).
//!
//! * The list has 「(default)」 (the profile without a name) and the names; the row with the star is
//!   the one that starts when no `-PROF` is given (`nDefaultIndex`).
//! * 新規作成 and 名前変更 ask for a name (`CDlgInput1`: here a small panel in the window of the
//!   dialog, because the dialog is a window of its own and has no editor view to cover). The name is
//!   checked (`NameError`), and 名前変更 moves the folder of the profile when there is one. 削除 only
//!   takes the name out of the list: the folder and its settings stay (`DeleteProf`).
//! * Every change of the list is written to the file at once (`UpdateIni`). 起動 (OK) returns the
//!   chosen profile; with 「デフォルト設定にして起動」 checked it first makes the row the default one.
//!   閉じる (Cancel, Esc and the close button of the window) returns nothing and writes only the
//!   check box when it changed (`GetData(false)`).
//! * 削除 and 名前変更 are not for 「(default)」; デフォルト解除 is only for a list that has a default.
//! * The only list action of the mouse is the click that chooses a row (a double click does
//!   nothing: `LBN_DBLCLK` is not handled in Sakura).
//! * The help is the page of the profile manager (`F_PROFILEMGR`'s `HLP000363`).
//!
//! The dialog is a window of its own that is not modal (GPUI has no modal child window); one is
//! open at a time. At the start of the process, [`startup`] shows it in a process of its own, so that
//! the profile is known before the settings file is read.

use std::rc::Rc;

use crate::a11y::Labelled as _;
use crate::access_keys::{Entry, Found, OnAccessKey as _, pick};
use crate::input_dialog::{DialogBackTab, DialogTab, cycle_focus, limit_edit};
use crate::input_select::{focus_selecting, input_field};
use crate::support::help::OnHelpKey as _;
use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, ClickEvent, Context, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, PromptLevel, Render,
    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Subscription, Window,
    WindowBounds, WindowOptions, div, px, rgb, rgba, size,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{Disableable as _, Root, h_flex, v_flex};
use matsu_config::profile_manager::{MAX_NAME_LEN, NameError, ProfileSettings};
use matsu_plugin::functions::window::truncate_utf16;

pub(crate) mod in_use;
pub(crate) mod startup;
mod store;

use store::MoveError;
pub(crate) use store::Store;

const TEXT: u32 = 0x222222;
const BORDER: u32 = 0xaaaaaa;
const CHOSEN_BG: u32 = 0xcfe0f8;
const FOCUS_RING: u32 = 0x2f6fd0;

/// `IDD_PROFILEMGR`: the caption and the labels, with the access key as the dialogs write it.
const TITLE: &str = "プロファイルマネージャ";
const DEFSTART_LABEL: &str = "デフォルト設定にして起動(E)";
const START_LABEL: &str = "起動(S)";
const CLOSE_LABEL: &str = "閉じる(X)";
const HELP_LABEL: &str = "ヘルプ(H)";
const CREATE_LABEL: &str = "新規作成(N)";
const RENAME_LABEL: &str = "名前変更(R)";
const DELETE_LABEL: &str = "削除(D)";
const DEFSET_LABEL: &str = "デフォルト設定(F)";
const DEFCLEAR_LABEL: &str = "デフォルト解除(C)";
/// The list has no label of its own in the resource; this is its name for a screen reader.
const LIST_NAME: &str = "プロファイル一覧";

/// `STR_DLGPROFILE_*` (`sakura_rc.rc`).
pub(crate) const ERR_WRITE: &str = "プロフィール設定の書き込みに失敗しました。";
const NEW_TITLE: &str = "新規プロファイル名";
const NEW_MESSAGE: &str = "プロファイル名を入力してください";
pub(crate) const ERR_INVALID_CHAR: &str = "利用できない文字が含まれています。";
pub(crate) const ERR_ALREADY: &str = "すでに同じ名前のプロファイルがあります。";
pub(crate) const ERR_FILE: &str = "同名のファイルがあるため作成できません。";
const RENAME_TITLE: &str = "プロファイル名の変更";
const RENAME_MESSAGE: &str = "プロファイル名を入力してください";
pub(crate) const ERR_IN_USE: &str =
    "このプロファイルは使用中です。\n使用中のプロファイルの名前変更と削除はできません。";
pub(crate) const ERR_RENAME: &str =
    "名前の変更に失敗しました。\nプロファイルは利用中の可能性があります。";

/// The page that describes the dialog (the section of `F_PROFILEMGR`).
const HELP_PAGE: &str = "help-other/file-shell.md";

/// What the dialog does with the answer: the profile that 起動 chose (the empty name is the profile
/// without a name), or `None` for 閉じる.
pub(crate) type Done = Box<dyn FnOnce(&mut App, Option<String>)>;

/// What an access key does in the dialog.
#[derive(Clone, Copy)]
enum Use {
    DefaultStart,
    Start,
    Close,
    Help,
    Create,
    Rename,
    Delete,
    SetDefault,
    ClearDefault,
}

/// What the name panel is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asking {
    /// 新規作成.
    Create,
    /// 名前変更 of this row.
    Rename(usize),
}

/// The panel that asks for a name (`CDlgInput1`).
struct NameEntry {
    asking: Asking,
    title: &'static str,
    message: &'static str,
    input: Entity<InputState>,
    /// The text of the field after the last change, within the maximum length. A copy of what the field
    /// holds (ownership.md 2): the change event of the field does not say what was edited, so
    /// `limit_edit` needs the text from before to find the edit. It is made new with the panel and ends
    /// with it, and `limit_entry` is its only writer.
    accepted: String,
    _subscription: Subscription,
}

pub(crate) struct ProfileManagerDialog {
    store: Store,
    /// The list, the star and the check box. The file follows it (`UpdateIni`); this is the one
    /// copy that the window changes.
    settings: ProfileSettings,
    /// The row that is chosen (0 is 「(default)」).
    selected: usize,
    /// Whether this window changed the check box since it was last written. The file owns the value
    /// otherwise: a window that did not touch the box writes what the file has (ownership.md 2).
    box_changed: bool,
    scroll: ScrollHandle,
    entry: Option<NameEntry>,
    on_done: Option<Done>,
    focus: FocusHandle,
    list_focus: FocusHandle,
    entry_ok: FocusHandle,
    entry_cancel: FocusHandle,
}

impl ProfileManagerDialog {
    fn new(store: Store, on_done: Done, cx: &mut Context<Self>) -> Self {
        // `SetData`: a file that is not there is the default list, and the row that starts is chosen
        let settings = store.read().unwrap_or_default();
        let selected = usize::try_from(settings.default_index)
            .ok()
            .filter(|row| *row < settings.rows())
            .unwrap_or(0);
        let scroll = ScrollHandle::new();
        scroll.scroll_to_item(selected);
        Self {
            store,
            settings,
            selected,
            box_changed: false,
            scroll,
            entry: None,
            on_done: Some(on_done),
            focus: cx.focus_handle(),
            list_focus: cx.focus_handle().tab_stop(true),
            entry_ok: cx.focus_handle().tab_stop(true),
            entry_cancel: cx.focus_handle().tab_stop(true),
        }
    }

    /// The list as the dialog shows it.
    #[cfg(test)]
    pub(crate) fn settings(&self) -> &ProfileSettings {
        &self.settings
    }

    /// The row that is chosen.
    #[cfg(test)]
    pub(crate) fn selected(&self) -> usize {
        self.selected
    }

    /// Whether the panel that asks for a name is open.
    #[cfg(test)]
    pub(crate) fn asking(&self) -> bool {
        self.entry.is_some()
    }

    /// The row of the list is chosen (a click, the arrow keys). 削除 and 名前変更 follow the row
    /// (`LBN_SELCHANGE`).
    pub(crate) fn select(&mut self, row: usize, cx: &mut Context<Self>) {
        self.selected = row.min(self.settings.rows() - 1);
        self.scroll.scroll_to_item(self.selected);
        cx.notify();
    }

    /// 削除 and 名前変更 are for the profiles with a name.
    fn rename_usable(&self) -> bool {
        self.selected != 0
    }

    /// デフォルト解除 is for a list that has a default row.
    fn clear_usable(&self) -> bool {
        self.settings.default_index != -1
    }

    /// The check box 「デフォルト設定にして起動」. Nothing is written until the dialog is closed.
    pub(crate) fn set_default_start(&mut self, on: bool, cx: &mut Context<Self>) {
        self.settings.default_select = on;
        self.box_changed = true;
        cx.notify();
    }

    /// `ErrorMessage`: a box with the text and OK. A refusal without a text says nothing (Sakura
    /// ignores an empty name).
    fn tell(&self, failure: &Failure, window: &mut Window, cx: &mut Context<Self>) {
        if !failure.message.is_empty() {
            crate::boxes::tell(
                window,
                PromptLevel::Critical,
                failure.message,
                failure.detail.as_deref(),
                cx,
            );
        }
    }

    /// The list as it is on the disk now, with the check box of this window (`UpdateIni` writes the
    /// check box with every change). The window keeps no list of its own to write back: other windows
    /// and processes change the file too.
    fn fresh(&self) -> ProfileSettings {
        // A file that cannot be read (it is gone, or something is in its place) leaves the window's own
        // list: the window does not turn into an empty list that the next action would write.
        let mut list = self.store.read().unwrap_or_else(|| self.settings.clone());
        // the check box is this window's only when it changed it
        if self.box_changed {
            list.default_select = self.settings.default_select;
        }
        list
    }

    /// Shows `list`: the chosen row stays on the same profile when it is still there, else on the
    /// same place.
    fn adopt(&mut self, list: ProfileSettings, cx: &mut Context<Self>) {
        let name = self.settings.profile_of(self.selected).to_owned();
        self.selected = row_named(&list, &name)
            .unwrap_or(self.selected)
            .min(list.rows() - 1);
        self.settings = list;
        self.scroll.scroll_to_item(self.selected);
        cx.notify();
    }

    /// One action on the list, `change_with` without a second step.
    fn change(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        edit: impl FnOnce(&Store, &mut ProfileSettings) -> Result<(), Failure>,
    ) -> bool {
        self.change_with(window, cx, edit, |_, ()| Ok(()), |_| {})
    }

    /// One action on the list. The file is locked against the other windows and processes for the whole
    /// of it (`transact`); then the result is what the window shows. When it fails, the reason is told
    /// (after the lock is let go: no box is shown while it is held) and the window shows what the disk
    /// has. Returns whether the action was done.
    fn change_with<T>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        edit: impl FnOnce(&Store, &mut ProfileSettings) -> Result<T, Failure>,
        then: impl FnOnce(&Store, T) -> Result<(), Failure>,
        undo: impl FnOnce(&mut ProfileSettings),
    ) -> bool {
        match self.transact(edit, then, undo) {
            Ok(list) => {
                // the box was written with the list; the file owns it again
                self.box_changed = false;
                self.adopt(list, cx);
                true
            }
            Err(failure) => {
                self.tell(&failure, window, cx);
                let disk = self.fresh();
                self.adopt(disk, cx);
                false
            }
        }
    }

    /// Under the lock of the file: reads it, `edit` changes the one thing (and may refuse), the list is
    /// written, and `then` does what has to follow the write (a rename moves the folder). If `then` fails,
    /// the file is read again and `undo` applies the inverse of this one change to it (never a copy of an
    /// older list: another process cannot have written meanwhile because of the lock, but the rule does
    /// not rest on that) and writes it. Returns the list that was written.
    fn transact<T>(
        &self,
        edit: impl FnOnce(&Store, &mut ProfileSettings) -> Result<T, Failure>,
        then: impl FnOnce(&Store, T) -> Result<(), Failure>,
        undo: impl FnOnce(&mut ProfileSettings),
    ) -> Result<ProfileSettings, Failure> {
        let _lock = self
            .store
            .lock()
            .map_err(|error| Failure::because(ERR_WRITE, &error))?;
        let mut list = self.fresh();
        let value = edit(&self.store, &mut list)?;
        self.store
            .write(&list)
            .map_err(|error| Failure::because(ERR_WRITE, &error))?;
        if let Err(mut failure) = then(&self.store, value) {
            let mut back = self.fresh();
            undo(&mut back);
            if let Err(error) = self.store.write(&back) {
                let cause = failure.detail.take().unwrap_or_default();
                failure.detail = Some(format!("{cause}\n{ERR_WRITE}({error})"));
            }
            return Err(failure);
        }
        Ok(list)
    }

    // The panel that asks for a name

    fn open_entry(
        &mut self,
        asking: Asking,
        initial: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (title, message) = match asking {
            Asking::Create => (NEW_TITLE, NEW_MESSAGE),
            Asking::Rename(_) => (RENAME_TITLE, RENAME_MESSAGE),
        };
        let input = cx.new(|cx| input_field(window, cx));
        let initial = truncate_utf16(initial, MAX_NAME_LEN).to_owned();
        input.update(cx, |state, cx| state.set_value(initial.clone(), window, cx));
        let subscription = cx.subscribe_in(
            &input,
            window,
            |this: &mut Self, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.limit_entry(window, cx),
                InputEvent::PressEnter { .. } => this.commit_entry(window, cx),
                _ => {}
            },
        );
        focus_selecting(&input, window, cx);
        self.entry = Some(NameEntry {
            asking,
            title,
            message,
            input,
            accepted: initial,
            _subscription: subscription,
        });
        cx.notify();
    }

    /// Keeps the field within the maximum length (the edit control of Sakura's dialog refuses what
    /// is beyond it), as [`crate::input_dialog`] does.
    fn limit_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.entry.as_mut() else {
            return;
        };
        let text = entry.input.read(cx).value().to_string();
        match limit_edit(&entry.accepted, &text, MAX_NAME_LEN) {
            Some((kept, caret)) => {
                entry.accepted.clone_from(&kept);
                entry.input.update(cx, |state, cx| {
                    state.set_value(kept, window, cx);
                    state.set_selected_range(caret..caret, cx);
                });
            }
            None => entry.accepted = text,
        }
    }

    /// 新規作成: asks for a name.
    pub(crate) fn begin_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.entry.is_none() {
            self.open_entry(Asking::Create, "", window, cx);
        }
    }

    /// 名前変更: asks for the new name of the chosen row, which starts as it is now.
    pub(crate) fn begin_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.entry.is_none() && self.rename_usable() {
            let row = self.selected;
            let current = self.settings.profile_of(row).to_owned();
            self.open_entry(Asking::Rename(row), &current, window, cx);
        }
    }

    /// OK of the name panel: the name is used.
    pub(crate) fn commit_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.entry.take() else {
            return;
        };
        let text = entry.input.read(cx).value().to_string();
        let name = truncate_utf16(&text, MAX_NAME_LEN).to_owned();
        window.focus(&self.list_focus, cx);
        cx.notify();
        match entry.asking {
            Asking::Create => self.create(&name, window, cx),
            Asking::Rename(row) => self.rename(row, &name, window, cx),
        }
    }

    /// Cancel of the name panel: nothing changes.
    pub(crate) fn cancel_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.entry.take().is_some() {
            window.focus(&self.list_focus, cx);
            cx.notify();
        }
    }

    // What the buttons do

    /// `CreateProf`, after the name was asked. The chosen row stays.
    pub(crate) fn create(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.change(window, cx, |store, list| {
            check_name(list, name, None)?;
            if store.file_in_the_way(name) {
                return Err(Failure::new(ERR_FILE));
            }
            list.add(name);
            Ok(())
        });
    }

    /// `RenameProf`, after the name was asked, as one locked action (`transact`): the name is checked, a
    /// profile in use is refused, the new list is written, and only then the folder of the profile is
    /// moved. If the move fails, the rename is undone in the list as it is then (`undo_rename`), so the
    /// file never says more than the folders have. The row keeps its star.
    pub(crate) fn rename(
        &mut self,
        row: usize,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let old = self.settings.profile_of(row).to_owned();
        // nothing typed, or the same name: nothing happens and nothing is said
        if row == 0 || name.is_empty() || name == old {
            return;
        }
        self.change_with(
            window,
            cx,
            |store, list| {
                // a missing row: another window took the profile out of the list meanwhile
                let row = row_named(list, &old)
                    .filter(|row| *row != 0)
                    .ok_or_else(Failure::none)?;
                check_name(list, name, Some(row))?;
                // The check and the move are under the lock of the list, and an editor makes its mark under
                // the same lock before it reads anything of the profile (`in_use::mark_ordered`): either
                // the mark is there when this check looks, or the editor starts after the move. What is
                // left: an editor that could not take the lock (it marks without it and says so), and a
                // program that does not make the mark (the lock of the mark is advisory on macOS and
                // Linux). On Windows an open folder cannot be moved at all.
                if in_use_of(store, &old) {
                    return Err(Failure::new(ERR_IN_USE));
                }
                let plan = store.plan_rename(&old, name).map_err(|error| match error {
                    MoveError::Exists => {
                        Failure::because(ERR_RENAME, &"同じ名前のフォルダーがあります")
                    }
                    MoveError::FileInTheWay => Failure::new(ERR_FILE),
                })?;
                list.rename(row, name);
                Ok(plan)
            },
            |store, plan| match plan {
                Some(plan) => store
                    .move_dir(&plan)
                    .map_err(|error| Failure::because(ERR_RENAME, &error)),
                None => Ok(()),
            },
            |list| undo_rename(list, name, &old),
        );
    }

    /// `DeleteProf`: the chosen row leaves the list (its folder stays on the disk). A profile in use is
    /// refused.
    pub(crate) fn delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.settings.profile_of(self.selected).to_owned();
        if name.is_empty() {
            return;
        }
        self.change(window, cx, |store, list| {
            if in_use_of(store, &name) {
                return Err(Failure::new(ERR_IN_USE));
            }
            let row = row_named(list, &name).ok_or_else(Failure::none)?;
            list.remove(row);
            Ok(())
        });
    }

    /// デフォルト設定: the chosen row is the one that starts.
    pub(crate) fn make_default(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.settings.profile_of(self.selected).to_owned();
        self.change(window, cx, |_, list| {
            let row = row_named(list, &name).ok_or_else(Failure::none)?;
            list.set_default(row);
            Ok(())
        });
    }

    /// デフォルト解除.
    pub(crate) fn clear_default(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.change(window, cx, |_, list| {
            list.clear_default();
            Ok(())
        });
    }

    /// 起動 (`GetData(true)`): the chosen profile goes to whoever opened the dialog. With the check
    /// box checked the row becomes the default one first.
    pub(crate) fn start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.settings.profile_of(self.selected).to_owned();
        let checked = self.settings.default_select;
        if checked || self.box_changed {
            self.change(window, cx, |_, list| {
                if checked && let Some(row) = row_named(list, &name) {
                    list.set_default(row);
                }
                Ok(())
            });
        }
        self.answer(Some(name), cx);
        window.remove_window();
    }

    /// 閉じる (`GetData(false)`): nothing is chosen; the check box is kept if it changed.
    pub(crate) fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_close(window, cx);
        window.remove_window();
    }

    /// The part of 閉じる that does not remove the window (the close button of the window does it).
    fn finish_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.on_done.is_none() {
            return;
        }
        if self.box_changed {
            self.change(window, cx, |_, _| Ok(()));
        }
        self.answer(None, cx);
    }

    fn answer(&mut self, chosen: Option<String>, cx: &mut Context<Self>) {
        if let Some(done) = self.on_done.take() {
            done(cx, chosen);
        }
    }

    /// Alt with the access key of a label (`IsDialogMessage`). Returns whether the letter was one
    /// of the dialog's.
    pub(crate) fn use_access_key(
        &mut self,
        letter: char,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.entry.is_some() {
            return false;
        }
        let entries = [
            Entry::labelled(DEFSTART_LABEL, true, Use::DefaultStart),
            Entry::labelled(START_LABEL, true, Use::Start),
            Entry::labelled(CLOSE_LABEL, true, Use::Close),
            Entry::labelled(HELP_LABEL, true, Use::Help),
            Entry::labelled(CREATE_LABEL, true, Use::Create),
            Entry::labelled(RENAME_LABEL, self.rename_usable(), Use::Rename),
            Entry::labelled(DELETE_LABEL, self.rename_usable(), Use::Delete),
            Entry::labelled(DEFSET_LABEL, true, Use::SetDefault),
            Entry::labelled(DEFCLEAR_LABEL, self.clear_usable(), Use::ClearDefault),
        ];
        match pick(letter, &entries) {
            Found::Nothing => return false,
            Found::Several => {}
            Found::One(Use::DefaultStart) => {
                let on = !self.settings.default_select;
                self.set_default_start(on, cx);
            }
            Found::One(Use::Start) => self.start(window, cx),
            Found::One(Use::Close) => self.close(window, cx),
            Found::One(Use::Help) => open_help(cx),
            Found::One(Use::Create) => self.begin_create(window, cx),
            Found::One(Use::Rename) => self.begin_rename(window, cx),
            Found::One(Use::Delete) => self.delete(window, cx),
            Found::One(Use::SetDefault) => self.make_default(window, cx),
            Found::One(Use::ClearDefault) => self.clear_default(window, cx),
        }
        true
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if self.entry.is_some() {
            // the name panel: Esc is its Cancel, Enter in the field is its OK (the field says so),
            // and Enter or Space on its buttons presses them
            match key {
                "escape" => self.cancel_entry(window, cx),
                "enter" | "space" if self.entry_cancel.is_focused(window) => {
                    self.cancel_entry(window, cx);
                }
                "enter" | "space" if self.entry_ok.is_focused(window) => {
                    self.commit_entry(window, cx);
                }
                _ => return,
            }
            cx.stop_propagation();
            return;
        }
        let list = self.list_focus.is_focused(window);
        let last = self.settings.rows() - 1;
        match key {
            // `IDCANCEL`
            "escape" => self.close(window, cx),
            "up" if list => self.select(self.selected.saturating_sub(1), cx),
            "down" if list => self.select(self.selected + 1, cx),
            "home" if list => self.select(0, cx),
            "end" if list => self.select(last, cx),
            // `IDOK`, the default button, when no button has the focus
            "enter" if list || self.focus.is_focused(window) => self.start(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Focusable for ProfileManagerDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

/// `IDC_BUTTON_HELP`. The dialog stays.
fn open_help(cx: &mut App) {
    crate::support::help::open_page(cx, HELP_PAGE, None);
}

impl Render for ProfileManagerDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        // The focus is somewhere in the dialog; when it is not, the list has it. Not while the name panel
        // is open: its field was just focused and is not in the tree of the last frame yet.
        if self.entry.is_none() && !self.focus.contains_focused(window, cx) {
            window.focus(&self.list_focus, cx);
        }
        let ui_font = gpui_component::Theme::global(cx).font_family.clone();
        let list_ring = if self.list_focus.is_focused(window) {
            FOCUS_RING
        } else {
            BORDER
        };
        let rows = (0..self.settings.rows()).map(|row| {
            let chosen = row == self.selected;
            let starred = self.settings.is_default_row(row);
            let name = self.settings.row_label(row);
            // `SetData`: the default row shows a star after its name
            let shown = if starred {
                format!("{name}*")
            } else {
                name.to_owned()
            };
            let spoken = if starred {
                format!("{name}、デフォルト")
            } else {
                name.to_owned()
            };
            crate::a11y::list_row(div().id(("prof-row", row)), &spoken, chosen)
                .px_2()
                .py_px()
                .bg(rgb(if chosen { CHOSEN_BG } else { 0xffffff }))
                .overflow_hidden()
                .whitespace_nowrap()
                .child(shown)
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.select(row, cx);
                    window.focus(&this.list_focus, cx);
                }))
        });
        let list = crate::a11y::list_box(div().id("prof-list"), LIST_NAME)
            .track_focus(&self.list_focus)
            .h(px(220.))
            .flex_1()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .border_1()
            .border_color(rgb(list_ring))
            .bg(rgb(0xffffff))
            .children(rows);

        let button = |id: &'static str,
                      label: &'static str,
                      usable: bool,
                      press: fn(&mut Self, &mut Window, &mut Context<Self>),
                      cx: &mut Context<Self>| {
            div().debug_selector(|| id.to_owned()).child(
                Button::new(id)
                    .labelled(crate::common_settings::model::parse_label(label).0)
                    .w_full()
                    .disabled(!usable)
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        press(this, window, cx);
                    })),
            )
        };
        let side = v_flex()
            .gap_2()
            .w(px(140.))
            .child(button(
                "prof-create",
                CREATE_LABEL,
                true,
                Self::begin_create,
                cx,
            ))
            .child(button(
                "prof-rename",
                RENAME_LABEL,
                self.rename_usable(),
                Self::begin_rename,
                cx,
            ))
            .child(button(
                "prof-delete",
                DELETE_LABEL,
                self.rename_usable(),
                Self::delete,
                cx,
            ))
            .child(div().h(px(8.)))
            .child(button(
                "prof-defset",
                DEFSET_LABEL,
                true,
                Self::make_default,
                cx,
            ))
            .child(button(
                "prof-defclear",
                DEFCLEAR_LABEL,
                self.clear_usable(),
                Self::clear_default,
                cx,
            ));
        let check = Checkbox::new("prof-defstart")
            .labelled(crate::common_settings::model::parse_label(DEFSTART_LABEL).0)
            .checked(self.settings.default_select)
            .on_click(cx.listener(|this, value: &bool, _, cx| {
                this.set_default_start(*value, cx);
            }));
        let bottom = h_flex()
            .gap_2()
            .justify_end()
            .child(button("prof-start", START_LABEL, true, Self::start, cx))
            .child(button("prof-close", CLOSE_LABEL, true, Self::close, cx))
            .child(
                div().child(
                    Button::new("prof-help")
                        .labelled(crate::common_settings::model::parse_label(HELP_LABEL).0)
                        .on_click(|_: &ClickEvent, _, cx| open_help(cx)),
                ),
            );

        let entry = self
            .entry
            .as_ref()
            .map(|entry| self.render_entry(entry, window, cx));
        crate::a11y::dialog(div().id("profile-manager-dialog"), TITLE)
            .on_help_page_key(HELP_PAGE)
            .key_context("ProfileManagerDialog")
            .track_focus(&self.focus)
            .on_access_key(move |letter, window, cx| {
                this.update(cx, |dialog, cx| dialog.use_access_key(letter, window, cx))
            })
            .on_key_down(cx.listener(Self::on_key_down))
            .relative()
            .size_full()
            .p_3()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0xf0f0f0))
            .text_color(rgb(TEXT))
            .text_size(px(14.))
            .font_family(ui_font)
            .child(h_flex().gap_3().items_start().child(list).child(side))
            .child(check)
            .child(bottom)
            .children(entry)
    }
}

impl ProfileManagerDialog {
    /// The panel that asks for a name, over the dialog (`IDD_INPUT1`: the message, the field, OK and
    /// Cancel). The keys are those of [`crate::input_dialog`].
    fn render_entry(
        &self,
        entry: &NameEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let ring = |focused: bool| if focused { FOCUS_RING } else { BORDER };
        let make = |id: &'static str, label: &'static str, handle: &FocusHandle| {
            crate::a11y::button(div().id(id), label)
                .track_focus(handle)
                .px_4()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(ring(handle.is_focused(window))))
                .bg(rgb(0xf4f4f4))
                .child(label)
        };
        let ok = make("prof-entry-ok", "OK", &self.entry_ok).on_click(
            cx.listener(|this, _: &ClickEvent, window, cx| this.commit_entry(window, cx)),
        );
        let cancel = make("prof-entry-cancel", "キャンセル", &self.entry_cancel).on_click(
            cx.listener(|this, _: &ClickEvent, window, cx| this.cancel_entry(window, cx)),
        );
        let message: SharedString = entry.message.into();
        let order = [
            entry.input.read(cx).focus_handle(cx),
            self.entry_ok.clone(),
            self.entry_cancel.clone(),
        ];
        let order_back = order.clone();
        let panel = crate::a11y::dialog(div().id("prof-entry"), entry.title)
            .aria_description(message.clone())
            .key_context("InputDialog")
            .on_action(cx.listener(move |_, _: &DialogTab, window, cx| {
                cycle_focus(&order, true, window, cx);
            }))
            .on_action(cx.listener(move |_, _: &DialogBackTab, window, cx| {
                cycle_focus(&order_back, false, window, cx);
            }))
            .w(px(380.))
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0xf0f0f0))
            .border_1()
            .border_color(rgb(0x888888))
            .child(entry.title)
            .child(div().mt_2().child(message.clone()))
            .child(Input::new(&entry.input).aria_label(message))
            .child(
                div()
                    .mt_2()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(ok)
                    .child(cancel),
            );
        div()
            .id("prof-entry-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .bg(rgba(0x00000055))
            .flex()
            .items_center()
            .justify_center()
            .child(panel)
    }
}

/// Why an action did not happen: the text of the box (empty: nothing is said) and the cause.
struct Failure {
    message: &'static str,
    detail: Option<String>,
}

impl Failure {
    fn new(message: &'static str) -> Self {
        Self {
            message,
            detail: None,
        }
    }

    /// A refusal that says nothing (an empty name, a profile that is gone).
    fn none() -> Self {
        Self::new("")
    }

    /// With the cause that the OS gave.
    fn because(message: &'static str, cause: &dyn std::fmt::Display) -> Self {
        Self {
            message,
            detail: Some(cause.to_string()),
        }
    }
}

/// The checks of `CreateProf` and `RenameProf` on the list as it is on the disk, with their texts.
fn check_name(list: &ProfileSettings, name: &str, skip: Option<usize>) -> Result<(), Failure> {
    list.check_name(name, skip).map_err(|error| match error {
        NameError::Empty => Failure::none(),
        NameError::InvalidChar => Failure::new(ERR_INVALID_CHAR),
        NameError::Duplicate => Failure::new(ERR_ALREADY),
    })
}

/// Whether an editor runs with the profile (`in_use`).
fn in_use_of(store: &Store, name: &str) -> bool {
    store.dir(name).is_some_and(|dir| in_use::is_held(&dir))
}

/// The inverse of a rename on the list as it is now: the row that has the new name gets the old one back.
/// A list without the new name (another window changed it) is left alone.
fn undo_rename(list: &mut ProfileSettings, new: &str, old: &str) {
    if let Some(row) = row_named(list, new).filter(|row| *row != 0) {
        list.rename(row, old);
    }
}

/// The row of the profile of this name (the empty name is 「(default)」).
fn row_named(list: &ProfileSettings, name: &str) -> Option<usize> {
    if name.is_empty() {
        return Some(0);
    }
    list.profiles.iter().position(|p| p == name).map(|i| i + 1)
}

/// The dialog that is open and its window.
struct OpenDialog {
    window: AnyWindowHandle,
    /// For the tests, which drive the dialog.
    #[cfg_attr(not(test), allow(dead_code))]
    dialog: gpui::WeakEntity<ProfileManagerDialog>,
}

impl gpui::Global for OpenDialog {}

/// The dialog that is open and its window.
#[cfg(test)]
pub(crate) fn current(cx: &App) -> Option<(Entity<ProfileManagerDialog>, AnyWindowHandle)> {
    let open = cx.try_global::<OpenDialog>()?;
    if !cx.windows().contains(&open.window) {
        return None;
    }
    Some((open.dialog.upgrade()?, open.window))
}

/// Shows the dialog; `on_done` gets the profile that 起動 chose, or `None` for 閉じる (however the
/// dialog is closed, once). One that is open comes to the front instead (and `on_done` is dropped).
pub(crate) fn open(
    cx: &mut App,
    store: Store,
    on_done: Done,
) -> Option<Entity<ProfileManagerDialog>> {
    if let Some(open) = cx.try_global::<OpenDialog>()
        && cx.windows().contains(&open.window)
    {
        let window = open.window;
        window
            .update(cx, |_, window, _| window.activate_window())
            .ok();
        return None;
    }
    let bounds = Bounds::centered(None, size(px(440.), px(360.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        icon: crate::app_icon::window_icon(),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some(TITLE.into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let slot: Rc<std::cell::RefCell<Option<Entity<ProfileManagerDialog>>>> = Rc::default();
    let made = slot.clone();
    let opened = cx.open_window(options, move |window, cx| {
        let dialog = cx.new(|cx| ProfileManagerDialog::new(store, on_done, cx));
        *made.borrow_mut() = Some(dialog.clone());
        // The close button of the window is 閉じる as well (`IDCANCEL`).
        let closing = dialog.downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            if let Some(dialog) = closing.upgrade() {
                dialog.update(cx, |dialog, cx| dialog.finish_close(window, cx));
            }
            true
        });
        cx.new(|cx| Root::new(dialog, window, cx))
    });
    let handle = opened.ok()?;
    let dialog = slot.borrow_mut().take()?;
    cx.set_global(OpenDialog {
        window: handle.into(),
        dialog: dialog.downgrade(),
    });
    Some(dialog)
}

#[cfg(test)]
mod tests;
