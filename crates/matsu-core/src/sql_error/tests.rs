use super::*;

fn at(file: Option<&str>, line: usize, column: Option<usize>) -> Option<ErrorSpot> {
    Some(ErrorSpot {
        file: file.map(str::to_owned),
        line,
        column,
    })
}

#[test]
fn psql_names_the_file_and_the_line() {
    let p = |t| parse(ErrorStyle::PostgreSql, t);
    assert_eq!(
        p("psql:/work/a.sql:12: ERROR:  syntax error at or near \"x\""),
        at(Some("/work/a.sql"), 12, None)
    );
    // a drive letter's colon is not the end of the file name
    assert_eq!(
        p("psql:C:\\work\\a.sql:3: ERROR:  relation \"t\" does not exist"),
        at(Some("C:\\work\\a.sql"), 3, None)
    );
    // the standard input has no file
    assert_eq!(p("psql:<stdin>:7: ERROR:  x"), at(None, 7, None));
    // the line of the statement (`LINE 1:`) is not a place in the file
    assert_eq!(p("LINE 1: select * frm t;"), None);
    assert_eq!(p("ERROR:  syntax error"), None);
}

#[test]
fn mysql_says_at_line() {
    let p = |t| parse(ErrorStyle::MySql, t);
    assert_eq!(
        p("ERROR 1064 (42000) at line 3: You have an error in your SQL syntax"),
        at(None, 3, None)
    );
    assert_eq!(
        p("ERROR 1146 (42S02) at line 9 in file: '/w/a.sql': Table 'd.t' doesn't exist"),
        at(Some("/w/a.sql"), 9, None)
    );
    assert_eq!(p("ERROR 1045 (28000): Access denied"), None);
    assert_eq!(p("Warning: at line 3"), None);
}

#[test]
fn sqlcmd_says_line() {
    let p = |t| parse(ErrorStyle::SqlServer, t);
    assert_eq!(
        p("Msg 102, Level 15, State 1, Server DB1, Line 4"),
        at(None, 4, None)
    );
    assert_eq!(
        p("Msg 208, Level 16, State 1, Procedure p, Line 12"),
        at(None, 12, None)
    );
    assert_eq!(p("Invalid object name 'x'."), None);
}

#[test]
fn sqlite_says_near_line() {
    let p = |t| parse(ErrorStyle::Sqlite, t);
    assert_eq!(
        p("Parse error near line 3: near \"frm\": syntax error"),
        at(None, 3, None)
    );
    assert_eq!(
        p("Runtime error near line 5: no such table: t (1)"),
        at(None, 5, None)
    );
    assert_eq!(p("Error: near line 2: in prepare"), at(None, 2, None));
    assert_eq!(p("sqlite> near line 2"), None);
}

#[test]
fn bq_says_the_line_and_column_in_brackets() {
    let p = |t| parse(ErrorStyle::BigQuery, t);
    assert_eq!(
        p(
            "BigQuery error in query operation: Error processing job 'p:j': Syntax error: Unexpected identifier \"x\" at [3:5]"
        ),
        at(None, 3, Some(5))
    );
    assert_eq!(p("Unexpected end of script at [2:"), None);
    assert_eq!(
        p("Not found: Table p:d.t was not found in location US"),
        None
    );
}

#[test]
fn oracle_clients() {
    let p = |t| parse(ErrorStyle::Oracle, t);
    assert_eq!(p("ERROR at line 3:"), at(None, 3, None));
    assert_eq!(
        p("Error starting at line : 7 in command -"),
        at(None, 7, None)
    );
    assert_eq!(
        p("Error at Command Line : 7 Column : 12"),
        at(None, 7, Some(12))
    );
    assert_eq!(
        p("5/3     PLS-00103: Encountered the symbol \"X\""),
        at(None, 5, Some(3))
    );
    assert_eq!(p("ORA-00942: table or view does not exist"), None);
    assert_eq!(p("10/12 rows selected"), None);
    assert_eq!(p("Error report -"), None);
}

#[test]
fn a_jump_does_not_reach_a_network_path_or_a_device() {
    for path in [
        "a.sql",
        "sub/b.sql",
        "/work/a.sql",
        "C:\\work\\a.sql",
        "../up/a.sql",
        "日本語/ファイル.sql",
        "console.sql",
        "com10.sql",
        "com0.sql",
    ] {
        assert!(is_safe_jump_path(path), "{path}");
    }
    for path in [
        "",
        "\\\\evil\\share\\x.sql",
        "//evil/share/x.sql",
        "\\\\?\\C:\\x.sql",
        "\\\\.\\COM1",
        "\\/evil/x.sql",
        "/\\evil\\x.sql",
        "NUL",
        "nul.txt",
        "dir/CON.sql",
        "C:\\x\\com1",
        "LPT9.sql",
        "AUX",
        "a\0b.sql",
    ] {
        assert!(!is_safe_jump_path(path), "{path:?}");
    }
}

#[test]
fn the_common_form_works_for_every_style() {
    for style in [
        ErrorStyle::Generic,
        ErrorStyle::Oracle,
        ErrorStyle::PostgreSql,
    ] {
        assert_eq!(
            parse(style, "a.sql:2: ERROR: boom"),
            at(Some("a.sql"), 2, None)
        );
        assert_eq!(
            parse(style, "sub/b.sql:14:7: error: boom"),
            at(Some("sub/b.sql"), 14, Some(7))
        );
        assert_eq!(
            parse(style, "C:\\w\\c.sql:5: boom"),
            at(Some("C:\\w\\c.sql"), 5, None)
        );
    }
    for text in [
        "no numbers here",
        "12:30:45 started",
        "note: 3: not a file",
        ":3: x",
        "a.sql:0: zero",
        "a.sql:3 no second colon",
        "-v:3: option",
        "#CmdLine  : psql -f a.sql:3: x",
    ] {
        assert_eq!(parse(ErrorStyle::Generic, text), None, "{text}");
    }
}
