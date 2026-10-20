//! The file chooser portal (`org.freedesktop.portal.FileChooser`): `OpenFile` and `SaveFile` with
//! `current_folder` and `current_name`. The portal shows the dialog of the desktop (GTK or Qt)
//! without this program linking GTK, which is LGPL.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use raw_window_handle::RawWindowHandle;
use zbus::zvariant::Value;

use super::{
    Chosen, Error, Extra, ExtraValue, FileFilter, Kind, Request, extension_for_filter,
    path_from_file_uri, with_default_extension,
};
use crate::portal::{Answer, Failure, Portal};

const INTERFACE: &str = "org.freedesktop.portal.FileChooser";

/// The window the dialog belongs to, as the portal names it (`parent_window`): `x11:<hex id>`,
/// or empty when there is none or it cannot be told (a Wayland window needs an exported handle).
#[derive(Clone, Debug, Default)]
pub struct Owner(String);

impl Owner {
    pub fn from_handle(handle: RawWindowHandle) -> Self {
        Self(match handle {
            RawWindowHandle::Xlib(handle) => format!("x11:{:x}", handle.window),
            RawWindowHandle::Xcb(handle) => format!("x11:{:x}", handle.window.get()),
            _ => String::new(),
        })
    }
}

/// A filter as the portal takes it: `(name, [(0, glob)])` (`(sa(us))`; 0 is a glob, 1 a MIME type).
/// `*.*` means every file for Windows, but a glob of the portal needs a dot in the name for it, so
/// it is sent as `*`.
fn filter_tuple(filter: &FileFilter) -> (String, Vec<(u32, String)>) {
    let globs = filter
        .patterns
        .iter()
        .map(|p| {
            (
                0,
                if p == "*.*" {
                    "*".to_owned()
                } else {
                    p.clone()
                },
            )
        })
        .collect();
    (filter.name.clone(), globs)
}

/// `current_folder` is a byte string that ends with NUL (`ay`).
fn folder_bytes(folder: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    let mut bytes = folder.as_os_str().as_bytes().to_vec();
    bytes.push(0);
    bytes
}

/// A choice as the portal takes it: `(id, label, [(option id, option label)], initial option id)`
/// (`(ssa(ss)s)`). A list has its items as options; a check box has none and is `"true"` or
/// `"false"`. The id is the index into [`Request::extras`].
type Choice = (String, String, Vec<(String, String)>, String);

fn choices_of(extras: &[Extra]) -> Vec<Choice> {
    extras
        .iter()
        .enumerate()
        .filter_map(|(i, extra)| {
            Some(match extra {
                Extra::Combo {
                    label,
                    items,
                    selected,
                } => (
                    i.to_string(),
                    label.clone(),
                    items
                        .iter()
                        .enumerate()
                        .map(|(n, item)| (n.to_string(), item.clone()))
                        .collect(),
                    selected.to_string(),
                ),
                // The portal cannot disable a check box: `enabled` is for the dialog of Windows.
                Extra::Check { label, checked, .. } => (
                    i.to_string(),
                    label.clone(),
                    Vec::new(),
                    checked.to_string(),
                ),
                // The portal cannot change the name or the folder when a choice is made.
                Extra::Recent { .. } => return None,
            })
        })
        .collect()
}

/// What the user chose, from the `choices` of the answer (`a(ss)`: id and option id). A choice the
/// answer leaves out keeps the value it started with; an answer without `choices`, or
/// with none of ours, gives none.
fn values_of(
    extras: &[Extra],
    results: &HashMap<String, zbus::zvariant::OwnedValue>,
) -> Vec<ExtraValue> {
    // A portal that ignores `choices` showed no fields and answers without them: no values, so
    // that the caller asks another way.
    let Some(answered) = results
        .get("choices")
        .and_then(|value| <Vec<(String, String)>>::try_from(value.try_clone().ok()?).ok())
    else {
        return Vec::new();
    };
    // An empty array, or one that answers none of our ids, is a portal that showed no fields too.
    // If only some ids come back, the others keep their starting value: the user could still
    // have seen and left them alone, and dropping the whole answer would ask twice.
    if !(0..extras.len()).any(|i| answered.iter().any(|(id, _)| *id == i.to_string())) {
        return Vec::new();
    }
    extras
        .iter()
        .enumerate()
        .map(|(i, extra)| {
            let id = i.to_string();
            let chosen = answered
                .iter()
                .rev()
                .find(|(answer_id, _)| *answer_id == id)
                .map(|(_, option)| option.as_str());
            match extra {
                Extra::Combo {
                    items, selected, ..
                } => ExtraValue::Combo(
                    chosen
                        .and_then(|option| option.parse::<usize>().ok())
                        .filter(|n| *n < items.len())
                        .unwrap_or(*selected),
                ),
                Extra::Check { checked, .. } => {
                    ExtraValue::Check(chosen.map_or(*checked, |option| option == "true"))
                }
                Extra::Recent { .. } => ExtraValue::Nothing,
            }
        })
        .collect()
}

pub(super) async fn show(owner: Owner, request: Request) -> Result<Option<Chosen>, Error> {
    let portal = Portal::new(INTERFACE).await.map_err(Error::Unavailable)?;
    let method = match request.kind {
        Kind::SaveFile => "SaveFile",
        Kind::OpenFile { .. } | Kind::Folder => "OpenFile",
    };
    let answer = portal
        .request(method, |mut options| {
            options.insert("modal", true.into());
            match request.kind {
                Kind::OpenFile { multiple } => {
                    options.insert("multiple", multiple.into());
                }
                Kind::Folder => {
                    options.insert("directory", true.into());
                }
                Kind::SaveFile => {}
            }
            // The open dialog has the list of file types too (`DoModal_GetOpenFileName`).
            if request.kind != Kind::Folder && !request.filters.is_empty() {
                let filters: Vec<_> = request.filters.iter().map(filter_tuple).collect();
                options.insert("filters", Value::from(filters));
                if let Some(current) = request.filters.get(request.selected_filter) {
                    options.insert("current_filter", Value::from(filter_tuple(current)));
                }
            }
            if request.kind == Kind::SaveFile && !request.extras.is_empty() {
                options.insert("choices", Value::from(choices_of(&request.extras)));
            }
            if let Some(folder) = request.start.as_deref() {
                options.insert("current_folder", Value::from(folder_bytes(folder)));
            }
            if let Some(name) = &request.name
                && request.kind == Kind::SaveFile
            {
                options.insert("current_name", Value::from(name.clone()));
            }
            (owner.0.as_str(), request.title.as_str(), options)
        })
        .await;
    match answer {
        // No portal that has the file chooser: the caller uses another dialog.
        Err(Failure::Call(why)) => Err(Error::Unavailable(why)),
        Err(Failure::Answer(why)) => Err(Error::Failed(why)),
        Ok(Answer::Cancelled) => Ok(None),
        Ok(Answer::Failed) => Err(Error::Failed("ポータルが失敗を返した".to_owned())),
        Ok(Answer::Done(results)) => Ok(paths_of(&results).map(|paths| {
            // the portal does not add the default extension, as the dialog of Windows does
            let paths = match (&request.default_extension, request.kind) {
                (Some(fallback), Kind::SaveFile) => {
                    let extension =
                        extension_for_filter(chosen_filter(&request, &results), fallback);
                    paths
                        .into_iter()
                        .map(|path| with_default_extension(path, &extension))
                        .collect()
                }
                _ => paths,
            };
            let extras = if request.kind == Kind::SaveFile {
                values_of(&request.extras, &results)
            } else {
                Vec::new()
            };
            Chosen { paths, extras }
        })),
    }
}

/// The filter the user chose: the `current_filter` of the answer (found by its name), else the one
/// the dialog showed first.
fn chosen_filter<'a>(
    request: &'a Request,
    results: &HashMap<String, zbus::zvariant::OwnedValue>,
) -> Option<&'a FileFilter> {
    let name = results
        .get("current_filter")
        .and_then(|value| <(String, Vec<(u32, String)>)>::try_from(value.try_clone().ok()?).ok())
        .map(|(name, _)| name);
    match name {
        Some(name) => request.filters.iter().find(|f| f.name == name),
        None => request.filters.get(request.selected_filter),
    }
}

/// The `uris` of the answer as paths. An answer without one is a cancel.
fn paths_of(results: &HashMap<String, zbus::zvariant::OwnedValue>) -> Option<Vec<PathBuf>> {
    let uris = results
        .get("uris")
        .and_then(|uris| <Vec<String>>::try_from(uris.try_clone().ok()?).ok())?;
    let paths: Vec<PathBuf> = uris
        .iter()
        .filter_map(|uri| path_from_file_uri(uri))
        .collect();
    (!paths.is_empty()).then_some(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filter_is_a_name_and_globs_for_the_portal() {
        let filter = FileFilter {
            name: "ユーザー指定".to_owned(),
            patterns: vec!["*.cpp".to_owned(), "*.*".to_owned()],
        };
        // `current_filter` is `(sa(us))`, and `filters` is a list of them: `a(sa(us))`
        let value = Value::from(filter_tuple(&filter));
        assert_eq!(value.value_signature().to_string(), "(sa(us))");
        assert_eq!(
            filter_tuple(&filter).1,
            [(0, "*.cpp".to_owned()), (0, "*".to_owned())]
        );
        let list = Value::from(vec![filter_tuple(&filter)]);
        assert_eq!(list.value_signature().to_string(), "a(sa(us))");
    }

    #[test]
    fn the_extension_follows_the_filter_the_portal_says_was_chosen() {
        let filter = |name: &str, patterns: &[&str]| FileFilter {
            name: name.to_owned(),
            patterns: patterns.iter().map(|p| (*p).to_owned()).collect(),
        };
        let request = Request {
            kind: Kind::SaveFile,
            start: None,
            name: None,
            title: String::new(),
            filters: vec![
                filter("ユーザー指定", &["*.c", "*.h"]),
                filter("すべてのファイル", &["*.*"]),
            ],
            selected_filter: 0,
            default_extension: Some("txt".to_owned()),
            extras: Vec::new(),
            follow: None,
        };
        let extension = |chosen: Option<&FileFilter>| {
            let mut results = HashMap::new();
            if let Some(f) = chosen {
                results.insert(
                    "current_filter".to_owned(),
                    Value::from(filter_tuple(f)).try_into().unwrap(),
                );
            }
            extension_for_filter(chosen_filter(&request, &results), "txt")
        };
        assert_eq!(extension(Some(&request.filters[0])), "c");
        assert_eq!(extension(Some(&request.filters[1])), "txt");
        // no `current_filter`: the one shown first
        assert_eq!(extension(None), "c");
        let path = with_default_extension(PathBuf::from("/nonexistent-ume/bar"), "c");
        assert_eq!(path, PathBuf::from("/nonexistent-ume/bar.c"));
    }

    fn extras() -> Vec<Extra> {
        vec![
            Extra::Combo {
                label: "文字コードセット".to_owned(),
                items: vec!["SJIS".to_owned(), "UTF-8".to_owned()],
                selected: 1,
            },
            Extra::Check {
                label: "BOM".to_owned(),
                checked: true,
                enabled: true,
            },
        ]
    }

    #[test]
    fn the_fields_are_choices_of_the_portal() {
        let choices = choices_of(&extras());
        // `choices` is `a(ssa(ss)s)`
        let value = Value::from(choices.clone());
        assert_eq!(value.value_signature().to_string(), "a(ssa(ss)s)");
        assert_eq!(choices[0].0, "0");
        assert_eq!(choices[0].2.len(), 2);
        assert_eq!(choices[0].3, "1");
        // a check box has no options and starts as "true" or "false"
        assert_eq!(
            choices[1],
            (
                "1".to_owned(),
                "BOM".to_owned(),
                Vec::new(),
                "true".to_owned()
            )
        );
    }

    #[test]
    fn the_choices_of_the_answer_are_the_values() {
        let extras = extras();
        let answer = |pairs: &[(&str, &str)]| {
            let mut results = HashMap::new();
            let pairs: Vec<(String, String)> = pairs
                .iter()
                .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
                .collect();
            results.insert("choices".to_owned(), Value::from(pairs).try_into().unwrap());
            results
        };
        assert_eq!(
            values_of(&extras, &answer(&[("0", "0"), ("1", "false")])),
            [ExtraValue::Combo(0), ExtraValue::Check(false)]
        );
        // a choice the answer leaves out, or an option that is not one, keeps its start
        assert_eq!(
            values_of(&extras, &answer(&[("0", "7")])),
            [ExtraValue::Combo(1), ExtraValue::Check(true)]
        );
        assert_eq!(values_of(&extras, &HashMap::new()), []);
        // an empty array, or ids that are not ours: the portal showed no fields
        assert_eq!(values_of(&extras, &answer(&[])), []);
        assert_eq!(values_of(&extras, &answer(&[("9", "0")])), []);
        // only some of the ids: the others keep their start
        assert_eq!(
            values_of(&extras, &answer(&[("1", "false")])),
            [ExtraValue::Combo(1), ExtraValue::Check(false)]
        );
    }

    #[test]
    fn the_folder_ends_with_nul() {
        assert_eq!(
            folder_bytes(Path::new("/a/メモ")),
            b"/a/\xe3\x83\xa1\xe3\x83\xa2\0"
        );
    }

    #[test]
    fn the_uris_of_the_answer_are_paths() {
        let mut results = HashMap::new();
        let uris = vec![
            "file:///home/a/b.txt".to_owned(),
            "file:///home/a/c%20d.txt".to_owned(),
        ];
        results.insert("uris".to_owned(), Value::from(uris).try_into().unwrap());
        assert_eq!(
            paths_of(&results),
            Some(vec![
                PathBuf::from("/home/a/b.txt"),
                PathBuf::from("/home/a/c d.txt")
            ])
        );
        assert_eq!(paths_of(&HashMap::new()), None);
    }

    #[test]
    fn an_x11_window_is_named_by_its_id() {
        let mut xlib = raw_window_handle::XlibWindowHandle::new(0x2a00007);
        xlib.visual_id = 0;
        assert_eq!(
            Owner::from_handle(RawWindowHandle::Xlib(xlib)).0,
            "x11:2a00007"
        );
        assert_eq!(
            Owner::from_handle(RawWindowHandle::Web(
                raw_window_handle::WebWindowHandle::new(1)
            ))
            .0,
            ""
        );
    }
}
