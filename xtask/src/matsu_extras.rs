//! Functions that only MATSU Editor has. They are added to the tables that come from Sakura's
//! source, so that the generated tables are complete and `committed_tables_match_sakura` still
//! checks them.
//!
//! Codes: the highest code of Sakura's commands in `Funccode_x.hsrc` is 32018 (the command
//! range is 30000-32767). MATSU-only commands use [`FIRST_CODE`] and up, far from Sakura's, so
//! that a Sakura function added later does not collide with them. Names start with `F_` like
//! Sakura's, and the command ID is the name without it (`copy_lf`).

use crate::parse::{Function, Kind, MenuItem, MenuKind, Tables};

/// The first code reserved for MATSU-only functions (up to 32767, the end of the command range).
pub const FIRST_CODE: u16 = 32700;

/// One MATSU-only command.
pub struct Extra {
    pub code: u16,
    pub name: &'static str,
    pub id: &'static str,
    /// The index into the function groups (5 is クリップボード系, 12 is ウィンドウ系).
    pub group: usize,
    pub label: &'static str,
    /// The function after which this one is placed in the function table, the main menu and
    /// the macro table.
    pub after: &'static str,
    /// The access key in the main menu.
    pub access_key: char,
    /// The name a macro calls.
    pub macro_name: &'static str,
    /// A separator line in the main menu before this item (the start of a group of its own).
    pub separator_before: bool,
    /// A separator line in the main menu after this item (the end of that group).
    pub separator_after: bool,
}

/// In order: the second one is placed after the first.
pub const EXTRAS: &[Extra] = &[
    Extra {
        code: 32700,
        name: "F_COPY_LF",
        id: "copy_lf",
        group: 5,
        label: "LF改行でコピー",
        after: "F_COPY_CRLF",
        access_key: 'N',
        macro_name: "CopyLF",
        separator_before: false,
        separator_after: false,
    },
    Extra {
        code: 32701,
        name: "F_COPY_CR",
        id: "copy_cr",
        group: 5,
        label: "CR改行でコピー",
        after: "F_COPY_LF",
        access_key: 'M',
        macro_name: "CopyCR",
        separator_before: false,
        separator_after: false,
    },
    // The preview tab (ADR-0024): Sakura has nothing like it.
    Extra {
        code: 32702,
        name: "F_PREVIEW_TAB",
        id: "preview_tab",
        group: 12,
        label: "プレビューを別のタブで開く",
        after: "F_WIN_OUTPUT",
        access_key: 'G',
        macro_name: "PreviewTab",
        separator_before: false,
        separator_after: false,
    },
    // Chooses a connection of `databases.ini` and runs the document with it (ADR-0023): Sakura has
    // only the default-connection run (F10).
    Extra {
        code: 32703,
        name: "F_PLSQL_COMPILE_ON_DATABASE_CHOICE",
        id: "plsql_compile_on_database_choice",
        group: 0,
        label: "接続を選んでデータベースで実行...",
        after: "F_PLSQL_COMPILE_ON_SQLPLUS",
        access_key: 'C',
        macro_name: "ExecDatabaseChoice",
        separator_before: false,
        separator_after: false,
    },
    // Encoding and decoding of the selection (group 7, 変換): a group of its own in the 変換 menu,
    // after 「空白→TAB」 and before the 「文字コード変換」 submenu. Sakura has none of these (its
    // 「Base64デコードして保存」 writes a file and stays as it is). The macro names avoid Sakura's
    // `Base64Decode`.
    Extra {
        code: 32704,
        name: "F_URL_ENCODE",
        id: "url_encode",
        group: 7,
        label: "URLエンコード",
        after: "F_SPACETOTAB",
        access_key: 'E',
        macro_name: "EncodeUrl",
        separator_before: true,
        separator_after: false,
    },
    Extra {
        code: 32705,
        name: "F_URL_DECODE",
        id: "url_decode",
        group: 7,
        label: "URLデコード",
        after: "F_URL_ENCODE",
        access_key: 'D',
        macro_name: "DecodeUrl",
        separator_before: false,
        separator_after: false,
    },
    Extra {
        code: 32706,
        name: "F_HTML_ESCAPE",
        id: "html_escape",
        group: 7,
        label: "HTMLエスケープ",
        after: "F_URL_DECODE",
        access_key: 'P',
        macro_name: "EscapeHtml",
        separator_before: false,
        separator_after: false,
    },
    Extra {
        code: 32707,
        name: "F_HTML_UNESCAPE",
        id: "html_unescape",
        group: 7,
        label: "HTMLエスケープを戻す",
        after: "F_HTML_ESCAPE",
        access_key: 'R',
        macro_name: "UnescapeHtml",
        separator_before: false,
        separator_after: false,
    },
    Extra {
        code: 32708,
        name: "F_BASE64_ENCODE",
        id: "base64_encode",
        group: 7,
        label: "Base64エンコード",
        after: "F_HTML_UNESCAPE",
        access_key: 'B',
        macro_name: "EncodeBase64",
        separator_before: false,
        separator_after: false,
    },
    Extra {
        code: 32709,
        name: "F_BASE64_DECODE_TEXT",
        id: "base64_decode_text",
        group: 7,
        label: "Base64デコード",
        after: "F_BASE64_ENCODE",
        access_key: 'Y',
        macro_name: "DecodeBase64",
        separator_before: false,
        separator_after: true,
    },
    // The shell tab (ADR-0028): a terminal in a tab, next to the preview in the window menu.
    Extra {
        code: 32710,
        name: "F_SHELL_TAB",
        id: "shell_tab",
        group: 12,
        label: "シェルのタブを開く",
        after: "F_PREVIEW_TAB",
        access_key: 'L',
        macro_name: "ShellTab",
        separator_before: false,
        separator_after: false,
    },
];

/// Labels MATSU Editor words differently from Sakura (ADR-0018 type F): `(function name, label)`.
/// The function name, the code and the macro name stay Sakura's, so keys, macros and the menus
/// saved in `matsu.ini` (which hold codes) keep working. The database functions are not Oracle's
/// SQL*Plus here (ADR-0023): they run the client of any connection in `databases.ini`.
pub const RELABELS: &[(&str, &str)] = &[
    ("F_PLSQL_COMPILE_ON_SQLPLUS", "データベースで実行"),
    ("F_ACTIVATE_SQLPLUS", "データベースの出力を表示"),
];

/// Adds the functions and the main menu items, and words the labels in [`RELABELS`].
pub fn apply(tables: &mut Tables) -> Result<(), String> {
    for (name, label) in RELABELS {
        let function = tables
            .functions
            .iter_mut()
            .find(|f| f.name == *name)
            .ok_or_else(|| format!("{name} is not a function"))?;
        function.label = Some((*label).to_string());
        let code = function.code;
        for item in tables
            .menu
            .iter_mut()
            .filter(|i| i.kind == MenuKind::Leaf && i.code == code)
        {
            item.label = Some((*label).to_string());
        }
    }
    for extra in EXTRAS {
        if extra.code < FIRST_CODE {
            return Err(format!(
                "{}: code {} is below {FIRST_CODE}",
                extra.name, extra.code
            ));
        }
        if tables.functions.iter().any(|f| f.code == extra.code) {
            return Err(format!("code {} is already used", extra.code));
        }
        let position = tables
            .functions
            .iter()
            .position(|f| f.name == extra.after)
            .ok_or_else(|| format!("{} is not a function", extra.after))?;
        let after_code = tables.functions[position].code;
        tables.functions.insert(
            position + 1,
            Function {
                code: extra.code,
                name: extra.name.to_string(),
                id: extra.id.to_string(),
                kind: Kind::Command,
                groups: vec![extra.group],
                label: Some(extra.label.to_string()),
            },
        );
        let menu = tables
            .menu
            .iter()
            .position(|i| i.kind == MenuKind::Leaf && i.code == after_code)
            .ok_or_else(|| format!("{} is not in the main menu", extra.after))?;
        let level = tables.menu[menu].level;
        let separator = || MenuItem {
            level,
            kind: MenuKind::Separator,
            code: 1,
            access_key: None,
            label: None,
        };
        let mut items = Vec::new();
        if extra.separator_before {
            items.push(separator());
        }
        items.push(MenuItem {
            level,
            kind: MenuKind::Leaf,
            code: extra.code,
            access_key: Some(extra.access_key),
            label: Some(extra.label.to_string()),
        });
        if extra.separator_after {
            items.push(separator());
        }
        tables.menu.splice(menu + 1..menu + 1, items);
    }
    add_oneliner_functions(tables)?;
    add_oneliner_menu(tables)
}

/// The one-liners (ADR-0029): `F_ONELINER_0` to `F_ONELINER_49` run one-liner n of `[OneLiner]`, in a row
/// from [`ONELINER_FIRST_CODE`] like Sakura's `F_USERMACRO_0 + n`; then the special item that stands for the
/// registered ones in a menu, the input dialog and the settings. They are in no group of the command list
/// (the key, toolbar and menu pages give them a kind of their own). The macro table has a row for each
/// ([`oneliner_macros`]) and the main menu a submenu ([`add_oneliner_menu`]). The codes are the ones of
/// `matsu_config::settings::oneliner`.
pub const ONELINER_FIRST_CODE: u16 = 32711;
const ONELINER_COUNT: u16 = 50;

/// The functions of the one-liners, in the order of the function table.
fn oneliner_functions() -> Vec<Function> {
    let mut out: Vec<Function> = (0..ONELINER_COUNT)
        .map(|n| Function {
            code: ONELINER_FIRST_CODE + n,
            name: format!("F_ONELINER_{n}"),
            id: format!("oneliner_{n}"),
            kind: Kind::Command,
            groups: Vec::new(),
            label: Some(format!("ワンライナー {n}")),
        })
        .collect();
    let next = ONELINER_FIRST_CODE + ONELINER_COUNT;
    for (offset, name, id, kind, label) in [
        (
            0,
            "F_ONELINER_LIST",
            "oneliner_list",
            Kind::MenuSpecial,
            "登録済みワンライナーリスト",
        ),
        (
            1,
            "F_ONELINER_DIALOG",
            "oneliner_dialog",
            Kind::Command,
            "ワンライナーを入力して実行...",
        ),
        (
            2,
            "F_ONELINER_SETTINGS",
            "oneliner_settings",
            Kind::Command,
            "ワンライナーの設定...",
        ),
        (
            3,
            "F_AI_LOG_SHOW",
            "ai_log_show",
            Kind::Command,
            "AI ログを表示",
        ),
    ] {
        out.push(Function {
            code: next + offset,
            name: name.to_string(),
            id: id.to_string(),
            kind,
            groups: Vec::new(),
            label: Some(label.to_string()),
        });
    }
    out
}

/// Adds the functions of the one-liners after `F_SHELL_TAB`.
fn add_oneliner_functions(tables: &mut Tables) -> Result<(), String> {
    let functions = oneliner_functions();
    for f in &functions {
        if tables.functions.iter().any(|g| g.code == f.code) {
            return Err(format!("code {} is already used", f.code));
        }
    }
    let position = tables
        .functions
        .iter()
        .position(|f| f.name == "F_SHELL_TAB")
        .ok_or("F_SHELL_TAB is not a function")?;
    tables
        .functions
        .splice(position + 1..position + 1, functions);
    Ok(())
}

/// The macro names of the one-liners, in the order of the macro table: `OneLiner0` to `OneLiner49` call
/// `F_ONELINER_0` to `F_ONELINER_49` (a key macro records them), and `OneLinerDialog` opens the input
/// dialog (not recorded, like `ExecCommandDialog`). `(macro name, function name, recorded by key macros)`.
/// A macro can run a registered one-liner but there is no macro that adds or changes one (ADR-0029).
pub fn oneliner_macros() -> Vec<(String, String, bool)> {
    let mut rows: Vec<(String, String, bool)> = (0..ONELINER_COUNT)
        .map(|n| (format!("OneLiner{n}"), format!("F_ONELINER_{n}"), true))
        .collect();
    rows.push((
        "OneLinerDialog".to_string(),
        "F_ONELINER_DIALOG".to_string(),
        false,
    ));
    rows
}

/// The submenu 「ワンライナー」 of the main menu (ADR-0029), right after 「外部コマンド実行」 in the tool menu:
/// the registered one-liners, a separator, the input dialog and the settings. The node is a row of
/// `F_NODE` with its own name, like a node the user makes.
fn add_oneliner_menu(tables: &mut Tables) -> Result<(), String> {
    let after = tables
        .functions
        .iter()
        .find(|f| f.name == "F_EXECMD_DIALOG")
        .map(|f| f.code)
        .ok_or("F_EXECMD_DIALOG is not a function")?;
    let at = tables
        .menu
        .iter()
        .position(|i| i.kind == MenuKind::Leaf && i.code == after)
        .ok_or("F_EXECMD_DIALOG is not in the main menu")?;
    let level = tables.menu[at].level;
    let list = ONELINER_FIRST_CODE + ONELINER_COUNT;
    let item = |level, kind, code, access_key, label: Option<&str>| MenuItem {
        level,
        kind,
        code,
        access_key,
        label: label.map(str::to_string),
    };
    let items = vec![
        item(level, MenuKind::Node, 0, Some('O'), Some("ワンライナー")),
        item(
            level + 1,
            MenuKind::Special,
            list,
            None,
            Some("登録済みワンライナーリスト"),
        ),
        item(level + 1, MenuKind::Separator, 1, None, None),
        item(
            level + 1,
            MenuKind::Leaf,
            list + 1,
            Some('I'),
            Some("ワンライナーを入力して実行..."),
        ),
        item(
            level + 1,
            MenuKind::Leaf,
            list + 2,
            Some('S'),
            Some("ワンライナーの設定..."),
        ),
    ];
    tables.menu.splice(at + 1..at + 1, items);
    Ok(())
}
