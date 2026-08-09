use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StartRequest {
    pub session_type: SessionType,
    pub work_duration_secs: u64,
    pub break_duration_secs: u64,
    pub long_break_duration_secs: u64,
    pub sessions_before_long_break: u32,
    pub break_ratio: f64,
    pub task: Option<String>,
    pub hooks: HooksConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HooksConfig {
    pub on_work_start: Option<String>,
    pub on_work_end: Option<String>,
    pub on_break_start: Option<String>,
    pub on_break_end: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Request {
    Start(StartRequest),
    Status,
    Pause,
    Cancel,
    Break,
    Stop,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StatusResponse {
    pub state: SessionState,
    pub session_type: Option<SessionType>,
    pub elapsed_secs: u64,
    pub total_duration_secs: u64,
    pub task: Option<String>,
    pub current_session: u32,
    pub sessions_before_long_break: u32,
    pub work_duration_secs: u64,
    pub break_duration_secs: u64,
    pub accumulated_work_secs: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Response {
    Ok(StatusResponse),
    Error { message: String },
    DaemonNotRunning,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Idle,
    Working,
    OnBreak,
    Paused,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    Pomodoro,
    Flowmodoro,
}
