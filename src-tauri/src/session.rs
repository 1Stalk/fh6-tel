/// Grace window: a new stream starting within this many ms of the last close,
/// with race time that went backwards, is treated as a rewind of the same session.
const REWIND_WINDOW_MS: u64 = 30_000;
/// Rewinds to the very start are indistinguishable from a new race; only treat
/// mid-race values (> 5 s) as rewind candidates.
const REWIND_MIN_RACE_TIME: f32 = 5.0;

pub enum SessionAction {
    Open { car_ordinal: i32, car_class: i32, car_pi: i32 },
    Close,
    None,
}

/// A lap that just finished and should be persisted.
pub struct CompletedLap {
    pub lap_number: i64,
    pub lap_time: f32,
}

pub struct SessionManager {
    auto_record: bool,
    active_id: Option<i64>,
    best_lap: f32,
    last_race_time: f32,
    // Highest progress time seen this session; the rewind baseline. Tracking
    // the peak (not the latest) means a rewind that scrubs time backward
    // during the close grace window still reads as "went backward".
    peak_race_time: f32,
    // Lap tracking for the active session
    seen_lap: Option<u16>,
    last_current_lap: f32,
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
            peak_race_time: 0.0,
            seen_lap: None,
            last_current_lap: 0.0,
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
            self.seen_lap = None;
            self.last_current_lap = 0.0;
            self.peak_race_time = 0.0;
        }
    }

    pub fn update_best_lap(&mut self, lap: f32) {
        if lap > 0.0 && lap < self.best_lap {
            self.best_lap = lap;
        }
    }

    /// The best lap to persist on close: -1.0 means "no lap recorded", which
    /// `db::close_session` treats as "keep the existing best" (rewind-safe).
    pub fn best_for_close(&self) -> f32 {
        if self.best_lap == f32::MAX { -1.0 } else { self.best_lap }
    }

    /// Feed every in-event tick. Returns a lap whenever the lap counter
    /// advanced (its time is the game-reported `last_lap`). Also tracks the
    /// in-progress lap time so a session that ends mid-lap can still record it.
    pub fn note_tick(
        &mut self,
        lap_number: u16,
        current_lap: f32,
        last_lap: f32,
    ) -> Option<CompletedLap> {
        if current_lap > 0.0 {
            self.last_current_lap = current_lap;
        }
        let completed = match self.seen_lap {
            Some(prev) if lap_number > prev && last_lap > 0.0 => {
                self.update_best_lap(last_lap);
                Some(CompletedLap { lap_number: prev as i64, lap_time: last_lap })
            }
            _ => None,
        };
        // A decreasing lap number means a rewind — just resync.
        self.seen_lap = Some(lap_number);
        completed
    }

    /// Call on session close. If a lap was in progress (never crossed the line),
    /// returns it so it can be saved; also folds it into the best lap.
    pub fn take_final_lap(&mut self) -> Option<CompletedLap> {
        let t = self.last_current_lap;
        let ln = self.seen_lap.unwrap_or(0) as i64;
        self.last_current_lap = 0.0;
        // Ignore sub-second residue (e.g. line crossed exactly at the end).
        if t > 1.0 {
            self.update_best_lap(t);
            Some(CompletedLap { lap_number: ln, lap_time: t })
        } else {
            None
        }
    }

    pub fn update_race_time(&mut self, t: f32) {
        self.last_race_time = t;
        if t > self.peak_race_time {
            self.peak_race_time = t;
        }
    }

    /// Call when a session is about to close. Stashes state needed to detect a
    /// subsequent rewind within REWIND_WINDOW_MS.
    pub fn note_close(&mut self, wall_ms: u64) {
        self.closed_id = self.active_id;
        self.closed_wall_ms = wall_ms;
        self.last_race_time_at_close = self.peak_race_time;
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
            (true, false) if self.active_id.is_some() => SessionAction::Close,
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
        assert!(matches!(action, SessionAction::Close));
    }

    #[test]
    fn note_tick_emits_completed_lap_on_advance() {
        let mut sm = SessionManager::new(true);
        sm.set_active_id(Some(1));
        assert!(sm.note_tick(1, 12.0, 0.0).is_none()); // first sighting
        assert!(sm.note_tick(1, 45.0, 0.0).is_none()); // same lap
        let done = sm.note_tick(2, 1.0, 91.3).expect("lap 1 completed");
        assert_eq!(done.lap_number, 1);
        assert!((done.lap_time - 91.3).abs() < 0.001);
    }

    #[test]
    fn take_final_lap_returns_in_progress_lap_and_updates_best() {
        let mut sm = SessionManager::new(true);
        sm.set_active_id(Some(1));
        sm.note_tick(1, 30.0, 0.0);
        sm.note_tick(1, 58.4, 0.0); // mid-lap, never crossed the line
        let f = sm.take_final_lap().expect("final lap");
        assert_eq!(f.lap_number, 1);
        assert!((f.lap_time - 58.4).abs() < 0.001);
        assert!((sm.best_for_close() - 58.4).abs() < 0.001);
        // Consumed — no second emission.
        assert!(sm.take_final_lap().is_none());
    }

    #[test]
    fn take_final_lap_ignores_sub_second_residue() {
        let mut sm = SessionManager::new(true);
        sm.set_active_id(Some(1));
        sm.note_tick(3, 0.4, 0.0);
        assert!(sm.take_final_lap().is_none());
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
    fn rewind_during_grace_window_still_reopens() {
        let mut sm = SessionManager::new(true);
        sm.set_active_id(Some(11));
        sm.update_race_time(90.0); // peak
        // Rewind scrubs the timer back while the close grace period runs.
        sm.update_race_time(8.0);
        sm.note_close(1_000);
        sm.set_active_id(None);
        // New stream resumes at the rewound time — baseline is the peak (90),
        // so this is still recognised as a rewind, not a fresh session.
        let reopen = sm.check_reopen(9.0, 2_000);
        assert_eq!(reopen, Some(11));
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
