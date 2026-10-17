# flow

Pomodoro and Flowmodoro timer for Linux, with a background daemon and a command-line client.

## Requirements

- Rust (edition 2024)
- `libdbus` (for notifications through zbus)

## Installation

```sh
cargo build --release
install -m 755 target/release/flow ~/.local/bin/flow
```

## Usage

```sh
# Start a pomodoro (25m work / 5m break by default)
flow start

# Start with a task and custom durations
flow start --task "fix bugs" --work 50m --break 10m

# Start a flowmodoro (break proportional to time worked, 5:1 ratio)
flow start --flow

# Show current status
flow status
flow status --json
flow status --waybar

# Pause / resume
flow pause

# Cancel the current session
flow cancel

# Request a break (flowmodoro only)
flow break

# Timer (counts down)
flow timer 10m
flow timer 30s --task "cook pasta"

# Stopwatch (counts up)
flow stopwatch
flow stopwatch --task "meeting"

# Stop the daemon
flow stop

# View / edit configuration
flow config
flow config --work 50m --break 10m

# Session history
flow history
flow history --json
```

## Configuration

The configuration file lives at `$XDG_CONFIG_HOME/flow/config.json` (or `~/.config/flow/config.json`). Values from `flow config` are used as defaults when starting a session:

- `work_duration_secs` — work duration (25m by default)
- `break_duration_secs` — break duration (5m by default)
- `long_break_duration_secs` — long break duration (15m by default)
- `sessions_before_long_break` — sessions before a long break (4 by default)
- `break_ratio` — Flowmodoro work-to-break ratio (5 by default)
- `hooks` — commands to run when phases change

### Hooks

Commands can be configured to run at the start or end of each phase, either through the CLI or in the configuration:

```sh
flow start --on-work-start "mpv notify.mp3" --on-work-end "notify-send 'Done'"
```

## Notes

- The daemon uses a Unix socket at `$XDG_RUNTIME_DIR/flow/flow.sock` and starts automatically if it is not running.
- Notifications are sent asynchronously over D-Bus; if there is no session bus, they are silently skipped.
