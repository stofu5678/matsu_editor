//! Tests of the command line. The cases follow `src/test/cpp/tests1/test-ccommandline.cpp` of
//! Sakura and the rules of `CCommandLine::ParseCommandLine` / `CheckCommandLine`.

use std::ffi::OsString;
use std::path::PathBuf;

use super::*;

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn line(args: &[&str]) -> CommandLine {
    parse(&os(args))
}

fn files(args: &[&str]) -> Vec<PathBuf> {
    line(args).files
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

#[test]
fn nothing_is_the_state_before_reading() {
    let cl = line(&[]);
    assert_eq!(cl, CommandLine::default());
    assert_eq!(cl.caret_x, None);
    assert_eq!(cl.window_x, None);
    assert!(!cl.view_mode && !cl.grep_mode && !cl.no_window);
    assert_eq!(cl.group, None);
    assert_eq!(cl.grep, GrepArgs::default());
}

#[test]
fn names_without_dashes_are_files_in_order() {
    assert_eq!(
        files(&["a.txt", "b.txt", "日本語.txt"]),
        paths(&["a.txt", "b.txt", "日本語.txt"])
    );
}

#[test]
fn options_and_files_mix_on_one_line() {
    let cl = line(&["-R", "a.txt", "-X=3", "b.txt", "-CODE=4"]);
    assert_eq!(cl.files, paths(&["a.txt", "b.txt"]));
    assert!(cl.view_mode);
    assert_eq!(cl.caret_x, Some(2));
    assert_eq!(cl.code, Some(4));
}

#[test]
fn options_without_a_file() {
    let cl = line(&["-GREPMODE", "-GKEY=abc"]);
    assert!(cl.files.is_empty());
    assert!(cl.grep_mode);
    assert_eq!(cl.grep.key, "abc");
}

#[test]
fn caret_and_view_count_from_one() {
    // ParseCaretLocationX, Y, ParseViewLeftCol, ParseViewTopLine
    let cl = line(&["-X=123", "-Y=45", "-VX=6", "-VY=7"]);
    assert_eq!(cl.caret_x, Some(122));
    assert_eq!(cl.caret_y, Some(44));
    assert_eq!(cl.view_left, Some(5));
    assert_eq!(cl.view_top, Some(6));
}

#[test]
fn a_number_below_one_is_no_position() {
    assert_eq!(line(&["-X=1"]).caret_x, Some(0));
    assert_eq!(line(&["-X=0"]).caret_x, None);
    assert_eq!(line(&["-X=-5"]).caret_x, None);
    assert_eq!(line(&["-Y=abc"]).caret_y, None);
    // a later option takes the value of an earlier one back
    assert_eq!(line(&["-X=5", "-X=0"]).caret_x, None);
}

#[test]
fn the_window_size_is_one_less_and_the_place_is_as_given() {
    // ParseWindowSizeX, Y, ParseWindowOriginX, Y
    let cl = line(&["-SX=123", "-SY=77", "-WX=123", "-WY=-40"]);
    assert_eq!(cl.window_width, Some(122));
    assert_eq!(cl.window_height, Some(76));
    assert_eq!(cl.window_x, Some(123));
    assert_eq!(cl.window_y, Some(-40));
    let none = line(&["-SX=0", "-SY=x", "-WX=-2147483648"]);
    assert_eq!(none.window_width, None);
    assert_eq!(none.window_height, None);
    // CW_USEDEFAULT is no place
    assert_eq!(none.window_x, None);
}

#[test]
fn doc_type_is_cut_at_seven_characters() {
    // ParseDocType
    assert_eq!(line(&["-TYPE=C/C++"]).doc_type.as_deref(), Some("C/C++"));
    assert_eq!(
        line(&["-TYPE=TooLongTypeName"]).doc_type.as_deref(),
        Some("TooLong")
    );
    assert_eq!(
        line(&["-TYPE=日本語日本語日本語"]).doc_type.as_deref(),
        Some("日本語日本語日")
    );
    assert_eq!(line(&[]).doc_type, None);
}

#[test]
fn the_encoding_number_is_kept_as_given() {
    assert_eq!(line(&["-CODE=4"]).code, Some(4));
    assert_eq!(line(&["-CODE=99"]).code, Some(99));
    assert_eq!(line(&["-CODE=2", "-CODE=0"]).code, Some(0));
    // not a number: _wtoi gives 0
    assert_eq!(line(&["-CODE=utf"]).code, Some(0));
    assert_eq!(line(&["-CODE=4abc"]).code, Some(4));
    assert_eq!(line(&["-CODE=  7"]).code, Some(7));
    assert_eq!(line(&["-CODE=+8"]).code, Some(8));
}

#[test]
fn flags_without_values() {
    // ParseNoWin, ParseViewMode, ParseDebugMode, ParseProfileManager
    assert!(line(&["-NOWIN"]).no_window);
    assert!(line(&["-R"]).view_mode);
    assert!(line(&["-PROFMGR"]).profile_manager);
    assert!(line(&["-GREPDLG"]).grep_dialog);
    let debug = line(&["-DEBUGMODE"]);
    assert!(debug.debug_mode);
    assert_eq!(debug.doc_type.as_deref(), Some("output"));
}

#[test]
fn grep_mode_gives_the_grep_type_unless_one_was_given_first() {
    assert_eq!(line(&["-GREPMODE"]).doc_type.as_deref(), Some("grepout"));
    assert_eq!(
        line(&["-TYPE=txt", "-GREPMODE"]).doc_type.as_deref(),
        Some("txt")
    );
    // -TYPE after it takes the place ("unconditionally")
    assert_eq!(
        line(&["-GREPMODE", "-TYPE=txt"]).doc_type.as_deref(),
        Some("txt")
    );
}

#[test]
fn a_flag_with_a_value_is_not_a_flag() {
    let cl = line(&["-R=1", "-NOWIN=1", "-GREPMODE=1"]);
    assert!(!cl.view_mode && !cl.no_window && !cl.grep_mode);
    assert!(cl.files.is_empty());
}

#[test]
fn option_names_do_not_care_about_the_case() {
    let cl = line(&["-r", "-x=3", "-Vy=9", "-gkey=k", "-grepmode", "-Type=txt"]);
    assert!(cl.view_mode && cl.grep_mode);
    assert_eq!(cl.caret_x, Some(2));
    assert_eq!(cl.view_top, Some(8));
    assert_eq!(cl.grep.key, "k");
    assert_eq!(cl.doc_type.as_deref(), Some("txt"));
}

#[test]
fn a_colon_is_an_equal_sign() {
    let cl = line(&["-X:3", "-GKEY:abc", "-M:run.mac"]);
    assert_eq!(cl.caret_x, Some(2));
    assert_eq!(cl.grep.key, "abc");
    assert_eq!(cl.macro_, "run.mac");
}

#[test]
fn an_option_without_a_value_is_dropped() {
    // 2010.06.12 syat: a missing value is no option
    let cl = line(&[
        "-X=",
        "-CODE=",
        "-GKEY=",
        "-GKEY=\"\"",
        "-M=",
        "-X",
        "-TYPE",
    ]);
    assert_eq!(cl, CommandLine::default());
}

#[test]
fn replace_and_profile_may_have_an_empty_value() {
    let cl = line(&["-GREPR=", "-PROF="]);
    assert_eq!(cl.grep.replace.as_deref(), Some(""));
    assert_eq!(cl.profile.as_deref(), Some(""));
    assert_eq!(line(&["-PROF=work"]).profile.as_deref(), Some("work"));
    assert_eq!(line(&[]).profile, None);
    assert_eq!(line(&[]).grep.replace, None);
}

#[test]
fn an_unknown_option_is_dropped_and_is_not_a_file() {
    let cl = line(&["-NOSUCH", "-FOO=1", "-", "a.txt"]);
    assert_eq!(
        cl,
        CommandLine {
            files: paths(&["a.txt"]),
            ..CommandLine::default()
        }
    );
}

#[test]
fn two_dashes_end_the_options() {
    // the example of the help: sakura.exe -- "-aaa.txt"
    assert_eq!(files(&["--", "-aaa.txt"]), paths(&["-aaa.txt"]));
    let cl = line(&["-R", "--", "-R", "-X=3", "--"]);
    assert!(cl.view_mode);
    assert_eq!(cl.caret_x, None);
    assert_eq!(cl.files, paths(&["-R", "-X=3", "--"]));
    // options before it count
    assert_eq!(line(&["-X=3", "--", "a"]).caret_x, Some(2));
}

#[test]
fn quotes_around_an_option_and_around_its_value_are_taken_away() {
    // Visual Studio puts quotes around each argument (2007.09.09 genta)
    let cl = line(&["\"-R\"", "\"-CODE=4\"", "-X=\"7\"", "\"-Y=\"8\"\""]);
    assert!(cl.view_mode);
    assert_eq!(cl.code, Some(4));
    assert_eq!(cl.caret_x, Some(6));
    assert_eq!(cl.caret_y, Some(7));
    // the closing mark may be missing
    assert_eq!(line(&["\"-X=3"]).caret_x, Some(2));
    assert_eq!(line(&["-X=\"3"]).caret_x, Some(2));
    // a number may start with a quotation mark (AtoiOptionInt)
    assert_eq!(line(&["-X='5"]).caret_x, Some(4));
}

#[test]
fn quotes_around_a_file_are_taken_away_and_doubled_ones_are_one() {
    assert_eq!(
        files(&["\"C:\\my dir\\a.txt\""]),
        paths(&["C:\\my dir\\a.txt"])
    );
    assert_eq!(files(&["\"a b.txt"]), paths(&["a b.txt"]));
    assert_eq!(
        files(&["\"the \"\"3rd\"\" file\""]),
        paths(&["the \"3rd\" file"])
    );
    // one mark alone, and an empty pair, are no file
    assert!(files(&["\""]).is_empty());
    assert!(files(&["\"\""]).is_empty());
    assert!(files(&[""]).is_empty());
}

#[test]
fn a_quoted_name_that_starts_with_a_dash_is_an_option_unless_after_two_dashes() {
    // help: a file name that starts with `-` is an option even in quotes
    assert!(files(&["\"-a.txt\""]).is_empty());
    assert_eq!(files(&["--", "\"-a.txt\""]), paths(&["-a.txt"]));
}

#[test]
fn file_urls_are_files() {
    // Nov. 11, 2005 susu: `file:///` is dropped; UME Editor also decodes the %
    if cfg!(unix) {
        assert_eq!(files(&["file:///tmp/a%20b.txt"]), paths(&["/tmp/a b.txt"]));
        assert_eq!(files(&["\"file:///tmp/c.txt\""]), paths(&["/tmp/c.txt"]));
    }
    // other URLs are names
    assert_eq!(
        files(&["https://example.com/a"]),
        paths(&["https://example.com/a"])
    );
}

#[cfg(unix)]
#[test]
fn a_name_that_is_not_text_is_a_file() {
    use std::os::unix::ffi::OsStringExt;
    let name = OsString::from_vec(vec![b'-', 0xFF, b'.', b't']);
    let cl = parse(&[name.clone(), OsString::from("a")]);
    assert_eq!(cl.files, vec![PathBuf::from(name), PathBuf::from("a")]);
}

#[test]
fn grep_conditions() {
    // ParseGrepKey, ParseGrepReplaceKey, ParseGrepFile, ParseGrepFolder
    let cl = line(&[
        "-GREPMODE",
        "-GKEY=\\w+",
        "-GREPR=$1。",
        "-GFILE=*.rs;#.git;!*.lock",
        "-GFOLDER=C:\\work\\sakura",
    ]);
    assert_eq!(cl.grep.key, "\\w+");
    assert_eq!(cl.grep.replace.as_deref(), Some("$1。"));
    assert_eq!(cl.grep.file, "*.rs;#.git;!*.lock");
    assert_eq!(cl.grep.folder, "C:\\work\\sakura");
}

#[test]
fn doubled_quotes_in_grep_values_are_one() {
    // the help: -GKEY="printf( ""%s"  searches for  printf( "%s
    assert_eq!(line(&["-GKEY=\"printf( \"\"%s\""]).grep.key, "printf( \"%s");
    assert_eq!(line(&["-GKEY=printf(\"\"x"]).grep.key, "printf(\"x");
    assert_eq!(
        line(&["-GREPR=\"a\"\"b\""]).grep.replace.as_deref(),
        Some("a\"b")
    );
    assert_eq!(line(&["-GFILE=\"a\"\"b\""]).grep.file, "a\"b");
    assert_eq!(line(&["-GFOLDER=\"C:\\x y\""]).grep.folder, "C:\\x y");
    assert_eq!(
        line(&["-M=\"Editor.InsText('a\"\"b')\""]).macro_,
        "Editor.InsText('a\"b')"
    );
}

#[test]
fn the_grep_conditions_start_as_sakura_has_them() {
    // ConstructWithoutParam: the defaults of GrepInfo
    let g = GrepArgs::default();
    assert_eq!(g.charset, 0, "CODE_SJIS");
    assert_eq!(g.output_style, 1);
    assert_eq!(g.output_line_type, 0);
    assert!(g.header);
    assert!(!g.sub_folder && !g.regex && !g.match_case && !g.whole_word);
    assert!(!g.paste && !g.backup && !g.except_file_regex && g.replace.is_none());
}

#[test]
fn each_letter_of_grep_options() {
    let on = |letters: &str| line(&[&format!("-GOPT={letters}")]).grep;
    assert!(on("X").keep_current_folder);
    assert!(on("U").stdout);
    assert!(!on("H").header);
    assert!(on("S").sub_folder);
    assert!(on("L").match_case);
    assert!(on("R").regex);
    assert!(on("W").whole_word);
    assert_eq!(on("K").charset, 99);
    assert_eq!(on("P").output_line_type, 1);
    assert_eq!(on("N").output_line_type, 2);
    assert_eq!(on("1").output_style, 1);
    assert_eq!(on("2").output_style, 2);
    assert_eq!(on("3").output_style, 3);
    assert!(on("F").file_only);
    assert!(on("B").base_folder);
    assert!(on("D").separate_folder);
    assert!(on("C").paste);
    assert!(on("O").backup);
    assert!(on("E").except_file_regex);
}

#[test]
fn grep_option_letters_combine_and_only_capitals_count() {
    let g = line(&["-GOPT=SRK"]).grep;
    assert!(g.sub_folder && g.regex);
    assert_eq!(g.charset, 99);
    // ParseGrepExceptFileRegexp_Combination: a small e is nothing
    let g = line(&["-GOPT=Se"]).grep;
    assert!(g.sub_folder && !g.except_file_regex);
    // letters that mean nothing are skipped
    let g = line(&["-GOPT=Z?S"]).grep;
    assert!(g.sub_folder);
    // the last output letter wins
    assert_eq!(line(&["-GOPT=P", "-GOPT=N"]).grep.output_line_type, 2);
    assert_eq!(line(&["-GOPT=3P2"]).grep.output_style, 2);
    // the options add up between -GOPT arguments
    let g = line(&["-GOPT=S", "-GOPT=R"]).grep;
    assert!(g.sub_folder && g.regex);
}

#[test]
fn grep_encoding() {
    assert_eq!(line(&["-GCODE=4"]).grep.charset, 4);
    assert_eq!(line(&["-GCODE=99"]).grep.charset, 99);
    // K and -GCODE work on the same value; the later one wins
    assert_eq!(line(&["-GOPT=K", "-GCODE=4"]).grep.charset, 4);
    assert_eq!(line(&["-GCODE=4", "-GOPT=K"]).grep.charset, 99);
}

#[test]
fn macro_and_its_type() {
    // ParseMacroFileName, ParseMacroType
    let cl = line(&["-M=真っ黒.mac", "-MTYPE=PascalScript"]);
    assert_eq!(cl.macro_, "真っ黒.mac");
    assert_eq!(cl.macro_type, "PascalScript");
    // -M is not -MTYPE
    let cl = line(&["-MTYPE=js"]);
    assert_eq!(cl.macro_, "");
    assert_eq!(cl.macro_type, "js");
    // the examples of the help
    let cl = line(&["-M=InsText('data')", "-MTYPE=mac", "a.txt"]);
    assert_eq!(cl.macro_, "InsText('data')");
    assert_eq!(cl.files, paths(&["a.txt"]));
    let cl = line(&["-M=D:\\macro.mac", "-MTYPE=file"]);
    assert_eq!(cl.macro_, "D:\\macro.mac");
}

#[test]
fn group_and_profile() {
    // ParseGroup, ParseProfileName
    assert_eq!(line(&["-GROUP=3"]).group, Some(3));
    assert_eq!(line(&["-GROUP=0"]).group, Some(0));
    assert_eq!(line(&["-PROF=執筆用"]).profile.as_deref(), Some("執筆用"));
    assert_eq!(
        line(&["-PROF=\"my work\""]).profile.as_deref(),
        Some("my work")
    );
}

#[test]
fn a_value_that_has_an_equal_sign_keeps_it() {
    assert_eq!(line(&["-GKEY=a=b:c"]).grep.key, "a=b:c");
    assert_eq!(line(&["-M=x=y"]).macro_, "x=y");
}

#[test]
fn the_first_name_that_fits_wins() {
    // M and MTYPE, X and the others that end in X
    let cl = line(&["-MTYPE=a", "-M=b", "-VX=2", "-WX=3", "-SX=4"]);
    assert_eq!(cl.macro_type, "a");
    assert_eq!(cl.macro_, "b");
    assert_eq!(cl.view_left, Some(1));
    assert_eq!(cl.window_x, Some(3));
    assert_eq!(cl.window_width, Some(3));
    assert_eq!(cl.caret_x, None);
}

#[test]
fn a_response_file_adds_its_arguments_after_the_real_ones() {
    let mut asked = Vec::new();
    let mut read = |path: &Path| {
        asked.push(path.to_owned());
        Some("c.txt -X=9  -R  d.txt".to_owned())
    };
    let cl = parse_with(&os(&["a.txt", "-@=args.rsp", "b.txt"]), &mut read);
    assert_eq!(asked, [PathBuf::from("args.rsp")]);
    assert_eq!(cl.files, paths(&["a.txt", "b.txt", "c.txt", "d.txt"]));
    assert_eq!(cl.caret_x, Some(8));
    assert!(cl.view_mode);
}

#[test]
fn a_response_file_has_no_response_file() {
    let mut calls = 0;
    let mut read = |_: &Path| {
        calls += 1;
        Some("-@=again.rsp x.txt".to_owned())
    };
    let cl = parse_with(&os(&["-@=args.rsp"]), &mut read);
    assert_eq!(calls, 1);
    assert_eq!(cl.files, paths(&["x.txt"]));
}

#[test]
fn a_response_file_that_cannot_be_read_is_nothing() {
    let cl = parse_with(&os(&["-@=missing.rsp", "a.txt"]), &mut |_| None);
    assert_eq!(cl.files, paths(&["a.txt"]));
    // and without a reader the option is only noted
    assert_eq!(line(&["-@=x"]), CommandLine::default());
}

#[test]
fn the_last_response_file_counts() {
    let mut asked = Vec::new();
    let mut read = |path: &Path| {
        asked.push(path.to_owned());
        None
    };
    parse_with(&os(&["-@=one", "-@=two"]), &mut read);
    assert_eq!(asked, [PathBuf::from("two")]);
}

#[test]
fn response_text_reads_utf8_with_a_bom_and_shift_jis() {
    let mut utf8 = vec![0xEF, 0xBB, 0xBF];
    utf8.extend("-X=3 日本語.txt".as_bytes());
    assert_eq!(response_text(&utf8), "-X=3 日本語.txt");
    // "日本語" in Shift_JIS
    assert_eq!(
        response_text(&[0x93, 0xFA, 0x96, 0x7B, 0x8C, 0xEA]),
        "日本語"
    );
    // the lines are glued without anything between them (`ReadLineW`)
    assert_eq!(response_text(b"a.txt \r\n-R\r\nb.txt"), "a.txt -Rb.txt");
}

#[test]
fn profile_names_stay_in_the_settings_folder() {
    assert_eq!(valid_profile_name("work"), Some("work"));
    assert_eq!(valid_profile_name("執筆 用"), Some("執筆 用"));
    for bad in [
        "", ".", "..", "a/b", "a\\b", "C:", "../x", "a:b", "x|y", "a\nb",
    ] {
        assert_eq!(valid_profile_name(bad), None, "{bad:?}");
    }
    assert_eq!(valid_profile_name(&"x".repeat(260)), None);
}

#[test]
fn numbers_are_read_as_cs_atoi_does() {
    assert_eq!(atoi("123"), 123);
    assert_eq!(atoi("  -12x"), -12);
    assert_eq!(atoi("+0"), 0);
    assert_eq!(atoi(""), 0);
    assert_eq!(atoi("x1"), 0);
    assert_eq!(atoi("99999999999"), i32::MAX);
    assert_eq!(atoi("-99999999999"), i32::MIN);
    assert_eq!(atoi("2147483648"), i32::MAX);
}
