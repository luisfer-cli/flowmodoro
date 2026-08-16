use crate::config::Config;
use crate::history::{self, HistoryEntry};
use crate::hooks;
use crate::notify;
use crate::protocol::{Request, Response, SessionState, SessionType, StatusResponse};
use crate::state::Session;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::{Duration, Instant};

struct DaemonSession {
    session: Option<Session>,
    last_tick: Instant,
    pending_notifications: Vec<(String, String, u8)>,
}

impl DaemonSession {
    fn new() -> Self {
        DaemonSession {
            session: None,
            last_tick: Instant::now(),
            pending_notifications: Vec::new(),
        }
    }

    fn is_active(&self) -> bool {
        self.session.is_some()
    }

    fn tick(&mut self) {
        if let Some(ref mut s) = self.session {
            for (summary, body, urgency) in self.pending_notifications.drain(..) {
                notify::send(&summary, &body, urgency);
            }

            let elapsed = self.last_tick.elapsed();
            if s.phase_start.is_some() {
                s.accumulated += elapsed;
                s.phase_start = Some(Instant::now());
            }
            self.last_tick = Instant::now();

            if s.is_expired() {
                match s.state {
                    SessionState::Working => {
                        let worked_secs = s.elapsed().as_secs();
                        let is_pomodoro = s.session_type == SessionType::Pomodoro;

                        if is_pomodoro {
                            history::append(&HistoryEntry {
                                timestamp: timestamp_now(),
                                session_type: s.session_type,
                                work_duration_secs: worked_secs,
                                break_duration_secs: s.break_duration.as_secs(),
                                task: s.task.clone(),
                                completed: true,
                            });
                        }

                        hooks::run_on_work_end(&s.hooks);
                        self.pending_notifications.push((
                            "Work session complete!".to_string(),
                            format!(
                                "Worked for {}. Time for a break.",
                                format_duration(worked_secs),
                            ),
                            2,
                        ));

                        s.advance_to_break();
                        hooks::run_on_break_start(&s.hooks);
                        let break_mins = (if s.current_session >= s.sessions_before_long_break {
                            s.long_break_duration
                        } else {
                            s.break_duration
                        })
                        .as_secs()
                            / 60;
                        self.pending_notifications.push((
                            "Break time!".to_string(),
                            format!("Take a {} minute break.", break_mins),
                            1,
                        ));
                    }
                    SessionState::OnBreak => {
                        let session_done = s.current_session >= s.sessions_before_long_break;
                        hooks::run_on_break_end(&s.hooks);

                        if session_done && s.session_type == SessionType::Pomodoro {
                            notify::send(
                                "All sessions complete!",
                                "You've finished all your pomodoro sessions.",
                                2,
                            );
                            self.session = None;
                            self.pending_notifications.clear();
                        } else if s.session_type == SessionType::Flowmodoro {
                            notify::send("Break complete!", "Your flowmodoro cycle is done.", 1);
                            self.session = None;
                            self.pending_notifications.clear();
                        } else {
                            s.advance_to_work();
                            hooks::run_on_work_start(&s.hooks);
                            self.pending_notifications.push((
                                "Break over!".to_string(),
                                format!(
                                    "Starting session {}/{}.",
                                    s.current_session, s.sessions_before_long_break
                                ),
                                2,
                            ));
                        }
                    }
                    _ => {}
                }
            }
        } else {
            self.pending_notifications.clear();
        }
    }

    fn to_status(&self) -> StatusResponse {
        match &self.session {
            Some(s) => {
                let total = match (s.state, s.session_type) {
                    (SessionState::Working, SessionType::Flowmodoro) => 0,
                    _ => s.total_duration().as_secs(),
                };
                StatusResponse {
                    state: s.state,
                    session_type: Some(s.session_type),
                    elapsed_secs: s.elapsed().as_secs(),
                    total_duration_secs: total,
                    task: s.task.clone(),
                    current_session: s.current_session,
                    sessions_before_long_break: s.sessions_before_long_break,
                    work_duration_secs: s.work_duration.as_secs(),
                    break_duration_secs: s.break_duration.as_secs(),
                    accumulated_work_secs: s.accumulated.as_secs(),
                }
            }
            None => StatusResponse {
                state: SessionState::Idle,
                session_type: None,
                elapsed_secs: 0,
                total_duration_secs: 0,
                task: None,
                current_session: 0,
                sessions_before_long_break: 0,
                work_duration_secs: 0,
                break_duration_secs: 0,
                accumulated_work_secs: 0,
            },
        }
    }

    fn handle(&mut self, req: Request) -> Response {
        match req {
            Request::Start(start_req) => {
                if self.is_active() {
                    return Response::Error {
                        message: "A session is already active. Cancel it first.".to_string(),
                    };
                }

                let session = match start_req.session_type {
                    SessionType::Pomodoro => Session::new_pomodoro(
                        start_req.work_duration_secs,
                        start_req.break_duration_secs,
                        start_req.long_break_duration_secs,
                        start_req.sessions_before_long_break,
                        start_req.task,
                        start_req.hooks,
                    ),
                    SessionType::Flowmodoro => Session::new_flowmodoro(
                        start_req.break_ratio,
                        start_req.task,
                        start_req.hooks,
                    ),
                };

                hooks::run_on_work_start(&session.hooks);
                notify::send(
                    "Focus time!",
                    &format!(
                        "{} started.",
                        match session.session_type {
                            SessionType::Pomodoro => {
                                let mins = session.work_duration.as_secs() / 60;
                                format!("Pomodoro - {} min work", mins)
                            }
                            SessionType::Flowmodoro => {
                                "Flowmodoro - take your time".to_string()
                            }
                        }
                    ),
                    2,
                );

                self.session = Some(session);
                self.last_tick = Instant::now();
                Response::Ok(self.to_status())
            }

            Request::Status => Response::Ok(self.to_status()),

            Request::Pause => match &mut self.session {
                Some(s) if s.state == SessionState::Paused => {
                    s.resume();
                    self.last_tick = Instant::now();
                    notify::send("Resumed", &session_label(s), 1);
                    Response::Ok(self.to_status())
                }
                Some(s) => {
                    s.pause();
                    notify::send("Paused", &session_label(s), 1);
                    Response::Ok(self.to_status())
                }
                None => Response::Error {
                    message: "No active session.".to_string(),
                },
            },

            Request::Cancel => match &mut self.session {
                Some(s) => {
                    let worked = s.elapsed().as_secs();

                    history::append(&HistoryEntry {
                        timestamp: timestamp_now(),
                        session_type: s.session_type,
                        work_duration_secs: worked,
                        break_duration_secs: 0,
                        task: s.task.clone(),
                        completed: false,
                    });

                    self.session = None;
                    self.last_tick = Instant::now();
                    self.pending_notifications.clear();
                    notify::send(
                        "Session cancelled",
                        "The current session has been cancelled.",
                        1,
                    );
                    Response::Ok(self.to_status())
                }
                None => Response::Error {
                    message: "No active session.".to_string(),
                },
            },

            Request::Break => match &mut self.session {
                Some(s)
                    if s.session_type == SessionType::Flowmodoro
                        && s.state == SessionState::Working =>
                {
                    let worked_secs = s.elapsed().as_secs();

                    history::append(&HistoryEntry {
                        timestamp: timestamp_now(),
                        session_type: s.session_type,
                        work_duration_secs: worked_secs,
                        break_duration_secs: (worked_secs as f64 / s.break_ratio) as u64,
                        task: s.task.clone(),
                        completed: true,
                    });

                    hooks::run_on_work_end(&s.hooks);
                    s.flow_request_break();
                    hooks::run_on_break_start(&s.hooks);

                    let break_secs = s.break_duration.as_secs();
                    self.last_tick = Instant::now();
                    notify::send(
                        "Break time!",
                        &format!(
                            "You worked for {}. Take a {} break.",
                            format_duration(worked_secs),
                            format_duration(break_secs),
                        ),
                        1,
                    );

                    Response::Ok(self.to_status())
                }
                Some(_) => Response::Error {
                    message:
                        "Break can only be requested during an active flowmodoro work session."
                            .to_string(),
                },
                None => Response::Error {
                    message: "No active session.".to_string(),
                },
            },

            Request::Stop => {
                self.session = None;
                self.pending_notifications.clear();
                Response::Ok(self.to_status())
            }
        }
    }
}

fn session_label(s: &Session) -> String {
    match s.session_type {
        SessionType::Pomodoro => format!(
            "Pomodoro {}/{}, {}",
            s.current_session,
            s.sessions_before_long_break,
            format_duration(s.elapsed().as_secs()),
        ),
        SessionType::Flowmodoro => {
            format!("Flowmodoro, {}", format_duration(s.elapsed().as_secs()),)
        }
    }
}

fn handle_client(stream: &mut UnixStream, ds: &mut DaemonSession) -> bool {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();

    if reader.read_line(&mut line).is_err() {
        return false;
    }

    let request: Request = match serde_json::from_str(line.trim()) {
        Ok(req) => req,
        Err(e) => {
            let resp = Response::Error {
                message: format!("Invalid request: {}", e),
            };
            let _ = writeln!(stream, "{}", serde_json::to_string(&resp).unwrap());
            return false;
        }
    };

    let is_stop = matches!(request, Request::Stop);
    let response = ds.handle(request);
    let _ = writeln!(stream, "{}", serde_json::to_string(&response).unwrap());

    is_stop
}

pub fn run() {
    match unsafe { nix::unistd::fork() }.expect("fork failed") {
        nix::unistd::ForkResult::Child => {}
        nix::unistd::ForkResult::Parent { .. } => return,
    }

    nix::unistd::setsid().expect("setsid failed");

    match unsafe { nix::unistd::fork() }.expect("second fork failed") {
        nix::unistd::ForkResult::Child => {}
        nix::unistd::ForkResult::Parent { .. } => {
            std::process::exit(0);
        }
    }

    let socket_path = Config::socket_path();

    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let _ = std::fs::remove_file(&socket_path);

    let listener = UnixListener::bind(&socket_path).expect("bind failed");
    listener
        .set_nonblocking(true)
        .expect("set_nonblocking failed");

    unsafe {
        nix::sys::signal::signal(
            nix::sys::signal::Signal::SIGTERM,
            nix::sys::signal::SigHandler::SigDfl,
        )
        .ok();
        nix::sys::signal::signal(
            nix::sys::signal::Signal::SIGINT,
            nix::sys::signal::SigHandler::SigDfl,
        )
        .ok();
    }

    let mut ds = DaemonSession::new();

    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                if handle_client(&mut stream, &mut ds) {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }

        ds.tick();
        std::thread::sleep(Duration::from_millis(100));
    }

    let _ = std::fs::remove_file(&socket_path);
    std::process::exit(0);
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

fn timestamp_now() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let total_secs = dur.as_secs();
    let days = total_secs / 86400;
    let remaining = total_secs % 86400;
    let (year, month, day) = civil_from_days(days as i64);
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        year, month, day, hours, minutes, seconds
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}
