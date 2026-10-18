//! `CPassiveTimer` (`sakura_core/agent/CAutoSaveAgent.cpp`): the interval of the automatic save.
//!
//! Something else calls it at a steady pace; it says whether the interval has passed since the
//! last time (or since it was turned on).

use std::time::{Duration, Instant};

/// The longest interval the setting allows, in minutes (`CPassiveTimer::SetInterval`:
/// `INT_MAX` milliseconds).
const MAX_MINUTES: i32 = 35791;

#[derive(Debug, Default)]
pub(crate) struct PassiveTimer {
    /// The time of the last action, while the timer is on.
    last: Option<Instant>,
}

impl PassiveTimer {
    /// `CPassiveTimer::Enable` and `CheckAction` together: whether the interval of `minutes` has
    /// passed at `now`. Turning the timer on starts the measuring (no action at once); an interval
    /// below 1 minute counts as 1 (`SetInterval`).
    pub fn check(&mut self, now: Instant, enabled: bool, minutes: i32) -> bool {
        if !enabled {
            self.last = None;
            return false;
        }
        let Some(last) = self.last else {
            self.last = Some(now);
            return false;
        };
        let interval = Duration::from_secs(60 * minutes.clamp(1, MAX_MINUTES) as u64);
        if now.saturating_duration_since(last) < interval {
            return false;
        }
        self.last = Some(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_acts_when_the_interval_has_passed_and_starts_over() {
        let start = Instant::now();
        let mut timer = PassiveTimer::default();
        assert!(!timer.check(start, true, 2), "turning it on only starts");
        assert!(!timer.check(start + Duration::from_secs(119), true, 2));
        assert!(timer.check(start + Duration::from_secs(120), true, 2));
        assert!(!timer.check(start + Duration::from_secs(121), true, 2));
        assert!(timer.check(start + Duration::from_secs(240), true, 2));
    }

    #[test]
    fn turning_it_off_and_on_resets_it() {
        let start = Instant::now();
        let mut timer = PassiveTimer::default();
        timer.check(start, true, 1);
        assert!(!timer.check(start + Duration::from_secs(90), false, 1));
        assert!(!timer.check(start + Duration::from_secs(100), true, 1));
        assert!(!timer.check(start + Duration::from_secs(150), true, 1));
        assert!(timer.check(start + Duration::from_secs(160), true, 1));
    }

    #[test]
    fn an_interval_below_one_minute_is_one_minute() {
        let start = Instant::now();
        let mut timer = PassiveTimer::default();
        timer.check(start, true, 0);
        assert!(!timer.check(start + Duration::from_secs(59), true, 0));
        assert!(timer.check(start + Duration::from_secs(60), true, -5));
    }
}
