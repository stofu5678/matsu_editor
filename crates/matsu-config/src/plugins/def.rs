//! `plugin.def`: the definition file of a plugin (ADR-0026).
//!
//! The file is in the INI format of [`Profile`]: Shift_JIS, or UTF-8 with a BOM
//! (`help/plugin/Text/implementation01.html`). A language file `local\plugin_<language>.def`
//! overrides some texts. [`PluginDef::from_profiles`] reads the sections the way Sakura's
//! `CPlugin::ReadPluginDefCommon`, `ReadPluginDefPlug`, `ReadPluginDefCommand`,
//! `ReadPluginDefOption` and `ReadPluginDefString` do (`sakura_core/plugin/CPlugin.cpp`), and the
//! sections of the two kinds as `CWSHPlugin::ReadPluginDef` and `CDllPlugin::ReadPluginDef` do.
//!
//! Sakura never writes `plugin.def`; neither does this module.

use crate::profile::Profile;

/// The name of the definition file (`PII_FILENAME`).
pub const FILE_NAME: &str = "plugin.def";
/// The folder of the language files, in the folder of the plugin (`PII_L10NDIR`).
pub const L10N_DIR: &str = "local";
/// How many plugs one jack can have, and one more than the most commands (`MAX_PLUG_CMD`).
pub const MAX_PLUG_CMD: usize = 50;
/// One more than the most options a plugin can define (`MAX_PLUG_OPTION`).
pub const MAX_PLUG_OPTION: usize = 100;
/// One more than the most strings (`MAX_PLUG_STRING`).
pub const MAX_PLUG_STRING: usize = 100;

/// The jacks: the places where a plugin can be called (`CJackManager`'s `jackNames`). `Install`,
/// `Uninstall`, `AppStart` and `AppEnd` are commented out in Sakura.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Jack {
    Command,
    EditorStart,
    EditorEnd,
    DocumentOpen,
    DocumentClose,
    DocumentBeforeSave,
    DocumentAfterSave,
    Outline,
    SmartIndent,
    Complement,
    ComplementGlobal,
}

impl Jack {
    /// Every jack, in Sakura's order.
    pub const ALL: [Jack; 11] = [
        Jack::Command,
        Jack::EditorStart,
        Jack::EditorEnd,
        Jack::DocumentOpen,
        Jack::DocumentClose,
        Jack::DocumentBeforeSave,
        Jack::DocumentAfterSave,
        Jack::Outline,
        Jack::SmartIndent,
        Jack::Complement,
        Jack::ComplementGlobal,
    ];

    /// The name in `[Plug]`.
    pub fn name(self) -> &'static str {
        match self {
            Jack::Command => "Command",
            Jack::EditorStart => "EditorStart",
            Jack::EditorEnd => "EditorEnd",
            Jack::DocumentOpen => "DocumentOpen",
            Jack::DocumentClose => "DocumentClose",
            Jack::DocumentBeforeSave => "DocumentBeforeSave",
            Jack::DocumentAfterSave => "DocumentAfterSave",
            Jack::Outline => "Outline",
            Jack::SmartIndent => "SmartIndent",
            Jack::Complement => "Complement",
            Jack::ComplementGlobal => "ComplementGlobal",
        }
    }
}

/// The `Type` of `[Plugin]` (`CPluginManager::LoadPlugin`, compared without regard to case).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginKind {
    /// `wsh`: scripts. MATSU runs them by their extension (ADR-0026, chapter 1).
    Wsh,
    /// `script`: MATSU's name for the same thing (ADR-0026, chapter 1).
    Script,
    /// `dll`: Sakura's DLL plugins. MATSU does not load them.
    Dll,
    /// Anything else; Sakura does not load such a plugin either (`LoadPlugin` returns `nullptr`).
    Other(String),
}

/// What of a plugin MATSU cannot run (ADR-0026, chapter 1), for the list of the settings page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// `Type=dll`.
    Dll,
    /// A handler is a VBScript file (ADR-0025 does not add VBScript).
    VbScript,
    /// A `Type` that is none of `wsh`, `script` and `dll`.
    UnknownType(String),
}

/// A plug: a function of the plugin tied to a jack (`[Plug]`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plug {
    pub jack: Jack,
    /// 0 for the key `Outline`, n for `Outline[n]`.
    pub index: usize,
    /// The script file (relative to the folder of the plugin) or the name of the DLL function.
    pub handler: String,
    /// The name shown to the user; the handler when the file has no `.Label`.
    pub label: String,
}

/// A command of the plugin (`[Command]`, `C[n]`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandDef {
    /// From 1.
    pub index: usize,
    pub handler: String,
    pub label: String,
    /// The icon file (16x16 BMP), or empty.
    pub icon: String,
}

/// The type of an option (`O[n].Type`). Sakura compares it in lower case; a type it does not
/// know is a string (`CDlgPluginOption`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptionType {
    Str,
    Dir,
    Bool,
    Int,
    Sel,
}

impl OptionType {
    fn parse(text: &str) -> Self {
        match text.to_lowercase().as_str() {
            "bool" => OptionType::Bool,
            "int" => OptionType::Int,
            "sel" => OptionType::Sel,
            "dir" => OptionType::Dir,
            _ => OptionType::Str,
        }
    }
}

/// An option of the plugin (`[Option]`): where its value is kept in `plugins\<folder>.ini`, and
/// how the settings dialog edits it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionDef {
    pub index: usize,
    pub label: String,
    pub section: String,
    pub key: String,
    pub kind: OptionType,
    /// The raw `O[n].Select`: items split by `|`, each `shown:value` or just the value.
    pub select: String,
    pub default: String,
}

impl OptionDef {
    /// The items of `Select`: the shown text and the value (`CDlgPluginOption::SepSelect`: with no
    /// `:` the text is the value too).
    pub fn selects(&self) -> Vec<(String, String)> {
        self.select
            .split('|')
            .filter(|item| !item.is_empty())
            .map(|item| match item.split_once(':') {
                Some((shown, value)) => (shown.to_owned(), value.to_owned()),
                None => (item.to_owned(), item.to_owned()),
            })
            .collect()
    }
}

/// A plugin definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub url: String,
    pub kind: PluginKind,
    /// `[Wsh] UseCache` (default 0, `CWSHPlugin.h`).
    pub use_cache: bool,
    /// `[DLL] Name`.
    pub dll_name: String,
    pub plugs: Vec<Plug>,
    pub commands: Vec<CommandDef>,
    pub options: Vec<OptionDef>,
    /// `S[1]` and on; the first is a dummy, as in Sakura (`m_aStrings`).
    pub strings: Vec<String>,
}

/// The value of `key` in the language file if it has the key, else in the main file
/// (`IOProfileData` leaves the value alone when the key is missing).
fn text(main: &Profile, l10n: Option<&Profile>, section: &str, key: &str) -> Option<String> {
    l10n.and_then(|p| p.get(section, key))
        .or_else(|| main.get(section, key))
        .map(str::to_owned)
}

impl PluginDef {
    /// Reads a definition from the bytes of `plugin.def` and, if there is one, of the language file.
    pub fn from_bytes(def: &[u8], l10n: Option<&[u8]>) -> Self {
        let main = Profile::from_bytes(def);
        let l10n = l10n.map(Profile::from_bytes);
        Self::from_profiles(&main, l10n.as_ref())
    }

    /// Reads a definition (`CPlugin::ReadPluginDef*`).
    pub fn from_profiles(main: &Profile, l10n: Option<&Profile>) -> Self {
        let get = |section: &str, key: &str| main.get(section, key).map(str::to_owned);
        let kind = match get("Plugin", "Type")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "wsh" => PluginKind::Wsh,
            "script" => PluginKind::Script,
            "dll" => PluginKind::Dll,
            other => PluginKind::Other(other.to_owned()),
        };
        // `ReadPluginDefCommon`: the language file replaces the name, description and URL.
        let common = |key: &str| text(main, l10n, "Plugin", key).unwrap_or_default();
        PluginDef {
            id: get("Plugin", "Id").unwrap_or_default(),
            name: common("Name"),
            description: common("Description"),
            author: get("Plugin", "Author").unwrap_or_default(),
            version: get("Plugin", "Version").unwrap_or_default(),
            url: common("Url"),
            kind,
            use_cache: main.get_bool("Wsh", "UseCache").unwrap_or(false),
            dll_name: get("DLL", "Name").unwrap_or_default(),
            plugs: read_plugs(main, l10n),
            commands: read_commands(main, l10n),
            options: read_options(main, l10n),
            strings: read_strings(main, l10n),
        }
    }

    /// The number of commands counted the way `InstallPlugin` does: up to the first `C[n]` whose
    /// value is empty or missing. (`ReadPluginDefCommand` stops at a missing key only.)
    pub fn install_command_count(main: &Profile) -> usize {
        (1..MAX_PLUG_CMD)
            .take_while(|i| {
                main.get("Command", &format!("C[{i}]"))
                    .is_some_and(|v| !v.is_empty())
            })
            .count()
    }

    /// Why MATSU cannot run the plugin, if it cannot (ADR-0026, chapter 1).
    pub fn unsupported(&self) -> Option<Unsupported> {
        match &self.kind {
            PluginKind::Dll => return Some(Unsupported::Dll),
            PluginKind::Other(name) => return Some(Unsupported::UnknownType(name.clone())),
            PluginKind::Wsh | PluginKind::Script => {}
        }
        let mut handlers = self
            .plugs
            .iter()
            .map(|p| p.handler.as_str())
            .chain(self.commands.iter().map(|c| c.handler.as_str()));
        handlers
            .any(is_vbscript)
            .then_some(Unsupported::VbScript)
    }

    /// The commands the plugin has (`GetCommandCount`).
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }
}

/// A handler that is a VBScript file: `.vbs`, or `.vbe` (encoded VBScript).
fn is_vbscript(handler: &str) -> bool {
    let ext = handler.rsplit_once('.').map(|(_, e)| e.to_lowercase());
    matches!(ext.as_deref(), Some("vbs" | "vbe"))
}

/// `ReadPluginDefPlug`: for each jack, `Jack`, `Jack[1]`, ... until a key is missing.
fn read_plugs(main: &Profile, l10n: Option<&Profile>) -> Vec<Plug> {
    let mut plugs = Vec::new();
    for jack in Jack::ALL {
        for index in 0..MAX_PLUG_CMD {
            let suffix = if index == 0 {
                String::new()
            } else {
                format!("[{index}]")
            };
            let key = format!("{}{suffix}", jack.name());
            let Some(handler) = main.get("Plug", &key) else {
                break;
            };
            let label = text(main, l10n, "Plug", &format!("{key}.Label")).unwrap_or_default();
            plugs.push(Plug {
                jack,
                index,
                handler: handler.to_owned(),
                label: if label.is_empty() {
                    handler.to_owned()
                } else {
                    label
                },
            });
        }
    }
    plugs
}

/// `ReadPluginDefCommand`: `C[1]` to `C[49]`, until a key is missing.
fn read_commands(main: &Profile, l10n: Option<&Profile>) -> Vec<CommandDef> {
    let mut commands = Vec::new();
    for index in 1..MAX_PLUG_CMD {
        let Some(handler) = main.get("Command", &format!("C[{index}]")) else {
            break;
        };
        let field = |name: &str| {
            text(main, l10n, "Command", &format!("C[{index}].{name}")).unwrap_or_default()
        };
        let label = field("Label");
        commands.push(CommandDef {
            index,
            handler: handler.to_owned(),
            label: if label.is_empty() {
                handler.to_owned()
            } else {
                label
            },
            icon: field("Icon"),
        });
    }
    commands
}

/// `ReadPluginDefOption`: `O[1]` to `O[99]`. A missing `O[n].Key` skips that number only; an
/// option whose section is empty (a section is inherited from the option before it) or whose key
/// is empty is ignored.
fn read_options(main: &Profile, l10n: Option<&Profile>) -> Vec<OptionDef> {
    let mut options = Vec::new();
    let mut section = String::new();
    for index in 1..MAX_PLUG_OPTION {
        // Key, Section, Type and Default are read from the plugin's own file only, as in Sakura;
        // the language file replaces the label and the choices.
        let own = |name: &str| {
            main.get("Option", &format!("O[{index}].{name}"))
                .map(str::to_owned)
        };
        let Some(key) = own("Key") else {
            continue;
        };
        if let Some(s) = own("Section").filter(|s| !s.is_empty()) {
            section = s;
        }
        if section.is_empty() || key.is_empty() {
            continue;
        }
        let translated = |name: &str| {
            text(main, l10n, "Option", &format!("O[{index}].{name}")).unwrap_or_default()
        };
        let label = translated("Label");
        options.push(OptionDef {
            index,
            label: if label.is_empty() { key.clone() } else { label },
            section: section.clone(),
            key,
            kind: OptionType::parse(&own("Type").unwrap_or_default()),
            select: translated("Select"),
            default: own("Default").unwrap_or_default(),
        });
    }
    options
}

/// `ReadPluginDefString`: `S[1]` to `S[99]`; one that is missing is an empty string.
fn read_strings(main: &Profile, l10n: Option<&Profile>) -> Vec<String> {
    let mut strings = vec![String::new()];
    for index in 1..MAX_PLUG_STRING {
        strings.push(text(main, l10n, "String", &format!("S[{index}]")).unwrap_or_default());
    }
    strings
}
