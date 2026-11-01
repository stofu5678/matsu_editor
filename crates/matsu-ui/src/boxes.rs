//! The message boxes (`window.prompt`) of the editor, in one place.
//!
//! Two things are decided here for every box, so that the call sites do not have to:
//!
//! * The buttons. GPUI's Windows box lets Esc and the close button of the title bar through only
//!   when one of its buttons is a [`PromptButton::Cancel`], and it makes a plain label (a `&str`)
//!   an ordinary button. Windows' `MessageBox`, which Sakura uses everywhere
//!   (`sakura_core/util/MessageBoxF.cpp`), closes a box with Esc and the close button when it has
//!   a cancel button (the result is `IDCANCEL`) or only an OK button (the result is `IDOK`), and
//!   keeps it open when it has Yes / No or Abort / Retry / Ignore. [`windows_buttons`] gives the
//!   same rules. macOS and Linux keep the buttons as they were.
//! * The answer. GPUI's Windows box is cancelled when the receiver of its answer is dropped, so a
//!   box whose answer nobody needs has to be held until it is closed ([`tell`], [`hold`]).

use std::future::Future;

use gpui::{App, AsyncWindowContext, PromptButton, PromptLevel, Window};

/// The box went away without an answer.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Closed;

/// Whether `label` is the label of a cancel button (`キャンセル`, `キャンセル(C)`, `Cancel`).
fn is_cancel(label: &str) -> bool {
    label.starts_with("キャンセル") || label.eq_ignore_ascii_case("cancel")
}

/// The buttons of a box on Windows. The rules of `MessageBox`:
///
/// * a cancel button is the one that Esc and the close button press;
/// * a box with the OK button only is closed by Esc and the close button as OK (its button is
///   given as the cancel one, so that GPUI lets them through; the answer is its index, 0);
/// * the other boxes (Yes / No, ...) have no Esc.
pub(crate) fn windows_buttons(labels: &[&str]) -> Vec<PromptButton> {
    if let [only] = labels
        && only.eq_ignore_ascii_case("ok")
    {
        return vec![PromptButton::cancel(*only)];
    }
    labels
        .iter()
        .map(|label| {
            if is_cancel(label) {
                PromptButton::cancel(*label)
            } else {
                PromptButton::from(*label)
            }
        })
        .collect()
}

/// The buttons of a box on this system.
pub(crate) fn buttons(labels: &[&str]) -> Vec<PromptButton> {
    if cfg!(windows) {
        windows_buttons(labels)
    } else {
        labels
            .iter()
            .map(|label| PromptButton::from(*label))
            .collect()
    }
}

/// Shows a box in `window`. The answer is the index of the button that was pressed.
pub(crate) fn prompt(
    window: &mut Window,
    level: PromptLevel,
    message: &str,
    detail: Option<&str>,
    labels: &[&str],
    cx: &mut App,
) -> impl Future<Output = Result<usize, Closed>> + use<> {
    let answer = window.prompt(level, message, detail, &buttons(labels), cx);
    async move { answer.await.map_err(|_| Closed) }
}

/// [`prompt`] for a task of the window.
pub(crate) fn prompt_async(
    cx: &mut AsyncWindowContext,
    level: PromptLevel,
    message: &str,
    detail: Option<&str>,
    labels: &[&str],
) -> impl Future<Output = Result<usize, Closed>> + use<> {
    let answer = cx.prompt(level, message, detail, &buttons(labels));
    async move { answer.await.map_err(|_| Closed) }
}

/// The caption of a message box (Sakura shows its own name).
pub(crate) const CAPTION: &str = "MATSU Editor";

/// Tells the user something in a box with an OK button, and does not wait for the OK.
pub(crate) fn tell(
    window: &mut Window,
    level: PromptLevel,
    message: &str,
    detail: Option<&str>,
    cx: &mut App,
) {
    let answer = prompt(window, level, message, detail, &["OK"], cx);
    hold(answer, cx);
}

/// Keeps the answer of a box until it comes, and then drops it. Dropping an answer at once would
/// cancel the box on Windows.
pub(crate) fn hold(answer: impl Future + 'static, cx: &mut App) {
    cx.spawn(async move |_| {
        answer.await;
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::pin::Pin;
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};

    use gpui::TestAppContext;

    use super::*;

    fn kinds(buttons: &[PromptButton]) -> Vec<(&'static str, String)> {
        buttons
            .iter()
            .map(|b| {
                let kind = match b {
                    PromptButton::Ok(_) => "ok",
                    PromptButton::Cancel(_) => "cancel",
                    PromptButton::Other(_) => "other",
                };
                (kind, b.label().to_string())
            })
            .collect()
    }

    fn kind(kind: &'static str, label: &str) -> (&'static str, String) {
        (kind, label.to_string())
    }

    #[test]
    fn a_cancel_label_is_the_cancel_button() {
        for label in ["キャンセル", "キャンセル(C)", "キャンセル(&X)", "Cancel"] {
            assert_eq!(
                kinds(&windows_buttons(&["保存する(Y)", label])),
                [kind("other", "保存する(Y)"), kind("cancel", label)],
            );
        }
        // The cancel button that is not the last one, too.
        assert_eq!(
            kinds(&windows_buttons(&["キャンセル", "再実行", "続行"])),
            [
                kind("cancel", "キャンセル"),
                kind("other", "再実行"),
                kind("other", "続行")
            ]
        );
    }

    #[test]
    fn a_box_with_the_ok_button_only_is_closed_by_esc() {
        assert_eq!(kinds(&windows_buttons(&["OK"])), [kind("cancel", "OK")]);
    }

    #[test]
    fn ok_with_cancel_keeps_its_ok() {
        assert_eq!(
            kinds(&windows_buttons(&["OK", "キャンセル"])),
            [kind("ok", "OK"), kind("cancel", "キャンセル")]
        );
    }

    #[test]
    fn boxes_without_a_cancel_button_have_no_esc() {
        for labels in [
            &["はい", "いいえ"][..],
            &["はい(Y)", "いいえ(N)"],
            &["中止", "再試行", "無視"],
            &["消す", "残す"],
        ] {
            assert!(
                kinds(&windows_buttons(labels))
                    .iter()
                    .all(|(kind, _)| *kind == "other"),
                "{labels:?}"
            );
        }
    }

    #[test]
    fn the_buttons_of_other_systems_are_the_plain_ones() {
        if cfg!(windows) {
            return;
        }
        assert_eq!(
            kinds(&buttons(&["OK", "キャンセル"])),
            [kind("ok", "OK"), kind("other", "キャンセル")]
        );
    }

    /// An answer that comes when `ready` is set, and says when it is dropped.
    struct Watched {
        dropped: Rc<Cell<bool>>,
        ready: Rc<Cell<bool>>,
        waker: Rc<RefCell<Option<Waker>>>,
    }

    impl Future for Watched {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            if self.ready.get() {
                Poll::Ready(())
            } else {
                *self.waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    impl Drop for Watched {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }

    #[gpui::test]
    fn a_held_answer_is_kept_until_it_comes(cx: &mut TestAppContext) {
        let dropped = Rc::new(Cell::new(false));
        let ready = Rc::new(Cell::new(false));
        let waker = Rc::new(RefCell::new(None));
        let answer = Watched {
            dropped: dropped.clone(),
            ready: ready.clone(),
            waker: waker.clone(),
        };
        cx.update(|app| hold(answer, app));
        cx.run_until_parked();
        assert!(!dropped.get(), "the answer must not be dropped at once");
        ready.set(true);
        waker.borrow_mut().take().expect("it waited").wake();
        cx.run_until_parked();
        assert!(dropped.get(), "the answer is dropped when it has come");
    }
}
