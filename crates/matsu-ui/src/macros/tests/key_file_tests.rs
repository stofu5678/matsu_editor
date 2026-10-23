//! Tests of `RecKey.mac`: stopping a recording writes it in the macro folder (Sakura:
//! `Command_RECKEYMACRO`, `CShareData::GetMacroFilename( -1 )`), every window plays it, and
//! `InsText` is a function of key macros.

use std::path::Path;

use gpui::TestAppContext;
use matsu_commands::command::Arg;
use matsu_commands::key_macro::KeyMacro;
use matsu_config::Profile;

use super::{EXEC, F_EXECEXTMACRO, REC, setup, text_arg, write};
use crate::settings::UserSettings;

const F_INSTEXT_W: u16 = 30606;

/// Makes `ini` the settings file, with the macro folder `folder` as written (`None`: not written).
fn settings(cx: &mut TestAppContext, ini: &Path, folder: Option<&str>) {
    let text = match folder {
        Some(folder) => format!("[Folders]\nszMACROFOLDER={folder}\n"),
        None => "[Common]\nnothing=1\n".to_owned(),
    };
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    let profile = Profile::from_bytes(&bytes);
    let ini = ini.to_path_buf();
    cx.update(|app| app.set_global(UserSettings::from_profile(profile, Some(ini))));
}

fn calls_in(path: &Path) -> usize {
    KeyMacro::load(path).unwrap().steps().len()
}

#[gpui::test]
fn stopping_a_recording_writes_reckey_mac_in_an_absolute_macro_folder(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let macros = dir.path().join("my macros");
    std::fs::create_dir(&macros).unwrap();
    settings(cx, &dir.path().join("matsu.ini"), macros.to_str());
    let (_f, cx) = setup(cx, "");
    cx.simulate_keystrokes(REC);
    cx.simulate_input("あb");
    assert!(!macros.join("RecKey.mac").exists(), "only a stop writes it");
    cx.simulate_keystrokes(REC);
    assert_eq!(calls_in(&macros.join("RecKey.mac")), 2);
}

#[gpui::test]
fn a_relative_macro_folder_is_taken_from_the_folder_of_the_settings(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("macros")).unwrap();
    settings(cx, &dir.path().join("matsu.ini"), Some("macros"));
    let (_f, cx) = setup(cx, "");
    cx.simulate_keystrokes(REC);
    cx.simulate_input("a");
    cx.simulate_keystrokes(REC);
    assert_eq!(calls_in(&dir.path().join("macros").join("RecKey.mac")), 1);
}

#[gpui::test]
fn without_a_macro_folder_the_file_is_in_the_folder_of_the_settings(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    settings(cx, &dir.path().join("matsu.ini"), None);
    let (_f, cx) = setup(cx, "");
    cx.simulate_keystrokes(REC);
    cx.simulate_input("a");
    cx.simulate_keystrokes(REC);
    assert_eq!(calls_in(&dir.path().join("RecKey.mac")), 1);
}

#[gpui::test]
fn another_window_plays_the_recording_from_the_file(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    settings(cx, &dir.path().join("matsu.ini"), dir.path().to_str());
    {
        let (_f, cx) = setup(cx, "");
        cx.simulate_keystrokes(REC);
        cx.simulate_input("ab");
        cx.simulate_keystrokes(REC);
    }
    // The file is what is played: change it, and the other window plays the change.
    write(dir.path(), "RecKey.mac", "Char(88);\n");
    let (f, cx) = setup(cx, "z");
    cx.simulate_keystrokes(EXEC);
    assert_eq!(f.lines(cx), ["Xz"], "{}", f.output(cx));
}

#[gpui::test]
fn a_macro_folder_that_cannot_be_written_is_an_error(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    settings(cx, &dir.path().join("matsu.ini"), missing.to_str());
    let (f, cx) = setup(cx, "");
    cx.simulate_keystrokes(REC);
    cx.simulate_input("a");
    cx.simulate_keystrokes(REC);
    assert!(!missing.exists());
    let output = f.output(cx);
    assert!(
        output.contains("マクロファイルを作成できませんでした。"),
        "{output}"
    );
    assert!(output.contains("RecKey.mac"), "{output}");
    assert!(!f.view.read_with(cx, |v, _| v.is_recording_key_macro()));
    // Like Sakura, playing goes to the file that was named, which is not there.
    cx.simulate_keystrokes(EXEC);
    assert_eq!(f.lines(cx), ["a"]);
    assert!(f.output(cx).matches("RecKey.mac").count() >= 2);
}

#[gpui::test]
fn a_recording_without_a_folder_stays_in_memory(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    cx.simulate_keystrokes(REC);
    cx.simulate_input("ab");
    cx.simulate_keystrokes(REC);
    assert_eq!(f.output(cx), "");
    cx.simulate_keystrokes(EXEC);
    assert_eq!(f.lines(cx), ["abab"]);
}

#[gpui::test]
fn instext_is_recorded_and_played(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    settings(cx, &dir.path().join("matsu.ini"), dir.path().to_str());
    let (f, cx) = setup(cx, "xyz");
    cx.simulate_keystrokes(REC);
    f.run(cx, F_INSTEXT_W, &[Arg::Text("あ\r\n".to_owned())]);
    cx.simulate_keystrokes(REC);
    let saved = std::fs::read_to_string(dir.path().join("RecKey.mac")).unwrap();
    assert!(saved.contains("InsText("), "{saved}");
    assert_eq!(f.lines(cx), ["あ", "xyz"]);
    cx.simulate_keystrokes(EXEC);
    assert_eq!(f.lines(cx), ["あ", "あ", "xyz"]);
}

#[gpui::test]
fn a_key_macro_file_with_instext_runs(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let mac = write(
        dir.path(),
        "ins.mac",
        "InsText('1\\n2');\nRight();\nInsText('!');\n",
    );
    let (f, cx) = setup(cx, "xyz");
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&mac)]);
    cx.run_until_parked();
    assert_eq!(f.lines(cx), ["1", "2x!yz"], "{}", f.output(cx));
    assert_eq!(f.output(cx), "");
}
