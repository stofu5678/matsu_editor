//! The destinations of "send to a generative AI" (ADR-0035, decision 5): the section `[AI]` of `matsu.ini`.
//!
//! Sending to an AI is separate from the one-liners ([`super::oneliner`]): a destination is a name and one
//! command line, like a one-liner, and the section has the same numbering, so a destination `n` can be called
//! as one function. The section is MATSU's own (ADR-0024, decision 7).
//!
//! ```text
//! [AI]
//! Name[000]=英訳
//! Command[000]=claude -p --no-session-persistence --tools "" --permission-mode dontAsk
//! Output[000]=1
//! AskBeforeSend[000]=1
//! ```
//!
//! * `Name[nnn]`, `Command[nnn]`, `Output[nnn]`, `AskBeforeSend[nnn]` for `nnn` in `000..=049` ([`MAX_AI`]).
//!   A slot whose `Command` is empty, only spaces or absent is not registered: it reads as an empty slot, and
//!   writing it removes its keys.
//! * `Output`: 0 replace, 1 next line, 2 the log tab only ([`AiOutput`]). Unlike a one-liner, the default and
//!   the reading of any other value (absent, not a number, another number) is 1.
//! * `AskBeforeSend`: 0 sends without the confirmation window; 1 is the default and the reading of anything
//!   else.
//! * Keys of the section that this program does not know stay in the file when it is written back
//!   ([`Profile`] keeps every line it does not set).
//! * The limits of a name and a command, and the one-line rule, are the one-liners' (see
//!   [`super::oneliner`]); the errors are [`OneLinerError`] too.
//!
//! [`migrate`] moves the slots that the old `[OneLiner]` marked with `AI[nnn]=1` into this section.

use super::oneliner::{
    COMMAND_MAX_UNITS, NAME_MAX_UNITS, OneLiner, OneLinerError, OneLiners, OutputTo,
    SECTION as ONELINER, check, cut_name, has_break, key, units,
};
use crate::profile::{Profile, ProfileError};

/// The section of the file.
pub const SECTION: &str = "AI";
/// How many destinations can be registered (the same as the one-liners).
pub const MAX_AI: usize = 50;

/// Where the result of a destination goes (`Output`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AiOutput {
    /// 0: replaces the target text.
    Replace,
    /// 1: goes into the lines after the target.
    #[default]
    NextLine,
    /// 2: only the log tab; the document stays.
    LogOnly,
}

impl AiOutput {
    /// The value of `Output`; any other number is [`AiOutput::NextLine`].
    pub fn from_number(n: i32) -> Self {
        match n {
            0 => Self::Replace,
            2 => Self::LogOnly,
            _ => Self::NextLine,
        }
    }

    /// The value written to `Output`.
    pub fn number(self) -> i32 {
        match self {
            Self::Replace => 0,
            Self::NextLine => 1,
            Self::LogOnly => 2,
        }
    }
}

impl From<OutputTo> for AiOutput {
    /// What the one-liner's `Output` means here: the output window was the place of the log, so it is the log.
    fn from(output: OutputTo) -> Self {
        match output {
            OutputTo::Replace => Self::Replace,
            OutputTo::NextLine => Self::NextLine,
            OutputTo::OutputWindow => Self::LogOnly,
        }
    }
}

/// One destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiDestination {
    /// The name shown in menus.
    pub name: String,
    /// The shell command line: one line.
    pub command: String,
    pub output: AiOutput,
    /// `AskBeforeSend`: show the confirmation window first.
    pub ask_before_send: bool,
}

impl Default for AiDestination {
    fn default() -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            output: AiOutput::default(),
            ask_before_send: true,
        }
    }
}

impl AiDestination {
    /// A destination with a command can be used.
    pub fn is_registered(&self) -> bool {
        !self.command.trim().is_empty()
    }

    /// The name, or the command when there is no name.
    pub fn title(&self) -> &str {
        if self.name.is_empty() {
            &self.command
        } else {
            &self.name
        }
    }
}

/// The 50 slots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiDestinations {
    pub slots: Vec<AiDestination>,
}

impl Default for AiDestinations {
    fn default() -> Self {
        Self {
            slots: vec![AiDestination::default(); MAX_AI],
        }
    }
}

/// A 0/1 key of `[AI]`; anything else is `default`.
fn flag(profile: &Profile, name: &str, default: bool) -> bool {
    match profile.get_i32(SECTION, name) {
        Some(0) => false,
        Some(1) => true,
        _ => default,
    }
}

/// Whether `[AI]` has a usable command in slot `index` (the slot is taken, even when its other keys are bad).
fn is_taken(profile: &Profile, index: usize) -> bool {
    profile
        .get(SECTION, &key("Command", index))
        .is_some_and(|c| !c.trim().is_empty())
}

impl AiDestinations {
    /// Reads the section; what is missing or cannot be used is an empty slot (see the module).
    pub fn from_profile(profile: &Profile) -> Self {
        let mut table = Self::default();
        for (i, slot) in table.slots.iter_mut().enumerate() {
            let Some(command) = profile.get(SECTION, &key("Command", i)) else {
                continue;
            };
            if command.trim().is_empty()
                || has_break(command)
                || units(command) >= COMMAND_MAX_UNITS
            {
                continue;
            }
            command.clone_into(&mut slot.command);
            slot.name = cut_name(profile.get(SECTION, &key("Name", i)).unwrap_or("").trim());
            slot.output = profile
                .get_i32(SECTION, &key("Output", i))
                .map_or_else(AiOutput::default, AiOutput::from_number);
            slot.ask_before_send = flag(profile, &key("AskBeforeSend", i), true);
        }
        table
    }

    /// The destination `index`, if it is in the table.
    pub fn get(&self, index: usize) -> Option<&AiDestination> {
        self.slots.get(index)
    }

    /// Checks every value before anything is written.
    fn validate(&self) -> Result<(), OneLinerError> {
        for (i, slot) in self.slots.iter().take(MAX_AI).enumerate() {
            if slot.is_registered() {
                check(&key("Name", i), &slot.name, NAME_MAX_UNITS)?;
                check(&key("Command", i), &slot.command, COMMAND_MAX_UNITS)?;
            }
        }
        Ok(())
    }

    /// Writes the section. A slot with no command is not written (its keys are removed). A value with a line
    /// break or over its limit is refused before the file is touched. Returns whether the file changed; a
    /// value the file already has is left as written.
    pub fn store(&self, profile: &mut Profile) -> Result<bool, OneLinerError> {
        self.validate()?;
        let mut changed = false;
        let mut gone = Vec::new();
        for i in 0..MAX_AI {
            match self.slots.get(i).filter(|s| s.is_registered()) {
                Some(slot) => changed |= write_slot(profile, i, slot)?,
                None => {
                    gone.extend(["Name", "Command", "Output", "AskBeforeSend"].map(|p| key(p, i)));
                }
            }
        }
        for name in gone {
            changed |= profile.remove(SECTION, &name);
        }
        Ok(changed)
    }
}

/// Writes the four keys of slot `index`; returns whether the file changed. Does not check the values.
fn write_slot(
    profile: &mut Profile,
    index: usize,
    slot: &AiDestination,
) -> Result<bool, ProfileError> {
    let output = slot.output.number().to_string();
    let ask = if slot.ask_before_send { "1" } else { "0" };
    let mut changed = false;
    for (prefix, value) in [
        ("Name", slot.name.as_str()),
        ("Command", slot.command.as_str()),
        ("Output", output.as_str()),
        ("AskBeforeSend", ask),
    ] {
        let name = key(prefix, index);
        if profile.get(SECTION, &name) != Some(value) {
            profile.set(SECTION, &name, value)?;
            changed = true;
        }
    }
    Ok(changed)
}

/// What [`migrate`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiMigration {
    /// No registered `[OneLiner]` slot has `AI=1`; nothing changed.
    NothingToMove,
    /// `count` slots moved to `[AI]` and their keys were removed from `[OneLiner]`. The caller saves the
    /// profile once and tells the user once.
    Moved { count: usize },
    /// `[AI]` has room for fewer than the `count` slots to move; **nothing changed**. The slots stay marked
    /// ([`OneLiner::is_ai_marked`]) and must not run as one-liners. A later run moves them once there is room.
    Blocked { count: usize },
}

impl AiMigration {
    /// Whether the profile changed (and has to be saved).
    pub fn changed(self) -> bool {
        matches!(self, Self::Moved { .. })
    }
}

/// ADR-0035, decision 6: moves the registered `[OneLiner]` slots with `AI[nnn]=1` into `[AI]`.
///
/// 1. Counts first, changing nothing: the slots to move against the free slots of `[AI]` (a slot with a usable
///    command is taken). When they do not fit, returns [`AiMigration::Blocked`] and leaves the profile alone:
///    no part is moved, because MATSU does not decide which one goes first.
/// 2. When they fit, each slot goes to the same number in `[AI]` (the first free one if that is taken), with
///    its name, command, `Output` (the output window becomes the log tab) and `AskBeforeSend`, and the keys
///    `Name`, `Command`, `Output`, `AI`, `AskBeforeSend` of that number are removed from `[OneLiner]`. Both
///    happen in the profile in memory, so one save writes both.
///
/// Only the keys of the moved slots are touched: other slots, the history and unknown keys stay as they are.
/// A second run finds nothing marked and does nothing. The values of the moved slots were read by
/// [`OneLiners::from_profile`], so they are within the limits and the writing cannot be refused; if the file
/// refuses a key anyway, the error is returned and the profile may be half changed, so the caller drops it
/// (it works on the profile it loaded and saves only on `Ok`).
pub fn migrate(profile: &mut Profile) -> Result<AiMigration, ProfileError> {
    let one = OneLiners::from_profile(profile);
    let marked = one.ai_marked();
    if marked.is_empty() {
        return Ok(AiMigration::NothingToMove);
    }
    let mut free: Vec<usize> = (0..MAX_AI).filter(|&i| !is_taken(profile, i)).collect();
    if marked.len() > free.len() {
        return Ok(AiMigration::Blocked {
            count: marked.len(),
        });
    }
    for &from in &marked {
        let slot: &OneLiner = &one.slots[from];
        // Same number if free, else the first free one.
        let to = match free.iter().position(|&i| i == from) {
            Some(at) => free.remove(at),
            None => free.remove(0),
        };
        let dest = AiDestination {
            name: slot.name.clone(),
            command: slot.command.clone(),
            output: slot.output.into(),
            ask_before_send: slot.ask_before_send,
        };
        write_slot(profile, to, &dest)?;
        for prefix in ["Name", "Command", "Output", "AI", "AskBeforeSend"] {
            profile.remove(ONELINER, &key(prefix, from));
        }
    }
    Ok(AiMigration::Moved {
        count: marked.len(),
    })
}

#[cfg(test)]
mod tests;
