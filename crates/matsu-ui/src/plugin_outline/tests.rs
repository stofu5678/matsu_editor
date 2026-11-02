//! Tests of the outlines of the plugins: what the `Outline` object collects, which plugins offer an
//! outline, and the list that the type settings show.

use std::fs;
use std::path::PathBuf;

use gpui::TestAppContext;
use matsu_commands::plugin_commands::plug_code;
use matsu_config::Profile;
use matsu_config::plugins::consent::{ConsentFile, fingerprint};
use matsu_config::plugins::{DEFAULT_LANGUAGE, PluginRec, PluginSettings, PluginState, SearchDirs};
use matsu_outline::OutlineKind;
use matsu_plugin::plugin_host::Outcome;
use matsu_plugin::{Value, json};

use super::*;

pub(crate) const ID: &str = "test.outline";
pub(crate) const FOLDER: &str = "Outl";

/// A folder with a settings file and a plugin `Outl` whose jack `Outline` has the label
/// "Test outline" and runs `script`.
pub(crate) struct Place {
    pub(crate) dir: PathBuf,
}

impl Place {
    pub(crate) fn new(name: &str, script: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "matsu-plugin-outline-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let plugin = dir.join("plugins").join(FOLDER);
        fs::create_dir_all(&plugin).unwrap();
        fs::write(
            plugin.join("plugin.def"),
            format!(
                "[Plugin]\nId={ID}\nName=Outl\nType=wsh\n\n[Plug]\nOutline=outline.js\nOutline.Label=Test outline\n\n[Wsh]\nUseCache=1\n"
            ),
        )
        .unwrap();
        fs::write(plugin.join("outline.js"), script).unwrap();
        Self { dir }
    }

    pub(crate) fn settings(&self) -> PathBuf {
        self.dir.join("matsu.ini")
    }

    /// The code of the one `Outline` plug of the plugin in slot 0 (`Outline` is plug 0).
    pub(crate) fn code() -> i32 {
        i32::from(plug_code(0, 0).unwrap())
    }

    /// The settings of a user: the plugin is registered in slot 0, the switch is `enabled`, the user
    /// agreed to the files as they are when `agree`, and the basic type has the outline `outline`.
    pub(crate) fn user(&self, outline: i32, enabled: bool, agree: bool) -> UserSettings {
        let dirs = SearchDirs::new(&self.settings(), None);
        let def = dirs.load_def(FOLDER, DEFAULT_LANGUAGE).unwrap();
        let mut settings = PluginSettings {
            enabled,
            ..PluginSettings::default()
        };
        settings.table[0] = PluginRec {
            id: def.id.clone(),
            name: FOLDER.into(),
            cmd_num: 0,
            state: PluginState::Stopped,
        };
        let ini =
            format!("[Types(0)]\r\nnDefaultOutline={outline}\r\n[Other]\r\nnTypesCount=17\r\n");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(ini.as_bytes());
        let mut profile = Profile::from_bytes(&bytes);
        settings.store(&mut profile).unwrap();
        profile.save(&self.settings()).unwrap();
        if agree {
            let files = fingerprint(&dirs.plugin_dir(FOLDER).unwrap()).unwrap();
            let mut consent = ConsentFile::default();
            consent
                .enable(&def.id, FOLDER, &files, "2026-11-02")
                .unwrap();
            consent.save(&dirs.consent_file()).unwrap();
        }
        UserSettings::from_profile(profile, Some(self.settings()))
    }
}

impl Drop for Place {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn call(collected: &mut Collected, name: &str, params: &[Value]) -> Result<Value, RpcError> {
    collected.call(name, params)
}

fn done(mut collected: Collected) -> matsu_outline::Analysis {
    collected.ending = Some(Ending::Done(Outcome::Ok));
    collected.to_analysis(std::time::Duration::from_secs(10))
}

#[test]
fn the_four_add_calls_take_sakuras_arguments() {
    let mut c = Collected::new();
    // AddFuncInfo(line, column, text, information)
    call(
        &mut c,
        "Outline.AddFuncInfo",
        &[json!(3), json!(5), json!("a"), json!(2)],
    )
    .unwrap();
    // AddFuncInfo2: the low 16 bits of the last argument are the depth (`FUNCINFO_INFOMASK`)
    call(
        &mut c,
        "Outline.AddFuncInfo2",
        &[json!(4), json!(1), json!("b"), json!(0x10000 + 3)],
    )
    .unwrap();
    // AddFuncInfo3 and 4 take the file name last
    call(
        &mut c,
        "Outline.AddFuncInfo3",
        &[json!(5), json!(1), json!("c"), json!(7), json!("x.txt")],
    )
    .unwrap();
    call(
        &mut c,
        "Outline.AddFuncInfo4",
        &[json!(6), json!(1), json!("d"), json!(1), json!("y.txt")],
    )
    .unwrap();
    let analysis = done(c);
    let items = analysis.items.items();
    assert_eq!(items.len(), 4);
    assert_eq!(
        (items[0].line, items[0].col, items[0].info, items[0].depth),
        (3, 5, 2, 0)
    );
    assert_eq!((items[1].info, items[1].depth), (0x10000, 3));
    assert_eq!((items[2].file.as_str(), items[2].info), ("x.txt", 7));
    assert_eq!((items[3].file.as_str(), items[3].depth), ("y.txt", 1));
    assert_eq!(items[3].info, 0);
    assert!(items[0].file.is_empty());
}

#[test]
fn numbers_may_come_as_text_and_a_missing_argument_is_an_error() {
    let mut c = Collected::new();
    // WSH hands over text: `_wtoi` reads the digits at its start
    call(
        &mut c,
        "Outline.AddFuncInfo",
        &[json!("7"), json!(" 2x"), json!(12), json!("")],
    )
    .unwrap();
    let analysis = done(c);
    let item = &analysis.items.items()[0];
    assert_eq!(
        (item.line, item.col, item.name.as_str(), item.info),
        (7, 2, "12", 0)
    );

    let mut c = Collected::new();
    let error = call(
        &mut c,
        "Outline.AddFuncInfo",
        &[json!(1), json!(1), json!("a")],
    )
    .unwrap_err();
    assert!(error.message.contains("argument 4"), "{}", error.message);
    // 3 and 4 need the file name
    assert!(
        call(
            &mut c,
            "Outline.AddFuncInfo3",
            &[json!(1), json!(1), json!("a"), json!(0)]
        )
        .is_err()
    );
    assert!(done(c).items.is_empty());
}

#[test]
fn title_list_type_and_labels_go_to_the_analysis() {
    let mut c = Collected::new();
    call(&mut c, "Outline.SetTitle", &[json!("My outline")]).unwrap();
    call(&mut c, "Outline.SetListType", &[json!(200)]).unwrap();
    call(
        &mut c,
        "Outline.SetLabel",
        &[json!(10), json!(" モジュール")],
    )
    .unwrap();
    let analysis = done(c);
    assert_eq!(analysis.title.as_deref(), Some("My outline"));
    assert_eq!(analysis.kind, OutlineKind::ClassTree);
    assert_eq!(analysis.items.append_text(10), " モジュール");
    assert_eq!(analysis.error, None);
}

#[test]
fn the_list_type_chooses_how_the_result_is_shown() {
    let kind = |list_type: i64| {
        let mut c = Collected::new();
        call(&mut c, "Outline.SetListType", &[json!(list_type)]).unwrap();
        done(c).kind
    };
    assert_eq!(kind(100), OutlineKind::Tree);
    assert_eq!(kind(101), OutlineKind::TreeTagJump);
    assert_eq!(kind(200), OutlineKind::ClassTree);
    assert_eq!(kind(300), OutlineKind::List);
    // the numbers of the kinds Sakura has still work (the manual advises against them): 8 is the list of
    // Visual Basic, 3 the topic tree
    assert_eq!(kind(8), OutlineKind::VisualBasic);
    assert_eq!(kind(3), OutlineKind::Text);
    // without a choice, or with a number nobody knows, it is a list (`OUTLINE_PLUGIN`)
    assert_eq!(done(Collected::new()).kind, OutlineKind::List);
    assert_eq!(kind(77), OutlineKind::List);
}

#[test]
fn a_failed_call_gives_an_empty_list_and_the_reason() {
    let mut c = Collected::new();
    c.label = "Plug".into();
    call(
        &mut c,
        "Outline.AddFuncInfo",
        &[json!(1), json!(1), json!("kept?"), json!(0)],
    )
    .unwrap();
    c.ending = Some(Ending::Done(Outcome::TimedOut));
    let limit = std::time::Duration::from_secs(10);
    let analysis = c.to_analysis(limit);
    assert!(analysis.items.is_empty());
    assert_eq!(analysis.title.as_deref(), Some("Plug"));
    assert_eq!(
        analysis.error.as_deref(),
        Some("プラグイン Plug: 時間切れで中止しました (10 秒)")
    );
    c.ending = Some(Ending::Done(Outcome::Failed(
        "Error: boom\n    at x".into(),
    )));
    let text = c.to_analysis(limit).error.unwrap();
    assert!(
        text.starts_with("プラグイン Plug: エラーで終了しました: Error: boom"),
        "{text}"
    );
    assert!(!text.contains("at x"), "only the first line: {text}");
    c.ending = Some(Ending::Refused("失敗が続いたので止めています".into()));
    assert_eq!(
        c.to_analysis(limit).error.as_deref(),
        Some("プラグイン Plug: 失敗が続いたので止めています")
    );
}

#[test]
fn a_call_in_progress_is_known_until_its_result_is_dropped() {
    let (a, b) = (gpui::EntityId::from(1_u64), gpui::EntityId::from(2_u64));
    assert!(!is_running_in(a));
    let c = Collected::new().in_view(a);
    assert!(is_running_in(a));
    // another window is not held back
    assert!(!is_running_in(b));
    drop(c);
    assert!(!is_running_in(a));
}

#[test]
fn the_list_of_the_sheet_is_made_once_for_the_same_extra_choices() {
    static BASE: &[(&str, i32)] = &[("a", 1), ("b", 2)];
    assert!(std::ptr::eq(with_extra(BASE, &[]), BASE));
    let extra = vec![("p".to_owned(), 20000)];
    let list = with_extra(BASE, &extra);
    assert_eq!(list, [("a", 1), ("b", 2), ("p", 20000)]);
    assert!(std::ptr::eq(list, with_extra(BASE, &extra)));
    let other = with_extra(BASE, &[("q".to_owned(), 20001)]);
    assert_eq!(other.last(), Some(&("q", 20001)));
}

#[test]
fn atoi_reads_the_digits_at_the_start() {
    assert_eq!(atoi("12"), 12);
    assert_eq!(atoi("  -3abc"), -3);
    assert_eq!(atoi("+4"), 4);
    assert_eq!(atoi("abc"), 0);
    assert_eq!(atoi(""), 0);
}

#[test]
fn a_plugin_that_may_run_offers_its_outline_and_one_that_may_not_does_not() {
    let place = Place::new("choices", "");
    let code = Place::code();
    refresh_for(&place.user(0, true, true));
    assert_eq!(choices(), [("Test outline".to_owned(), code)]);

    // plugins are off
    refresh_for(&place.user(0, false, true));
    assert!(choices().is_empty());
    // the user did not agree to the files
    let _ = fs::remove_file(SearchDirs::new(&place.settings(), None).consent_file());
    refresh_for(&place.user(0, true, false));
    assert!(choices().is_empty());
}

#[gpui::test]
fn the_number_of_a_plug_finds_the_plug_that_may_run(cx: &mut TestAppContext) {
    let place = Place::new("find", "");
    let code = Place::code();
    cx.update(|app| app.set_global(place.user(code, true, true)));
    let found = cx.update(|app| find(app, code)).unwrap();
    assert_eq!(
        (found.slot, found.folder.as_str(), found.handler.as_str()),
        (0, FOLDER, "outline.js")
    );
    assert_eq!(found.label, "Test outline");
    // a number of Sakura's own outlines, or of no plug, is none
    assert!(cx.update(|app| find(app, 1)).is_none());
    assert!(cx.update(|app| find(app, code + 1)).is_none());
    assert!(cx.update(|app| find(app, -5)).is_none());
    // not when plugins are off
    cx.update(|app| app.set_global(place.user(code, false, true)));
    assert!(cx.update(|app| find(app, code)).is_none());
}
