//! Tests of the file name display without a window: the replacements, the special folders, the
//! short form of a long path and the title formats.

use std::collections::HashMap;

use ume_config::dialog_dir::MetaSource;

use super::short_path::short_path;
use super::title::{CAPTION_ACTIVE, CAPTION_INACTIVE, Subject, expand};
use super::{FileNames, NO_TITLE};

/// The meta folders of a test machine (a Windows user `u`).
struct Machine {
    shell: HashMap<&'static str, &'static str>,
    env: HashMap<&'static str, &'static str>,
}

impl Machine {
    fn windows() -> Self {
        Self {
            shell: HashMap::from([
                ("Desktop", "C:\\Users\\u\\Desktop"),
                ("Personal", "C:\\Users\\u\\Documents"),
                (
                    "Cache",
                    "C:\\Users\\u\\AppData\\Local\\Microsoft\\Windows\\INetCache",
                ),
                ("Common Desktop", "C:\\Users\\Public\\Desktop"),
                ("Common Documents", "C:\\Users\\Public\\Documents"),
                ("AppData", "C:\\Users\\u\\AppData\\Roaming"),
            ]),
            env: HashMap::from([("TEMP", "C:\\Users\\u\\AppData\\Local\\Temp")]),
        }
    }

    fn mac() -> Self {
        Self {
            shell: HashMap::from([
                ("Desktop", "/Users/u/Desktop"),
                ("Personal", "/Users/u/Documents"),
            ]),
            env: HashMap::from([("TEMP", "/var/tmp")]),
        }
    }
}

impl MetaSource for Machine {
    fn program_dir(&self) -> Option<String> {
        Some("C:\\Apps\\ume".to_owned())
    }

    fn settings_dir(&self) -> Option<String> {
        Some("C:\\Users\\u\\AppData\\Roaming\\ume".to_owned())
    }

    fn shell_folder(&self, name: &str) -> Option<String> {
        self.shell
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, path)| (*path).to_owned())
    }

    fn env_var(&self, name: &str) -> Option<String> {
        self.env
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| (*value).to_owned())
    }
}

fn row(from: &str, to: &str) -> Vec<String> {
    vec![from.to_owned(), to.to_owned()]
}

/// Sakura's list (`CShareData::InitShareData`).
fn sakura_rows() -> Vec<Vec<String>> {
    vec![
        row("%DeskTop%\\", "デスクトップ\\"),
        row("%Personal%\\", "マイドキュメント\\"),
        row("%Cache%\\Content.IE5\\", "IEキャッシュ\\"),
        row("%TEMP%\\", "TEMP\\"),
        row("%Common DeskTop%\\", "共有デスクトップ\\"),
        row("%Common Documents%\\", "共有ドキュメント\\"),
        row("%AppData%\\", "アプリデータ\\"),
    ]
}

fn windows_names(short: bool, width: usize, rows: &[Vec<String>]) -> FileNames {
    FileNames::new(short, width, rows, &Machine::windows(), false)
}

#[test]
fn each_special_folder_of_sakuras_list_is_replaced() {
    let names = windows_names(false, 100, &sakura_rows());
    let cases = [
        ("C:\\Users\\u\\Desktop\\a.txt", "デスクトップ\\a.txt"),
        (
            "C:\\Users\\u\\Documents\\x\\b.txt",
            "マイドキュメント\\x\\b.txt",
        ),
        (
            "C:\\Users\\u\\AppData\\Local\\Microsoft\\Windows\\INetCache\\Content.IE5\\c.htm",
            "IEキャッシュ\\c.htm",
        ),
        ("C:\\Users\\u\\AppData\\Local\\Temp\\t.txt", "TEMP\\t.txt"),
        (
            "C:\\Users\\Public\\Desktop\\d.txt",
            "共有デスクトップ\\d.txt",
        ),
        (
            "C:\\Users\\Public\\Documents\\e.txt",
            "共有ドキュメント\\e.txt",
        ),
        (
            "C:\\Users\\u\\AppData\\Roaming\\f.ini",
            "アプリデータ\\f.ini",
        ),
    ];
    for (path, shown) in cases {
        assert_eq!(names.transform(path, true), shown, "{path}");
    }
    // A path elsewhere is shown as it is.
    assert_eq!(names.transform("D:\\work\\a.txt", true), "D:\\work\\a.txt");
}

#[test]
fn the_replacement_ignores_case_and_replaces_every_match() {
    let names = windows_names(false, 100, &[row("%DeskTop%\\", "D\\")]);
    assert_eq!(
        names.transform("c:\\USERS\\u\\desktop\\a.txt", true),
        "D\\a.txt"
    );
    // Not only the start: Sakura replaces each place that matches.
    let twice = windows_names(false, 100, &[row("ab", "X")]);
    assert_eq!(twice.transform("1AB2ab3", true), "1X2X3");
}

#[test]
fn a_users_replacement_works_and_the_order_matters() {
    // The result of a replacement is the input of the next one.
    let rows = [
        row("C:\\work\\", "W\\"),
        row("W\\proj\\", "P:"),
        row("P:src", "S"),
    ];
    let names = windows_names(false, 100, &rows);
    assert_eq!(
        names.transform("C:\\work\\proj\\src\\a.rs", true),
        "S\\a.rs"
    );
    // In the other order the later rows have nothing to match yet.
    let reversed = [rows[2].clone(), rows[1].clone(), rows[0].clone()];
    let names = windows_names(false, 100, &reversed);
    assert_eq!(
        names.transform("C:\\work\\proj\\src\\a.rs", true),
        "W\\proj\\src\\a.rs"
    );
}

#[test]
fn rows_that_cannot_be_used_are_left_out() {
    let rows = [
        row("", "empty"),
        row("%NoSuchFolder%\\", "never\\"),
        row("D:\\data\\", "データ\\"),
    ];
    let names = windows_names(false, 100, &rows);
    assert_eq!(names.transform("D:\\data\\a.txt", true), "データ\\a.txt");
    // A meta folder nobody knows stays as typed, and so matches only that text.
    assert_eq!(
        names.transform("C:\\%NoSuchFolder%\\a.txt", true),
        "C:\\never\\a.txt"
    );
    // `%%` is a `%`.
    let percent = windows_names(false, 100, &[row("C:\\100%%\\", "百\\")]);
    assert_eq!(percent.transform("C:\\100%\\a.txt", true), "百\\a.txt");
}

#[test]
fn the_folders_of_the_program_and_of_the_settings_are_meta_folders() {
    let rows = [row("%SAKURA%\\", "本体\\"), row("%SAKURADATA%\\", "設定\\")];
    let names = windows_names(false, 100, &rows);
    assert_eq!(
        names.transform("C:\\Apps\\ume\\macro.mac", true),
        "本体\\macro.mac"
    );
    assert_eq!(
        names.transform("C:\\Users\\u\\AppData\\Roaming\\ume\\ume.ini", true),
        "設定\\ume.ini"
    );
}

#[test]
fn off_windows_the_list_uses_slashes() {
    let names = FileNames::new(false, 100, &sakura_rows(), &Machine::mac(), true);
    assert_eq!(
        names.transform("/Users/u/Desktop/メモ.txt", true),
        "デスクトップ/メモ.txt"
    );
    assert_eq!(
        names.transform("/Users/u/Documents/a/b.txt", true),
        "マイドキュメント/a/b.txt"
    );
    assert_eq!(names.transform("/var/tmp/t.txt", true), "TEMP/t.txt");
    // The folders of Windows do not exist: `%AppData%\` is left as typed and matches nothing.
    assert_eq!(
        names.transform("/Users/u/.config/x", true),
        "/Users/u/.config/x"
    );
}

#[test]
fn a_document_without_a_file_has_no_title() {
    let names = FileNames::plain();
    assert_eq!(names.name_of(None, None, true), NO_TITLE);
    assert_eq!(
        names.name_of(Some(std::path::Path::new("/a/b.txt")), None, true),
        "/a/b.txt"
    );
}

// ---- the short form ----

#[test]
fn a_path_that_fits_is_not_shortened() {
    assert_eq!(short_path("C:\\a\\b.txt", 20, true), "C:\\a\\b.txt");
    // The width is in columns: the 8 characters of the name are 16 columns.
    assert_eq!(short_path("日本語の名前.txt", 20, true), "日本語の名前.txt");
}

#[test]
fn the_folders_in_the_middle_are_left_out() {
    // The example of `GetShortViewPath`.
    assert_eq!(
        short_path("C:\\sub1\\sub2\\sub3\\file.ext", 20, true),
        "C:\\...\\sub3\\file.ext"
    );
    assert_eq!(
        short_path("C:\\sub1\\sub2\\sub3\\file.ext", 15, true),
        "C:\\...\\file.ext"
    );
    // With slashes.
    assert_eq!(
        short_path("/home/user/projects/ume/src/main.rs", 24, true),
        "/.../ume/src/main.rs"
    );
}

#[test]
fn japanese_names_count_two_columns_each() {
    let path = "C:\\ユーザー\\山田太郎\\ドキュメント\\報告書\\第一章.txt";
    let shown = short_path(path, 30, true);
    assert_eq!(shown, "C:\\...\\報告書\\第一章.txt");
    assert!(unicode_width::UnicodeWidthStr::width(shown.as_str()) <= 30);
    // A narrower limit leaves out more.
    let narrower = short_path(path, 20, true);
    assert_eq!(narrower, "C:\\...\\第一章.txt");
}

#[test]
fn a_name_that_is_still_too_long_is_cut_but_keeps_its_extension() {
    // `fit`: cut the title, keep the extension ("C:\...\longf...ext").
    let shown = short_path("C:\\dir\\sub\\とても長いファイル名のメモ帳.txt", 16, true);
    // `strTempOld` is one character behind, as in Sakura.
    assert_eq!(shown, "C:\\...\\と...txt");
    assert!(unicode_width::UnicodeWidthStr::width(shown.as_str()) <= 16);
    // Not `fit`: left as long as it is.
    assert_eq!(
        short_path("C:\\dir\\sub\\とても長いファイル名のメモ帳.txt", 16, false),
        "C:\\...\\とても長いファイル名のメモ帳.txt"
    );
}

#[test]
fn network_paths_and_urls_keep_their_head() {
    // `\\server\share\` stays.
    assert_eq!(
        short_path("\\\\server\\share\\a\\b\\c\\d\\file.txt", 29, true),
        "\\\\server\\share\\...\\d\\file.txt"
    );
    // `\\?\C:\` stays.
    assert_eq!(
        short_path("\\\\?\\C:\\one\\two\\three\\file.txt", 25, true),
        "\\\\?\\C:\\...\\three\\file.txt"
    );
    // `http://host/` stays.
    assert_eq!(
        short_path("http://host/one/.../page.html", 34, true),
        "http://host/one/.../page.html"
    );
}

#[test]
fn a_path_with_no_folder_to_leave_out_is_cut_only_with_fit() {
    assert_eq!(
        short_path("verylongfilename.txt", 8, false),
        "verylongfilename.txt"
    );
    assert_eq!(short_path("verylongfilename.txt", 8, true), "veryl...");
}

#[test]
fn the_setting_decides_whether_and_how_far_a_path_is_shortened() {
    let rows = [row("D:\\data\\", "データ\\")];
    let path = "D:\\data\\one\\two\\three\\file.txt";
    // Replaced first, then shortened.
    let names = windows_names(true, 20, &rows);
    assert_eq!(names.transform(path, true), "データ\\...\\file.txt");
    let off = windows_names(false, 20, &rows);
    assert_eq!(
        off.transform(path, true),
        "データ\\one\\two\\three\\file.txt"
    );
    // Without a list the short form works on the path itself.
    let only_short = windows_names(true, 20, &[]);
    assert_eq!(only_short.transform(path, true), "D:\\...\\file.txt");
}

// ---- the title ----

fn subject(path: Option<&str>) -> Subject<'_> {
    Subject {
        path,
        ..Subject::default()
    }
}

fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(windows)]
#[test]
fn the_active_title_shows_the_short_path_and_the_inactive_one_the_name() {
    let names = windows_names(true, 100, &sakura_rows());
    let path = Some("C:\\Users\\u\\Desktop\\メモ.txt");
    let active = expand(CAPTION_ACTIVE, &subject(path), &names);
    assert_eq!(
        active,
        format!("デスクトップ\\メモ.txt - UME Editor {} ", version()) + " "
    );
    let inactive = expand(CAPTION_INACTIVE, &subject(path), &names);
    assert_eq!(
        inactive,
        format!("メモ.txt - UME Editor {} ", version()) + " "
    );
}

#[test]
fn the_title_has_the_marks_of_the_document_and_the_window() {
    let names = FileNames::plain();
    let marked = Subject {
        path: Some("/a/b.txt"),
        modified: true,
        view_mode: true,
        unwritable: false,
        recording_macro: true,
        grep_key: None,
        caret: (0, 0),
        untitled_number: None,
    };
    assert_eq!(
        expand(CAPTION_ACTIVE, &marked, &names),
        format!(
            "/a/b.txt(更新) - UME Editor {} (ビューモード)  【キーマクロの記録中】 ",
            version()
        )
    );
    // A document with no file is 無題.
    assert_eq!(
        expand(CAPTION_INACTIVE, &subject(None), &names),
        format!("(無題) - UME Editor {}  ", version())
    );
}

/// `$n` and `STR_NO_TITLE1` with the number of the window
/// (`CSakuraEnvironment::ExpandParameter`, `GetMenuFullLabel`).
#[test]
fn an_untitled_document_shows_its_number() {
    let names = FileNames::plain();
    let third = Subject {
        untitled_number: Some(3),
        ..Subject::default()
    };
    assert_eq!(
        expand(CAPTION_INACTIVE, &third, &names),
        format!("(無題)3 - UME Editor {}  ", version())
    );
    // No number yet (`m_nId == -1`), or the number of a window that shows a file: nothing.
    assert_eq!(expand("$f$n", &subject(None), &names), "(無題)",);
    let file = Subject {
        path: Some("/a/b.txt"),
        untitled_number: Some(3),
        ..Subject::default()
    };
    assert_eq!(expand("$f$n", &file, &names), "b.txt");
    assert_eq!(names.name_of(None, Some(12), true), "(無題)12");
    assert_eq!(names.name_of(None, None, true), "(無題)");
    assert_eq!(
        names.name_of(Some(std::path::Path::new("/a/b.txt")), Some(12), true),
        "/a/b.txt"
    );
}

/// The headers of the print are made from the same context as the title (`$R`, `$N`).
#[test]
fn the_context_of_the_title_has_the_write_state_and_the_short_path() {
    use super::title::context;
    use ume_print::WriteState;
    let names = windows_names(true, 100, &sakura_rows());
    let locked = Subject {
        path: Some("C:\\Users\\u\\Desktop\\メモ.txt"),
        unwritable: true,
        ..Subject::default()
    };
    let ctx = context(&locked, &names);
    assert_eq!(ctx.write_state, WriteState::Locked);
    assert_eq!(
        ctx.file_path_short.as_deref(),
        Some("デスクトップ\\メモ.txt")
    );
    let view = Subject {
        view_mode: true,
        unwritable: true,
        ..locked
    };
    assert_eq!(context(&view, &names).write_state, WriteState::ViewMode);
    assert_eq!(
        context(&subject(None), &names).write_state,
        WriteState::Writable
    );
}

#[test]
fn a_grep_window_shows_its_key() {
    let names = FileNames::plain();
    let grep = Subject {
        grep_key: Some("検索語"),
        ..Subject::default()
    };
    assert_eq!(
        expand(CAPTION_ACTIVE, &grep, &names),
        format!("検索語 - UME Editor {}  ", version())
    );
    // The key is cut at 32 units, with `...`.
    let long = "a".repeat(40);
    let grep = Subject {
        grep_key: Some(&long),
        ..Subject::default()
    };
    assert!(
        expand(CAPTION_ACTIVE, &grep, &names).starts_with(&format!("{}... - ", "a".repeat(32)))
    );
}

#[cfg(windows)]
#[test]
fn the_folder_of_the_title_is_shortened_too() {
    let names = windows_names(false, 100, &sakura_rows());
    let shown = expand(
        "$E|$e|$N|$F",
        &subject(Some("C:\\Users\\u\\Desktop\\a.txt")),
        &names,
    );
    assert_eq!(
        shown,
        "デスクトップ\\|C:\\Users\\u\\Desktop\\|デスクトップ\\a.txt|C:\\Users\\u\\Desktop\\a.txt"
    );
}

#[test]
fn a_users_format_is_expanded() {
    let names = FileNames::plain();
    assert_eq!(
        expand(
            "[$g]$b $$ ${U?*$:-$}",
            &subject(Some("/a/b.tar.gz")),
            &names
        ),
        "[b.tar]gz $ -"
    );
}

#[test]
fn an_unwritable_document_is_marked_and_the_view_mode_comes_first() {
    let names = FileNames::plain();
    let unwritable = Subject {
        path: Some("/a/b.txt"),
        unwritable: true,
        ..Subject::default()
    };
    assert_eq!(
        expand(CAPTION_ACTIVE, &unwritable, &names),
        format!("/a/b.txt - UME Editor {} (上書き禁止) ", version())
    );
    // `_ExParam_Evaluate`: the view mode wins over a file that cannot be written.
    let both = Subject {
        view_mode: true,
        ..unwritable
    };
    assert_eq!(
        expand(CAPTION_ACTIVE, &both, &names),
        format!("/a/b.txt - UME Editor {} (ビューモード) ", version())
    );
}

#[test]
fn the_caret_of_the_title_counts_from_one() {
    let names = FileNames::plain();
    let at = Subject {
        caret: (4, 9),
        ..Subject::default()
    };
    assert_eq!(expand("$y 行 $x 桁", &at, &names), "5 行 10 桁");
}

#[test]
fn the_separator_after_the_head_is_not_repeated() {
    // A folder path with one folder after the head: `C:\` once, not twice.
    assert_eq!(
        short_path("C:\\longfolder\\", 10, false),
        "C:\\longfolder\\"
    );
    assert_eq!(short_path("C:\\longfolder\\", 10, true), "C:\\long...");
}
