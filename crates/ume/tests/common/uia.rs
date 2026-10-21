//! A small UI Automation (UIA) client for the tests that drive the window of the built `ume`
//! program from outside, by the accessibility names of its controls, the way a user (or a screen
//! reader) does. The window belongs to another process, so every search starts from the
//! top-level windows of its process id.
//!
//! Every wait has a time limit: an element that does not show up makes the test fail with a dump
//! of what UIA saw, never hang.
#![cfg(windows)]
#![allow(dead_code, unsafe_code)]

use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationCondition, IUIAutomationElement,
    IUIAutomationInvokePattern, IUIAutomationSelectionItemPattern, IUIAutomationTogglePattern,
    ToggleState_On, TreeScope_Children, TreeScope_Descendants, UIA_ButtonControlTypeId,
    UIA_CheckBoxControlTypeId, UIA_InvokePatternId, UIA_SelectionItemPatternId,
    UIA_TabItemControlTypeId, UIA_TogglePatternId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    PostMessageW, WM_CHAR, WM_CLOSE, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
};

/// How long to wait for an element to show up.
pub const LIMIT: Duration = Duration::from_secs(60);

/// The UIA client of the test thread.
pub struct Uia {
    automation: IUIAutomation,
    all: IUIAutomationCondition,
}

impl Uia {
    /// Whether the tests may drive windows here: on CI (`CI` is set; GitHub Actions sets it on every
    /// runner) or when `UME_UIA_TESTS=1` asks for it. On a developer's own machine they would open
    /// windows and send keys to the one in front.
    fn allowed() -> bool {
        std::env::var_os("CI").is_some() || std::env::var("UME_UIA_TESTS").is_ok_and(|v| v == "1")
    }

    /// `None` when the test must not run here, with the reason printed.
    pub fn connect() -> Option<Self> {
        if !Self::allowed() {
            eprintln!(
                "SKIPPED: this test opens real windows; set UME_UIA_TESTS=1 to run it on this \
                 machine (it always runs on CI)"
            );
            return None;
        }
        // SAFETY: plain COM calls on the test thread. The result of `CoInitializeEx` is not
        // checked on purpose: a thread that is in an apartment already can still use UIA.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
            let all = automation.CreateTrueCondition().ok()?;
            let top = automation
                .GetRootElement()
                .ok()?
                .FindAll(TreeScope_Children, &all)
                .ok()?;
            if top.Length().ok()? == 0 {
                eprintln!("SKIPPED: this session has no desktop that UI Automation can see");
                return None;
            }
            Some(Self { automation, all })
        }
    }

    /// The top-level windows of the process `pid`.
    pub fn windows_of(&self, pid: u32) -> Vec<Element> {
        // SAFETY: plain calls on live interfaces.
        unsafe {
            let Ok(root) = self.automation.GetRootElement() else {
                return Vec::new();
            };
            let Ok(windows) = root.FindAll(TreeScope_Children, &self.all) else {
                return Vec::new();
            };
            (0..windows.Length().unwrap_or(0))
                .filter_map(|i| windows.GetElement(i).ok())
                .filter(|w| w.CurrentProcessId().ok() == Some(pid as i32))
                .map(Element)
                .collect()
        }
    }

    /// The first element, in the windows of `pid`, for which `matches` holds, with its window.
    pub fn find(&self, pid: u32, matches: impl Fn(&Element) -> bool) -> Option<(Element, Element)> {
        for window in self.windows_of(pid) {
            if let Some(found) = window.descendants(self).into_iter().find(|e| matches(e)) {
                return Some((window, found));
            }
        }
        None
    }

    /// Waits for such an element. Fails after `LIMIT` with the trees of the windows of `pid`.
    pub fn wait(
        &self,
        pid: u32,
        what: &str,
        matches: impl Fn(&Element) -> bool,
    ) -> (Element, Element) {
        let start = Instant::now();
        loop {
            if let Some(found) = self.find(pid, &matches) {
                return found;
            }
            if start.elapsed() > LIMIT {
                panic!("no {what} showed up; {}", self.dump(pid));
            }
            thread::sleep(Duration::from_millis(200));
        }
    }

    /// Waits until no element matches.
    pub fn wait_gone(&self, pid: u32, what: &str, matches: impl Fn(&Element) -> bool) {
        let start = Instant::now();
        while self.find(pid, &matches).is_some() {
            if start.elapsed() > LIMIT {
                panic!("{what} did not go away; {}", self.dump(pid));
            }
            thread::sleep(Duration::from_millis(200));
        }
    }

    /// What UIA sees of the windows of `pid`, for a message.
    pub fn dump(&self, pid: u32) -> String {
        let windows = self.windows_of(pid);
        if windows.is_empty() {
            return format!("the process {pid} has no window");
        }
        windows
            .iter()
            .map(|w| format!("window {}\n{}", w.describe(), w.dump(self)))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// One element of the UIA tree.
#[derive(Clone)]
pub struct Element(IUIAutomationElement);

impl Element {
    pub fn name(&self) -> String {
        // SAFETY: plain call on a live interface.
        unsafe { self.0.CurrentName() }
            .map(|n| n.to_string())
            .unwrap_or_default()
    }

    fn control_type(&self) -> i32 {
        // SAFETY: plain call on a live interface.
        unsafe { self.0.CurrentControlType() }
            .map(|t| t.0)
            .unwrap_or(0)
    }

    pub fn is_enabled(&self) -> bool {
        // SAFETY: plain call on a live interface.
        unsafe { self.0.CurrentIsEnabled() }
            .map(|b| b.as_bool())
            .unwrap_or(false)
    }

    pub fn is_button(&self) -> bool {
        self.control_type() == UIA_ButtonControlTypeId.0
    }

    pub fn is_check_box(&self) -> bool {
        self.control_type() == UIA_CheckBoxControlTypeId.0
    }

    pub fn is_tab_item(&self) -> bool {
        self.control_type() == UIA_TabItemControlTypeId.0
    }

    pub fn describe(&self) -> String {
        format!("type={} name={:?}", self.control_type(), self.name())
    }

    /// Every descendant, in tree order.
    pub fn descendants(&self, uia: &Uia) -> Vec<Element> {
        // SAFETY: plain calls on live interfaces.
        unsafe {
            let Ok(all) = self.0.FindAll(TreeScope_Descendants, &uia.all) else {
                return Vec::new();
            };
            (0..all.Length().unwrap_or(0))
                .filter_map(|i| all.GetElement(i).ok())
                .map(Element)
                .collect()
        }
    }

    /// The tree below this element, for a message.
    pub fn dump(&self, uia: &Uia) -> String {
        self.descendants(uia)
            .iter()
            .map(|e| format!("  {}", e.describe()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The window handle of a top-level window.
    pub fn handle(&self) -> HWND {
        // SAFETY: plain call on a live interface.
        unsafe { self.0.CurrentNativeWindowHandle() }
            .unwrap_or_else(|e| panic!("{} has no window handle: {e}", self.describe()))
    }

    /// Presses a button (or opens a menu).
    pub fn invoke(&self) {
        // SAFETY: plain calls on a live interface.
        unsafe {
            self.0
                .GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
                .and_then(|p| p.Invoke())
                .unwrap_or_else(|e| panic!("cannot press {}: {e}", self.describe()));
        }
    }

    /// Clicks the middle of the element with the left button, as mouse messages to the window
    /// `window` (a top-level window of the process): a move, then the button down and up. The
    /// invoke pattern of an item of a pop-up menu does nothing in the window of UME, which wants
    /// the mouse (a click handler), so this does what a user does.
    pub fn click_in(&self, window: &Element) {
        // SAFETY: plain calls on a live interface; the messages go to a window of another process.
        unsafe {
            let rect = self
                .0
                .CurrentBoundingRectangle()
                .unwrap_or_else(|e| panic!("{} has no rectangle: {e}", self.describe()));
            let hwnd = window.handle();
            let mut point = POINT {
                x: (rect.left + rect.right) / 2,
                y: (rect.top + rect.bottom) / 2,
            };
            let _ = ScreenToClient(hwnd, &mut point);
            let at = LPARAM(((point.y as u16 as isize) << 16) | (point.x as u16 as isize));
            let post = |message, wparam: usize| {
                PostMessageW(Some(hwnd), message, WPARAM(wparam), at)
                    .unwrap_or_else(|e| panic!("cannot post a mouse message: {e}"));
            };
            post(WM_MOUSEMOVE, 0);
            thread::sleep(Duration::from_millis(150));
            post(WM_LBUTTONDOWN, 1);
            thread::sleep(Duration::from_millis(80));
            post(WM_LBUTTONUP, 0);
        }
    }

    /// Chooses a tab (the selection pattern), or presses it when it has no such pattern.
    pub fn select(&self) {
        // SAFETY: plain calls on a live interface.
        unsafe {
            let selected = self
                .0
                .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                    UIA_SelectionItemPatternId,
                )
                .and_then(|p| p.Select());
            if let Err(e) = selected {
                self.0
                    .GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
                    .and_then(|p| p.Invoke())
                    .unwrap_or_else(|e2| {
                        panic!(
                            "cannot choose {}: {e}; and cannot press it: {e2}",
                            self.describe()
                        )
                    });
            }
        }
    }

    /// Flips a check box.
    pub fn toggle(&self) {
        // SAFETY: plain calls on a live interface.
        unsafe {
            self.0
                .GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)
                .and_then(|p| p.Toggle())
                .unwrap_or_else(|e| panic!("cannot toggle {}: {e}", self.describe()));
        }
    }

    /// Whether a check box is on.
    pub fn is_checked(&self) -> bool {
        // SAFETY: plain calls on a live interface.
        unsafe {
            self.0
                .GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)
                .and_then(|p| p.CurrentToggleState())
                .unwrap_or_else(|e| panic!("{} has no toggle state: {e}", self.describe()))
                == ToggleState_On
        }
    }

    /// Types `text` into the window as characters (`WM_CHAR`, one per UTF-16 unit), which needs
    /// no keyboard focus: the keys of `SendInput` do not reach a window on some runners (the
    /// Windows ARM64 ones, see `crates/ume-platform/tests/common`).
    pub fn post_chars(&self, text: &str) {
        let window = self.handle();
        for unit in text.encode_utf16() {
            // SAFETY: a plain message to a window of another process.
            unsafe {
                PostMessageW(Some(window), WM_CHAR, WPARAM(unit as usize), LPARAM(1))
                    .unwrap_or_else(|e| panic!("cannot post a character: {e}"));
            }
        }
    }

    /// Asks the window to close, like the close button (`WM_CLOSE`).
    pub fn post_close(&self) {
        // SAFETY: a plain message to a window of another process.
        unsafe {
            PostMessageW(Some(self.handle()), WM_CLOSE, WPARAM(0), LPARAM(0))
                .unwrap_or_else(|e| panic!("cannot close the window: {e}"));
        }
    }
}
