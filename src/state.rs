use crate::protocol::{HooksConfig, SessionState, SessionType};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Session {
    pub session_type: SessionType,
    pub state: SessionState,
    pub phase_start: Option<Instant>,
    pub accumulated: Duration,
    pub work_duration: Duration,
    pub break_duration: Duration,
    pub long_break_duration: Duration,
    pub sessions_before_long_break: u32,
    pub current_session: u32,
    pub break_ratio: f64,
    pub task: Option<String>,
    pub hooks: HooksConfig,
}

impl Session {
    pub fn new_pomodoro(
        work_secs: u64,
        break_secs: u64,
        long_break_secs: u64,
        sessions_before_long: u32,
        task: Option<String>,
        hooks: HooksConfig,
    ) -> Self {
        Session {
            session_type: SessionType::Pomodoro,
            state: SessionState::Working,
            phase_start: Some(Instant::now()),
            accumulated: Duration::ZERO,
            work_duration: Duration::from_secs(work_secs),
            break_duration: Duration::from_secs(break_secs),
            long_break_duration: Duration::from_secs(long_break_secs),
            sessions_before_long_break: sessions_before_long,
            current_session: 1,
            break_ratio: 0.0,
            task,
            hooks,
        }
    }

    pub fn new_flowmodoro(break_ratio: f64, task: Option<String>, hooks: HooksConfig) -> Self {
        Session {
            session_type: SessionType::Flowmodoro,
            state: SessionState::Working,
            phase_start: Some(Instant::now()),
            accumulated: Duration::ZERO,
            work_duration: Duration::MAX,
            break_duration: Duration::MAX,
            long_break_duration: Duration::MAX,
            sessions_before_long_break: 1,
            current_session: 1,
            break_ratio,
            task,
            hooks,
        }
    }

    pub fn elapsed(&self) -> Duration {
        match self.phase_start {
            Some(start) => self.accumulated + start.elapsed(),
            None => self.accumulated,
        }
    }

    pub fn total_duration(&self) -> Duration {
        match self.state {
            SessionState::Working | SessionState::Paused => self.work_duration,
            SessionState::OnBreak => self.current_break_duration(),
            SessionState::Idle => Duration::ZERO,
        }
    }

    fn current_break_duration(&self) -> Duration {
        if self.session_type == SessionType::Flowmodoro {
            self.break_duration
        } else if self.current_session >= self.sessions_before_long_break {
            self.long_break_duration
        } else {
            self.break_duration
        }
    }

    pub fn is_expired(&self) -> bool {
        match self.state {
            SessionState::Working => {
                self.session_type == SessionType::Pomodoro && self.elapsed() >= self.work_duration
            }
            SessionState::OnBreak => self.elapsed() >= self.total_duration(),
            _ => false,
        }
    }

    pub fn pause(&mut self) {
        if self.state == SessionState::Working || self.state == SessionState::OnBreak {
            self.accumulated += self.phase_start.unwrap().elapsed();
            self.phase_start = None;
            self.state = SessionState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == SessionState::Paused {
            self.phase_start = Some(Instant::now());
            self.state = match self.session_type {
                SessionType::Pomodoro => {
                    if self.accumulated < self.work_duration {
                        SessionState::Working
                    } else {
                        SessionState::OnBreak
                    }
                }
                SessionType::Flowmodoro => {
                    if self.accumulated < self.work_duration {
                        SessionState::Working
                    } else {
                        SessionState::OnBreak
                    }
                }
            };
        }
    }

    pub fn advance_to_break(&mut self) {
        self.state = SessionState::OnBreak;
        self.phase_start = Some(Instant::now());
        self.accumulated = Duration::ZERO;
    }

    pub fn advance_to_work(&mut self) {
        if self.current_session >= self.sessions_before_long_break {
            self.current_session = 1;
        } else {
            self.current_session += 1;
        }
        self.state = SessionState::Working;
        self.phase_start = Some(Instant::now());
        self.accumulated = Duration::ZERO;
    }

    pub fn flow_request_break(&mut self) {
        let worked = self.elapsed();
        let break_dur = Duration::from_secs_f64(worked.as_secs_f64() / self.break_ratio);
        self.work_duration = worked;
        self.break_duration = break_dur;
        self.state = SessionState::OnBreak;
        self.phase_start = Some(Instant::now());
        self.accumulated = Duration::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hooks() -> HooksConfig {
        HooksConfig {
            on_work_start: None,
            on_work_end: None,
            on_break_start: None,
            on_break_end: None,
        }
    }

    #[test]
    fn pomodoro_initial_state() {
        let s = Session::new_pomodoro(1500, 300, 900, 4, Some("task".into()), hooks());
        assert_eq!(s.state, SessionState::Working);
        assert_eq!(s.session_type, SessionType::Pomodoro);
        assert_eq!(s.current_session, 1);
        assert_eq!(s.work_duration.as_secs(), 1500);
        assert!(!s.is_expired());
    }

    #[test]
    fn flowmodoro_never_expires_while_working() {
        let s = Session::new_flowmodoro(5.0, None, hooks());
        assert_eq!(s.session_type, SessionType::Flowmodoro);
        assert!(!s.is_expired());
    }

    #[test]
    fn advance_to_work_wraps_around() {
        let mut s = Session::new_pomodoro(1500, 300, 900, 4, None, hooks());
        s.advance_to_work();
        assert_eq!(s.current_session, 2);
        s.advance_to_work();
        s.advance_to_work();
        assert_eq!(s.current_session, 4);
        s.advance_to_work();
        assert_eq!(s.current_session, 1);
    }

    #[test]
    fn long_break_used_on_last_session() {
        let mut s = Session::new_pomodoro(1500, 300, 900, 4, None, hooks());
        s.advance_to_work();
        s.advance_to_work();
        s.advance_to_work();
        assert_eq!(s.current_session, 4);
        s.advance_to_break();
        assert_eq!(s.total_duration().as_secs(), 900);
    }

    #[test]
    fn flow_request_break_sets_proportional_break() {
        let mut s = Session::new_flowmodoro(5.0, None, hooks());
        s.accumulated = Duration::from_secs(3000);
        s.phase_start = None;
        s.flow_request_break();
        assert_eq!(s.state, SessionState::OnBreak);
        assert_eq!(s.break_duration.as_secs(), 600);
    }
}
