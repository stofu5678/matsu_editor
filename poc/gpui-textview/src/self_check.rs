//! `--self-check <dir>`, built with `--features self-check`: goes through the PoC's keys, saves
//! what the window shows as PNG files in `<dir>`, prints what happened, and quits. It checks the
//! PoC where nobody watches the screen, with no macOS screen-recording or accessibility
//! permission.
//!
//! On macOS the keys go in as macOS key events, made as the keyboard makes them, so they also
//! check GPUI's reading of the keyboard layout (US or JIS). Elsewhere they go through
//! `Window::dispatch_keystroke`. The IME is played by calling the input handler as an IME does;
//! whether a real IME behaves is for a person to check.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use anyhow::{Context as _, Result, ensure};
use gpui::{AnyWindowHandle, App, AsyncApp, Bounds, EntityInputHandler, Global, WeakEntity};

use crate::{Editor, FONTS};

/// The window's editor, which the check looks at.
pub struct Target(pub WeakEntity<Editor>);

impl Global for Target {}

/// Takes `--self-check <dir>` from the command line.
pub fn take_arg() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--self-check" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

pub fn run(dir: PathBuf, cx: &mut App) {
    cx.spawn(async move |cx| {
        match drive(&dir, cx).await {
            Ok(()) => eprintln!("self-check: ok"),
            Err(err) => eprintln!("self-check: FAILED: {err:#}"),
        }
        cx.update(|cx| cx.quit());
    })
    .detach();
}

async fn drive(dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let window = cx
        .update(|cx| cx.windows().first().copied())
        .context("no window")?;
    let editor = cx
        .update(|cx| cx.try_global::<Target>().and_then(|t| t.0.upgrade()))
        .context("no editor")?;
    settle(cx).await;
    eprintln!("self-check: memory at start: {}", memory());
    save(window, &dir.join("opened.png"), cx)?;

    #[cfg(target_os = "macos")]
    {
        window.update(cx, |_, window, _| window.activate_window())?;
        settle(cx).await;
        let key_window = window.update(cx, |_, window, _| mac::is_key_window(window))??;
        ensure!(
            key_window,
            "the window is not the key window, so it gets no key events"
        );
        let (keyboard, jis) = mac::keyboard();
        eprintln!(
            "self-check: macOS key events from a {} keyboard (type {keyboard})",
            if jis { "JIS" } else { "US" }
        );
    }
    let keystrokes = Rc::new(RefCell::new(Vec::new()));
    let _subscription = cx.update(|cx| {
        let keystrokes = keystrokes.clone();
        cx.observe_keystrokes(move |event, _, _| {
            let action = event.action.as_ref().map_or("none", |action| action.name());
            keystrokes
                .borrow_mut()
                .push(format!("{} → {action}", event.keystroke.unparse()));
        })
    });
    let seen = || {
        keystrokes
            .borrow_mut()
            .drain(..)
            .collect::<Vec<_>>()
            .join(", ")
    };

    // F7 goes round the fonts.
    for (i, name) in FONTS.iter().enumerate() {
        save(window, &dir.join(format!("font-{i}.png")), cx)?;
        press(window, "f7", cx).await?;
        let font = cx.update(|cx| editor.read(cx).font_index);
        eprintln!("self-check:   f7 = {}: {} → {}", seen(), name, FONTS[font]);
        ensure!(font == (i + 1) % FONTS.len(), "F7 did not switch the font");
    }
    eprintln!("self-check: memory after the fonts: {}", memory());

    // Cmd on a Mac and Ctrl elsewhere; Control on a Mac too.
    for (keystroke, step) in [
        ("secondary--", -1.),
        ("secondary-=", 1.),
        ("secondary-+", 1.),
        ("ctrl--", -1.),
        ("ctrl-=", 1.),
        ("ctrl-+", 1.),
    ] {
        let before = cx.update(|cx| editor.read(cx).font_size);
        press(window, keystroke, cx).await?;
        let after = cx.update(|cx| editor.read(cx).font_size);
        eprintln!(
            "self-check:   {keystroke} = {}: {before} → {after} pt",
            seen()
        );
        ensure!(
            after == before + step,
            "{keystroke} did not change the font size"
        );
    }

    // The IME composes "にほんご", converts it to "日本語", and commits it.
    let row = cx.update(|cx| editor.read(cx).cursor.0);
    let candidate_bounds = window.update(cx, |_, window, cx| {
        editor.update(cx, |editor, cx| {
            editor.replace_and_mark_text_in_range(None, "にほんご", Some(4..4), window, cx);
            editor.replace_and_mark_text_in_range(None, "日本語", Some(0..3), window, cx);
            let marked = editor.marked_text_range(window, cx)?;
            editor.bounds_for_range(marked, Bounds::default(), window, cx)
        })
    })?;
    eprintln!("self-check: the IME candidate window goes near {candidate_bounds:?}");
    ensure!(
        candidate_bounds.is_some(),
        "no bounds for the composed text"
    );
    settle(cx).await;
    save(window, &dir.join("composing.png"), cx)?;
    window.update(cx, |_, window, cx| {
        editor.update(cx, |editor, cx| {
            editor.replace_text_in_range(None, "日本語", window, cx);
        })
    })?;
    let line = cx.update(|cx| editor.read(cx).lines[row].clone());
    ensure!(
        line == "日本語",
        "the composed text was not committed: {line:?}"
    );

    // F8 makes a million lines.
    press(window, "f8", cx).await?;
    let lines = cx.update(|cx| editor.read(cx).lines.len());
    eprintln!("self-check:   f8 = {}: {lines} lines", seen());
    ensure!(lines == 1_000_000, "F8 did not make a million lines");
    save(window, &dir.join("large.png"), cx)?;
    #[cfg(target_os = "macos")]
    {
        press(window, "cmd-down", cx).await?;
        let row = cx.update(|cx| editor.read(cx).cursor.0);
        eprintln!("self-check:   cmd-down = {}: line {}", seen(), row + 1);
        ensure!(row == lines - 1, "Cmd+Down did not go to the end");
        save(window, &dir.join("large-end.png"), cx)?;
    }
    eprintln!("self-check: memory at the end: {}", memory());
    Ok(())
}

/// Presses a key and waits for the window to draw the result.
async fn press(window: AnyWindowHandle, keystroke: &str, cx: &mut AsyncApp) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let _ = window;
        mac::post(keystroke)?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let keystroke = gpui::Keystroke::parse(keystroke)?;
        window.update(cx, |_, window, cx| window.dispatch_keystroke(keystroke, cx))?;
    }
    settle(cx).await;
    Ok(())
}

/// Waits for the window to draw the latest state.
async fn settle(cx: &mut AsyncApp) {
    cx.background_executor()
        .timer(Duration::from_millis(500))
        .await;
}

fn save(window: AnyWindowHandle, path: &Path, cx: &mut AsyncApp) -> Result<()> {
    let image = window.update(cx, |_, window, _| window.render_to_image())??;
    image.save(path)?;
    eprintln!("self-check: saved {}", path.display());
    Ok(())
}

/// The memory of this process: on macOS, the physical footprint, which Activity Monitor shows
/// as Memory; elsewhere only the Rust heap (`POC_MEMSTAT` has the rest on Windows).
fn memory() -> String {
    let heap = crate::memstat::report();
    #[cfg(target_os = "macos")]
    if let Some(bytes) = mac::footprint() {
        return format!(
            "footprint {:.0} MB, {}",
            bytes as f64 / 1024. / 1024.,
            heap.trim()
        );
    }
    heap.trim().to_owned()
}

/// Key presses as macOS key events in the app's own event queue, which needs no permission.
/// They carry the key code, the modifier flags, and the keyboard type, as the keyboard's do, so
/// macOS works out what they type from the keyboard layout. Keys that macOS takes before any app
/// sees them are not covered: on a Mac keyboard, F1 to F12 without fn set the brightness, the
/// volume and so on.
#[cfg(target_os = "macos")]
#[allow(deprecated, unexpected_cfgs)]
mod mac {
    use std::ffi::c_void;

    use anyhow::{Result, anyhow, bail};
    use cocoa::appkit::{NSApp, NSApplication};
    use cocoa::base::{BOOL, NO, YES, id};
    use gpui::Window;
    use objc::{class, msg_send, sel, sel_impl};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    // Virtual key codes (kVK_* in HIToolbox/Events.h) name a place on the keyboard, whatever
    // it types.
    /// = on a US keyboard, ^ on a JIS one.
    const KEY_EQUAL: u16 = 0x18;
    /// - on both; Shift+- is = on a JIS keyboard.
    const KEY_MINUS: u16 = 0x1b;
    /// Shift+; is + on a JIS keyboard.
    const KEY_SEMICOLON: u16 = 0x29;
    const KEY_F7: u16 = 0x62;
    const KEY_F8: u16 = 0x64;
    const KEY_DOWN: u16 = 0x7d;

    // Modifier flags (CGEventFlags; NSEvent's are the same bits).
    const SHIFT: u64 = 0x2_0000;
    const CTRL: u64 = 0x4_0000;
    const CMD: u64 = 0x10_0000;
    /// Set on the arrow keys' events.
    const NUMPAD: u64 = 0x20_0000;
    /// Set on the F keys' and the arrow keys' events.
    const FN: u64 = 0x80_0000;
    /// kCGKeyboardEventKeyboardType: the kind of keyboard (US, JIS, ...) the key is on.
    const KEYBOARD_TYPE: u32 = 10;
    /// kCGEventSourceStateHIDSystemState: what the keyboards themselves report.
    const HID_SYSTEM_STATE: i32 = 1;

    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn KBGetLayoutType(keyboard_type: i16) -> u32;
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceCreate(state: i32) -> *mut c_void;
        fn CGEventSourceGetKeyboardType(source: *const c_void) -> u32;
        fn CGEventCreateKeyboardEvent(source: *const c_void, key: u16, down: bool) -> *mut c_void;
        fn CGEventSetFlags(event: *mut c_void, flags: u64);
        fn CGEventSetIntegerValueField(event: *mut c_void, field: u32, value: i64);
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(object: *const c_void);
    }

    /// The start of `rusage_info_v0` in <sys/resource.h>.
    #[repr(C)]
    struct RusageInfo {
        uuid: [u8; 16],
        user_time: u64,
        system_time: u64,
        pkg_idle_wkups: u64,
        interrupt_wkups: u64,
        pageins: u64,
        wired_size: u64,
        resident_size: u64,
        phys_footprint: u64,
        proc_start_abstime: u64,
        proc_exit_abstime: u64,
    }

    unsafe extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut c_void) -> i32;
    }

    /// This process's physical footprint in bytes (RUSAGE_INFO_V0).
    pub fn footprint() -> Option<u64> {
        let mut info = std::mem::MaybeUninit::<RusageInfo>::zeroed();
        let result =
            unsafe { proc_pid_rusage(std::process::id() as i32, 0, info.as_mut_ptr().cast()) };
        (result == 0).then(|| unsafe { info.assume_init() }.phys_footprint)
    }

    /// The keyboard that was typed on last, and whether it is JIS. Other than JIS, the US
    /// layout is assumed.
    pub fn keyboard() -> (u32, bool) {
        let keyboard = unsafe {
            let source = CGEventSourceCreate(HID_SYSTEM_STATE);
            let keyboard = CGEventSourceGetKeyboardType(source);
            CFRelease(source);
            keyboard
        };
        let jis = unsafe { KBGetLayoutType(keyboard as i16) } == u32::from_be_bytes(*b"JIS ");
        (keyboard, jis)
    }

    /// Puts the key's down and up events in the app's queue, for the key window.
    pub fn post(keystroke: &str) -> Result<()> {
        let (keyboard, jis) = keyboard();
        let equal = if jis {
            (KEY_MINUS, SHIFT)
        } else {
            (KEY_EQUAL, 0)
        };
        let plus = if jis {
            (KEY_SEMICOLON, SHIFT)
        } else {
            (KEY_EQUAL, SHIFT)
        };
        let ((code, shift), flags) = match keystroke {
            "f7" => ((KEY_F7, 0), FN),
            "f8" => ((KEY_F8, 0), FN),
            "secondary-=" => (equal, CMD),
            "secondary-+" => (plus, CMD),
            "secondary--" => ((KEY_MINUS, 0), CMD),
            "ctrl-=" => (equal, CTRL),
            "ctrl-+" => (plus, CTRL),
            "ctrl--" => ((KEY_MINUS, 0), CTRL),
            "cmd-down" => ((KEY_DOWN, 0), CMD | NUMPAD | FN),
            _ => bail!("no macOS key event for {keystroke}"),
        };
        for down in [true, false] {
            unsafe {
                let event = CGEventCreateKeyboardEvent(std::ptr::null(), code, down);
                CGEventSetFlags(event, flags | shift);
                CGEventSetIntegerValueField(event, KEYBOARD_TYPE, keyboard.into());
                let ns_event: id = msg_send![class!(NSEvent), eventWithCGEvent: event];
                NSApp().postEvent_atStart_(ns_event, NO);
                CFRelease(event);
            }
        }
        Ok(())
    }

    pub fn is_key_window(window: &Window) -> Result<bool> {
        let handle = HasWindowHandle::window_handle(window)
            .map_err(|err| anyhow!("no handle for the window: {err}"))?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            bail!("not an AppKit window");
        };
        let view = handle.ns_view.as_ptr() as id;
        let key: BOOL = unsafe {
            let ns_window: id = msg_send![view, window];
            msg_send![ns_window, isKeyWindow]
        };
        Ok(key == YES)
    }
}
