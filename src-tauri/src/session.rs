/// Grace window: a new stream starting within this many ms of the last close,
/// with race time that went backwards, is treated as a rewind of the same session.
const REWIND_WINDOW_MS: u64 = 30_000;
/// Rewinds to the very start are indistinguishable from a new race; only treat
/// mid-race values (> 5 s) as rewind candidates.
const REWIND_MIN_RACE_TIME: f32 = 5.0;

pub enum SessionAction {
    Open { car_ordinal: i32, car_class: i32, car_pi: i32 },
    Close { best_lap: f32 },
    None,
}

pub struct SessionManager {
    auto_record: bool,
    active_id: Option<i64>,
    best_lap: f32,
    last_race_time: f32,
    // Rewind detection state — set on close, consumed on reopen
    closed_id: Option<i64>,
    closed_wall_ms: u64,
    last_race_time_at_close: f32,
}

impl SessionManager {
    pub fn new(auto_record: bool) -> Self {
        Self {
            auto_record,
            active_id: Option::None,
            best_lap: f32::MAX,
            last_race_time: 0.0,
            closed_id: None,
            closed_wall_ms: 0,
            last_race_time_at_close: 0.0,
        }
    }

    pub fn active_session_id(&self) -> Option<i64> {
        self.active_id
    }

    pub fn set_auto_record(&mut self, v: bool) {
        self.auto_record = v;
    }

    pub fn set_active_id(&mut self, id: Option<i64>) {
        self.active_id = id;
        if id.is_none() {
            self.best_lap = f32::MAX;
        }
    }

    pub fn update_best_lap(&mut self, lap: f32) {
        if lap > 0.0 && lap < self.best_lap {
            self.best_lap = lap;
        }
    }

    pub fn update_race_time(&mut self, t: f32) {
        self.last_race_time = t;
    }

    /// Call when a session is about to close. Stashes state needed to detect a
    /// subsequent rewind within REWIND_WINDOW_MS.
    pub fn note_close(&mut self, wall_ms: u64) {
        self.closed_id = self.active_id;
        self.closed_wall_ms = wall_ms;
        self.last_race_time_at_close = self.last_race_time;
    }

    /// Returns the session id to reopen when `new_race_time` went backward
    /// within the rewind window. Consumes the stashed state so it cannot fire twice.
    pub fn check_reopen(&mut self, new_race_time: f32, now_wall_ms: u64) -> Option<i64> {
        let id = self.closed_id?;
        let gap_ms = now_wall_ms.saturating_sub(self.closed_wall_ms);
        if gap_ms < REWIND_WINDOW_MS
            && new_race_time > REWIND_MIN_RACE_TIME
            && new_race_time < self.last_race_time_at_close
        {
            self.closed_id = None;
            Some(id)
        } else {
            None
        }
    }

    pub fn on_race_on_change(
        &mut self,
        was_racing: bool,
        is_racing: bool,
        car_ordinal: i32,
        car_class: i32,
        car_pi: i32,
    ) -> SessionAction {
        match (was_racing, is_racing) {
            (false, true) if self.auto_record => SessionAction::Open { car_ordinal, car_class, car_pi },
            (true, false) if self.active_id.is_some() => {
                let best = if self.best_lap == f32::MAX { -1.0 } else { self.best_lap };
                SessionAction::Close { best_lap: best }
            }
            _ => SessionAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_session_when_not_racing() {
        let sm = SessionManager::new(true);
        assert!(sm.active_session_id().is_none());
    }

    #[test]
    fn opens_session_on_race_start() {
        let mut sm = SessionManager::new(true);
        let action = sm.on_race_on_change(false, true, 99, 3, 800);
        assert!(matches!(action, SessionAction::Open { car_ordinal: 99, .. }));
    }

    #[test]
    fn closes_session_on_race_end() {
        let mut sm = SessionManager::new(true);
        sm.on_race_on_change(false, true, 0, 0, 0);
        sm.set_active_id(Some(1));
        let action = sm.on_race_on_change(true, false, 0, 0, 0);
        assert!(matches!(action, SessionAction::Close { .. }));
    }

    #[test]
    fn no_action_when_race_on_unchanged() {
        let mut sm = SessionManager::new(true);
        let action = sm.on_race_on_change(true, true, 0, 0, 0);
        assert!(matches!(action, SessionAction::None));
    }

    #[test]
    fn disabled_auto_record_never_opens() {
        let mut sm = SessionManager::new(false);
        let action = sm.on_race_on_change(false, true, 0, 0, 0);
        assert!(matches!(action, SessionAction::None));
    }

    #[test]
    fn rewind_reopens_session_within_window() {
        let mut sm = SessionManager::new(true);
        sm.set_active_id(Some(42));
        sm.update_race_time(90.0);
        sm.note_close(1_000_000);
        sm.set_active_id(None);
        // New stream starts at 60 s — time went backward, within 30 s wall gap
        let reopen = sm.check_reopen(60.0, 1_005_000);
        assert_eq!(reopen, Some(42));
    }

    #[test]
    fn no_reopen_after_long_gap() {
        let mut sm = SessionManager::new(true);
        sm.set_active_id(Some(7));
        sm.update_race_time(90.0);
        sm.note_close(0);
        sm.set_active_id(None);
        // 60 s gap — beyond the window
        let reopen = sm.check_reopen(60.0, 60_001);
        assert!(reopen.is_none());
    }

    #[test]
    fn no_reopen_for_fresh_start() {
        let mut sm = SessionManager::new(true);
        sm.set_active_id(Some(5));
        sm.update_race_time(120.0);
        sm.note_close(0);
        sm.set_active_id(None);
        // Race time near zero — looks like a new race, not a rewind
        let reopen = sm.check_reopen(1.0, 2_000);
        assert!(reopen.is_none());
    }

    #[test]
    fn no_reopen_when_time_advances() {
        let mut sm = SessionManager::new(true);
        sm.set_active_id(Some(3));
        sm.update_race_time(45.0);
        sm.note_close(0);
        sm.set_active_id(None);
        // Race time went forward — not a rewind
        let reopen = sm.check_reopen(50.0, 5_000);
        assert!(reopen.is_none());
    }

    #[test]
    fn check_reopen_consumes_closed_id() {
        let mut sm = SessionManager::new(true);
        sm.set_active_id(Some(9));
        sm.update_race_time(80.0);
        sm.note_close(0);
        sm.set_active_id(None);
        // First call succeeds
        assert!(sm.check_reopen(40.0, 1_000).is_some());
        // Second call returns None — closed_id was consumed
        assert!(sm.check_reopen(40.0, 2_000).is_none());
    }
}
