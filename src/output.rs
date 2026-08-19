use crate::protocol::{SessionState, SessionType, StatusResponse};
use serde::Serialize;

#[derive(Serialize)]
struct WaybarOutput {
    text: String,
    tooltip: String,
    class: String,
    percentage: u32,
}

pub fn human(status: &StatusResponse) {
    match status.state {
        SessionState::Idle => {
            println!("No active session.");
        }
        SessionState::Working => {
            let elapsed = format_duration(status.elapsed_secs);
            let task = status
                .task
                .as_deref()
                .map(|t| format!(" ({})", t))
                .unwrap_or_default();
            match status.session_type {
                Some(SessionType::Flowmodoro) => {
                    println!("Flowmodoro Working{} - elapsed: {}", task, elapsed);
                }
                Some(SessionType::Stopwatch) => {
                    println!("Stopwatch{} - elapsed: {}", task, elapsed);
                }
                Some(SessionType::Timer) => {
                    let total = format_duration(status.total_duration_secs);
                    let remaining = if status.elapsed_secs < status.total_duration_secs {
                        format_duration(status.total_duration_secs - status.elapsed_secs)
                    } else {
                        "done".to_string()
                    };
                    println!(
                        "Timer{} - elapsed: {} | remaining: {} | total: {}",
                        task, elapsed, remaining, total,
                    );
                }
                Some(SessionType::Pomodoro) | None => {
                    let total = format_duration(status.total_duration_secs);
                    let remaining = if status.elapsed_secs < status.total_duration_secs {
                        format_duration(status.total_duration_secs - status.elapsed_secs)
                    } else {
                        "done".to_string()
                    };
                    let session_info = format!(
                        "session {}/{}",
                        status.current_session, status.sessions_before_long_break
                    );
                    println!(
                        "Pomodoro Working{} - [{}] elapsed: {} | remaining: {} | total: {}",
                        task, session_info, elapsed, remaining, total,
                    );
                }
            }
        }
        SessionState::OnBreak => {
            let elapsed = format_duration(status.elapsed_secs);
            let total = format_duration(status.total_duration_secs);
            let remaining = if status.elapsed_secs < status.total_duration_secs {
                format_duration(status.total_duration_secs - status.elapsed_secs)
            } else {
                "done".to_string()
            };
            println!(
                "Break - elapsed: {} | remaining: {} | total: {}",
                elapsed, remaining, total,
            );
        }
        SessionState::Paused => {
            let elapsed = format_duration(status.elapsed_secs);
            println!("Paused at {}", elapsed);
        }
    }
}

pub fn json(status: &StatusResponse) {
    println!("{}", serde_json::to_string_pretty(status).unwrap());
}

pub fn waybar(status: &StatusResponse) {
    let (text, class) = match status.state {
        SessionState::Idle => ("".to_string(), "idle".to_string()),
        SessionState::Working => {
            let (icon, text) = match status.session_type {
                Some(SessionType::Flowmodoro) => ("F", format_duration(status.elapsed_secs)),
                Some(SessionType::Stopwatch) => ("S", format_duration(status.elapsed_secs)),
                Some(SessionType::Timer) => {
                    let remaining = status
                        .total_duration_secs
                        .saturating_sub(status.elapsed_secs);
                    ("T", format_duration(remaining))
                }
                _ => {
                    let remaining = status
                        .total_duration_secs
                        .saturating_sub(status.elapsed_secs);
                    ("P", format_duration(remaining))
                }
            };
            (format!("{} {}", icon, text), "working".to_string())
        }
        SessionState::OnBreak => {
            let remaining = status
                .total_duration_secs
                .saturating_sub(status.elapsed_secs);
            (
                format!("B {}", format_duration(remaining)),
                "break".to_string(),
            )
        }
        SessionState::Paused => ("Paused".to_string(), "paused".to_string()),
    };

    let percentage = (status.elapsed_secs * 100)
        .checked_div(status.total_duration_secs)
        .unwrap_or(0) as u32;

    let output = WaybarOutput {
        text,
        tooltip: format!(
            "{} {:?} session {}/{}",
            status.task.as_deref().unwrap_or("flow"),
            status.state,
            status.current_session,
            status.sessions_before_long_break,
        ),
        class,
        percentage,
    };

    println!("{}", serde_json::to_string(&output).unwrap());
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{:02}:{:02}", m, s)
    }
}
