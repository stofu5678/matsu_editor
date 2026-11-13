//! What the program tells the terminal besides drawing (ADR-0028 point 5): its title, the bell, and text to
//! put on the clipboard (OSC 52).
//!
//! * The title (OSC 0 and 2, already cleaned and cut by `matsu-terminal`) is the caption of the tab and what
//!   the window's title bar shows after `シェル: `; resetting it goes back to the shell's program name.
//! * The bell: Windows Terminal rings the system sound by default, and also marks the tab and flashes the
//!   taskbar when the window is in the background. GPUI has no sound, so the nearest is visual: a bell that
//!   rings while the user is looking at the terminal flashes the screen briefly; a bell that rings while the
//!   tab or the window is in the background marks the caption of the tab (until the tab is looked at again)
//!   and asks the OS for attention (the taskbar flash on Windows, the dock bounce on macOS, the urgency hint
//!   on X11). Bells that come within a second of each other are one.
//! * OSC 52 writes go to the clipboard they name: `c` the system clipboard, `p` and `s` the primary selection
//!   where the platform has one (Linux), else the clipboard. Reads are refused by `matsu-terminal`.

use std::time::{Duration, Instant};

use gpui::{ClipboardItem, Context, Window};
use matsu_terminal::ClipboardKind;

use super::TerminalView;

/// Bells that come closer than this are one.
pub(super) const BELL_QUIET: Duration = Duration::from_secs(1);

/// How long the screen flashes.
const BELL_FLASH: Duration = Duration::from_millis(150);

/// Lets one bell through, then none until [`BELL_QUIET`] has passed since it.
#[derive(Default)]
pub(super) struct BellGate {
    last: Option<Instant>,
}

impl BellGate {
    /// Whether a bell at `now` is a new one (and not the tail of the last).
    pub(super) fn rings(&mut self, now: Instant) -> bool {
        if self
            .last
            .is_some_and(|last| now.saturating_duration_since(last) < BELL_QUIET)
        {
            return false;
        }
        self.last = Some(now);
        true
    }
}

impl TerminalView {
    /// The program set the title: an empty one is none.
    pub(super) fn set_title(&mut self, title: &str, cx: &mut Context<Self>) {
        let title = (!title.trim().is_empty()).then(|| title.to_owned());
        if title != self.title {
            self.title = title;
            cx.notify();
            self.tell_window(cx);
        }
    }

    /// The program reset the title: the shell's program name again.
    pub(super) fn reset_title(&mut self, cx: &mut Context<Self>) {
        if self.title.take().is_some() {
            cx.notify();
            self.tell_window(cx);
        }
    }

    /// The title the program set, if it has.
    #[cfg(test)]
    pub(crate) fn program_title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The text of the tab and of the window's title bar: the program's title, else the shell's name.
    pub(crate) fn title_text(&self) -> &str {
        self.title.as_deref().unwrap_or(&self.program)
    }

    /// The bell rang.
    pub(super) fn on_bell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.bell.rings(Instant::now()) {
            return;
        }
        self.bells_rung += 1;
        if window.is_window_active() && self.focus.is_focused(window) {
            self.bell_flash = true;
            cx.notify();
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(BELL_FLASH).await;
                this.update(cx, |this, cx| {
                    this.bell_flash = false;
                    cx.notify();
                })
                .ok();
            })
            .detach();
        } else {
            if !self.bell_marked {
                self.bell_marked = true;
                self.tell_window(cx);
            }
            // Does nothing while the window is the active one (its tab is simply in the background).
            window.request_attention();
        }
    }

    /// The tab is looked at again: the mark of a bell goes.
    pub(crate) fn clear_bell_mark(&mut self, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.bell_marked) {
            self.tell_window(cx);
        }
    }

    /// A bell rang in the background and the tab has not been looked at since: its caption is marked.
    pub(crate) fn bell_marked(&self) -> bool {
        self.bell_marked
    }

    /// The screen flashes (a bell rang while the terminal was looked at).
    pub(in crate::terminal) fn bell_flashing(&self) -> bool {
        self.bell_flash
    }

    /// How many bells got through (the quick ones after one are not counted; for the tests).
    #[cfg(test)]
    pub(crate) fn bells_rung(&self) -> usize {
        self.bells_rung
    }

    /// The program asked to put text on the clipboard (OSC 52).
    pub(super) fn store_clipboard(kind: ClipboardKind, text: String, cx: &mut Context<Self>) {
        let item = ClipboardItem::new_string(text);
        match kind {
            #[cfg(any(target_os = "linux", target_os = "freebsd"))]
            ClipboardKind::Selection => cx.write_to_primary(item),
            // Systems without a primary selection use the clipboard for it.
            #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
            ClipboardKind::Selection => cx.write_to_clipboard(item),
            ClipboardKind::Clipboard => cx.write_to_clipboard(item),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bells_that_come_quickly_are_one() {
        let mut gate = BellGate::default();
        let start = Instant::now();
        assert!(gate.rings(start));
        assert!(!gate.rings(start + Duration::from_millis(10)));
        assert!(!gate.rings(start + Duration::from_millis(999)));
        // the quiet time counts from the bell that got through, so a stream of bells rings once a second
        assert!(gate.rings(start + BELL_QUIET));
        assert!(!gate.rings(start + BELL_QUIET + Duration::from_millis(1)));
        assert!(gate.rings(start + 2 * BELL_QUIET));
    }

    #[test]
    fn a_clock_that_goes_back_does_not_ring() {
        let mut gate = BellGate::default();
        let start = Instant::now() + Duration::from_secs(10);
        assert!(gate.rings(start));
        assert!(!gate.rings(start - Duration::from_secs(5)));
    }
}
