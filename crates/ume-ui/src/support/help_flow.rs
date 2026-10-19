//! The help functions on the view: what `F_EXTHELP1`, `F_EXTHTMLHELP`, `F_HELP_CONTENTS` and
//! `F_HELP_SEARCH` do (the rules are in [`super::help`]).
//!
//! Sakura asks 「外部ヘルプ１が設定されていません。今すぐ設定しますか?」 when no file is set, shows
//! the 支援 page of the common settings on Yes, and tries again when the page is closed. The settings
//! window here does not block, so on Yes it opens at the page and the function ends; the user runs
//! it again after setting the file.

use gpui::{Context, PromptLevel, Window};
use ume_config::settings::TypeSettings;

use super::help::{self, Opening};
use super::prefs;
use crate::common_settings::{self, Page};
use crate::file_ops::ask::{self, Answer};
use crate::view::EditorView;

/// At most `_MAX_PATH` - 1 characters of the word go to the viewer (`GetCurrentTextForSearch`
/// strips a longer word).
const MAX_KEYWORD_CHARS: usize = 259;

impl EditorView {
    /// The settings of the document type of this view (a type that is not there gets the
    /// defaults).
    pub(crate) fn type_settings(&self, cx: &gpui::App) -> TypeSettings {
        let index = self.type_index();
        crate::doc_types::with_user(cx, |user| {
            user.shared_types().get(index).cloned().unwrap_or_default()
        })
    }

    /// `Command_EXTHELP1`.
    pub(crate) fn ext_help1(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let t = self.type_settings(cx);
        let prefs = prefs::current(cx);
        let Some(file) = help::ext_help(&t, &prefs) else {
            self.ask_to_set_help(help::EXT_HELP_NOT_SET, window, cx);
            return;
        };
        // `GetCurrentTextForSearch( cmemCurText, false )`
        let keyword = self.search_seed(cx).unwrap_or_default();
        let path = self.help_path(file, cx);
        let single = help::html_help_single(&t, &prefs);
        self.open_help(help::plan_ext_help(path, &keyword, single), cx);
    }

    /// `Command_EXTHTMLHELP`: `file` and `keyword` are the arguments of the macro; a missing or
    /// empty one is the file of the settings and the word at the caret.
    pub(crate) fn ext_html_help(
        &mut self,
        file: Option<&str>,
        keyword: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let t = self.type_settings(cx);
        let prefs = prefs::current(cx);
        let file = match file.filter(|f| !f.is_empty()) {
            Some(file) => file.to_owned(),
            None => match help::ext_html_help(&t, &prefs) {
                Some((file, _)) => file.to_owned(),
                None => {
                    self.ask_to_set_help(help::EXT_HTML_HELP_NOT_SET, window, cx);
                    return;
                }
            },
        };
        let keyword = match keyword.filter(|k| !k.is_empty()) {
            Some(keyword) => keyword.to_owned(),
            // `GetCurrentTextForSearch( cmemCurText )`
            None => self
                .search_seed(cx)
                .unwrap_or_default()
                .chars()
                .take(MAX_KEYWORD_CHARS)
                .collect(),
        };
        let path = self.help_path(&file, cx);
        let single = help::html_help_single(&t, &prefs);
        self.open_help(help::plan_html_help(path, &keyword, single), cx);
    }

    /// `Command_HELP_CONTENTS` (`contents`) and `Command_HELP_SEARCH`.
    pub(crate) fn program_help(&mut self, contents: bool, cx: &mut Context<Self>) {
        let url = if contents {
            help::CONTENTS_URL
        } else {
            help::INDEX_URL
        };
        self.open_help(Opening::Url(url.to_owned()), cx);
    }

    /// A relative path of a help file starts at the folder of the settings file, else at the
    /// folder of the program (`GetInidirOrExedir`).
    fn help_path(&self, file: &str, cx: &gpui::App) -> std::path::PathBuf {
        let ini_dir = crate::doc_types::with_user(cx, |user| {
            crate::settings::history_file(user)
                .and_then(|path| path.parent().map(std::path::Path::to_owned))
        });
        help::inidir_or_exedir(file, ini_dir.as_deref(), help::exe_dir().as_deref())
    }

    fn open_help(&mut self, opening: Opening, cx: &mut Context<Self>) {
        let opened = help::perform(cx, opening);
        if opened.keyword_lost {
            self.notice = Some(help::NO_KEYWORD_LOOKUP.to_owned());
            cx.notify();
        }
    }

    /// The question of Sakura when no file is set: set it now?
    fn ask_to_set_help(
        &mut self,
        message: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::file_ops::start(window, cx, async move |_, cx| {
            let answer = ask::ask(
                cx,
                PromptLevel::Warning,
                message,
                None,
                ["はい(Y)", "いいえ(N)", "キャンセル"],
            )
            .await;
            if answer == Answer::Yes {
                cx.update(|window, app| {
                    let opener = window.window_handle();
                    common_settings::open(app, Some(Page::Support), Some(opener));
                })
                .ok();
            }
        });
    }
}
