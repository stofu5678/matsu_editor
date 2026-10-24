//! `session.ini`: which documents are kept, and in which window and tab (ADR-0022, section 3).
//!
//! A window is a group (ADR-0015). The ids are 32 hex digits (see [`is_valid_id`]); anything else in the
//! file is ignored, so a damaged `session.ini` can never name a file outside the folder.

use crate::profile::{Profile, ProfileError};

const SECTION: &str = "Session";

/// One window: the ids of its documents in tab order, and the tab that is in front.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionWindow {
    pub docs: Vec<String>,
    /// The index in `docs` of the front tab.
    pub active: usize,
}

/// The windows that had unsaved documents.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Session {
    pub windows: Vec<SessionWindow>,
    /// The index in `windows` of the window that was in front.
    pub active_window: usize,
}

/// Whether `id` is a name this module makes: 32 lower-case hex digits.
pub fn is_valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl Session {
    /// Every id the session lists, in order.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.windows
            .iter()
            .flat_map(|w| w.docs.iter().map(String::as_str))
    }

    /// Whether the session lists `id`.
    pub fn contains(&self, id: &str) -> bool {
        self.ids().any(|i| i == id)
    }

    /// The session without `id` (a window that has no document left goes away).
    pub fn without(&self, id: &str) -> Session {
        let mut windows = Vec::new();
        let mut active_window = 0;
        for (index, w) in self.windows.iter().enumerate() {
            let docs: Vec<String> = w.docs.iter().filter(|d| *d != id).cloned().collect();
            if docs.is_empty() {
                continue;
            }
            let active = w.docs[..w.active.min(w.docs.len())]
                .iter()
                .filter(|d| *d != id)
                .count()
                .min(docs.len() - 1);
            if index == self.active_window {
                active_window = windows.len();
            }
            windows.push(SessionWindow { docs, active });
        }
        Session {
            windows,
            active_window,
        }
    }

    pub(crate) fn to_bytes(&self) -> Result<Vec<u8>, ProfileError> {
        let mut p = Profile::new();
        p.set(SECTION, "Windows", &self.windows.len().to_string())?;
        p.set(SECTION, "ActiveWindow", &self.active_window.to_string())?;
        for (i, w) in self.windows.iter().enumerate() {
            p.set(SECTION, &format!("Window{i}.Docs"), &w.docs.join(","))?;
            p.set(SECTION, &format!("Window{i}.Active"), &w.active.to_string())?;
        }
        Ok(p.to_bytes())
    }

    /// Reads `session.ini`. Ids that are not valid are dropped; a window with no valid id is dropped.
    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        let p = Profile::from_bytes(bytes);
        let count: usize = p
            .get(SECTION, "Windows")
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        let mut windows = Vec::new();
        let mut active_window = 0;
        let wanted: usize = p
            .get(SECTION, "ActiveWindow")
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        // A damaged count must not make a long loop.
        for i in 0..count.min(10_000) {
            let docs: Vec<String> = p
                .get(SECTION, &format!("Window{i}.Docs"))
                .unwrap_or("")
                .split(',')
                .map(str::trim)
                .filter(|d| is_valid_id(d))
                .map(str::to_owned)
                .collect();
            if docs.is_empty() {
                continue;
            }
            let active: usize = p
                .get(SECTION, &format!("Window{i}.Active"))
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0);
            if i == wanted {
                active_window = windows.len();
            }
            windows.push(SessionWindow {
                active: active.min(docs.len() - 1),
                docs,
            });
        }
        Session {
            windows,
            active_window,
        }
    }
}
