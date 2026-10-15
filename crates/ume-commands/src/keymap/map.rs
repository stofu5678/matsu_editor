//! The key map: key + modifiers to Sakura function code to command.
//!
//! Sakura: `CKeyBind::GetFuncCode` and `GetFuncCodeAt` (`sakura_core/func/CKeyBind.cpp`) pick
//! the row of the key and the cell of the modifiers; an empty cell falls back to
//! `GetDefFuncCode`. `GetKeyStr` is the reverse lookup that menus use.

use std::collections::{HashMap, HashSet};

use super::key::{Chord, Key, Modifiers};
use super::options::{
    CMD_KEYS, ControlKey, EMACS_KEYS, KeyMapOptions, OPTION_ARROWS, OverlayEntry, Platform,
    STANDARD_KEYS,
};
use super::user::UserRow;
use crate::command::Registration;
use crate::registry::Registry;
use crate::sakura::{FUNCTIONS, Function, FunctionKind, KEY_BINDINGS};

/// What a chord does.
#[derive(Clone, Copy, Debug)]
pub enum Lookup {
    /// Run this command.
    Command(&'static Registration),
    /// Bound to a Sakura function that has no command yet. The UI tells the user that the
    /// function is not implemented. `function` is `None` if the code is not a Sakura function
    /// (a number from a user's file).
    NotImplemented {
        code: u16,
        function: Option<&'static Function>,
    },
    /// Nothing is bound.
    Unbound,
    /// Deliberately not a command: the OS or the text input handles it (macOS's hide and
    /// minimize, Option+character with `option_text_input`).
    PassThrough,
}

/// What the tables say, before the registry is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolved {
    Code(u16),
    Unbound,
    PassThrough,
}

/// The key map: Sakura's default table, the macOS rules on top, and the user's rows.
#[derive(Clone, Debug)]
pub struct KeyMap {
    options: KeyMapOptions,
    /// Function codes of the 8 modifier combinations, per key.
    rows: HashMap<Key, [u16; 8]>,
    /// The keys in Sakura's table order (the mouse rows first), then keys the user added.
    order: Vec<Key>,
    /// Keys whose row the user replaced.
    user_keys: HashSet<Key>,
    /// Chords with the macOS rules; `None` means "leave it to the OS".
    overlay: HashMap<Chord, Option<u16>>,
}

fn code_of(name: &str) -> Option<u16> {
    FUNCTIONS
        .iter()
        .find(|f| f.name == name && f.kind != FunctionKind::Reserved)
        .map(|f| f.code)
}

fn default_row(key: Key) -> Option<[u16; 8]> {
    KEY_BINDINGS
        .iter()
        .find(|b| b.key_code == key.code())
        .map(|b| b.functions)
}

fn function_of(code: u16) -> Option<&'static Function> {
    FUNCTIONS
        .iter()
        .find(|f| f.code == code && f.kind != FunctionKind::Reserved)
}

impl KeyMap {
    /// Sakura's default key map with the given options.
    pub fn new(options: KeyMapOptions) -> Self {
        let mut rows = HashMap::new();
        let mut order = Vec::new();
        for b in KEY_BINDINGS {
            if let Some(key) = Key::from_code(b.key_code) {
                rows.insert(key, b.functions);
                order.push(key);
            }
        }
        let mut overlay = HashMap::new();
        if options.platform == Platform::MacOs {
            let mut add = |entries: &[OverlayEntry]| {
                for &(key, mods, name) in entries {
                    let value = match name {
                        Some(n) => match code_of(n) {
                            Some(code) => Some(code),
                            None => continue,
                        },
                        None => None,
                    };
                    overlay.insert(Chord::new(key, mods), value);
                }
            };
            if options.ctrl_as_cmd {
                if options.standard_keys {
                    add(STANDARD_KEYS);
                }
            } else if options.cmd_keys {
                add(CMD_KEYS);
            }
            if options.option_arrows_by_word {
                add(OPTION_ARROWS);
            }
            if options.ctrl_as_cmd && options.control_key == ControlKey::Emacs {
                add(EMACS_KEYS);
            }
        }
        Self {
            options,
            rows,
            order,
            user_keys: HashSet::new(),
            overlay,
        }
    }

    pub fn options(&self) -> &KeyMapOptions {
        &self.options
    }

    /// Replaces the rows of the keys in `rows`, as Sakura does when it reads `KeyBind[...]`
    /// entries: the whole row of a key is replaced, an empty cell stays empty. The macOS rules
    /// do not apply to the keys of rows that differ from Sakura's default row, so a user's
    /// table is used as written.
    pub fn apply_user_rows(&mut self, rows: &[UserRow]) {
        for row in rows {
            if self.rows.insert(row.key, row.functions).is_none() {
                self.order.push(row.key);
            }
            // Sakura writes the whole table, so a file that was never customized has the
            // default row for every key. Those keys keep the macOS rules.
            if default_row(row.key) == Some(row.functions) {
                self.user_keys.remove(&row.key);
            } else {
                self.user_keys.insert(row.key);
            }
        }
    }

    /// The whole current table, in order, in the form of the user's rows (what Sakura writes to
    /// `sakura.ini`).
    pub fn rows(&self) -> Vec<UserRow> {
        self.order
            .iter()
            .map(|&key| UserRow {
                key,
                functions: self.rows[&key],
            })
            .collect()
    }

    /// The function code (or the reason there is none) for a chord.
    pub fn resolve(&self, chord: Chord) -> Resolved {
        let Chord { key, mods } = chord;
        let logical = match self.options.platform {
            Platform::Other => {
                if mods.meta {
                    return Resolved::Unbound;
                }
                mods
            }
            Platform::MacOs => {
                if self.options.option_text_input
                    && key.is_printable()
                    && mods.alt
                    && !mods.meta
                    && !mods.ctrl
                {
                    return Resolved::PassThrough;
                }
                if !self.user_keys.contains(&key)
                    && let Some(value) = self.overlay.get(&chord)
                {
                    return value.map_or(Resolved::PassThrough, Resolved::Code);
                }
                if self.options.ctrl_as_cmd {
                    // Control has no Sakura meaning; Cmd takes the place of Ctrl.
                    if mods.ctrl {
                        return Resolved::Unbound;
                    }
                    Modifiers {
                        ctrl: mods.meta,
                        meta: false,
                        ..mods
                    }
                } else {
                    if mods.meta {
                        return Resolved::Unbound;
                    }
                    mods
                }
            }
        };
        let index = logical.sakura_index();
        let code = self.rows.get(&key).map_or(0, |row| row[index]);
        if code != 0 {
            return Resolved::Code(code);
        }
        match default_function(key, index) {
            Some(code) => Resolved::Code(code),
            None => Resolved::Unbound,
        }
    }

    /// Looks the chord up in the registry of the program.
    pub fn lookup(&self, chord: Chord) -> Lookup {
        self.lookup_in(Registry::global(), chord)
    }

    /// Looks the chord up in `registry`.
    pub fn lookup_in(&self, registry: &Registry, chord: Chord) -> Lookup {
        match self.resolve(chord) {
            Resolved::Unbound => Lookup::Unbound,
            Resolved::PassThrough => Lookup::PassThrough,
            Resolved::Code(code) => match registry.get_by_code(code) {
                Some(reg) => Lookup::Command(reg),
                None => Lookup::NotImplemented {
                    code,
                    function: function_of(code),
                },
            },
        }
    }

    /// The chord to show for a function in menus and settings, or `None` if no chord runs it.
    /// Sakura's order (`CKeyBind::GetKeyStr`): fewer modifiers first, keys before mouse
    /// actions. The macOS rules are included, and a table cell that they shadow is skipped.
    pub fn shortcut_for(&self, code: u16) -> Option<Chord> {
        self.shortcuts_for(code).into_iter().next()
    }

    /// All chords that run the function, in the order of [`KeyMap::shortcut_for`].
    pub fn shortcuts_for(&self, code: u16) -> Vec<Chord> {
        let mut found: Vec<Chord> = Vec::new();
        let is_mouse = |k: &Key| matches!(k, Key::Mouse(_));
        for mouse in [false, true] {
            for index in 0..8 {
                for key in self.order.iter().filter(|k| is_mouse(k) == mouse) {
                    let in_row = self.rows[key][index] == code
                        || default_function(*key, index) == Some(code);
                    if in_row {
                        found.push(self.physical(*key, index));
                    }
                }
            }
        }
        // The macOS rules: Cmd chords come first (Cmd+G before F3), Control chords last.
        let mut overlays: Vec<Chord> = self
            .overlay
            .iter()
            .filter(|(_, v)| **v == Some(code))
            .map(|(c, _)| *c)
            .collect();
        overlays.sort_by_key(|c| (c.mods.sakura_index(), c.key.code(), c.mods.meta));
        let (control, rest): (Vec<Chord>, Vec<Chord>) =
            overlays.into_iter().partition(|c| c.mods.ctrl);
        let mut found: Vec<Chord> = rest.into_iter().chain(found).chain(control).collect();
        let mut result: Vec<Chord> = Vec::new();
        for chord in found.drain(..) {
            if self.resolve(chord) == Resolved::Code(code) && !result.contains(&chord) {
                result.push(chord);
            }
        }
        result
    }

    /// The chord that a Sakura table cell is pressed with on this OS.
    fn physical(&self, key: Key, index: usize) -> Chord {
        let mut mods = Modifiers::from_sakura_index(index);
        if self.options.platform == Platform::MacOs && self.options.ctrl_as_cmd && mods.ctrl {
            mods.ctrl = false;
            mods.meta = true;
        }
        Chord::new(key, mods)
    }
}

#[cfg(test)]
impl KeyMap {
    pub(super) fn overlay_len(&self) -> usize {
        self.overlay.len()
    }
}

impl Default for KeyMap {
    fn default() -> Self {
        Self::new(KeyMapOptions::default())
    }
}

/// `CKeyBind::GetDefFuncCode`: what an empty cell does. Sakura closes the file with Ctrl+F4
/// (or the window, when the tab bar shows one window per tab) and the window with Alt+F4.
/// The tab bar is not modeled; this is the case without tabs.
fn default_function(key: Key, index: usize) -> Option<u16> {
    use crate::sakura::modifier;
    match (key, index) {
        (Key::Function(4), modifier::CTRL) => code_of("F_FILECLOSE"),
        (Key::Function(4), modifier::ALT) => code_of("F_WINCLOSE"),
        _ => None,
    }
}
