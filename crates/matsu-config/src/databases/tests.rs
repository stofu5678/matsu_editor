use super::*;

fn read(text: &str) -> Databases {
    Databases::from_profile(&Profile::from_bytes(text.as_bytes()))
}

fn connection(name: &str, command: &str) -> Connection {
    Connection {
        name: name.into(),
        kind: Kind::Other,
        command: command.into(),
        folder: None,
        encoding: Encoding::Utf8,
        input_encoding: None,
    }
}

#[test]
fn reads_connections_and_the_default() {
    let d = read(
        "[Database]\nDefault=work\n\n[Connection.work]\nKind=PostgreSQL\nCommand=psql -f $F\nEncoding=shift_jis\nFolder=C:\\sql\n\n[Connection.other one]\nCommand=tool\n",
    );
    assert!(d.problems.is_empty(), "{:?}", d.problems);
    assert_eq!(d.default.as_deref(), Some("work"));
    let work = d.default_connection().unwrap();
    assert_eq!(work.kind, Kind::PostgreSql);
    assert_eq!(work.command, "psql -f $F");
    assert_eq!(work.encoding, Encoding::ShiftJis);
    assert_eq!(work.input_encoding(), Encoding::ShiftJis);
    assert_eq!(work.folder.as_deref(), Some("C:\\sql"));
    let other = d.get("other one").unwrap();
    assert_eq!(other.kind, Kind::Other);
    assert_eq!(other.encoding, Encoding::Utf8);
    assert_eq!(other.folder, None);
}

#[test]
fn a_wrong_section_does_not_stop_the_others() {
    let d = read(
        "[Connection.a]\nKind=oracle\n\n[Connection.b]\nKind=cobol\nCommand=x\n\n[Connection.c]\nCommand=x\nEncoding=ebcdic\n\n[Connection.d]\nCommand=ok\n\n[Database]\nDefault=zzz\n",
    );
    assert_eq!(d.connections.len(), 1);
    assert_eq!(d.connections[0].name, "d");
    assert!(d.problems.contains(&Problem::NoCommand("a".into())));
    assert!(matches!(&d.problems[1], Problem::UnknownKind { value, .. } if value == "cobol"));
    assert!(matches!(&d.problems[2], Problem::UnknownEncoding { value, .. } if value == "ebcdic"));
    assert!(d.problems.contains(&Problem::NoSuchDefault("zzz".into())));
    assert!(d.default_connection().is_none());
}

#[test]
fn the_first_section_of_a_name_counts() {
    let d = read("[Connection.a]\nCommand=first\n[Connection.a]\nCommand=second\n");
    assert_eq!(d.connections.len(), 1);
    assert_eq!(d.connections[0].command, "first");
}

#[test]
fn templates_split_at_the_file() {
    assert_eq!(
        template_parts("psql -f $F --x=$$HOME $Q"),
        [
            Part::Text("psql -f ".into()),
            Part::File,
            Part::Text(" --x=$HOME $Q".into())
        ]
    );
    assert_eq!(template_parts("a$"), [Part::Text("a$".into())]);
    // `$$F` is a dollar sign and an F, not the file
    assert_eq!(template_parts("$$F"), [Part::Text("$F".into())]);
    assert!(connection("n", "x $F").uses_file());
    assert!(!connection("n", "x $$F").uses_file());
    assert!(!connection("n", "sqlite3 db").uses_file());
}

#[test]
fn writing_and_reading_agree_and_other_lines_stay() {
    let mut profile = Profile::from_bytes(b"; mine\n[Database]\nDefault=old\n\n[Other]\nx=1\n");
    let written = Databases {
        default: Some("a".into()),
        connections: vec![Connection {
            name: "a".into(),
            kind: Kind::MySql,
            command: "mysql --login-path=l".into(),
            folder: Some("/tmp".into()),
            encoding: Encoding::Utf16,
            input_encoding: Some(Encoding::Utf8),
        }],
        problems: Vec::new(),
    };
    written.write_to(&mut profile).unwrap();
    let text = String::from_utf8(profile.to_bytes()).unwrap();
    assert!(text.contains("; mine") && text.contains("[Other]"));
    assert_eq!(Databases::from_profile(&profile), written);
}

#[test]
fn a_bad_name_is_not_written() {
    for name in ["", " a", "a]", "a=b", "a\nb"] {
        let d = Databases {
            default: None,
            connections: vec![connection(name, "x")],
            problems: Vec::new(),
        };
        assert!(d.write_to(&mut Profile::new()).is_err(), "{name:?}");
    }
}

#[test]
fn passwords_on_the_command_line_are_found() {
    let found = password_in_command;
    assert_eq!(found(Kind::MySql, "mysql -u me -psecret db"), Some("-p"));
    assert_eq!(found(Kind::MySql, "mysql -u me -p db"), None);
    assert_eq!(
        found(Kind::MySql, "mysql --password=secret"),
        Some("--password=")
    );
    assert_eq!(found(Kind::MySql, "mysql --password db"), None);
    // a connection string with the parameter (libpq, ODBC), quoted or not
    assert_eq!(
        found(
            Kind::PostgreSql,
            "psql \"host=localhost dbname=app user=me password=secret\" -f $F"
        ),
        Some("password=")
    );
    assert_eq!(
        found(Kind::PostgreSql, "psql \"password='x y'\" -f $F"),
        Some("password=")
    );
    assert_eq!(
        found(Kind::Other, "tool \"Server=h;Database=d;Pwd=secret;\""),
        Some("password=")
    );
    assert_eq!(found(Kind::Other, "tool \"Server=h;Password=;\""), None);
    assert_eq!(
        found(Kind::Other, "tool --nopassword=1 --passwordfile=x"),
        None
    );
    // psql's -p is the port
    assert_eq!(found(Kind::PostgreSql, "psql -p5432 -d db -f $F"), None);
    assert_eq!(
        found(Kind::PostgreSql, "psql postgresql://me:pw@host/db"),
        Some("://user:password@")
    );
    assert_eq!(
        found(Kind::PostgreSql, "psql postgresql://me@host/db"),
        None
    );
    assert_eq!(
        found(Kind::PostgreSql, "PGPASSWORD=x psql"),
        Some("PGPASSWORD=")
    );
    assert_eq!(
        found(Kind::SqlServer, "sqlcmd -U me -P secret -i $F"),
        Some("-P")
    );
    assert_eq!(found(Kind::SqlServer, "sqlcmd -S host -E -i $F"), None);
    assert_eq!(
        found(Kind::Oracle, "sqlplus scott/tiger@db @$F"),
        Some("user/password@")
    );
    assert_eq!(found(Kind::Oracle, "sqlplus -L /@MYDB @$F"), None);
    assert_eq!(found(Kind::Oracle, "sqlplus -L scott@db @$F"), None);
    assert_eq!(found(Kind::Oracle, "/usr/bin/sqlplus -L /@db"), None);
    assert_eq!(found(Kind::BigQuery, "bq query"), None);
}

#[test]
fn the_examples_are_valid_and_have_no_password_and_no_default() {
    let bytes = example_file_bytes();
    let d = Databases::from_profile(&Profile::from_bytes(&bytes));
    assert!(d.problems.is_empty(), "{:?}", d.problems);
    assert_eq!(d.default, None);
    assert_eq!(d.connections.len(), 6);
    for c in &d.connections {
        assert_eq!(password_in_command(c.kind, &c.command), None, "{}", c.name);
    }
    // the file keeps the note about passwords for the reader
    assert!(
        String::from_utf8(bytes)
            .unwrap()
            .contains("Never write a password")
    );
}

#[test]
fn the_example_file_is_made_once() {
    let dir = std::env::temp_dir().join(format!("matsu-databases-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("deeper").join(FILE_NAME);
    assert!(create_example_file(&path).unwrap());
    let first = std::fs::read(&path).unwrap();
    std::fs::write(&path, b"[Database]\nDefault=mine\n").unwrap();
    assert!(!create_example_file(&path).unwrap());
    assert_eq!(std::fs::read(&path).unwrap(), b"[Database]\nDefault=mine\n");
    assert!(first.starts_with(&[0xEF, 0xBB, 0xBF]));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_file_is_beside_the_settings_file() {
    assert_eq!(
        path_beside(Path::new("/c/matsu-editor/matsu.ini")),
        PathBuf::from("/c/matsu-editor/databases.ini")
    );
}
