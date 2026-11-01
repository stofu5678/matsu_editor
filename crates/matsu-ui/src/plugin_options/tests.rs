//! Tests of the plugin option dialog: how each type of `[Option]` is read and shown, what OK and
//! キャンセル write to `plugins\<name>.ini`, and that the host answers `Plugin.GetOption` with the
//! new values.

use std::fs;
use std::path::PathBuf;

use gpui::{Entity, TestAppContext, VisualTestContext};
use matsu_config::plugins::options::PluginOptions;
use matsu_config::plugins::{PluginDef, SearchDirs};
use matsu_plugin::plugin_host::PluginApi;
use matsu_plugin::{Value, json};

use super::dialog::{OptionDialog, current};
use super::model::{BOOL_FALSE, BOOL_TRUE, Form, atoi, with_trailing_separator};

/// A plugin with one option of each type, in the order Bool, Int, Sel, Dir, Str. The last option
/// has no `Section`: it takes the one before it (`ReadPluginDefOption`).
const DEF: &str = "[Plugin]\r\nId=jp.example.options\r\nName=オプション\r\nType=wsh\r\n\
[Option]\r\n\
O[1].Section=Main\r\nO[1].Key=Flag\r\nO[1].Label=旗\r\nO[1].Type=bool\r\nO[1].Default=1\r\n\
O[2].Key=Level\r\nO[2].Label=レベル\r\nO[2].Type=INT\r\nO[2].Default=3\r\n\
O[3].Section=Other\r\nO[3].Key=Mode\r\nO[3].Label=方式\r\nO[3].Type=sel\r\nO[3].Select=自動:0|固定:1|x\r\nO[3].Default=1\r\n\
O[4].Key=Dir\r\nO[4].Label=置き場\r\nO[4].Type=Dir\r\n\
O[5].Key=Name\r\nO[5].Type=whatever\r\nO[5].Default=既定\r\n";

struct Place {
    dir: tempfile::TempDir,
}

impl Place {
    fn new() -> Self {
        let place = Place {
            dir: tempfile::tempdir().unwrap(),
        };
        let plugin = place.dirs().user.join("Opt");
        fs::create_dir_all(&plugin).unwrap();
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(DEF.as_bytes());
        fs::write(plugin.join("plugin.def"), bytes).unwrap();
        place
    }

    fn dirs(&self) -> SearchDirs {
        SearchDirs::new(&self.dir.path().join("matsu.ini"), None)
    }

    fn def(&self) -> PluginDef {
        self.dirs()
            .load_def("Opt", matsu_config::plugins::DEFAULT_LANGUAGE)
            .unwrap()
    }

    fn file(&self) -> PathBuf {
        self.dirs().option_file("Opt")
    }

    fn form(&self) -> Form {
        Form::load(&self.def(), &self.file())
    }

    fn written(&self) -> PluginOptions {
        PluginOptions::load(&self.file())
    }
}

fn shown(form: &Form) -> Vec<String> {
    form.entries.iter().map(|e| e.shown()).collect()
}

// ---- the model ----

#[test]
fn the_values_of_a_plugin_that_has_no_file_are_the_defaults() {
    let place = Place::new();
    let form = place.form();
    assert_eq!(form.title(), "オプション プラグインの設定");
    let labels: Vec<_> = form.entries.iter().map(|e| e.def.label.as_str()).collect();
    // A label that is missing is the key.
    assert_eq!(labels, ["旗", "レベル", "方式", "置き場", "Name"]);
    assert_eq!(
        shown(&form),
        [BOOL_TRUE, "3", "固定", "", "既定"].map(str::to_owned)
    );
    // Opening the dialog writes nothing.
    assert!(!place.file().exists());
}

#[test]
fn the_values_of_the_file_win_and_are_made_to_fit_the_type() {
    let place = Place::new();
    let mut options = PluginOptions::load(&place.file());
    options.set("Main", "Flag", "0").unwrap();
    options.set("Main", "Level", "42abc").unwrap();
    options.set("Other", "Mode", "0").unwrap();
    options.set("Other", "Dir", "C:\\Work\\").unwrap();
    options.set("Other", "Name", "  値  ").unwrap();
    options.save().unwrap();
    // `Dir` and `Name` take the section of the option before them (`Other`).
    let form = place.form();
    assert_eq!(
        shown(&form),
        [BOOL_FALSE, "42", "自動", "C:\\Work\\", "  値  "].map(str::to_owned)
    );
}

#[test]
fn a_choice_without_a_colon_shows_its_value_and_an_unknown_value_shows_nothing() {
    let place = Place::new();
    let mut form = place.form();
    form.set(2, "x");
    assert_eq!(form.entries[2].shown(), "x");
    form.set(2, "nothing like it");
    assert_eq!(form.entries[2].shown(), "");
    // The value stays as it is until the user chooses another one.
    assert_eq!(form.entries[2].value, "nothing like it");
}

#[test]
fn atoi_is_like_the_c_function() {
    assert_eq!(atoi("12"), 12);
    assert_eq!(atoi("  -7x"), -7);
    assert_eq!(atoi("+5"), 5);
    assert_eq!(atoi("abc"), 0);
    assert_eq!(atoi(""), 0);
    assert_eq!(atoi("99999999999"), i32::MAX);
    assert_eq!(atoi("-99999999999"), i32::MIN);
}

#[test]
fn a_folder_gets_the_separator_at_its_end() {
    let sep = std::path::MAIN_SEPARATOR;
    assert_eq!(with_trailing_separator("a"), format!("a{sep}"));
    assert_eq!(with_trailing_separator("a/"), "a/");
    assert_eq!(with_trailing_separator("a\\"), "a\\");
}

#[test]
fn save_writes_every_option_with_the_section_and_key_of_the_definition() {
    let place = Place::new();
    let mut form = place.form();
    form.set(0, "0");
    form.set(1, "-8");
    form.set(2, "0");
    form.set(3, "D:\\x\\");
    form.set(4, "新しい");
    form.save().unwrap();
    let file = place.written();
    assert_eq!(file.get("Main", "Flag"), Some("0"));
    assert_eq!(file.get("Main", "Level"), Some("-8"));
    assert_eq!(file.get("Other", "Mode"), Some("0"));
    assert_eq!(file.get("Other", "Dir"), Some("D:\\x\\"));
    assert_eq!(file.get("Other", "Name"), Some("新しい"));
    // What is written reads back as the same values.
    assert_eq!(
        shown(&place.form()),
        [BOOL_FALSE, "-8", "自動", "D:\\x\\", "新しい"].map(str::to_owned)
    );
}

#[test]
fn save_keeps_the_lines_that_are_not_options() {
    let place = Place::new();
    let mut options = PluginOptions::load(&place.file());
    options.set("Free", "Own", "keep").unwrap();
    options.save().unwrap();
    let mut form = place.form();
    form.set(1, "9");
    form.save().unwrap();
    let file = place.written();
    assert_eq!(file.get("Free", "Own"), Some("keep"));
    assert_eq!(file.get("Main", "Level"), Some("9"));
}

#[test]
fn a_value_the_file_cannot_hold_is_an_error_and_nothing_is_written() {
    let place = Place::new();
    let mut form = place.form();
    form.set(4, "two\nlines");
    assert!(form.save().is_err());
    assert!(!place.file().exists());
}

#[test]
fn the_host_answers_get_option_with_what_the_dialog_wrote() {
    let place = Place::new();
    let dirs = place.dirs();
    let ask = |section: &str, key: &str| -> Value {
        let mut api = crate::macros::plugin_call::DefApi::new(&dirs, "Opt", place.def(), 1);
        api.call("Plugin.GetOption", &[json!(section), json!(key)])
            .unwrap()
    };
    // Before: the defaults of the definition.
    assert_eq!(ask("Main", "Level"), json!("3"));
    let mut form = place.form();
    form.set(1, "77");
    form.set(2, "0");
    form.save().unwrap();
    // The next call reads the new values (a plugin that keeps running needs no restart).
    assert_eq!(ask("Main", "Level"), json!("77"));
    assert_eq!(ask("Other", "Mode"), json!("0"));
    assert_eq!(ask("Main", "Flag"), json!("1"));
}

// ---- the window ----

fn open_dialog<'a>(
    cx: &'a mut TestAppContext,
    place: &Place,
) -> (Entity<OptionDialog>, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    let form = place.form();
    cx.add_window_view(move |window, cx| OptionDialog::new(form, window, cx))
}

#[gpui::test]
fn ok_writes_the_options_and_closes_the_window(cx: &mut TestAppContext) {
    let place = Place::new();
    let (dialog, vcx) = open_dialog(cx, &place);
    dialog.update_in(vcx, |d, window, dcx| {
        // 項目 1 is a bool, 2 an int, 3 a choice, 4 a folder and 5 a string.
        d.select(0, window, dcx);
        d.set_bool(false, dcx);
        d.select(1, window, dcx);
        d.spin(1, window, dcx);
        d.spin(1, window, dcx);
        d.select(2, window, dcx);
        d.choose("0", dcx);
        d.select(3, window, dcx);
        d.set_directory("E:\\Tools", window, dcx);
        d.select(4, window, dcx);
        d.typed("abc", dcx);
        assert_eq!(d.selected(), Some(4));
        d.ok(window, dcx);
    });
    let file = place.written();
    assert_eq!(file.get("Main", "Flag"), Some("0"));
    assert_eq!(file.get("Main", "Level"), Some("5"));
    assert_eq!(file.get("Other", "Mode"), Some("0"));
    assert_eq!(
        file.get("Other", "Dir").map(|s| s.starts_with("E:\\Tools")),
        Some(true)
    );
    assert_eq!(file.get("Other", "Name"), Some("abc"));
}

#[gpui::test]
fn cancel_writes_nothing(cx: &mut TestAppContext) {
    let place = Place::new();
    let (dialog, vcx) = open_dialog(cx, &place);
    dialog.update_in(vcx, |d, window, dcx| {
        d.select(1, window, dcx);
        d.spin(1, window, dcx);
        d.cancel(window);
    });
    assert!(!place.file().exists());
}

#[gpui::test]
fn the_edit_part_follows_the_selected_row(cx: &mut TestAppContext) {
    let place = Place::new();
    let (dialog, vcx) = open_dialog(cx, &place);
    dialog.update_in(vcx, |d, window, dcx| {
        assert_eq!(d.selected(), None, "nothing is selected at first");
        d.select(1, window, dcx);
        assert_eq!(d.input_text(dcx), "3");
        d.select(4, window, dcx);
        assert_eq!(d.input_text(dcx), "既定");
        // Typing into a row that is not a text one does not change it.
        d.select(0, window, dcx);
        d.typed("zzz", dcx);
        assert_eq!(d.form().entries[0].value, "1");
        // The spin stops at the end of the range.
        d.select(1, window, dcx);
        d.typed("2147483647", dcx);
        d.spin(1, window, dcx);
        assert_eq!(d.form().entries[1].value, "2147483647");
    });
}

#[gpui::test]
fn a_plugin_without_options_has_nothing_to_write(cx: &mut TestAppContext) {
    let place = Place::new();
    cx.update(gpui_component::init);
    let mut def = place.def();
    def.options.clear();
    let form = Form::load(&def, &place.file());
    let (dialog, vcx) = cx.add_window_view(move |window, cx| OptionDialog::new(form, window, cx));
    dialog.update_in(vcx, |d, window, dcx| {
        d.ok(window, dcx);
    });
    assert!(!place.file().exists());
}

#[gpui::test]
fn opening_twice_brings_the_first_window_to_the_front(cx: &mut TestAppContext) {
    let place = Place::new();
    cx.update(gpui_component::init);
    let opened = cx.update(|app| super::open(app, &place.dirs(), "Opt"));
    assert!(opened);
    assert!(cx.update(|app| current(app)).is_some());
    let again = cx.update(|app| super::open(app, &place.dirs(), "Opt"));
    assert!(again, "the call succeeds, and no second window opens");
    assert_eq!(cx.update(|app| app.windows().len()), 1);
    // A folder with no `plugin.def` cannot be opened.
    assert!(!cx.update(|app| super::open(app, &place.dirs(), "Gone")));
}
