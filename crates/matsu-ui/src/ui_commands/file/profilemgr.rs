//! `F_PROFILEMGR`: the profile manager.
//!
//! Sakura: `Command_PROFILEMGR` (`sakura_core/cmd/CViewCommander_File.cpp`) shows the dialog, and if
//! 起動 is pressed it starts a new editor with `-PROF="<the chosen profile>"`
//! (`CControlTray::OpenNewEditor`). 閉じる does nothing. The function has no macro name, no key
//! and no place in the default menu in Sakura (`Funccode_x.hsrc`: `なし`; it is in the list of the
//! functions that a custom menu or a key can be given, and it is a button of the toolbar,
//! `CMenuDrawer.cpp`).
//!
//! The dialog is `crate::profile_manager`; the new editor is a new process, because a profile is
//! chosen for a whole process (the settings file it reads).

use crate::profile_manager::{self, Store};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct ProfileMgr;

impl UiCommand for ProfileMgr {
    const ID: &'static str = "profilemgr";
    const SAKURA: &'static str = "F_PROFILEMGR";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let Some(store) = Store::for_process() else {
            view.notice = Some("プロファイルの一覧を置く場所がありません".to_owned());
            return Ok(());
        };
        profile_manager::open(
            cx,
            store,
            Box::new(|_, chosen| {
                if let Some(profile) = chosen {
                    profile_manager::startup::start_editor(&profile);
                }
            }),
        );
        Ok(())
    }
}

register_ui_command!(ProfileMgr);
