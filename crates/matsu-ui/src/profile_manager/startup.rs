//! The profile at the start of the process (`CProcessFactory::ProfileSelect`, `sakura_core/_main/CProcessFactory.cpp`).
//!
//! [`settle`] runs before the settings file is read, because the profile decides which file that is.
//! `matsu_config::profile_manager::startup` decides whether the dialog is shown. The dialog is a
//! window, and GPUI runs one application per process, so it is shown by a process of its own: this
//! program started again with [`PICK_ARG`], which shows only the dialog and tells the choice on its
//! standard output ([`report`], [`parse_report`]). The launch waits for it (`DoModal`).
//!
//! Cancel (閉じる) ends the launch: Sakura's `ProfileSelect` returns false and the process ends
//! (`プロファイルマネージャで「閉じる」を選んだ。プロセス終了`).

use std::ffi::OsStr;
use std::io::Write as _;
use std::process::{Command, Stdio};

use matsu_config::profile_manager::{self, Startup};

use super::{Store, open};
use crate::command_line::CommandLine;

/// The option that makes this program show the profile manager alone and report the choice.
pub const PICK_ARG: &str = "--profile-manager-pick";

/// What the process that showed the dialog says: the first line.
const CHOSEN: &str = "profile:";
const CANCELLED: &str = "cancelled";

/// What a launch does after the dialog.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Picked {
    /// 起動: this profile (the empty name is the one without a name).
    Profile(String),
    /// 閉じる: the launch ends.
    Cancelled,
}

/// The line the picking process writes for its answer.
pub(crate) fn report(picked: &Picked) -> String {
    match picked {
        Picked::Profile(name) => format!("{CHOSEN}{name}\n"),
        Picked::Cancelled => format!("{CANCELLED}\n"),
    }
}

/// What the picking process said. `None` when it said nothing that is an answer (it was killed, or it
/// failed before the dialog).
pub(crate) fn parse_report(output: &[u8]) -> Option<Picked> {
    let text = String::from_utf8_lossy(output);
    text.lines().find_map(|line| {
        if let Some(name) = line.strip_prefix(CHOSEN) {
            Some(Picked::Profile(name.to_owned()))
        } else if line == CANCELLED {
            Some(Picked::Cancelled)
        } else {
            None
        }
    })
}

/// Decides the profile of this launch and puts it in `options`. Returns `false` when the launch is to
/// end (the dialog was closed with 閉じる).
///
/// `headless` is a launch that has no window to show, a Grep that writes to the standard output
/// (`-GOPT=U`): it is a command, and a dialog that waits for a click would hang the script that runs it.
/// Such a launch uses the profile without a name when the choice is left to the dialog, unless the
/// dialog is asked for (`-PROFMGR`).
pub(crate) fn settle(options: &mut CommandLine, headless: bool) -> bool {
    // a system without a place for the settings file has no list: nothing to ask
    let Some(store) = Store::for_process() else {
        return true;
    };
    let file = store.read();
    let decision = profile_manager::startup(
        options.profile_manager,
        options.profile.as_deref(),
        file.as_ref(),
    );
    let picked = match decision {
        Startup::Profile(name) => Picked::Profile(name),
        Startup::Dialog if headless && !options.profile_manager => Picked::Profile(String::new()),
        Startup::Dialog => match pick() {
            Some(picked) => picked,
            None => {
                eprintln!(
                    "matsu: the profile manager could not be shown; using the profile without a name"
                );
                Picked::Profile(String::new())
            }
        },
    };
    match picked {
        Picked::Profile(name) => {
            options.profile = Some(name);
            true
        }
        Picked::Cancelled => false,
    }
}

/// Runs this program with [`PICK_ARG`] and waits for the answer. `None` when it could not run.
fn pick() -> Option<Picked> {
    let exe = std::env::current_exe().ok()?;
    let output = Command::new(exe)
        .arg(PICK_ARG)
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .ok()?;
    parse_report(&output.stdout)
}

/// The process of [`PICK_ARG`]: shows the dialog alone, writes the answer to the standard output and
/// returns when the dialog is closed.
pub(crate) fn run_picker() {
    let Some(store) = Store::for_process() else {
        return;
    };
    // The callback of the dialog is `'static` and the answer is read after `run` returns, so the two
    // share it (ownership.md 3: a GPUI callback that cannot go through an `Entity`).
    let answer = std::rc::Rc::new(std::cell::RefCell::new(None));
    let sink = answer.clone();
    gpui_platform::application()
        .with_assets(gpui_kit_assets::Assets)
        .run(move |cx| {
            gpui_component::init(cx);
            // the title of the message boxes (Windows), as the editor's
            matsu_platform::dialog_title::install();
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            open(
                cx,
                store,
                Box::new(move |_, chosen| {
                    *sink.borrow_mut() = Some(chosen);
                }),
            );
        });
    // a dialog that went away without an answer is a 閉じる
    let picked = match answer.borrow_mut().take().flatten() {
        Some(name) => Picked::Profile(name),
        None => Picked::Cancelled,
    };
    let mut out = std::io::stdout();
    let _ = out.write_all(report(&picked).as_bytes());
    let _ = out.flush();
}

/// Whether the arguments are those of the picking process.
pub(crate) fn is_picker(args: &[impl AsRef<OsStr>]) -> bool {
    args.first().is_some_and(|arg| arg.as_ref() == PICK_ARG)
}

/// F_PROFILEMGR: starts the editor with the profile, as a launch of its own (`OpenNewEditor` with
/// `-PROF="name"`). The new process finds the editor that runs with the profile and hands the launch
/// to it, or becomes it. The profile name is one argument, so that it needs no quoting.
pub(crate) fn start_editor(profile: &str) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Ok(mut child) = Command::new(exe)
        .arg(format!("-PROF={profile}"))
        .stdin(Stdio::null())
        .spawn()
    else {
        return;
    };
    // The launch hands over to a running editor and ends soon, or becomes an editor that runs on its
    // own. This thread only waits for it, so that it does not stay a zombie; it ends with the child.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
}
