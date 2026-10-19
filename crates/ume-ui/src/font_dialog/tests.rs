//! Tests of the font functions: UME Editor's own font dialog, `F_FONT`, the font of a document type,
//! the font of the tabs and `F_SETFONTSIZE`, in GPUI's headless test windows with a real settings
//! file in a private temporary folder. (The font dialog of Windows has no desktop to run on in CI;
//! the tests use the dialog of UME Editor, which is what the others use.)

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;
use ume_config::Profile;
use ume_config::history::HistoryStore;
use ume_config::settings::FontSetting;

use super::dialog::{self, FontDialog};
use super::model::{Choice, Style};
use crate::document::SharedDocument;
use crate::file_ops::windows::open_editor_window;
use crate::history::{HistoryOptions, HistoryState};
use crate::settings::UserSettings;
use crate::settings::view_settings::points_to_px;
use crate::view::EditorView;

// ---- the harness ----

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ume-ui-fonts-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The settings file of a test.
struct Ini {
    _dir: TempDir,
    path: PathBuf,
}

impl Ini {
    fn profile(&self) -> Profile {
        Profile::load(&self.path).unwrap()
    }

    fn get(&self, section: &str, key: &str) -> Option<String> {
        self.profile().get(section, key).map(str::to_owned)
    }
}

/// `[Font]` with MS Gothic-like numbers, a face this machine may not have, 10 points.
const FONT: &str = "[Font]\r\nnPointSize=100\r\nlf=-13,0,0,0,400,0,0,0,128,3,2,1,49\r\n\
    lfFaceName=Foo\r\nbFontIs_FIXED_PITCH=0\r\n";

/// A settings file with `text` read as `app::run` reads it, and one editor window.
fn setup(cx: &mut TestAppContext, text: &str) -> (Ini, AnyWindowHandle) {
    cx.update(gpui_component::init);
    let dir = TempDir::new();
    let path = dir.0.join("ume.ini");
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    std::fs::write(&path, bytes).unwrap();
    cx.update(|cx| {
        let user = UserSettings::load_from(std::slice::from_ref(&path));
        cx.set_global(HistoryState::on_disk(
            HistoryStore::new(path.clone(), false),
            HistoryOptions::default(),
        ));
        crate::tabs::settings::install(cx, &user);
        cx.set_global(user);
    });
    let handle = open_window(cx);
    (Ini { _dir: dir, path }, handle)
}

fn open_window(cx: &mut TestAppContext) -> AnyWindowHandle {
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    handle
}

fn view_of(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<EditorView> {
    cx.update(|app| crate::file_ops::windows::workspace_of(app, handle))
        .unwrap()
        .read_with(cx, |ws, _| ws.editor().clone())
}

/// Runs a function of Sakura in the window, as a key does.
fn run(cx: &mut TestAppContext, handle: AnyWindowHandle, function: &str, args: &[Arg]) {
    let code = FUNCTIONS.iter().find(|f| f.name == function).unwrap().code;
    let view = view_of(cx, handle);
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, args, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

/// What a view shows of the font: the family, the weight, the italic and the size in pixels.
fn shown(cx: &mut TestAppContext, view: &Entity<EditorView>) -> (Option<String>, i32, bool, f32) {
    view.read_with(cx, |v, _| {
        let s = v.view_settings();
        (
            s.font_family.clone(),
            s.font_weight,
            s.font_italic,
            v.font_px(),
        )
    })
}

/// Does something to the font dialog that is open, as a click does.
fn with_font_dialog<R>(
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut FontDialog, &mut gpui::Window, &mut gpui::Context<FontDialog>) -> R,
) -> R {
    let (open, handle) = cx
        .update(|app| dialog::current(app))
        .expect("no font dialog is open");
    let result = handle
        .update(cx, |_, window, app| {
            open.update(app, |d, dcx| f(d, window, dcx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

fn font_dialog_is_open(cx: &mut TestAppContext) -> bool {
    cx.update(|app| dialog::current(app)).is_some()
}

/// Chooses a family, a style and a size in the font dialog and presses OK.
fn choose_and_ok(cx: &mut TestAppContext, family: &str, style: Style, tenths: i32) {
    with_font_dialog(cx, |d, window, dcx| {
        d.select_family(family, dcx);
        d.select_style(style, dcx);
        d.select_size(tenths, window, dcx);
    });
    with_font_dialog(cx, |d, window, dcx| d.ok(window, dcx));
    assert!(!font_dialog_is_open(cx), "OK closes the font dialog");
}

// ---- UME Editor's font dialog ----

/// Opens the dialog in a window of its own, with a place for the font it answers.
fn open_dialog(
    cx: &mut TestAppContext,
    base: FontSetting,
    families: Vec<String>,
) -> (
    Entity<FontDialog>,
    &mut VisualTestContext,
    Rc<RefCell<Vec<FontSetting>>>,
) {
    cx.update(gpui_component::init);
    let answers: Rc<RefCell<Vec<FontSetting>>> = Rc::default();
    let sink = answers.clone();
    let (dialog, vcx) = cx.add_window_view(move |window, cx| {
        FontDialog::new(
            base,
            families,
            Box::new(move |_, font| sink.borrow_mut().push(font)),
            window,
            cx,
        )
    });
    (dialog, vcx, answers)
}

fn families(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

#[gpui::test]
fn the_dialog_starts_at_the_font_it_is_given(cx: &mut TestAppContext) {
    let mut base = FontSetting::default();
    base.log_font.face_name = "Beta".to_owned();
    base.log_font.weight = 700;
    base.point_size = 120;
    let (dialog, vcx, _) = open_dialog(cx, base, families(&["Alpha", "Beta"]));
    dialog.read_with(vcx, |d, _| {
        assert_eq!(
            *d.choice(),
            Choice {
                family: "Beta".to_owned(),
                style: Style::Bold,
                size: 120
            }
        );
        assert_eq!(d.families(), ["Alpha", "Beta"]);
    });
}

#[gpui::test]
fn ok_gives_the_font_the_choice_makes_and_closes(cx: &mut TestAppContext) {
    let (dialog, vcx, answers) = open_dialog(cx, FontSetting::default(), families(&["Alpha"]));
    dialog.update_in(vcx, |d, window, dcx| {
        d.select_family("Alpha", dcx);
        d.select_style(Style::BoldItalic, dcx);
        d.select_size(160, window, dcx);
        d.ok(window, dcx);
    });
    let answers = answers.borrow();
    assert_eq!(answers.len(), 1);
    let font = &answers[0];
    assert_eq!(font.point_size, 160);
    // 16 pt at 96 dpi: 21.33 pixels, rounded to 21.
    assert_eq!(font.log_font.height, -21);
    assert_eq!(
        (
            font.log_font.face_name.as_str(),
            font.log_font.weight,
            font.log_font.italic
        ),
        ("Alpha", 700, 1)
    );
    // What the dialog cannot tell stays as it was.
    assert_eq!(
        font.log_font.char_set,
        FontSetting::default().log_font.char_set
    );
}

#[gpui::test]
fn cancel_gives_nothing(cx: &mut TestAppContext) {
    let (dialog, vcx, answers) = open_dialog(cx, FontSetting::default(), families(&["Alpha"]));
    dialog.update_in(vcx, |d, window, dcx| {
        d.select_family("Alpha", dcx);
        d.cancel(window);
    });
    assert!(answers.borrow().is_empty());
}

#[gpui::test]
fn a_typed_size_is_the_size_and_text_that_is_not_a_number_keeps_the_last_one(
    cx: &mut TestAppContext,
) {
    let (dialog, vcx, _) = open_dialog(cx, FontSetting::default(), families(&["Alpha"]));
    dialog.update(vcx, |d, dcx| d.typed_size("12.5", dcx));
    assert_eq!(dialog.read_with(vcx, |d, _| d.choice().size), 125);
    dialog.update(vcx, |d, dcx| d.typed_size("abc", dcx));
    assert_eq!(dialog.read_with(vcx, |d, _| d.choice().size), 125);
    // A size outside the limits is brought into them.
    dialog.update(vcx, |d, dcx| d.typed_size("500", dcx));
    assert_eq!(dialog.read_with(vcx, |d, _| d.choice().size), 720);
    dialog.update(vcx, |d, dcx| d.typed_size("0", dcx));
    assert_eq!(dialog.read_with(vcx, |d, _| d.choice().size), 10);
}

// ---- F_FONT ----

#[gpui::test]
fn f_font_changes_the_common_font_of_every_window(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, FONT);
    let b = open_window(cx);
    let (view_a, view_b) = (view_of(cx, a), view_of(cx, b));
    assert_eq!(shown(cx, &view_a).0.as_deref(), Some("Foo"));

    run(cx, a, "F_FONT", &[]);
    assert!(font_dialog_is_open(cx));
    choose_and_ok(cx, "Bar", Style::Bold, 120);

    // 12 pt is 16 pixels at 96 dpi.
    for view in [&view_a, &view_b] {
        assert_eq!(shown(cx, view), (Some("Bar".to_owned()), 700, false, 16.0));
    }
    // The file has the new font and keeps the rest; the font is now "fixed pitch" (2010.09.06).
    assert_eq!(ini.get("Font", "lfFaceName").as_deref(), Some("Bar"));
    assert_eq!(ini.get("Font", "nPointSize").as_deref(), Some("120"));
    assert_eq!(
        ini.get("Font", "lf").as_deref(),
        Some("-16,0,0,0,700,0,0,0,128,3,2,1,49")
    );
    assert_eq!(ini.get("Font", "bFontIs_FIXED_PITCH").as_deref(), Some("1"));
    // A window made afterwards has it too.
    let c = open_window(cx);
    let view_c = view_of(cx, c);
    assert_eq!(
        shown(cx, &view_c),
        (Some("Bar".to_owned()), 700, false, 16.0)
    );
}

#[gpui::test]
fn f_font_cancelled_changes_nothing(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, FONT);
    let before = std::fs::read(&ini.path).unwrap();
    let view = view_of(cx, a);
    run(cx, a, "F_FONT", &[]);
    with_font_dialog(cx, |d, _, dcx| d.select_family("Bar", dcx));
    with_font_dialog(cx, |d, window, _| d.cancel(window));
    assert_eq!(std::fs::read(&ini.path).unwrap(), before);
    assert_eq!(shown(cx, &view).0.as_deref(), Some("Foo"));
}

#[gpui::test]
fn f_font_starts_at_the_common_font(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, FONT);
    run(cx, a, "F_FONT", &[]);
    let choice = with_font_dialog(cx, |d, _, _| d.choice().clone());
    assert_eq!(
        choice,
        Choice {
            family: "Foo".to_owned(),
            style: Style::Regular,
            size: 100
        }
    );
}

#[gpui::test]
fn a_window_whose_type_has_its_own_font_keeps_it(cx: &mut TestAppContext) {
    // Type 0 (the basic type, which an untitled document has) has a font of its own.
    let text = format!(
        "{FONT}[Types(0)]\r\nbUseTypeFont=1\r\nnPointSize=140\r\n\
         lf=-19,0,0,0,400,0,0,0,128,3,2,1,49\r\nlfFaceName=TypeFace\r\n"
    );
    let (_ini, a) = setup(cx, &text);
    let view = view_of(cx, a);
    assert_eq!(shown(cx, &view).0.as_deref(), Some("TypeFace"));
    run(cx, a, "F_FONT", &[]);
    choose_and_ok(cx, "Bar", Style::Regular, 120);
    assert_eq!(
        shown(cx, &view),
        (Some("TypeFace".to_owned()), 400, false, points_to_px(140))
    );
}

#[gpui::test]
fn a_size_set_for_the_window_stays_for_a_new_face_and_ends_for_a_new_size(cx: &mut TestAppContext) {
    // `CEditDoc::OnChangeSetting`: the temporary size follows a change of the face, and ends when
    // the size of the font of the file changes.
    let (_ini, a) = setup(cx, FONT);
    let view = view_of(cx, a);
    run(cx, a, "F_SETFONTSIZEUP", &[]);
    let zoomed = points_to_px(110);
    assert_eq!(shown(cx, &view).3, zoomed);

    run(cx, a, "F_FONT", &[]);
    choose_and_ok(cx, "Bar", Style::Regular, 100);
    assert_eq!(shown(cx, &view).0.as_deref(), Some("Bar"));
    assert_eq!(shown(cx, &view).3, zoomed, "the same size: the zoom stays");

    run(cx, a, "F_FONT", &[]);
    choose_and_ok(cx, "Bar", Style::Regular, 140);
    assert_eq!(
        shown(cx, &view).3,
        points_to_px(140),
        "a new size: the zoom ends"
    );
}

// ---- the font of a type ----

#[gpui::test]
fn the_font_of_a_type_is_chosen_in_the_type_settings_and_used_at_once(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, FONT);
    let view = view_of(cx, a);
    run(cx, a, "F_OPTION_TYPE", &[]);
    let sheet = cx
        .update(|app| crate::type_settings::current_dialog(app))
        .expect("the type settings are open");
    let sheet_window = cx
        .update(|app| crate::type_settings::dialog_window(app))
        .unwrap();
    let press = |cx: &mut TestAppContext,
                 f: &dyn Fn(
        &mut crate::common_settings::CommonSettingsDialog,
        &mut gpui::Window,
        &mut gpui::Context<crate::common_settings::CommonSettingsDialog>,
    )| {
        sheet_window
            .update(cx, |_, window, app| {
                sheet.update(app, |d, dcx| f(d, window, dcx))
            })
            .unwrap();
        cx.run_until_parked();
    };

    // The font is not used: the check box is cleared (and cannot be checked), nothing is shown.
    let value = |cx: &mut TestAppContext, id: &'static str| {
        sheet.read_with(cx, |d, _| d.values().get(id).cloned().unwrap())
    };
    assert_eq!(
        value(cx, "use_type_font"),
        crate::common_settings::model::Value::Bool(false)
    );

    // The button: the dialog starts at the common font, and the font chosen is used.
    press(cx, &|d, window, dcx| {
        let initial = crate::font_dialog::apply::common_font(dcx).font;
        d.choose_font("type_font", initial, window, dcx);
    });
    let started = with_font_dialog(cx, |d, _, _| d.choice().clone());
    assert_eq!(started.family, "Foo");
    choose_and_ok(cx, "Baz", Style::Italic, 140);
    assert_eq!(
        value(cx, "use_type_font"),
        crate::common_settings::model::Value::Bool(true)
    );
    // Nothing is written until OK.
    assert_eq!(ini.get("Types(0)", "bUseTypeFont"), None);

    press(cx, &|d, window, dcx| d.ok(window, dcx));
    assert_eq!(ini.get("Types(0)", "bUseTypeFont").as_deref(), Some("1"));
    assert_eq!(ini.get("Types(0)", "lfFaceName").as_deref(), Some("Baz"));
    assert_eq!(ini.get("Types(0)", "nPointSize").as_deref(), Some("140"));
    assert_eq!(
        ini.get("Types(0)", "lf").as_deref(),
        Some("-19,0,0,0,400,1,0,0,128,3,2,1,49")
    );
    // The common font is as it was; the window of the type shows the type's font.
    assert_eq!(ini.get("Font", "lfFaceName").as_deref(), Some("Foo"));
    assert_eq!(
        shown(cx, &view),
        (Some("Baz".to_owned()), 400, true, points_to_px(140))
    );
}

#[gpui::test]
fn clearing_the_check_box_of_the_type_font_gives_the_common_font_back(cx: &mut TestAppContext) {
    let text = format!(
        "{FONT}[Types(0)]\r\nbUseTypeFont=1\r\nnPointSize=140\r\n\
         lf=-19,0,0,0,400,0,0,0,128,3,2,1,49\r\nlfFaceName=TypeFace\r\n"
    );
    let (ini, a) = setup(cx, &text);
    let view = view_of(cx, a);
    run(cx, a, "F_OPTION_TYPE", &[]);
    let sheet = cx
        .update(|app| crate::type_settings::current_dialog(app))
        .unwrap();
    let sheet_window = cx
        .update(|app| crate::type_settings::dialog_window(app))
        .unwrap();
    sheet_window
        .update(cx, |_, window, app| {
            sheet.update(app, |d, dcx| {
                d.set_bool("use_type_font", false, dcx);
                d.ok(window, dcx);
            })
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(ini.get("Types(0)", "bUseTypeFont").as_deref(), Some("0"));
    // The type keeps its font in the file, as Sakura does (`m_lf` is not cleared).
    assert_eq!(
        ini.get("Types(0)", "lfFaceName").as_deref(),
        Some("TypeFace")
    );
    assert_eq!(shown(cx, &view).0.as_deref(), Some("Foo"));
}

// ---- the font of the tabs ----

#[gpui::test]
fn the_font_of_the_tabs_is_chosen_in_the_common_settings_and_written(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    assert_eq!(
        cx.update(|app| crate::tabs::settings::tab_font(app)).0,
        None
    );
    cx.update(|app| {
        crate::common_settings::open(app, Some(crate::common_settings::Page::Tab), Some(a));
    });
    cx.run_until_parked();
    let sheet = cx
        .update(|app| crate::common_settings::current_dialog(app))
        .unwrap();
    let sheet_window = cx
        .update(|app| crate::common_settings::dialog_window(app))
        .unwrap();
    sheet_window
        .update(cx, |_, window, app| {
            sheet.update(app, |d, dcx| {
                d.choose_font("tab_font_value", FontSetting::default(), window, dcx);
            })
        })
        .unwrap();
    cx.run_until_parked();
    choose_and_ok(cx, "TabFace", Style::Bold, 90);
    // Nothing is written until OK.
    assert_eq!(ini.get("Common", "lfTabFaceName"), None);
    sheet_window
        .update(cx, |_, window, app| {
            sheet.update(app, |d, dcx| d.ok(window, dcx))
        })
        .unwrap();
    cx.run_until_parked();

    assert_eq!(
        ini.get("Common", "lfTabFaceName").as_deref(),
        Some("TabFace")
    );
    assert_eq!(ini.get("Common", "lfTabFontPs").as_deref(), Some("90"));
    assert_eq!(
        ini.get("Common", "lfTabFont").as_deref(),
        Some("-12,0,0,0,700,0,0,0,128,3,2,1,49")
    );
    // The tab bar draws with it: 9 pt is 12 pixels.
    let font = cx.update(|app| crate::tabs::settings::tab_font(app));
    assert_eq!(font.family().as_deref(), Some("TabFace"));
    assert_eq!(font.size_px(), Some(12.0));
    assert_eq!(font.weight(), Some(700.0));
}

#[test]
fn the_tab_font_is_read_from_the_common_section() {
    use crate::tabs::settings::TabFont;
    assert_eq!(TabFont::from_profile(&Profile::new()), TabFont(None));
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(
        b"[Common]\r\nlfTabFont=-15,0,0,0,400,1,0,0,0,3,2,1,34\r\nlfTabFontPs=110\r\nlfTabFaceName=Sans\r\n",
    );
    let font = TabFont::from_profile(&Profile::from_bytes(&bytes));
    assert_eq!(font.family().as_deref(), Some("Sans"));
    // 11 pt at 96 dpi
    assert_eq!(font.size_px(), Some(15.0));
    assert_eq!(font.weight(), Some(400.0));
    assert!(font.italic());
    // A face name that the file lacks: no font of the tabs.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"[Common]\r\nlfTabFont=-15,0,0,0,400,1,0,0,0,3,2,1,34\r\n");
    assert_eq!(
        TabFont::from_profile(&Profile::from_bytes(&bytes)),
        TabFont(None)
    );
}

// ---- F_SETFONTSIZE ----

fn set_font_size(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    size: i64,
    shift: i64,
    mode: i64,
) {
    run(
        cx,
        handle,
        "F_SETFONTSIZE",
        &[Arg::Int(size), Arg::Int(shift), Arg::Int(mode)],
    );
}

#[gpui::test]
fn set_font_size_mode_2_changes_this_window_only(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, FONT);
    let b = open_window(cx);
    let (view_a, view_b) = (view_of(cx, a), view_of(cx, b));
    let before = std::fs::read(&ini.path).unwrap();
    let base = points_to_px(100);

    // A size to set: 20 pt.
    set_font_size(cx, a, 200, 0, 2);
    assert_eq!(shown(cx, &view_a).3, points_to_px(200));
    assert_eq!(shown(cx, &view_b).3, base);
    // A size outside the limits is brought into them: 1 to 72 points.
    set_font_size(cx, a, 5000, 0, 2);
    assert_eq!(shown(cx, &view_a).3, points_to_px(720));
    set_font_size(cx, a, 1, 0, 2);
    assert_eq!(shown(cx, &view_a).3, points_to_px(10));
    // Steps of the zoom table from the size of the file (10 pt): +1 is 11 pt, +2 more is 12.5 pt.
    set_font_size(cx, a, 0, 1, 2);
    assert_eq!(shown(cx, &view_a).3, points_to_px(110));
    set_font_size(cx, a, 0, 1, 2);
    assert_eq!(shown(cx, &view_a).3, points_to_px(125));
    set_font_size(cx, a, 0, -2, 2);
    assert_eq!(shown(cx, &view_a).3, base);
    // The file is not written: the size is a temporary one.
    assert_eq!(std::fs::read(&ini.path).unwrap(), before);
}

#[gpui::test]
fn set_font_size_with_nothing_to_do_does_nothing(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, FONT);
    let view = view_of(cx, a);
    let before = std::fs::read(&ini.path).unwrap();
    set_font_size(cx, a, 0, 0, 2);
    set_font_size(cx, a, 200, 0, 3);
    set_font_size(cx, a, 200, 0, -1);
    set_font_size(cx, a, 200, 0, 7);
    assert_eq!(shown(cx, &view).3, points_to_px(100));
    assert_eq!(std::fs::read(&ini.path).unwrap(), before);
}

#[gpui::test]
fn set_font_size_changes_only_truetype_fonts(cx: &mut TestAppContext) {
    // `lfOutPrecision` 1 is `OUT_STRING_PRECIS`, a raster font.
    let raster = "[Font]\r\nnPointSize=100\r\nlf=-13,0,0,0,400,0,0,0,128,1,2,1,49\r\n\
        lfFaceName=Foo\r\n";
    let (ini, a) = setup(cx, raster);
    let view = view_of(cx, a);
    let before = std::fs::read(&ini.path).unwrap();
    for mode in 0..=2 {
        set_font_size(cx, a, 200, 0, mode);
    }
    run(cx, a, "F_SETFONTSIZEUP", &[]);
    assert_eq!(shown(cx, &view).3, points_to_px(100));
    assert_eq!(std::fs::read(&ini.path).unwrap(), before);
}

#[gpui::test]
fn set_font_size_mode_0_changes_the_common_font_of_every_window(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, FONT);
    let b = open_window(cx);
    let (view_a, view_b) = (view_of(cx, a), view_of(cx, b));
    set_font_size(cx, a, 0, 1, 0);
    // The next size of the zoom table, in both windows; the face stays.
    for view in [&view_a, &view_b] {
        assert_eq!(
            shown(cx, view),
            (Some("Foo".to_owned()), 400, false, points_to_px(110))
        );
    }
    assert_eq!(ini.get("Font", "nPointSize").as_deref(), Some("110"));
    assert_eq!(
        ini.get("Font", "lf").as_deref(),
        Some("-15,0,0,0,400,0,0,0,128,3,2,1,49")
    );
    assert_eq!(ini.get("Font", "lfFaceName").as_deref(), Some("Foo"));
    // A size to set.
    set_font_size(cx, b, 160, 0, 0);
    assert_eq!(shown(cx, &view_a).3, points_to_px(160));
    assert_eq!(ini.get("Font", "nPointSize").as_deref(), Some("160"));
}

#[gpui::test]
fn set_font_size_mode_1_gives_the_type_a_font_of_its_own(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, FONT);
    let view = view_of(cx, a);
    set_font_size(cx, a, 0, 1, 1);
    assert_eq!(
        shown(cx, &view),
        (Some("Foo".to_owned()), 400, false, points_to_px(110))
    );
    // The type uses its own font now, with the face of the font the window showed; the common font
    // is as it was.
    assert_eq!(ini.get("Types(0)", "bUseTypeFont").as_deref(), Some("1"));
    assert_eq!(ini.get("Types(0)", "nPointSize").as_deref(), Some("110"));
    assert_eq!(ini.get("Types(0)", "lfFaceName").as_deref(), Some("Foo"));
    assert_eq!(ini.get("Font", "nPointSize").as_deref(), Some("100"));
    // Another change of the common font does not reach the window of the type.
    set_font_size(cx, a, 300, 0, 0);
    assert_eq!(shown(cx, &view).3, points_to_px(110));
}

#[gpui::test]
fn f_setfontsizeup_and_down_are_set_font_size_in_mode_2(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, FONT);
    let view = view_of(cx, a);
    run(cx, a, "F_SETFONTSIZEUP", &[]);
    assert_eq!(shown(cx, &view).3, points_to_px(110));
    run(cx, a, "F_SETFONTSIZEDOWN", &[]);
    assert_eq!(shown(cx, &view).3, points_to_px(100));
    set_font_size(cx, a, 0, 1, 2);
    assert_eq!(shown(cx, &view).3, points_to_px(110));
}

#[gpui::test]
fn a_huge_shift_from_a_script_does_not_panic(cx: &mut TestAppContext) {
    let (_ini, a) = setup(cx, FONT);
    let view = view_of(cx, a);
    set_font_size(cx, a, 0, i64::from(i32::MAX), 2);
    assert_eq!(shown(cx, &view).3, points_to_px(720));
    set_font_size(cx, a, 0, i64::from(i32::MIN), 2);
    assert_eq!(shown(cx, &view).3, points_to_px(10));
}

#[gpui::test]
fn changing_the_size_of_the_settings_keeps_the_temporary_wrap(cx: &mut TestAppContext) {
    // `PM_CHANGESETTING_FONTSIZE` reads the font again (`OnChangeSetting( true, false )`); a wrap
    // method set for the window only (`F_TMPWRAPWINDOW`) is not part of it.
    for mode in [0, 1] {
        let (_ini, a) = setup(cx, FONT);
        let view = view_of(cx, a);
        run(cx, a, "F_TMPWRAPWINDOW", &[]);
        let wrap = |cx: &mut TestAppContext| view.read_with(cx, |v, _| v.prefs.wrap_method);
        assert_eq!(wrap(cx), crate::settings::WrapMethod::WindowWidth);
        set_font_size(cx, a, 0, 1, mode);
        assert_eq!(shown(cx, &view).3, points_to_px(110), "mode {mode}");
        assert_eq!(
            wrap(cx),
            crate::settings::WrapMethod::WindowWidth,
            "mode {mode}"
        );
    }
}

#[gpui::test]
fn a_type_font_made_from_no_font_in_the_file_keeps_the_programs_face(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "");
    set_font_size(cx, a, 200, 0, 1);
    assert_eq!(
        ini.get("Types(0)", "lfFaceName").as_deref(),
        Some(crate::view::FONT_FAMILY)
    );
}
