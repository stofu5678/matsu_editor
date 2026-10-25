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
    // an empty option before it does not hide it
    assert_eq!(
        found(Kind::MySql, "mysql --init-command= -psecret"),
        Some("-p")
    );
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
    assert_eq!(
        found(
            Kind::PostgreSql,
            "psql \"host=localhost password = secret\" -f $F"
        ),
        Some("password=")
    );
    // the parameters of a URI: `?password=x`, `&password=x`, the key's passphrase
    for command in [
        "psql \"postgresql://alice@localhost/app?password=secret\" -f $F",
        "psql \"postgresql://alice@localhost/app?sslmode=require&password=secret\"",
        "psql postgresql://localhost/app?sslpassword=secret",
        "tool mysql://h/db?passphrase=x",
    ] {
        assert!(found(Kind::PostgreSql, command).is_some(), "{command}");
    }
    assert_eq!(
        found(
            Kind::PostgreSql,
            "psql \"postgresql://alice@localhost/app?sslmode=require&password=\""
        ),
        None
    );
    // outside a URI the `&` is the start of the password
    assert_eq!(
        found(Kind::PostgreSql, "psql \"host=localhost password=&secret\""),
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
        Some("user/password")
    );
    assert_eq!(
        found(Kind::Oracle, "sqlplus -L scott/tiger @$F"),
        Some("user/password")
    );
    assert_eq!(found(Kind::Oracle, "sqlplus -L / as sysdba @$F"), None);
    assert_eq!(
        found(Kind::Oracle, "sqlplus 'OPS$SCOTT/tiger@db' @$F"),
        Some("user/password")
    );
    assert_eq!(
        found(Kind::Oracle, "sqlplus \"scott/ti/ger@db\" @$F"),
        Some("user/password")
    );
    assert_eq!(found(Kind::Oracle, "sqlplus -L scripts/dir/x.sql"), None);
    // a program path with blanks is one word, not a login
    for command in [
        "\"/opt/Oracle Client/sqlplus\" -L /@MYDB @$F",
        "'/opt/Oracle Client/sqlplus' -L /@MYDB @$F",
        "\"C:\\Program Files\\Oracle\\sqlplus.exe\" -L /@MYDB @$F",
        "sqlplus -L /@MYDB @\"/my docs/a/b.sql\"",
    ] {
        assert_eq!(found(Kind::Oracle, command), None, "{command}");
    }
    // a login inside quotes with blanks, and what follows a quote that never closes
    assert_eq!(
        found(Kind::Oracle, "sqlplus \"scott/tiger@db as sysdba\" @$F"),
        Some("user/password")
    );
    assert_eq!(
        found(Kind::MySql, "mysql \"--init-command=x -psecret"),
        Some("-p")
    );
    // MySQL 8's numbered options
    assert_eq!(
        found(Kind::MySql, "mysql --password1=secret"),
        Some("--password=")
    );
    assert_eq!(found(Kind::MySql, "mysql --password2 db"), None);
    // a shell removes a backslash before a letter, and the program says the kind when the file does not
    assert_eq!(
        found(Kind::MySql, "mysql --passw\\ord=secret"),
        Some("--password=")
    );
    assert_eq!(found(Kind::Other, "mysql -u me -psecret db"), Some("-p"));
    assert_eq!(
        found(Kind::PostgreSql, "/usr/bin/mysql -psecret"),
        Some("-p")
    );
    assert_eq!(found(Kind::Other, "sqlcmd -U me -P secret"), Some("-P"));
    assert_eq!(found(Kind::Other, "psql -p5432 -d db"), None);
    // `%XX` in a connection string is decoded by the client
    assert_eq!(
        found(
            Kind::PostgreSql,
            "psql 'postgresql://alice@localhost/app?pass%77ord=secret'"
        ),
        Some("password=")
    );
    assert_eq!(
        found(
            Kind::PostgreSql,
            "psql postgresql://alice@h/app?sslmode=require%26x"
        ),
        None
    );
    assert_eq!(found(Kind::PostgreSql, "psql -f 100%.sql"), None);
    // a word that only looks like a client is not one; encoded delimiters stay delimiters
    assert_eq!(found(Kind::Other, "psql -h mysql -p5432 -d app"), None);
    assert_eq!(found(Kind::Other, "psql -h sqlcmd -P5432"), None);
    assert_eq!(found(Kind::Other, "env psql -h mysql -p5432 -f $F"), None);
    // the commands of a line: cut at `;` `|` `&&` and ` & `, not at the `&` inside a word
    assert_eq!(
        split_commands("a && b | c ; d (e) f&g & h"),
        ["a ", "", " b ", " c ", " d ", "e", " f&g ", " h"]
    );
    // a separator inside quotes is part of the value
    assert_eq!(
        split_commands("a 'b;c' \"d|e\" f\\;g ; h"),
        ["a 'b;c' \"d|e\" f\\;g ", " h"]
    );
    assert_eq!(
        found(Kind::MySql, "mysql --password=';secret'"),
        Some("--password=")
    );
    assert_eq!(found(Kind::MySql, "mysql -u me '-p;secret'"), Some("-p"));
    // the rules of one client are for its own command only
    assert_eq!(
        found(
            Kind::PostgreSql,
            "psql -p5432 -f $F && mysql --login-path=mylogin"
        ),
        None
    );
    assert_eq!(
        found(Kind::PostgreSql, "mysql --login-path=l ; psql -p5432 -f $F"),
        None
    );
    assert_eq!(found(Kind::MySql, "psql -p5432 | mysql -u me"), None);
    assert_eq!(
        found(
            Kind::PostgreSql,
            "psql -p5432 -f $F && mysql -u me -psecret"
        ),
        Some("-p")
    );
    assert_eq!(
        found(Kind::PostgreSql, "mysql -psecret ; psql -p5432"),
        Some("-p")
    );
    assert_eq!(
        found(
            Kind::PostgreSql,
            "psql postgresql://alice%3Aadmin@localhost/app"
        ),
        None
    );
    assert_eq!(
        found(Kind::Other, "env -i HOME=x mysql -psecret"),
        Some("-p")
    );
    assert_eq!(
        found(Kind::Other, "echo x && sudo -u me mysql -psecret"),
        Some("-p")
    );
    // short options of MySQL in one word
    assert_eq!(
        found(Kind::MySql, "mysql -u alice -vpS3cr3t app"),
        Some("-p")
    );
    assert_eq!(found(Kind::MySql, "mysql -svpS3cr3t"), Some("-p"));
    assert_eq!(found(Kind::MySql, "mysql -vp app"), None);
    assert_eq!(found(Kind::MySql, "mysql -v -p app"), None);
    assert_eq!(found(Kind::MySql, "mysql -uroot app"), None);
    // escaped quotes are letters to a shell: the words are looked at by blanks too
    assert_eq!(
        found(
            Kind::MySql,
            r#"mysql --init-command=\"x -psecret --init-command=\"y"#
        ),
        Some("-p")
    );
    assert_eq!(
        found(Kind::MySql, "\"/opt/My SQL/mysql\" -u me -psecret"),
        Some("-p")
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let again = dir.join("private").join(FILE_NAME);
        assert!(create_example_file(&again).unwrap());
        let mode = std::fs::metadata(&again).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "{mode:o}");
        // a link that is already there is not followed
        let link = dir.join("link.ini");
        std::os::unix::fs::symlink(dir.join("nowhere"), &link).unwrap();
        assert!(!create_example_file(&link).unwrap_or(false));
        assert!(!dir.join("nowhere").exists());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_file_is_beside_the_settings_file() {
    assert_eq!(
        path_beside(Path::new("/c/matsu-editor/matsu.ini")),
        PathBuf::from("/c/matsu-editor/databases.ini")
    );
}
