//! The end of the editor with `KeepUnsavedNoAsk` (ADR-0022, section 5), in GPUI's headless test windows with a real
//! folder: the copies are written and the questions about them are left out; everything that is not covered is
//! asked as before.

use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext};
use matsu_config::unsaved::Store;

use super::prefs::KeepPrefs;
use super::window_tests::{Fixture, doc_files, fixture, tick, type_text, workspace};
use crate::document::SharedDocument;
use crate::file_ops::windows::open_editor_window;

fn hot(cx: &mut TestAppContext, on: bool) {
    cx.update(|app| {
        app.set_global(KeepPrefs {
            no_ask: on,
            ..KeepPrefs::default()
        })
    });
}

fn run(cx: &mut TestAppContext, f: &Fixture, name: &str) {
    let code = matsu_commands::sakura::FUNCTIONS
        .iter()
        .find(|f| f.name == name)
        .unwrap()
        .code;
    let ws = workspace(cx, f.handle);
    let view = ws.read_with(cx, |ws, _| ws.editor().clone());
    f.handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &[], window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

fn asks(cx: &mut TestAppContext, f: &Fixture) -> bool {
    VisualTestContext::from_window(f.handle, cx)
        .into_mut()
        .has_pending_prompt()
}

fn window_count(cx: &mut TestAppContext) -> usize {
    cx.update(|app| app.windows().len())
}

#[gpui::test]
fn an_exit_of_all_windows_writes_the_latest_text_and_does_not_ask(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    type_text(cx, &f, "typed just now");
    // No tick yet: the copy does not exist.
    assert!(doc_files(&f.dir).is_empty());
    run(cx, &f, "F_EXITALL");
    assert!(
        !asks(cx, &f),
        "no question about a document whose copy is kept"
    );
    let files = doc_files(&f.dir);
    assert_eq!(files.len(), 1, "the latest text was written before the end");
    let bytes = std::fs::read(f.dir.join(&files[0])).unwrap();
    assert!(String::from_utf8_lossy(&bytes).ends_with("typed just now"));
}

#[gpui::test]
fn without_the_setting_the_exit_asks_as_before(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, false);
    type_text(cx, &f, "typed");
    run(cx, &f, "F_EXITALL");
    assert!(asks(cx, &f));
}

#[gpui::test]
fn the_last_window_closes_without_a_question_and_keeps_its_copy(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    type_text(cx, &f, "kept for the next start");
    run(cx, &f, "F_WINCLOSE");
    assert!(!asks(cx, &f));
    assert_eq!(window_count(cx), 0, "the window is closed");
    assert_eq!(doc_files(&f.dir).len(), 1, "the copy stays");
}

#[gpui::test]
fn a_window_that_is_not_the_last_asks(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    cx.update(|app| open_editor_window(app, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    type_text(cx, &f, "one of two windows");
    run(cx, &f, "F_WINCLOSE");
    assert!(asks(cx, &f), "closing one window is not the end");
}

#[gpui::test]
fn a_document_whose_copy_cannot_be_kept_is_asked_about(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    // Another process holds the folder: nothing can be kept.
    let held = Store::open(&f.dir).unwrap();
    type_text(cx, &f, "no copy possible");
    run(cx, &f, "F_EXITALL");
    assert!(asks(cx, &f));
    drop(held);
}

#[gpui::test]
fn a_document_over_the_limit_is_asked_about(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update(|app| {
        app.set_global(KeepPrefs {
            no_ask: true,
            max_mb: 1,
            ..KeepPrefs::default()
        })
    });
    let big = "x".repeat(1024 * 1024 + 10);
    let ws = workspace(cx, f.handle);
    ws.update(cx, |ws, cx| {
        ws.editor().clone().update(cx, |view, cx| {
            view.document().update(cx, |shared, _| {
                let (model, _) = shared.parts_mut();
                model
                    .replace(
                        matsu_core::buffer::Pos::new(0, 0),
                        matsu_core::buffer::Pos::new(0, 0),
                        &big,
                    )
                    .unwrap();
            });
        });
    });
    run(cx, &f, "F_EXITALL");
    assert!(asks(cx, &f), "a document that is not kept is asked about");
}

#[cfg(unix)]
#[gpui::test]
fn a_document_whose_write_fails_is_asked_about(cx: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt;
    let f = fixture(cx);
    hot(cx, true);
    type_text(cx, &f, "first");
    let t0 = Instant::now();
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    assert_eq!(doc_files(&f.dir).len(), 1);
    type_text(cx, &f, " and more");
    std::fs::set_permissions(&f.dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    if std::fs::write(f.dir.join("probe"), b"x").is_ok() {
        // Not enforced (running as root): nothing to test.
        std::fs::set_permissions(&f.dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    run(cx, &f, "F_EXITALL");
    let asked = asks(cx, &f);
    std::fs::set_permissions(&f.dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(asked, "the latest text could not be written");
}

#[gpui::test]
fn answering_no_to_an_uncovered_document_discards_its_copy_at_the_end(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    let held = Store::open(&f.dir).unwrap();
    type_text(cx, &f, "will be thrown away");
    run(cx, &f, "F_EXITALL");
    assert!(asks(cx, &f));
    let window = VisualTestContext::from_window(f.handle, cx).into_mut();
    window.simulate_prompt_answer("保存しない(N)");
    window.run_until_parked();
    assert!(doc_files(&f.dir).is_empty());
    drop(held);
}

#[gpui::test]
fn a_window_that_stays_empty_after_the_last_tab_closes_is_not_an_end(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    // Tab mode with 「最後のファイルを閉じても空のウィンドウを残す」: the last tab closes into an empty document.
    cx.update(|app| {
        app.set_global(crate::tabs::settings::TabSettings {
            show_bar: true,
            grouped: true,
            new_window: false,
        });
        let mut options = crate::tabs::settings::options(app);
        options.retain_empty = true;
        app.set_global(options);
    });
    type_text(cx, &f, "would be lost");
    run(cx, &f, "F_WINCLOSE");
    assert!(asks(cx, &f), "the text is asked about, the window stays");
}

#[gpui::test]
fn a_cancelled_flow_does_not_make_the_hot_exit_delete_the_copy(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    type_text(cx, &f, "covered");
    let t0 = Instant::now();
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    assert_eq!(doc_files(&f.dir).len(), 1);
    // An earlier flow got 「保存しない」 and was cancelled afterwards (the file chooser): the mark is still there.
    let ws = workspace(cx, f.handle);
    let view = ws.read_with(cx, |ws, _| ws.editor().clone());
    view.update(cx, |view, cx| {
        let this = cx.weak_entity();
        super::mark_close(view, this, cx);
    });
    run(cx, &f, "F_EXITALL");
    assert!(!asks(cx, &f));
    assert_eq!(
        doc_files(&f.dir).len(),
        1,
        "the copy that covers the document stays"
    );
}

/// Runs `confirm_discard_ending` in the window of the fixture, and says what it answered (`None` while it asks).
fn ending(
    cx: &mut TestAppContext,
    f: &Fixture,
    ends: impl Fn(&mut gpui::App) -> bool + 'static,
) -> std::rc::Rc<std::cell::Cell<Option<bool>>> {
    let answer = std::rc::Rc::new(std::cell::Cell::new(None));
    let ws = workspace(cx, f.handle);
    let view = ws.read_with(cx, |ws, _| ws.editor().downgrade());
    let out = answer.clone();
    f.handle
        .update(cx, |_, window, app| {
            window
                .spawn(app, async move |cx| {
                    let go_on =
                        crate::file_ops::close::confirm_discard_ending(&view, cx, &ends).await;
                    out.set(Some(go_on));
                })
                .detach();
        })
        .unwrap();
    cx.run_until_parked();
    answer
}

#[gpui::test]
fn a_window_that_opens_while_the_copies_are_written_makes_the_close_ask(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    type_text(cx, &f, "text");
    let t0 = Instant::now();
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    let view = workspace(cx, f.handle).read_with(cx, |ws, _| ws.editor().downgrade());
    // At the moment of the decision another window has opened: this close is not the end any more.
    let answer = ending(cx, &f, move |app| {
        open_editor_window(app, SharedDocument::empty(), None).unwrap();
        crate::file_ops::close::closing_ends_the_editor(app, &view)
    });
    assert!(asks(cx, &f), "the question is asked");
    assert_eq!(answer.get(), None);
    assert_eq!(doc_files(&f.dir).len(), 1);
}

#[gpui::test]
fn closing_without_the_question_records_the_file_in_the_history(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    cx.update(|app| {
        app.set_global(crate::history::HistoryState::in_memory(
            crate::history::HistoryOptions::default(),
        ))
    });
    let path = f.dir.parent().unwrap().join("named.txt");
    std::fs::write(&path, "x").unwrap();
    // A second window with the named document (the fixture's window has none).
    let handle = cx
        .update(|app| {
            open_editor_window(
                app,
                SharedDocument::new(
                    matsu_core::model::DocumentModel::empty(),
                    Some(path.clone()),
                ),
                None,
            )
        })
        .unwrap();
    cx.run_until_parked();
    let named = Fixture {
        handle,
        dir: f.dir.clone(),
        _tmp: tempfile::tempdir().unwrap(),
    };
    type_text(cx, &named, "edited");
    let t0 = Instant::now();
    tick(cx, &named, t0);
    tick(cx, &named, t0 + Duration::from_secs(3));
    let answer = ending(cx, &named, |_| true);
    assert_eq!(answer.get(), Some(true), "no question");
    let known = cx.update(|app| {
        crate::history::current(app)
            .and_then(|h| h.file(&crate::history::view::path_text(&path)).cloned())
            .is_some()
    });
    assert!(known, "the file is in the history");
    assert_eq!(doc_files(&f.dir).len(), 1, "the copy stays");
}

#[gpui::test]
fn a_tab_whose_question_was_left_out_is_asked_when_a_window_opens_during_a_later_question(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update(|app| {
        app.set_global(KeepPrefs {
            no_ask: true,
            max_mb: 1,
            ..KeepPrefs::default()
        });
        app.set_global(crate::tabs::settings::TabSettings {
            show_bar: true,
            grouped: true,
            new_window: false,
        });
        let mut options = crate::tabs::settings::options(app);
        options.retain_empty = false;
        options.close_one_win = false;
        app.set_global(options);
    });
    // Tab A is covered by its copy.
    type_text(cx, &f, "covered tab");
    let t0 = Instant::now();
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    // Tab B is too big to be kept, so it is asked about.
    run(cx, &f, "F_FILENEW");
    let ws = workspace(cx, f.handle);
    assert_eq!(ws.read_with(cx, |ws, _| ws.tab_count()), 2);
    let big = "x".repeat(1024 * 1024 + 10);
    ws.update(cx, |ws, cx| {
        ws.editor().clone().update(cx, |view, cx| {
            view.document().update(cx, |shared, _| {
                let (model, _) = shared.parts_mut();
                model
                    .replace(
                        matsu_core::buffer::Pos::new(0, 0),
                        matsu_core::buffer::Pos::new(0, 0),
                        &big,
                    )
                    .unwrap();
            });
        });
    });
    tick(cx, &f, t0 + Duration::from_secs(10));
    // The close button of the last window: A is left out, B is asked about.
    let window = VisualTestContext::from_window(f.handle, cx).into_mut();
    assert!(!window.simulate_close());
    window.run_until_parked();
    assert!(asks(cx, &f), "the question about the big tab");
    // A window opens while the question is open: the close is not the end any more.
    cx.update(|app| open_editor_window(app, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    let window = VisualTestContext::from_window(f.handle, cx).into_mut();
    window.simulate_prompt_answer("保存しない(N)");
    window.run_until_parked();
    // The tab that was left out is asked about now, and the window is still there.
    assert!(asks(cx, &f), "the first tab is asked about now");
    assert!(cx.update(|app| app.windows().contains(&f.handle)));
}

#[gpui::test]
fn the_copy_of_a_document_closed_without_the_question_stays_while_the_process_lives_on(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    hot(cx, true);
    type_text(cx, &f, "closed at the end");
    let t0 = Instant::now();
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    assert_eq!(doc_files(&f.dir).len(), 1);
    let answer = ending(cx, &f, |_| true);
    assert_eq!(answer.get(), Some(true), "no question");
    // The window goes, but the process does not end (a dialog that is not an editor window is open): the
    // housekeeping must not remove the copy with the document.
    f.handle
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
    for s in [10, 20, 60] {
        cx.update(|app| super::keeper::app_tick(app, t0 + Duration::from_secs(s)));
    }
    cx.update(|app| super::keeper::flush(app));
    assert_eq!(doc_files(&f.dir).len(), 1, "the copy is still there");
}

#[gpui::test]
fn the_end_of_the_process_waits_for_the_writer_itself(cx: &mut TestAppContext) {
    let f = fixture(cx);
    hot(cx, true);
    type_text(cx, &f, "last words");
    assert!(doc_files(&f.dir).is_empty());
    // What the quit callback does: the future that comes back is dropped at once (GPUI waits 200 ms for it), so
    // the copy must be written by the time the call returns.
    cx.update(|app| drop(super::settle_at_quit(app)));
    let files = doc_files(&f.dir);
    assert_eq!(files.len(), 1);
    let bytes = std::fs::read(f.dir.join(&files[0])).unwrap();
    assert!(String::from_utf8_lossy(&bytes).ends_with("last words"));
}

#[gpui::test]
fn a_cancelled_close_takes_back_what_it_left_out_for_the_copies(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update(|app| {
        app.set_global(KeepPrefs {
            no_ask: true,
            max_mb: 1,
            ..KeepPrefs::default()
        });
        app.set_global(crate::tabs::settings::TabSettings {
            show_bar: true,
            grouped: true,
            new_window: false,
        });
        let mut options = crate::tabs::settings::options(app);
        options.retain_empty = false;
        options.close_one_win = false;
        app.set_global(options);
    });
    // Tab A is covered by its copy.
    type_text(cx, &f, "covered tab");
    let t0 = Instant::now();
    tick(cx, &f, t0);
    tick(cx, &f, t0 + Duration::from_secs(3));
    let ws = workspace(cx, f.handle);
    let a = ws.read_with(cx, |ws, app| ws.editor().read(app).document().entity_id());
    // Tab B is too big to be kept.
    run(cx, &f, "F_FILENEW");
    let big = "x".repeat(1024 * 1024 + 10);
    ws.update(cx, |ws, cx| {
        ws.editor().clone().update(cx, |view, cx| {
            view.document().update(cx, |shared, _| {
                let (model, _) = shared.parts_mut();
                model
                    .replace(
                        matsu_core::buffer::Pos::new(0, 0),
                        matsu_core::buffer::Pos::new(0, 0),
                        &big,
                    )
                    .unwrap();
            });
        });
    });
    tick(cx, &f, t0 + Duration::from_secs(10));
    let window = VisualTestContext::from_window(f.handle, cx).into_mut();
    assert!(!window.simulate_close());
    window.run_until_parked();
    assert!(asks(cx, &f), "the question about the big tab");
    assert!(
        cx.update(|app| super::keeper::kept_for_exit(app, a)),
        "A was left out for its copy"
    );
    let window = VisualTestContext::from_window(f.handle, cx).into_mut();
    window.simulate_prompt_answer("キャンセル");
    window.run_until_parked();
    assert!(
        !cx.update(|app| super::keeper::kept_for_exit(app, a)),
        "the cancelled flow took it back"
    );
}
