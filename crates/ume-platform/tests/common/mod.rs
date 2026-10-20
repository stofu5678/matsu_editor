//! Helpers for the tests that drive the native Windows dialogs with UI Automation (UIA).
//!
//! The dialogs run on a thread of their own (`src/dialog/windows.rs`, `src/font_dialog/windows.rs`);
//! the test thread finds the dialog window by its title, reads and changes its controls through
//! UIA patterns, and presses a button. Every wait has a time limit: a dialog that does not show
//! up, or does not close, makes the test fail with a dump of what UIA saw, never hang.
//!
//! Each test file is its own test binary and uses only some of these helpers.
#![cfg(windows)]
#![allow(dead_code, unsafe_code)]

use std::future::Future;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, ExpandCollapseState_Expanded, IUIAutomation, IUIAutomationCondition,
    IUIAutomationElement, IUIAutomationExpandCollapsePattern, IUIAutomationInvokePattern,
    IUIAutomationSelectionItemPattern, IUIAutomationSelectionPattern, IUIAutomationTogglePattern,
    IUIAutomationValuePattern, IUIAutomationWindowPattern, ToggleState_On, TreeScope_Children,
    TreeScope_Descendants, UIA_ButtonControlTypeId, UIA_CheckBoxControlTypeId,
    UIA_ComboBoxControlTypeId, UIA_EditControlTypeId, UIA_ExpandCollapsePatternId,
    UIA_InvokePatternId, UIA_ListItemControlTypeId, UIA_SelectionItemPatternId,
    UIA_SelectionPatternId, UIA_TogglePatternId, UIA_ValuePatternId, UIA_WindowPatternId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CB_FINDSTRINGEXACT, CB_SETCURSEL, CBN_SELCHANGE, GetDlgCtrlID, GetParent, SendMessageW,
    WM_COMMAND,
};
use windows::core::{BSTR, HSTRING};

/// How long to wait for a dialog to show up, and for the API to answer after a button.
pub const LIMIT: Duration = Duration::from_secs(30);

/// Runs a future to its end on the calling thread (the dialogs need no reactor).
pub fn block_on<F: Future>(future: F) -> F::Output {
    struct Unpark(Thread);
    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        thread::park();
    }
}

/// The UIA client of the test thread.
pub struct Uia {
    automation: IUIAutomation,
    all: IUIAutomationCondition,
}

impl Uia {
    /// `None` when this session has no desktop UIA can see (a service session, say): the test
    /// then skips itself with a printed reason.
    pub fn connect() -> Option<Self> {
        // SAFETY: plain COM calls on the test thread. The result of `CoInitializeEx` is not
        // checked on purpose: a thread that is in an apartment already can still use UIA.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
            let all = automation.CreateTrueCondition().ok()?;
            // An interactive desktop has windows at least (the taskbar); a service session's has none.
            let top = automation
                .GetRootElement()
                .ok()?
                .FindAll(TreeScope_Children, &all)
                .ok()?;
            if top.Length().ok()? == 0 {
                return None;
            }
            Some(Self { automation, all })
        }
    }

    fn root(&self) -> IUIAutomationElement {
        // SAFETY: plain call on a live interface.
        unsafe { self.automation.GetRootElement() }.expect("the UIA root element")
    }

    /// The top-level windows of this process.
    pub fn windows_of_this_process(&self) -> Vec<Element> {
        // SAFETY: plain calls on live interfaces.
        unsafe {
            let Ok(windows) = self.root().FindAll(TreeScope_Children, &self.all) else {
                return Vec::new();
            };
            let pid = std::process::id() as i32;
            (0..windows.Length().unwrap_or(0))
                .filter_map(|i| windows.GetElement(i).ok())
                .filter(|w| w.CurrentProcessId().ok() == Some(pid))
                .map(Element)
                .collect()
        }
    }

    /// Waits for a top-level window of this process for which `matches` holds. Fails (with the
    /// windows of this process) after `LIMIT`, or at once if `ended` says that the dialog already
    /// ended.
    pub fn wait_window_by(
        &self,
        what: &str,
        matches: impl Fn(&Uia, &Element) -> bool,
        mut ended: impl FnMut() -> bool,
    ) -> Element {
        let start = Instant::now();
        loop {
            let windows = self.windows_of_this_process();
            if let Some(window) = windows.iter().find(|w| matches(self, w)) {
                return window.clone();
            }
            let gone = ended();
            if start.elapsed() > LIMIT || gone {
                let windows: Vec<String> = windows.iter().map(Element::describe).collect();
                panic!(
                    "the dialog ({what}) did not show up (the call had already returned: {gone}); \
                     the windows of this process:\n{}",
                    windows.join("\n")
                );
            }
            thread::sleep(Duration::from_millis(100));
        }
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

    pub fn automation_id(&self) -> String {
        // SAFETY: plain call on a live interface.
        unsafe { self.0.CurrentAutomationId() }
            .map(|n| n.to_string())
            .unwrap_or_default()
    }

    pub fn is_enabled(&self) -> bool {
        // SAFETY: plain call on a live interface.
        unsafe { self.0.CurrentIsEnabled() }
            .map(|b| b.as_bool())
            .unwrap_or(false)
    }

    fn control_type(&self) -> i32 {
        // SAFETY: plain call on a live interface.
        unsafe { self.0.CurrentControlType() }
            .map(|t| t.0)
            .unwrap_or(0)
    }

    pub fn describe(&self) -> String {
        format!(
            "type={} id={:?} name={:?} value={:?} enabled={}",
            self.control_type(),
            self.automation_id(),
            self.name(),
            self.value(),
            self.is_enabled()
        )
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

    /// The tree below this element, for a failure message.
    pub fn dump(&self, uia: &Uia) -> String {
        self.descendants(uia)
            .iter()
            .map(Element::describe)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The first descendant for which `matches` holds.
    pub fn find(&self, uia: &Uia, what: &str, matches: impl Fn(&Element) -> bool) -> Element {
        self.descendants(uia)
            .into_iter()
            .find(|e| matches(e))
            .unwrap_or_else(|| panic!("no {what} in the dialog; the controls:\n{}", self.dump(uia)))
    }

    /// The text of an edit or a combo box: the Value pattern, or for a selection-only list (the
    /// lists of `IFileDialogCustomize` have no Value pattern) the name of the selected item.
    /// Empty if it has neither.
    pub fn value(&self) -> String {
        // SAFETY: plain calls on a live interface.
        unsafe {
            if let Ok(value) = self
                .0
                .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
                .and_then(|p| p.CurrentValue())
            {
                return value.to_string();
            }
            self.0
                .GetCurrentPatternAs::<IUIAutomationSelectionPattern>(UIA_SelectionPatternId)
                .and_then(|p| p.GetCurrentSelection())
                .and_then(|items| items.GetElement(0))
                .and_then(|item| item.CurrentName())
                .map(|name| name.to_string())
                .unwrap_or_default()
        }
    }

    pub fn set_value(&self, text: &str) {
        // SAFETY: plain calls on a live interface.
        unsafe {
            self.0
                .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
                .and_then(|p| p.SetValue(&BSTR::from(text)))
                .unwrap_or_else(|e| panic!("cannot set {text:?} on {}: {e}", self.describe()));
        }
    }

    /// Presses a button.
    pub fn invoke(&self) {
        // SAFETY: plain calls on a live interface.
        unsafe {
            self.0
                .GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
                .and_then(|p| p.Invoke())
                .unwrap_or_else(|e| panic!("cannot press {}: {e}", self.describe()));
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

    fn list_item(&self, name: &str) -> Option<Element> {
        (self.control_type() == UIA_ListItemControlTypeId.0 && self.name() == name)
            .then(|| self.clone())
    }

    /// Chooses the item `item` of a drop-down list: opens it, selects the list item and closes it.
    pub fn choose(&self, uia: &Uia, item: &str) {
        // SAFETY: plain calls on live interfaces.
        unsafe {
            let expand = self
                .0
                .GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(
                    UIA_ExpandCollapsePatternId,
                )
                .ok();
            if let Some(expand) = &expand {
                let _ = expand.Expand();
            }
            // The items are below the combo box, or in a pop-up window of this process (never another program's).
            let find =
                |from: &Element| from.descendants(uia).iter().find_map(|e| e.list_item(item));
            let start = Instant::now();
            let mut found = find(self);
            while found.is_none() && start.elapsed() < Duration::from_secs(5) {
                thread::sleep(Duration::from_millis(100));
                found = find(self).or_else(|| uia.windows_of_this_process().iter().find_map(&find));
            }
            let found = found.unwrap_or_else(|| {
                panic!(
                    "no list item {item:?} in {}; below it:\n{}",
                    self.describe(),
                    self.dump(uia)
                )
            });
            found
                .0
                .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                    UIA_SelectionItemPatternId,
                )
                .and_then(|p| p.Select())
                .unwrap_or_else(|e| panic!("cannot select {item:?}: {e}"));
            if let Some(expand) = &expand
                && expand.CurrentExpandCollapseState().ok() == Some(ExpandCollapseState_Expanded)
            {
                let _ = expand.Collapse();
            }
        }
    }

    /// Chooses the item `text` of a combo box of a classic dialog the way a click does: the
    /// selection and the `CBN_SELCHANGE` notification to the dialog. The UIA selection pattern
    /// changes the list without the notification, so `ChooseFontW` would not see it.
    pub fn choose_classic(&self, text: &str) {
        // SAFETY: the window is a combo box of a dialog of this process; the string lives through
        // the calls, which are made from the test thread while the dialog thread pumps messages.
        unsafe {
            let combo = self
                .0
                .CurrentNativeWindowHandle()
                .unwrap_or_else(|e| panic!("{} has no window: {e}", self.describe()));
            let wide = HSTRING::from(text);
            let index = SendMessageW(
                combo,
                CB_FINDSTRINGEXACT,
                Some(WPARAM(usize::MAX)),
                Some(LPARAM(wide.as_ptr() as isize)),
            )
            .0;
            assert!(index >= 0, "{text:?} is not in {}", self.describe());
            SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(index as usize)), None);
            let parent = GetParent(combo).expect("the dialog of the combo box");
            let id = GetDlgCtrlID(combo) as usize;
            SendMessageW(
                parent,
                WM_COMMAND,
                Some(WPARAM((CBN_SELCHANGE as usize) << 16 | id)),
                Some(LPARAM(combo.0 as isize)),
            );
        }
    }

    /// Closes the window (the window pattern), for cleaning up.
    pub fn close(&self) {
        // SAFETY: plain calls on a live interface.
        unsafe {
            if let Ok(window) = self
                .0
                .GetCurrentPatternAs::<IUIAutomationWindowPattern>(UIA_WindowPatternId)
            {
                let _ = window.Close();
            }
        }
    }

    pub fn is_check_box(&self) -> bool {
        self.control_type() == UIA_CheckBoxControlTypeId.0
    }

    pub fn is_combo_box(&self) -> bool {
        self.control_type() == UIA_ComboBoxControlTypeId.0
    }

    pub fn is_edit(&self) -> bool {
        self.control_type() == UIA_EditControlTypeId.0
    }

    pub fn is_button(&self) -> bool {
        self.control_type() == UIA_ButtonControlTypeId.0
    }
}

/// A dialog that runs on a thread of its own, the way the application shows it: the answer of
/// the API comes through a channel.
pub struct Running<T> {
    answer: Receiver<T>,
    window: Option<Element>,
}

impl<T: Send + 'static> Running<T> {
    pub fn start(show: impl Future<Output = T> + Send + 'static) -> Self {
        let (sender, answer) = mpsc::channel();
        thread::Builder::new()
            .name("test dialog caller".to_owned())
            .spawn(move || {
                sender.send(block_on(show)).ok();
            })
            .expect("a thread for the dialog");
        Self {
            answer,
            window: None,
        }
    }

    /// Waits for the dialog window with `title`.
    pub fn wait_window(&mut self, uia: &Uia, title: &str) -> Element {
        self.wait_window_by(uia, title, |_, w| w.name() == title)
    }

    /// Waits for the dialog window for which `matches` holds (`what` names it in the failure).
    pub fn wait_window_by(
        &mut self,
        uia: &Uia,
        what: &str,
        matches: impl Fn(&Uia, &Element) -> bool,
    ) -> Element {
        let answer = &self.answer;
        let window = uia.wait_window_by(what, matches, || {
            // an answer cannot be put back, but the test fails then anyway
            !matches!(answer.try_recv(), Err(mpsc::TryRecvError::Empty))
        });
        self.window = Some(window.clone());
        window
    }

    /// Waits for the answer of the API (the dialog was closed by a button).
    pub fn finish(mut self) -> T {
        match self.answer.recv_timeout(LIMIT) {
            Ok(answer) => {
                // the dialog closed itself; on a timeout `drop` closes it
                self.window = None;
                answer
            }
            Err(RecvTimeoutError::Timeout) => panic!("the dialog did not close within {LIMIT:?}"),
            Err(RecvTimeoutError::Disconnected) => panic!("the dialog thread ended without answer"),
        }
    }
}

impl<T> Drop for Running<T> {
    /// A test that fails while the dialog is open closes the dialog, so that nothing stays on
    /// the desktop of the runner.
    fn drop(&mut self) {
        if let Some(window) = &self.window {
            window.close();
        }
    }
}
