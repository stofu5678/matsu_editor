//! インポート and エクスポート of one type in the type list (`CDlgTypeList::Import` and `Export`, with
//! `CImpExpType`, `CDlgTypeAscertain`; `sakura_core/typeprop/CDlgTypeList.cpp`, `CImpExpManager.cpp`,
//! `CDlgTypeAscertain.cpp`). The files are in `matsu_config::settings::type_file`.
//!
//! エクスポート asks for a file name (the name of the type with `.ini`), writes the type, and writes the
//! keyword definition files of its keyword sets next to it.
//!
//! インポート asks for a file, checks that it is a type file (`STR_IMPEXP_ERR_TYPE`) and, when it was
//! written with another version of the settings structure, whether to go on (`STR_IMPEXP_VER`). Then
//! the question of `CDlgTypeAscertain`: add the type to the list, or replace the selected one; and
//! where the colors come from: the file, a type of the list, or a color file (`*.col`) in the folder of
//! the file. Sakura asks nothing about a type of the same name: two types may have the same name.

use std::path::{Path, PathBuf};

use gpui::{
    ClickEvent, Context, InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Role,
    ScrollHandle, StatefulInteractiveElement, Styled, Toggled, Window, div, px, rgb, rgba,
};
use matsu_config::settings::type_file::{
    self, Colors, STRUCTURE_VERSION, TYPE_WILDCARD, Target, TypeFile,
};

use super::edit::edit_types;
use super::list::{SELECTED_BG, TEXT, TypeList};
use super::with_user;
use crate::common_settings::transfer::{cannot_export, cannot_open, exported};
use crate::dialog_focus::KeepTabInside as _;
use crate::file_dialog::{self, Ask, import_folder};

/// `STR_IMPEXP_ERR_TYPE`
const NOT_A_TYPE_FILE: &str = "不正な形式です。\nインポートを中止します";
/// `STR_DLGTYPEASC_IMPORT`
const AS_IT_IS: &str = "--そのままインポート--";

/// Where the colors of the imported type come from (`IDC_COMBO_COLORS`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ColorChoice {
    /// The colors of the file.
    AsFile,
    /// The colors of the type with this number.
    OfType(usize),
    /// A color file in the folder of the file.
    File(String),
}

/// The question of the import (`CDlgTypeAscertain`).
pub(crate) struct Ascertain {
    file: TypeFile,
    path: PathBuf,
    /// 新規追加 (`IDC_RADIO_TYPE_ADD`), or the selected type (`IDC_RADIO_TYPE_TO`).
    pub(super) add: bool,
    /// The name of the selected type, the other choice of 読込先.
    to_name: String,
    /// The id of that type: it is found again when the question is answered.
    to_id: Option<i32>,
    /// The choices of 色選択 with their text.
    choices: Vec<(ColorChoice, String)>,
    pub(super) color: usize,
    scroll: ScrollHandle,
}

impl Ascertain {
    #[cfg(test)]
    pub(crate) fn choices(&self) -> &[(ColorChoice, String)] {
        &self.choices
    }
}

/// The names of the `*.col` files in `dir`, in the order of their names.
fn color_files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            Path::new(name)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("col"))
        })
        .collect();
    names.sort();
    names
}

impl TypeList {
    /// インポート: asks for a file.
    pub(super) fn import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ask = import_folder::ask_for(
            cx,
            Ask::open(false).titled("タイプ別設定のインポート", "インポート"),
            TYPE_WILDCARD,
        );
        let receiver = file_dialog::ask(window, cx, ask);
        cx.spawn_in(window, async move |this, cx| {
            let path = receiver
                .await
                .ok()
                .flatten()
                .and_then(|paths| paths.into_iter().next());
            if let Some(path) = path {
                this.update_in(cx, |this, window, cx| {
                    import_folder::remember(cx, &path);
                    this.import_chosen(&path, window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// `CImpExpType::ImportAscertain` for the file `path`: checks the file, and shows the question.
    /// Returns whether the file was a type file that can be read.
    pub(super) fn import_chosen(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Ok(bytes) = std::fs::read(path) else {
            self.say(&cannot_open(path), window, cx);
            return false;
        };
        let Some(file) = TypeFile::parse(&bytes) else {
            self.say(NOT_A_TYPE_FILE, window, cx);
            return false;
        };
        if file.structure_version() == STRUCTURE_VERSION {
            self.ask(file, path.to_owned(), cx);
        } else {
            // `STR_IMPEXP_VER`
            let message = format!(
                "エクスポートした MATSU Editor({}/{}) とバージョンが異なります。\n\n\
                 インポートしてもよろしいですか？",
                file.version(),
                file.structure_version()
            );
            let path = path.to_owned();
            self.confirm(&message, window, cx, move |this, _, cx| {
                this.ask(file, path, cx)
            });
        }
        true
    }

    /// Shows the question of the import.
    fn ask(&mut self, file: TypeFile, path: PathBuf, cx: &mut Context<Self>) {
        let mut choices = vec![(ColorChoice::AsFile, AS_IT_IS.to_owned())];
        with_user(cx, |user| {
            for (i, t) in user.document_types().iter().enumerate() {
                // `CDlgTypeAscertain::SetData` has no spaces inside the parentheses.
                let text = if t.type_exts.is_empty() {
                    t.type_name.clone()
                } else {
                    format!("{} ({})", t.type_name, t.type_exts)
                };
                choices.push((ColorChoice::OfType(i), text));
            }
        });
        if let Some(dir) = path.parent() {
            for name in color_files(dir) {
                choices.push((ColorChoice::File(name.clone()), format!("File -- {name}")));
            }
        }
        let to_name = self.labels.get(self.selected).cloned().unwrap_or_default();
        let to_id = with_user(cx, |user| {
            user.document_types().get(self.selected).map(|t| t.id)
        });
        self.ascertain = Some(Ascertain {
            file,
            path,
            // The default is 新規追加 (`CheckDlgButton(IDC_RADIO_TYPE_ADD, TRUE)`) when there is room.
            add: self.has_room(),
            to_name,
            to_id,
            choices,
            color: 0,
            scroll: ScrollHandle::new(),
        });
        cx.notify();
    }

    /// キャンセル of the question.
    pub(super) fn ascertain_cancel(&mut self, cx: &mut Context<Self>) {
        self.ascertain = None;
        cx.notify();
    }

    /// OK of the question: `CImpExpType::Import` and the rest of `CDlgTypeList::Import`.
    pub(super) fn ascertain_ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(question) = self.ascertain.take() else {
            return;
        };
        cx.notify();
        let Ascertain {
            file,
            path,
            add,
            to_id,
            choices,
            color,
            ..
        } = question;
        let dir = path.parent().map(Path::to_owned).unwrap_or_default();
        let target = if add {
            Target::Add
        } else {
            // The type that was named in the question, found again: another window may have moved or
            // deleted it.
            let index = to_id.and_then(|id| {
                with_user(cx, |user| {
                    user.document_types().iter().position(|t| t.id == id)
                })
            });
            match index {
                Some(index) => Target::Overwrite(index),
                None => {
                    self.say("インポート先のタイプが無くなりました。", window, cx);
                    return;
                }
            }
        };
        // The color file is read here, so that the message can say if it was not.
        let mut shown = Vec::new();
        let colors = match choices.get(color).map(|(choice, _)| choice) {
            None | Some(ColorChoice::AsFile) => Colors::AsFile,
            Some(ColorChoice::OfType(i)) => Colors::OfType(*i),
            Some(ColorChoice::File(name)) => {
                let base = with_user(cx, |user| {
                    user.document_types()
                        .get(0)
                        .map(|t| t.colors.clone())
                        .unwrap_or_default()
                });
                match std::fs::read(dir.join(name))
                    .ok()
                    .and_then(|bytes| type_file::parse_colors(&bytes, &base))
                {
                    Some(set) => {
                        shown.push(name.clone());
                        Colors::Given(Box::new(set))
                    }
                    None => {
                        // Not read: the colors of 基本, without a message.
                        shown.push(format!("× {name}"));
                        Colors::OfType(0)
                    }
                }
            }
        };
        let mut read_keyword_file = |name: &str| std::fs::read(dir.join(name)).ok();
        let me = cx.entity_id();
        let result = edit_types(cx, me, |profile| {
            type_file::import_type(profile, &file, target, colors, &mut read_keyword_file)
        });
        match result {
            Ok(imported) => {
                self.reload(imported.index, cx);
                for (name, read) in imported.keyword_files {
                    shown.push(if read { name } else { format!("× {name}") });
                }
                // `STR_IMPEXP_OK_IMPORT`, the file, and the files that came with it.
                let mut message = format!("ファイルをインポートしました。\n\n{}", path.display());
                for line in shown {
                    message.push('\n');
                    message.push_str(&line);
                }
                self.say(&message, window, cx);
            }
            Err(e) => self.explain(&e, window, cx),
        }
    }

    /// Keys of the question: Enter is OK, Escape キャンセル, and the arrows choose the colors.
    pub(super) fn on_ascertain_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event.keystroke.key.as_str() {
            "escape" => self.ascertain_cancel(cx),
            "enter" => self.ascertain_ok(window, cx),
            key @ ("up" | "down") => {
                if let Some(q) = self.ascertain.as_mut() {
                    q.color = if key == "up" {
                        q.color.saturating_sub(1)
                    } else {
                        (q.color + 1).min(q.choices.len().saturating_sub(1))
                    };
                    q.scroll.scroll_to_item(q.color);
                }
                cx.notify();
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    /// エクスポート: asks for a file name.
    pub(super) fn export(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(name) = with_user(cx, |user| {
            user.document_types()
                .get(self.selected)
                .map(|t| type_file::export_file_name(&t.type_name))
        }) else {
            return;
        };
        let ask = import_folder::ask_for(
            cx,
            Ask::save()
                .name(Some(name))
                .titled("タイプ別設定のエクスポート", "保存"),
            TYPE_WILDCARD,
        );
        let receiver = file_dialog::ask(window, cx, ask);
        cx.spawn_in(window, async move |this, cx| {
            let path = receiver
                .await
                .ok()
                .flatten()
                .and_then(|paths| paths.into_iter().next());
            if let Some(path) = path {
                this.update_in(cx, |this, window, cx| {
                    import_folder::remember(cx, &path);
                    this.export_to(&path, window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    /// `CImpExpType::Export` to `path`: the type, and the keyword files next to it. Returns whether
    /// the type was written.
    pub(super) fn export_to(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let profile = crate::type_settings::current_profile(cx);
        let exported_type =
            type_file::export_type(&profile, self.selected, env!("CARGO_PKG_VERSION"));
        let written = exported_type
            .as_ref()
            .ok()
            .is_some_and(|out| std::fs::write(path, &out.bytes).is_ok());
        let (Ok(out), true) = (exported_type, written) else {
            self.say(&cannot_export(path), window, cx);
            return false;
        };
        let mut message = exported(path);
        let dir = path.parent().map(Path::to_owned).unwrap_or_default();
        for keyword_file in out.keyword_files {
            // A keyword file that cannot be written is left out without a message.
            if std::fs::write(dir.join(&keyword_file.file_name), &keyword_file.bytes).is_ok() {
                message.push('\n');
                message.push_str(&keyword_file.file_name);
            }
        }
        self.say(&message, window, cx);
        true
    }

    /// The question of the import (`IDD_TYPE_ASCERTAIN`).
    pub(super) fn render_ascertain(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(q) = self.ascertain.as_ref() else {
            return div().into_any_element();
        };
        let radio = |id: &'static str, text: String, checked: bool, enabled: bool| {
            let row = div()
                .id(id)
                .role(Role::RadioButton)
                .aria_label(crate::a11y::label_name(&text))
                .aria_toggled(if checked {
                    Toggled::True
                } else {
                    Toggled::False
                })
                .px_2()
                .cursor_pointer()
                .child(format!("{} {text}", if checked { "(●)" } else { "(　)" }));
            if enabled { row } else { row.opacity(0.45) }
        };
        let has_room = self.has_room();
        let rows: Vec<_> = q
            .choices
            .iter()
            .enumerate()
            .map(|(i, (_, text))| {
                let mut item = crate::a11y::list_row(
                    div().id(("type-ascertain-color", i)),
                    text,
                    i == q.color,
                )
                .px_2()
                .h(px(22.))
                .flex()
                .items_center()
                .cursor_pointer();
                if i == q.color {
                    item = item.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff));
                }
                item.child(text.clone()).on_click(cx.listener(
                    move |this, _: &ClickEvent, _, cx| {
                        if let Some(q) = this.ascertain.as_mut() {
                            q.color = i;
                        }
                        cx.notify();
                    },
                ))
            })
            .collect();
        let button = |id: &'static str, text: &'static str| {
            crate::a11y::button(div().id(id), text)
                .px_4()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(0xaaaaaa))
                .bg(rgb(0xf4f4f4))
                .cursor_pointer()
                .child(text)
        };
        let panel = crate::a11y::dialog(div().id("type-ascertain"), "インポート確認")
            .key_context("TypeList")
            .track_focus(&self.focus)
            .keep_tab_inside(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .w(px(460.))
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0xf0f0f0))
            .border_1()
            .border_color(rgb(0x888888))
            .text_color(rgb(TEXT))
            .text_size(px(14.))
            .child(div().child("インポート確認"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .child("読込先:")
                    .child(
                        radio(
                            "type-ascertain-add",
                            "新規追加(A)".to_owned(),
                            q.add,
                            has_room,
                        )
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _, cx| {
                                if this.has_room()
                                    && let Some(q) = this.ascertain.as_mut()
                                {
                                    q.add = true;
                                }
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        radio(
                            "type-ascertain-to",
                            format!("{}(B)", q.to_name),
                            !q.add,
                            true,
                        )
                        .on_click(cx.listener(
                            |this, _: &ClickEvent, _, cx| {
                                if let Some(q) = this.ascertain.as_mut() {
                                    q.add = false;
                                }
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(div().child(format!("新タイプ名: {}", q.file.type_name())))
            .child(div().child("色選択(C):"))
            .child(
                div()
                    .id("type-ascertain-colors")
                    .role(Role::ListBox)
                    .aria_label("色選択")
                    .max_h(px(160.))
                    .overflow_y_scroll()
                    .track_scroll(&q.scroll)
                    .bg(rgb(0xffffff))
                    .border_1()
                    .border_color(rgb(0xaaaaaa))
                    .children(rows),
            )
            .child(div().child("上記条件でインポートします。よろしいですか？"))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(button("type-ascertain-ok", "OK").on_click(cx.listener(
                        |this, _: &ClickEvent, window, cx| this.ascertain_ok(window, cx),
                    )))
                    .child(button("type-ascertain-cancel", "キャンセル(X)").on_click(
                        cx.listener(|this, _: &ClickEvent, _, cx| this.ascertain_cancel(cx)),
                    )),
            );
        div()
            .id("type-list-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .flex()
            .bg(rgba(0x00000055))
            .items_center()
            .justify_center()
            .child(panel)
            .into_any_element()
    }
}
