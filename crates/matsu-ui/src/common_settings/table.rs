//! An editable list for a page of the settings: rows of texts, a selected row, one edit box for
//! each column and the buttons that change the list.
//!
//! Sakura has this control on several pages, always built the same way: a list view with a
//! few columns, an edit box under it for each column, and the push buttons 先頭, 上へ, 下へ, 最終,
//! 挿入, 追加, 更新 and 削除 (`IDD_PROP_FNAME`, `CPropFileName`; the keyword help dictionaries and
//! the regular expression keywords of the type settings are the same). A page describes its list
//! with a [`TableSpec`] (the columns, the limit, the buttons, where the file keeps the rows) and
//! puts it in with `Control::table`. The rows are a [`Value::Rows`](super::model::Value) of the
//! dialog; the buttons are [`Op`]s, which [`apply`] does to the rows.
//!
//! The rules of the buttons are those of `CPropFileName::DispatchEvent`
//! (`sakura_core/prop/CPropComFileName.cpp`):
//!
//! * 挿入 puts the row before the selected one (at the end when none is selected), 追加 at the end.
//!   A row whose first column is empty is not put in. At the limit the answer is the message of
//!   the page (「これ以上登録できません。」).
//! * 更新 replaces the selected row; with no row selected and an empty list it adds one.
//! * 削除 removes the selected row and selects the one that comes to its place.
//! * 先頭, 上へ, 下へ, 最終 move the selected row (the row 0 when none is selected: only 最終
//!   does something then); a move that ends where it began does nothing.
//! * OK drops the rows whose first column is empty (`GetData`).

use matsu_config::{Profile, ProfileError};

/// A row of the list: one text for each column.
pub(crate) type Row = Vec<String>;

/// A column of the list.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Column {
    /// The heading of the column in the list (`STR_PROPCOMFNM_LIST1`).
    pub title: &'static str,
    /// The label of the edit box of the column, with `&` before the access key (`置換前(&F)`).
    pub edit_label: &'static str,
    /// The beginning of the key of this column in the file (`From`); the row number follows.
    pub key: &'static str,
    /// How much of the width of the list the column takes, in percent (`col.cx`).
    pub width: u32,
    /// The most characters the edit box takes (`EditCtl_LimitText`).
    pub max_chars: usize,
}

/// The push buttons of the list, in Sakura's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    Top,
    Up,
    Down,
    Last,
    Insert,
    Add,
    Update,
    Delete,
}

impl Op {
    /// Sakura's label with the access key of `IDD_PROP_FNAME`.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Op::Top => "先頭(&T)",
            Op::Up => "上へ(&U)",
            Op::Down => "下へ(&G)",
            Op::Last => "最終(&B)",
            Op::Insert => "挿入(&S)",
            Op::Add => "追加(&A)",
            Op::Update => "更新(&E)",
            Op::Delete => "削除(&D)",
        }
    }

    /// Sakura's order in the dialog: the moves, then the changes.
    pub(crate) const ALL: [Op; 8] = [
        Op::Top,
        Op::Up,
        Op::Down,
        Op::Last,
        Op::Insert,
        Op::Add,
        Op::Update,
        Op::Delete,
    ];
}

/// Where the rows are kept in the settings file.
#[derive(Clone, Copy)]
pub(crate) enum Storage {
    /// One key for each column and row, `<column key><row number>` in `section`, and the number
    /// of rows in `count_key` (`ShareData_IO_Nickname`: `ArrNum`, `From00`, `To00` ...). The row
    /// number has `digits` digits and starts at 0. When the file has no count the rows are the
    /// page's defaults; a key the file lacks is the default of that row, or empty.
    Numbered {
        section: &'static str,
        count_key: &'static str,
        digits: usize,
    },
    /// A page that keeps its rows its own way. `load` gives `None` when the file has none (the
    /// defaults then); `store` returns whether it changed the file.
    Custom {
        load: fn(&Profile) -> Option<Vec<Row>>,
        store: fn(&[Row], &mut Profile) -> Result<bool, ProfileError>,
    },
}

/// What a page says about a list.
#[derive(Clone, Copy)]
pub(crate) struct TableSpec {
    pub columns: &'static [Column],
    /// The most rows (`MAX_TRANSFORM_FILENAME`).
    pub max_rows: usize,
    /// The buttons the page has.
    pub ops: &'static [Op],
    /// The rows when the file has none (`CShareData::InitShareData`).
    pub defaults: &'static [&'static [&'static str]],
    pub storage: Storage,
    /// What is said when a row cannot be added (`STR_PROPCOMFNM_ERR_REG`).
    pub full_message: &'static str,
    /// A page that checks (and completes) a row before 挿入, 追加 and 更新 put it in, as
    /// `CPropTypesRegex::CheckKeywordList` and the 挿入/更新 of `CPropTypesKeyHelp` do.
    pub prepare: Option<Prepare>,
}

/// Checks the row about to be put in. It may complete the row. `rows` are the rows now, and
/// `updating` the row that the row replaces (`None` for a row that is added). `base` is the folder
/// that a relative path starts at. The error is what is said to the user; the list stays as it is.
pub(crate) type Prepare = fn(
    row: &mut Row,
    rows: &[Row],
    updating: Option<usize>,
    base: Option<&std::path::Path>,
) -> Result<(), String>;

/// Why a button did nothing that the user should be told.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refused {
    /// The list is full.
    Full,
    /// The page's check refused the row, with the words to say.
    Rejected(String),
}

impl TableSpec {
    /// The rows the page starts with.
    pub(crate) fn default_rows(&self) -> Vec<Row> {
        self.defaults
            .iter()
            .map(|row| row.iter().map(|t| (*t).to_owned()).collect())
            .collect()
    }

    /// The rows of the file (`ShareData_IO_Nickname` in reading mode, `SetData`): a row whose first
    /// column is empty is left out.
    pub(crate) fn load(&self, profile: &Profile) -> Vec<Row> {
        let rows = match self.storage {
            Storage::Numbered {
                section,
                count_key,
                digits,
            } => {
                let defaults = self.default_rows();
                let count = profile
                    .get_i32(section, count_key)
                    .map_or(defaults.len(), |n| {
                        usize::try_from(n).unwrap_or(0).min(self.max_rows)
                    });
                (0..count)
                    .map(|i| {
                        self.columns
                            .iter()
                            .enumerate()
                            .map(|(c, column)| {
                                profile
                                    .get_string(section, &numbered_key(column.key, i, digits))
                                    .or_else(|| defaults.get(i).and_then(|r| r.get(c).cloned()))
                                    .unwrap_or_default()
                            })
                            .collect()
                    })
                    .collect()
            }
            Storage::Custom { load, .. } => load(profile).unwrap_or_else(|| self.default_rows()),
        };
        self.cleaned(rows)
    }

    /// Writes the rows (`GetData`, `ShareData_IO_Nickname` in writing mode): the keys of the rows
    /// and the count; the keys of rows that are gone are removed. A key that already has the
    /// value is left alone. Returns whether the file changed.
    pub(crate) fn store(&self, rows: &[Row], profile: &mut Profile) -> Result<bool, ProfileError> {
        let rows = self.cleaned(rows.to_vec());
        match self.storage {
            Storage::Numbered {
                section,
                count_key,
                digits,
            } => {
                let mut changed = false;
                let mut put = |profile: &mut Profile, key: &str, value: &str| {
                    if profile.get(section, key) != Some(value) {
                        profile.set(section, key, value)?;
                        changed = true;
                    }
                    Ok::<(), ProfileError>(())
                };
                put(profile, count_key, &rows.len().to_string())?;
                for (i, row) in rows.iter().enumerate() {
                    for (column, text) in self.columns.iter().zip(row) {
                        put(profile, &numbered_key(column.key, i, digits), text)?;
                    }
                }
                for i in rows.len()..self.max_rows {
                    for column in self.columns {
                        changed |= profile.remove(section, &numbered_key(column.key, i, digits));
                    }
                }
                Ok(changed)
            }
            Storage::Custom { store, .. } => store(&rows, profile),
        }
    }

    /// The rows as OK takes them: at most `max_rows`, none with an empty first column, every row
    /// with a text for each column.
    fn cleaned(&self, rows: Vec<Row>) -> Vec<Row> {
        rows.into_iter()
            .filter(|row| row.first().is_some_and(|t| !t.is_empty()))
            .take(self.max_rows)
            .map(|mut row| {
                row.resize(self.columns.len(), String::new());
                row
            })
            .collect()
    }
}

/// `From%02d`: the key of a column in a row.
pub(crate) fn numbered_key(prefix: &str, index: usize, digits: usize) -> String {
    format!("{prefix}{index:0digits$}")
}

/// Does what the button `op` does (`CPropFileName::DispatchEvent`) to `rows`. `fields` are the
/// texts of the edit boxes, `selected` the selected row. Returns whether the rows or the selection
/// changed, or why the button was refused. After a change `selected` is the row to show in the
/// edit boxes (`None` clears them).
#[cfg(test)]
pub(crate) fn apply(
    spec: &TableSpec,
    rows: &mut Vec<Row>,
    selected: &mut Option<usize>,
    op: Op,
    fields: &[String],
) -> Result<bool, Refused> {
    apply_in(spec, rows, selected, op, fields, None)
}

/// Does what the button `op` does, for a page whose check reads files: `base` is the folder a
/// relative path starts at.
pub(crate) fn apply_in(
    spec: &TableSpec,
    rows: &mut Vec<Row>,
    selected: &mut Option<usize>,
    op: Op,
    fields: &[String],
    base: Option<&std::path::Path>,
) -> Result<bool, Refused> {
    let count = rows.len();
    // `EditCtl_LimitText`: an edit box takes only so many characters.
    let mut row: Row = spec
        .columns
        .iter()
        .enumerate()
        .map(|(i, column)| {
            fields
                .get(i)
                .map(|text| text.chars().take(column.max_chars).collect())
                .unwrap_or_default()
        })
        .collect();
    row.resize(spec.columns.len(), String::new());
    match op {
        Op::Insert => {
            let at = selected.unwrap_or(count);
            put(spec, rows, selected, at, row, true, base)
        }
        Op::Add => put(spec, rows, selected, count, row, true, base),
        Op::Update => match *selected {
            Some(at) => put(spec, rows, selected, at, row, false, base),
            // "No row is selected because there is none": the button adds the first row.
            None if count == 0 => put(spec, rows, selected, 0, row, true, base),
            None => Ok(false),
        },
        Op::Delete => match *selected {
            Some(at) if at < count => {
                rows.remove(at);
                // The row that comes to the place is selected; at the end nothing is.
                *selected = (at < rows.len()).then_some(at);
                Ok(true)
            }
            _ => Ok(false),
        },
        Op::Top => mv(rows, selected, 0),
        Op::Up => mv(rows, selected, as_signed(*selected) - 1),
        Op::Down => mv(rows, selected, as_signed(*selected) + 1),
        Op::Last => mv(rows, selected, count as isize - 1),
    }
}

fn as_signed(selected: Option<usize>) -> isize {
    selected.map_or(-1, |i| i as isize)
}

/// `SetListViewItem_FILENAME`: puts `row` at `at` (inserting it, or replacing the row there) and
/// selects it. An empty first column or a full list changes nothing.
fn put(
    spec: &TableSpec,
    rows: &mut Vec<Row>,
    selected: &mut Option<usize>,
    at: usize,
    mut row: Row,
    insert: bool,
    base: Option<&std::path::Path>,
) -> Result<bool, Refused> {
    if row.first().is_none_or(String::is_empty) {
        return Ok(false);
    }
    if insert && rows.len() >= spec.max_rows {
        return Err(Refused::Full);
    }
    if let Some(prepare) = spec.prepare {
        let updating = (!insert).then_some(at);
        prepare(&mut row, rows, updating, base).map_err(Refused::Rejected)?;
    }
    if insert {
        rows.insert(at.min(rows.len()), row);
    } else if let Some(slot) = rows.get_mut(at) {
        *slot = row;
    } else {
        return Ok(false);
    }
    *selected = Some(at.min(rows.len() - 1));
    Ok(true)
}

/// `MoveListViewItem_FILENAME`: moves the selected row to `to`. Both places are held inside the
/// list; a move that ends where it began does nothing.
fn mv(rows: &mut Vec<Row>, selected: &mut Option<usize>, to: isize) -> Result<bool, Refused> {
    let last = rows.len() as isize - 1;
    let from = as_signed(*selected).clamp(0, last.max(0));
    let to = to.clamp(0, last.max(0));
    if from == to || rows.is_empty() {
        return Ok(false);
    }
    let row = rows.remove(from as usize);
    rows.insert(to as usize, row);
    *selected = Some(to as usize);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLUMNS: &[Column] = &[
        Column {
            title: "前",
            edit_label: "前(&F)",
            key: "From",
            width: 60,
            max_chars: 259,
        },
        Column {
            title: "後",
            edit_label: "後(&R)",
            key: "To",
            width: 35,
            max_chars: 259,
        },
    ];

    const SPEC: TableSpec = TableSpec {
        columns: COLUMNS,
        max_rows: 4,
        ops: &Op::ALL,
        defaults: &[&["%A%\\", "a"], &["%B%\\", "b"]],
        storage: Storage::Numbered {
            section: "Nickname",
            count_key: "ArrNum",
            digits: 2,
        },
        full_message: "これ以上登録できません。",
        prepare: None,
    };

    fn rows(list: &[&str]) -> Vec<Row> {
        list.iter()
            .map(|t| vec![(*t).to_owned(), t.to_uppercase()])
            .collect()
    }

    fn firsts(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|r| r[0].as_str()).collect()
    }

    fn fields(a: &str, b: &str) -> Vec<String> {
        vec![a.to_owned(), b.to_owned()]
    }

    fn profile(text: &str) -> Profile {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        Profile::from_bytes(&bytes)
    }

    /// A page's check: refuses "bad", completes the second column, and gets the folder.
    fn check(
        row: &mut Row,
        _rows: &[Row],
        updating: Option<usize>,
        base: Option<&std::path::Path>,
    ) -> Result<(), String> {
        if row[0] == "bad" {
            return Err("no".to_owned());
        }
        row[1] = format!("{updating:?}:{}", base.is_some());
        Ok(())
    }

    #[test]
    fn the_check_of_a_page_refuses_or_completes_a_row_before_it_is_put_in() {
        let spec = TableSpec {
            prepare: Some(check),
            ..SPEC
        };
        let mut r = rows(&["a"]);
        let mut sel = Some(0);
        assert_eq!(
            apply(&spec, &mut r, &mut sel, Op::Add, &fields("bad", "x")),
            Err(Refused::Rejected("no".to_owned()))
        );
        assert_eq!(firsts(&r), ["a"]);
        // The check completes the row; a new row has no row it replaces.
        apply(&spec, &mut r, &mut sel, Op::Add, &fields("b", "x")).unwrap();
        assert_eq!(r[1], ["b", "None:false"]);
        // An update names the row, and the folder is given to the check.
        let base = std::path::Path::new("/ini");
        apply_in(
            &spec,
            &mut r,
            &mut sel,
            Op::Update,
            &fields("c", ""),
            Some(base),
        )
        .unwrap();
        assert_eq!(r[1], ["c", "Some(1):true"]);
        // The check does not see a row that is not put in (an empty first column, a full list).
        let mut r = rows(&["a", "b", "c", "d"]);
        assert_eq!(
            apply(&spec, &mut r, &mut sel, Op::Add, &fields("bad", "")),
            Err(Refused::Full)
        );
        assert_eq!(
            apply(&spec, &mut r, &mut sel, Op::Add, &fields("", "")),
            Ok(false)
        );
    }

    #[test]
    fn insert_goes_before_the_selected_row_and_add_to_the_end() {
        let mut r = rows(&["a", "b", "c"]);
        let mut sel = Some(1);
        assert_eq!(
            apply(&SPEC, &mut r, &mut sel, Op::Insert, &fields("x", "X")),
            Ok(true)
        );
        assert_eq!(firsts(&r), ["a", "x", "b", "c"]);
        assert_eq!(sel, Some(1));
        // Nothing selected: at the end.
        let mut r = rows(&["a", "b"]);
        let mut sel = None;
        apply(&SPEC, &mut r, &mut sel, Op::Insert, &fields("x", "")).unwrap();
        assert_eq!(firsts(&r), ["a", "b", "x"]);
        assert_eq!(sel, Some(2));
        // Add ignores the selection.
        let mut sel = Some(0);
        apply(&SPEC, &mut r, &mut sel, Op::Add, &fields("y", "")).unwrap();
        assert_eq!(firsts(&r), ["a", "b", "x", "y"]);
        assert_eq!(sel, Some(3));
    }

    #[test]
    fn an_empty_first_column_is_not_put_in_and_a_full_list_refuses() {
        let mut r = rows(&["a"]);
        let mut sel = None;
        assert_eq!(
            apply(&SPEC, &mut r, &mut sel, Op::Add, &fields("", "x")),
            Ok(false)
        );
        assert_eq!(
            apply(&SPEC, &mut r, &mut sel, Op::Insert, &fields("", "x")),
            Ok(false)
        );
        assert_eq!(r.len(), 1);
        let mut r = rows(&["a", "b", "c", "d"]);
        assert_eq!(
            apply(&SPEC, &mut r, &mut sel, Op::Add, &fields("e", "")),
            Err(Refused::Full)
        );
        assert_eq!(
            apply(&SPEC, &mut r, &mut sel, Op::Insert, &fields("e", "")),
            Err(Refused::Full)
        );
        // Update replaces and so works on a full list.
        let mut sel = Some(3);
        assert_eq!(
            apply(&SPEC, &mut r, &mut sel, Op::Update, &fields("z", "Z")),
            Ok(true)
        );
        assert_eq!(firsts(&r), ["a", "b", "c", "z"]);
    }

    #[test]
    fn update_replaces_the_selected_row_or_adds_the_first() {
        let mut r = rows(&["a", "b"]);
        let mut sel = Some(0);
        apply(&SPEC, &mut r, &mut sel, Op::Update, &fields("q", "Q")).unwrap();
        assert_eq!(r[0], fields("q", "Q"));
        assert_eq!(r.len(), 2);
        // No selection in a list that has rows: nothing.
        let mut sel = None;
        assert_eq!(
            apply(&SPEC, &mut r, &mut sel, Op::Update, &fields("n", "")),
            Ok(false)
        );
        // No rows at all: the button adds one.
        let mut r = Vec::new();
        apply(&SPEC, &mut r, &mut sel, Op::Update, &fields("n", "N")).unwrap();
        assert_eq!(firsts(&r), ["n"]);
        assert_eq!(sel, Some(0));
        // An empty first column does not update.
        assert_eq!(
            apply(&SPEC, &mut r, &mut sel, Op::Update, &fields("", "N")),
            Ok(false)
        );
        assert_eq!(firsts(&r), ["n"]);
    }

    #[test]
    fn delete_selects_the_row_that_takes_its_place() {
        let mut r = rows(&["a", "b", "c"]);
        let mut sel = Some(1);
        apply(&SPEC, &mut r, &mut sel, Op::Delete, &[]).unwrap();
        assert_eq!(firsts(&r), ["a", "c"]);
        assert_eq!(sel, Some(1));
        apply(&SPEC, &mut r, &mut sel, Op::Delete, &[]).unwrap();
        assert_eq!(firsts(&r), ["a"]);
        // The last row went: nothing is selected, and the button then does nothing.
        assert_eq!(sel, None);
        assert_eq!(apply(&SPEC, &mut r, &mut sel, Op::Delete, &[]), Ok(false));
        assert_eq!(firsts(&r), ["a"]);
    }

    #[test]
    fn rows_move_to_the_top_up_down_and_last() {
        let mut r = rows(&["a", "b", "c", "d"]);
        let mut sel = Some(2);
        apply(&SPEC, &mut r, &mut sel, Op::Top, &[]).unwrap();
        assert_eq!(firsts(&r), ["c", "a", "b", "d"]);
        assert_eq!(sel, Some(0));
        // Already at the top: nothing.
        assert_eq!(apply(&SPEC, &mut r, &mut sel, Op::Up, &[]), Ok(false));
        assert_eq!(apply(&SPEC, &mut r, &mut sel, Op::Top, &[]), Ok(false));
        apply(&SPEC, &mut r, &mut sel, Op::Down, &[]).unwrap();
        assert_eq!(firsts(&r), ["a", "c", "b", "d"]);
        assert_eq!(sel, Some(1));
        apply(&SPEC, &mut r, &mut sel, Op::Last, &[]).unwrap();
        assert_eq!(firsts(&r), ["a", "b", "d", "c"]);
        assert_eq!(sel, Some(3));
        assert_eq!(apply(&SPEC, &mut r, &mut sel, Op::Down, &[]), Ok(false));
        apply(&SPEC, &mut r, &mut sel, Op::Up, &[]).unwrap();
        assert_eq!(firsts(&r), ["a", "b", "c", "d"]);
        // A move keeps the texts of the row.
        assert_eq!(r[2], fields("c", "C"));
    }

    #[test]
    fn moving_without_a_selection_works_from_the_first_row() {
        // `MoveListViewItem_FILENAME` holds an index of -1 at 0: only 最終 does something.
        let mut r = rows(&["a", "b", "c"]);
        let mut sel = None;
        for op in [Op::Top, Op::Up, Op::Down] {
            assert_eq!(apply(&SPEC, &mut r, &mut sel, op, &[]), Ok(false));
        }
        apply(&SPEC, &mut r, &mut sel, Op::Last, &[]).unwrap();
        assert_eq!(firsts(&r), ["b", "c", "a"]);
        assert_eq!(sel, Some(2));
        let mut r = Vec::new();
        assert_eq!(apply(&SPEC, &mut r, &mut sel, Op::Last, &[]), Ok(false));
    }

    #[test]
    fn rows_come_from_the_file_or_the_defaults() {
        let from_defaults = SPEC.load(&Profile::new());
        assert_eq!(from_defaults.len(), 2);
        assert_eq!(from_defaults[1], fields("%B%\\", "b"));
        let file = profile(
            "[Nickname]\r\nArrNum=3\r\nFrom00=C:\\デスクトップ\\\r\nTo00=机\r\nFrom01=\r\nTo01=gone\r\nFrom02=x\r\n",
        );
        let loaded = SPEC.load(&file);
        // A row with an empty first column is dropped; a missing key is empty.
        assert_eq!(
            loaded,
            vec![fields("C:\\デスクトップ\\", "机"), fields("x", "")]
        );
        // The count is held to the limit.
        let big = profile("[Nickname]\r\nArrNum=99\r\nFrom00=a\r\nFrom03=d\r\nFrom05=f\r\n");
        // Only the first 4 rows (`max_rows`) are read, so row 5 is out. A key the file lacks is
        // what the page starts with (the default of that row, else empty), as in Sakura, whose
        // settings keep their default when the file does not say; row 2 has nothing.
        assert_eq!(
            SPEC.load(&big),
            vec![fields("a", "a"), fields("%B%\\", "b"), fields("d", "")]
        );
        let none = profile("[Nickname]\r\nArrNum=0\r\n");
        assert!(SPEC.load(&none).is_empty());
    }

    #[test]
    fn storing_writes_the_keys_and_removes_the_ones_that_are_gone() {
        let mut p = profile(
            "[Other]\r\nx=1\r\n[Nickname]\r\nbShortPath=0\r\nArrNum=3\r\nFrom00=a\r\nTo00=A\r\nFrom01=b\r\nTo01=B\r\nFrom02=c\r\nTo02=C\r\n",
        );
        let kept = vec![fields("a", "A"), fields("n", "N")];
        assert!(SPEC.store(&kept, &mut p).unwrap());
        assert_eq!(p.get("Nickname", "ArrNum"), Some("2"));
        assert_eq!(p.get("Nickname", "From01"), Some("n"));
        assert_eq!(p.get("Nickname", "To01"), Some("N"));
        assert_eq!(p.get("Nickname", "From02"), None);
        assert_eq!(p.get("Nickname", "To02"), None);
        // The rest of the file stays.
        assert_eq!(p.get("Nickname", "bShortPath"), Some("0"));
        assert_eq!(p.get("Other", "x"), Some("1"));
        // The same rows again change nothing.
        assert!(!SPEC.store(&kept, &mut p).unwrap());
        // Rows with an empty first column are not written.
        let mut p = Profile::new();
        SPEC.store(&[fields("", "x"), fields("k", "")], &mut p)
            .unwrap();
        assert_eq!(p.get("Nickname", "ArrNum"), Some("1"));
        assert_eq!(p.get("Nickname", "From00"), Some("k"));
        assert_eq!(p.get("Nickname", "To00"), Some(""));
    }

    #[test]
    fn the_key_of_a_row_has_the_digits_the_page_says() {
        assert_eq!(numbered_key("From", 3, 2), "From03");
        assert_eq!(numbered_key("Key", 12, 3), "Key012");
        assert_eq!(numbered_key("K", 7, 0), "K7");
    }
}
