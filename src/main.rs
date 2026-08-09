mod client;
mod config;
mod daemon;
mod history;
mod hooks;
mod notify;
mod output;
mod protocol;
mod state;

use clap::{Parser, Subcommand};
use protocol::{HooksConfig, Request, Response, SessionType, StartRequest};

const DAEMON_ARG: &str = "__flow_daemon__";

#[derive(Parser)]
#[command(name = "flow", about = "Pomodoro and Flowmodoro timer")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Start a new session")]
    Start {
        #[arg(long, help = "Start a flowmodoro session")]
        flow: bool,

        #[arg(
            long,
            value_parser = parse_duration,
            default_value = "25m",
            help = "Work duration (e.g., 25m, 1h, 90s)"
        )]
        work: u64,

        #[arg(
            long,
            value_parser = parse_duration,
            default_value = "5m",
            help = "Break duration"
        )]
        break_dur: u64,

        #[arg(
            long,
            value_parser = parse_duration,
            default_value = "15m",
            help = "Long break duration"
        )]
        long_break: u64,

        #[arg(long, default_value = "4", help = "Sessions before long break")]
        sessions: u32,

        #[arg(
            long,
            default_value = "5",
            help = "Flowmodoro break ratio (work:break)"
        )]
        ratio: f64,

        #[arg(long, help = "Task description")]
        task: Option<String>,

        #[arg(long, help = "Command to run when work starts")]
        on_work_start: Option<String>,

        #[arg(long, help = "Command to run when work ends")]
        on_work_end: Option<String>,

        #[arg(long, help = "Command to run when break starts")]
        on_break_start: Option<String>,

        #[arg(long, help = "Command to run when break ends")]
        on_break_end: Option<String>,
    },

    #[command(about = "Show current session status")]
    Status {
        #[arg(long, help = "Output as JSON")]
        json: bool,

        #[arg(long, help = "Output for waybar/polybar")]
        waybar: bool,
    },

    #[command(about = "Pause or resume the current session")]
    Pause,

    #[command(about = "Cancel the current session")]
    Cancel,

    #[command(about = "Request a break (flowmodoro only)")]
    Break,

    #[command(about = "Stop the daemon")]
    Stop,

    #[command(about = "Show or update configuration")]
    Config {
        #[arg(long, value_parser = parse_duration, help = "Set default work duration")]
        work: Option<u64>,

        #[arg(long, value_parser = parse_duration, help = "Set default break duration")]
        break_dur: Option<u64>,

        #[arg(long, value_parser = parse_duration, help = "Set default long break duration")]
        long_break: Option<u64>,

        #[arg(long, help = "Set default sessions before long break")]
        sessions: Option<u32>,

        #[arg(long, help = "Set default flowmodoro break ratio")]
        ratio: Option<f64>,
    },

    #[command(about = "Show session history")]
    History {
        #[arg(long, help = "Output as JSON")]
        json: bool,
    },
}

fn parse_duration(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if let Some(stripped) = s.strip_suffix('h') {
        stripped
            .parse::<f64>()
            .map(|v| (v * 3600.0) as u64)
            .map_err(|_| format!("Invalid duration: {}", s))
    } else if let Some(stripped) = s.strip_suffix('m') {
        stripped
            .parse::<f64>()
            .map(|v| (v * 60.0) as u64)
            .map_err(|_| format!("Invalid duration: {}", s))
    } else if let Some(stripped) = s.strip_suffix('s') {
        stripped
            .parse::<u64>()
            .map_err(|_| format!("Invalid duration: {}", s))
    } else {
        s.parse::<u64>()
            .map_err(|_| format!("Invalid duration: {}", s))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && args[1] == DAEMON_ARG {
        daemon::run();
        return;
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            flow,
            work,
            break_dur,
            long_break,
            sessions,
            ratio,
            task,
            on_work_start,
            on_work_end,
            on_break_start,
            on_break_end,
        } => {
            config::Config::ensure_dirs();

            if !client::daemon_running() {
                ensure_daemon();
            }

            let hooks = HooksConfig {
                on_work_start,
                on_work_end,
                on_break_start,
                on_break_end,
            };

            let session_type = if flow {
                SessionType::Flowmodoro
            } else {
                SessionType::Pomodoro
            };

            let long_break_actual = if sessions == 1 { break_dur } else { long_break };

            let req = Request::Start(StartRequest {
                session_type,
                work_duration_secs: work,
                break_duration_secs: break_dur,
                long_break_duration_secs: long_break_actual,
                sessions_before_long_break: sessions,
                break_ratio: ratio,
                task,
                hooks,
            });

            handle_response(client::send(req));
        }

        Commands::Status { json, waybar } => match client::send(Request::Status) {
            Response::Ok(status) => {
                if waybar {
                    output::waybar(&status);
                } else if json {
                    output::json(&status);
                } else {
                    output::human(&status);
                }
            }
            Response::Error { message } => eprintln!("Error: {}", message),
            Response::DaemonNotRunning => {
                println!("No active session.");
            }
        },

        Commands::Pause => handle_response(client::send(Request::Pause)),

        Commands::Cancel => handle_response(client::send(Request::Cancel)),

        Commands::Break => handle_response(client::send(Request::Break)),

        Commands::Stop => {
            if client::daemon_running() {
                let _ = client::send(Request::Stop);
                println!("Daemon stopped.");
            } else {
                println!("Daemon is not running.");
            }
        }

        Commands::Config {
            work,
            break_dur,
            long_break,
            sessions,
            ratio,
        } => {
            let mut config = config::Config::load();
            let mut changed = false;

            if let Some(w) = work {
                config.work_duration_secs = w;
                changed = true;
            }
            if let Some(b) = break_dur {
                config.break_duration_secs = b;
                changed = true;
            }
            if let Some(l) = long_break {
                config.long_break_duration_secs = l;
                changed = true;
            }
            if let Some(s) = sessions {
                config.sessions_before_long_break = s;
                changed = true;
            }
            if let Some(r) = ratio {
                config.break_ratio = r;
                changed = true;
            }

            if changed {
                config.save();
                println!("Configuration updated.");
            } else {
                println!("Configuration:");
            }

            println!(
                "  work:         {}",
                format_duration(config.work_duration_secs)
            );
            println!(
                "  break:        {}",
                format_duration(config.break_duration_secs)
            );
            println!(
                "  long break:   {}",
                format_duration(config.long_break_duration_secs)
            );
            println!(
                "  sessions:     {} before long break",
                config.sessions_before_long_break
            );
            println!("  flow ratio:   {}:1", config.break_ratio);
        }

        Commands::History { json } => {
            let entries = history::load();
            if json {
                println!("{}", serde_json::to_string_pretty(&entries).unwrap());
            } else {
                history::display(&entries);
            }
        }
    }
}

fn handle_response(response: Response) {
    match response {
        Response::Ok(status) => output::human(&status),
        Response::Error { message } => eprintln!("Error: {}", message),
        Response::DaemonNotRunning => {
            eprintln!("Daemon is not running. Start a session first.");
        }
    }
}

fn ensure_daemon() {
    let exe = std::env::current_exe().unwrap_or_else(|_| {
        std::env::args()
            .next()
            .map(std::path::PathBuf::from)
            .unwrap()
    });

    let _ = std::process::Command::new(exe)
        .arg(DAEMON_ARG)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    for _ in 0..50 {
        if client::daemon_running() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
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
