//! Reads Sakura's source files and builds the tables.
//!
//! Every parser here is a small text scan over one known file. They fail loudly
//! (with the file and the token) instead of guessing, so that a change in
//! Sakura's files cannot silently produce a wrong table.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

type Result<T> = std::result::Result<T, String>;

/// Files read from the Sakura clone (repository-relative paths).
pub const HSRC: &str = "sakura_core/Funccode_x.hsrc";
pub const RC_H: &str = "src/main/resources/sakura_rc.h";
pub const RC: &str = "sakura_core/sakura_rc.rc";
pub const FUNCCODE_CPP: &str = "sakura_core/func/Funccode.cpp";
pub const KEYBIND_CPP: &str = "sakura_core/func/CKeyBind.cpp";
pub const KEYBIND_H: &str = "sakura_core/func/CKeyBind.h";
pub const MAIN_MENU_INI: &str = "src/main/resources/MainMenu.ini";

/// What a function code is for. Decided from the code range and the name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Codes 0-10: "none", separator, and other placeholders.
    Reserved,
    /// 29001-29006: items that only exist in the main menu (window list, ...).
    MenuSpecial,
    /// 30000-32767: commands for menus, keys and toolbars.
    Command,
    /// 32768-39999 in `Funccode_x.hsrc`: macro-only commands.
    MacroOnly,
    /// 40000 and above: macro functions that return a value.
    MacroFunction,
    /// 34000 and above in `sakura_rc.h`: string IDs for menu labels.
    MenuLabel,
}

impl Kind {
    pub fn rust_name(self) -> &'static str {
        match self {
            Kind::Reserved => "Reserved",
            Kind::MenuSpecial => "MenuSpecial",
            Kind::Command => "Command",
            Kind::MacroOnly => "MacroOnly",
            Kind::MacroFunction => "MacroFunction",
            Kind::MenuLabel => "MenuLabel",
        }
    }
}

pub struct Function {
    pub code: u16,
    /// Sakura's name, for example `F_FILESAVE`.
    pub name: String,
    /// Our command ID, for example `filesave`.
    pub id: String,
    pub kind: Kind,
    /// Indexes into [`Tables::groups`]; Sakura lists a few functions in two groups.
    pub groups: Vec<usize>,
    pub label: Option<String>,
}

pub struct KeyBinding {
    pub key_code: u16,
    pub key_name: String,
    /// Function codes for none, Shift, Ctrl, Shift+Ctrl, Alt, Shift+Alt, Ctrl+Alt, Shift+Ctrl+Alt.
    pub functions: [u16; 8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuKind {
    Node,
    Leaf,
    Separator,
    Special,
}

impl MenuKind {
    pub fn rust_name(self) -> &'static str {
        match self {
            MenuKind::Node => "Node",
            MenuKind::Leaf => "Leaf",
            MenuKind::Separator => "Separator",
            MenuKind::Special => "Special",
        }
    }
}

pub struct MenuItem {
    pub level: u8,
    pub kind: MenuKind,
    pub code: u16,
    pub access_key: Option<char>,
    pub label: Option<String>,
}

pub struct Tables {
    pub functions: Vec<Function>,
    pub groups: Vec<String>,
    pub keys: Vec<KeyBinding>,
    pub key_parentheses: bool,
    pub menu: Vec<MenuItem>,
}

pub fn read_tables(sakura: &Path) -> Result<Tables> {
    let read = |rel: &str| read_text(&sakura.join(rel));

    // Names -> numbers. Sakura generates `Funccode_define.h` from the .hsrc file
    // (src/main/py/header_make.py); the string IDs of the menu labels are in sakura_rc.h.
    let hsrc = strip_comments(&read(HSRC)?);
    let rc_h = read(RC_H)?;
    let hsrc_entries = parse_hsrc(&hsrc)?;
    let mut names: HashMap<String, i64> = hsrc_entries.iter().cloned().collect();
    let rc_h_entries = parse_defines(&rc_h, "F_")?;
    for (name, value) in &rc_h_entries {
        if names.insert(name.clone(), *value).is_some() {
            return Err(format!("{RC_H}: {name} is also defined in {HSRC}"));
        }
    }

    // Labels: Sakura looks a label up by the numeric code, in the string table of sakura_rc.rc
    // (CFuncLookup::Funccode2Name calls LS(funccode)).
    let strings = parse_string_table(&read(RC)?);
    let mut labels: HashMap<u16, String> = HashMap::new();
    for (name, text) in &strings {
        if let Some(&value) = names.get(name) {
            let code = to_code(value, name)?;
            if let Some(old) = labels.insert(code, text.clone()) {
                return Err(format!(
                    "{RC}: two labels for code {code}: {old:?}, {text:?}"
                ));
            }
        }
    }

    // Groups: Funccode.cpp lists the codes of each group; the names are string-table entries.
    let cpp = strip_comments(&read(FUNCCODE_CPP)?);
    let kind_ids = parse_list(&cpp, "ppszFuncKind")?;
    let list_names = parse_list(&cpp, "ppnFuncListArr")?;
    if kind_ids.len() != list_names.len() {
        return Err(format!(
            "{FUNCCODE_CPP}: group names and group lists differ in number"
        ));
    }
    let mut groups = Vec::new();
    let mut group_of: HashMap<u16, Vec<usize>> = HashMap::new();
    for (index, (kind_id, list_name)) in kind_ids.iter().zip(&list_names).enumerate() {
        let group_name = strings
            .get(kind_id)
            .ok_or_else(|| format!("{RC}: no string {kind_id}"))?;
        groups.push(group_name.clone());
        for item in parse_list(&cpp, list_name)? {
            let value = *names
                .get(&item)
                .ok_or_else(|| format!("{FUNCCODE_CPP}: unknown {item}"))?;
            let code = to_code(value, &item)?;
            // F_DISABLE (0) is the placeholder of the "other" group; it belongs to no group.
            if code == 0 {
                continue;
            }
            group_of.entry(code).or_default().push(index);
        }
    }

    let mut functions = Vec::new();
    let mut ids = HashSet::new();
    let sources = hsrc_entries
        .iter()
        .map(|(n, v)| (n, *v, false))
        .chain(rc_h_entries.iter().map(|(n, v)| (n, *v, true)));
    for (name, value, from_rc_h) in sources {
        if value < 0 {
            continue; // F_INVALID = -1
        }
        if is_range_marker(name) {
            continue;
        }
        let code = to_code(value, name)?;
        let kind = classify(name, code, from_rc_h)?;
        let id = command_id(name);
        // Codes 0 and 2 have several names (placeholders); their IDs differ by name.
        if !ids.insert(id.clone()) {
            return Err(format!("command ID {id:?} (from {name}) is not unique"));
        }
        functions.push(Function {
            code,
            name: name.clone(),
            id,
            kind,
            groups: group_of.get(&code).cloned().unwrap_or_default(),
            label: labels.get(&code).cloned(),
        });
    }

    let keys = parse_key_bindings(sakura, &names, &strings)?;
    let (key_parentheses, menu) = parse_main_menu(&read(MAIN_MENU_INI)?, &functions, &labels)?;
    Ok(Tables {
        functions,
        groups,
        keys,
        key_parentheses,
        menu,
    })
}

// ---- reading files ----------------------------------------------------------------------

/// Reads a text file as UTF-8 or UTF-16LE (sakura_rc.rc), drops the BOM and turns CRLF into LF.
fn read_text(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let (pairs, tail) = rest.as_chunks::<2>();
        if !tail.is_empty() {
            return Err(format!("{}: odd UTF-16 byte count", path.display()));
        }
        let units: Vec<u16> = pairs.iter().map(|c| u16::from_le_bytes(*c)).collect();
        String::from_utf16(&units).map_err(|e| format!("{}: {e}", path.display()))?
    } else {
        String::from_utf8(bytes).map_err(|e| format!("{}: {e}", path.display()))?
    };
    Ok(text.trim_start_matches('\u{FEFF}').replace("\r\n", "\n"))
}

/// Removes `//` and `/* */` comments (keeping line breaks) outside string literals.
fn strip_comments(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        match (chars[i], chars.get(i + 1)) {
            ('/', Some('/')) => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            ('/', Some('*')) => {
                i += 2;
                while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                    if chars[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
                i += 2;
            }
            ('"', _) => {
                out.push('"');
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        out.push('\\');
                        i += 1;
                    }
                    if i < chars.len() {
                        out.push(chars[i]);
                        i += 1;
                    }
                }
                if i < chars.len() {
                    out.push('"');
                    i += 1;
                }
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

// ---- function codes ---------------------------------------------------------------------

/// `NAME = value,` entries of Funccode_x.hsrc (comments already removed). A value is a number
/// (decimal or 0x hex) or an earlier name. Only names that start with `F_` are returned.
fn parse_hsrc(text: &str) -> Result<Vec<(String, i64)>> {
    let mut known: HashMap<String, i64> = HashMap::new();
    let mut out = Vec::new();
    for piece in text.split(',') {
        let piece = piece.trim();
        if piece.is_empty() {
            continue;
        }
        let (name, value) = piece
            .split_once('=')
            .ok_or_else(|| format!("{HSRC}: expected NAME = value, got {piece:?}"))?;
        let (name, value) = (name.trim(), value.trim());
        let number = match parse_number(value) {
            Some(n) => n,
            None => *known
                .get(value)
                .ok_or_else(|| format!("{HSRC}: unknown value {value:?} of {name}"))?,
        };
        known.insert(name.to_string(), number);
        if name.starts_with("F_") {
            out.push((name.to_string(), number));
        }
    }
    Ok(out)
}

/// `#define NAME number` lines whose name starts with `prefix`.
fn parse_defines(text: &str, prefix: &str) -> Result<Vec<(String, i64)>> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        if parts.next() != Some("#define") {
            continue;
        }
        let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        if name.starts_with(prefix) {
            let number = parse_number(value).ok_or_else(|| format!("bad value in {line:?}"))?;
            out.push((name.to_string(), number));
        }
    }
    Ok(out)
}

fn parse_number(text: &str) -> Option<i64> {
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).ok()
    } else {
        text.parse().ok()
    }
}

fn to_code(value: i64, name: &str) -> Result<u16> {
    u16::try_from(value).map_err(|_| format!("{name} = {value} does not fit in 16 bits"))
}

/// Names such as `F_MENU_FIRST` or `F_WHEEL_LAST` mark ranges; they are not functions.
fn is_range_marker(name: &str) -> bool {
    name.ends_with("_FIRST")
        || name.ends_with("_LAST")
        || name.ends_with("_BASE")
        || name == "F_USERMACRO_0"
}

fn classify(name: &str, code: u16, from_rc_h: bool) -> Result<Kind> {
    Ok(match code {
        _ if from_rc_h => Kind::MenuLabel,
        0..=10 => Kind::Reserved,
        29001..=29006 => Kind::MenuSpecial,
        30000..=32767 => Kind::Command,
        32768..=39999 => Kind::MacroOnly,
        40000.. => Kind::MacroFunction,
        _ => return Err(format!("{name} = {code} is in no known range")),
    })
}

/// `F_WordDeleteToStart` -> `word_delete_to_start`, `F_FILESAVE` -> `filesave`.
///
/// Words are split where a lowercase letter or a digit is followed by an uppercase letter.
/// Names that are all uppercase without underscores (`F_FILESAVE`) stay one word, because
/// there is no way to find the word boundary mechanically.
fn command_id(name: &str) -> String {
    let rest = name.strip_prefix("F_").unwrap_or(name);
    let mut out = String::new();
    let mut previous: Option<char> = None;
    for c in rest.chars() {
        if c.is_ascii_uppercase()
            && previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit())
        {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
        previous = Some(c);
    }
    out
}

// ---- Funccode.cpp lists -----------------------------------------------------------------

/// The comma-separated identifiers inside `name[] = { ... }` (comments already removed).
fn parse_list(text: &str, name: &str) -> Result<Vec<String>> {
    let marker = format!("{name}[]");
    let start = text
        .find(&marker)
        .ok_or_else(|| format!("{FUNCCODE_CPP}: no array {name}"))?;
    let open = text[start..]
        .find('{')
        .ok_or_else(|| format!("{name}: no {{"))?
        + start;
    let close = text[open..]
        .find('}')
        .ok_or_else(|| format!("{name}: no }}"))?
        + open;
    Ok(text[open + 1..close]
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect())
}

// ---- sakura_rc.rc string table ----------------------------------------------------------

/// `NAME "text"` lines inside `STRINGTABLE ... BEGIN ... END` blocks. `""` becomes `"`.
/// `&&` (a literal `&` in a Windows menu) is kept as it is.
fn parse_string_table(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut in_table = false;
    let mut after_header = false;
    for line in text.lines() {
        let line = line.trim();
        if line == "STRINGTABLE" {
            after_header = true;
        } else if after_header && line == "BEGIN" {
            in_table = true;
            after_header = false;
        } else if line == "END" {
            in_table = false;
        } else if in_table
            && let Some((name, rest)) = line.split_once(char::is_whitespace)
            && let Some(quoted) = rest
                .trim()
                .strip_prefix('"')
                .and_then(|r| r.strip_suffix('"'))
        {
            out.insert(name.to_string(), quoted.replace("\"\"", "\""));
        }
    }
    out
}

// ---- CKeyBind.cpp -----------------------------------------------------------------------

/// Win32 virtual-key codes used as the first field of `KeyDataInit` (winuser.h).
const VIRTUAL_KEYS: &[(&str, u16)] = &[
    ("VK_BACK", 0x08),
    ("VK_TAB", 0x09),
    ("VK_RETURN", 0x0D),
    ("VK_ESCAPE", 0x1B),
    ("VK_SPACE", 0x20),
    ("VK_PRIOR", 0x21),
    ("VK_NEXT", 0x22),
    ("VK_END", 0x23),
    ("VK_HOME", 0x24),
    ("VK_LEFT", 0x25),
    ("VK_UP", 0x26),
    ("VK_RIGHT", 0x27),
    ("VK_DOWN", 0x28),
    ("VK_INSERT", 0x2D),
    ("VK_DELETE", 0x2E),
    ("VK_APPS", 0x5D),
];

fn parse_key_bindings(
    sakura: &Path,
    names: &HashMap<String, i64>,
    strings: &HashMap<String, String>,
) -> Result<Vec<KeyBinding>> {
    let raw = read_text(&sakura.join(KEYBIND_CPP))?;
    // `#define _SQL_RUN F_...` style aliases used inside the table.
    let mut aliases: HashMap<String, String> = HashMap::new();
    for line in strip_comments(&raw).lines() {
        let mut parts = line.split_whitespace();
        if parts.next() == Some("#define")
            && let (Some(name), Some(value)) = (parts.next(), parts.next())
            && name.starts_with('_')
            && value.starts_with("F_")
        {
            aliases.insert(name.to_string(), value.to_string());
        }
    }
    let vkex: HashMap<String, i64> = parse_defines(&read_text(&sakura.join(KEYBIND_H))?, "VKEX_")?
        .into_iter()
        .collect();

    let text = strip_comments(&raw);
    let start = text
        .find("KeyDataInit[] = {")
        .ok_or_else(|| format!("{KEYBIND_CPP}: no KeyDataInit"))?;
    let body_start = text[start..]
        .find('{')
        .map(|i| i + start + 1)
        .ok_or("no {")?;
    // Rows are the `{ ... }` groups at depth 1; the array ends at the `}` that closes depth 0.
    let mut rows = Vec::new();
    let (mut depth, mut row_start) = (1, 0);
    let mut in_string = false;
    for (offset, c) in text[body_start..].char_indices() {
        match c {
            '"' => in_string = !in_string,
            '{' if !in_string => {
                depth += 1;
                if depth == 2 {
                    row_start = body_start + offset + 1;
                }
            }
            '}' if !in_string => {
                depth -= 1;
                if depth == 1 {
                    rows.push(&text[row_start..body_start + offset]);
                } else if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
    }

    let mut keys = Vec::new();
    for row in rows {
        let (key_token, rest) = row
            .split_once(',')
            .ok_or_else(|| format!("bad key row {row:?}"))?;
        let key_token = key_token.trim();
        let key_code = if let Some(n) = parse_number(key_token) {
            u16::try_from(n).map_err(|_| format!("bad key code {key_token}"))?
        } else if let Some(c) = key_token
            .strip_prefix('\'')
            .and_then(|t| t.strip_suffix('\''))
        {
            let mut chars = c.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) if ch.is_ascii() => ch as u16,
                _ => return Err(format!("bad key character {key_token}")),
            }
        } else if let Some(&n) = vkex.get(key_token) {
            to_code(n, key_token)?
        } else if let Some(&(_, n)) = VIRTUAL_KEYS.iter().find(|(name, _)| *name == key_token) {
            n
        } else if let Some(n) = key_token
            .strip_prefix("VK_F")
            .and_then(|n| n.parse::<u16>().ok())
        {
            0x6F + n // VK_F1 = 0x70
        } else {
            return Err(format!("{KEYBIND_CPP}: unknown key {key_token}"));
        };

        // `{ name }` and `{ f0, ..., f7 }`
        let groups: Vec<&str> = rest
            .split('{')
            .skip(1)
            .map(|g| g.split('}').next().unwrap_or(""))
            .collect();
        let [name_group, function_group] = groups[..] else {
            return Err(format!("{KEYBIND_CPP}: bad key row {row:?}"));
        };
        let name_group = name_group.trim();
        let key_name = if let Some(literal) = name_group
            .strip_prefix("L\"")
            .and_then(|g| g.strip_suffix('"'))
        {
            literal.replace("\\\\", "\\")
        } else if let Some(id) = name_group.strip_prefix("(LPCWSTR)") {
            strings
                .get(id)
                .ok_or_else(|| format!("{RC}: no string {id}"))?
                .clone()
        } else {
            return Err(format!("{KEYBIND_CPP}: bad key name {name_group:?}"));
        };

        let mut functions = [0u16; 8];
        let tokens: Vec<&str> = function_group
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .collect();
        if tokens.len() != 8 {
            return Err(format!(
                "{KEYBIND_CPP}: {key_name} has {} functions",
                tokens.len()
            ));
        }
        for (slot, token) in functions.iter_mut().zip(tokens) {
            let token = aliases.get(token).map_or(token, String::as_str);
            let value = names
                .get(token)
                .ok_or_else(|| format!("{KEYBIND_CPP}: unknown {token}"))?;
            *slot = to_code(*value, token)?;
        }
        keys.push(KeyBinding {
            key_code,
            key_name,
            functions,
        });
    }
    Ok(keys)
}

// ---- MainMenu.ini -----------------------------------------------------------------------

/// The default main menu (`IDR_MENU1`, loaded by CShareData::InitShareData and read by
/// CShareData_IO::IO_MainMenu). First line: show access keys in parentheses (0 or 1). Then one
/// line per item: `level,type,function,access key,name`. Types: 0 node, 1 command, 2 separator,
/// 3 special (EMainMenuType in CommonSetting.h).
fn parse_main_menu(
    text: &str,
    functions: &[Function],
    labels: &HashMap<u16, String>,
) -> Result<(bool, Vec<MenuItem>)> {
    let known: HashSet<u16> = functions.iter().map(|f| f.code).collect();
    let mut lines = text.lines();
    let first = lines.next().ok_or("MainMenu.ini is empty")?.trim();
    let key_parentheses = match first {
        "0" => false,
        "1" => true,
        _ => return Err(format!("{MAIN_MENU_INI}: bad first line {first:?}")),
    };
    let mut menu = Vec::new();
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let fields: Vec<&str> = line.split(',').collect();
        let [level, kind, code, key, name] = fields[..] else {
            return Err(format!("{MAIN_MENU_INI}: expected 5 fields in {line:?}"));
        };
        if !name.is_empty() {
            return Err(format!(
                "{MAIN_MENU_INI}: custom node names are not supported: {line:?}"
            ));
        }
        let kind = match kind {
            "0" => MenuKind::Node,
            "1" => MenuKind::Leaf,
            "2" => MenuKind::Separator,
            "3" => MenuKind::Special,
            _ => return Err(format!("{MAIN_MENU_INI}: bad type in {line:?}")),
        };
        let code: u16 = code
            .parse()
            .map_err(|_| format!("{MAIN_MENU_INI}: bad code in {line:?}"))?;
        if !known.contains(&code) {
            return Err(format!("{MAIN_MENU_INI}: unknown function code {code}"));
        }
        let mut key_chars = key.chars();
        let access_key = match (key_chars.next(), key_chars.next()) {
            (None, _) => None,
            (Some(c), None) => Some(c),
            _ => return Err(format!("{MAIN_MENU_INI}: bad access key in {line:?}")),
        };
        let label = (kind != MenuKind::Separator)
            .then(|| labels.get(&code).cloned())
            .flatten();
        menu.push(MenuItem {
            level: level
                .parse()
                .map_err(|_| format!("{MAIN_MENU_INI}: bad level in {line:?}"))?,
            kind,
            code,
            access_key,
            label,
        });
    }
    Ok((key_parentheses, menu))
}
