//! 「強調キーワード」: `IDD_PROP_KEYWORD` (`sakura_core/sakura_rc.rc`), `CPropKeyword`
//! (`sakura_core/prop/CPropComKeyword.cpp`).
//!
//! The page edits the keyword sets (`CKeyWordSetMgr`): it picks a set, adds, renames and deletes a set, sets
//! its case rule, and adds, edits and deletes its keywords; 整理 sorts the keywords and removes the duplicates;
//! インポート and エクスポート read and write a `.kwd` file (`CImpExpKeyWord`). OK writes `[KeyWords]`
//! (`ShareData_IO_KeyWords`) and the numbers of the types that refer to the sets, under the lock of the
//! settings file, and every window colors with the new sets at once.
//!
//! * [`entry`]: the sets as the page edits them, with the rules and limits of `CKeyWordSetMgr`.
//! * [`store`]: the sets in the file. The sets are the rows of a hidden list (a [`Kind::Table`]) whose
//!   storage is the page's own; the page draws them itself.
//! * [`draw`]: the choice of the set and the list of keywords.
//!
//! # What differs from Sakura
//!
//! Sakura asks for a name or a keyword in a small dialog (`CDlgInput1`) that the buttons セット追加,
//! 変更, 追加 and 編集 open. Here the name and the keyword are typed in the two edit boxes of the page and the
//! buttons use what is in them; clicking a keyword puts it in its box. The sets of MATSU Editor's own
//! (ADR-0017) are sets like the others once OK has written the file: they can be edited and deleted.
//! The keywords are shown in the order of the set's case rule, as the list view of Sakura sorts them
//! (`LVS_SORTASCENDING`).

mod draw;
mod entry;
mod store;
mod transfer;

use gpui::{Context, PromptLevel, Window};

use super::super::Env;
use super::super::dialog::CommonSettingsDialog;
use super::super::model::{Control, Item, Value, Values};
use entry::{Edited, Entry, Refused, from_rows, rows_of};

/// The sets, as the rows of a hidden list.
pub(super) const SETS: &str = "keyword_sets";
/// What the sets of the file were when the dialog opened (one row); OK writes only while the file still has them.
const STAMP: &str = "keyword_stamp";
/// The number of the set that is chosen (`m_nCurrentKeyWordSetIdx`, `nCurrentKeyWordSetIdx`).
const CURRENT: &str = "keyword_current";
/// The keyword that is chosen, as its place in the set (`LVITEM::lParam`), or -1.
const WORD_SEL: &str = "keyword_word_sel";
/// The edit box of the name of a set.
const NAME: &str = "keyword_name";
/// The edit box of a keyword.
const WORD: &str = "keyword_word";
/// 英大文字小文字区別: a switch for the case rule of the chosen set. The box is drawn by the page; the control
/// is for its Alt key.
const CASE: &str = "keyword_case";

/// `STR_PROPCOMKEYWORD_SETMAX` (`%d` is `MAX_SETNUM`).
const SETS_FULL: &str = "セットは100個までしか登録できません。\n";
/// `STR_PROPCOMKEYWORD_KEYMAX`
const WORDS_FULL: &str = "登録できるキーワード数が上限に達しています。\n";
/// `STR_PROPCOMKEYWORD_DEL`
const CLEAN_QUESTION: &str =
    "現在の設定では強調キーワードとして表示できないキーワードを削除しますか？";
/// What OK says when the sets in the file are not the ones the dialog started from (not in Sakura, whose
/// sets are in shared memory).
const CHANGED_ELSEWHERE: &str = "強調キーワードのセットが、このダイアログを開いたあとに別の場所で変更されたため、セットは保存しませんでした。\nダイアログを開き直してください。";
/// `STR_IMPEXP_KEYWORD`
const IMPORT_FULL: &str =
    "キーワードの数が上限に達したため、いくつかのキーワードを追加できませんでした。";
/// `STR_IMPEXP_ERR_FILEOPEN`
pub(super) const CANNOT_OPEN: &str = "ファイルを開けませんでした。\n\n";
/// `STR_IMPEXP_OK_EXPORT`
const EXPORTED: &str = "ファイルをエクスポートしました。\n\n";

/// The number of the chosen set: `None` when there is no set. A number past the sets is the last set.
pub(super) fn current(values: &Values) -> Option<usize> {
    let count = values.rows(SETS).len();
    (count > 0).then(|| {
        usize::try_from(values.int(CURRENT))
            .unwrap_or(0)
            .min(count - 1)
    })
}

/// The row of the chosen set, as it is in the list.
fn current_row(values: &Values) -> Option<(usize, &[String])> {
    let at = current(values)?;
    Some((at, values.rows(SETS).get(at)?.as_slice()))
}

/// How many keywords the row of a set holds.
fn word_count(row: &[String]) -> usize {
    match row.get(3).map(String::as_str) {
        None | Some("") => 0,
        Some(words) => words.matches('\t').count() + 1,
    }
}

/// The chosen keyword: its place in the chosen set.
fn chosen_word(values: &Values) -> Option<usize> {
    let (_, row) = current_row(values)?;
    usize::try_from(values.int(WORD_SEL))
        .ok()
        .filter(|&i| i < word_count(row))
}

fn has_set(values: &Values) -> bool {
    current(values).is_some()
}

fn has_word(values: &Values) -> bool {
    chosen_word(values).is_some()
}

/// A change of the chosen set: no keyword is chosen in the new one.
fn set_chosen(values: &mut Values, _: &'static str) {
    values.set(WORD_SEL, Value::Int(-1));
}

/// `IDC_CHECK_KEYWORDCASE`: the case rule of the chosen set is the other one.
fn toggle_case(values: &mut Values, id: &'static str) {
    let Some(at) = current(values) else { return };
    let mut entries = from_rows(values.rows(SETS));
    let case = !entries[at].case_sensitive;
    entries[at].case_sensitive = case;
    values.set(SETS, Value::Rows(rows_of(&entries)));
    values.set(id, Value::Bool(case));
}

pub(in crate::common_settings) fn items() -> Option<Vec<Item>> {
    Some(vec![
        Item::Hidden(vec![
            Control::table(STAMP, "", &store::STAMP),
            Control::table(SETS, "", &store::SETS),
            Control::choice(CURRENT, "", "nCurrentKeyWordSetIdx", 0, &[])
                .in_section(matsu_config::settings::keywords::SECTION)
                .on_change(set_chosen),
            Control::choice(WORD_SEL, "", "", -1, &[]),
            Control::check(CASE, "英大文字小文字区別(&C)", "", false)
                .when(has_set)
                .on_change(toggle_case),
        ]),
        Item::Row(vec![
            Item::Custom(draw::set_picker),
            Control::text(NAME, "セット名の入力(&N)", "", "").into(),
            Control::button("keyword_add_set", "セット追加(&M)", |d, w, cx| {
                d.keyword_add_set(w, cx)
            })
            .into(),
            Control::button("keyword_rename_set", "変更(&H)", |d, w, cx| {
                d.keyword_rename_set(w, cx)
            })
            .when(has_set)
            .into(),
            Control::button(
                "keyword_delete_set",
                "セット削除(&R)...",
                |d, w, cx| d.keyword_delete_set(w, cx),
            )
            .when(has_set)
            .into(),
        ]),
        Item::Group(
            "強調キーワード(&K)",
            vec![
                Item::Custom(draw::word_list),
                Item::Row(vec![
                    Control::text(WORD, "キーワードの入力(&W)", "", "").into(),
                    Control::button("keyword_add_word", "追加(&A)", |d, w, cx| {
                        d.keyword_add_word(w, cx)
                    })
                    .when(has_set)
                    .into(),
                    Control::button("keyword_edit_word", "編集(&E)", |d, w, cx| {
                        d.keyword_edit_word(w, cx)
                    })
                    .when(has_word)
                    .into(),
                    Control::button("keyword_delete_word", "削除(&D)", |d, w, cx| {
                        d.keyword_delete_word(w, cx)
                    })
                    .when(has_word)
                    .into(),
                ]),
                Item::Row(vec![
                    Item::Custom(draw::case_box),
                    Control::button("keyword_clean", "整理(&O)", |d, w, cx| {
                        d.keyword_clean(w, cx)
                    })
                    .when(has_set)
                    .into(),
                    Control::button("keyword_import", "インポート(&I)...", |d, w, cx| {
                        d.keyword_import(w, cx)
                    })
                    .when(has_set)
                    .into(),
                    Control::button("keyword_export", "エクスポート(&X)...", |d, w, cx| {
                        d.keyword_export(w, cx)
                    })
                    .when(has_set)
                    .into(),
                ]),
            ],
        ),
    ])
}

/// What OK does to the windows: every view colors with the new sets and the types' numbers. At startup the
/// windows took the file as it is.
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, before: Option<&Values>) {
    let Some(before) = before else { return };
    if after.rows(SETS) == before.rows(SETS) {
        return;
    }
    // The file had other sets than the dialog started from (it was changed meanwhile): `store` wrote nothing.
    if store::stale() {
        if let Some(opener) = env.opener {
            opener
                .update(env.cx, |_, window, cx| {
                    crate::boxes::tell(window, PromptLevel::Info, CHANGED_ELSEWHERE, None, cx);
                })
                .ok();
        }
        return;
    }
    // `MYWM_CHANGESETTING`: every view takes the settings of its type again (the sets are shared by the types,
    // so any type may have changed).
    for (_, workspace) in crate::file_ops::windows::workspaces(env.cx) {
        for view in workspace.read(env.cx).all_views() {
            view.update(env.cx, |view, cx| {
                let index = view.type_index();
                view.refresh_type(index, cx);
            });
        }
    }
}

/// Tells the user something (`InfoMessage`); nobody waits for the answer.
fn say(window: &mut Window, cx: &mut Context<CommonSettingsDialog>, text: &str) {
    crate::boxes::tell(window, PromptLevel::Info, text, None, cx);
}

impl CommonSettingsDialog {
    fn keyword_entries(&self) -> Vec<Entry> {
        from_rows(self.values().rows(SETS))
    }

    fn keyword_put(&mut self, entries: &[Entry], cx: &mut Context<Self>) {
        self.set_rows(SETS, rows_of(entries), cx);
    }

    /// Chooses set `at` (`CBN_SELCHANGE`): no keyword is chosen and the keyword box is emptied.
    pub(super) fn keyword_select_set(
        &mut self,
        at: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_int(
            CURRENT,
            at.map_or(-1, |i| i32::try_from(i).unwrap_or(-1)),
            cx,
        );
        self.set_input(WORD, "", window, cx);
    }

    /// Chooses keyword `at` of the chosen set (a click on the list) and shows it in the keyword box.
    pub(super) fn keyword_select_word(
        &mut self,
        at: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let word = current_row(self.values())
            .zip(at)
            .and_then(|((_, row), i)| row.get(3)?.split('\t').nth(i).map(str::to_owned));
        let at = at.filter(|_| word.is_some());
        self.set_int(
            WORD_SEL,
            at.map_or(-1, |i| i32::try_from(i).unwrap_or(-1)),
            cx,
        );
        self.set_input(WORD, word.as_deref().unwrap_or(""), window, cx);
    }

    /// セット追加 (`IDC_BUTTON_ADDSET`): the name in the name box becomes a new set, which is chosen.
    fn keyword_add_set(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.input_text(NAME, cx).unwrap_or_default();
        let mut entries = self.keyword_entries();
        match entry::add_set(&mut entries, &name) {
            Err(Refused::TooManySets) => say(window, cx, SETS_FULL),
            Err(Refused::NoRoom) => say(window, cx, WORDS_FULL),
            Ok(None) => {}
            Ok(Some(at)) => {
                self.keyword_put(&entries, cx);
                self.keyword_select_set(Some(at), window, cx);
                self.set_input(NAME, "", window, cx);
            }
        }
    }

    /// 変更 (`IDC_BUTTON_KEYSETRENAME`): the chosen set takes the name in the name box.
    fn keyword_rename_set(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = current(self.values()) else {
            return;
        };
        let name = self.input_text(NAME, cx).unwrap_or_default();
        let mut entries = self.keyword_entries();
        if entry::rename_set(&mut entries, at, &name) {
            self.keyword_put(&entries, cx);
            self.set_input(NAME, "", window, cx);
        }
    }

    /// セット削除 (`IDC_BUTTON_DELSET`): asks, naming the types that use the set, and deletes it.
    fn keyword_delete_set(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((at, row)) = current_row(self.values()) else {
            return;
        };
        let origin = row.first().and_then(|o| o.parse::<usize>().ok());
        let name = row.get(1).cloned().unwrap_or_default();
        let users = origin.map_or_else(String::new, |origin| types_using(cx, origin));
        // `STR_PROPCOMKEYWORD_SETDEL`
        let question = format!(
            "「{name}」のセットを削除します。\nよろしいですか？\n削除しようとするセットは、以下のファイルタイプに割り当てられています。\n削除したセットは無効になります。\n\n{users}"
        );
        let answer = crate::boxes::prompt(
            window,
            PromptLevel::Info,
            &question,
            None,
            &["OK", "キャンセル"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(0) = answer.await {
                this.update_in(cx, |this, window, cx| {
                    this.keyword_remove_set(at, window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// Deletes set `at` and chooses the one that took its place (the last when it was the last).
    pub(super) fn keyword_remove_set(
        &mut self,
        at: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut entries = self.keyword_entries();
        if !entry::remove_set(&mut entries, at) {
            return;
        }
        let next = (!entries.is_empty()).then(|| at.min(entries.len() - 1));
        self.keyword_put(&entries, cx);
        self.keyword_select_set(next, window, cx);
    }

    /// 追加 (`IDC_BUTTON_ADDKEYWORD`): the keyword in the keyword box goes to the end of the chosen set.
    fn keyword_add_word(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = current(self.values()) else {
            return;
        };
        let text = self.input_text(WORD, cx).unwrap_or_default();
        let mut entries = self.keyword_entries();
        match entry::add_word(&mut entries, at, &text) {
            Err(_) => say(window, cx, WORDS_FULL),
            Ok(false) => {}
            Ok(true) => {
                self.keyword_put(&entries, cx);
                self.keyword_select_word(None, window, cx);
            }
        }
    }

    /// 編集 (`IDC_BUTTON_EDITKEYWORD`): the chosen keyword becomes the text of the keyword box; an empty
    /// box deletes it.
    fn keyword_edit_word(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(set), Some(word)) = (current(self.values()), chosen_word(self.values())) else {
            return;
        };
        let text = self.input_text(WORD, cx).unwrap_or_default();
        let mut entries = self.keyword_entries();
        match entry::edit_word(&mut entries, set, word, &text) {
            Edited::Unchanged => {}
            Edited::Replaced => {
                self.keyword_put(&entries, cx);
            }
            Edited::Removed => {
                self.keyword_put(&entries, cx);
                let left = entries[set].words.len();
                let next = (left > 0).then(|| word.min(left - 1));
                self.keyword_select_word(next, window, cx);
            }
        }
    }

    /// 削除 (`IDC_BUTTON_DELKEYWORD`): removes the chosen keyword; the one in its place is chosen.
    fn keyword_delete_word(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(set), Some(word)) = (current(self.values()), chosen_word(self.values())) else {
            return;
        };
        let mut entries = self.keyword_entries();
        if entry::remove_word(&mut entries, set, word) {
            self.keyword_put(&entries, cx);
            let left = entries[set].words.len();
            let next = (left > 0).then(|| word.min(left - 1));
            self.keyword_select_word(next, window, cx);
        }
    }

    /// 整理 (`IDC_BUTTON_KEYCLEAN`): asks, then sorts the keywords and removes the duplicates.
    fn keyword_clean(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(at) = current(self.values()) else {
            return;
        };
        let answer = crate::boxes::prompt(
            window,
            PromptLevel::Info,
            CLEAN_QUESTION,
            None,
            &["はい(&Y)", "いいえ(&N)"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(0) = answer.await {
                this.update_in(cx, |this, window, cx| {
                    this.keyword_clean_set(at, window, cx)
                })
                .ok();
            }
        })
        .detach();
    }

    /// Sorts set `at` and removes the duplicates. Returns how many keywords went.
    pub(super) fn keyword_clean_set(
        &mut self,
        at: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        let mut entries = self.keyword_entries();
        let removed = entry::clean(&mut entries, at);
        self.keyword_put(&entries, cx);
        // The keywords moved: none is chosen.
        self.keyword_select_word(None, window, cx);
        removed
    }
}

/// The types that use set `origin`, one on a line (`・name(extensions)`), as the question of セット削除 lists
/// them.
fn types_using(cx: &gpui::App, origin: usize) -> String {
    let Ok(number) = i32::try_from(origin) else {
        return String::new();
    };
    crate::doc_types::with_user(cx, |user| {
        user.document_types()
            .iter()
            .filter(|t| t.keyword_set_idx.contains(&number))
            .map(|t| format!("・{}({})\n", t.type_name, t.type_exts))
            .collect()
    })
}

#[cfg(test)]
mod tests;
