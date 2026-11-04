//! Tests of what is left of the outline pane in the window: the position and the size of the
//! floating pane (`bRememberOutlineWindowPos` and the keys that go with it), docking by dragging
//! the title, the settings of the type that follow the tab, and すべて展開.

use matsu_config::Profile;

use super::dock::{FloatRect, OutlineDock, write_float};

fn profile(text: &str) -> Profile {
    Profile::from_bytes(text.as_bytes())
}

const FLOAT_KEYS: &str = "[Common]\r\nbRememberOutlineWindowPos=1\r\nwidthOutlineWindow=320\r\n\
     heightOutlineWindow=410\r\nxOutlineWindowPos=-12\r\nyOutlineWindowPos=34\r\n";

#[test]
fn the_floating_keys_are_not_set_by_default() {
    // CShareData::InitShareData leaves them at 0 (`GetDropRect`: 初期値だと 0 になっている)
    let dock = OutlineDock::from_profile(&Profile::new());
    assert!(!dock.remember);
    assert_eq!(dock.float, FloatRect::default());
    assert!(!dock.float.has_size());
    assert_eq!(dock.float_if_remembered(), None);
}

#[test]
fn the_position_and_the_size_are_read_when_the_box_is_on() {
    let dock = OutlineDock::from_profile(&profile(FLOAT_KEYS));
    assert!(dock.remember);
    assert_eq!(
        dock.float,
        FloatRect {
            x: -12,
            y: 34,
            w: 320,
            h: 410
        }
    );
    assert_eq!(dock.float_if_remembered(), Some(dock.float));
}

#[test]
fn the_position_and_the_size_are_not_read_or_used_when_the_box_is_off() {
    // `ShareData_IO_Common` reads the four keys only inside `if( m_bRememberOutlineWindowPos )`
    let text = FLOAT_KEYS.replace("bRememberOutlineWindowPos=1", "bRememberOutlineWindowPos=0");
    let dock = OutlineDock::from_profile(&profile(&text));
    assert!(!dock.remember);
    assert_eq!(dock.float, FloatRect::default());
    assert_eq!(dock.float_if_remembered(), None);
    // and a file without the box is the same
    let dock = OutlineDock::from_profile(&profile(
        "[Common]\r\nwidthOutlineWindow=320\r\nheightOutlineWindow=410\r\n",
    ));
    assert_eq!(dock.float_if_remembered(), None);
}

#[test]
fn a_size_of_zero_is_not_set() {
    let dock = OutlineDock::from_profile(&profile(
        "[Common]\r\nbRememberOutlineWindowPos=1\r\nwidthOutlineWindow=320\r\n",
    ));
    assert!(dock.remember);
    assert_eq!(dock.float.w, 320);
    assert_eq!(dock.float_if_remembered(), None);
}

#[test]
fn the_floating_keys_are_written_with_the_box_on_and_the_box_alone_with_it_off() {
    let mut p = Profile::new();
    let mut dock = OutlineDock {
        remember: true,
        float: FloatRect {
            x: 5,
            y: 6,
            w: 300,
            h: 200,
        },
        ..OutlineDock::default()
    };
    assert!(write_float(&mut p, &dock).unwrap());
    assert_eq!(p.get("Common", "bRememberOutlineWindowPos"), Some("1"));
    assert_eq!(p.get("Common", "xOutlineWindowPos"), Some("5"));
    assert_eq!(p.get("Common", "yOutlineWindowPos"), Some("6"));
    assert_eq!(p.get("Common", "widthOutlineWindow"), Some("300"));
    assert_eq!(p.get("Common", "heightOutlineWindow"), Some("200"));
    assert_eq!(OutlineDock::from_profile(&p), dock);
    assert!(!write_float(&mut p, &dock).unwrap());
    // off: the box is written, the four keys are not touched (the file keeps what it has)
    dock.remember = false;
    dock.float.w = 999;
    assert!(write_float(&mut p, &dock).unwrap());
    assert_eq!(p.get("Common", "bRememberOutlineWindowPos"), Some("0"));
    assert_eq!(p.get("Common", "widthOutlineWindow"), Some("300"));
    // and a profile that never had the keys does not get them
    let mut fresh = Profile::new();
    assert!(write_float(&mut fresh, &dock).unwrap());
    assert_eq!(fresh.get("Common", "widthOutlineWindow"), None);
    assert_eq!(fresh.get("Common", "xOutlineWindowPos"), None);
}

#[test]
fn what_was_read_of_the_floating_keys_is_written_back_without_touching_the_file() {
    let mut p = profile(FLOAT_KEYS);
    let before = p.to_bytes();
    let dock = OutlineDock::from_profile(&p);
    assert!(!write_float(&mut p, &dock).unwrap());
    assert_eq!(p.to_bytes(), before);
}
