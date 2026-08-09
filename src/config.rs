use crate::protocol::HooksConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub work_duration_secs: u64,
    pub break_duration_secs: u64,
    pub long_break_duration_secs: u64,
    pub sessions_before_long_break: u32,
    pub break_ratio: f64,
    pub hooks: HooksConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            work_duration_secs: 25 * 60,
            break_duration_secs: 5 * 60,
            long_break_duration_secs: 15 * 60,
            sessions_before_long_break: 4,
            break_ratio: 5.0,
            hooks: HooksConfig {
                on_work_start: None,
                on_work_end: None,
                on_break_start: None,
                on_break_end: None,
            },
        }
    }
}

impl Config {
    pub fn config_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(dir).join("flow")
        } else {
            dirs_home().join(".config").join("flow")
        }
    }

    pub fn data_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
            PathBuf::from(dir).join("flow")
        } else {
            dirs_home().join(".local").join("share").join("flow")
        }
    }

    pub fn runtime_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
            PathBuf::from(dir).join("flow")
        } else if let Ok(dir) = std::env::var("TMPDIR") {
            PathBuf::from(dir).join(format!("flow-{}", nix::unistd::getuid()))
        } else {
            PathBuf::from(format!("/tmp/flow-{}", nix::unistd::getuid()))
        }
    }

    pub fn socket_path() -> PathBuf {
        Self::runtime_dir().join("flow.sock")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn history_path() -> PathBuf {
        Self::data_dir().join("history.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Config::default()
        }
    }

    pub fn save(&self) {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir).ok();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            std::fs::write(Self::config_path(), json).ok();
        }
    }

    pub fn ensure_dirs() {
        std::fs::create_dir_all(Self::config_dir()).ok();
        std::fs::create_dir_all(Self::data_dir()).ok();
        std::fs::create_dir_all(Self::runtime_dir()).ok();
    }
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
