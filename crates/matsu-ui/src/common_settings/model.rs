//! What a page of the common settings is made of: its controls, and the values they hold.
//!
//! A page is a list of [`Item`]s. A [`Control`] knows its label, the key of `[Common]` that holds
//! its value, the default (Sakura's, `CShareData::InitShareData`), the limits, and whether MATSU
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

use matsu_config::settings::{FontSetting, LogFont};
use matsu_config::{Profile, ProfileError};

use super::dialog::CommonSettingsDialog;
use super::table::{Row, TableSpec};

/// The section of the settings file that holds the settings of the dialog.
pub(crate) const SECTION: &str = "Common";

/// A value a control holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    Bool(bool),
    Int(i32),
    Text(String),
    /// An ordered list of numbers that a page edits with its own widget ([`Kind::Numbers`]).
    Numbers(Vec<i32>),
    /// The rows of an editable list ([`Kind::Table`]).
    Rows(Vec<Row>),
    /// A font ([`Kind::Font`]); one with no face name is not set.
    Font(FontSetting),
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

/// How a [`Kind::Numbers`] control reads its list from the settings file.
pub(crate) type LoadNumbers = fn(&Profile) -> Vec<i32>;

/// How a [`Kind::Numbers`] control writes its list to the settings file (keys, sections and all);
/// returns whether the file changed.
pub(crate) type StoreNumbers = fn(&[i32], &mut Profile) -> Result<bool, ProfileError>;

/// What a change of a control does to the other values of the dialog (the `CBN_SELCHANGE` and
/// `BN_CLICKED` handlers of Sakura's pages that set other controls). It gets the values and the id
/// of the control that changed.
pub(crate) type Changed = fn(&mut Values, &'static str);

/// `Changed` for a control whose change affects no other value.
pub(crate) fn unchanged(_: &mut Values, _: &'static str) {}

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
    /// A list box of choices that shows all of them (the color items of the type settings): the
    /// label and the number of each. The value is the number of the chosen one.
    List {
        options: &'static [(&'static str, i32)],
    },
    /// A number with its range (an edit box with a spin button in Sakura). A range that starts
    /// below 0 takes a sign (`GetDlgItemInt(..., TRUE)`).
    Number { min: i32, max: i32 },
    /// A color (`COLORREF`, `0x00BBGGRR`) chosen with a color picker.
    Color,
    /// An edit box.
    Text,
    /// A push button.
    Button { run: ButtonAction },
    /// A list of numbers that a page edits with a widget of its own ([`Item::Custom`]): the
    /// toolbar's buttons. The list is not one key but several, so the control says how to read
    /// and write them. The dialog draws nothing for the control itself.
    Numbers {
        load: LoadNumbers,
        store: StoreNumbers,
    },
    /// An editable list: rows of texts with the buttons that change them
    /// ([`super::table`]). The value is a [`Value::Rows`]; the file keeps it as the spec says.
    Table { spec: &'static TableSpec },
    /// A font (a `LOGFONT` and its size, `ShareData_IO_LogFont`) chosen with the font dialog by a
    /// button of the page. The value is a [`Value::Font`]; a font the file does not have is one
    /// with no face name ([`not_set`]), and is not written. An empty `key_lf` is a font that the
    /// sheet keeps itself (the font of a document type).
    Font {
        key_lf: &'static str,
        key_point_size: Option<&'static str>,
        key_face_name: Option<&'static str>,
    },
}

/// The label next to a font button: the face and the size, drawn in the font itself but not larger
/// than 16 points (`SetFontLabel`, `sakura_core/prop/CPropCommon.cpp`: "大きすぎるフォントは小さく表示").
/// `None` is an empty label; a font that is not set shows 「（既定）」.
pub(crate) fn font_label(
    font: Option<&FontSetting>,
    _: &mut gpui::Context<CommonSettingsDialog>,
) -> gpui::AnyElement {
    use gpui::{IntoElement as _, ParentElement as _, Styled as _, div, px};
    let Some(font) = font else {
        return div().into_any_element();
    };
    if font.log_font.face_name.is_empty() {
        return div().child("（既定）").into_any_element();
    }
    let family = crate::settings::view_settings::font_family_for(
        &font.log_font.face_name,
        matsu_config::location::Os::current(),
    );
    let points = font
        .point_size_for_dpi(crate::settings::view_settings::DPI)
        .min(160);
    let mut label = div()
        .text_size(px(crate::settings::view_settings::points_to_px(points)))
        .child(crate::font_dialog::model::label(font));
    if let Some(family) = family {
        label = label.font_family(family);
    }
    label.into_any_element()
}

/// A font that is not set: no face name.
pub(crate) fn not_set() -> FontSetting {
    FontSetting {
        log_font: LogFont {
            face_name: String::new(),
            ..FontSetting::default().log_font
        },
        point_size: 0,
    }
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
    /// Whether MATSU Editor does what the control says. A control that is not live is shown with
    /// the value of the file, greyed out, and never written.
    pub live: bool,
    pub enabled: Enabled,
    pub kind: Kind,
    /// The value is the opposite of the key's (`bDispTabWndMultiWin`).
    pub invert: bool,
    /// The section that holds the key: `[Common]` unless [`Control::in_section`] says otherwise
    /// (`ShareData_IO_Statusbar` writes `[Statusbar]`, `ShareData_IO_Nickname` `[Nickname]`).
    pub section: &'static str,
    /// A check box that is one bit of the number in the key (`nBackUpType_Opt1`): the bit; 0 for a
    /// control that holds the whole key. The other bits of the key stay as the file has them.
    pub bit: i32,
    /// For a [`bit`](Self::bit): the default of the whole number in the key.
    pub key_default: i32,
    /// What a change of this control does to the other values.
    pub changed: Changed,
    /// The name a screen reader says, for a control whose own label is empty because Sakura
    /// draws its caption as a separate text (or as the title of a group box, or on the check box
    /// next to it). Empty: the name is made from the label ([`Control::accessible_name`]).
    pub name: &'static str,
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
            section: SECTION,
            bit: 0,
            key_default: 0,
            changed: unchanged,
            name: "",
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

    /// A list box that shows every choice.
    pub(crate) const fn list(
        id: &'static str,
        label: &'static str,
        key: &'static str,
        default: i32,
        options: &'static [(&'static str, i32)],
    ) -> Self {
        Self::new(id, label, key, Def::Int(default), Kind::List { options })
    }

    /// A color.
    pub(crate) const fn color(
        id: &'static str,
        label: &'static str,
        key: &'static str,
        default: i32,
    ) -> Self {
        Self::new(id, label, key, Def::Int(default), Kind::Color)
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

    /// A list of numbers edited by a custom widget; `load` and `store` map it to the file.
    pub(crate) const fn numbers(id: &'static str, load: LoadNumbers, store: StoreNumbers) -> Self {
        Self::new(id, "", "", Def::Bool(false), Kind::Numbers { load, store })
    }

    /// An editable list of rows. `label` is the text above it; it holds no `key` (the spec says
    /// where the rows are in the file).
    pub(crate) const fn table(
        id: &'static str,
        label: &'static str,
        spec: &'static TableSpec,
    ) -> Self {
        Self::new(id, label, "", Def::Bool(false), Kind::Table { spec })
    }

    /// A font chosen with the font dialog; `key_lf` and the keys of `ShareData_IO_LogFont` hold it.
    pub(crate) const fn font(
        id: &'static str,
        key_lf: &'static str,
        key_point_size: Option<&'static str>,
        key_face_name: Option<&'static str>,
    ) -> Self {
        Self::new(
            id,
            "",
            "",
            Def::Bool(false),
            Kind::Font {
                key_lf,
                key_point_size,
                key_face_name,
            },
        )
    }

    /// A push button.
    pub(crate) const fn button(id: &'static str, label: &'static str, run: ButtonAction) -> Self {
        Self::new(id, label, "", Def::Bool(false), Kind::Button { run })
    }

    /// MATSU Editor does not do what this control says yet (or the OS has no such thing): it is
    /// shown greyed out with the note 「未対応」.
    pub(crate) const fn unavailable(mut self) -> Self {
        self.live = false;
        self
    }

    /// The name a screen reader says for the control, when its label is empty (see
    /// [`Control::name`]). Write it without an access key mark.
    pub(crate) const fn named(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    /// The name a screen reader says: [`Control::name`] if the page gave one, else the label
    /// without its access key mark.
    pub(crate) fn accessible_name(&self) -> String {
        if self.name.is_empty() {
            crate::a11y::plain_name(self.label)
        } else {
            self.name.to_owned()
        }
    }

    /// The control can be used only while `enabled` says so.
    pub(crate) const fn when(mut self, enabled: Enabled) -> Self {
        self.enabled = enabled;
        self
    }

    /// The key is in `section` of the file, not in `[Common]`.
    pub(crate) const fn in_section(mut self, section: &'static str) -> Self {
        self.section = section;
        self
    }

    /// The control holds only the bits `mask` of the number in its key, whose default (all bits)
    /// is `key_default` (`CommonSetting_Backup::GetBackupOpt` and `GetBackupCount`): a check box
    /// is one bit (`mask` has one), a number the low bits (`mask` is `0xffff`). The other bits of
    /// the key stay as the file has them.
    pub(crate) const fn packed(mut self, mask: i32, key_default: i32) -> Self {
        self.bit = mask;
        self.key_default = key_default;
        self
    }

    /// A check box that is the bit `mask` of the number in `key`.
    pub(crate) const fn flag(
        id: &'static str,
        label: &'static str,
        key: &'static str,
        mask: i32,
        key_default: i32,
    ) -> Self {
        Self::new(
            id,
            label,
            key,
            Def::Bool(key_default & mask == mask),
            Kind::Check,
        )
        .packed(mask, key_default)
    }

    /// `changed` runs when the user changes this control.
    pub(crate) const fn on_change(mut self, changed: Changed) -> Self {
        self.changed = changed;
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
    /// Something the dialog has no control for, drawn by the page (two lists with buttons between
    /// them). Its state lives in controls of the page that the dialog does not draw (list the
    /// controls in the page's items with [`Item::Hidden`]).
    Custom(CustomDraw),
    /// Controls that hold state for a [`Item::Custom`]; they are not drawn.
    Hidden(Vec<Control>),
}

/// Draws a [`Item::Custom`]. It reads the values of the dialog and changes them through the
/// dialog's `set_*` methods.
pub(crate) type CustomDraw =
    fn(&CommonSettingsDialog, &mut gpui::Context<CommonSettingsDialog>) -> gpui::AnyElement;

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
                Item::Hidden(list) => out.extend(list.iter().copied()),
                Item::Label(_) | Item::Custom(_) => {}
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
            if let Kind::Table { spec } = control.kind {
                map.insert(control.id, Value::Rows(spec.load(profile)));
                continue;
            }
            let fallback = control.default.value();
            let value = if let Kind::Numbers { load, .. } = control.kind {
                Value::Numbers(load(profile))
            } else if let Kind::Font {
                key_lf,
                key_point_size,
                key_face_name,
            } = control.kind
            {
                let mut font = not_set();
                if !key_lf.is_empty() {
                    font.read(
                        profile,
                        control.section,
                        key_lf,
                        key_point_size,
                        key_face_name,
                    );
                }
                Value::Font(font)
            } else if control.key.is_empty() {
                fallback
            } else {
                match control.kind {
                    Kind::Check if control.bit != 0 => {
                        let number = profile
                            .get_i32(control.section, control.key)
                            .unwrap_or(control.key_default);
                        Value::Bool(number & control.bit == control.bit)
                    }
                    Kind::Check => {
                        let default = matches!(fallback, Value::Bool(true));
                        // `invert`: the default is the control's, the file has the opposite.
                        let file_default = default != control.invert;
                        let on = profile
                            .get_bool(control.section, control.key)
                            .unwrap_or(file_default);
                        Value::Bool(on != control.invert)
                    }
                    Kind::Number { .. } if control.bit != 0 => {
                        let number = profile
                            .get_i32(control.section, control.key)
                            .unwrap_or(control.key_default);
                        Value::Int(number & control.bit)
                    }
                    Kind::Radio { .. }
                    | Kind::Choice { .. }
                    | Kind::List { .. }
                    | Kind::Color
                    | Kind::Number { .. } => profile
                        .get_i32(control.section, control.key)
                        .map_or(fallback, Value::Int),
                    Kind::Text => profile
                        .get_string(control.section, control.key)
                        .map_or(fallback, Value::Text),
                    Kind::Numbers { .. } => fallback,
                    Kind::Button { .. } | Kind::Table { .. } | Kind::Font { .. } => fallback,
                }
            };
            map.insert(control.id, value);
        }
        let mut values = Self { map };
        // The page バックアップ shows the kinds 5 and 6 as 1 and 3 with a check box.
        if values.map.contains_key(super::pages::backup::RETAIN_EXT) {
            super::pages::backup::after_load(&mut values);
        }
        values
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

    /// A list of numbers. A name that is not there is empty.
    pub(crate) fn numbers(&self, id: &str) -> &[i32] {
        match self.map.get(id) {
            Some(Value::Numbers(list)) => list,
            _ => &[],
        }
    }

    /// An edit box. A name that is not there is empty.
    pub(crate) fn text(&self, id: &str) -> &str {
        match self.map.get(id) {
            Some(Value::Text(t)) => t,
            _ => "",
        }
    }

    /// A font. A name that is not there is not set.
    pub(crate) fn font(&self, id: &str) -> FontSetting {
        match self.map.get(id) {
            Some(Value::Font(font)) => font.clone(),
            _ => not_set(),
        }
    }

    /// The rows of an editable list. A name that is not there has none.
    pub(crate) fn rows(&self, id: &str) -> &[Row] {
        match self.map.get(id) {
            Some(Value::Rows(rows)) => rows,
            _ => &[],
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
        if self.map.contains_key(super::pages::backup::RETAIN_EXT) {
            let mut folded = self.clone();
            super::pages::backup::before_store(&mut folded);
            return folded.store_values(controls, profile);
        }
        self.store_values(controls, profile)
    }

    fn store_values(
        &self,
        controls: &[Control],
        profile: &mut Profile,
    ) -> Result<bool, ProfileError> {
        let mut done: Vec<&str> = Vec::new();
        let mut changed = false;
        for control in controls {
            if let Kind::Numbers { store, .. } = control.kind {
                if control.live
                    && !done.contains(&control.id)
                    && let Some(Value::Numbers(list)) = self.map.get(control.id)
                {
                    done.push(control.id);
                    changed |= store(list, profile)?;
                }
                continue;
            }
            if let Kind::Font {
                key_lf,
                key_point_size,
                key_face_name,
            } = control.kind
            {
                if control.live
                    && !key_lf.is_empty()
                    && !done.contains(&control.id)
                    && let Some(Value::Font(font)) = self.map.get(control.id)
                    && !font.log_font.face_name.is_empty()
                {
                    done.push(control.id);
                    let mut now = not_set();
                    now.read(
                        profile,
                        control.section,
                        key_lf,
                        key_point_size,
                        key_face_name,
                    );
                    if now != *font {
                        font.write(
                            profile,
                            control.section,
                            key_lf,
                            key_point_size,
                            key_face_name,
                        )?;
                        changed = true;
                    }
                }
                continue;
            }
            if let Kind::Table { spec } = control.kind {
                if control.live
                    && !done.contains(&control.id)
                    && let Some(Value::Rows(rows)) = self.map.get(control.id)
                {
                    done.push(control.id);
                    changed |= spec.store(rows, profile)?;
                }
                continue;
            }
            if !control.live
                || control.key.is_empty()
                || matches!(control.kind, Kind::Button { .. })
                || done.contains(&control.id)
            {
                continue;
            }
            done.push(control.id);
            if control.bit != 0 {
                let current = profile
                    .get_i32(control.section, control.key)
                    .unwrap_or(control.key_default);
                let number = match self.map.get(control.id) {
                    Some(Value::Bool(true)) => current | control.bit,
                    Some(Value::Bool(false)) => current & !control.bit,
                    Some(Value::Int(i)) => (current & !control.bit) | (*i & control.bit),
                    _ => continue,
                };
                let text = number.to_string();
                if profile.get(control.section, control.key) != Some(text.as_str()) {
                    profile.set(control.section, control.key, &text)?;
                    changed = true;
                }
                continue;
            }
            let text = match self.map.get(control.id) {
                Some(Value::Bool(on)) => (if *on != control.invert { "1" } else { "0" }).to_owned(),
                Some(Value::Int(i)) => i.to_string(),
                Some(Value::Text(t)) => t.clone(),
                Some(Value::Numbers(_)) | Some(Value::Rows(_)) | Some(Value::Font(_)) | None => {
                    continue;
                }
            };
            if profile.get(control.section, control.key) != Some(text.as_str()) {
                profile.set(control.section, control.key, &text)?;
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

    #[test]
    fn a_control_can_be_a_part_of_a_number_in_another_section() {
        // `nBackUpType_Opt1` holds bits; `_Opt2` the generations in the low 16 bits.
        let list = [
            Control::flag("year", "Y", "bOpt1", 32, 56),
            Control::flag("sec", "S", "bOpt1", 1, 56),
            Control::flag("auto", "A", "bOpt1", 64, 56),
            Control::number("gens", "G", "bOpt2", 10, 1, 99).packed(0xffff, (98 << 16) + 10),
            Control::check("short", "S", "bShortPath", true).in_section("Nickname"),
        ];
        let v = Values::load(&list, &Profile::new());
        assert!(v.bool("year") && !v.bool("sec") && !v.bool("auto"));
        assert_eq!(v.int("gens"), 10);
        let file =
            profile("[Common]\r\nbOpt1=65\r\nbOpt2=6422547\r\n[Nickname]\r\nbShortPath=0\r\n");
        let mut v = Values::load(&list, &file);
        assert!(!v.bool("year") && v.bool("sec") && v.bool("auto"));
        assert_eq!(v.int("gens"), 19);
        assert!(!v.bool("short"));
        // The other bits of the key stay as the file has them.
        v.set("year", Value::Bool(true));
        v.set("gens", Value::Int(42));
        v.set("short", Value::Bool(true));
        let mut p = file.clone();
        assert!(v.store(&list, &mut p).unwrap());
        assert_eq!(p.get("Common", "bOpt1"), Some("97"));
        // 'b' << 16 is kept: only the low 16 bits are the generations.
        assert_eq!(p.get("Common", "bOpt2"), Some("6422570"));
        assert_eq!(p.get("Nickname", "bShortPath"), Some("1"));
        assert_eq!(p.get("Common", "bShortPath"), None);
    }
}
