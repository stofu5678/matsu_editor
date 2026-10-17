//! What a page of the common settings is made of: its controls, and the values they hold.
//!
//! A page is a list of [`Item`]s. A [`Control`] knows its label, the key of `[Common]` that holds
//! its value, the default (Sakura's, `CShareData::InitShareData`), the limits, and whether UME
//! Editor can use it yet. The dialog draws the items, [`Values`] reads and writes the keys, so a
//! page module only lists its controls and says what a change does to the windows
//! (`Page::apply`). The key names are Sakura's and must stay (ADR-0010).
//!
//! Sakura's rules that this follows (`CPropComGeneral.cpp`, `CPropComWin.cpp`, ...):
//!
//! * A number is clamped into its range when OK is pressed (`GetData`), not refused; text that is
//!   not a number is 0 (`GetDlgItemInt`).
//! * A check box that is the opposite of its key (「ウィンドウをまとめてグループ化する」 is
//!   `!bDispTabWndMultiWin`) says so with [`Control::inverted`].

use std::collections::HashMap;

use ume_config::{Profile, ProfileError};

use super::dialog::CommonSettingsDialog;

/// The section of the settings file that holds the settings of the dialog.
pub(crate) const SECTION: &str = "Common";

/// A value a control holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    Bool(bool),
    Int(i32),
    Text(String),
}

/// The default of a control: Sakura's value when the file has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Def {
    Bool(bool),
    Int(i32),
    Text(&'static str),
}

impl Def {
    fn value(self) -> Value {
        match self {
            Def::Bool(b) => Value::Bool(b),
            Def::Int(i) => Value::Int(i),
            Def::Text(t) => Value::Text(t.to_owned()),
        }
    }
}

/// What a button does. It gets the dialog and its window.
pub(crate) type ButtonAction =
    fn(&mut CommonSettingsDialog, &mut gpui::Window, &mut gpui::Context<CommonSettingsDialog>);

/// Whether a control can be used now, from the values of the page (`EnableTabPropInput` and the
/// like). A control that cannot be used is greyed out.
pub(crate) type Enabled = fn(&Values) -> bool;

/// `Enabled` for a control that does not depend on another.
pub(crate) fn always(_: &Values) -> bool {
    true
}

/// The sort of a control.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    /// A check box.
    Check,
    /// One radio button of a group. The group is the control's `id`; this one is `value`.
    Radio { value: i32 },
    /// A drop-down list (the choices are buttons here): the label and the number of each.
    Choice {
        options: &'static [(&'static str, i32)],
    },
    /// A number with its range (an edit box with a spin button in Sakura).
    Number { min: i32, max: i32 },
    /// An edit box.
    Text,
    /// A push button.
    Button { run: ButtonAction },
}

/// A control of a page.
#[derive(Clone, Copy)]
pub(crate) struct Control {
    /// Names the control; unique in the whole dialog. For a radio button, the name of the group.
    pub id: &'static str,
    /// Sakura's label, with `&` before the access key (`(&F)`); `&&` is an `&`.
    pub label: &'static str,
    /// The key in `[Common]`; empty for a control that has none (buttons, the hot key).
    pub key: &'static str,
    pub default: Def,
    /// Whether UME Editor does what the control says. A control that is not live is shown with
    /// the value of the file, greyed out, and never written.
    pub live: bool,
    pub enabled: Enabled,
    pub kind: Kind,
    /// The value is the opposite of the key's (`bDispTabWndMultiWin`).
    pub invert: bool,
}

impl Control {
    const fn new(
        id: &'static str,
        label: &'static str,
        key: &'static str,
        default: Def,
        kind: Kind,
    ) -> Self {
        Self {
            id,
            label,
            key,
            default,
            live: true,
            enabled: always,
            kind,
            invert: false,
        }
    }

    /// A check box.
    pub(crate) const fn check(
        id: &'static str,
        label: &'static str,
        key: &'static str,
        default: bool,
    ) -> Self {
        Self::new(id, label, key, Def::Bool(default), Kind::Check)
    }

    /// One radio button of the group `id`, which holds `key`.
    pub(crate) const fn radio(
        id: &'static str,
        label: &'static str,
        key: &'static str,
        value: i32,
        default: i32,
    ) -> Self {
        Self::new(id, label, key, Def::Int(default), Kind::Radio { value })
    }

    /// A drop-down list.
    pub(crate) const fn choice(
        id: &'static str,
        label: &'static str,
        key: &'static str,
        default: i32,
        options: &'static [(&'static str, i32)],
    ) -> Self {
        Self::new(id, label, key, Def::Int(default), Kind::Choice { options })
    }

    /// A number in `min..=max`.
    pub(crate) const fn number(
        id: &'static str,
        label: &'static str,
        key: &'static str,
        default: i32,
        min: i32,
        max: i32,
    ) -> Self {
        Self::new(id, label, key, Def::Int(default), Kind::Number { min, max })
    }

    /// An edit box.
    pub(crate) const fn text(
        id: &'static str,
        label: &'static str,
        key: &'static str,
        default: &'static str,
    ) -> Self {
        Self::new(id, label, key, Def::Text(default), Kind::Text)
    }

    /// A push button.
    pub(crate) const fn button(id: &'static str, label: &'static str, run: ButtonAction) -> Self {
        Self::new(id, label, "", Def::Bool(false), Kind::Button { run })
    }

    /// UME Editor does not do what this control says yet (or the OS has no such thing): it is
    /// shown greyed out with the note 「未対応」.
    pub(crate) const fn unavailable(mut self) -> Self {
        self.live = false;
        self
    }

    /// The control can be used only while `enabled` says so.
    pub(crate) const fn when(mut self, enabled: Enabled) -> Self {
        self.enabled = enabled;
        self
    }

    /// The check box is the opposite of its key.
    pub(crate) const fn inverted(mut self) -> Self {
        self.invert = true;
        self
    }

    /// The name of the control as an element id: a radio button is told from the others of its
    /// group by its value.
    pub(crate) fn element_id(&self) -> (&'static str, i32) {
        match self.kind {
            Kind::Radio { value } => (self.id, value),
            _ => (self.id, 0),
        }
    }
}

/// A part of a page.
pub(crate) enum Item {
    /// A control.
    Control(Control),
    /// A group box: a title round some items.
    Group(&'static str, Vec<Item>),
    /// Items side by side, each column on its own.
    Columns(Vec<Vec<Item>>),
    /// Items in a row.
    Row(Vec<Item>),
    /// A text that does not change (`LTEXT`).
    Label(&'static str),
}

impl From<Control> for Item {
    fn from(control: Control) -> Self {
        Item::Control(control)
    }
}

/// Every control of the items, in the order they are drawn.
pub(crate) fn controls(items: &[Item]) -> Vec<Control> {
    fn walk(items: &[Item], out: &mut Vec<Control>) {
        for item in items {
            match item {
                Item::Control(c) => out.push(*c),
                Item::Group(_, inner) | Item::Row(inner) => walk(inner, out),
                Item::Columns(columns) => columns.iter().for_each(|c| walk(c, out)),
                Item::Label(_) => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(items, &mut out);
    out
}

/// The label of a control as it is drawn, and its access key: `フリーカーソル(&F)` is
/// `フリーカーソル(F)` with the key F; `&URL` is `URL` with U; `&&` is `&`. The key is lowercased.
pub(crate) fn parse_label(label: &str) -> (String, Option<char>) {
    let mut text = String::with_capacity(label.len());
    let mut key = None;
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            text.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some('&') => {
                chars.next();
                text.push('&');
            }
            Some(next) => {
                if key.is_none() {
                    key = Some(next.to_lowercase().next().unwrap_or(next));
                }
            }
            None => text.push('&'),
        }
    }
    (text, key)
}

/// The numbers and texts of the controls by id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Values {
    map: HashMap<&'static str, Value>,
}

impl Values {
    /// The values of `controls` as the file has them; what the file lacks is the default.
    pub(crate) fn load(controls: &[Control], profile: &Profile) -> Self {
        let mut map = HashMap::new();
        for control in controls {
            if matches!(control.kind, Kind::Button { .. }) || map.contains_key(control.id) {
                continue;
            }
            let fallback = control.default.value();
            let value = if control.key.is_empty() {
                fallback
            } else {
                match control.kind {
                    Kind::Check => {
                        let default = matches!(fallback, Value::Bool(true));
                        // `invert`: the default is the control's, the file has the opposite.
                        let file_default = default != control.invert;
                        let on = profile
                            .get_bool(SECTION, control.key)
                            .unwrap_or(file_default);
                        Value::Bool(on != control.invert)
                    }
                    Kind::Radio { .. } | Kind::Choice { .. } | Kind::Number { .. } => profile
                        .get_i32(SECTION, control.key)
                        .map_or(fallback, Value::Int),
                    Kind::Text => profile
                        .get_string(SECTION, control.key)
                        .map_or(fallback, Value::Text),
                    Kind::Button { .. } => fallback,
                }
            };
            map.insert(control.id, value);
        }
        Self { map }
    }

    pub(crate) fn get(&self, id: &str) -> Option<&Value> {
        self.map.get(id)
    }

    /// A check box. A name that is not there is off.
    pub(crate) fn bool(&self, id: &str) -> bool {
        matches!(self.map.get(id), Some(Value::Bool(true)))
    }

    /// A number, a radio group or a drop-down list. A name that is not there is 0.
    pub(crate) fn int(&self, id: &str) -> i32 {
        match self.map.get(id) {
            Some(Value::Int(i)) => *i,
            _ => 0,
        }
    }

    /// An edit box. A name that is not there is empty.
    #[cfg(test)]
    pub(crate) fn text(&self, id: &str) -> &str {
        match self.map.get(id) {
            Some(Value::Text(t)) => t,
            _ => "",
        }
    }

    pub(crate) fn set(&mut self, id: &'static str, value: Value) {
        self.map.insert(id, value);
    }

    /// What `GetData` of Sakura's pages does to the numbers: into their range.
    pub(crate) fn clamp_numbers(&mut self, controls: &[Control]) {
        for control in controls {
            if let Kind::Number { min, max } = control.kind
                && let Some(Value::Int(i)) = self.map.get_mut(control.id)
            {
                *i = (*i).clamp(min, max);
            }
        }
    }

    /// Writes the controls that are live into `profile`. Returns whether the file would change.
    /// A key that already has the value is left as it is, byte for byte.
    pub(crate) fn store(
        &self,
        controls: &[Control],
        profile: &mut Profile,
    ) -> Result<bool, ProfileError> {
        let mut done: Vec<&str> = Vec::new();
        let mut changed = false;
        for control in controls {
            if !control.live
                || control.key.is_empty()
                || matches!(control.kind, Kind::Button { .. })
                || done.contains(&control.id)
            {
                continue;
            }
            done.push(control.id);
            let text = match self.map.get(control.id) {
                Some(Value::Bool(on)) => (if *on != control.invert { "1" } else { "0" }).to_owned(),
                Some(Value::Int(i)) => i.to_string(),
                Some(Value::Text(t)) => t.clone(),
                None => continue,
            };
            if profile.get(SECTION, control.key) != Some(text.as_str()) {
                profile.set(SECTION, control.key, &text)?;
                changed = true;
            }
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(text: &str) -> Profile {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        Profile::from_bytes(&bytes)
    }

    #[test]
    fn labels_have_their_access_key() {
        assert_eq!(
            parse_label("フリーカーソル(&F)"),
            ("フリーカーソル(F)".into(), Some('f'))
        );
        assert_eq!(
            parse_label("クリックで&URLを選択する"),
            ("クリックでURLを選択する".into(), Some('u'))
        );
        assert_eq!(
            parse_label("ドラッグ && ドロップ編集する(&D)"),
            ("ドラッグ & ドロップ編集する(D)".into(), Some('d'))
        );
        assert_eq!(parse_label("&Windows風"), ("Windows風".into(), Some('w')));
        assert_eq!(parse_label("OK"), ("OK".into(), None));
        assert_eq!(parse_label("a &"), ("a &".into(), None));
    }

    #[test]
    fn values_come_from_the_file_or_the_default() {
        let list = [
            Control::check("a", "A", "bA", true),
            Control::check("b", "B", "bB", false),
            Control::number("n", "N", "nN", 15, 0, 36),
            Control::text("t", "T", "szT", "%Personal%\\"),
        ];
        let v = Values::load(&list, &profile("[Common]\r\nbA=0\r\nnN=7\r\n"));
        assert!(!v.bool("a"));
        assert!(!v.bool("b"));
        assert_eq!(v.int("n"), 7);
        assert_eq!(v.text("t"), "%Personal%\\");
        let v = Values::load(&list, &Profile::new());
        assert!(v.bool("a"));
        assert_eq!(v.int("n"), 15);
    }

    #[test]
    fn an_inverted_check_box_is_the_opposite_of_its_key() {
        // 「グループ化する」 is `!bDispTabWndMultiWin`; Sakura's default of the key is off.
        let list = [Control::check("grouped", "G", "bDispTabWndMultiWin", true).inverted()];
        assert!(Values::load(&list, &Profile::new()).bool("grouped"));
        let on = profile("[Common]\r\nbDispTabWndMultiWin=1\r\n");
        let v = Values::load(&list, &on);
        assert!(!v.bool("grouped"));
        let mut p = Profile::new();
        v.store(&list, &mut p).unwrap();
        assert_eq!(p.get("Common", "bDispTabWndMultiWin"), Some("1"));
    }

    #[test]
    fn numbers_are_clamped_not_refused() {
        let list = [Control::number("n", "N", "nN", 15, 0, 36)];
        let mut v = Values::load(&list, &profile("[Common]\r\nnN=99\r\n"));
        assert_eq!(v.int("n"), 99);
        v.clamp_numbers(&list);
        assert_eq!(v.int("n"), 36);
        v.set("n", Value::Int(-3));
        v.clamp_numbers(&list);
        assert_eq!(v.int("n"), 0);
    }

    #[test]
    fn only_live_controls_are_written_and_the_rest_of_the_file_stays() {
        let list = [
            Control::check("a", "A", "bA", false),
            Control::check("b", "B", "bB", false).unavailable(),
            Control::radio("r", "R0", "eR", 0, 0),
            Control::radio("r", "R1", "eR", 1, 0),
        ];
        let mut v = Values::load(&list, &Profile::new());
        v.set("a", Value::Bool(true));
        v.set("b", Value::Bool(true));
        v.set("r", Value::Int(1));
        let mut p = profile("[Other]\r\nx=1\r\n[Common]\r\nbA=0\r\n");
        assert!(v.store(&list, &mut p).unwrap());
        assert_eq!(p.get("Common", "bA"), Some("1"));
        assert_eq!(p.get("Common", "bB"), None);
        assert_eq!(p.get("Common", "eR"), Some("1"));
        assert_eq!(p.get("Other", "x"), Some("1"));
        // The same values again change nothing.
        assert!(!v.store(&list, &mut p).unwrap());
    }
}
