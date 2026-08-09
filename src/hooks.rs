use crate::protocol::HooksConfig;
use std::process::Command;

pub fn run(hook: Option<&str>) {
    if let Some(cmd) = hook {
        let _ = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
}

pub fn run_on_work_start(hooks: &HooksConfig) {
    run(hooks.on_work_start.as_deref());
}

pub fn run_on_work_end(hooks: &HooksConfig) {
    run(hooks.on_work_end.as_deref());
}

pub fn run_on_break_start(hooks: &HooksConfig) {
    run(hooks.on_break_start.as_deref());
}

pub fn run_on_break_end(hooks: &HooksConfig) {
    run(hooks.on_break_end.as_deref());
}
