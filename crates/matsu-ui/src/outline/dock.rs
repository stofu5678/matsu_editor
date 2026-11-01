//! Where the outline pane sits in the window, how large it is, and how it is sorted: Sakura's
//! docking settings, kept in the settings file under Sakura's keys.
//!
//! Sakura: `sakura_core/outline/CDlgFuncList.h` (`ProfDockSet` and the other `Prof*` accessors),
//! `CDlgFuncList::DoMenu`, `CDlgFuncList::ChangeLayout`, `sakura_core/env/CShareData_IO.cpp`
//! (`ShareData_IO_Common`, `ShareData_IO_TypeConfig`, `ShareData_IO_OutlineDockRect`),
//! `sakura_core/env/CShareData.cpp` and `sakura_core/types/CType.cpp` (the defaults).
//!
//! * `[Common]`: `nOutlineDockSet` (0: the settings of `[Common]` are used, 1: the ones of the type),
//!   `bOutlineDockSync` (every window follows a change), `bOutlineDockDisp` (the pane is open),
//!   `eOutlineDockSide` ([`DockSide`]) and `xyOutlineDock` (`left,top,right,bottom`: the width of
//!   the pane docked at the left, the height at the top, the width at the right, the height at
//!   the bottom).
//! * `[Types(N)]`: the same `bOutlineDockDisp`, `eOutlineDockSide` and `xyOutlineDock` for the type,
//!   and the sort of the type: `nOutlineSortCol` and `bOutlineSortDesc` (the list), `nOutlineSortType`
//!   (the tree).
//!
//! Sakura keeps them in shared memory and writes the file when the last window closes. Here the
//! value is a global of the process, and the file is written when a value changes, under the lock
//! of the settings file ([`crate::history::write_settings`]), as the other saved states are.
//!
//! The sizes are GPUI's logical pixels, where Sakura's are device pixels.
//!
//! Where MATSU differs from Sakura's defaults: a file without `eOutlineDockSide` docks the pane at
//! the right (Sakura: floating, `DOCKSIDE_FLOAT`), which is what this program did before the side
//! could be chosen.

use std::collections::BTreeMap;

use gpui::{App, Global};
use matsu_config::Profile;

const COMMON: &str = "Common";
const KEY_SET: &str = "nOutlineDockSet";
const KEY_SYNC: &str = "bOutlineDockSync";
const KEY_DISP: &str = "bOutlineDockDisp";
const KEY_SIDE: &str = "eOutlineDockSide";
const KEY_RECT: &str = "xyOutlineDock";
const KEY_SORT_COL: &str = "nOutlineSortCol";
const KEY_SORT_DESC: &str = "bOutlineSortDesc";
const KEY_SORT_TYPE: &str = "nOutlineSortType";

/// `MAX_TYPES`: the number of `Types(N)` sections that can exist.
const MAX_TYPES: usize = matsu_config::settings::types::MAX_TYPES as usize;

/// `DOCK_MIN_SIZE`: the smallest width (height) of the docked pane, and the least the text keeps.
pub(crate) const DOCK_MIN_SIZE: f32 = 60.0;

/// `EDockSide` (`sakura_core/env/CommonSetting.h`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum DockSide {
    /// `DOCKSIDE_UNDOCKABLE` (-1): floating, and not to be docked by dragging.
    Undockable,
    /// `DOCKSIDE_FLOAT` (0).
    Float,
    /// `DOCKSIDE_LEFT` (1).
    Left,
    /// `DOCKSIDE_TOP` (2).
    Top,
    /// `DOCKSIDE_RIGHT` (3).
    #[default]
    Right,
    /// `DOCKSIDE_BOTTOM` (4).
    Bottom,
}

impl DockSide {
    /// The side for a number of the file; one that Sakura does not know is the default.
    pub(crate) fn from_code(code: i32) -> Self {
        match code {
            -1 => Self::Undockable,
            0 => Self::Float,
            1 => Self::Left,
            2 => Self::Top,
            3 => Self::Right,
            4 => Self::Bottom,
            _ => Self::default(),
        }
    }

    pub(crate) fn code(self) -> i32 {
        match self {
            Self::Undockable => -1,
            Self::Float => 0,
            Self::Left => 1,
            Self::Top => 2,
            Self::Right => 3,
            Self::Bottom => 4,
        }
    }

    /// `IsDocking`: the pane is a part of the window (`m_eDockSide > DOCKSIDE_FLOAT`).
    pub(crate) fn is_docked(self) -> bool {
        self.code() > Self::Float.code()
    }

    /// The pane is at the left or the right: it takes width and keeps the height of the window.
    pub(crate) fn is_vertical(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    /// The side whose size the pane uses: a floating pane has the width of the one at the right.
    pub(crate) fn size_side(self) -> Self {
        match self {
            Self::Float | Self::Undockable => Self::Right,
            docked => docked,
        }
    }
}

/// The place of the pane in one set of settings (`[Common]` or a type).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DockPlace {
    /// `bOutlineDockDisp`: the pane is open.
    pub disp: bool,
    pub side: DockSide,
    /// `xyOutlineDock`: the width at the left. 0 is "not set yet": a third of the window.
    pub left: i32,
    /// The height at the top.
    pub top: i32,
    /// The width at the right.
    pub right: i32,
    /// The height at the bottom.
    pub bottom: i32,
}

impl DockPlace {
    /// The width (height) kept for `side`; 0 when none is kept or the side has none.
    pub(crate) fn size_of(&self, side: DockSide) -> i32 {
        match side {
            DockSide::Left => self.left,
            DockSide::Top => self.top,
            DockSide::Right => self.right,
            DockSide::Bottom => self.bottom,
            DockSide::Float | DockSide::Undockable => 0,
        }
    }

    /// Keeps the width (height) of `side`.
    pub(crate) fn set_size(&mut self, side: DockSide, size: i32) {
        match side {
            DockSide::Left => self.left = size,
            DockSide::Top => self.top = size,
            DockSide::Right => self.right = size,
            DockSide::Bottom => self.bottom = size,
            DockSide::Float | DockSide::Undockable => {}
        }
    }

    fn read(profile: &Profile, section: &str) -> Self {
        let d = Self::default();
        let (left, top, right, bottom) = profile
            .get(section, KEY_RECT)
            .and_then(parse_rect)
            .unwrap_or((d.left, d.top, d.right, d.bottom));
        Self {
            disp: profile.get_bool(section, KEY_DISP).unwrap_or(d.disp),
            side: profile
                .get_i32(section, KEY_SIDE)
                .map_or(d.side, DockSide::from_code),
            left,
            top,
            right,
            bottom,
        }
    }

    /// Writes the keys that differ from the file (`ShareData_IO_OutlineDockRect` writes
    /// `%d,%d,%d,%d`). Returns whether the file changed.
    fn write(
        &self,
        profile: &mut Profile,
        section: &str,
    ) -> Result<bool, matsu_config::ProfileError> {
        let snapshot = |p: &Profile| {
            [KEY_DISP, KEY_SIDE, KEY_RECT].map(|key| p.get(section, key).map(str::to_owned))
        };
        let before = snapshot(profile);
        profile.set_bool(section, KEY_DISP, self.disp)?;
        profile.set_i32(section, KEY_SIDE, self.side.code())?;
        if profile.get(section, KEY_RECT).and_then(parse_rect)
            != Some((self.left, self.top, self.right, self.bottom))
        {
            profile.set(
                section,
                KEY_RECT,
                &format!("{},{},{},{}", self.left, self.top, self.right, self.bottom),
            )?;
        }
        Ok(before != snapshot(profile))
    }
}

/// `%d,%d,%d,%d`: four numbers or nothing (`IOProfileData<RECT>` fails otherwise).
fn parse_rect(text: &str) -> Option<(i32, i32, i32, i32)> {
    let mut parts = text.split(',').map(|p| p.trim().parse::<i32>());
    Some((
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    ))
}

/// What a type keeps: its own place and its sort.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TypeOutline {
    pub place: DockPlace,
    /// `nOutlineSortCol` (`EFuncListCol`).
    pub sort_col: i32,
    /// `bOutlineSortDesc`.
    pub sort_desc: bool,
    /// `nOutlineSortType` (`SORTTYPE_*`).
    pub sort_type: i32,
}

/// Which set of settings a place belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Common,
    Type(usize),
}

fn type_section(index: usize) -> String {
    matsu_config::settings::TypeSettings::section_name(index as i32)
}

/// The docking settings of the whole program (`CommonSetting_OutLine` and the outline part of each
/// `STypeConfig`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OutlineDock {
    /// `nOutlineDockSet`: 0 uses [`common`](Self::common), 1 uses the place of the type.
    pub dock_set: i32,
    /// `bOutlineDockSync`: the other windows follow a change.
    pub sync: bool,
    pub common: DockPlace,
    pub types: BTreeMap<usize, TypeOutline>,
}

impl Default for OutlineDock {
    fn default() -> Self {
        Self {
            dock_set: 0,
            // CShareData::InitShareData: TRUE
            sync: true,
            common: DockPlace::default(),
            types: BTreeMap::new(),
        }
    }
}

impl Global for OutlineDock {}

impl OutlineDock {
    pub(crate) fn from_profile(profile: &Profile) -> Self {
        let d = Self::default();
        let mut types = BTreeMap::new();
        for index in 0..MAX_TYPES {
            let section = type_section(index);
            if profile.entries(&section).next().is_none() {
                continue;
            }
            let t = TypeOutline::default();
            types.insert(
                index,
                TypeOutline {
                    place: DockPlace::read(profile, &section),
                    sort_col: profile
                        .get_i32(&section, KEY_SORT_COL)
                        .unwrap_or(t.sort_col),
                    sort_desc: profile
                        .get_bool(&section, KEY_SORT_DESC)
                        .unwrap_or(t.sort_desc),
                    sort_type: profile
                        .get_i32(&section, KEY_SORT_TYPE)
                        .unwrap_or(t.sort_type),
                },
            );
        }
        Self {
            dock_set: profile.get_i32(COMMON, KEY_SET).unwrap_or(d.dock_set),
            sync: profile.get_bool(COMMON, KEY_SYNC).unwrap_or(d.sync),
            common: DockPlace::read(profile, COMMON),
            types,
        }
    }

    /// The settings of a type, which are the defaults for one the file does not have.
    pub(crate) fn type_outline(&self, index: usize) -> TypeOutline {
        self.types.get(&index).copied().unwrap_or_default()
    }

    /// Where the set of settings in use for type `index` is (`ProfDockSet`).
    pub(crate) fn target(&self, index: usize) -> Target {
        if self.dock_set == 0 {
            Target::Common
        } else {
            Target::Type(index)
        }
    }

    /// The place in use for type `index` (`ProfDockDisp`, `ProfDockSide`, `ProfDockLeft`...).
    pub(crate) fn place(&self, index: usize) -> DockPlace {
        match self.target(index) {
            Target::Common => self.common,
            Target::Type(i) => self.type_outline(i).place,
        }
    }

    /// The place that `target` names.
    pub(crate) fn place_of(&self, target: Target) -> DockPlace {
        match target {
            Target::Common => self.common,
            Target::Type(i) => self.type_outline(i).place,
        }
    }

    pub(crate) fn set_place(&mut self, target: Target, place: DockPlace) {
        match target {
            Target::Common => self.common = place,
            Target::Type(i) => {
                let mut t = self.type_outline(i);
                t.place = place;
                self.types.insert(i, t);
            }
        }
    }

    /// `CDlgFuncList::DoMenu`, 継承情報を統一: the place of the pane now goes to `[Common]` and to
    /// each of the first `type_count` types.
    pub(crate) fn unify(&mut self, place: DockPlace, type_count: usize) {
        self.common = place;
        for index in 0..type_count.min(MAX_TYPES) {
            let mut t = self.type_outline(index);
            t.place = place;
            self.types.insert(index, t);
        }
    }

    /// The sort of type `index` as (column, descending, tree sort).
    pub(crate) fn set_sort(&mut self, index: usize, col: i32, desc: bool, tree: i32) {
        let mut t = self.type_outline(index);
        t.sort_col = col;
        t.sort_desc = desc;
        t.sort_type = tree;
        self.types.insert(index, t);
    }
}

/// Writes the place of `target`: the keys that differ.
pub(crate) fn write_place(
    profile: &mut Profile,
    target: Target,
    place: &DockPlace,
) -> Result<bool, matsu_config::ProfileError> {
    match target {
        Target::Common => place.write(profile, COMMON),
        Target::Type(i) => place.write(profile, &type_section(i)),
    }
}

/// Writes the choices of `[Common]` (`nOutlineDockSet`, `bOutlineDockSync`).
pub(crate) fn write_common(
    profile: &mut Profile,
    dock: &OutlineDock,
) -> Result<bool, matsu_config::ProfileError> {
    let snapshot =
        |p: &Profile| [KEY_SET, KEY_SYNC].map(|key| p.get(COMMON, key).map(str::to_owned));
    let before = snapshot(profile);
    profile.set_i32(COMMON, KEY_SET, dock.dock_set)?;
    profile.set_bool(COMMON, KEY_SYNC, dock.sync)?;
    Ok(before != snapshot(profile))
}

/// Writes the sort of type `index`.
pub(crate) fn write_sort(
    profile: &mut Profile,
    index: usize,
    sort: &TypeOutline,
) -> Result<bool, matsu_config::ProfileError> {
    let section = type_section(index);
    let snapshot = |p: &Profile| {
        [KEY_SORT_COL, KEY_SORT_DESC, KEY_SORT_TYPE]
            .map(|key| p.get(&section, key).map(str::to_owned))
    };
    let before = snapshot(profile);
    profile.set_i32(&section, KEY_SORT_COL, sort.sort_col)?;
    profile.set_bool(&section, KEY_SORT_DESC, sort.sort_desc)?;
    profile.set_i32(&section, KEY_SORT_TYPE, sort.sort_type)?;
    Ok(before != snapshot(profile))
}

/// Writes everything `dock` holds to `profile`: for a session that has no settings file, whose
/// profile is the one read at the start.
pub(crate) fn write_all(
    profile: &mut Profile,
    dock: &OutlineDock,
) -> Result<bool, matsu_config::ProfileError> {
    let mut changed = write_common(profile, dock)?;
    changed |= write_place(profile, Target::Common, &dock.common)?;
    for (&index, t) in &dock.types {
        changed |= write_place(profile, Target::Type(index), &t.place)?;
        changed |= write_sort(profile, index, t)?;
    }
    Ok(changed)
}

/// The settings now (the defaults before the application installs them).
pub(crate) fn current(cx: &App) -> OutlineDock {
    cx.try_global::<OutlineDock>().cloned().unwrap_or_default()
}

/// Changes the settings of the program and leaves the file as it is (a size that is being dragged).
pub(crate) fn set_memory(cx: &mut App, change: impl FnOnce(&mut OutlineDock)) {
    let mut dock = current(cx);
    change(&mut dock);
    cx.set_global(dock);
}

/// Writes the settings of the program to the file with `write`, changed or not.
pub(crate) fn flush(
    cx: &mut App,
    write: impl FnOnce(&mut Profile, &OutlineDock) -> Result<bool, matsu_config::ProfileError> + 'static,
) {
    let dock = current(cx);
    crate::history::write_settings(cx, move |profile| write(profile, &dock));
}

/// Changes the settings of the program and writes them to the file with `write`. Nothing happens
/// when `change` leaves them as they were.
pub(crate) fn update(
    cx: &mut App,
    change: impl FnOnce(&mut OutlineDock),
    write: impl FnOnce(&mut Profile, &OutlineDock) -> Result<bool, matsu_config::ProfileError> + 'static,
) {
    let mut dock = current(cx);
    let old = dock.clone();
    change(&mut dock);
    if dock == old && cx.has_global::<OutlineDock>() {
        return;
    }
    cx.set_global(dock.clone());
    crate::history::write_settings(cx, move |profile| write(profile, &dock));
}
