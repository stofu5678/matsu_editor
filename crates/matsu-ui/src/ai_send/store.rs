//! What 「次から聞かない」 changes in `[AI]` of the settings file (ADR-0030, ADR-0035).
//!
//! The file is changed the way the one-liners' history is (`crate::oneliner::store`): under the lock that every
//! writer takes (`crate::history::write_settings`), the section is read again from the file and changed there, so
//! what another window saved at the same moment is not lost. The settings the windows hold become what the file has
//! then (or, without a file on disk, what the change makes of them, for this session).

use gpui::{App, BorrowAppContext as _};
use matsu_config::settings::ai::AiDestinations;
use matsu_config::settings::oneliner::OneLinerError;

use crate::settings::UserSettings;

/// Turns `AskBeforeSend` off for destination `index`, if it still is the one whose command is `command` (the settings
/// may have changed while the window was open) and still asks. Returns whether it changed anything.
fn turn_off(table: &mut AiDestinations, index: usize, command: &str) -> bool {
    let Some(slot) = table
        .slots
        .get_mut(index)
        .filter(|slot| slot.ask_before_send && slot.command == command)
    else {
        return false;
    };
    slot.ask_before_send = false;
    true
}

/// 「次から聞かない」 of the confirmation window: writes `AskBeforeSend[nnn]=0` for destination `index`.
pub(crate) fn stop_asking(cx: &mut App, index: usize, command: &str) {
    let mut made: Option<AiDestinations> = None;
    let mut read_file = false;
    // a setting that cannot be written is kept for this session; the run itself does not depend on it
    crate::history::write_settings(cx, |profile| {
        read_file = true;
        let mut table = AiDestinations::from_profile(profile);
        if !turn_off(&mut table, index, command) {
            return Ok(false);
        }
        match table.store(profile) {
            Ok(changed) => {
                made = Some(table);
                Ok(changed)
            }
            Err(OneLinerError::Profile(e)) => Err(e),
            // values that were read from the file are within the limits, so this does not happen
            Err(_) => Ok(false),
        }
    });
    let table = match made {
        Some(table) => table,
        // the file was read and has nothing to change: the windows' table is not asked
        None if read_file => return,
        // no file to write: the change is for this session
        None => {
            let mut table = super::table(cx);
            if !turn_off(&mut table, index, command) {
                return;
            }
            table
        }
    };
    if cx.try_global::<UserSettings>().is_none() {
        cx.set_global(UserSettings::default());
    }
    cx.update_global::<UserSettings, _>(|user, _| user.set_ai_destinations(table));
}
