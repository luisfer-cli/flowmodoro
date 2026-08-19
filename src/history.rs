use crate::config::Config;
use crate::protocol::SessionType;
use serde::{Deserialize, Serialize};
use std::io::BufReader;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub session_type: SessionType,
    pub work_duration_secs: u64,
    pub break_duration_secs: u64,
    pub task: Option<String>,
    pub completed: bool,
}

pub fn append(entry: &HistoryEntry) {
    let path = Config::history_path();
    let mut entries: Vec<HistoryEntry> = if path.exists() {
        let file = std::fs::File::open(&path).unwrap();
        serde_json::from_reader(BufReader::new(file)).unwrap_or_default()
    } else {
        Vec::new()
    };

    entries.push(entry.clone());

    if let Ok(json) = serde_json::to_string_pretty(&entries) {
        std::fs::write(&path, json).ok();
    }
}

pub fn load() -> Vec<HistoryEntry> {
    let path = Config::history_path();
    if path.exists() {
        let file = std::fs::File::open(&path).unwrap();
        serde_json::from_reader(BufReader::new(file)).unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub fn display(entries: &[HistoryEntry]) {
    if entries.is_empty() {
        println!("No sessions recorded yet.");
        return;
    }

    let total_work: u64 = entries.iter().map(|e| e.work_duration_secs).sum();
    let completed: usize = entries.iter().filter(|e| e.completed).count();
    let pomodoros: usize = entries
        .iter()
        .filter(|e| matches!(e.session_type, SessionType::Pomodoro))
        .count();
    let flows: usize = entries
        .iter()
        .filter(|e| matches!(e.session_type, SessionType::Flowmodoro))
        .count();

    println!(
        "Sessions: {} total ({} pomodoro, {} flowmodoro)",
        entries.len(),
        pomodoros,
        flows
    );
    println!("Completed: {}", completed);
    println!("Total focus time: {}", format_duration(total_work));
    println!();

    for entry in entries.iter().rev().take(10) {
        let marker = if entry.completed { "[OK]" } else { "[--]" };
        let task = entry.task.as_deref().unwrap_or("");
        let session_type = match entry.session_type {
            SessionType::Pomodoro => "pomodoro ",
            SessionType::Flowmodoro => "flowmodoro",
            SessionType::Timer => "timer     ",
            SessionType::Stopwatch => "stopwatch ",
        };
        println!(
            "  {} {} {}  work: {}  break: {}  {}",
            marker,
            entry.timestamp,
            session_type,
            format_duration(entry.work_duration_secs),
            format_duration(entry.break_duration_secs),
            task,
        );
    }

    if entries.len() > 10 {
        println!("  ... and {} more", entries.len() - 10);
    }
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}h{:02}m{:02}s", h, m, s)
    } else {
        format!("{}m{:02}s", m, s)
    }
}
