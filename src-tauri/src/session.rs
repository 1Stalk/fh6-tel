pub enum SessionAction {
    Open { car_ordinal: i32, car_pi: i32 },
    Close { best_lap: f32 },
    None,
}

pub struct SessionManager {
    auto_record: bool,
    active_id: Option<i64>,
    best_lap: f32,
}

impl SessionManager {
    pub fn new(auto_record: bool) -> Self {
        Self { auto_record, active_id: Option::None, best_lap: f32::MAX }
    }

    pub fn active_session_id(&self) -> Option<i64> {
        self.active_id
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

    pub fn on_race_on_change(
        &mut self,
        was_racing: bool,
        is_racing: bool,
        car_ordinal: i32,
        car_pi: i32,
    ) -> SessionAction {
        match (was_racing, is_racing) {
            (false, true) if self.auto_record => SessionAction::Open { car_ordinal, car_pi },
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
        let action = sm.on_race_on_change(false, true, 99, 800);
        assert!(matches!(action, SessionAction::Open { car_ordinal: 99, .. }));
    }

    #[test]
    fn closes_session_on_race_end() {
        let mut sm = SessionManager::new(true);
        sm.on_race_on_change(false, true, 0, 0);
        sm.set_active_id(Some(1));
        let action = sm.on_race_on_change(true, false, 0, 0);
        assert!(matches!(action, SessionAction::Close { .. }));
    }

    #[test]
    fn no_action_when_race_on_unchanged() {
        let mut sm = SessionManager::new(true);
        let action = sm.on_race_on_change(true, true, 0, 0);
        assert!(matches!(action, SessionAction::None));
    }

    #[test]
    fn disabled_auto_record_never_opens() {
        let mut sm = SessionManager::new(false);
        let action = sm.on_race_on_change(false, true, 0, 0);
        assert!(matches!(action, SessionAction::None));
    }
}
